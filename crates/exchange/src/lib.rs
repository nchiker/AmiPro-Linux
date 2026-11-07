//! Export to other word processors' formats (`docs/research/EXPORT_NOTES.md`,
//! `docs/Design/UI_SPEC.md` §30).
//!
//! Ami Pro exported through filters: it walked the document once and handed
//! each format's filter one stream of tokens (`rtf.flt`'s
//! `PROCESSFILTERTOKEN`). This crate works the same way. [`build`] resolves
//! the model once into an [`ExchangeDocument`]: styles over runs, paragraph
//! formats, list numbers, field results, page layouts as sections, and
//! frames placed on their pages, everything in Ami Pro's meaning worked out.
//! Each format's filter ([`docx`], [`odt`], [`rtf`], [`text`], [`markdown`]) only spells
//! that document out; none of them looks at the Ami Pro model. The import
//! run reads each format back into the same form.

pub mod build;
pub mod docx;
pub mod fields;
pub mod import;
pub mod losses;
pub mod markdown;
pub mod numbering;
pub mod odt;
pub mod read_docx;
pub mod read_markdown;
pub mod read_odt;
pub mod read_rtf;
pub mod read_text;
pub mod realise;
pub mod rtf;
pub mod set;
pub mod sheets;
pub mod text;
mod xml;
pub mod xml_tree;
pub mod zip;

use std::sync::Arc;

pub use amipro_document::{Colour, Twips};
pub use build::{BuildOptions, PictureSink, build};
pub use losses::{ExportLoss, Format};

/// A document as every format's filter sees it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExchangeDocument {
    pub properties: Properties,
    /// Every font the document uses, in order of first use.
    pub fonts: Vec<Font>,
    /// Paragraph styles; [`Paragraph::style`] and [`Style::based_on`] index
    /// them. The first is the default (Body Text).
    pub styles: Vec<Style>,
    /// List definitions (abstract lists): levels and their number formats.
    pub lists: Vec<List>,
    /// Instances of the lists: each counts on its own, from its starts.
    pub list_instances: Vec<ListInstance>,
    /// One section per stretch of pages with one page layout.
    pub sections: Vec<Section>,
    /// The pictures, converted; shapes index them.
    pub images: Vec<Image>,
    pub settings: Settings,
}

/// The document's own settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    /// Left and right pages have their own headers (Ami Pro's left pages).
    pub even_and_odd_headers: bool,
    /// Left pages mirror right pages' margins.
    pub mirror_margins: bool,
    /// Words may be split at line ends (some style hyphenates).
    pub hyphenation: bool,
    /// The hyphenation zone in twips.
    pub hyphenation_zone: Twips,
    /// The tables' row heights are Ami Pro's own measure: the app built
    /// the document, or wrote the file (its generator is AmiPro-Linux).
    /// Otherwise a row's height is the file's minimum, which Ami Pro does
    /// not keep (`realise`).
    pub ami_pro_rows: bool,
    /// Footnote Options (Ami Pro's `[fopts]`): the first footnote's
    /// number (0 reads as 1), numbering again on each page, and every
    /// footnote at the end of the document.
    pub footnote_start: u32,
    pub footnotes_each_page: bool,
    pub endnotes: bool,
}

/// The name the app writes as a file's generator.
pub const GENERATOR: &str = "AmiPro-Linux";

/// The document's properties, from Ami Pro's Doc Info.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Properties {
    /// The document's title: its file name without the extension.
    pub title: String,
    /// Ami Pro's description.
    pub description: String,
    pub keywords: String,
    /// The named user fields with a value, by name.
    pub custom: Vec<(String, String)>,
    /// The style sheet, as a template name.
    pub template: String,
    /// Wall-clock times, as Ami Pro shows them.
    pub created: Option<amipro_document::WallClock>,
    pub modified: Option<amipro_document::WallClock>,
    pub revisions: u32,
    pub editing_minutes: u32,
    pub pages: u32,
    pub words: u32,
    pub characters: u32,
}

/// How a font's letters are shaped, from Ami Pro's face kinds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FontFamily {
    Roman,
    Swiss,
    Modern,
    Script,
    Decorative,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    pub name: String,
    pub family: FontFamily,
    /// Fixed-pitch (every character as wide).
    pub fixed: bool,
}

/// A paragraph style: every property set.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub name: String,
    pub based_on: Option<usize>,
    /// The style Enter gives the next paragraph; `None` is this one.
    pub next: Option<usize>,
    pub paragraph: ParagraphProps,
    pub character: CharProps,
    /// Ami Pro's outline level, 1 to 9.
    pub outline_level: Option<u8>,
    /// The list the style's paragraphs belong to, and their level (0-based).
    pub list: Option<(usize, u8)>,
}

