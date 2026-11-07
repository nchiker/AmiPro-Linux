//! Word Documents (ECMA-376, Office Open XML) read into an exchange document
//! (`docs/research/IMPORT_NOTES.md`, "What other programs write").
//!
//! The package's parts are found through their relationships. Styles are
//! resolved through the document defaults and their based-on chains; a
//! character style is applied as its formatting, its toggles (bold, italic,
//! caps…) turning the paragraph style's over as Word does. Numbering comes
//! from the abstract lists and their instances. The body is read paragraph
//! by paragraph with its runs, fields (simple and complex), links, tracked
//! changes (accepted), and content controls; tables onto their grid; and
//! drawings (inline, anchored, text boxes, VML) as shapes. Strict OOXML's
//! namespaces are read as the transitional ones.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::import::{FindingKind, Findings, ImportError};
use crate::xml_tree::{Element, Tree};
use crate::zip::{Archive, Inflate};
use crate::{
    Alignment, Anchor, Block, Cell, CharProps, Colour, Columns, DateSource, ExchangeDocument,
    Field, FieldKind, Font, FontFamily, Footnote, HeaderFooters, HorizontalFrom, Image, ImageKind,
    Inline, LineKind, LineSpacing, List, ListInstance, ListLevel, Note, NumberFormat, PageBorder,
    PageNumbering, PageSetup, Paragraph, ParagraphProps, PlacedPicture, Properties, Row, RowHeight,
    RuleLine, Script, Section, Settings, Shape, ShapeContent, ShapeLine, ShapeShadow, ShapeWrap,
    Style, Tab, TabKind, TabLeader, Table, Twips, Underline, VerticalFrom, VerticalMerge,
};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const W_STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const R_STRICT: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships";
const WP: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const WP_STRICT: &str = "http://purl.oclc.org/ooxml/drawingml/wordprocessingDrawing";
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const A_STRICT: &str = "http://purl.oclc.org/ooxml/drawingml/main";
const WPS: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingShape";
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const V: &str = "urn:schemas-microsoft-com:vml";
const PACKAGE_RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const DC: &str = "http://purl.org/dc/elements/1.1/";
const DCTERMS: &str = "http://purl.org/dc/terms/";
const CP: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
const EXTENDED: &str = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties";
const CUSTOM: &str = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties";
const VT: &str = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes";

/// Whether `element` is `local` in WordprocessingML (either flavour).
fn is_w(element: &Element, local: &str) -> bool {
    element.local() == local && matches!(element.namespace(), W | W_STRICT)
}

fn in_ns(element: &Element, namespaces: &[&str], local: &str) -> bool {
    element.local() == local && namespaces.contains(&element.namespace())
}

/// The first child `w:local`.
fn child<'t>(element: &Element<'t>, local: &str) -> Option<Element<'t>> {
    element.children().find(|child| is_w(child, local))
}

/// A `w:` attribute (or one without a namespace, as some writers write).
fn attr<'t>(element: &Element<'t>, local: &str) -> Option<&'t str> {
    element
        .attribute(W, local)
        .or_else(|| element.attribute(W_STRICT, local))
        .or_else(|| element.attribute("", local))
}

fn val<'t>(element: &Element<'t>) -> Option<&'t str> {
    attr(element, "val")
}

fn number(text: Option<&str>) -> Option<i64> {
    let text = text?.trim();
    text.parse::<i64>()
        .ok()
        .or_else(|| text.parse::<f64>().ok().map(|value| value as i64))
        .or_else(|| {
            // Strict OOXML writes measures with units ("12pt", "1in").
            let (number, unit) = text.split_at(text.find(|c: char| c.is_alphabetic())?);
            let value: f64 = number.parse().ok()?;
            Some(match unit {
                "pt" => value * 20.0,
                "in" => value * 1440.0,
                "cm" => value * 567.0,
                "mm" => value * 56.7,
                "pc" | "pi" => value * 240.0,
                _ => value,
            } as i64)
        })
}

fn twips_of(text: Option<&str>) -> Option<Twips> {
    number(text).map(|value| value.clamp(-1_000_000, 1_000_000) as Twips)
}

/// A toggle (`w:b`, `w:i`, …): on unless its value says off.
fn toggle(element: &Element) -> bool {
    !matches!(val(element), Some("0" | "false" | "off" | "none"))
}

/// `bytes` (a DOCX package) as an exchange document.
pub fn read_docx(
    bytes: &[u8],
    inflate: Inflate,
    findings: &mut Findings,
) -> Result<ExchangeDocument, ImportError> {
    let archive = Archive::open(bytes)?;
    let package = Package { archive, inflate };
    let main = package.main_part()?;
    let rels = package.rels(&main);
    let document = package
        .xml(&main)
        .ok_or_else(|| ImportError::Damaged("the main document part is missing".into()))??;
    let mut reader = Reader::new(&package, &main, rels, findings);
    reader.read_theme();
    reader.read_settings();
    reader.read_fonts();
    reader.read_styles();
    reader.read_numbering();
    reader.read_notes();
    reader.read_comments();
    reader.read_properties();
    let body = document
        .root()
        .children()
        .find(|element| is_w(element, "body"))
        .ok_or_else(|| ImportError::Damaged("the document has no body".into()))?;
    reader.read_body(&body);
    Ok(reader.finish())
}

struct Package<'a> {
    archive: Archive<'a>,
    inflate: Inflate<'a>,
}

impl Package<'_> {
    fn bytes(&self, name: &str) -> Option<Vec<u8>> {
        let name = name.trim_start_matches('/');
        self.archive.read(name, self.inflate).ok()
    }

    fn xml(&self, name: &str) -> Option<Result<Tree, ImportError>> {
        let bytes = self.bytes(name)?;
        Some(Tree::parse(&bytes).map_err(ImportError::from))
    }

    /// The main document's part: the package's office document relation.
    fn main_part(&self) -> Result<String, ImportError> {
        if let Some(Ok(rels)) = self.xml("_rels/.rels") {
            for relation in rels.root().children() {
                let kind = relation.attribute("", "Type").unwrap_or("");
                if kind.ends_with("/officeDocument") {
                    if let Some(target) = relation.attribute("", "Target") {
                        return Ok(target.trim_start_matches('/').to_owned());
                    }
                }
            }
        }
        if self.archive.contains("word/document.xml") {
            return Ok("word/document.xml".into());
        }
        if self.archive.contains("EncryptionInfo") || self.archive.contains("EncryptedPackage") {
            return Err(ImportError::Encrypted);
        }
        Err(ImportError::Damaged("no main document part".into()))
    }

    /// A part's relations: id → (type, the target's part name).
    fn rels(&self, part: &str) -> BTreeMap<String, (String, String)> {
        let (directory, file) = match part.rfind('/') {
            Some(slash) => (&part[..slash], &part[slash + 1..]),
            None => ("", part),
        };
        let rels_name = if directory.is_empty() {
            format!("_rels/{file}.rels")
        } else {
            format!("{directory}/_rels/{file}.rels")
        };
        let mut out = BTreeMap::new();
        let Some(Ok(tree)) = self.xml(&rels_name) else {
            return out;
        };
        for relation in tree.root().children() {
            if relation.namespace() != PACKAGE_RELS || relation.local() != "Relationship" {
                continue;
            }
            let (Some(id), Some(target)) = (
                relation.attribute("", "Id"),
                relation.attribute("", "Target"),
            ) else {
                continue;
            };
            if relation.attribute("", "TargetMode") == Some("External") {
                continue;
            }
            let kind = relation.attribute("", "Type").unwrap_or("").to_owned();
            out.insert(id.to_owned(), (kind, resolve(directory, target)));
        }
        out
    }
}

/// A relative target resolved against `directory`.
fn resolve(directory: &str, target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.to_owned();
    }
    let mut parts: Vec<&str> = if directory.is_empty() {
        Vec::new()
    } else {
        directory.split('/').collect()
    };
    for piece in target.split('/') {
        match piece {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            piece => parts.push(piece),
        }
    }
    parts.join("/")
}

// ---- Formatting ----

/// Character formatting as read, with what it does not say unset.
#[derive(Clone, Debug, Default, PartialEq)]
struct Run {
    font: Option<String>,
    /// Half points.
    size: Option<i64>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<Underline>,
    strike: Option<bool>,
    script: Option<Script>,
    caps: Option<bool>,
    small_caps: Option<bool>,
    colour: Option<Colour>,
    hidden: Option<bool>,
    highlight: bool,
    spacing: bool,
    style: Option<String>,
}

impl Run {
    fn over(&mut self, other: &Run) {
        fn take<T: Clone>(target: &mut Option<T>, value: &Option<T>) {
            if value.is_some() {
                target.clone_from(value);
            }
        }
        take(&mut self.font, &other.font);
        take(&mut self.size, &other.size);
        take(&mut self.bold, &other.bold);
        take(&mut self.italic, &other.italic);
        take(&mut self.underline, &other.underline);
        take(&mut self.strike, &other.strike);
        take(&mut self.script, &other.script);
        take(&mut self.caps, &other.caps);
        take(&mut self.small_caps, &other.small_caps);
        take(&mut self.colour, &other.colour);
        take(&mut self.hidden, &other.hidden);
        take(&mut self.style, &other.style);
        self.highlight |= other.highlight;
        self.spacing |= other.spacing;
    }

    fn props(&self) -> CharProps {
        CharProps {
            font: self.font.clone(),
            size: self.size.map(|size| (size * 10) as Twips),
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            strike: self.strike,
            script: self.script,
            caps: self.caps,
            small_caps: self.small_caps,
            colour: self.colour,
        }
    }
}

/// Paragraph formatting as read.
#[derive(Clone, Debug, Default, PartialEq)]
struct Para {
    props: ParagraphProps,
    style: Option<String>,
    numbering: Option<(i64, u8)>,
    outline: Option<u8>,
    frame: Option<FramePr>,
    other_borders: bool,
    section: bool,
}

