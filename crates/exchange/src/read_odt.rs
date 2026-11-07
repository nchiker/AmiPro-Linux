//! OpenDocument Text (ISO/IEC 26300; `.odt` packages and flat `.fodt`)
//! read into an exchange document (`docs/research/IMPORT_NOTES.md`).
//!
//! Styles come from `styles.xml` and the automatic styles of
//! `content.xml`, resolved through their parents and the family's default
//! style; a paragraph's automatic style is its named parent with what the
//! automatic one sets over it. Master pages and their page layouts give
//! the sections, a paragraph style's master page starting a new one; the
//! page's margins in ODF reach the header and footer, so the text's margin
//! is the margin, the band, and its spacing. Lists are nested `text:list`s
//! counted by their list style, continuing the list their `continue-list`
//! names; frames are `draw:frame`s placed by their graphic style; tables
//! are read onto their grid with their spans and covered cells. Tracked
//! changes are read as the document stands: the body already holds the
//! inserted text and leaves the deleted text out.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::import::{FindingKind, Findings, ImportError};
use crate::xml_tree::{Content, Element, Tree};
use crate::zip::{Archive, Inflate};
use crate::{
    Alignment, Anchor, Block, Cell, CharProps, Colour, Columns, DateSource, ExchangeDocument,
    Field, FieldKind, Font, FontFamily, Footnote, HeaderFooters, HorizontalFrom, Image, ImageKind,
    Inline, LineKind, LineSpacing, List, ListInstance, ListLevel, Note, NumberFormat, PageBorder,
    PageNumbering, PageSetup, Paragraph, ParagraphProps, PlacedPicture, Properties, Row, RowHeight,
    RuleLine, Script, Section, Settings, Shape, ShapeContent, ShapeLine, ShapeShadow, ShapeWrap,
    Style, Tab, TabKind, TabLeader, Table, Twips, Underline, VerticalFrom, VerticalMerge,
};

const OFFICE: &str = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";
const STYLE: &str = "urn:oasis:names:tc:opendocument:xmlns:style:1.0";
const TEXT: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const TABLE: &str = "urn:oasis:names:tc:opendocument:xmlns:table:1.0";
const DRAW: &str = "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";
const FO: &str = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0";
const SVG: &str = "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0";
const NUMBER: &str = "urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0";
const XLINK: &str = "http://www.w3.org/1999/xlink";
const DC: &str = "http://purl.org/dc/elements/1.1/";
const META: &str = "urn:oasis:names:tc:opendocument:xmlns:meta:1.0";
const MANIFEST: &str = "urn:oasis:names:tc:opendocument:xmlns:manifest:1.0";
const XML: &str = "http://www.w3.org/XML/1998/namespace";

/// The deepest nesting of lists, tables, frames, and notes read.
const MAX_DEPTH: usize = 64;

type Pictures<'p> = dyn FnMut(&str) -> Option<Vec<u8>> + 'p;

/// A package (`.odt`) as an exchange document.
pub fn read_odt(
    bytes: &[u8],
    inflate: Inflate,
    findings: &mut Findings,
) -> Result<ExchangeDocument, ImportError> {
    let archive = Archive::open(bytes)?;
    let part = |name: &str| archive.read(name, inflate).ok();
    if let Some(manifest) = part("META-INF/manifest.xml")
        && let Ok(tree) = Tree::parse(&manifest)
        && tree
            .root()
            .descendants()
            .any(|element| element.is(MANIFEST, "encryption-data"))
    {
        return Err(ImportError::Encrypted);
    }
    let content = part("content.xml")
        .ok_or_else(|| ImportError::Damaged("the content part is missing".into()))?;
    let content = Tree::parse(&content)?;
    let styles = part("styles.xml").and_then(|bytes| Tree::parse(&bytes).ok());
    let meta = part("meta.xml").and_then(|bytes| Tree::parse(&bytes).ok());
    let mut pictures = |name: &str| {
        let name = name.trim_start_matches("./");
        archive.read(name, inflate).ok()
    };
    let mut reader = Reader::new(findings);
    if let Some(styles) = &styles {
        reader.read_styles(styles.root(), false, &mut pictures);
    }
    reader.read_styles(content.root(), true, &mut pictures);
    if let Some(meta) = &meta {
        reader.read_meta(meta.root());
    }
    reader.read_body(content.root(), &mut pictures);
    Ok(reader.finish())
}

/// A flat OpenDocument file (`.fodt`) as an exchange document.
pub fn read_fodt(bytes: &[u8], findings: &mut Findings) -> Result<ExchangeDocument, ImportError> {
    let tree = Tree::parse(bytes)?;
    let root = tree.root();
    if !root.is(OFFICE, "document") {
        return Err(ImportError::Damaged("not an OpenDocument file".into()));
    }
    // Pictures in a flat file are in the file (`office:binary-data`).
    let mut pictures = |_: &str| None;
    let mut reader = Reader::new(findings);
    reader.read_styles(root, false, &mut pictures);
    reader.read_meta(root);
    reader.read_body(root, &mut pictures);
    Ok(reader.finish())
}

/// A length (`1.5cm`, `12pt`, `0.25in`, `3mm`) in twips.
fn length(value: Option<&str>) -> Option<Twips> {
    let value = value?.trim();
    let split = value
        .find(|c: char| c.is_ascii_alphabetic() || c == '%')
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(split);
    let number: f64 = number.trim().parse().ok()?;
    let twips = match unit {
        "pt" => number * 20.0,
        "in" | "inch" => number * 1440.0,
        "cm" => number * 1440.0 / 2.54,
        "mm" => number * 144.0 / 2.54,
        "pc" => number * 240.0,
        "px" => number * 15.0,
        "" => number,
        _ => return None,
    };
    twips
        .is_finite()
        .then(|| twips.round().clamp(-1_000_000.0, 1_000_000.0) as Twips)
}

/// `#rrggbb` as a colour (`0x00BBGGRR`).
fn colour(value: Option<&str>) -> Option<Colour> {
    let value = value?.trim().strip_prefix('#')?;
    if value.len() != 6 {
        return None;
    }
    let number = u32::from_str_radix(value, 16).ok()?;
    Some((number >> 16 & 0xFF) | (number >> 8 & 0xFF) << 8 | (number & 0xFF) << 16)
}

/// A border (`0.5pt solid #000000`) as a rule.
fn border(value: Option<&str>) -> Option<RuleLine> {
    let value = value?.trim();
    let mut line = RuleLine {
        kind: LineKind::Single,
        width: 10,
        colour: 0,
        space: 0,
    };
    let mut drawn = false;
    for part in value.split_whitespace() {
        if part.starts_with('#') {
            line.colour = colour(Some(part)).unwrap_or(0);
        } else if matches!(part, "none" | "hidden") {
            return None;
        } else if part == "double" {
            line.kind = LineKind::Double;
            drawn = true;
        } else if part.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            // solid, dotted, dashed, groove, ridge, inset, outset
            drawn = true;
        } else if let Some(width) = length(Some(part)) {
            line.width = width.max(1);
        }
    }
    drawn.then_some(line)
}