/// Paragraph formatting. In a style every field is set; in a paragraph only
/// what it sets over its style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphProps {
    pub alignment: Option<Alignment>,
    /// From the column's left edge, the first line's and the others'.
    pub indent_left: Option<Twips>,
    pub indent_right: Option<Twips>,
    /// The first line's indent beyond `indent_left`: negative hangs.
    pub first_line: Option<Twips>,
    pub space_before: Option<Twips>,
    pub space_after: Option<Twips>,
    pub line: Option<LineSpacing>,
    pub keep_with_next: Option<bool>,
    /// The paragraph's lines stay on one page (Allow break within off).
    pub keep_together: Option<bool>,
    pub page_break_before: Option<bool>,
    pub column_break_before: Option<bool>,
    pub tabs: Option<Vec<Tab>>,
    /// Rules above and below (Ami Pro's Lines), as borders.
    pub border_top: Option<Option<RuleLine>>,
    pub border_bottom: Option<Option<RuleLine>>,
    /// No word of the paragraph is split at a line's end.
    pub no_hyphenation: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Alignment {
    #[default]
    Left,
    Right,
    Centre,
    Justify,
}

/// The height of a paragraph's lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineSpacing {
    /// The fonts' own height.
    Single,
    /// A multiple of the single height (1.5, 2).
    Multiple(f32),
    /// Exactly this height.
    Exact(Twips),
    /// This height, or more for taller text (Ami Pro's printer line
    /// height for the style's font).
    AtLeast(Twips),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tab {
    /// From the column's left edge.
    pub position: Twips,
    pub kind: TabKind,
    pub leader: TabLeader,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabKind {
    Left,
    Centre,
    Right,
    Decimal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabLeader {
    #[default]
    None,
    Dots,
    Hyphens,
    Line,
}

/// A paragraph rule or a table line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleLine {
    pub kind: LineKind,
    /// The whole line's width, every stroke and gap.
    pub width: Twips,
    pub colour: Colour,
    /// The space between the rule and the text.
    pub space: Twips,
}

/// How a line is drawn: one stroke, or several (Ami Pro's styles 8–12).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineKind {
    Single,
    Double,
    Triple,
    ThickThin,
    ThinThick,
}

/// Character formatting. In a style every field is set; in a run only
/// what it sets over its paragraph's style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharProps {
    pub font: Option<String>,
    /// Twentieths of a point.
    pub size: Option<Twips>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<Underline>,
    pub strike: Option<bool>,
    pub script: Option<Script>,
    pub caps: Option<bool>,
    pub small_caps: Option<bool>,
    pub colour: Option<Colour>,
}