impl Para {
    fn over(&mut self, other: &Para) {
        let props = &mut self.props;
        let more = &other.props;
        fn take<T: Clone>(target: &mut Option<T>, value: &Option<T>) {
            if value.is_some() {
                target.clone_from(value);
            }
        }
        take(&mut props.alignment, &more.alignment);
        take(&mut props.indent_left, &more.indent_left);
        take(&mut props.indent_right, &more.indent_right);
        take(&mut props.first_line, &more.first_line);
        take(&mut props.space_before, &more.space_before);
        take(&mut props.space_after, &more.space_after);
        take(&mut props.line, &more.line);
        take(&mut props.keep_with_next, &more.keep_with_next);
        take(&mut props.keep_together, &more.keep_together);
        take(&mut props.page_break_before, &more.page_break_before);
        take(&mut props.border_top, &more.border_top);
        take(&mut props.border_bottom, &more.border_bottom);
        take(&mut props.no_hyphenation, &more.no_hyphenation);
        if let Some(tabs) = &more.tabs {
            // A stop at `-position - 1` clears the inherited stop there.
            let mut merged = props.tabs.clone().unwrap_or_default();
            for tab in tabs {
                let at = if tab.position < 0 {
                    -tab.position - 1
                } else {
                    tab.position
                };
                merged.retain(|known| known.position != at);
                if tab.position >= 0 {
                    merged.push(*tab);
                }
            }
            merged.sort_by_key(|tab| tab.position);
            props.tabs = Some(merged);
        }
        take(&mut self.style, &other.style);
        take(&mut self.numbering, &other.numbering);
        take(&mut self.outline, &other.outline);
        take(&mut self.frame, &other.frame);
        self.other_borders |= other.other_borders;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct FramePr {
    width: Twips,
    height: Twips,
    x: Twips,
    y: Twips,
    horizontal: Option<HorizontalFrom>,
    vertical: Option<VerticalFrom>,
    wrap: ShapeWrap,
}

struct StyleDef {
    id: String,
    name: String,
    kind: String,
    default: bool,
    based_on: Option<String>,
    next: Option<String>,
    para: Para,
    run: Run,
    table_borders: [Option<RuleLine>; 6],
}

/// A table's borders: top, right, bottom, left, inside across, inside down.
type TableBorders = [Option<RuleLine>; 6];

struct Reader<'p, 'a, 'f> {
    package: &'p Package<'a>,
    /// The bookmarks started and not yet ended, by id.
    bookmark_ids: BTreeMap<String, String>,
    rels: BTreeMap<String, (String, String)>,
    findings: &'f mut Findings,
    theme_fonts: (String, String),
    theme_colours: BTreeMap<String, Colour>,
    settings: Settings,
    tab_default: Twips,
    fonts: Vec<Font>,
    styles: Vec<StyleDef>,
    defaults: (Run, Para),
    /// Paragraph styles in the exchange document's order, by id.
    style_order: Vec<String>,
    abstracts: BTreeMap<i64, (usize, Option<String>)>,
    lists: Vec<List>,
    instances: BTreeMap<i64, usize>,
    list_instances: Vec<ListInstance>,
    footnotes: BTreeMap<i64, Vec<Block>>,
    endnotes: BTreeMap<i64, Vec<Block>>,
    comments: BTreeMap<i64, Note>,
    properties: Properties,
    images: Vec<Image>,
    image_parts: BTreeMap<String, usize>,
    sections: Vec<Section>,
    headers: HeaderFooters,
    footers: HeaderFooters,
    next_shape: u32,
    /// The relations of the part being read (headers have their own).
    part_rels: Vec<BTreeMap<String, (String, String)>>,
    fields: Vec<OpenField>,
    depth: usize,
    /// The next text starts an inline of its own (a field's result).
    split: bool,
}

struct OpenField {
    instruction: String,
    /// Where its result begins in the paragraph's inlines.
    result_from: Option<usize>,
}

impl<'p, 'a, 'f> Reader<'p, 'a, 'f> {
    fn new(
        package: &'p Package<'a>,
        main: &str,
        rels: BTreeMap<String, (String, String)>,
        findings: &'f mut Findings,
    ) -> Self {
        let _ = main;
        Self {
            package,
            bookmark_ids: BTreeMap::new(),
            rels: rels.clone(),
            findings,
            theme_fonts: ("Calibri Light".into(), "Calibri".into()),
            theme_colours: BTreeMap::new(),
            settings: Settings::default(),
            tab_default: 720,
            fonts: Vec::new(),
            styles: Vec::new(),
            defaults: (Run::default(), Para::default()),
            style_order: vec![String::new()],
            abstracts: BTreeMap::new(),
            lists: Vec::new(),
            instances: BTreeMap::new(),
            list_instances: Vec::new(),
            footnotes: BTreeMap::new(),
            endnotes: BTreeMap::new(),
            comments: BTreeMap::new(),
            properties: Properties::default(),
            images: Vec::new(),
            image_parts: BTreeMap::new(),
            sections: Vec::new(),
            headers: HeaderFooters::default(),
            footers: HeaderFooters::default(),
            next_shape: 1,
            part_rels: vec![rels],
            fields: Vec::new(),
            depth: 0,
            split: false,
        }
    }

    /// The part a relation of the main document of `kind` names.
    fn related(&self, kind: &str) -> Option<String> {
        self.rels
            .values()
            .find(|(relation, _)| relation.ends_with(kind))
            .map(|(_, target)| target.clone())
    }

    fn related_xml(&self, kind: &str) -> Option<Tree> {
        let part = self.related(kind)?;
        self.package.xml(&part)?.ok()
    }

    // ---- Theme, settings, fonts ----

    fn read_theme(&mut self) {
        let Some(theme) = self.related_xml("/theme") else {
            return;
        };
        for element in theme.root().descendants() {
            let namespaces = [A, A_STRICT];
            if (in_ns(&element, &namespaces, "majorFont")
                || in_ns(&element, &namespaces, "minorFont"))
                && let Some(latin) = element.children().find(|c| in_ns(c, &namespaces, "latin"))
                && let Some(face) = latin
                    .attribute("", "typeface")
                    .filter(|face| !face.is_empty())
            {
                if element.local() == "majorFont" {
                    self.theme_fonts.0 = face.to_owned();
                } else {
                    self.theme_fonts.1 = face.to_owned();
                }
            }
            if in_ns(&element, &namespaces, "clrScheme") {
                for colour in element.children() {
                    let value = colour.children().find_map(|inner| {
                        inner
                            .attribute("", "val")
                            .filter(|_| inner.local() == "srgbClr")
                            .or_else(|| inner.attribute("", "lastClr"))
                    });
                    if let Some(value) = value.and_then(hex_colour) {
                        self.theme_colours.insert(colour.local().to_owned(), value);
                    }
                }
            }
        }
    }

    fn read_settings(&mut self) {
        let Some(settings) = self.related_xml("/settings") else {
            return;
        };
        for element in settings.root().children() {
            match element.local() {
                "evenAndOddHeaders" if toggle(&element) => {
                    self.settings.even_and_odd_headers = true;
                }
                "mirrorMargins" if toggle(&element) => self.settings.mirror_margins = true,
                "autoHyphenation" if toggle(&element) => self.settings.hyphenation = true,
                "hyphenationZone" => {
                    self.settings.hyphenation_zone = twips_of(val(&element)).unwrap_or(360);
                }
                "defaultTabStop" => {
                    self.tab_default = twips_of(val(&element)).unwrap_or(720).max(36);
                }
                _ => {}
            }
        }
    }

    fn read_fonts(&mut self) {
        let Some(table) = self.related_xml("/fontTable") else {
            return;
        };
        for font in table
            .root()
            .children()
            .filter(|element| is_w(element, "font"))
        {
            let Some(name) = attr(&font, "name") else {
                continue;
            };
            let family = child(&font, "family")
                .and_then(|family| val(&family))
                .map_or(FontFamily::Unknown, |family| match family {
                    "roman" => FontFamily::Roman,
                    "swiss" => FontFamily::Swiss,
                    "modern" => FontFamily::Modern,
                    "script" => FontFamily::Script,
                    "decorative" => FontFamily::Decorative,
                    _ => FontFamily::Unknown,
                });
            let fixed = child(&font, "pitch").and_then(|pitch| val(&pitch)) == Some("fixed");
            self.fonts.push(Font {
                name: name.to_owned(),
                family,
                fixed,
            });
        }
    }

    // ---- Styles ----

    fn read_styles(&mut self) {
        let Some(styles) = self.related_xml("/styles") else {
            return;
        };
        for element in styles.root().children() {
            if is_w(&element, "docDefaults") {
                if let Some(run) = child(&element, "rPrDefault").and_then(|d| child(&d, "rPr")) {
                    self.defaults.0 = self.run_props(&run);
                }
                if let Some(para) = child(&element, "pPrDefault").and_then(|d| child(&d, "pPr")) {
                    self.defaults.1 = self.para_props(&para);
                }
                continue;
            }
            if !is_w(&element, "style") {
                continue;
            }
            let id = attr(&element, "styleId").unwrap_or("").to_owned();
            let name = child(&element, "name")
                .and_then(|name| val(&name))
                .map_or_else(|| id.clone(), str::to_owned);
            let kind = attr(&element, "type").unwrap_or("paragraph").to_owned();
            let para = child(&element, "pPr").map_or_else(Para::default, |p| self.para_props(&p));
            let run = child(&element, "rPr").map_or_else(Run::default, |r| self.run_props(&r));
            let table_borders = child(&element, "tblPr")
                .and_then(|properties| child(&properties, "tblBorders"))
                .map_or([None; 6], |borders| self.table_borders(&borders));
            self.styles.push(StyleDef {
                id,
                name: built_in_name(&name),
                kind,
                default: attr(&element, "default")
                    .is_some_and(|value| value == "1" || value == "true"),
                based_on: child(&element, "basedOn")
                    .and_then(|b| val(&b))
                    .map(str::to_owned),
                next: child(&element, "next")
                    .and_then(|n| val(&n))
                    .map(str::to_owned),
                para,
                run,
                table_borders,
            });
        }
        // The default paragraph style first, the others in order.
        let paragraph = |style: &&StyleDef| style.kind == "paragraph";
        let default = self
            .styles
            .iter()
            .filter(paragraph)
            .find(|style| style.default)
            .or_else(|| self.styles.iter().find(paragraph))
            .map(|style| style.id.clone());
        self.style_order = default.clone().into_iter().collect();
        for style in self.styles.iter().filter(paragraph) {
            if Some(&style.id) != default.as_ref() {
                self.style_order.push(style.id.clone());
            }
        }
        if self.style_order.is_empty() {
            self.style_order.push(String::new());
        }
    }

    fn style(&self, id: &str) -> Option<&StyleDef> {
        self.styles.iter().find(|style| style.id == id)
    }

    /// A style's formatting through its based-on chain.
    fn resolved(&self, id: &str) -> (Run, Para) {
        let mut chain = Vec::new();
        let mut current = Some(id.to_owned());
        while let Some(id) = current {
            if chain.len() > 32 || chain.contains(&id) {
                break;
            }
            let Some(style) = self.style(&id) else {
                break;
            };
            chain.push(id);
            current = style.based_on.clone();
        }
        let (mut run, mut para) = (Run::default(), Para::default());
        for id in chain.iter().rev() {
            if let Some(style) = self.style(id) {
                run.over(&style.run);
                para.over(&style.para);
            }
        }
        (run, para)
    }

    /// A paragraph style's whole formatting: the defaults, then its chain.
    fn paragraph_style(&self, id: &str) -> (Run, Para) {
        let (mut run, mut para) = self.defaults.clone();
        let (own_run, own_para) = self.resolved(id);
        run.over(&own_run);
        para.over(&own_para);
        (run, para)
    }

    fn style_index(&self, id: Option<&str>) -> usize {
        let default = || self.style_order.first().cloned().unwrap_or_default();
        let id = id.map_or_else(default, str::to_owned);
        self.style_order
            .iter()
            .position(|known| *known == id)
            .unwrap_or(0)
    }

    // ---- Numbering ----

    fn read_numbering(&mut self) {
        let Some(numbering) = self.related_xml("/numbering") else {
            return;
        };
        let root = numbering.root();
        for element in root
            .children()
            .filter(|element| is_w(element, "abstractNum"))
        {
            let Some(id) = number(attr(&element, "abstractNumId")) else {
                continue;
            };
            let link = child(&element, "numStyleLink")
                .and_then(|link| val(&link))
                .map(str::to_owned);
            let mut levels = Vec::new();
            for level in element.children().filter(|element| is_w(element, "lvl")) {
                levels.push(self.level(&level));
            }
            let index = self.lists.len();
            self.lists.push(List { levels });
            self.abstracts.insert(id, (index, link));
        }
        for element in root.children().filter(|element| is_w(element, "num")) {
            let Some(id) = number(attr(&element, "numId")) else {
                continue;
            };
            let Some(abstract_id) = child(&element, "abstractNumId").and_then(|a| number(val(&a)))
            else {
                continue;
            };
            let Some((list, _)) = self.abstracts.get(&abstract_id).cloned() else {
                continue;
            };
            let mut starts = [None; 9];
            for over in element
                .children()
                .filter(|element| is_w(element, "lvlOverride"))
            {
                let level = number(attr(&over, "ilvl")).unwrap_or(0).clamp(0, 8) as usize;
                if let Some(start) = child(&over, "startOverride").and_then(|s| number(val(&s))) {
                    starts[level] = Some(start.max(0) as u32);
                }
            }
            self.instances.insert(id, self.list_instances.len());
            self.list_instances.push(ListInstance { list, starts });
        }
    }

    fn level(&self, level: &Element) -> ListLevel {
        let format = child(level, "numFmt")
            .and_then(|format| val(&format))
            .map_or(NumberFormat::Decimal, |format| match format {
                "upperRoman" => NumberFormat::UpperRoman,
                "lowerRoman" => NumberFormat::LowerRoman,
                "upperLetter" => NumberFormat::UpperLetter,
                "lowerLetter" => NumberFormat::LowerLetter,
                "bullet" => NumberFormat::Bullet,
                "none" => NumberFormat::None,
                _ => NumberFormat::Decimal,
            });
        let text = child(level, "lvlText")
            .and_then(|text| val(&text))
            .unwrap_or("")
            .to_owned();
        let run = child(level, "rPr").map_or_else(Run::default, |r| self.run_props(&r));
        let para = child(level, "pPr").map_or_else(Para::default, |p| self.para_props(&p));
        ListLevel {
            format,
            text: if format == NumberFormat::Bullet {
                text.chars().map(word_bullet).collect()
            } else {
                text
            },
            start: child(level, "start")
                .and_then(|start| number(val(&start)))
                .unwrap_or(1)
                .max(0) as u32,
            restart_after_higher: child(level, "lvlRestart")
                .and_then(|restart| number(val(&restart)))
                .is_none_or(|restart| restart != 0),
            font: run.font.clone().filter(|_| format == NumberFormat::Bullet),
            props: CharProps {
                bold: run.bold,
                italic: run.italic,
                underline: run.underline,
                caps: run.caps,
                script: run.script,
                ..CharProps::default()
            },
            right_align: child(level, "lvlJc").and_then(|jc| val(&jc)) == Some("right"),
            indent_left: para.props.indent_left.unwrap_or(0),
            hanging: -para.props.first_line.unwrap_or(0),
        }
    }

