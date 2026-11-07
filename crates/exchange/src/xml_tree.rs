//! Reading XML into a tree, for the formats that are XML (DOCX's and ODT's
//! parts, flat ODT): elements with their namespaces resolved, attributes,
//! and text, built without recursion. No dependencies.
//!
//! It reads what the formats' writers produce and refuses what is not XML,
//! but it is lenient where real files are not quite right: an undeclared
//! prefix keeps its prefix as its namespace, and an unknown entity stays as
//! written. It never fetches anything: a document type declaration is
//! skipped with its internal subset, so no entity it declares is expanded.

use std::borrow::Cow;

/// How deeply elements may nest (Word itself nests tables far less deeply).
pub const MAX_DEPTH: usize = 512;

/// Why a part is not read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XmlError {
    pub message: &'static str,
    /// The byte where reading stopped.
    pub offset: usize,
}

impl std::fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (byte {})", self.message, self.offset)
    }
}

/// A child of an element: another element (by its index) or text.
#[derive(Clone, Debug, PartialEq)]
pub enum Child {
    Element(usize),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
struct Attribute {
    namespace: u16,
    local: String,
    value: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Node {
    namespace: u16,
    local: String,
    attributes: Vec<Attribute>,
    children: Vec<Child>,
}

/// A parsed XML document: its elements in document order, the root first.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    nodes: Vec<Node>,
    /// Namespace names by index; 0 is no namespace.
    namespaces: Vec<String>,
}

/// An element of a [`Tree`].
#[derive(Clone, Copy, Debug)]
pub struct Element<'t> {
    tree: &'t Tree,
    index: usize,
}

impl Tree {
    /// Reads `bytes`: UTF-8 or UTF-16 by their byte order mark, or the
    /// declaration's Windows code page 1252 or ISO 8859-1.
    pub fn parse(bytes: &[u8]) -> Result<Self, XmlError> {
        let text = decode(bytes);
        parse_text(&text)
    }

    pub fn root(&self) -> Element<'_> {
        Element {
            tree: self,
            index: 0,
        }
    }

    fn namespace_index(&self, uri: &str) -> Option<u16> {
        self.namespaces
            .iter()
            .position(|known| known == uri)
            .map(|index| index as u16)
    }
}

impl<'t> Element<'t> {
    fn node(&self) -> &'t Node {
        &self.tree.nodes[self.index]
    }

    /// The local name.
    pub fn local(&self) -> &'t str {
        &self.node().local
    }

    /// The namespace name, empty for none.
    pub fn namespace(&self) -> &'t str {
        &self.tree.namespaces[usize::from(self.node().namespace)]
    }

    /// Whether this is element `local` in namespace `namespace`.
    pub fn is(&self, namespace: &str, local: &str) -> bool {
        self.node().local == local && self.namespace() == namespace
    }

    /// The attribute `local` in `namespace` (empty for an unprefixed one).
    pub fn attribute(&self, namespace: &str, local: &str) -> Option<&'t str> {
        let namespace = if namespace.is_empty() {
            0
        } else {
            self.tree.namespace_index(namespace)?
        };
        self.node()
            .attributes
            .iter()
            .find(|attribute| attribute.namespace == namespace && attribute.local == local)
            .map(|attribute| attribute.value.as_str())
    }

    /// Every attribute: namespace, local name, value.
    pub fn attributes(&self) -> impl Iterator<Item = (&'t str, &'t str, &'t str)> + 't {
        let tree = self.tree;
        self.node().attributes.iter().map(move |attribute| {
            (
                tree.namespaces[usize::from(attribute.namespace)].as_str(),
                attribute.local.as_str(),
                attribute.value.as_str(),
            )
        })
    }

    /// The child elements and text, in order.
    pub fn nodes(&self) -> impl Iterator<Item = Content<'t>> + 't {
        let tree = self.tree;
        self.node().children.iter().map(move |child| match child {
            Child::Element(index) => Content::Element(Element {
                tree,
                index: *index,
            }),
            Child::Text(text) => Content::Text(text),
        })
    }

    /// The child elements, in order.
    pub fn children(&self) -> impl Iterator<Item = Element<'t>> + 't {
        self.nodes().filter_map(|node| match node {
            Content::Element(element) => Some(element),
            Content::Text(_) => None,
        })
    }

    /// The first child element `local` in `namespace`.
    pub fn child(&self, namespace: &str, local: &str) -> Option<Element<'t>> {
        self.children().find(|element| element.is(namespace, local))
    }

    /// The child elements `local` in `namespace`.
    pub fn children_named<'n>(
        &self,
        namespace: &'n str,
        local: &'n str,
    ) -> impl Iterator<Item = Element<'t>> + 'n
    where
        't: 'n,
    {
        self.children()
            .filter(move |element| element.is(namespace, local))
    }

    /// Every element below this one, in document order (not this one).
    pub fn descendants(&self) -> impl Iterator<Item = Element<'t>> + 't {
        let tree = self.tree;
        let mut stack: Vec<usize> = self
            .node()
            .children
            .iter()
            .rev()
            .filter_map(|child| match child {
                Child::Element(index) => Some(*index),
                Child::Text(_) => None,
            })
            .collect();
        std::iter::from_fn(move || {
            let index = stack.pop()?;
            stack.extend(
                tree.nodes[index]
                    .children
                    .iter()
                    .rev()
                    .filter_map(|child| match child {
                        Child::Element(index) => Some(*index),
                        Child::Text(_) => None,
                    }),
            );
            Some(Element { tree, index })
        })
    }

    /// The text of this element and every element below it.
    pub fn text(&self) -> String {
        let mut out = String::new();
        let mut pending: Vec<std::slice::Iter<'_, Child>> =
            vec![self.tree.nodes[self.index].children.iter()];
        while let Some(children) = pending.last_mut() {
            match children.next() {
                Some(Child::Text(text)) => out.push_str(text),
                Some(Child::Element(index)) => {
                    pending.push(self.tree.nodes[*index].children.iter());
                }
                None => {
                    pending.pop();
                }
            }
        }
        out
    }
}