/// A style's name as shown: `_20_` and the like as their characters.
fn shown_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(at) = rest.find('_') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let code = after
            .find('_')
            .filter(|end| (1..=6).contains(end))
            .and_then(|end| Some((end, u32::from_str_radix(&after[..end], 16).ok()?)))
            .and_then(|(end, code)| Some((end, char::from_u32(code)?)));
        match code {
            Some((end, character)) => {
                out.push(character);
                rest = &after[end + 1..];
            }
            None => {
                out.push('_');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[derive(Clone, Debug, Default)]
struct StyleEntry {
    family: String,
    display: String,
    parent: Option<String>,
    next: Option<String>,
    list_style: Option<String>,
    master_page: Option<String>,
    page_number: Option<u32>,
    /// A break after the paragraph (`page` or `column`).
    break_after: Option<String>,
    outline: Option<u8>,
    paragraph: ParagraphProps,
    text: Text,
    /// Borders on the sides Ami Pro has no rule for, or shading.
    side_borders: bool,
    /// Graphic, table, column, row, and cell properties by name.
    other: BTreeMap<String, String>,
    automatic: bool,
}

/// Character formatting as read, with a relative size.
#[derive(Clone, Debug, Default, PartialEq)]
struct Text {
    props: CharProps,
    /// A size relative to the parent's, in percent.
    relative: Option<f64>,
    hidden: bool,
    highlight: bool,
    spacing: bool,
}

impl Text {
    fn over(&mut self, other: &Text) {
        let size = match (other.props.size, other.relative) {
            (Some(size), _) => Some(size),
            (None, Some(percent)) => self
                .props
                .size
                .map(|size| (f64::from(size) * percent / 100.0).round() as Twips),
            (None, None) => self.props.size,
        };
        self.props.overlay(&other.props);
        self.props.size = size;
        self.hidden |= other.hidden;
        self.highlight |= other.highlight;
        self.spacing |= other.spacing;
    }
}

fn overlay_paragraph(target: &mut ParagraphProps, over: &ParagraphProps) {
    fn take<T: Clone>(target: &mut Option<T>, value: &Option<T>) {
        if value.is_some() {
            target.clone_from(value);
        }
    }
    take(&mut target.alignment, &over.alignment);
    take(&mut target.indent_left, &over.indent_left);
    take(&mut target.indent_right, &over.indent_right);
    take(&mut target.first_line, &over.first_line);
    take(&mut target.space_before, &over.space_before);
    take(&mut target.space_after, &over.space_after);
    take(&mut target.line, &over.line);
    take(&mut target.keep_with_next, &over.keep_with_next);
    take(&mut target.keep_together, &over.keep_together);
    take(&mut target.page_break_before, &over.page_break_before);
    take(&mut target.column_break_before, &over.column_break_before);
    take(&mut target.tabs, &over.tabs);
    take(&mut target.border_top, &over.border_top);
    take(&mut target.border_bottom, &over.border_bottom);
    take(&mut target.no_hyphenation, &over.no_hyphenation);
}

/// A master page: its page layout and its headers and footers.
#[derive(Clone, Default)]
struct Master {
    layout: String,
    headers: HeaderFooters,
    footers: HeaderFooters,
}

#[derive(Clone)]
struct LayoutEntry {
    page: PageSetup,
    /// The header's height and its spacing to the text.
    header: Option<(Twips, Twips)>,
    footer: Option<(Twips, Twips)>,
    format: Option<NumberFormat>,
    mirrored: bool,
}

impl Default for LayoutEntry {
    fn default() -> Self {
        Self {
            page: PageSetup {
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
            },
            header: None,
            footer: None,
            format: None,
            mirrored: false,
        }
    }
}

/// A list paragraph's place: its instance and level.
#[derive(Clone, Copy)]
struct ListPlace {
    instance: usize,
    level: u8,
}

/// An index or contents mark whose text is the text up to its end mark.
struct OpenMark {
    id: String,
    /// Where in the paragraph's inlines its field stands.
    at: usize,
}

struct Reader<'f> {
    findings: &'f mut Findings,
    styles: BTreeMap<String, StyleEntry>,
    /// The named paragraph styles in the file's order.
    order: Vec<String>,
    defaults: BTreeMap<String, StyleEntry>,
    list_styles: BTreeMap<String, usize>,
    lists: Vec<List>,
    instances: Vec<ListInstance>,
    /// Each `text:list`'s instance by its `xml:id`, and each list style's
    /// last instance.
    list_ids: BTreeMap<String, usize>,
    last_instance: BTreeMap<String, usize>,
    layouts: BTreeMap<String, LayoutEntry>,
    masters: BTreeMap<String, Master>,
    master_order: Vec<String>,
    data_styles: BTreeMap<String, String>,
    /// Tracked changes by id: insertions, deletions, format changes.
    changes: BTreeMap<String, FindingKind>,
    fonts: Vec<Font>,
    font_names: BTreeMap<String, String>,
    properties: Properties,
    images: Vec<Image>,
    image_names: BTreeMap<String, usize>,
    sections: Vec<Section>,
    settings: Settings,
    next_shape: u32,
    tab_default: Twips,
    depth: usize,
    /// White space: the last character given was a space (or the
    /// paragraph has none yet), so the next is collapsed.
    space: bool,
    /// The paragraph styles paragraphs name, by slot.
    referenced: Vec<String>,
    referenced_slots: BTreeMap<String, usize>,
    used: BTreeSet<String>,
    /// Frames anchored to the page, waiting for the next paragraph.
    pending_shapes: Vec<Inline>,
    open_marks: Vec<OpenMark>,
    /// The page the body's frames are placed on (for centred frames).
    page: PageSetup,
}

impl<'f> Reader<'f> {
    fn new(findings: &'f mut Findings) -> Self {
        Self {
            findings,
            styles: BTreeMap::new(),
            order: Vec::new(),
            defaults: BTreeMap::new(),
            list_styles: BTreeMap::new(),
            lists: Vec::new(),
            instances: Vec::new(),
            list_ids: BTreeMap::new(),
            last_instance: BTreeMap::new(),
            layouts: BTreeMap::new(),
            masters: BTreeMap::new(),
            master_order: Vec::new(),
            data_styles: BTreeMap::new(),
            changes: BTreeMap::new(),
            fonts: Vec::new(),
            font_names: BTreeMap::new(),
            properties: Properties::default(),
            images: Vec::new(),
            image_names: BTreeMap::new(),
            sections: Vec::new(),
            settings: Settings::default(),
            next_shape: 1,
            tab_default: 720,
            depth: 0,
            space: true,
            referenced: Vec::new(),
            referenced_slots: BTreeMap::new(),
            used: BTreeSet::new(),
            pending_shapes: Vec::new(),
            open_marks: Vec::new(),
            page: LayoutEntry::default().page,
        }
    }

    // ---- Styles ----

    /// The styles under `root` (a styles or content part, or a flat file):
    /// `content` reads only the content part's automatic styles and fonts.
    fn read_styles(&mut self, root: Element<'_>, content: bool, pictures: &mut Pictures<'_>) {
        for group in root.children() {
            if group.namespace() != OFFICE {
                continue;
            }
            match group.local() {
                "font-face-decls" => self.font_faces(&group),
                "styles" if !content => self.style_group(&group, false),
                "automatic-styles" => self.style_group(&group, true),
                "master-styles" if !content => {
                    for master in group.children().filter(|e| e.is(STYLE, "master-page")) {
                        self.master_page(&master, pictures);
                    }
                }
                _ => {}
            }
        }
    }

    fn font_faces(&mut self, group: &Element<'_>) {
        for face in group.children().filter(|e| e.is(STYLE, "font-face")) {
            let Some(name) = face.attribute(STYLE, "name") else {
                continue;
            };
            let family = face
                .attribute(SVG, "font-family")
                .unwrap_or(name)
                .trim()
                .trim_matches(['\'', '"'])
                .to_owned();
            if family.is_empty() {
                continue;
            }
            self.font_names.insert(name.to_owned(), family.clone());
            if !self.fonts.iter().any(|font| font.name == family) {
                self.fonts.push(Font {
                    name: family,
                    family: match face.attribute(STYLE, "font-family-generic") {
                        Some("roman") => FontFamily::Roman,
                        Some("swiss") => FontFamily::Swiss,
                        Some("modern") => FontFamily::Modern,
                        Some("script") => FontFamily::Script,
                        Some("decorative") => FontFamily::Decorative,
                        _ => FontFamily::Unknown,
                    },
                    fixed: face.attribute(STYLE, "font-pitch") == Some("fixed"),
                });
            }
        }
    }

    fn style_group(&mut self, group: &Element<'_>, automatic: bool) {
        for element in group.children() {
            match (element.namespace(), element.local()) {
                (STYLE, "style") => {
                    let Some(name) = element.attribute(STYLE, "name") else {
                        continue;
                    };
                    let entry = self.style_entry(&element, automatic);
                    if !automatic
                        && entry.family == "paragraph"
                        && !self.order.iter().any(|known| known == name)
                    {
                        self.order.push(name.to_owned());
                    }
                    self.styles.insert(name.to_owned(), entry);
                }
                (STYLE, "default-style") => {
                    let entry = self.style_entry(&element, false);
                    if let Some(distance) = element
                        .child(STYLE, "paragraph-properties")
                        .and_then(|p| length(p.attribute(STYLE, "tab-stop-distance")))
                        .filter(|distance| *distance > 0)
                    {
                        self.tab_default = distance;
                    }
                    self.defaults.insert(entry.family.clone(), entry);
                }
                (TEXT, "list-style") => self.list_style(&element),
                (STYLE, "page-layout") => self.page_layout(&element),
                (NUMBER, "date-style" | "time-style") => {
                    if let Some(name) = element.attribute(STYLE, "name") {
                        self.data_styles
                            .insert(name.to_owned(), data_style_picture(&element));
                    }
                }
                _ => {}
            }
        }
    }

    fn style_entry(&self, element: &Element<'_>, automatic: bool) -> StyleEntry {
        let name = element.attribute(STYLE, "name").unwrap_or("");
        let mut entry = StyleEntry {
            family: element
                .attribute(STYLE, "family")
                .unwrap_or("paragraph")
                .to_owned(),
            display: element
                .attribute(STYLE, "display-name")
                .map_or_else(|| shown_name(name), str::to_owned),
            parent: element
                .attribute(STYLE, "parent-style-name")
                .map(str::to_owned),
            next: element
                .attribute(STYLE, "next-style-name")
                .map(str::to_owned),
            list_style: element
                .attribute(STYLE, "list-style-name")
                .filter(|list| !list.is_empty())
                .map(str::to_owned),
            master_page: element
                .attribute(STYLE, "master-page-name")
                .filter(|master| !master.is_empty())
                .map(str::to_owned),
            outline: element
                .attribute(STYLE, "default-outline-level")
                .and_then(|level| level.parse::<u8>().ok())
                .filter(|level| (1..=9).contains(level)),
            automatic,
            ..StyleEntry::default()
        };
        for properties in element.children() {
            match properties.local() {
                "paragraph-properties" => {
                    entry.paragraph = self.paragraph_props(&properties);
                    entry.page_number = properties
                        .attribute(STYLE, "page-number")
                        .and_then(|number| number.parse().ok());
                    entry.break_after = properties.attribute(FO, "break-after").map(str::to_owned);
                    let shaded = properties
                        .attribute(FO, "background-color")
                        .is_some_and(|value| value != "transparent");
                    let sides = ["border-left", "border-right"].iter().any(|side| {
                        border(
                            properties
                                .attribute(FO, side)
                                .or(properties.attribute(FO, "border")),
                        )
                        .is_some()
                    });
                    entry.side_borders = shaded || sides;
                }
                "text-properties" => {
                    entry.text = self.text_props(&properties);
                    match properties.attribute(FO, "hyphenate") {
                        Some("true") => entry.paragraph.no_hyphenation = Some(false),
                        Some(_) => entry.paragraph.no_hyphenation = Some(true),
                        None => {}
                    }
                }
                _ => {
                    for (_, local, value) in properties.attributes() {
                        entry.other.insert(local.to_owned(), value.to_owned());
                    }
                    // Columns of a frame or section.
                    if let Some(columns) = properties.child(STYLE, "columns")
                        && let Some(count) = columns.attribute(FO, "column-count")
                    {
                        entry.other.insert("column-count".into(), count.to_owned());
                        if let Some(gap) = columns.attribute(FO, "column-gap") {
                            entry.other.insert("column-gap".into(), gap.to_owned());
                        }
                    }
                }
            }
        }
        entry
    }

    fn paragraph_props(&self, element: &Element<'_>) -> ParagraphProps {
        let mut props = ParagraphProps::default();
        if let Some(align) = element.attribute(FO, "text-align") {
            props.alignment = Some(match align {
                "center" => Alignment::Centre,
                "end" | "right" => Alignment::Right,
                "justify" => Alignment::Justify,
                _ => Alignment::Left,
            });
        }
        props.indent_left = length(element.attribute(FO, "margin-left"));
        props.indent_right = length(element.attribute(FO, "margin-right"));
        props.first_line = length(element.attribute(FO, "text-indent"));
        props.space_before = length(element.attribute(FO, "margin-top")).map(|v| v.max(0));
        props.space_after = length(element.attribute(FO, "margin-bottom")).map(|v| v.max(0));
        if let Some(height) = element.attribute(FO, "line-height") {
            props.line = if let Some(percent) = height.trim().strip_suffix('%') {
                percent
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|percent| *percent > 0.0)
                    .map(|percent| {
                        if (percent - 100.0).abs() < 0.5 {
                            LineSpacing::Single
                        } else {
                            LineSpacing::Multiple(percent / 100.0)
                        }
                    })
            } else if height == "normal" {
                Some(LineSpacing::Single)
            } else {
                length(Some(height)).map(|height| LineSpacing::Exact(height.max(1)))
            };
        }
        if let Some(least) = length(element.attribute(STYLE, "line-height-at-least")) {
            props.line = Some(LineSpacing::AtLeast(least.max(1)));
        }
        if let Some(keep) = element.attribute(FO, "keep-with-next") {
            props.keep_with_next = Some(keep == "always");
        }
        if let Some(keep) = element.attribute(FO, "keep-together") {
            props.keep_together = Some(keep == "always");
        }
        match element.attribute(FO, "break-before") {
            Some("page") => props.page_break_before = Some(true),
            Some("column") => props.column_break_before = Some(true),
            Some(_) => {
                props.page_break_before = Some(false);
                props.column_break_before = Some(false);
            }
            None => {}
        }
        let all = element.attribute(FO, "border");
        for (side, slot) in [
            ("top", &mut props.border_top),
            ("bottom", &mut props.border_bottom),
        ] {
            if let Some(value) = element.attribute(FO, &format!("border-{side}")).or(all) {
                let space = length(
                    element
                        .attribute(FO, &format!("padding-{side}"))
                        .or(element.attribute(FO, "padding")),
                )
                .unwrap_or(0)
                .max(0);
                *slot = Some(border(Some(value)).map(|line| RuleLine { space, ..line }));
            }
        }
        if let Some(stops) = element.child(STYLE, "tab-stops") {
            let tabs = stops
                .children()
                .filter(|stop| stop.is(STYLE, "tab-stop"))
                .filter_map(|stop| {
                    Some(Tab {
                        position: length(stop.attribute(STYLE, "position"))?,
                        kind: match stop.attribute(STYLE, "type") {
                            Some("center") => TabKind::Centre,
                            Some("right") => TabKind::Right,
                            Some("char") => TabKind::Decimal,
                            _ => TabKind::Left,
                        },
                        leader: match (
                            stop.attribute(STYLE, "leader-style"),
                            stop.attribute(STYLE, "leader-text"),
                        ) {
                            (Some("none"), _) | (None, None) => TabLeader::None,
                            (_, Some(".")) | (Some("dotted"), _) => TabLeader::Dots,
                            (_, Some("-")) | (Some("dash" | "long-dash"), _) => TabLeader::Hyphens,
                            (Some(_), _) | (_, Some(_)) => TabLeader::Line,
                        },
                    })
                })
                .collect();
            props.tabs = Some(tabs);
        }
        props
    }

    fn text_props(&self, element: &Element<'_>) -> Text {
        let mut text = Text::default();
        let props = &mut text.props;
        if let Some(name) = element.attribute(STYLE, "font-name") {
            props.font = Some(
                self.font_names
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.to_owned()),
            );
        } else if let Some(family) = element.attribute(FO, "font-family") {
            props.font = Some(family.trim().trim_matches(['\'', '"']).to_owned());
        }
        if let Some(size) = element.attribute(FO, "font-size") {
            if let Some(percent) = size.trim().strip_suffix('%') {
                text.relative = percent.trim().parse().ok();
            } else {
                props.size = length(Some(size)).map(|size| size.clamp(20, 32_767));
            }
        }
        if let Some(weight) = element.attribute(FO, "font-weight") {
            props.bold = Some(weight == "bold" || weight.parse::<u32>().is_ok_and(|w| w >= 600));
        }
        if let Some(style) = element.attribute(FO, "font-style") {
            props.italic = Some(style == "italic" || style == "oblique");
        }
        if let Some(style) = element.attribute(STYLE, "text-underline-style") {
            props.underline = Some(if style == "none" {
                Underline::None
            } else if element.attribute(STYLE, "text-underline-type") == Some("double") {
                Underline::Double
            } else if element.attribute(STYLE, "text-underline-mode") == Some("skip-white-space") {
                Underline::Words
            } else {
                Underline::Single
            });
        }
        if let Some(style) = element.attribute(STYLE, "text-line-through-style") {
            props.strike = Some(style != "none");
        }
        if let Some(position) = element.attribute(STYLE, "text-position") {
            let first = position.split_whitespace().next().unwrap_or("0");
            props.script = Some(match first {
                "super" => Script::Super,
                "sub" => Script::Sub,
                value => match value.trim_end_matches('%').parse::<f32>() {
                    Ok(raise) if raise > 0.0 => Script::Super,
                    Ok(raise) if raise < 0.0 => Script::Sub,
                    _ => Script::Normal,
                },
            });
        }
        if let Some(variant) = element.attribute(FO, "font-variant") {
            props.small_caps = Some(variant == "small-caps");
        }
        if let Some(transform) = element.attribute(FO, "text-transform") {
            props.caps = Some(transform == "uppercase");
        }
        if let Some(value) = element.attribute(FO, "color") {
            props.colour = colour(Some(value));
        }
        text.hidden = element.attribute(TEXT, "display") == Some("none");
        text.highlight = element
            .attribute(FO, "background-color")
            .is_some_and(|value| value != "transparent");
        text.spacing = element
            .attribute(FO, "letter-spacing")
            .is_some_and(|value| value != "normal" && length(Some(value)) != Some(0))
            || element
                .attribute(STYLE, "text-scale")
                .is_some_and(|value| value.trim() != "100%");
        text
    }

    /// The chain of `name`'s styles from itself to its root.
    fn chain(&self, name: &str) -> Vec<&StyleEntry> {
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = Some(name);
        while let Some(name) = current {
            if chain.len() > 32 || !seen.insert(name) {
                break;
            }
            let Some(entry) = self.styles.get(name) else {
                break;
            };
            chain.push(entry);
            current = entry.parent.as_deref();
        }
        chain
    }

    /// A style's whole paragraph and character formatting, its list style,
    /// and its outline level: the family's default, then its parents, then
    /// itself.
    fn resolve(&self, name: &str) -> (ParagraphProps, Text, Option<String>, Option<u8>) {
        let chain = self.chain(name);
        let family = chain
            .first()
            .map_or("paragraph", |entry| entry.family.as_str());
        let mut paragraph = ParagraphProps::default();
        let mut text = Text {
            props: CharProps {
                size: Some(240),
                ..CharProps::default()
            },
            ..Text::default()
        };
        if let Some(default) = self.defaults.get(family) {
            overlay_paragraph(&mut paragraph, &default.paragraph);
            text.over(&default.text);
        }
        let mut list = None;
        let mut outline = None;
        for entry in chain.iter().rev() {
            overlay_paragraph(&mut paragraph, &entry.paragraph);
            text.over(&entry.text);
            if entry.list_style.is_some() {
                list.clone_from(&entry.list_style);
            }
            if entry.outline.is_some() {
                outline = entry.outline;
            }
        }
        (paragraph, text, list, outline)
    }

    /// The graphic, table, or cell properties of a style through its
    /// parents.
    fn other(&self, name: Option<&str>) -> BTreeMap<String, String> {
        let mut other = BTreeMap::new();
        if let Some(name) = name {
            for entry in self.chain(name).iter().rev() {
                for (key, value) in &entry.other {
                    other.insert(key.clone(), value.clone());
                }
            }
        }
        other
    }

    /// The named style an automatic one stands on.
    fn named(&self, name: &str) -> String {
        let mut current = name.to_owned();
        for _ in 0..32 {
            match self.styles.get(&current) {
                Some(entry) if entry.automatic => match &entry.parent {
                    Some(parent) => current = parent.clone(),
                    None => return "Standard".into(),
                },
                _ => return current,
            }
        }
        current
    }

    fn list_style(&mut self, element: &Element<'_>) {
        let Some(name) = element.attribute(STYLE, "name") else {
            return;
        };
        let mut levels: Vec<ListLevel> = Vec::new();
        for level in element.children() {
            let number: usize = level
                .attribute(TEXT, "level")
                .and_then(|value| value.parse().ok())
                .unwrap_or(levels.len() + 1)
                .clamp(1, 10);
            if number > 9 || number != levels.len() + 1 {
                continue;
            }
            let escape = |text: &str| text.replace('%', "%%");
            let (format, text) = match level.local() {
                "list-level-style-number" => {
                    let format = level
                        .attribute(STYLE, "num-format")
                        .and_then(number_format)
                        .unwrap_or(NumberFormat::None);
                    let shown: usize = level
                        .attribute(TEXT, "display-levels")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(1)
                        .clamp(1, number);
                    let numbers: Vec<String> = (number + 1 - shown..=number)
                        .map(|level| format!("%{level}"))
                        .collect();
                    let text = format!(
                        "{}{}{}",
                        escape(level.attribute(STYLE, "num-prefix").unwrap_or("")),
                        if format == NumberFormat::None {
                            String::new()
                        } else {
                            numbers.join(".")
                        },
                        escape(level.attribute(STYLE, "num-suffix").unwrap_or(""))
                    );
                    // Text and no number: Ami Pro's bullet text.
                    if format == NumberFormat::None && !text.is_empty() {
                        (NumberFormat::Bullet, text)
                    } else {
                        (format, text)
                    }
                }
                "list-level-style-bullet" => (
                    NumberFormat::Bullet,
                    level
                        .attribute(TEXT, "bullet-char")
                        .filter(|bullet| !bullet.is_empty())
                        .unwrap_or("\u{2022}")
                        .to_owned(),
                ),
                "list-level-style-image" => (NumberFormat::Bullet, "\u{2022}".to_owned()),
                _ => continue,
            };
            let properties = level.child(STYLE, "list-level-properties");
            let alignment = properties
                .as_ref()
                .and_then(|p| p.child(STYLE, "list-level-label-alignment"));
            let (indent_left, hanging) = match &alignment {
                Some(alignment) => (
                    length(alignment.attribute(FO, "margin-left")).unwrap_or(0),
                    -length(alignment.attribute(FO, "text-indent")).unwrap_or(0),
                ),
                None => {
                    let before = properties
                        .as_ref()
                        .and_then(|p| length(p.attribute(TEXT, "space-before")))
                        .unwrap_or(0);
                    let width = properties
                        .as_ref()
                        .and_then(|p| length(p.attribute(TEXT, "min-label-width")))
                        .unwrap_or(0);
                    (before + width, width)
                }
            };
            let mut props = level
                .child(STYLE, "text-properties")
                .map(|p| self.text_props(&p).props)
                .unwrap_or_default();
            let font = props.font.take();
            let right_align = properties
                .as_ref()
                .and_then(|p| p.attribute(FO, "text-align"))
                .is_some_and(|align| matches!(align, "end" | "right"));
            levels.push(ListLevel {
                format,
                text,
                start: level
                    .attribute(TEXT, "start-value")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(1),
                restart_after_higher: true,
                font,
                props,
                right_align,
                indent_left: indent_left.max(0),
                hanging,
            });
        }
        if levels.is_empty() {
            return;
        }
        let index = self.lists.len();
        self.lists.push(List { levels });
        self.list_styles.insert(name.to_owned(), index);
    }

    fn page_layout(&mut self, element: &Element<'_>) {
        let Some(name) = element.attribute(STYLE, "name") else {
            return;
        };
        let mut entry = LayoutEntry::default();
        if let Some(properties) = element.child(STYLE, "page-layout-properties") {
            let page = &mut entry.page;
            let get = |name: &str| properties.attribute(FO, name);
            page.width = length(get("page-width"))
                .filter(|width| *width > 720)
                .unwrap_or(12_240);
            page.height = length(get("page-height"))
                .filter(|height| *height > 720)
                .unwrap_or(15_840);
            page.landscape = properties.attribute(STYLE, "print-orientation") == Some("landscape");
            let margin = |name: &str, fallback: Twips| {
                length(get(name).or(get("margin")))
                    .unwrap_or(fallback)
                    .max(0)
            };
            page.margin_top = margin("margin-top", 1_440);
            page.margin_bottom = margin("margin-bottom", 1_440);
            page.margin_left = margin("margin-left", 1_440);
            page.margin_right = margin("margin-right", 1_440);
            entry.mirrored = properties.attribute(STYLE, "page-usage") == Some("mirrored");
            entry.format = properties
                .attribute(STYLE, "num-format")
                .and_then(number_format);
            let sides = ["top", "bottom", "left", "right"]
                .map(|side| border(get(&format!("border-{side}")).or(get("border"))));
            if let Some(line) = sides.iter().flatten().next().copied() {
                let distance = ["top", "bottom", "left", "right"]
                    .iter()
                    .filter_map(|side| length(get(&format!("padding-{side}")).or(get("padding"))))
                    .max()
                    .unwrap_or(0)
                    .max(0);
                page.border = Some(PageBorder {
                    top: sides[0].is_some(),
                    bottom: sides[1].is_some(),
                    left: sides[2].is_some(),
                    right: sides[3].is_some(),
                    line,
                    distance,
                });
            }
            if let Some(columns) = properties.child(STYLE, "columns") {
                page.columns =
                    page_columns(&columns, page.width - page.margin_left - page.margin_right);
            }
        }
        for (local, header) in [("header-style", true), ("footer-style", false)] {
            if let Some(band) = element
                .child(STYLE, local)
                .and_then(|style| style.child(STYLE, "header-footer-properties"))
            {
                let height = length(band.attribute(FO, "min-height"))
                    .or_else(|| length(band.attribute(SVG, "height")))
                    .unwrap_or(0)
                    .max(0);
                let spacing = length(band.attribute(
                    FO,
                    if header {
                        "margin-bottom"
                    } else {
                        "margin-top"
                    },
                ))
                .unwrap_or(0)
                .max(0);
                if header {
                    entry.header = Some((height, spacing));
                } else {
                    entry.footer = Some((height, spacing));
                }
            }
        }
        self.layouts.insert(name.to_owned(), entry);
    }

    fn master_page(&mut self, element: &Element<'_>, pictures: &mut Pictures<'_>) {
        let Some(name) = element.attribute(STYLE, "name") else {
            return;
        };
        let mut master = Master {
            layout: element
                .attribute(STYLE, "page-layout-name")
                .unwrap_or("")
                .to_owned(),
            ..Master::default()
        };
        for band in element.children() {
            if band.attribute(STYLE, "display") == Some("false") {
                continue;
            }
            let slot = match band.local() {
                "header" => &mut master.headers.default,
                "header-left" => &mut master.headers.even,
                "header-first" => &mut master.headers.first,
                "footer" => &mut master.footers.default,
                "footer-left" => &mut master.footers.even,
                "footer-first" => &mut master.footers.first,
                _ => continue,
            };
            *slot = Some(self.blocks_of(&band, pictures));
        }
        if !self.master_order.iter().any(|known| known == name) {
            self.master_order.push(name.to_owned());
        }
        self.masters.insert(name.to_owned(), master);
    }

    // ---- Meta ----

    fn read_meta(&mut self, root: Element<'_>) {
        let Some(meta) = root.child(OFFICE, "meta") else {
            return;
        };
        for element in meta.children() {
            let text = element.text();
            match (element.namespace(), element.local()) {
                (DC, "title") => self.properties.title = text,
                (DC, "description") => self.properties.description = text,
                (DC, "subject") if self.properties.description.is_empty() => {
                    self.properties.description = text;
                }
                (META, "keyword") => {
                    if !self.properties.keywords.is_empty() {
                        self.properties.keywords.push_str(", ");
                    }
                    self.properties.keywords.push_str(&text);
                }
                (META, "creation-date") => self.properties.created = iso_date(&text),
                (DC, "date") => self.properties.modified = iso_date(&text),
                (META, "editing-cycles") => self.properties.revisions = text.parse().unwrap_or(0),
                (META, "generator") => {
                    self.settings.ami_pro_rows = text.trim().starts_with(crate::GENERATOR);
                }
                (META, "editing-duration") => self.properties.editing_minutes = minutes(&text),
                (META, "template") => {
                    if let Some(title) = element.attribute(XLINK, "title") {
                        self.properties.template = title.to_owned();
                    }
                }
                (META, "document-statistic") => {
                    let count = |name: &str| {
                        element
                            .attribute(META, name)
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0)
                    };
                    self.properties.pages = count("page-count");
                    self.properties.words = count("word-count");
                    self.properties.characters = count("character-count");
                }
                (META, "user-defined") => {
                    if let Some(name) = element.attribute(META, "name") {
                        self.properties.custom.push((name.to_owned(), text));
                    }
                }
                _ => {}
            }
        }
    }

    // ---- The body ----

    fn read_body(&mut self, root: Element<'_>, pictures: &mut Pictures<'_>) {
        let mut master = self
            .master_order
            .iter()
            .find(|name| name.as_str() == "Standard")
            .or_else(|| self.master_order.first())
            .cloned()
            .unwrap_or_else(|| "Standard".into());
        let mut page_start = None;
        self.page = self.section_page(&master);
        let Some(body) = root
            .child(OFFICE, "body")
            .and_then(|body| body.child(OFFICE, "text"))
        else {
            let section = self.section_for(&master, None, Vec::new());
            self.sections.push(section);
            return;
        };
        if let Some(changes) = body.child(TEXT, "tracked-changes") {
            for region in changes.children().filter(|e| e.is(TEXT, "changed-region")) {
                let id = region
                    .attribute(XML, "id")
                    .or_else(|| region.attribute(TEXT, "id"));
                let kind = region.children().find_map(|change| match change.local() {
                    "insertion" => Some(FindingKind::Insertions),
                    "deletion" => Some(FindingKind::Deletions),
                    "format-change" => Some(FindingKind::FormatChanges),
                    _ => None,
                });
                if let (Some(id), Some(kind)) = (id, kind) {
                    self.changes.insert(id.to_owned(), kind);
                }
            }
        }
        let mut blocks: Vec<Block> = Vec::new();
        for element in body.children() {
            // A paragraph whose style names a master page starts a section.
            let starts = matches!(element.local(), "p" | "h")
                .then(|| element.attribute(TEXT, "style-name"))
                .flatten()
                .or_else(|| {
                    element
                        .is(TABLE, "table")
                        .then(|| element.attribute(TABLE, "style-name"))
                        .flatten()
                })
                .and_then(|style| self.master_of(style));
            if let Some((next, start)) = starts {
                if !blocks.is_empty() || !self.pending_shapes.is_empty() {
                    self.flush_shapes(&mut blocks);
                    let section =
                        self.section_for(&master, page_start, std::mem::take(&mut blocks));
                    self.sections.push(section);
                }
                master = next;
                page_start = start;
                self.page = self.section_page(&master);
            }
            let before = blocks.len();
            self.block(&element, &mut blocks, pictures, None);
            // Frames on the page go into the paragraph after them.
            if !self.pending_shapes.is_empty()
                && let Some(Block::Paragraph(paragraph)) = blocks.get_mut(before)
            {
                let mut inlines = std::mem::take(&mut self.pending_shapes);
                inlines.append(&mut paragraph.inlines);
                paragraph.inlines = inlines;
            }
        }
        self.flush_shapes(&mut blocks);
        let section = self.section_for(&master, page_start, blocks);
        self.sections.push(section);
    }

    fn flush_shapes(&mut self, blocks: &mut Vec<Block>) {
        if !self.pending_shapes.is_empty() {
            blocks.push(Block::Paragraph(Paragraph {
                inlines: std::mem::take(&mut self.pending_shapes),
                ..Paragraph::default()
            }));
        }
    }

    /// The master page a paragraph or table style starts, and the page
    /// number it starts with.
    fn master_of(&self, style: &str) -> Option<(String, Option<u32>)> {
        let entry = self.styles.get(style)?;
        let master = entry.master_page.clone()?;
        let start = entry.page_number.or_else(|| {
            entry
                .other
                .get("page-number")
                .and_then(|number| number.parse().ok())
        });
        Some((master, start))
    }

    /// The page a master gives the text: its margins below the bands.
    fn section_page(&self, master: &str) -> PageSetup {
        let master = self.masters.get(master).cloned().unwrap_or_default();
        let layout = self
            .layouts
            .get(&master.layout)
            .cloned()
            .unwrap_or_default();
        let mut page = layout.page.clone();
        let header = !master.headers.is_empty();
        let footer = !master.footers.is_empty();
        // ODF's margins reach the bands: the text starts below the band.
        match layout.header.filter(|_| header) {
            Some((height, spacing)) => {
                page.header_distance = page.margin_top;
                page.margin_top += height + spacing;
            }
            None => page.header_distance = page.margin_top / 2,
        }
        match layout.footer.filter(|_| footer) {
            Some((height, spacing)) => {
                page.footer_distance = page.margin_bottom;
                page.margin_bottom += height + spacing;
            }
            None => page.footer_distance = page.margin_bottom / 2,
        }
        page
    }

    fn section_for(&mut self, master: &str, start: Option<u32>, mut blocks: Vec<Block>) -> Section {
        if blocks.is_empty() {
            blocks.push(Block::Paragraph(Paragraph::default()));
        }
        let page = self.section_page(master);
        let entry = self.masters.get(master).cloned().unwrap_or_default();
        let layout = self.layouts.get(&entry.layout).cloned().unwrap_or_default();
        if layout.mirrored {
            self.settings.mirror_margins = true;
        }
        if entry.headers.even.is_some() || entry.footers.even.is_some() {
            self.settings.even_and_odd_headers = true;
        }
        let page_numbers = match (layout.format, start) {
            (None, None) => None,
            (format, start) => Some(PageNumbering {
                format: format.unwrap_or(NumberFormat::Decimal),
                start,
            }),
        };
        Section {
            page,
            new_page: true,
            headers: entry.headers,
            footers: entry.footers,
            page_numbers,
            blocks,
        }
    }

    /// The blocks inside `container` (a band, a cell, a text box, a note).
    fn blocks_of(&mut self, container: &Element<'_>, pictures: &mut Pictures<'_>) -> Vec<Block> {
        let mut blocks = Vec::new();
        let pending = std::mem::take(&mut self.pending_shapes);
        for element in container.children() {
            self.block(&element, &mut blocks, pictures, None);
        }
        self.flush_shapes(&mut blocks);
        self.pending_shapes = pending;
        if blocks.is_empty() {
            blocks.push(Block::Paragraph(Paragraph::default()));
        }
        blocks
    }

    /// A block-level element: its blocks, in the list place `list` when
    /// it is in a list.
    fn block(
        &mut self,
        element: &Element<'_>,
        blocks: &mut Vec<Block>,
        pictures: &mut Pictures<'_>,
        list: Option<ListPlace>,
    ) {
        if self.depth >= MAX_DEPTH {
            return;
        }
        self.depth += 1;
        match (element.namespace(), element.local()) {
            (TEXT, "p" | "h") => {
                let paragraph = self.paragraph(element, pictures, list);
                blocks.push(Block::Paragraph(paragraph));
            }
            (TEXT, "list") => self.list(element, blocks, pictures, list),
            (TEXT, "numbered-paragraph") => {
                for inner in element.children() {
                    self.block(&inner, blocks, pictures, list);
                }
            }
            (TABLE, "table") => {
                if let Some(table) = self.table(element, pictures) {
                    blocks.push(table);
                }
            }
            (TEXT, "section") => {
                if let Some(style) = element.attribute(TEXT, "style-name")
                    && self
                        .other(Some(style))
                        .get("column-count")
                        .and_then(|count| count.parse::<u8>().ok())
                        .is_some_and(|count| count > 1)
                {
                    self.findings.count(FindingKind::Columns);
                }
                if element.attribute(TEXT, "display") == Some("none") {
                    self.findings.count(FindingKind::HiddenText);
                } else {
                    for inner in element.children() {
                        self.block(&inner, blocks, pictures, list);
                    }
                }
            }
            (TEXT, "index-body" | "index-title") => {
                for inner in element.children() {
                    self.block(&inner, blocks, pictures, None);
                }
            }
            (
                TEXT,
                "table-of-content" | "alphabetical-index" | "illustration-index" | "table-index"
                | "object-index" | "user-index" | "bibliography",
            ) => {
                self.findings.count(FindingKind::GeneratedText);
                if let Some(body) = element.child(TEXT, "index-body") {
                    self.block(&body, blocks, pictures, None);
                }
            }
            (DRAW, _) => {
                // A frame between paragraphs (anchored to the page).
                let mut inlines = Vec::new();
                let props = CharProps::default();
                self.inline_element(element, &mut inlines, pictures, &props, &props);
                self.pending_shapes.extend(inlines);
            }
            _ => {}
        }
        self.depth -= 1;
    }

    fn list(
        &mut self,
        element: &Element<'_>,
        blocks: &mut Vec<Block>,
        pictures: &mut Pictures<'_>,
        outer: Option<ListPlace>,
    ) {
        let place = match outer {
            Some(outer) => Some(ListPlace {
                instance: outer.instance,
                level: outer.level.saturating_add(1).min(8),
            }),
            None => element
                .attribute(TEXT, "style-name")
                .and_then(|style| Some((style, *self.list_styles.get(style)?)))
                .map(|(style, list)| {
                    let continued = element
                        .attribute(TEXT, "continue-list")
                        .and_then(|id| self.list_ids.get(id).copied())
                        .or_else(|| {
                            (element.attribute(TEXT, "continue-numbering") == Some("true"))
                                .then(|| self.last_instance.get(style).copied())
                                .flatten()
                        });
                    let instance = continued.unwrap_or_else(|| {
                        self.instances.push(ListInstance {
                            list,
                            starts: [None; 9],
                        });
                        self.instances.len() - 1
                    });
                    self.last_instance.insert(style.to_owned(), instance);
                    ListPlace { instance, level: 0 }
                }),
        };
        if let (Some(place), Some(id)) = (place, element.attribute(XML, "id")) {
            self.list_ids.insert(id.to_owned(), place.instance);
        }
        for item in element.children() {
            let header = item.is(TEXT, "list-header");
            if !(header || item.is(TEXT, "list-item")) {
                continue;
            }
            if let (Some(place), Some(start)) = (
                place,
                item.attribute(TEXT, "start-value")
                    .and_then(|start| start.parse::<u32>().ok()),
            ) && let Some(instance) = self.instances.get_mut(place.instance)
            {
                instance.starts[usize::from(place.level)].get_or_insert(start);
            }
            for inner in item.children() {
                let place = if header && inner.local() != "list" {
                    None
                } else {
                    place
                };
                match (place, outer, inner.local()) {
                    // A list in an unnumbered list is read as paragraphs.
                    (None, None, "list") if !header => {
                        for nested in inner.children() {
                            for block in nested.children() {
                                self.block(&block, blocks, pictures, None);
                            }
                        }
                    }
                    _ => self.block(&inner, blocks, pictures, place),
                }
            }
        }
    }

    /// The slot standing for a paragraph style's name until the styles are
    /// numbered.
    fn slot(&mut self, name: &str) -> usize {
        if let Some(slot) = self.referenced_slots.get(name) {
            return *slot;
        }
        self.referenced.push(name.to_owned());
        self.referenced_slots
            .insert(name.to_owned(), self.referenced.len() - 1);
        self.referenced.len() - 1
    }

    fn paragraph(
        &mut self,
        element: &Element<'_>,
        pictures: &mut Pictures<'_>,
        list: Option<ListPlace>,
    ) -> Paragraph {
        let style_name = element
            .attribute(TEXT, "style-name")
            .unwrap_or("Standard")
            .to_owned();
        let named = self.named(&style_name);
        // The automatic styles' own formatting over the named style.
        let mut props = ParagraphProps::default();
        let mut own = Text::default();
        let mut side_borders = false;
        for entry in self
            .chain(&style_name)
            .into_iter()
            .take_while(|entry| entry.automatic)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            overlay_paragraph(&mut props, &entry.paragraph);
            own.over(&entry.text);
            side_borders |= entry.side_borders;
        }
        if side_borders {
            self.findings.count(FindingKind::ParagraphBorders);
        }
        let (_, base, ..) = self.resolve(&named);
        let mut run = base.clone();
        run.over(&own);
        let saved_space = std::mem::replace(&mut self.space, true);
        let saved_marks = std::mem::take(&mut self.open_marks);
        let mut inlines = Vec::new();
        self.inlines(element, &mut inlines, pictures, &run.props, &base.props);
        // A break after the paragraph: Ami Pro's break at its end.
        match self
            .chain(&style_name)
            .iter()
            .find_map(|entry| entry.break_after.as_deref())
        {
            Some("page") => inlines.push(Inline::PageBreak),
            Some("column") => inlines.push(Inline::ColumnBreak),
            _ => {}
        }
        self.space = saved_space;
        self.open_marks = saved_marks;
        self.used.insert(named.clone());
        Paragraph {
            style: self.slot(&named),
            props,
            list: list.map(|place| (Some(place.instance), place.level)),
            number_text: None,
            inlines,
            mark: difference(&run.props, &base.props),
        }
    }

    /// The inlines of `element` (a paragraph or a span).
    fn inlines(
        &mut self,
        element: &Element<'_>,
        out: &mut Vec<Inline>,
        pictures: &mut Pictures<'_>,
        run: &CharProps,
        base: &CharProps,
    ) {
        for node in element.nodes() {
            match node {
                Content::Text(text) => {
                    let text = self.collapse(text);
                    push_text(out, &text, difference(run, base));
                }
                Content::Element(child) => self.inline_element(&child, out, pictures, run, base),
            }
        }
    }

    /// ODF's white space: runs of spaces, tabs, and line ends as one space,
    /// none at a paragraph's start.
    fn collapse(&mut self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for character in text.chars() {
            if matches!(character, ' ' | '\t' | '\n' | '\r') {
                if !self.space {
                    out.push(' ');
                    self.space = true;
                }
            } else {
                out.push(character);
                self.space = false;
            }
        }
        out
    }

    fn inline_element(
        &mut self,
        element: &Element<'_>,
        out: &mut Vec<Inline>,
        pictures: &mut Pictures<'_>,
        run: &CharProps,
        base: &CharProps,
    ) {
        if self.depth >= MAX_DEPTH {
            return;
        }
        self.depth += 1;
        self.inline_element_inner(element, out, pictures, run, base);
        self.depth -= 1;
    }

    fn inline_element_inner(
        &mut self,
        element: &Element<'_>,
        out: &mut Vec<Inline>,
        pictures: &mut Pictures<'_>,
        run: &CharProps,
        base: &CharProps,
    ) {
        let props = difference(run, base);
        let local = element.local();
        match element.namespace() {
            TEXT => {}
            DRAW => {
                self.drawing(element, out, pictures, run, base);
                return;
            }
            OFFICE if local == "annotation" => {
                self.annotation(element, out, pictures);
                return;
            }
            _ => return,
        }
        match local {
            "span" => {
                let style = element.attribute(TEXT, "style-name");
                let mut span = Text {
                    props: run.clone(),
                    ..Text::default()
                };
                if let Some(style) = style {
                    let chain: Vec<StyleEntry> =
                        self.chain(style).into_iter().rev().cloned().collect();
                    let mut text = Text::default();
                    for entry in &chain {
                        text.over(&entry.text);
                    }
                    if text.hidden {
                        self.findings.count(FindingKind::HiddenText);
                        return;
                    }
                    if text.highlight {
                        self.findings.count(FindingKind::Highlight);
                    }
                    if text.spacing {
                        self.findings.count(FindingKind::CharacterSpacing);
                    }
                    if let Some(named) = chain.iter().find(|entry| !entry.automatic) {
                        self.findings
                            .count(FindingKind::CharacterStyle(named.display.clone()));
                    }
                    span.over(&text);
                }
                self.inlines(element, out, pictures, &span.props, base);
            }
            "s" => {
                let count = element
                    .attribute(TEXT, "c")
                    .and_then(|count| count.parse::<usize>().ok())
                    .unwrap_or(1)
                    .clamp(1, 1_000);
                push_text(out, &" ".repeat(count), props);
                self.space = false;
            }
            "tab" => {
                out.push(Inline::Tab { props });
                self.space = false;
            }
            "line-break" => {
                out.push(Inline::LineBreak);
                self.space = false;
            }
            "soft-page-break" | "reference-mark-end" | "sequence-decls" | "variable-decls"
            | "user-field-decls" | "hidden-paragraph" | "script" => {}
            "a" => {
                let marker = self.findings.at(FindingKind::Links);
                out.push(Inline::Marker(marker));
                self.inlines(element, out, pictures, run, base);
            }
            // Bookmarks as Ami Pro's (the notes run); a point bookmark
            // starts and ends at once.
            "bookmark" | "bookmark-start" | "bookmark-end" => {
                let name = element
                    .attribute(TEXT, "name")
                    .unwrap_or("")
                    .trim()
                    .to_owned();
                if !name.is_empty() {
                    if local != "bookmark-end" {
                        out.push(Inline::Bookmark {
                            name: name.clone(),
                            end: false,
                        });
                    }
                    if local != "bookmark-start" {
                        out.push(Inline::Bookmark { name, end: true });
                    }
                }
            }
            "change-start" | "change" => {
                let kind = element
                    .attribute(TEXT, "change-id")
                    .and_then(|id| self.changes.get(id))
                    .cloned()
                    .unwrap_or(if local == "change" {
                        FindingKind::Deletions
                    } else {
                        FindingKind::Insertions
                    });
                let marker = self.findings.at(kind);
                out.push(Inline::Marker(marker));
            }
            "change-end" => {}
            "note" => {
                let endnote = element.attribute(TEXT, "note-class") == Some("endnote");
                if let Some(body) = element.child(TEXT, "note-body") {
                    let saved = std::mem::replace(&mut self.space, true);
                    let blocks = self.blocks_of(&body, pictures);
                    self.space = saved;
                    out.push(Inline::Footnote(Box::new(Footnote {
                        endnote,
                        blocks,
                        props,
                    })));
                }
            }
            "hidden-text" => self.findings.count(FindingKind::HiddenText),
            "page-number" => {
                if element
                    .attribute(TEXT, "select-page")
                    .is_some_and(|page| page != "current")
                {
                    self.kept_field(element, out, props);
                } else {
                    let format = element
                        .attribute(STYLE, "num-format")
                        .and_then(number_format)
                        .unwrap_or(NumberFormat::Decimal);
                    out.push(field(FieldKind::Page { format }, element.text(), props));
                }
            }
            "page-count" => out.push(field(FieldKind::NumPages, element.text(), props)),
            "date" | "creation-date" | "modification-date" | "print-date" | "time"
            | "creation-time" | "modification-time" | "print-time" => {
                self.date_field(element, out, props);
            }
            "title" => out.push(field(FieldKind::Title, element.text(), props)),
            "description" | "subject" => {
                out.push(field(FieldKind::Description, element.text(), props));
            }
            "keywords" => out.push(field(FieldKind::Keywords, element.text(), props)),
            "file-name" => out.push(field(
                FieldKind::FileName {
                    path: element.attribute(TEXT, "display") == Some("full"),
                },
                element.text(),
                props,
            )),
            "template-name" => out.push(field(FieldKind::Template, element.text(), props)),
            "editing-cycles" => out.push(field(FieldKind::Revisions, element.text(), props)),
            "editing-duration" => out.push(field(FieldKind::EditingTime, element.text(), props)),
            "word-count" => out.push(field(FieldKind::Words, element.text(), props)),
            "character-count" => out.push(field(FieldKind::Characters, element.text(), props)),
            "user-defined" => out.push(field(
                FieldKind::Custom(element.attribute(TEXT, "name").unwrap_or("").to_owned()),
                element.text(),
                props,
            )),
            "sequence" => out.push(field(
                FieldKind::Seq {
                    name: element.attribute(TEXT, "name").unwrap_or("Seq").to_owned(),
                },
                element.text(),
                props,
            )),
            "alphabetical-index-mark" | "alphabetical-index-mark-start" => {
                let value = element.attribute(TEXT, "string-value").unwrap_or("");
                let (primary, secondary) = match element.attribute(TEXT, "key1") {
                    Some(key) => (key.to_owned(), value.to_owned()),
                    None => (value.to_owned(), String::new()),
                };
                self.open_mark(element, out);
                out.push(field(
                    FieldKind::IndexEntry { primary, secondary },
                    String::new(),
                    CharProps::default(),
                ));
            }
            "toc-mark" | "toc-mark-start" | "user-index-mark" | "user-index-mark-start" => {
                self.open_mark(element, out);
                out.push(field(
                    FieldKind::ContentsEntry {
                        text: element
                            .attribute(TEXT, "string-value")
                            .unwrap_or("")
                            .to_owned(),
                        level: element
                            .attribute(TEXT, "outline-level")
                            .and_then(|level| level.parse().ok())
                            .unwrap_or(1),
                    },
                    String::new(),
                    CharProps::default(),
                ));
            }
            "alphabetical-index-mark-end" | "toc-mark-end" | "user-index-mark-end" => {
                self.close_mark(element, out);
            }
            "text-input" | "drop-down" | "checkbox" => {
                self.findings.count(FindingKind::FormFields);
                let text = self.collapse(&element.text());
                push_text(out, &text, props);
            }
            "chapter"
            | "reference-ref"
            | "bookmark-ref"
            | "sequence-ref"
            | "note-ref"
            | "variable-get"
            | "variable-set"
            | "variable-input"
            | "user-field-get"
            | "user-field-input"
            | "expression"
            | "author-name"
            | "author-initials"
            | "sender-firstname"
            | "sender-lastname"
            | "sender-initials"
            | "sender-title"
            | "sender-position"
            | "sender-email"
            | "sender-phone-private"
            | "sender-fax"
            | "sender-company"
            | "sender-phone-work"
            | "sender-street"
            | "sender-city"
            | "sender-postal-code"
            | "sender-country"
            | "sender-state-or-province"
            | "conditional-text"
            | "page-variable-get"
            | "page-continuation"
            | "database-display"
            | "database-name"
            | "database-row-number"
            | "sheet-name"
            | "bibliography-mark"
            | "measure"
            | "table-count"
            | "image-count"
            | "object-count"
            | "paragraph-count"
            | "table-formula"
            | "printed-by"
            | "initial-creator"
            | "creator"
            | "creation-author"
            | "modification-author"
            | "print-author"
            | "dde-connection"
            | "placeholder"
            | "execute-macro"
            | "template"
            | "subject-field" => self.kept_field(element, out, props),
            // Ruby, metadata, and unknown wrappers: their content.
            _ => self.inlines(element, out, pictures, run, base),
        }
    }

    /// A field with no Ami Pro counterpart: its result, and a finding.
    fn kept_field(&mut self, element: &Element<'_>, out: &mut Vec<Inline>, props: CharProps) {
        let marker = self
            .findings
            .at(FindingKind::Field(element.local().to_owned()));
        out.push(Inline::Marker(marker));
        let text = self.collapse(&element.text());
        push_text(out, &text, props);
    }

    fn date_field(&mut self, element: &Element<'_>, out: &mut Vec<Inline>, props: CharProps) {
        let local = element.local();
        if element.attribute(TEXT, "fixed") == Some("true") {
            push_text(out, &element.text(), props);
            self.space = false;
            return;
        }
        let source = if local.starts_with("creation") {
            DateSource::Created
        } else if local.starts_with("modification") {
            DateSource::Saved
        } else {
            if local.starts_with("print") {
                self.findings.count(FindingKind::Field(local.to_owned()));
            }
            DateSource::Now
        };
        let picture = element
            .attribute(STYLE, "data-style-name")
            .and_then(|name| self.data_styles.get(name));
        let time = local.ends_with("time")
            || picture.is_some_and(|picture| crate::read_rtf::is_time(picture));
        let kind = if time {
            FieldKind::Time { source }
        } else {
            FieldKind::Date {
                source,
                picture: picture.map_or_else(
                    || "MM/dd/yy".to_owned(),
                    |picture| crate::read_rtf::date_picture(picture),
                ),
                upper: false,
            }
        };
        out.push(field(kind, element.text(), props));
        self.space = false;
    }

    /// An index or contents mark that starts here; its text is the text up
    /// to its end mark where it has no text of its own.
    fn open_mark(&mut self, element: &Element<'_>, out: &[Inline]) {
        if element.local().ends_with("-start")
            && let Some(id) = element.attribute(TEXT, "id")
        {
            self.open_marks.push(OpenMark {
                id: id.to_owned(),
                at: out.len(),
            });
        }
    }

    fn close_mark(&mut self, element: &Element<'_>, out: &mut [Inline]) {
        let Some(id) = element.attribute(TEXT, "id") else {
            return;
        };
        let Some(index) = self.open_marks.iter().position(|mark| mark.id == id) else {
            return;
        };
        let mark = self.open_marks.remove(index);
        let text: String = out
            .get(mark.at + 1..)
            .unwrap_or_default()
            .iter()
            .filter_map(|inline| match inline {
                Inline::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        let text = text.trim().to_owned();
        if let Some(Inline::Field(field)) = out.get_mut(mark.at) {
            match &mut field.kind {
                FieldKind::IndexEntry { primary, secondary } => {
                    if primary.is_empty() {
                        *primary = text;
                    } else if secondary.is_empty() {
                        *secondary = text;
                    }
                }
                FieldKind::ContentsEntry { text: entry, .. } if entry.is_empty() => *entry = text,
                _ => {}
            }
        }
    }

    fn annotation(
        &mut self,
        element: &Element<'_>,
        out: &mut Vec<Inline>,
        pictures: &mut Pictures<'_>,
    ) {
        let author = element
            .child(DC, "creator")
            .map(|e| e.text().trim().to_owned())
            .unwrap_or_default();
        let initials = element
            .child(META, "creator-initials")
            .or_else(|| {
                element
                    .children()
                    .find(|child| child.local() == "creator-initials")
            })
            .map(|e| e.text().trim().to_owned())
            .unwrap_or_default();
        let date = element.child(DC, "date").and_then(|e| iso_date(&e.text()));
        let saved = std::mem::replace(&mut self.space, true);
        let blocks = self.blocks_of(element, pictures);
        self.space = saved;
        out.push(Inline::Marker(self.findings.at(FindingKind::Comments)));
        out.push(Inline::Note(Box::new(Note {
            author,
            initials,
            date,
            blocks,
        })));
    }

    // ---- Frames ----

    fn drawing(
        &mut self,
        element: &Element<'_>,
        out: &mut Vec<Inline>,
        pictures: &mut Pictures<'_>,
        run: &CharProps,
        base: &CharProps,
    ) {
        match element.local() {
            "frame" => {
                if let Some(shape) = self.frame(element, pictures) {
                    out.push(Inline::Shape(Box::new(shape)));
                }
            }
            "a" => {
                let marker = self.findings.at(FindingKind::Links);
                out.push(Inline::Marker(marker));
                for inner in element.children() {
                    self.inline_element(&inner, out, pictures, run, base);
                }
            }
            "g" => {
                self.findings.count(FindingKind::Shape("group".into()));
                for inner in element.children() {
                    self.inline_element(&inner, out, pictures, run, base);
                }
            }
            "custom-shape" | "rect" | "ellipse" | "circle" | "polygon" | "path" | "polyline"
            | "regular-polygon" | "caption" => {
                let kind = match element.local() {
                    "custom-shape" => element
                        .child(DRAW, "enhanced-geometry")
                        .and_then(|geometry| geometry.attribute(DRAW, "type"))
                        .unwrap_or("shape")
                        .to_owned(),
                    other => other.to_owned(),
                };
                self.findings.count(FindingKind::Shape(kind));
                // A drawn shape's text, kept in a text frame of its size.
                if element.children().any(|child| child.is(TEXT, "p"))
                    && let Some(shape) = self.frame(element, pictures)
                {
                    out.push(Inline::Shape(Box::new(shape)));
                }
            }
            "line" | "connector" | "measure" => {
                self.findings
                    .count(FindingKind::Shape(element.local().to_owned()));
            }
            _ => {}
        }
    }

    /// A frame (or a drawn shape holding text) as a shape.
    fn frame(&mut self, element: &Element<'_>, pictures: &mut Pictures<'_>) -> Option<Shape> {
        let text_box = element.child(DRAW, "text-box");
        let width = length(element.attribute(SVG, "width"))
            .or_else(|| text_box.and_then(|text| length(text.attribute(FO, "min-width"))))
            .unwrap_or(1_440)
            .clamp(20, 200_000);
        let height = length(element.attribute(SVG, "height"))
            .or_else(|| text_box.and_then(|text| length(text.attribute(FO, "min-height"))))
            .unwrap_or(720)
            .clamp(20, 200_000);
        let style = self.other(element.attribute(DRAW, "style-name"));
        let get = |name: &str| style.get(name).map(String::as_str);
        let anchor_type = element
            .attribute(TEXT, "anchor-type")
            .unwrap_or("paragraph");
        let anchor = if anchor_type == "as-char" {
            Anchor::Inline { drop: 0 }
        } else {
            let horizontal = match (anchor_type, get("horizontal-rel")) {
                (_, Some("page" | "page-start-margin" | "page-end-margin")) | ("page", None) => {
                    HorizontalFrom::Page
                }
                (_, Some("page-content")) => HorizontalFrom::Margin,
                _ => HorizontalFrom::Column,
            };
            let vertical = match (anchor_type, get("vertical-rel")) {
                (_, Some("page")) | ("page", None) => VerticalFrom::Page,
                (_, Some("page-content")) => VerticalFrom::Margin,
                _ => VerticalFrom::Paragraph,
            };
            let page = &self.page;
            let across = match horizontal {
                HorizontalFrom::Page => page.width,
                _ => page.width - page.margin_left - page.margin_right,
            };
            let down = match vertical {
                VerticalFrom::Page => page.height,
                VerticalFrom::Margin => page.height - page.margin_top - page.margin_bottom,
                VerticalFrom::Paragraph => height,
            };
            let x = match get("horizontal-pos") {
                Some("center") => (across - width) / 2,
                Some("right" | "outside") => across - width,
                Some("left" | "inside") => 0,
                _ => length(element.attribute(SVG, "x")).unwrap_or(0),
            };
            let y = match get("vertical-pos") {
                Some("middle") => (down - height) / 2,
                Some("bottom") => down - height,
                Some("top") => 0,
                _ => length(element.attribute(SVG, "y")).unwrap_or(0),
            };
            let wrap = match get("wrap") {
                Some("none") => ShapeWrap::TopAndBottom,
                Some("run-through") => ShapeWrap::None,
                _ => ShapeWrap::Square,
            };
            Anchor::Floating {
                horizontal: (horizontal, x),
                vertical: (vertical, y),
                wrap,
                behind: get("run-through") == Some("background"),
            }
        };
        let sides = ["top", "right", "bottom", "left"]
            .map(|side| border(get(&format!("border-{side}")).or(get("border"))));
        let mut line = sides.iter().flatten().next().map(|line| ShapeLine {
            kind: line.kind,
            width: line.width,
            colour: line.colour,
            sides: sides.map(|side| side.is_some()),
        });
        // A drawn shape's outline.
        if element.local() != "frame" && get("stroke").is_some_and(|stroke| stroke != "none") {
            line = Some(ShapeLine {
                kind: LineKind::Single,
                width: length(get("stroke-width")).unwrap_or(10).max(1),
                colour: colour(get("stroke-color")).unwrap_or(0),
                sides: [true; 4],
            });
        }
        let fill = match (get("fill"), get("background-color")) {
            (Some("none"), _) => None,
            (Some("solid"), _) => {
                colour(get("fill-color")).or_else(|| colour(get("background-color")))
            }
            (_, Some(value)) => colour(Some(value)),
            _ if element.local() != "frame" && get("fill").is_none() => colour(get("fill-color")),
            _ => None,
        };
        let shadow = get("shadow").filter(|value| *value != "none").map(|value| {
            let parts: Vec<&str> = value.split_whitespace().collect();
            ShapeShadow {
                dx: parts
                    .get(1)
                    .and_then(|part| length(Some(part)))
                    .unwrap_or(72),
                dy: parts
                    .get(2)
                    .and_then(|part| length(Some(part)))
                    .unwrap_or(72),
                colour: parts
                    .first()
                    .and_then(|part| colour(Some(part)))
                    .unwrap_or(0x80_8080),
            }
        });
        let padding = |side: &str| {
            length(get(&format!("padding-{side}")).or(get("padding")))
                .unwrap_or(0)
                .max(0)
        };
        let insets = [
            padding("top"),
            padding("right"),
            padding("bottom"),
            padding("left"),
        ];
        let name = element
            .attribute(DRAW, "name")
            .map(|name| strip_counter(name).to_owned())
            .unwrap_or_default();
        let id = self.next_shape;
        self.next_shape += 1;
        let z = element
            .attribute(DRAW, "z-index")
            .and_then(|z| z.parse().ok())
            .unwrap_or(id);
        let mut corner_radius = length(element.attribute(DRAW, "corner-radius")).unwrap_or(0);
        let images: Vec<Element<'_>> = element.children().filter(|e| e.is(DRAW, "image")).collect();
        let object = element
            .children()
            .any(|child| child.is(DRAW, "object") || child.is(DRAW, "object-ole"));
        let bare = text_box.is_some_and(|text| {
            let mut children = text.children();
            children.next().is_some_and(|only| {
                only.is(TEXT, "p")
                    && only.attributes().next().is_none()
                    && only.nodes().next().is_none()
            }) && children.next().is_none()
        });
        let content = if bare {
            ShapeContent::Empty
        } else if let Some(text) = text_box {
            corner_radius = length(text.attribute(DRAW, "corner-radius")).unwrap_or(corner_radius);
            let saved = std::mem::replace(&mut self.space, true);
            let blocks = self.blocks_of(&text, pictures);
            self.space = saved;
            ShapeContent::Text {
                blocks,
                insets,
                columns: get("column-count")
                    .and_then(|count| count.parse::<u8>().ok())
                    .unwrap_or(1)
                    .clamp(1, 8),
                column_gap: length(get("column-gap")).unwrap_or(0).max(0),
            }
        } else if element.local() != "frame" {
            let saved = std::mem::replace(&mut self.space, true);
            let blocks = self.blocks_of(element, pictures);
            self.space = saved;
            ShapeContent::Text {
                blocks,
                insets,
                columns: 1,
                column_gap: 0,
            }
        } else if let Some(image) = images.first() {
            if object {
                self.findings.count(FindingKind::Objects);
            }
            match self.image(image, images.get(1), pictures) {
                Some(index) => {
                    let mut picture = PlacedPicture {
                        image: index,
                        left: insets[3],
                        top: insets[0],
                        width: (width - insets[1] - insets[3]).max(1),
                        height: (height - insets[0] - insets[2]).max(1),
                        description: element
                            .child(SVG, "desc")
                            .or_else(|| element.child(SVG, "title"))
                            .map(|e| e.text())
                            .unwrap_or_default(),
                    };
                    if let Some(clip) = get("clip").and_then(clip_rect) {
                        let [top, right, bottom, left] = clip;
                        picture.left -= left;
                        picture.top -= top;
                        picture.width += left + right;
                        picture.height += top + bottom;
                    }
                    ShapeContent::Picture(picture)
                }
                None => ShapeContent::Empty,
            }
        } else if object {
            self.findings.count(FindingKind::Objects);
            ShapeContent::Empty
        } else {
            ShapeContent::Empty
        };
        Some(Shape {
            id,
            name,
            anchor,
            width,
            height,
            z,
            line,
            fill,
            shadow,
            corner_radius: corner_radius.max(0),
            content,
        })
    }

    /// A picture (and the PNG standing in for an SVG): its image's index.
    fn image(
        &mut self,
        image: &Element<'_>,
        fallback: Option<&Element<'_>>,
        pictures: &mut Pictures<'_>,
    ) -> Option<usize> {
        let href = image.attribute(XLINK, "href").unwrap_or("");
        if let Some(index) = self.image_names.get(href).filter(|_| !href.is_empty()) {
            return Some(*index);
        }
        let read =
            |image: &Element<'_>, pictures: &mut Pictures<'_>| -> Option<(String, Vec<u8>)> {
                match image.attribute(XLINK, "href") {
                    Some(href) if !href.is_empty() => Some((href.to_owned(), pictures(href)?)),
                    _ => {
                        let data = image.child(OFFICE, "binary-data")?.text();
                        Some((String::new(), base64(&data)?))
                    }
                }
            };
        let Some((key, bytes)) = read(image, pictures) else {
            let what = if href.contains("://") || href.starts_with("..") || href.starts_with('/') {
                "linked".to_owned()
            } else {
                "missing".to_owned()
            };
            self.findings.count(FindingKind::Picture(what));
            return None;
        };
        let Some(kind) = crate::read_docx::image_kind(&key, &bytes) else {
            let extension = key
                .rsplit('.')
                .next()
                .unwrap_or("unknown")
                .to_ascii_uppercase();
            self.findings.count(FindingKind::Picture(extension));
            return None;
        };
        let fallback = (kind == ImageKind::Svg)
            .then(|| fallback.and_then(|fallback| read(fallback, pictures)))
            .flatten()
            .filter(|(key, bytes)| crate::read_docx::image_kind(key, bytes) == Some(ImageKind::Png))
            .map(|(_, bytes)| Arc::from(bytes));
        self.images.push(Image {
            bytes: Arc::from(bytes),
            kind,
            fallback,
            pixels: None,
        });
        let index = self.images.len() - 1;
        if !key.is_empty() {
            self.image_names.insert(key, index);
        }
        Some(index)
    }

    // ---- Tables ----

    /// The grid columns' widths under `element` (a table or a column
    /// group).
    fn columns_of(&self, element: &Element<'_>, columns: &mut Vec<Twips>) {
        for child in element.children() {
            match child.local() {
                "table-column" => {
                    let repeat = child
                        .attribute(TABLE, "number-columns-repeated")
                        .and_then(|n| n.parse::<usize>().ok())
                        .unwrap_or(1)
                        .clamp(1, 64);
                    let width = length(
                        self.other(child.attribute(TABLE, "style-name"))
                            .get("column-width")
                            .map(String::as_str),
                    )
                    .unwrap_or(1_440)
                    .max(20);
                    for _ in 0..repeat {
                        if columns.len() < 64 {
                            columns.push(width);
                        }
                    }
                }
                "table-columns" | "table-column-group" | "table-header-columns" => {
                    self.columns_of(&child, columns);
                }
                _ => {}
            }
        }
    }

    fn table(&mut self, element: &Element<'_>, pictures: &mut Pictures<'_>) -> Option<Block> {
        let table_style = self.other(element.attribute(TABLE, "style-name"));
        let mut columns: Vec<Twips> = Vec::new();
        self.columns_of(element, &mut columns);
        // The rows: each with its heading flag.
        let mut row_elements: Vec<(Element<'_>, bool)> = Vec::new();
        for part in element.children() {
            match part.local() {
                "table-header-rows" => {
                    for row in part.children().filter(|r| r.is(TABLE, "table-row")) {
                        row_elements.push((row, true));
                    }
                }
                "table-row" => row_elements.push((part, false)),
                "table-rows" | "table-row-group" => {
                    for row in part.children() {
                        if row.is(TABLE, "table-row") {
                            row_elements.push((row, false));
                        } else if row.is(TABLE, "table-header-rows") {
                            for inner in row.children().filter(|r| r.is(TABLE, "table-row")) {
                                row_elements.push((inner, true));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if row_elements.is_empty() {
            return None;
        }
        // Each row's cells: a cell with its column and row spans, or a
        // covered cell (`None`).
        type Slot = Option<(Cell, u16, u16)>;
        let mut margins = None;
        let mut rows: Vec<(RowHeight, bool, bool, bool, Vec<Slot>)> = Vec::new();
        for (row, heading) in row_elements {
            let repeat = row
                .attribute(TABLE, "number-rows-repeated")
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 1_000);
            let row_style = self.other(row.attribute(TABLE, "style-name"));
            let height = match (
                length(row_style.get("row-height").map(String::as_str)),
                length(row_style.get("min-row-height").map(String::as_str)),
            ) {
                (Some(height), _)
                    if row_style.get("use-optimal-row-height").map(String::as_str)
                        != Some("true") =>
                {
                    RowHeight::Exact(height.max(0))
                }
                (_, Some(height)) => RowHeight::AtLeast(height.max(0)),
                _ => RowHeight::AtLeast(0),
            };
            let keep = row_style
                .get("keep-together")
                .is_some_and(|keep| keep == "always");
            let breaks = row_style
                .get("break-before")
                .is_some_and(|value| value == "page");
            let mut cells: Vec<Slot> = Vec::new();
            for cell in row.children() {
                let covered = cell.is(TABLE, "covered-table-cell");
                if !(covered || cell.is(TABLE, "table-cell")) {
                    continue;
                }
                let repeat = cell
                    .attribute(TABLE, "number-columns-repeated")
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(1)
                    .clamp(1, 64);
                if covered {
                    for _ in 0..repeat {
                        cells.push(None);
                    }
                    continue;
                }
                let span = cell
                    .attribute(TABLE, "number-columns-spanned")
                    .and_then(|n| n.parse::<u16>().ok())
                    .unwrap_or(1)
                    .clamp(1, 64);
                let rows_spanned = cell
                    .attribute(TABLE, "number-rows-spanned")
                    .and_then(|n| n.parse::<u16>().ok())
                    .unwrap_or(1)
                    .clamp(1, 1_000);
                let style = self.other(cell.attribute(TABLE, "style-name"));
                let get = |name: &str| style.get(name).map(String::as_str);
                let borders = ["top", "right", "bottom", "left"]
                    .map(|side| border(get(&format!("border-{side}")).or(get("border"))));
                let padding = ["top", "right", "bottom", "left"].map(|side| {
                    length(get(&format!("padding-{side}")).or(get("padding")))
                        .unwrap_or(0)
                        .max(0)
                });
                margins.get_or_insert(padding);
                let shading = get("background-color")
                    .filter(|value| *value != "transparent")
                    .and_then(|value| colour(Some(value)));
                let blocks = self.blocks_of(&cell, pictures);
                for _ in 0..repeat {
                    cells.push(Some((
                        Cell {
                            span,
                            merge: if rows_spanned > 1 {
                                VerticalMerge::Start
                            } else {
                                VerticalMerge::None
                            },
                            blocks: blocks.clone(),
                            borders,
                            shading,
                        },
                        span,
                        rows_spanned,
                    )));
                }
            }
            for _ in 0..repeat {
                rows.push((height, heading, keep, breaks, cells.clone()));
            }
        }
        let count = rows
            .iter()
            .map(|(.., cells)| cells.len())
            .max()
            .unwrap_or(1)
            .max(columns.len())
            .clamp(1, 64);
        columns.resize(count, 1_440);
        // Joined blocks: the rows left below each column's block, and its
        // width.
        let mut pending: Vec<(u16, u16)> = vec![(0, 0); count];
        let mut table_rows = Vec::new();
        for (height, heading, keep, breaks, cells) in rows {
            let mut out = Vec::new();
            let mut column = 0;
            let mut iter = cells.into_iter().peekable();
            while column < count {
                let (left, span) = pending[column];
                if left > 0 {
                    pending[column].0 -= 1;
                    for _ in 0..span {
                        if iter.peek().is_some_and(Option::is_none) {
                            iter.next();
                        }
                    }
                    out.push(Cell {
                        span,
                        merge: VerticalMerge::Continue,
                        blocks: vec![Block::Paragraph(Paragraph::default())],
                        borders: [None; 4],
                        shading: None,
                    });
                    column += usize::from(span.max(1));
                    continue;
                }
                match iter.next() {
                    Some(Some((mut cell, span, rows_spanned))) => {
                        let span = span.min(u16::try_from(count - column).unwrap_or(1)).max(1);
                        cell.span = span;
                        if rows_spanned > 1 {
                            pending[column] = (rows_spanned - 1, span);
                        }
                        // The covered cells beside it.
                        for _ in 1..span {
                            if iter.peek().is_some_and(Option::is_none) {
                                iter.next();
                            }
                        }
                        column += usize::from(span);
                        out.push(cell);
                    }
                    // A covered cell with no cell joining it.
                    Some(None) => {
                        out.push(Cell {
                            span: 1,
                            merge: VerticalMerge::None,
                            blocks: vec![Block::Paragraph(Paragraph::default())],
                            borders: [None; 4],
                            shading: None,
                        });
                        column += 1;
                    }
                    None => break,
                }
            }
            if out.is_empty() {
                continue;
            }
            table_rows.push(Row {
                height,
                heading,
                keep_together: keep,
                page_break_before: breaks,
                cells: out,
            });
        }
        if table_rows.is_empty() {
            return None;
        }
        // A heading row only at the top.
        let mut top = true;
        for row in &mut table_rows {
            top &= row.heading;
            row.heading = top;
        }
        if table_style
            .get("break-before")
            .is_some_and(|value| value == "page")
            && let Some(row) = table_rows.first_mut()
        {
            row.page_break_before = true;
        }
        let align = match table_style.get("align").map(String::as_str) {
            Some("center") => Alignment::Centre,
            Some("right") => Alignment::Right,
            _ => Alignment::Left,
        };
        Some(Block::Table(Table {
            columns,
            rows: table_rows,
            indent: length(table_style.get("margin-left").map(String::as_str))
                .unwrap_or(0)
                .max(0),
            align,
            cell_margins: margins.unwrap_or([0; 4]),
        }))
    }

    fn finish(mut self) -> ExchangeDocument {
        // The body style first (IM4): the file's Body Text where
        // paragraphs use it, else the default style, the others in order.
        let mut names: Vec<String> = self
            .order
            .iter()
            .filter(|name| self.styles.contains_key(*name))
            .cloned()
            .collect();
        let is_body = |reader: &Self, name: &str| {
            let display = reader
                .styles
                .get(name)
                .map_or_else(|| shown_name(name), |entry| entry.display.clone());
            display.eq_ignore_ascii_case("Body Text") || display.eq_ignore_ascii_case("Text body")
        };
        let body = names
            .iter()
            .position(|name| is_body(&self, name) && self.used.contains(name))
            .or_else(|| names.iter().position(|name| name == "Standard"))
            .or_else(|| names.iter().position(|name| is_body(&self, name)))
            .unwrap_or(0);
        if names.is_empty() {
            names.push("Standard".into());
        } else {
            let first = names.remove(body);
            names.insert(0, first);
        }
        let index_of: BTreeMap<&str, usize> = names
            .iter()
            .enumerate()
            .map(|(index, name)| (name.as_str(), index))
            .collect();
        let mut styles = Vec::new();
        for name in &names {
            let (paragraph, text, list, outline) = self.resolve(name);
            let entry = self.styles.get(name).cloned().unwrap_or_default();
            styles.push(Style {
                name: if entry.display.is_empty() {
                    shown_name(name)
                } else {
                    entry.display.clone()
                },
                based_on: entry
                    .parent
                    .as_deref()
                    .and_then(|parent| index_of.get(parent).copied()),
                next: entry
                    .next
                    .as_deref()
                    .and_then(|next| index_of.get(next).copied()),
                paragraph: complete(&paragraph, self.tab_default),
                character: complete_text(&text.props),
                outline_level: outline,
                list: list
                    .and_then(|list| self.list_styles.get(&list))
                    .map(|list| (*list, outline.map_or(0, |level| level.saturating_sub(1)))),
            });
        }
        self.settings.hyphenation = styles
            .iter()
            .any(|style| style.paragraph.no_hyphenation == Some(false));
        let slots: Vec<usize> = self
            .referenced
            .iter()
            .map(|name| index_of.get(name.as_str()).copied().unwrap_or(0))
            .collect();
        let mut sections = std::mem::take(&mut self.sections);
        for section in &mut sections {
            renumber(&mut section.blocks, &slots);
            for set in [&mut section.headers, &mut section.footers] {
                for part in [&mut set.default, &mut set.even, &mut set.first]
                    .into_iter()
                    .flatten()
                {
                    renumber(part, &slots);
                }
            }
        }
        // List paragraphs whose instance is missing are plain paragraphs.
        ExchangeDocument {
            properties: std::mem::take(&mut self.properties),
            fonts: std::mem::take(&mut self.fonts),
            styles,
            lists: std::mem::take(&mut self.lists),
            list_instances: std::mem::take(&mut self.instances),
            sections,
            images: std::mem::take(&mut self.images),
            settings: self.settings.clone(),
        }
    }
}

/// Paragraphs' style slots as the styles' indices.
fn renumber(blocks: &mut [Block], slots: &[usize]) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                paragraph.style = slots.get(paragraph.style).copied().unwrap_or(0);
                for inline in &mut paragraph.inlines {
                    match inline {
                        Inline::Shape(shape) => {
                            if let ShapeContent::Text { blocks, .. } = &mut shape.content {
                                renumber(blocks, slots);
                            }
                        }
                        Inline::Footnote(footnote) => renumber(&mut footnote.blocks, slots),
                        Inline::Note(note) => renumber(&mut note.blocks, slots),
                        _ => {}
                    }
                }
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        renumber(&mut cell.blocks, slots);
                    }
                }
            }
        }
    }
}

/// A page's columns (`style:columns`) over a text width.
fn page_columns(columns: &Element<'_>, text_width: Twips) -> Columns {
    let count = columns
        .attribute(FO, "column-count")
        .and_then(|count| count.parse::<u8>().ok())
        .unwrap_or(1)
        .clamp(1, 8);
    let mut result = Columns {
        count,
        gap: length(columns.attribute(FO, "column-gap"))
            .unwrap_or(720)
            .max(0),
        widths: None,
        line_between: columns
            .child(STYLE, "column-sep")
            .is_some_and(|line| line.attribute(STYLE, "style") != Some("none")),
    };
    let each: Vec<(f64, Twips, Twips)> = columns
        .children()
        .filter(|column| column.is(STYLE, "column"))
        .map(|column| {
            (
                column
                    .attribute(STYLE, "rel-width")
                    .and_then(|width| width.trim().trim_end_matches('*').parse().ok())
                    .unwrap_or(1.0),
                length(column.attribute(FO, "start-indent")).unwrap_or(0),
                length(column.attribute(FO, "end-indent")).unwrap_or(0),
            )
        })
        .collect();
    if each.len() == usize::from(count) && count > 1 {
        let total: f64 = each.iter().map(|(width, ..)| width).sum::<f64>().max(1.0);
        let widths: Vec<Twips> = each
            .iter()
            .map(|(width, start, end)| {
                (width / total * f64::from(text_width)).round() as Twips - start - end
            })
            .collect();
        let gaps: Vec<Twips> = (0..each.len())
            .map(|index| match each.get(index + 1) {
                Some((_, start, _)) => each[index].2 + start,
                None => 0,
            })
            .collect();
        if let Some(gap) = gaps.first() {
            result.gap = (*gap).max(0);
        }
        let even = widths.windows(2).all(|pair| (pair[0] - pair[1]).abs() <= 2)
            && gaps[..gaps.len() - 1]
                .windows(2)
                .all(|pair| (pair[0] - pair[1]).abs() <= 2);
        if !even {
            result.widths = Some(
                widths
                    .into_iter()
                    .zip(gaps)
                    .map(|(width, gap)| (width.max(20), gap.max(0)))
                    .collect(),
            );
        }
    }
    result
}

/// A data style (`number:date-style`) as a Word date picture.
fn data_style_picture(element: &Element<'_>) -> String {
    let mut picture = String::new();
    for part in element.children() {
        let long = part.attribute(NUMBER, "style") == Some("long");
        match part.local() {
            "day" => picture.push_str(if long { "dd" } else { "d" }),
            "month" => picture.push_str(
                match (part.attribute(NUMBER, "textual") == Some("true"), long) {
                    (true, true) => "MMMM",
                    (true, false) => "MMM",
                    (false, true) => "MM",
                    (false, false) => "M",
                },
            ),
            "year" => picture.push_str(if long { "yyyy" } else { "yy" }),
            "day-of-week" => picture.push_str(if long { "dddd" } else { "ddd" }),
            "hours" => picture.push_str(if long { "HH" } else { "H" }),
            "minutes" => picture.push_str(if long { "mm" } else { "m" }),
            "seconds" => picture.push_str(if long { "ss" } else { "s" }),
            "am-pm" => picture.push_str("AM/PM"),
            "text" => picture.push_str(&part.text()),
            _ => {}
        }
    }
    picture
}

/// A clip (`rect(top, right, bottom, left)`) in twips.
fn clip_rect(value: &str) -> Option<[Twips; 4]> {
    let inner = value.trim().strip_prefix("rect(")?.strip_suffix(')')?;
    let parts: Vec<Twips> = inner
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            if part == "auto" {
                Some(0)
            } else {
                length(Some(part))
            }
        })
        .collect::<Option<_>>()?;
    match parts.as_slice() {
        [top, right, bottom, left] => Some([*top, *right, *bottom, *left]),
        _ => None,
    }
}