impl CharProps {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Sets every property `other` sets.
    pub fn overlay(&mut self, other: &Self) {
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
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Underline {
    #[default]
    None,
    Single,
    Words,
    Double,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Script {
    #[default]
    Normal,
    Super,
    Sub,
}

/// A list: up to nine levels.
#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub levels: Vec<ListLevel>,
}

/// One level of a list: how its number or bullet is shown.
#[derive(Clone, Debug, PartialEq)]
pub struct ListLevel {
    pub format: NumberFormat,
    /// The text shown, with `%1` to `%9` for the levels' numbers; for a
    /// bullet, the bullet character.
    pub text: String,
    pub start: u32,
    /// The level starts again after a paragraph at a higher level.
    pub restart_after_higher: bool,
    /// The font of a bullet character, when not the paragraph's.
    pub font: Option<String>,
    pub props: CharProps,
    /// The number's text is right-aligned in front of the paragraph.
    pub right_align: bool,
    /// The paragraph's indents with this level.
    pub indent_left: Twips,
    pub hanging: Twips,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumberFormat {
    Decimal,
    UpperRoman,
    LowerRoman,
    UpperLetter,
    LowerLetter,
    Bullet,
    /// Only the level's text, no number.
    None,
}

/// One instance of a list: paragraphs pointing at it count together.
#[derive(Clone, Debug, PartialEq)]
pub struct ListInstance {
    pub list: usize,
    /// Where each level starts in this instance, where not the list's start.
    pub starts: [Option<u32>; 9],
}

/// A stretch of pages with one page layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub page: PageSetup,
    /// The section starts on a new page (an inserted page layout) or
    /// continues the page (a floating header or footer from the next page).
    pub new_page: bool,
    pub headers: HeaderFooters,
    pub footers: HeaderFooters,
    pub page_numbers: Option<PageNumbering>,
    pub blocks: Vec<Block>,
}

/// A section's paper and text area.
#[derive(Clone, Debug, PartialEq)]
pub struct PageSetup {
    pub width: Twips,
    pub height: Twips,
    pub landscape: bool,
    pub margin_top: Twips,
    pub margin_bottom: Twips,
    pub margin_left: Twips,
    pub margin_right: Twips,
    /// From the paper's top to the header's text, and its bottom to the
    /// footer's.
    pub header_distance: Twips,
    pub footer_distance: Twips,
    pub columns: Columns,
    /// Lines around the page.
    pub border: Option<PageBorder>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Columns {
    pub count: u8,
    pub gap: Twips,
    /// Uneven columns: each column's width and the gap after it.
    pub widths: Option<Vec<(Twips, Twips)>>,
    pub line_between: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageBorder {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
    pub line: RuleLine,
    /// From the paper's edge to the line.
    pub distance: Twips,
}

/// A section's headers or footers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeaderFooters {
    /// Right (odd) pages, or every page.
    pub default: Option<Vec<Block>>,
    /// Left (even) pages, when they differ.
    pub even: Option<Vec<Block>>,
    /// The section's first page, when it differs (Begin on second page).
    pub first: Option<Vec<Block>>,
}

impl HeaderFooters {
    pub fn is_empty(&self) -> bool {
        self.default.is_none() && self.even.is_none() && self.first.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageNumbering {
    pub format: NumberFormat,
    pub start: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Paragraph(Paragraph),
    Table(Table),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Paragraph {
    pub style: usize,
    pub props: ParagraphProps,
    /// The list instance the paragraph counts in and its level (0-based),
    /// or `Some((None, _))` where its style's list cannot show its number
    /// (then `number_text` is written as text).
    pub list: Option<(Option<usize>, u8)>,
    /// What the paragraph shows before its text (its number or bullet), as
    /// Ami Pro shows it: for plain text, and where no list can show it.
    pub number_text: Option<String>,
    pub inlines: Vec<Inline>,
    /// The paragraph mark's formatting: an empty paragraph's line.
    pub mark: CharProps,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text {
        text: String,
        props: CharProps,
    },
    Tab {
        props: CharProps,
    },
    /// A hard return: a new line in the same paragraph.
    LineBreak,
    PageBreak,
    ColumnBreak,
    Field(Field),
    /// A character drawn in a font of its own (an inserted bullet).
    Symbol {
        character: char,
        font: Option<String>,
        props: CharProps,
    },
    Shape(Box<Shape>),
    /// A comment (Ami Pro's note).
    Note(Box<Note>),
    /// A footnote or endnote.
    Footnote(Box<Footnote>),
    /// Where a bookmark's text starts or ends (Ami Pro's `<:Z…>` marks).
    Bookmark {
        name: String,
        end: bool,
    },
    /// A place an import finding concerns ([`import::Finding::markers`]);
    /// shows nothing.
    Marker(u32),
}

/// A comment on the text, which Ami Pro keeps as a note.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
    pub author: String,
    pub initials: String,
    pub date: Option<amipro_document::WallClock>,
    pub blocks: Vec<Block>,
}

/// A footnote or an endnote: its text, and its reference mark's formatting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Footnote {
    pub endnote: bool,
    pub blocks: Vec<Block>,
    pub props: CharProps,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub kind: FieldKind,
    /// What the field shows now, as Ami Pro showed it.
    pub result: String,
    pub props: CharProps,
}

/// What a field shows, where the formats have the same field.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    Page {
        format: NumberFormat,
    },
    NumPages,
    /// A date: `picture` is Word's date picture (`MMMM d, yyyy`).
    Date {
        source: DateSource,
        picture: String,
        upper: bool,
    },
    Time {
        source: DateSource,
    },
    Title,
    Description,
    Keywords,
    /// A named user field (a custom property).
    Custom(String),
    FileName {
        path: bool,
    },
    Template,
    Revisions,
    EditingTime,
    Words,
    Characters,
    /// A sequence number of a named sequence.
    Seq {
        name: String,
    },
    /// An index entry (Ami Pro's `Index "primary" # "secondary"`): shows
    /// nothing.
    IndexEntry {
        primary: String,
        secondary: String,
    },
    /// A table of contents entry at a level: shows nothing.
    ContentsEntry {
        text: String,
        level: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DateSource {
    Now,
    Created,
    Saved,
}

/// A frame: a text box, a picture, or an empty box.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub id: u32,
    pub name: String,
    pub anchor: Anchor,
    pub width: Twips,
    pub height: Twips,
    /// Back to front: a larger number is drawn over a smaller one.
    pub z: u32,
    pub line: Option<ShapeLine>,
    /// `None`: transparent.
    pub fill: Option<Colour>,
    pub shadow: Option<ShapeShadow>,
    /// Rounded corners: the corner's radius.
    pub corner_radius: Twips,
    pub content: ShapeContent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    /// In the line, like a large character; `drop` is how far its bottom
    /// hangs below the baseline.
    Inline { drop: Twips },
    Floating {
        horizontal: (HorizontalFrom, Twips),
        vertical: (VerticalFrom, Twips),
        wrap: ShapeWrap,
        /// Drawn behind the text.
        behind: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HorizontalFrom {
    Page,
    Margin,
    Column,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerticalFrom {
    Page,
    Margin,
    Paragraph,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShapeWrap {
    /// Text on both sides (Wrap around).
    #[default]
    Square,
    /// No text beside (No wrap beside).
    TopAndBottom,
    /// Text runs through (No wrap around).
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeLine {
    pub kind: LineKind,
    pub width: Twips,
    pub colour: Colour,
    /// Which sides carry the line; the formats draw a whole outline only.
    pub sides: [bool; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeShadow {
    /// The shadow's offset right and down.
    pub dx: Twips,
    pub dy: Twips,
    pub colour: Colour,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeContent {
    Text {
        blocks: Vec<Block>,
        /// From each edge to the text: top, right, bottom, left.
        insets: [Twips; 4],
        columns: u8,
        column_gap: Twips,
    },
    Picture(PlacedPicture),
    Empty,
}

/// A picture in its frame.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedPicture {
    pub image: usize,
    /// Where the picture lies, from the frame's top-left; it may reach past
    /// the frame, which cuts it.
    pub left: Twips,
    pub top: Twips,
    pub width: Twips,
    pub height: Twips,
    pub description: String,
}

/// A picture's bytes in a form the formats take.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub bytes: Arc<[u8]>,
    pub kind: ImageKind,
    /// A PNG for programs that cannot show `kind` (SVG).
    pub fallback: Option<Arc<[u8]>>,
    /// Its size in pixels, where known.
    pub pixels: Option<(u32, u32)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Bmp,
    /// A Windows metafile with its placeable header.
    Wmf,
    /// An enhanced metafile (Word's drawings).
    Emf,
    Tiff,
    Svg,
}

impl ImageKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Wmf => "wmf",
            Self::Emf => "emf",
            Self::Tiff => "tiff",
            Self::Svg => "svg",
        }
    }

    pub fn media_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Bmp => "image/bmp",
            Self::Wmf => "image/x-wmf",
            Self::Emf => "image/x-emf",
            Self::Tiff => "image/tiff",
            Self::Svg => "image/svg+xml",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    /// Each grid column's width, gutter included.
    pub columns: Vec<Twips>,
    pub rows: Vec<Row>,
    /// From the column's left edge to the table's.
    pub indent: Twips,
    /// Where the table sits across the column (a centred line centres a
    /// table alone in it).
    pub align: Alignment,
    /// The space around each cell's text: top, right, bottom, left.
    pub cell_margins: [Twips; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub height: RowHeight,
    /// A heading row, repeated where the table continues.
    pub heading: bool,
    /// The row may not split between pages.
    pub keep_together: bool,
    /// The row starts a new page.
    pub page_break_before: bool,
    pub cells: Vec<Cell>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowHeight {
    AtLeast(Twips),
    Exact(Twips),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// The grid columns the cell covers.
    pub span: u16,
    pub merge: VerticalMerge,
    pub blocks: Vec<Block>,
    /// Top, right, bottom, left.
    pub borders: [Option<RuleLine>; 4],
    pub shading: Option<Colour>,
}

/// How a cell joins the cells above and below it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerticalMerge {
    None,
    /// The top of a joined block.
    Start,
    /// Covered by the block above.
    Continue,
}

/// `colour` (`0x00BBGGRR`) as `RRGGBB`.
pub fn hex(colour: Colour) -> String {
    let [red, green, blue, _] = colour.to_le_bytes();
    format!("{red:02X}{green:02X}{blue:02X}")
}

/// The red, green, and blue of `colour` (`0x00BBGGRR`).
pub fn rgb(colour: Colour) -> (u8, u8, u8) {
    let [red, green, blue, _] = colour.to_le_bytes();
    (red, green, blue)
}