    /// A paragraph's list instance and level: its own, or its style's.
    fn numbering_of(&self, para: &Para) -> Option<(usize, u8)> {
        let (id, level) = para.numbering?;
        if id == 0 {
            return None;
        }
        self.instances.get(&id).map(|instance| (*instance, level))
    }

    // ---- Notes, comments, properties ----

    fn read_notes(&mut self) {
        for (kind, endnote) in [("/footnotes", false), ("/endnotes", true)] {
            let Some(part) = self.related(kind) else {
                continue;
            };
            let Some(Ok(tree)) = self.package.xml(&part) else {
                continue;
            };
            let rels = self.package.rels(&part);
            self.part_rels.push(rels);
            for note in tree.root().children() {
                if !(is_w(&note, "footnote") || is_w(&note, "endnote")) {
                    continue;
                }
                // Separators are not notes.
                if attr(&note, "type").is_some_and(|kind| kind != "normal") {
                    continue;
                }
                let Some(id) = number(attr(&note, "id")) else {
                    continue;
                };
                let blocks = self.blocks_of(&note);
                if endnote {
                    self.endnotes.insert(id, blocks);
                } else {
                    self.footnotes.insert(id, blocks);
                }
            }
            self.part_rels.pop();
        }
    }

    fn read_comments(&mut self) {
        let Some(part) = self.related("/comments") else {
            return;
        };
        let Some(Ok(tree)) = self.package.xml(&part) else {
            return;
        };
        for comment in tree
            .root()
            .children()
            .filter(|element| is_w(element, "comment"))
        {
            let Some(id) = number(attr(&comment, "id")) else {
                continue;
            };
            let blocks = self.blocks_of(&comment);
            self.comments.insert(
                id,
                Note {
                    author: attr(&comment, "author").unwrap_or("").to_owned(),
                    initials: attr(&comment, "initials").unwrap_or("").to_owned(),
                    date: attr(&comment, "date").and_then(w3c_date),
                    blocks,
                },
            );
        }
    }

    fn read_properties(&mut self) {
        let core = self.package.xml("docProps/core.xml").and_then(Result::ok);
        if let Some(core) = core {
            for element in core.root().children() {
                let text = element.text().trim().to_owned();
                match (element.namespace(), element.local()) {
                    (DC, "title") => self.properties.title = text,
                    (DC, "description") => self.properties.description = text,
                    (DC, "subject") if self.properties.description.is_empty() => {
                        self.properties.description = text;
                    }
                    (CP, "keywords") => self.properties.keywords = text,
                    (CP, "revision") => {
                        self.properties.revisions = text.parse().unwrap_or(0);
                    }
                    (DCTERMS, "created") => self.properties.created = w3c_date(&text),
                    (DCTERMS, "modified") => self.properties.modified = w3c_date(&text),
                    _ => {}
                }
            }
        }
        if let Some(Ok(app)) = self.package.xml("docProps/app.xml") {
            for element in app.root().children() {
                if element.namespace() != EXTENDED {
                    continue;
                }
                let value: u32 = element.text().trim().parse().unwrap_or(0);
                match element.local() {
                    "Pages" => self.properties.pages = value,
                    "Words" => self.properties.words = value,
                    "Characters" => self.properties.characters = value,
                    "TotalTime" => self.properties.editing_minutes = value,
                    "Application" => {
                        self.settings.ami_pro_rows =
                            element.text().trim().starts_with(crate::GENERATOR);
                    }
                    _ => {}
                }
            }
        }
        if let Some(Ok(custom)) = self.package.xml("docProps/custom.xml") {
            for property in custom.root().children() {
                if property.namespace() != CUSTOM {
                    continue;
                }
                let Some(name) = property.attribute("", "name") else {
                    continue;
                };
                let value = property
                    .children()
                    .find(|value| value.namespace() == VT)
                    .map(|value| value.text())
                    .unwrap_or_default();
                self.properties.custom.push((name.to_owned(), value));
            }
        }
    }

    // ---- Formatting from XML ----

    fn run_props(&self, element: &Element) -> Run {
        let mut run = Run::default();
        for property in element.children() {
            match property.local() {
                "rStyle" => run.style = val(&property).map(str::to_owned),
                "rFonts" => {
                    let theme =
                        attr(&property, "asciiTheme").or_else(|| attr(&property, "hAnsiTheme"));
                    run.font = match theme {
                        Some(theme) if theme.starts_with("major") => {
                            Some(self.theme_fonts.0.clone())
                        }
                        Some(_) => Some(self.theme_fonts.1.clone()),
                        None => attr(&property, "ascii")
                            .or_else(|| attr(&property, "hAnsi"))
                            .map(str::to_owned),
                    };
                }
                "b" => run.bold = Some(toggle(&property)),
                "i" => run.italic = Some(toggle(&property)),
                "caps" => run.caps = Some(toggle(&property)),
                "smallCaps" => run.small_caps = Some(toggle(&property)),
                "strike" | "dstrike" => {
                    let on = toggle(&property);
                    if on || run.strike.is_none() {
                        run.strike = Some(on);
                    }
                }
                "vanish" | "specVanish" => run.hidden = Some(toggle(&property)),
                "u" => {
                    run.underline = Some(match val(&property) {
                        Some("none") | Some("0") => Underline::None,
                        Some("double") => Underline::Double,
                        Some("words") => Underline::Words,
                        _ => Underline::Single,
                    });
                }
                "vertAlign" => {
                    run.script = Some(match val(&property) {
                        Some("superscript") => Script::Super,
                        Some("subscript") => Script::Sub,
                        _ => Script::Normal,
                    });
                }
                "sz" => run.size = number(val(&property)),
                "color" => {
                    let theme =
                        attr(&property, "themeColor").and_then(|theme| self.theme_colour(theme));
                    run.colour = theme.or_else(|| match val(&property) {
                        Some("auto") | None => Some(0),
                        Some(value) => hex_colour(value),
                    });
                }
                "highlight" => run.highlight = val(&property) != Some("none"),
                "shd" => {
                    run.highlight |= attr(&property, "fill")
                        .is_some_and(|fill| fill != "auto" && !fill.eq_ignore_ascii_case("FFFFFF"));
                }
                "spacing" | "w" | "position" => {
                    let value = number(val(&property)).unwrap_or(0);
                    if value != 0 && !(property.local() == "w" && value == 100) {
                        run.spacing = true;
                    }
                }
                _ => {}
            }
        }
        run
    }

    fn theme_colour(&self, theme: &str) -> Option<Colour> {
        let key = match theme {
            "text1" | "dark1" => "dk1",
            "text2" | "dark2" => "dk2",
            "background1" | "light1" => "lt1",
            "background2" | "light2" => "lt2",
            "hyperlink" => "hlink",
            "followedHyperlink" => "folHlink",
            other => other,
        };
        self.theme_colours.get(key).copied()
    }

    fn para_props(&self, element: &Element) -> Para {
        let mut para = Para::default();
        let props = &mut para.props;
        for property in element.children() {
            match property.local() {
                "pStyle" => para.style = val(&property).map(str::to_owned),
                "jc" => {
                    props.alignment = Some(match val(&property) {
                        Some("center") => Alignment::Centre,
                        Some("right" | "end") => Alignment::Right,
                        Some(
                            "both" | "distribute" | "lowKashida" | "mediumKashida" | "highKashida"
                            | "thaiDistribute",
                        ) => Alignment::Justify,
                        _ => Alignment::Left,
                    });
                }
                "ind" => {
                    if let Some(left) =
                        twips_of(attr(&property, "left").or_else(|| attr(&property, "start")))
                    {
                        props.indent_left = Some(left);
                    }
                    if let Some(right) =
                        twips_of(attr(&property, "right").or_else(|| attr(&property, "end")))
                    {
                        props.indent_right = Some(right);
                    }
                    if let Some(hanging) = twips_of(attr(&property, "hanging")) {
                        props.first_line = Some(-hanging);
                    } else if let Some(first) = twips_of(attr(&property, "firstLine")) {
                        props.first_line = Some(first);
                    }
                }
                "spacing" => {
                    if let Some(before) = twips_of(attr(&property, "before")) {
                        props.space_before = Some(before.max(0));
                    }
                    if let Some(after) = twips_of(attr(&property, "after")) {
                        props.space_after = Some(after.max(0));
                    }
                    if attr(&property, "beforeAutospacing").is_some_and(|v| v == "1") {
                        props.space_before = Some(280);
                    }
                    if attr(&property, "afterAutospacing").is_some_and(|v| v == "1") {
                        props.space_after = Some(280);
                    }
                    if let Some(line) = twips_of(attr(&property, "line")) {
                        props.line = Some(match attr(&property, "lineRule") {
                            Some("exact") => LineSpacing::Exact(line.abs()),
                            Some("atLeast") => LineSpacing::AtLeast(line.abs()),
                            _ => LineSpacing::Multiple(line as f32 / 240.0),
                        });
                    }
                }
                "keepNext" => props.keep_with_next = Some(toggle(&property)),
                "keepLines" => props.keep_together = Some(toggle(&property)),
                "pageBreakBefore" => props.page_break_before = Some(toggle(&property)),
                "suppressAutoHyphens" => props.no_hyphenation = Some(toggle(&property)),
                "outlineLvl" => {
                    para.outline = number(val(&property))
                        .filter(|level| (0..9).contains(level))
                        .map(|level| level as u8 + 1);
                }
                "numPr" => {
                    let id = child(&property, "numId").and_then(|n| number(val(&n)));
                    let level = child(&property, "ilvl")
                        .and_then(|l| number(val(&l)))
                        .unwrap_or(0)
                        .clamp(0, 8) as u8;
                    if let Some(id) = id {
                        para.numbering = Some((id, level));
                    }
                }
                "tabs" => {
                    let mut tabs = Vec::new();
                    for tab in property.children().filter(|tab| is_w(tab, "tab")) {
                        let Some(position) = twips_of(attr(&tab, "pos")) else {
                            continue;
                        };
                        let kind = match val(&tab) {
                            Some("clear") => {
                                // A stop cleared from the style.
                                tabs.push(Tab {
                                    position: -position - 1,
                                    kind: TabKind::Left,
                                    leader: TabLeader::None,
                                });
                                continue;
                            }
                            Some("center") => TabKind::Centre,
                            Some("right" | "end") => TabKind::Right,
                            Some("decimal") => TabKind::Decimal,
                            Some("bar") => continue,
                            _ => TabKind::Left,
                        };
                        let leader = match attr(&tab, "leader") {
                            Some("dot" | "middleDot") => TabLeader::Dots,
                            Some("hyphen") => TabLeader::Hyphens,
                            Some("underscore" | "heavy") => TabLeader::Line,
                            _ => TabLeader::None,
                        };
                        tabs.push(Tab {
                            position,
                            kind,
                            leader,
                        });
                    }
                    props.tabs = Some(tabs);
                }
                "pBdr" => {
                    for side in property.children() {
                        match side.local() {
                            "top" => props.border_top = Some(self.border(&side)),
                            "bottom" => props.border_bottom = Some(self.border(&side)),
                            "left" | "right" | "between" | "bar" | "start" | "end"
                                if self.border(&side).is_some() =>
                            {
                                para.other_borders = true;
                            }
                            _ => {}
                        }
                    }
                }
                "shd" => {
                    if attr(&property, "fill")
                        .is_some_and(|fill| fill != "auto" && !fill.eq_ignore_ascii_case("FFFFFF"))
                    {
                        para.other_borders = true;
                    }
                }
                "framePr" => para.frame = Some(frame_pr(&property)),
                "sectPr" => para.section = true,
                _ => {}
            }
        }
        para
    }