/// What `props` sets beyond `base`.
fn difference(props: &CharProps, base: &CharProps) -> CharProps {
    fn keep<T: PartialEq + Clone>(value: &Option<T>, base: &Option<T>) -> Option<T> {
        if value.is_some() && value != base {
            value.clone()
        } else {
            None
        }
    }
    CharProps {
        font: keep(&props.font, &base.font),
        size: keep(&props.size, &base.size),
        bold: keep(&props.bold, &base.bold),
        italic: keep(&props.italic, &base.italic),
        underline: keep(&props.underline, &base.underline),
        strike: keep(&props.strike, &base.strike),
        script: keep(&props.script, &base.script),
        caps: keep(&props.caps, &base.caps),
        small_caps: keep(&props.small_caps, &base.small_caps),
        colour: keep(&props.colour, &base.colour),
    }
}

fn push_text(out: &mut Vec<Inline>, text: &str, props: CharProps) {
    if text.is_empty() {
        return;
    }
    if let Some(Inline::Text {
        text: last,
        props: same,
    }) = out.last_mut()
        && *same == props
    {
        last.push_str(text);
        return;
    }
    out.push(Inline::Text {
        text: text.to_owned(),
        props,
    });
}

fn field(kind: FieldKind, result: String, props: CharProps) -> Inline {
    Inline::Field(Field {
        kind,
        result,
        props,
    })
}