/// A child of an element as [`Element::nodes`] gives it.
#[derive(Clone, Copy, Debug)]
pub enum Content<'t> {
    Element(Element<'t>),
    Text(&'t str),
}

/// `bytes` as text: by the byte order mark, else by the declaration's
/// encoding (Windows 1252 and ISO 8859-1 read as Windows 1252), else UTF-8
/// with damaged sequences replaced.
fn decode(bytes: &[u8]) -> Cow<'_, str> {
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest),
        [0xFF, 0xFE, rest @ ..] => Cow::Owned(utf16(rest, u16::from_le_bytes)),
        [0xFE, 0xFF, rest @ ..] => Cow::Owned(utf16(rest, u16::from_be_bytes)),
        _ => {
            let head = &bytes[..bytes.len().min(200)];
            let head = String::from_utf8_lossy(head).to_ascii_lowercase();
            let declared = head.starts_with("<?xml")
                && (head.contains("windows-1252") || head.contains("iso-8859-1"));
            if declared {
                Cow::Owned(amipro_document::decode_windows_1252(bytes))
            } else {
                String::from_utf8_lossy(bytes)
            }
        }
    }
}

fn utf16(bytes: &[u8], word: fn([u8; 2]) -> u16) -> String {
    let words = bytes.chunks_exact(2).map(|pair| word([pair[0], pair[1]]));
    char::decode_utf16(words)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// An element being read: its node, the namespaces it declared (prefix and
/// the index each had before), and its qualified name for the end tag.
struct Open {
    node: usize,
    declared: Vec<(String, Option<u16>)>,
    name: String,
}

fn parse_text(text: &str) -> Result<Tree, XmlError> {
    let bytes = text.as_bytes();
    let mut tree = Tree {
        nodes: Vec::new(),
        namespaces: vec![String::new(), XML_NAMESPACE.to_owned()],
    };
    // Prefixes in scope: "" is the default namespace.
    let mut scope: std::collections::HashMap<String, u16> =
        std::collections::HashMap::from([("xml".to_owned(), 1), (String::new(), 0)]);
    let mut open: Vec<Open> = Vec::new();
    let mut at = 0;
    let error = |message, offset| XmlError { message, offset };
    while at < bytes.len() {
        if bytes[at] != b'<' {
            let end = text[at..].find('<').map_or(bytes.len(), |end| at + end);
            if let Some(parent) = open.last() {
                let content = unescape(&text[at..end]);
                push_text(&mut tree.nodes[parent.node].children, content.into_owned());
            } else if !text[at..end].trim().is_empty() && !tree.nodes.is_empty() {
                // Text after the root element.
                return Err(error("text outside the root element", at));
            }
            at = end;
            continue;
        }
        let rest = &text[at..];
        if rest.starts_with("<!--") {
            at += rest.find("-->").ok_or(error("an unclosed comment", at))? + 3;
        } else if rest.starts_with("<![CDATA[") {
            let end = rest
                .find("]]>")
                .ok_or(error("an unclosed CDATA section", at))?;
            if let Some(parent) = open.last() {
                push_text(
                    &mut tree.nodes[parent.node].children,
                    rest[9..end].to_owned(),
                );
            }
            at += end + 3;
        } else if rest.starts_with("<?") {
            at += rest
                .find("?>")
                .ok_or(error("an unclosed processing instruction", at))?
                + 2;
        } else if rest.starts_with("<!") {
            // A document type declaration, with any internal subset.
            let mut depth = 0usize;
            let mut end = None;
            for (offset, byte) in rest.bytes().enumerate().skip(2) {
                match byte {
                    b'[' => depth += 1,
                    b']' => depth = depth.saturating_sub(1),
                    b'>' if depth == 0 => {
                        end = Some(offset);
                        break;
                    }
                    _ => {}
                }
            }
            at += end.ok_or(error("an unclosed declaration", at))? + 1;
        } else if let Some(body) = rest.strip_prefix("</") {
            let end = body.find('>').ok_or(error("an unclosed end tag", at))?;
            let name = body[..end].trim();
            let element = open.pop().ok_or(error("an end tag without a start", at))?;
            if element.name != name {
                return Err(error("an end tag that does not match its start", at));
            }
            for (prefix, before) in element.declared {
                match before {
                    Some(index) => scope.insert(prefix, index),
                    None => scope.remove(&prefix),
                };
            }
            at += 2 + end + 1;
        } else {
            let (tag, empty, length) = start_tag(rest).ok_or(error("an unclosed start tag", at))?;
            let (name, attributes) = split_tag(tag).ok_or(error("a damaged start tag", at))?;
            if open.is_empty() && !tree.nodes.is_empty() {
                return Err(error("a second root element", at));
            }
            if open.len() >= MAX_DEPTH {
                return Err(error("elements nested too deeply", at));
            }
            // Namespace declarations first: they apply to this element.
            let mut declared = Vec::new();
            for (attribute, value) in &attributes {
                let prefix = if *attribute == "xmlns" {
                    Some("")
                } else {
                    attribute.strip_prefix("xmlns:")
                };
                if let Some(prefix) = prefix {
                    let value = unescape(value);
                    let index = intern(&mut tree.namespaces, &value);
                    let before = scope.insert(prefix.to_owned(), index);
                    declared.push((prefix.to_owned(), before));
                }
            }
            let mut resolve = |qualified: &str, default: bool| -> (u16, String) {
                match qualified.split_once(':') {
                    Some((prefix, local)) => {
                        let index = match scope.get(prefix) {
                            Some(&index) => index,
                            // Undeclared: the prefix stands for itself.
                            None => intern(&mut tree.namespaces, &format!("{prefix}:")),
                        };
                        (index, local.to_owned())
                    }
                    None if default => (scope.get("").copied().unwrap_or(0), qualified.to_owned()),
                    None => (0, qualified.to_owned()),
                }
            };
            let (namespace, local) = resolve(name, true);
            let attributes = attributes
                .iter()
                .filter(|(attribute, _)| *attribute != "xmlns" && !attribute.starts_with("xmlns:"))
                .map(|(attribute, value)| {
                    let (namespace, local) = resolve(attribute, false);
                    Attribute {
                        namespace,
                        local,
                        value: normalise_attribute(&unescape(value)),
                    }
                })
                .collect();
            let index = tree.nodes.len();
            tree.nodes.push(Node {
                namespace,
                local,
                attributes,
                children: Vec::new(),
            });
            if let Some(parent) = open.last() {
                tree.nodes[parent.node].children.push(Child::Element(index));
            }
            if empty {
                for (prefix, before) in declared {
                    match before {
                        Some(index) => scope.insert(prefix, index),
                        None => scope.remove(&prefix),
                    };
                }
            } else {
                open.push(Open {
                    node: index,
                    declared,
                    name: name.to_owned(),
                });
            }
            at += length;
        }
    }
    if tree.nodes.is_empty() {
        return Err(error("no root element", at));
    }
    if !open.is_empty() {
        return Err(error("an element is not closed", at));
    }
    Ok(tree)
}

fn intern(namespaces: &mut Vec<String>, uri: &str) -> u16 {
    if let Some(index) = namespaces.iter().position(|known| known == uri) {
        return index as u16;
    }
    namespaces.push(uri.to_owned());
    (namespaces.len() - 1).min(usize::from(u16::MAX)) as u16
}

fn push_text(children: &mut Vec<Child>, text: String) {
    if text.is_empty() {
        return;
    }
    if let Some(Child::Text(last)) = children.last_mut() {
        last.push_str(&text);
    } else {
        children.push(Child::Text(text));
    }
}

/// The start tag at the front of `rest` (which starts with `<`): its inside,
/// whether it closes itself, and its length with the brackets.
fn start_tag(rest: &str) -> Option<(&str, bool, usize)> {
    let mut quote = None;
    for (offset, byte) in rest.bytes().enumerate().skip(1) {
        match (quote, byte) {
            (None, b'"' | b'\'') => quote = Some(byte),
            (Some(open), _) if byte == open => quote = None,
            (None, b'>') => {
                let inside = &rest[1..offset];
                return Some(match inside.strip_suffix('/') {
                    Some(inside) => (inside, true, offset + 1),
                    None => (inside, false, offset + 1),
                });
            }
            (None, b'<') => return None,
            _ => {}
        }
    }
    None
}

/// A start tag's name and its attributes' names and raw values.
fn split_tag(tag: &str) -> Option<(&str, Vec<(&str, &str)>)> {
    let tag = tag.trim_end();
    let name_end = tag
        .find(|character: char| character.is_ascii_whitespace())
        .unwrap_or(tag.len());
    let name = &tag[..name_end];
    if name.is_empty() {
        return None;
    }
    let mut attributes = Vec::new();
    let mut rest = tag[name_end..].trim_start();
    while !rest.is_empty() {
        let equals = rest.find('=')?;
        let attribute = rest[..equals].trim();
        let after = rest[equals + 1..].trim_start();
        let quote = after.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let end = after[1..].find(quote)? + 1;
        attributes.push((attribute, &after[1..end]));
        rest = after[end + 1..].trim_start();
    }
    Some((name, attributes))
}

/// Character and entity references replaced; an unknown entity stays.
fn unescape(raw: &str) -> Cow<'_, str> {
    if !raw.contains('&') {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find(';').filter(|&end| end <= 12);
        let replaced = end.and_then(|end| {
            let name = &after[1..end];
            let character = match name {
                "lt" => Some('<'),
                "gt" => Some('>'),
                "amp" => Some('&'),
                "apos" => Some('\''),
                "quot" => Some('"'),
                _ => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|n| n.parse().ok()))
                    .and_then(char::from_u32),
            }?;
            Some((character, end))
        });
        match replaced {
            Some((character, end)) => {
                out.push(character);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// XML's attribute-value normalisation: tabs, line feeds, and returns as
/// spaces (character references were replaced before, as XML says after).
fn normalise_attribute(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    #[test]
    fn namespaces_attributes_and_text_are_read() {
        let tree = Tree::parse(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:r><w:t xml:space="preserve"> a &amp; b &#x263A; </w:t></w:r><!-- c --><w:r><w:t><![CDATA[<x>]]></w:t></w:r></w:p>
<w:sectPr w:rsidR='00'/></w:body></w:document>"#,
        )
        .unwrap();
        let root = tree.root();
        assert!(root.is(W, "document"));
        let body = root.child(W, "body").unwrap();
        let paragraph = body.child(W, "p").unwrap();
        assert_eq!(paragraph.text(), " a & b \u{263A} <x>");
        let text = paragraph.descendants().find(|e| e.is(W, "t")).unwrap();
        assert_eq!(text.attribute(XML_NAMESPACE, "space"), Some("preserve"));
        assert_eq!(
            body.child(W, "sectPr").unwrap().attribute(W, "rsidR"),
            Some("00")
        );
    }

    #[test]
    fn default_and_undeclared_namespaces() {
        let tree = Tree::parse(br#"<a xmlns="urn:a"><b x="1"/><p:c/></a>"#).unwrap();
        let root = tree.root();
        assert!(root.is("urn:a", "a"));
        let b = root.child("urn:a", "b").unwrap();
        assert_eq!(b.attribute("", "x"), Some("1"));
        assert!(root.children().nth(1).unwrap().is("p:", "c"));
    }

    #[test]
    fn a_document_type_is_skipped_and_never_expanded() {
        let tree =
            Tree::parse(br#"<!DOCTYPE a [<!ENTITY e SYSTEM "file:///etc/passwd">]><a>&e;</a>"#)
                .unwrap();
        assert_eq!(tree.root().text(), "&e;");
    }

    #[test]
    fn what_is_not_xml_is_refused_without_a_panic() {
        assert!(Tree::parse(b"<a><b></a>").is_err());
        assert!(Tree::parse(b"<a>").is_err());
        assert!(Tree::parse(b"").is_err());
        assert!(Tree::parse(b"<a/><b/>").is_err());
        let deep = "<a>".repeat(MAX_DEPTH + 1);
        assert!(Tree::parse(deep.as_bytes()).is_err());
        let sample = br#"<a x="1"><b>t&amp;</b><c/></a>"#;
        for cut in 0..sample.len() {
            let _ = Tree::parse(&sample[..cut]);
        }
    }

    #[test]
    fn utf16_and_windows_1252_are_decoded() {
        let mut utf16 = vec![0xFF, 0xFE];
        for unit in "<a>é</a>".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(Tree::parse(&utf16).unwrap().root().text(), "é");
        let ansi = b"<?xml version=\"1.0\" encoding=\"windows-1252\"?><a>\x93q\x94</a>";
        assert_eq!(
            Tree::parse(ansi).unwrap().root().text(),
            "\u{201C}q\u{201D}"
        );
    }
}