    /// A border element as a rule (none for `nil` and `none`).
    fn border(&self, side: &Element) -> Option<RuleLine> {
        let kind = val(side)?;
        if matches!(kind, "nil" | "none") {
            return None;
        }
        let size = number(attr(side, "sz")).unwrap_or(4);
        let colour = match attr(side, "color") {
            Some("auto") | None => 0,
            Some(value) => hex_colour(value).unwrap_or(0),
        };
        Some(RuleLine {
            kind: match kind {
                "double" => LineKind::Double,
                "triple" => LineKind::Triple,
                "thickThinSmallGap" | "thickThinMediumGap" | "thickThinLargeGap" => {
                    LineKind::ThickThin
                }
                "thinThickSmallGap" | "thinThickMediumGap" | "thinThickLargeGap" => {
                    LineKind::ThinThick
                }
                _ => LineKind::Single,
            },
            // Eighths of a point.
            width: ((size * 20 + 4) / 8).max(1) as Twips,
            colour,
            space: number(attr(side, "space")).unwrap_or(0).max(0) as Twips * 20,
        })
    }

    fn table_borders(&self, element: &Element) -> TableBorders {
        let mut out = [None; 6];
        for side in element.children() {
            let index = match side.local() {
                "top" => 0,
                "right" | "end" => 1,
                "bottom" => 2,
                "left" | "start" => 3,
                "insideH" => 4,
                "insideV" => 5,
                _ => continue,
            };
            out[index] = self.border(&side);
        }
        out
    }

    // ---- The body ----

    fn read_body(&mut self, body: &Element) {
        let mut blocks = Vec::new();
        for element in body.children() {
            if is_w(&element, "sectPr") {
                let section = self.section(&element, std::mem::take(&mut blocks));
                self.sections.push(section);
                continue;
            }
            let ends_section = is_w(&element, "p")
                && child(&element, "pPr").is_some_and(|p| child(&p, "sectPr").is_some());
            self.block(&element, &mut blocks);
            if ends_section
                && let Some(properties) = child(&element, "pPr").and_then(|p| child(&p, "sectPr"))
            {
                let section = self.section(&properties, std::mem::take(&mut blocks));
                self.sections.push(section);
            }
        }
        if !blocks.is_empty() {
            let section = self.default_section(blocks);
            self.sections.push(section);
        }
        // The empty paragraph Word needs after a table that ends the text.
        if let Some(section) = self.sections.last_mut() {
            let blocks = &mut section.blocks;
            if blocks.len() > 1
                && matches!(blocks[blocks.len() - 2], Block::Table(_))
                && matches!(blocks.last(), Some(Block::Paragraph(last)) if last.inlines.is_empty() && last.props == ParagraphProps::default())
            {
                blocks.pop();
            }
        }
    }

    /// The blocks inside `container` (a note, a cell, a header, a text box).
    fn blocks_of(&mut self, container: &Element) -> Vec<Block> {
        let mut blocks = Vec::new();
        for element in container.children() {
            self.block(&element, &mut blocks);
        }
        if blocks.is_empty() {
            blocks.push(Block::Paragraph(Paragraph::default()));
        }
        blocks
    }

    fn block(&mut self, element: &Element, blocks: &mut Vec<Block>) {
        self.depth += 1;
        if self.depth > 64 {
            self.depth -= 1;
            return;
        }
        if is_w(element, "p") {
            let made = self.paragraph(element);
            blocks.extend(made);
        } else if is_w(element, "tbl") {
            if let Some(table) = self.table(element) {
                blocks.push(table);
            }
        } else if is_w(element, "sdt") {
            if let Some(content) = child(element, "sdtContent") {
                for inner in content.children() {
                    self.block(&inner, blocks);
                }
            }
        } else if is_w(element, "customXml")
            || element.namespace() == MC && element.local() == "AlternateContent"
        {
            let inner = alternate(element).unwrap_or(*element);
            for inner in inner.children() {
                self.block(&inner, blocks);
            }
        } else if is_w(element, "altChunk") {
            self.findings
                .count(FindingKind::Unreadable("an embedded document".into()));
        }
        self.depth -= 1;
    }

    /// A paragraph, and a frame of framed paragraphs around it.
    fn paragraph(&mut self, element: &Element) -> Vec<Block> {
        let para = child(element, "pPr").map_or_else(Para::default, |p| self.para_props(&p));
        let style_id = para
            .style
            .clone()
            .unwrap_or_else(|| self.style_order[0].clone());
        let (style_run, _) = self.paragraph_style(&style_id);
        let mark = child(element, "pPr")
            .and_then(|p| child(&p, "rPr"))
            .map_or_else(Run::default, |r| self.run_props(&r));
        let mut inlines = Vec::new();
        self.inlines(element, &style_run, &mut inlines);
        if para.other_borders {
            self.findings.count(FindingKind::ParagraphBorders);
        }
        let list = self
            .numbering_of(&para)
            .map(|(instance, level)| (Some(instance), level));
        let paragraph = Paragraph {
            style: self.style_index(Some(&style_id)),
            props: own_props(&para),
            list,
            number_text: None,
            inlines,
            mark: mark.props(),
        };
        match para.frame {
            Some(frame) if frame.width > 0 => vec![self.framed(frame, paragraph)],
            _ => vec![Block::Paragraph(paragraph)],
        }
    }

    /// A framed paragraph (`w:framePr`): a frame holding it.
    fn framed(&mut self, frame: FramePr, paragraph: Paragraph) -> Block {
        let id = self.shape_id();
        let height = if frame.height > 0 { frame.height } else { 300 };
        let shape = Shape {
            id,
            name: String::new(),
            anchor: Anchor::Floating {
                horizontal: (frame.horizontal.unwrap_or(HorizontalFrom::Column), frame.x),
                vertical: (frame.vertical.unwrap_or(VerticalFrom::Paragraph), frame.y),
                wrap: frame.wrap,
                behind: false,
            },
            width: frame.width,
            height,
            z: id,
            line: None,
            fill: None,
            shadow: None,
            corner_radius: 0,
            content: ShapeContent::Text {
                blocks: vec![Block::Paragraph(paragraph)],
                insets: [0; 4],
                columns: 1,
                column_gap: 0,
            },
        };
        Block::Paragraph(Paragraph {
            inlines: vec![Inline::Shape(Box::new(shape))],
            ..Paragraph::default()
        })
    }

    fn shape_id(&mut self) -> u32 {
        let id = self.next_shape;
        self.next_shape += 1;
        id
    }

    /// The inlines of a paragraph-level container (a paragraph, a link, an
    /// insertion…).
    fn inlines(&mut self, container: &Element, style_run: &Run, out: &mut Vec<Inline>) {
        for element in container.children() {
            let local = element.local();
            if !matches!(element.namespace(), W | W_STRICT) {
                if element.namespace() == MC && local == "AlternateContent" {
                    if let Some(chosen) = alternate(&element) {
                        self.inlines(&chosen, style_run, out);
                    }
                }
                if local == "oMath" || local == "oMathPara" {
                    self.findings.count(FindingKind::Equation);
                    let text = element.text();
                    if !text.is_empty() {
                        out.push(Inline::Text {
                            text,
                            props: CharProps::default(),
                        });
                    }
                }
                continue;
            }
            match local {
                "r" => self.run(&element, style_run, out),
                "hyperlink" => {
                    let marker = self.findings.at(FindingKind::Links);
                    out.push(Inline::Marker(marker));
                    self.inlines(&element, style_run, out);
                }
                "ins" | "moveTo" => {
                    self.findings.count(FindingKind::Insertions);
                    self.inlines(&element, style_run, out);
                }
                "del" | "moveFrom" => self.findings.count(FindingKind::Deletions),
                "smartTag" | "customXml" | "dir" | "bdo" => self.inlines(&element, style_run, out),
                "sdt" => {
                    if child(&element, "sdtPr").is_some_and(|properties| {
                        properties.children().any(|kind| {
                            matches!(
                                kind.local(),
                                "text" | "dropDownList" | "comboBox" | "date" | "checkbox"
                            )
                        })
                    }) {
                        self.findings.count(FindingKind::FormFields);
                    }
                    if let Some(content) = child(&element, "sdtContent") {
                        self.inlines(&content, style_run, out);
                    }
                }
                "fldSimple" => {
                    let instruction = attr(&element, "instr").unwrap_or("").to_owned();
                    let start = out.len();
                    self.split = true;
                    self.inlines(&element, style_run, out);
                    let result: Vec<Inline> = out.split_off(start);
                    self.field(&instruction, result, out);
                }
                // Bookmarks, but Word's own (named from `_`), as Ami
                // Pro's (the notes run).
                "bookmarkStart" => {
                    let name = attr(&element, "name").unwrap_or("").to_owned();
                    let id = attr(&element, "id").unwrap_or("").to_owned();
                    if !name.starts_with('_') && !name.is_empty() {
                        self.bookmark_ids.insert(id, name.clone());
                        out.push(Inline::Bookmark { name, end: false });
                    }
                }
                "bookmarkEnd" => {
                    let id = attr(&element, "id").unwrap_or("");
                    if let Some(name) = self.bookmark_ids.remove(id) {
                        out.push(Inline::Bookmark { name, end: true });
                    }
                }
                "commentRangeStart" => {}
                _ => {}
            }
        }
    }

    /// A run's contents.
    fn run(&mut self, element: &Element, style_run: &Run, out: &mut Vec<Inline>) {
        let direct = child(element, "rPr").map_or_else(Run::default, |r| self.run_props(&r));
        // The character style, its toggles over the paragraph's style's.
        let mut run = Run::default();
        if let Some(style) = &direct.style {
            let (character, _) = self.resolved(style);
            run = character.clone();
            for (own, base) in [
                (&mut run.bold, style_run.bold),
                (&mut run.italic, style_run.italic),
                (&mut run.caps, style_run.caps),
                (&mut run.small_caps, style_run.small_caps),
                (&mut run.strike, style_run.strike),
            ] {
                if *own == Some(true) && base == Some(true) {
                    *own = Some(false);
                }
            }
            if let Some(name) = self.style(style).map(|style| style.name.clone())
                && !name.eq_ignore_ascii_case("Default Paragraph Font")
                && !name.eq_ignore_ascii_case("Hyperlink")
            {
                self.findings.count(FindingKind::CharacterStyle(name));
            }
        }
        run.over(&direct);
        if run.hidden == Some(true) {
            self.findings.count(FindingKind::HiddenText);
            return;
        }
        if run.highlight {
            self.findings.count(FindingKind::Highlight);
        }
        if run.spacing {
            self.findings.count(FindingKind::CharacterSpacing);
        }
        let props = run.props();
        for part in element.children() {
            if part.namespace() == MC && part.local() == "AlternateContent" {
                if let Some(chosen) = alternate(&part) {
                    for inner in chosen.children() {
                        self.run_part(&inner, &props, out);
                    }
                }
                continue;
            }
            self.run_part(&part, &props, out);
        }
    }

    /// Text in a run, joined to the text before it where they agree (not
    /// across a field result's start).
    fn push_text(&mut self, out: &mut Vec<Inline>, text: &str, props: &CharProps) {
        if text.is_empty() {
            return;
        }
        if !std::mem::take(&mut self.split)
            && let Some(Inline::Text {
                text: last,
                props: same,
            }) = out.last_mut()
            && same == props
        {
            last.push_str(text);
            return;
        }
        out.push(Inline::Text {
            text: text.to_owned(),
            props: props.clone(),
        });
    }