/// A frame's name without the counter the app's writer adds (`Logo 3`).
fn strip_counter(name: &str) -> &str {
    match name.rsplit_once(' ') {
        Some((stem, number))
            if !stem.is_empty()
                && !number.is_empty()
                && number.chars().all(|c| c.is_ascii_digit()) =>
        {
            stem
        }
        _ => name,
    }
}

fn number_format(value: &str) -> Option<NumberFormat> {
    Some(match value {
        "1" => NumberFormat::Decimal,
        "a" => NumberFormat::LowerLetter,
        "A" => NumberFormat::UpperLetter,
        "i" => NumberFormat::LowerRoman,
        "I" => NumberFormat::UpperRoman,
        "" => NumberFormat::None,
        _ => return None,
    })
}

/// Paragraph formatting with every field set: ODF's defaults where the
/// style says nothing, the default stops where it has none.
fn complete(props: &ParagraphProps, tab_default: Twips) -> ParagraphProps {
    let mut tabs = props.tabs.clone().unwrap_or_default();
    tabs.retain(|tab| tab.position >= 0);
    if tabs.is_empty() {
        let mut position = tab_default.max(144);
        while position <= 8_640 && tabs.len() < 22 {
            tabs.push(Tab {
                position,
                kind: TabKind::Left,
                leader: TabLeader::None,
            });
            position += tab_default.max(144);
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
        column_break_before: Some(props.column_break_before.unwrap_or(false)),
        tabs: Some(tabs),
        border_top: Some(props.border_top.flatten()),
        border_bottom: Some(props.border_bottom.flatten()),
        no_hyphenation: Some(props.no_hyphenation.unwrap_or(true)),
    }
}

fn complete_text(props: &CharProps) -> CharProps {
    CharProps {
        font: Some(
            props
                .font
                .clone()
                .unwrap_or_else(|| "Liberation Serif".into()),
        ),
        size: Some(props.size.unwrap_or(240)),
        bold: Some(props.bold.unwrap_or(false)),
        italic: Some(props.italic.unwrap_or(false)),
        underline: Some(props.underline.unwrap_or(Underline::None)),
        strike: Some(props.strike.unwrap_or(false)),
        script: Some(props.script.unwrap_or(Script::Normal)),
        caps: Some(props.caps.unwrap_or(false)),
        small_caps: Some(props.small_caps.unwrap_or(false)),
        colour: Some(props.colour.unwrap_or(0)),
    }
}

/// An ISO date and time (`2026-10-21T19:13:00.123`).
fn iso_date(text: &str) -> Option<amipro_document::WallClock> {
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

/// An ISO duration (`PT1H23M`, `P1DT2H`) in minutes.
fn minutes(text: &str) -> u32 {
    let mut total = 0.0;
    let mut number = String::new();
    let mut in_time = false;
    for character in text.chars() {
        match character {
            'T' => in_time = true,
            '0'..='9' | '.' => number.push(character),
            unit => {
                let value: f64 = number.parse().unwrap_or(0.0);
                number.clear();
                total += match (unit, in_time) {
                    ('D', _) => value * 1440.0,
                    ('H', true) => value * 60.0,
                    ('M', true) => value,
                    ('S', true) => value / 60.0,
                    _ => 0.0,
                };
            }
        }
    }
    total.clamp(0.0, f64::from(u32::MAX)) as u32
}

/// Base64 (a flat file's pictures).
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => continue,
        };
        buffer = (buffer << 6 | u32::from(value)) & 0xFF_FFFF;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAT: &str = r#"<?xml version="1.0"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:number="urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
<office:meta><dc:title>Flat</dc:title></office:meta>
<office:styles>
<style:default-style style:family="paragraph"><style:text-properties fo:font-size="12pt"/></style:default-style>
<style:style style:name="Standard" style:family="paragraph"/>
<style:style style:name="Text_20_body" style:display-name="Body Text" style:family="paragraph" style:parent-style-name="Standard"><style:paragraph-properties fo:margin-bottom="0.0972in"/></style:style>
<style:style style:name="Heading_20_1" style:display-name="Heading 1" style:family="paragraph" style:parent-style-name="Standard" style:default-outline-level="1"><style:text-properties fo:font-size="130%" fo:font-weight="bold"/></style:style>
<text:list-style style:name="L1"><text:list-level-style-number text:level="1" style:num-suffix="." style:num-format="1"/><text:list-level-style-number text:level="2" style:num-suffix=")" style:num-format="a"/></text:list-style>
<number:date-style style:name="N1"><number:month number:textual="true" number:style="long"/><number:text> </number:text><number:day/><number:text>, </number:text><number:year number:style="long"/></number:date-style>
</office:styles>
<office:automatic-styles>
<style:style style:name="P1" style:family="paragraph" style:parent-style-name="Text_20_body"><style:paragraph-properties fo:text-align="center"/></style:style>
<style:style style:name="T1" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>
</office:automatic-styles>
<office:body><office:text>
<text:h text:style-name="Heading_20_1" text:outline-level="1">Head</text:h>
<text:p text:style-name="P1">  Centred   <text:span text:style-name="T1">bold</text:span><text:s text:c="2"/>end<text:tab/>x <text:date style:data-style-name="N1">October 21, 2026</text:date></text:p>
<text:list xml:id="list1" text:style-name="L1"><text:list-item><text:p text:style-name="Text_20_body">one</text:p><text:list><text:list-item><text:p text:style-name="Text_20_body">nested</text:p></text:list-item></text:list></text:list-item></text:list>
<text:p text:style-name="Text_20_body">between</text:p>
<text:list text:continue-list="list1" text:style-name="L1"><text:list-item><text:p text:style-name="Text_20_body">two</text:p></text:list-item></text:list>
<table:table><table:table-column table:number-columns-repeated="2"/><table:table-row><table:table-cell table:number-columns-spanned="2" table:number-rows-spanned="2"><text:p>both</text:p></table:table-cell><table:covered-table-cell/></table:table-row><table:table-row><table:covered-table-cell table:number-columns-repeated="2"/></table:table-row><table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell><text:p>b</text:p></table:table-cell></table:table-row></table:table>
</office:text></office:body></office:document>"#;

    fn text_of(paragraph: &Paragraph) -> String {
        paragraph
            .inlines
            .iter()
            .map(|inline| match inline {
                Inline::Text { text, .. } => text.clone(),
                Inline::Tab { .. } => "\t".into(),
                Inline::Field(field) => format!("[{}]", field.result),
                _ => String::new(),
            })
            .collect()
    }

    #[test]
    fn styles_lists_spaces_and_tables() {
        let mut findings = Findings::default();
        let read = read_fodt(FLAT.as_bytes(), &mut findings).unwrap();
        assert_eq!(read.properties.title, "Flat");
        let names: Vec<&str> = read
            .styles
            .iter()
            .map(|style| style.name.as_str())
            .collect();
        assert_eq!(names[0], "Body Text");
        let heading = read
            .styles
            .iter()
            .find(|style| style.name == "Heading 1")
            .unwrap();
        assert_eq!(heading.character.size, Some(312));
        assert_eq!(heading.character.bold, Some(true));
        assert_eq!(heading.outline_level, Some(1));
        let blocks = &read.sections[0].blocks;
        let Block::Paragraph(centred) = &blocks[1] else {
            panic!("a paragraph")
        };
        assert_eq!(read.styles[centred.style].name, "Body Text");
        assert_eq!(centred.props.alignment, Some(Alignment::Centre));
        assert_eq!(text_of(centred), "Centred bold  end\tx [October 21, 2026]");
        let date = centred.inlines.iter().find_map(|inline| match inline {
            Inline::Field(field) => Some(field.kind.clone()),
            _ => None,
        });
        assert!(matches!(date, Some(FieldKind::Date { picture, .. }) if picture == "MMMM d, yyyy"));
        let list = &read.lists[0];
        assert_eq!(list.levels[1].text, "%2)");
        let lists: Vec<_> = blocks
            .iter()
            .filter_map(|block| match block {
                Block::Paragraph(paragraph) => paragraph.list,
                Block::Table(_) => None,
            })
            .collect();
        // One instance, continued after the paragraph between.
        assert_eq!(lists, vec![(Some(0), 0), (Some(0), 1), (Some(0), 0)]);
        let Block::Table(table) = &blocks[6] else {
            panic!("a table")
        };
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.rows[0].cells.len(), 1);
        assert_eq!(table.rows[0].cells[0].span, 2);
        assert_eq!(table.rows[0].cells[0].merge, VerticalMerge::Start);
        assert_eq!(table.rows[1].cells[0].merge, VerticalMerge::Continue);
        assert_eq!(table.rows[1].cells[0].span, 2);
        assert_eq!(table.rows[2].cells.len(), 2);
    }

    #[test]
    fn names_lengths_and_durations() {
        assert_eq!(shown_name("Text_20_body"), "Text body");
        assert_eq!(shown_name("a_b"), "a_b");
        assert_eq!(length(Some("1in")), Some(1_440));
        assert_eq!(length(Some("2.54cm")), Some(1_440));
        assert_eq!(length(Some("12pt")), Some(240));
        assert_eq!(minutes("PT1H23M10S"), 83);
        assert_eq!(strip_counter("Logo 3"), "Logo");
        assert_eq!(strip_counter("Frame1"), "Frame1");
        assert_eq!(base64("aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(clip_rect("rect(1pt, 0pt, 2pt, auto)"), Some([20, 0, 40, 0]));
        assert_eq!(colour(Some("#ff0000")), Some(0xFF));
        assert!(border(Some("none")).is_none());
        assert_eq!(
            border(Some("0.5pt solid #000000")).map(|line| line.width),
            Some(10)
        );
    }
}