    fn run_part(&mut self, part: &Element, props: &CharProps, out: &mut Vec<Inline>) {
        // Inside a complex field's code, text is the code.
        let in_code = self
            .fields
            .last()
            .is_some_and(|field| field.result_from.is_none());
        match (part.namespace(), part.local()) {
            (W | W_STRICT, "t") => {
                if in_code {
                    return;
                }
                self.push_text(out, &part.text(), props);
            }
            (W | W_STRICT, "instrText") => {
                if let Some(field) = self.fields.last_mut() {
                    field.instruction.push_str(&part.text());
                }
            }
            (W | W_STRICT, "delText" | "delInstrText") => {}
            (W | W_STRICT, "tab" | "ptab") if !in_code => out.push(Inline::Tab {
                props: props.clone(),
            }),
            (W | W_STRICT, "br") if !in_code => out.push(match attr(part, "type") {
                Some("page") => Inline::PageBreak,
                Some("column") => Inline::ColumnBreak,
                _ => Inline::LineBreak,
            }),
            (W | W_STRICT, "cr") if !in_code => out.push(Inline::LineBreak),
            (W | W_STRICT, "noBreakHyphen") if !in_code => self.push_text(out, "\u{2011}", props),
            (W | W_STRICT, "softHyphen") if !in_code => self.push_text(out, "\u{AD}", props),
            (W | W_STRICT, "sym") if !in_code => {
                let character = attr(part, "char")
                    .and_then(|code| u32::from_str_radix(code, 16).ok())
                    .map(|code| {
                        if (0xF000..=0xF0FF).contains(&code) {
                            code - 0xF000
                        } else {
                            code
                        }
                    })
                    .and_then(char::from_u32)
                    .unwrap_or('?');
                out.push(Inline::Symbol {
                    character,
                    font: attr(part, "font").map(str::to_owned),
                    props: props.clone(),
                });
            }
            (W | W_STRICT, "fldChar") => match attr(part, "fldCharType") {
                Some("begin") => self.fields.push(OpenField {
                    instruction: String::new(),
                    result_from: None,
                }),
                Some("separate") => {
                    if let Some(field) = self.fields.last_mut() {
                        field.result_from = Some(out.len());
                    }
                    self.split = true;
                }
                Some("end") => {
                    if let Some(field) = self.fields.pop() {
                        let start = field.result_from.unwrap_or(out.len()).min(out.len());
                        let result = out.split_off(start);
                        self.field(&field.instruction, result, out);
                    }
                }
                _ => {}
            },
            (W | W_STRICT, "footnoteReference" | "endnoteReference") => {
                let endnote = part.local() == "endnoteReference";
                let id = number(attr(part, "id")).unwrap_or(-1);
                let blocks = if endnote {
                    self.endnotes.get(&id).cloned()
                } else {
                    self.footnotes.get(&id).cloned()
                };
                if let Some(blocks) = blocks {
                    out.push(Inline::Footnote(Box::new(Footnote {
                        endnote,
                        blocks,
                        props: props.clone(),
                    })));
                }
            }
            (W | W_STRICT, "commentReference") => {
                let id = number(attr(part, "id")).unwrap_or(-1);
                if let Some(note) = self.comments.get(&id).cloned() {
                    out.push(Inline::Marker(self.findings.at(FindingKind::Comments)));
                    out.push(Inline::Note(Box::new(note)));
                }
            }
            (W | W_STRICT, "drawing") if !in_code => {
                for drawing in part.children() {
                    if let Some(shape) = self.drawing(&drawing) {
                        out.push(Inline::Shape(Box::new(shape)));
                    }
                }
            }
            (W | W_STRICT, "pict" | "object") if !in_code => {
                if part.local() == "object" {
                    self.findings.count(FindingKind::Objects);
                }
                for shape in part.descendants().filter(|e| e.namespace() == V) {
                    if matches!(
                        shape.local(),
                        "shape" | "rect" | "roundrect" | "textbox" | "oval"
                    ) && let Some(made) = self.vml(&shape)
                    {
                        out.push(Inline::Shape(Box::new(made)));
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    /// A field from its code and its result.
    fn field(&mut self, instruction: &str, result: Vec<Inline>, out: &mut Vec<Inline>) {
        let words = split_code(instruction);
        let name = words
            .first()
            .map(|word| word.to_ascii_uppercase())
            .unwrap_or_default();
        let text: String = result
            .iter()
            .map(|inline| match inline {
                Inline::Text { text, .. } => text.as_str(),
                _ => "",
            })
            .collect();
        let props = result
            .iter()
            .find_map(|inline| match inline {
                Inline::Text { props, .. } => Some(props.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let picture = words
            .iter()
            .position(|word| word == "\\@")
            .and_then(|at| words.get(at + 1))
            .cloned()
            .unwrap_or_else(|| "MM/dd/yy".into());
        let upper = instruction.contains("\\* Upper");
        let format = if instruction.contains("ROMAN") {
            NumberFormat::UpperRoman
        } else if instruction.contains("roman") {
            NumberFormat::LowerRoman
        } else if instruction.contains("ALPHABETIC") {
            NumberFormat::UpperLetter
        } else if instruction.contains("alphabetic") {
            NumberFormat::LowerLetter
        } else {
            NumberFormat::Decimal
        };
        let argument = words.get(1).cloned().unwrap_or_default();
        let time = (picture.contains('h') || picture.contains('H')) && !picture.contains('d');
        let date = |source| {
            if time {
                FieldKind::Time { source }
            } else {
                FieldKind::Date {
                    source,
                    picture: crate::read_rtf::date_picture(&picture),
                    upper,
                }
            }
        };
        let kind = match name.as_str() {
            "PAGE" => Some(FieldKind::Page { format }),
            "NUMPAGES" | "SECTIONPAGES" => Some(FieldKind::NumPages),
            "DATE" | "PRINTDATE" => Some(date(DateSource::Now)),
            "TIME" => Some(FieldKind::Time {
                source: DateSource::Now,
            }),
            "CREATEDATE" => Some(date(DateSource::Created)),
            "SAVEDATE" => Some(date(DateSource::Saved)),
            "TITLE" => Some(FieldKind::Title),
            "COMMENTS" | "SUBJECT" => Some(FieldKind::Description),
            "KEYWORDS" => Some(FieldKind::Keywords),
            "FILENAME" => Some(FieldKind::FileName {
                path: instruction.contains("\\p"),
            }),
            "TEMPLATE" => Some(FieldKind::Template),
            "REVNUM" => Some(FieldKind::Revisions),
            "EDITTIME" => Some(FieldKind::EditingTime),
            "NUMWORDS" => Some(FieldKind::Words),
            "NUMCHARS" => Some(FieldKind::Characters),
            "DOCPROPERTY" => Some(FieldKind::Custom(argument.clone())),
            "SEQ" => Some(FieldKind::Seq {
                name: argument.clone(),
            }),
            "XE" => {
                let (primary, secondary) = argument.split_once(':').unwrap_or((&argument, ""));
                Some(FieldKind::IndexEntry {
                    primary: primary.to_owned(),
                    secondary: secondary.to_owned(),
                })
            }
            "TC" => Some(FieldKind::ContentsEntry {
                text: argument.clone(),
                level: words
                    .iter()
                    .position(|word| word == "\\l")
                    .and_then(|at| words.get(at + 1))
                    .and_then(|level| level.parse().ok())
                    .unwrap_or(1),
            }),
            _ => None,
        };
        match kind {
            Some(kind) => out.push(Inline::Field(Field {
                kind,
                result: text,
                props,
            })),
            None => {
                let found = match name.as_str() {
                    "HYPERLINK" => Some(FindingKind::Links),
                    "TOC" | "INDEX" | "TOA" => Some(FindingKind::GeneratedText),
                    "" => None,
                    "FORMTEXT" | "FORMCHECKBOX" | "FORMDROPDOWN" => Some(FindingKind::FormFields),
                    "INCLUDEPICTURE" | "EMBED" | "LINK" | "INCLUDETEXT" => {
                        Some(FindingKind::Objects)
                    }
                    other => Some(FindingKind::Field(other.to_owned())),
                };
                if let Some(found) = found {
                    let marker = self.findings.at(found);
                    out.push(Inline::Marker(marker));
                }
                out.extend(result);
            }
        }
    }

    // ---- Tables ----

    fn table(&mut self, element: &Element) -> Option<Block> {
        let properties = child(element, "tblPr");
        let style_borders = properties
            .as_ref()
            .and_then(|p| child(p, "tblStyle"))
            .and_then(|style| val(&style))
            .and_then(|id| self.style(id))
            .map_or([None; 6], |style| style.table_borders);
        let own_borders = properties
            .as_ref()
            .and_then(|p| child(p, "tblBorders"))
            .map(|borders| self.table_borders(&borders));
        let mut borders = style_borders;
        if let Some(own) = own_borders {
            for (slot, value) in borders.iter_mut().zip(own) {
                *slot = value.or(*slot);
            }
        }
        let indent = properties
            .as_ref()
            .and_then(|p| child(p, "tblInd"))
            .and_then(|ind| twips_of(attr(&ind, "w")))
            .unwrap_or(0);
        let align = properties
            .as_ref()
            .and_then(|p| child(p, "jc"))
            .and_then(|jc| val(&jc))
            .map_or(Alignment::Left, |jc| match jc {
                "center" => Alignment::Centre,
                "right" | "end" => Alignment::Right,
                _ => Alignment::Left,
            });
        let mut margins = [0, 108, 0, 108];
        if let Some(cell_margins) = properties.as_ref().and_then(|p| child(p, "tblCellMar")) {
            for side in cell_margins.children() {
                let width = twips_of(attr(&side, "w")).unwrap_or(0).max(0);
                match side.local() {
                    "top" => margins[0] = width,
                    "right" | "end" => margins[1] = width,
                    "bottom" => margins[2] = width,
                    "left" | "start" => margins[3] = width,
                    _ => {}
                }
            }
        }
        let floating = properties.as_ref().and_then(|p| child(p, "tblpPr"));
        let mut columns: Vec<Twips> = child(element, "tblGrid")
            .map(|grid| {
                grid.children()
                    .filter(|column| is_w(column, "gridCol"))
                    .map(|column| twips_of(attr(&column, "w")).unwrap_or(1_440).max(20))
                    .collect()
            })
            .unwrap_or_default();
        let mut rows = Vec::new();
        let mut row_elements: Vec<Element> = Vec::new();
        for item in element.children() {
            if is_w(&item, "tr") {
                row_elements.push(item);
            } else if is_w(&item, "sdt")
                && let Some(content) = child(&item, "sdtContent")
            {
                row_elements.extend(content.children().filter(|row| is_w(row, "tr")));
            }
        }
        let row_count = row_elements.len();
        for (row_index, row) in row_elements.iter().enumerate() {
            let row_props = child(row, "trPr");
            let height = row_props
                .as_ref()
                .and_then(|p| child(p, "trHeight"))
                .map(|height| {
                    let value = twips_of(val(&height)).unwrap_or(0).max(0);
                    match attr(&height, "hRule") {
                        Some("exact") => RowHeight::Exact(value),
                        _ => RowHeight::AtLeast(value),
                    }
                })
                .unwrap_or(RowHeight::AtLeast(0));
            let heading = row_props
                .as_ref()
                .and_then(|p| child(p, "tblHeader"))
                .is_some_and(|header| toggle(&header));
            let keep = row_props
                .as_ref()
                .and_then(|p| child(p, "cantSplit"))
                .is_some_and(|keep| toggle(&keep));
            let mut cells = Vec::new();
            let mut cell_elements: Vec<Element> = Vec::new();
            for item in row.children() {
                if is_w(&item, "tc") {
                    cell_elements.push(item);
                } else if is_w(&item, "sdt")
                    && let Some(content) = child(&item, "sdtContent")
                {
                    cell_elements.extend(content.children().filter(|cell| is_w(cell, "tc")));
                }
            }
            let cell_count = cell_elements.len();
            for (cell_index, cell) in cell_elements.iter().enumerate() {
                let cell_props = child(cell, "tcPr");
                let span = cell_props
                    .as_ref()
                    .and_then(|p| child(p, "gridSpan"))
                    .and_then(|span| number(val(&span)))
                    .unwrap_or(1)
                    .clamp(1, 63) as u16;
                let merge = cell_props.as_ref().and_then(|p| child(p, "vMerge")).map_or(
                    VerticalMerge::None,
                    |merge| match val(&merge) {
                        Some("restart") => VerticalMerge::Start,
                        _ => VerticalMerge::Continue,
                    },
                );
                // Borders: the cell's own, else the table's at its place.
                let edge = |index: usize, inside: usize, outer: bool| {
                    if outer {
                        borders[index]
                    } else {
                        borders[inside]
                    }
                };
                let mut cell_borders = [
                    edge(0, 4, row_index == 0),
                    edge(1, 5, cell_index + 1 == cell_count),
                    edge(2, 4, row_index + 1 == row_count),
                    edge(3, 5, cell_index == 0),
                ];
                if let Some(own) = cell_props.as_ref().and_then(|p| child(p, "tcBorders")) {
                    let own = self.table_borders(&own);
                    for (slot, given) in cell_borders.iter_mut().zip(own) {
                        if given.is_some() {
                            *slot = given;
                        }
                    }
                }
                let shading = cell_props
                    .as_ref()
                    .and_then(|p| child(p, "shd"))
                    .and_then(|shd| attr(&shd, "fill"))
                    .filter(|fill| *fill != "auto")
                    .and_then(hex_colour)
                    .filter(|colour| *colour != 0xFF_FFFF);
                let mut blocks = Vec::new();
                for inner in cell.children() {
                    if is_w(&inner, "tbl") {
                        // A table in a table: flattened by `realise`.
                        if let Some(nested) = self.table(&inner) {
                            blocks.push(nested);
                        }
                        continue;
                    }
                    self.block(&inner, &mut blocks);
                }
                if blocks.is_empty() {
                    blocks.push(Block::Paragraph(Paragraph::default()));
                }
                cells.push(Cell {
                    span,
                    merge,
                    blocks,
                    borders: cell_borders,
                    shading,
                });
            }
            let used: usize = cells.iter().map(|cell| usize::from(cell.span)).sum();
            if used > columns.len() {
                columns.resize(used, 1_440);
            }
            rows.push(Row {
                height,
                heading,
                keep_together: keep,
                page_break_before: false,
                cells,
            });
        }
        if rows.is_empty() {
            return None;
        }
        if columns.is_empty() {
            columns.push(1_440);
        }
        let table = Table {
            columns,
            rows,
            indent: indent.max(0),
            align,
            cell_margins: margins,
        };
        if let Some(floating) = floating {
            // A floating table: a frame holding it.
            self.findings.count(FindingKind::FloatingTable);
            let width: Twips = table.columns.iter().sum();
            let height: Twips = table
                .rows
                .iter()
                .map(|row| match row.height {
                    RowHeight::AtLeast(height) | RowHeight::Exact(height) => height.max(300),
                })
                .sum();
            let horizontal = match attr(&floating, "horzAnchor") {
                Some("page") => HorizontalFrom::Page,
                Some("margin") => HorizontalFrom::Margin,
                _ => HorizontalFrom::Column,
            };
            let vertical = match attr(&floating, "vertAnchor") {
                Some("page") => VerticalFrom::Page,
                Some("margin") => VerticalFrom::Margin,
                _ => VerticalFrom::Paragraph,
            };
            let id = self.shape_id();
            let shape = Shape {
                id,
                name: String::new(),
                anchor: Anchor::Floating {
                    horizontal: (horizontal, twips_of(attr(&floating, "tblpX")).unwrap_or(0)),
                    vertical: (vertical, twips_of(attr(&floating, "tblpY")).unwrap_or(0)),
                    wrap: ShapeWrap::Square,
                    behind: false,
                },
                width,
                height,
                z: id,
                line: None,
                fill: None,
                shadow: None,
                corner_radius: 0,
                content: ShapeContent::Text {
                    blocks: vec![Block::Table(table)],
                    insets: [0; 4],
                    columns: 1,
                    column_gap: 0,
                },
            };
            return Some(Block::Paragraph(Paragraph {
                inlines: vec![Inline::Shape(Box::new(shape))],
                ..Paragraph::default()
            }));
        }
        Some(Block::Table(table))
    }

    // ---- Sections ----

    fn default_section(&mut self, blocks: Vec<Block>) -> Section {
        let page = self
            .sections
            .last()
            .map_or_else(letter_page, |section| section.page.clone());
        Section {
            page,
            new_page: true,
            headers: self.headers.clone(),
            footers: self.footers.clone(),
            page_numbers: None,
            blocks: if blocks.is_empty() {
                vec![Block::Paragraph(Paragraph::default())]
            } else {
                blocks
            },
        }
    }

    fn section(&mut self, properties: &Element, mut blocks: Vec<Block>) -> Section {
        let mut page = letter_page();
        let mut new_page = true;
        let mut title_page = false;
        let mut numbering = None;
        let mut columns_seen = false;
        for property in properties.children() {
            match property.local() {
                "pgSz" => {
                    page.width = twips_of(attr(&property, "w")).unwrap_or(12_240);
                    page.height = twips_of(attr(&property, "h")).unwrap_or(15_840);
                    page.landscape = attr(&property, "orient") == Some("landscape");
                }
                "pgMar" => {
                    page.margin_top = twips_of(attr(&property, "top")).unwrap_or(1_440).abs();
                    page.margin_bottom = twips_of(attr(&property, "bottom")).unwrap_or(1_440).abs();
                    page.margin_left =
                        twips_of(attr(&property, "left").or_else(|| attr(&property, "start")))
                            .unwrap_or(1_440)
                            + twips_of(attr(&property, "gutter")).unwrap_or(0);
                    page.margin_right =
                        twips_of(attr(&property, "right").or_else(|| attr(&property, "end")))
                            .unwrap_or(1_440);
                    page.header_distance = twips_of(attr(&property, "header")).unwrap_or(720);
                    page.footer_distance = twips_of(attr(&property, "footer")).unwrap_or(720);
                }
                "cols" => {
                    columns_seen = true;
                    let count = number(attr(&property, "num")).unwrap_or(1).clamp(1, 8) as u8;
                    let gap = twips_of(attr(&property, "space")).unwrap_or(720);
                    let widths: Vec<(Twips, Twips)> = property
                        .children()
                        .filter(|column| is_w(column, "col"))
                        .map(|column| {
                            (
                                twips_of(attr(&column, "w")).unwrap_or(0),
                                twips_of(attr(&column, "space")).unwrap_or(0),
                            )
                        })
                        .collect();
                    let equal = attr(&property, "equalWidth") != Some("0");
                    page.columns = Columns {
                        count: if widths.len() > 1 && !equal {
                            widths.len().min(8) as u8
                        } else {
                            count
                        },
                        gap,
                        widths: (!equal && widths.len() > 1).then_some(widths),
                        line_between: attr(&property, "sep")
                            .is_some_and(|sep| sep == "1" || sep == "true"),
                    };
                }
                "titlePg" => title_page = toggle(&property),
                "type" => new_page = val(&property) != Some("continuous"),
                "pgNumType" => {
                    let format = match attr(&property, "fmt") {
                        Some("upperRoman") => NumberFormat::UpperRoman,
                        Some("lowerRoman") => NumberFormat::LowerRoman,
                        Some("upperLetter") => NumberFormat::UpperLetter,
                        Some("lowerLetter") => NumberFormat::LowerLetter,
                        _ => NumberFormat::Decimal,
                    };
                    let start = number(attr(&property, "start")).map(|start| start.max(0) as u32);
                    numbering = Some(PageNumbering { format, start });
                }
                "pgBorders" => {
                    let from_text = attr(&property, "offsetFrom") != Some("page");
                    let mut border = PageBorder {
                        top: false,
                        bottom: false,
                        left: false,
                        right: false,
                        line: RuleLine {
                            kind: LineKind::Single,
                            width: 10,
                            colour: 0,
                            space: 0,
                        },
                        distance: 0,
                    };
                    for side in property.children() {
                        let Some(line) = self.border(&side) else {
                            continue;
                        };
                        border.line = RuleLine { space: 0, ..line };
                        border.distance = if from_text { 360 } else { line.space };
                        match side.local() {
                            "top" => border.top = true,
                            "bottom" => border.bottom = true,
                            "left" => border.left = true,
                            "right" => border.right = true,
                            _ => {}
                        }
                    }
                    if border.top || border.bottom || border.left || border.right {
                        page.border = Some(border);
                    }
                }
                "headerReference" | "footerReference" => {
                    let kind = attr(&property, "type").unwrap_or("default");
                    let Some(id) = property
                        .attribute(R, "id")
                        .or_else(|| property.attribute(R_STRICT, "id"))
                    else {
                        continue;
                    };
                    let Some((_, part)) = self.rels.get(id).cloned() else {
                        continue;
                    };
                    let content = self.header_part(&part);
                    let set = if property.local() == "headerReference" {
                        &mut self.headers
                    } else {
                        &mut self.footers
                    };
                    match kind {
                        "even" => set.even = Some(content),
                        "first" => set.first = Some(content),
                        _ => set.default = Some(content),
                    }
                }
                _ => {}
            }
        }
        let _ = columns_seen;
        if blocks.is_empty() {
            blocks.push(Block::Paragraph(Paragraph::default()));
        }
        let mut headers = self.headers.clone();
        let mut footers = self.footers.clone();
        if !title_page {
            headers.first = None;
            footers.first = None;
        }
        if !self.settings.even_and_odd_headers {
            headers.even = None;
            footers.even = None;
        }
        Section {
            page,
            new_page: new_page || self.sections.is_empty(),
            headers,
            footers,
            page_numbers: numbering,
            blocks,
        }
    }

    /// A header or footer part's blocks.
    fn header_part(&mut self, part: &str) -> Vec<Block> {
        let Some(Ok(tree)) = self.package.xml(part) else {
            return vec![Block::Paragraph(Paragraph::default())];
        };
        let rels = self.package.rels(part);
        self.part_rels.push(rels);
        let root = tree.root();
        let blocks = self.blocks_of(&root);
        self.part_rels.pop();
        blocks
    }

    // ---- Drawings ----

    /// A relation of the part being read.
    fn part_relation(&self, id: &str) -> Option<String> {
        self.part_rels
            .last()
            .and_then(|rels| rels.get(id))
            .map(|(_, target)| target.clone())
    }

    /// The image a relation names, read once.
    fn image(&mut self, id: &str) -> Option<usize> {
        let part = self.part_relation(id)?;
        if let Some(index) = self.image_parts.get(&part) {
            return Some(*index);
        }
        let bytes = self.package.bytes(&part)?;
        let kind = image_kind(&part, &bytes)?;
        self.images.push(Image {
            bytes: Arc::from(bytes),
            kind,
            fallback: None,
            pixels: None,
        });
        let index = self.images.len() - 1;
        self.image_parts.insert(part, index);
        Some(index)
    }

    fn drawing(&mut self, drawing: &Element) -> Option<Shape> {
        let namespaces = [WP, WP_STRICT];
        let inline = in_ns(drawing, &namespaces, "inline");
        if !inline && !in_ns(drawing, &namespaces, "anchor") {
            return None;
        }
        let emu = |value: Option<&str>| number(value).map_or(0, |value| (value / 635) as Twips);
        let extent = drawing.children().find(|e| in_ns(e, &namespaces, "extent"));
        let width = extent
            .as_ref()
            .map_or(1_440, |e| emu(e.attribute("", "cx")))
            .max(20);
        let height = extent
            .as_ref()
            .map_or(1_440, |e| emu(e.attribute("", "cy")))
            .max(20);
        let properties = drawing.children().find(|e| in_ns(e, &namespaces, "docPr"));
        let name = properties
            .as_ref()
            .and_then(|p| p.attribute("", "name"))
            .unwrap_or("")
            .to_owned();
        let description = properties
            .as_ref()
            .and_then(|p| p.attribute("", "descr"))
            .unwrap_or("")
            .to_owned();
        let anchor = if inline {
            Anchor::Inline { drop: 0 }
        } else {
            let position = |local: &str| drawing.children().find(|e| in_ns(e, &namespaces, local));
            let offset = |element: &Option<Element>| {
                element
                    .as_ref()
                    .and_then(|e| e.children().find(|c| c.local() == "posOffset"))
                    .map_or(0, |offset| emu(Some(offset.text().trim())))
            };
            let horizontal_element = position("positionH");
            let vertical_element = position("positionV");
            let horizontal = match horizontal_element
                .as_ref()
                .and_then(|e| e.attribute("", "relativeFrom"))
            {
                Some("page") => HorizontalFrom::Page,
                Some("column" | "character") => HorizontalFrom::Column,
                _ => HorizontalFrom::Margin,
            };
            let vertical = match vertical_element
                .as_ref()
                .and_then(|e| e.attribute("", "relativeFrom"))
            {
                Some("page") => VerticalFrom::Page,
                Some("margin" | "topMargin") => VerticalFrom::Margin,
                _ => VerticalFrom::Paragraph,
            };
            let wrap = drawing
                .children()
                .find_map(|e| {
                    if !namespaces.contains(&e.namespace()) {
                        return None;
                    }
                    match e.local() {
                        "wrapSquare" | "wrapTight" | "wrapThrough" => Some(ShapeWrap::Square),
                        "wrapTopAndBottom" => Some(ShapeWrap::TopAndBottom),
                        "wrapNone" => Some(ShapeWrap::None),
                        _ => None,
                    }
                })
                .unwrap_or(ShapeWrap::Square);
            Anchor::Floating {
                horizontal: (horizontal, offset(&horizontal_element)),
                vertical: (vertical, offset(&vertical_element)),
                wrap,
                behind: drawing.attribute("", "behindDoc") == Some("1"),
            }
        };
        let z = number(drawing.attribute("", "relativeHeight"))
            .unwrap_or(0)
            .clamp(0, 1 << 24) as u32;
        let graphic = drawing
            .descendants()
            .find(|e| in_ns(e, &[A, A_STRICT], "graphicData"))?;
        let uri = graphic.attribute("", "uri").unwrap_or("");
        let id = self.shape_id();
        let mut shape = Shape {
            id,
            name,
            anchor,
            width,
            height,
            z,
            line: None,
            fill: None,
            shadow: None,
            corner_radius: 0,
            content: ShapeContent::Empty,
        };
        if uri.ends_with("/picture") {
            let blip = graphic.descendants().find(|e| e.local() == "blip")?;
            let embed = blip
                .attribute(R, "embed")
                .or_else(|| blip.attribute(R_STRICT, "embed"))?;
            let image = self.image(embed)?;
            shape.content = ShapeContent::Picture(PlacedPicture {
                image,
                left: 0,
                top: 0,
                width,
                height,
                description,
            });
            return Some(shape);
        }
        if uri.ends_with("wordprocessingShape") {
            let wsp = graphic
                .children()
                .find(|e| e.namespace() == WPS && e.local() == "wsp")?;
            self.shape_look(&wsp, &mut shape);
            let text = wsp.descendants().find(|e| is_w(e, "txbxContent"));
            if let Some(text) = text {
                let blocks = self.blocks_of(&text);
                // A box holding only a picture (the app's frame larger
                // than its picture): a picture frame with the box's look.
                if let [Block::Paragraph(only)] = blocks.as_slice()
                    && let [Inline::Shape(inner)] = only.inlines.as_slice()
                    && let ShapeContent::Picture(picture) = &inner.content
                {
                    let body = wsp.children().find(|e| e.local() == "bodyPr");
                    let inset = |name: &str| {
                        body.as_ref()
                            .and_then(|b| number(b.attribute("", name)))
                            .unwrap_or(0)
                            / 635
                    };
                    shape.content = ShapeContent::Picture(PlacedPicture {
                        left: inset("lIns") as Twips,
                        top: inset("tIns") as Twips,
                        ..picture.clone()
                    });
                    return Some(shape);
                }
                let body = wsp.children().find(|e| e.local() == "bodyPr");
                let inset = |name: &str, default: i64| {
                    body.as_ref()
                        .and_then(|b| number(b.attribute("", name)))
                        .unwrap_or(default)
                        / 635
                };
                shape.content = ShapeContent::Text {
                    blocks,
                    insets: [
                        inset("tIns", 45_720) as Twips,
                        inset("rIns", 91_440) as Twips,
                        inset("bIns", 45_720) as Twips,
                        inset("lIns", 91_440) as Twips,
                    ],
                    columns: body
                        .as_ref()
                        .and_then(|b| number(b.attribute("", "numCol")))
                        .unwrap_or(1)
                        .clamp(1, 8) as u8,
                    column_gap: body
                        .as_ref()
                        .and_then(|b| number(b.attribute("", "spcCol")))
                        .map_or(0, |gap| (gap / 635) as Twips),
                };
            } else if shape.line.is_none() && shape.fill.is_none() {
                self.findings.count(FindingKind::Shape("drawn".into()));
                return None;
            }
            return Some(shape);
        }
        if uri.ends_with("/chart") {
            self.findings.count(FindingKind::Chart);
        } else if uri.ends_with("wordprocessingGroup") || uri.ends_with("wordprocessingCanvas") {
            self.findings.count(FindingKind::Shape("group".into()));
        } else {
            self.findings.count(FindingKind::Shape("drawn".into()));
        }
        // Keep the first picture the drawing holds, if any.
        let blip = graphic.descendants().find(|e| e.local() == "blip")?;
        let embed = blip.attribute(R, "embed")?;
        let image = self.image(embed)?;
        shape.content = ShapeContent::Picture(PlacedPicture {
            image,
            left: 0,
            top: 0,
            width,
            height,
            description,
        });
        Some(shape)
    }

    /// A DrawingML shape's line, fill, and corners.
    fn shape_look(&self, wsp: &Element, shape: &mut Shape) {
        let Some(properties) = wsp.children().find(|e| e.local() == "spPr") else {
            return;
        };
        for property in properties.children() {
            match property.local() {
                "solidFill" => shape.fill = self.drawing_colour(&property),
                "ln" => {
                    if property.children().any(|e| e.local() == "noFill") {
                        continue;
                    }
                    let width = number(property.attribute("", "w"))
                        .map_or(10, |w| (w / 635).max(1) as Twips);
                    let colour = property
                        .children()
                        .find(|e| e.local() == "solidFill")
                        .and_then(|fill| self.drawing_colour(&fill))
                        .unwrap_or(0);
                    let kind = match property.attribute("", "cmpd") {
                        Some("dbl") => LineKind::Double,
                        Some("tri") => LineKind::Triple,
                        Some("thickThin") => LineKind::ThickThin,
                        Some("thinThick") => LineKind::ThinThick,
                        _ => LineKind::Single,
                    };
                    shape.line = Some(ShapeLine {
                        kind,
                        width,
                        colour,
                        sides: [true; 4],
                    });
                }
                "prstGeom" if property.attribute("", "prst") == Some("roundRect") => {
                    // The adjustment is the radius in hundred-thousandths
                    // of the shorter side (one sixth when not given).
                    let adjust = property
                        .descendants()
                        .find(|guide| guide.local() == "gd")
                        .and_then(|guide| guide.attribute("", "fmla"))
                        .and_then(|formula| formula.strip_prefix("val "))
                        .and_then(|value| value.trim().parse::<i64>().ok())
                        .unwrap_or(16_667);
                    let shorter = i64::from(shape.width.min(shape.height));
                    shape.corner_radius = ((adjust * shorter + 50_000) / 100_000) as Twips;
                }
                _ => {}
            }
        }
        if let Some(shadow) = properties.descendants().find(|e| e.local() == "outerShdw") {
            let distance = number(shadow.attribute("", "dist")).unwrap_or(38_100) as f64 / 635.0;
            let direction =
                number(shadow.attribute("", "dir")).unwrap_or(2_700_000) as f64 / 60_000.0;
            let (dy, dx) = direction.to_radians().sin_cos();
            shape.shadow = Some(ShapeShadow {
                dx: (dx * distance).round() as Twips,
                dy: (dy * distance).round() as Twips,
                colour: self.drawing_colour(&shadow).unwrap_or(0x80_8080),
            });
        }
    }

    fn drawing_colour(&self, fill: &Element) -> Option<Colour> {
        let colour = fill.children().next()?;
        match colour.local() {
            "srgbClr" => colour.attribute("", "val").and_then(hex_colour),
            "schemeClr" => colour
                .attribute("", "val")
                .and_then(|theme| self.theme_colour(theme))
                .or(Some(0)),
            "sysClr" => colour.attribute("", "lastClr").and_then(hex_colour),
            _ => None,
        }
    }

    /// An older VML shape (`w:pict`): a text box or a picture.
    fn vml(&mut self, shape: &Element) -> Option<Shape> {
        let style = shape.attribute("", "style").unwrap_or("");
        let mut values: BTreeMap<String, String> = BTreeMap::new();
        for pair in style.split(';') {
            if let Some((name, value)) = pair.split_once(':') {
                values.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
            }
        }
        let length = |name: &str| values.get(name).and_then(|value| css_twips(value));
        let width = length("width").unwrap_or(1_440).max(20);
        let height = length("height").unwrap_or(720).max(20);
        let absolute = values
            .get("position")
            .is_some_and(|position| position == "absolute");
        let horizontal = match values
            .get("mso-position-horizontal-relative")
            .map(String::as_str)
        {
            Some("page") => HorizontalFrom::Page,
            Some("margin") => HorizontalFrom::Margin,
            _ => HorizontalFrom::Column,
        };
        let vertical = match values
            .get("mso-position-vertical-relative")
            .map(String::as_str)
        {
            Some("page") => VerticalFrom::Page,
            Some("margin") => VerticalFrom::Margin,
            _ => VerticalFrom::Paragraph,
        };
        let anchor = if absolute {
            Anchor::Floating {
                horizontal: (
                    horizontal,
                    length("margin-left")
                        .or_else(|| length("left"))
                        .unwrap_or(0),
                ),
                vertical: (
                    vertical,
                    length("margin-top").or_else(|| length("top")).unwrap_or(0),
                ),
                wrap: ShapeWrap::Square,
                behind: values.get("z-index").is_some_and(|z| z.starts_with('-')),
            }
        } else {
            Anchor::Inline { drop: 0 }
        };
        let id = self.shape_id();
        let mut made = Shape {
            id,
            name: shape.attribute("", "id").unwrap_or("").to_owned(),
            anchor,
            width,
            height,
            z: id,
            line: None,
            fill: None,
            shadow: None,
            corner_radius: 0,
            content: ShapeContent::Empty,
        };
        if shape.attribute("", "stroked") != Some("f") && shape.local() != "shape" {
            made.line = Some(ShapeLine {
                kind: LineKind::Single,
                width: 10,
                colour: shape
                    .attribute("", "strokecolor")
                    .and_then(css_colour)
                    .unwrap_or(0),
                sides: [true; 4],
            });
        }
        if let Some(fill) = shape.attribute("", "fillcolor").and_then(css_colour) {
            made.fill = Some(fill);
        }
        if let Some(text) = shape.descendants().find(|e| is_w(e, "txbxContent")) {
            made.content = ShapeContent::Text {
                blocks: self.blocks_of(&text),
                insets: [72, 144, 72, 144],
                columns: 1,
                column_gap: 0,
            };
            return Some(made);
        }
        if let Some(image) = shape
            .descendants()
            .find(|e| e.namespace() == V && e.local() == "imagedata")
        {
            let id = image
                .attribute(R, "id")
                .or_else(|| image.attribute("urn:schemas-microsoft-com:office:office", "relid"))?;
            let index = self.image(id)?;
            made.content = ShapeContent::Picture(PlacedPicture {
                image: index,
                left: 0,
                top: 0,
                width,
                height,
                description: image
                    .attribute("urn:schemas-microsoft-com:office:office", "title")
                    .unwrap_or("")
                    .to_owned(),
            });
            return Some(made);
        }
        self.findings.count(FindingKind::Shape("drawn".into()));
        None
    }

    fn finish(mut self) -> ExchangeDocument {
        if self.sections.is_empty() {
            let section = self.default_section(Vec::new());
            self.sections.push(section);
        }
        // Each paragraph style, every field set.
        let mut styles = Vec::new();
        for id in &self.style_order {
            let (run, para) = self.paragraph_style(id);
            let definition = self.style(id);
            let name = definition.map_or_else(|| "Normal".to_owned(), |style| style.name.clone());
            let mut props = complete_paragraph(&para.props, self.tab_default);
            props.column_break_before = Some(false);
            let index = |other: &Option<String>| {
                other
                    .as_ref()
                    .and_then(|other| self.style_order.iter().position(|known| known == other))
            };
            styles.push(Style {
                name,
                based_on: definition.and_then(|style| index(&style.based_on)),
                next: definition.and_then(|style| index(&style.next)),
                paragraph: props,
                character: complete_run(&run),
                outline_level: para.outline,
                list: self
                    .numbering_of(&para)
                    .map(|(instance, level)| (self.list_instances[instance].list, level)),
            });
        }
        ExchangeDocument {
            properties: std::mem::take(&mut self.properties),
            fonts: std::mem::take(&mut self.fonts),
            styles,
            lists: std::mem::take(&mut self.lists),
            list_instances: std::mem::take(&mut self.list_instances),
            sections: std::mem::take(&mut self.sections),
            images: std::mem::take(&mut self.images),
            settings: self.settings.clone(),
        }
    }
}

/// The branch of an `mc:AlternateContent` to read: a choice whose
/// requirements are known (WordprocessingShape, DrawingML), else the
/// fallback.
fn alternate<'t>(element: &Element<'t>) -> Option<Element<'t>> {
    for choice in element.children() {
        if choice.namespace() == MC && choice.local() == "Choice" {
            let requires = choice.attribute("", "Requires").unwrap_or("");
            if requires
                .split_whitespace()
                .all(|prefix| matches!(prefix, "wps" | "wpg" | "a14" | "w14" | "w15" | "wp14"))
            {
                return Some(choice);
            }
        }
    }
    element
        .children()
        .find(|fallback| fallback.namespace() == MC && fallback.local() == "Fallback")
}

/// What a paragraph sets itself (its style's are the style's).
fn own_props(para: &Para) -> ParagraphProps {
    let mut props = para.props.clone();
    if let Some(tabs) = &mut props.tabs {
        tabs.retain(|tab| tab.position >= 0);
    }
    props
}

/// Paragraph formatting with every field set: Word's defaults where the
/// style says nothing, the default stops where it has none.
fn complete_paragraph(props: &ParagraphProps, tab_default: Twips) -> ParagraphProps {
    let mut tabs = props.tabs.clone().unwrap_or_default();
    tabs.retain(|tab| tab.position >= 0);
    if tabs.is_empty() {
        let mut position = tab_default;
        while position <= 8_640 && tabs.len() < 22 {
            tabs.push(Tab {
                position,
                kind: TabKind::Left,
                leader: TabLeader::None,
            });
            position += tab_default;
        }
    }
    ParagraphProps {
        alignment: Some(props.alignment.unwrap_or(Alignment::Left)),
        indent_left: Some(props.indent_left.unwrap_or(0)),
        indent_right: Some(props.indent_right.unwrap_or(0)),
        first_line: Some(props.first_line.unwrap_or(0)),
        space_before: Some(props.space_before.unwrap_or(0)),
        space_after: Some(props.space_after.unwrap_or(0)),
        line: Some(props.line.unwrap_or(LineSpacing::Single)),
        keep_with_next: Some(props.keep_with_next.unwrap_or(false)),
        keep_together: Some(props.keep_together.unwrap_or(false)),
        page_break_before: Some(props.page_break_before.unwrap_or(false)),
        column_break_before: Some(false),
        tabs: Some(tabs),
        border_top: Some(props.border_top.flatten()),
        border_bottom: Some(props.border_bottom.flatten()),
        no_hyphenation: Some(props.no_hyphenation.unwrap_or(false)),
    }
}

/// Character formatting with every field set (Word's defaults: Times New
/// Roman, 10 point).
fn complete_run(run: &Run) -> CharProps {
    CharProps {
        font: Some(run.font.clone().unwrap_or_else(|| "Times New Roman".into())),
        size: Some(run.size.map_or(200, |size| (size * 10) as Twips)),
        bold: Some(run.bold.unwrap_or(false)),
        italic: Some(run.italic.unwrap_or(false)),
        underline: Some(run.underline.unwrap_or(Underline::None)),
        strike: Some(run.strike.unwrap_or(false)),
        script: Some(run.script.unwrap_or(Script::Normal)),
        caps: Some(run.caps.unwrap_or(false)),
        small_caps: Some(run.small_caps.unwrap_or(false)),
        colour: Some(run.colour.unwrap_or(0)),
    }
}

fn frame_pr(element: &Element) -> FramePr {
    let horizontal = match attr(element, "hAnchor") {
        Some("page") => Some(HorizontalFrom::Page),
        Some("margin") => Some(HorizontalFrom::Margin),
        Some(_) => Some(HorizontalFrom::Column),
        None => None,
    };
    let vertical = match attr(element, "vAnchor") {
        Some("page") => Some(VerticalFrom::Page),
        Some("margin") => Some(VerticalFrom::Margin),
        Some(_) => Some(VerticalFrom::Paragraph),
        None => None,
    };
    FramePr {
        width: twips_of(attr(element, "w")).unwrap_or(0),
        height: twips_of(attr(element, "h")).unwrap_or(0).abs(),
        x: twips_of(attr(element, "x")).unwrap_or(0),
        y: twips_of(attr(element, "y")).unwrap_or(0),
        horizontal,
        vertical,
        wrap: match attr(element, "wrap") {
            Some("notBeside") => ShapeWrap::TopAndBottom,
            Some("none") => ShapeWrap::None,
            _ => ShapeWrap::Square,
        },
    }
}

/// Word's built-in style names as Word shows them ("heading 1" is Heading
/// 1).
fn built_in_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if let Some(level) = lower.strip_prefix("heading ") {
        return format!("Heading {level}");
    }
    match lower.as_str() {
        "title" => "Title".into(),
        "subtitle" => "Subtitle".into(),
        "caption" => "Caption".into(),
        "header" => "Header".into(),
        "footer" => "Footer".into(),
        "footnote text" => "Footnote Text".into(),
        "toc 1" | "toc 2" | "toc 3" | "toc 4" | "toc 5" | "toc 6" | "toc 7" | "toc 8" | "toc 9" => {
            format!("TOC {}", &lower[4..])
        }
        _ => name.to_owned(),
    }
}

/// Word's bullet characters (Symbol and Wingdings in the private area) as
/// the nearest Unicode.
fn word_bullet(character: char) -> char {
    match character as u32 {
        0xF0B7 | 0xF0A7 | 0xF06C => '\u{2022}',
        0xF0FC => '\u{2713}',
        0xF0D8 | 0xF0E0 => '\u{27A2}',
        0xF06E => '\u{25A0}',
        0xF071 => '\u{25A1}',
        0xF0A8 => '\u{25E6}',
        _ => character,
    }
}

fn hex_colour(value: &str) -> Option<Colour> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 {
        return None;
    }
    let number = u32::from_str_radix(value, 16).ok()?;
    let (red, green, blue) = (number >> 16 & 0xFF, number >> 8 & 0xFF, number & 0xFF);
    Some(red | green << 8 | blue << 16)
}

/// A CSS colour (`#rrggbb`, `#rgb`, or a few names).
fn css_colour(value: &str) -> Option<Colour> {
    let value = value.split_whitespace().next()?;
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 3 {
            let expanded: String = hex.chars().flat_map(|c| [c, c]).collect();
            return hex_colour(&expanded);
        }
        return hex_colour(hex);
    }
    Some(match value {
        "black" => 0,
        "white" => 0xFF_FFFF,
        "red" => 0x0000FF,
        "green" => 0x008000,
        "blue" => 0xFF0000,
        "yellow" => 0x00FFFF,
        "gray" | "grey" => 0x80_8080,
        _ => return None,
    })
}

/// A CSS length in twips.
fn css_twips(value: &str) -> Option<Twips> {
    let value = value.trim();
    let split = value
        .find(|c: char| c.is_alphabetic())
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(split);
    let number: f64 = number.trim().parse().ok()?;
    Some(match unit {
        "pt" | "" => number * 20.0,
        "in" => number * 1440.0,
        "cm" => number * 567.0,
        "mm" => number * 56.7,
        "px" => number * 15.0,
        _ => number * 20.0,
    } as Twips)
}

/// A W3C date and time (`2026-10-21T19:13:00Z`).
fn w3c_date(text: &str) -> Option<amipro_document::WallClock> {
    let text = text.trim();
    let digits = |range: std::ops::Range<usize>| text.get(range)?.parse::<i32>().ok();
    Some(amipro_document::WallClock {
        year: digits(0..4)?,
        month: digits(5..7)?.clamp(1, 12) as u8,
        day: digits(8..10)?.clamp(1, 31) as u8,
        hour: digits(11..13).unwrap_or(0).clamp(0, 23) as u8,
        minute: digits(14..16).unwrap_or(0).clamp(0, 59) as u8,
        second: digits(17..19).unwrap_or(0).clamp(0, 59) as u8,
    })
}

/// A field code's words, quoted ones whole.
fn split_code(code: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in code.chars() {
        match character {
            '"' => {
                if quoted {
                    words.push(std::mem::take(&mut current));
                }
                quoted = !quoted;
            }
            character if character.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            character => current.push(character),
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn letter_page() -> PageSetup {
    PageSetup {
        width: 12_240,
        height: 15_840,
        landscape: false,
        margin_top: 1_440,
        margin_bottom: 1_440,
        margin_left: 1_440,
        margin_right: 1_440,
        header_distance: 720,
        footer_distance: 720,
        columns: Columns {
            count: 1,
            gap: 720,
            widths: None,
            line_between: false,
        },
        border: None,
    }
}

/// An image part's kind, by its bytes and then its name.
pub(crate) fn image_kind(name: &str, bytes: &[u8]) -> Option<ImageKind> {
    Some(match bytes {
        [0x89, b'P', b'N', b'G', ..] => ImageKind::Png,
        [0xFF, 0xD8, ..] => ImageKind::Jpeg,
        [b'G', b'I', b'F', ..] => ImageKind::Gif,
        [b'B', b'M', ..] => ImageKind::Bmp,
        [0xD7, 0xCD, 0xC6, 0x9A, ..] => ImageKind::Wmf,
        [0x01, 0, 0, 0, ..] if bytes.get(40..44) == Some(b" EMF") => ImageKind::Emf,
        [b'I', b'I', 42, 0, ..] | [b'M', b'M', 0, 42, ..] => ImageKind::Tiff,
        _ => {
            // By its name where its bytes do not say.
            let lower = name.to_ascii_lowercase();
            let extension = lower.rsplit('.').next().unwrap_or("");
            match extension {
                "svg" => ImageKind::Svg,
                "wmf" => ImageKind::Wmf,
                "emf" => ImageKind::Emf,
                "png" => ImageKind::Png,
                "jpg" | "jpeg" => ImageKind::Jpeg,
                "gif" => ImageKind::Gif,
                "bmp" | "dib" => ImageKind::Bmp,
                "tif" | "tiff" => ImageKind::Tiff,
                _ => return None,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::{Entry, write};

    fn no_inflate(_: &[u8], _: usize) -> Option<Vec<u8>> {
        None
    }

    fn package(document: &str, styles: &str) -> Vec<u8> {
        write(
            &[
                Entry::new(
                    "_rels/.rels",
                    r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
                ),
                Entry::new(
                    "word/_rels/document.xml.rels",
                    r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="s" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#,
                ),
                Entry::new("word/document.xml", document),
                Entry::new("word/styles.xml", styles),
            ],
            None,
        )
    }

    #[test]
    fn styles_runs_toggles_and_tables() {
        let styles = format!(
            r#"<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri"/><w:sz w:val="22"/></w:rPr></w:rPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="240"/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style>
<w:style w:type="character" w:styleId="Strong"><w:name w:val="Strong"/><w:rPr><w:b/></w:rPr></w:style></w:styles>"#
        );
        let document = format!(
            r#"<w:document xmlns:w="{W}"><w:body>
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Head </w:t></w:r><w:r><w:rPr><w:rStyle w:val="Strong"/></w:rPr><w:t>not bold</w:t></w:r></w:p>
<w:p><w:r><w:t xml:space="preserve">Body </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText>PAGE</w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>4</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>
<w:tbl><w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid><w:tr><w:tc><w:tcPr><w:gridSpan w:val="2"/></w:tcPr><w:p><w:r><w:t>both</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p/></w:tc><w:tc><w:p/></w:tc></w:tr></w:tbl>
<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1134" w:bottom="1440" w:left="1134" w:header="709" w:footer="709" w:gutter="0"/></w:sectPr></w:body></w:document>"#
        );
        let bytes = package(&document, &styles);
        let mut findings = Findings::default();
        let read = read_docx(&bytes, &no_inflate, &mut findings).unwrap();
        assert_eq!(read.styles[0].name, "Normal");
        assert_eq!(read.styles[1].name, "Heading 1");
        assert_eq!(read.styles[1].character.size, Some(320));
        assert_eq!(read.styles[1].character.font.as_deref(), Some("Calibri"));
        let section = &read.sections[0];
        assert_eq!(section.page.width, 11_906);
        let Block::Paragraph(heading) = &section.blocks[0] else {
            panic!("a paragraph")
        };
        assert_eq!(heading.style, 1);
        let strong = heading.inlines.iter().find_map(|inline| match inline {
            Inline::Text { text, props } if text == "not bold" => Some(props.bold),
            _ => None,
        });
        // Strong's bold turns the heading's off.
        assert_eq!(strong, Some(Some(false)));
        let Block::Paragraph(body) = &section.blocks[1] else {
            panic!("a paragraph")
        };
        assert!(body.inlines.iter().any(|inline| matches!(inline, Inline::Field(Field { kind: FieldKind::Page { .. }, result, .. }) if result == "4")));
        let Block::Table(table) = &section.blocks[2] else {
            panic!("a table")
        };
        assert_eq!(table.rows[0].cells[0].span, 2);
        assert!(
            findings
                .iter()
                .any(|f| matches!(f.kind, FindingKind::CharacterStyle(_)))
        );
    }

    #[test]
    fn damaged_packages_are_refused() {
        let mut findings = Findings::default();
        assert!(read_docx(b"PK\x03\x04", &no_inflate, &mut findings).is_err());
        let bytes = package("<w:document/>", "<w:styles/>");
        assert!(read_docx(&bytes, &no_inflate, &mut findings).is_err());
    }
}
