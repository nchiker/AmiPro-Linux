//! GUI- and file-format-independent document model.
//!
//! A document is an immutable-by-convention root: flows, paragraphs, styles,
//! and layouts sit behind `Arc`, so cloning a root is cheap and an edit
//! copies only the parts it touches (`docs/research/DECISIONS.md`, Phase 2.2,
//! D1 and D6).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use native::unescape;

pub mod bookmarks;
mod codepage;
pub mod contents;
mod fields;
pub mod formula;
mod frame;
mod geometry;
pub mod language;
pub mod merge;
mod native;
pub mod number;
mod power;

pub use codepage::{decode_windows_1252, windows_1252};
pub use fields::{
    DATE_FORMATS, DateField, DateOrder, DateSettings, DateSource, DocInfoEntry, DocInfoField,
    DocumentInfo, FieldContext, Statistics, WallClock, count_words_and_characters, format_date,
    format_time,
};
pub use frame::{
    CellContent, CellFormat, CellLeader, CellLines, CellMargins, CellSpan, EmbeddedObject, Frame,
    FrameContent, FrameStyle, Halftone, ImageAdjust, Offset, PICTURE_MAX, PageSide, Picture,
    PictureFormat, PictureScaling, PictureSizing, PictureSource, Placement, RepeatPages, Shadow,
    TGF_STATE_WORDS, Table, TableCell, TableLayout, TableRows, TableSpacing, Wrap,
    line_style_width,
};
pub use geometry::{
    BLACK, Border, Colour, ColumnEdges, ColumnLine, LayoutBox, LegacyData, LegacyValue,
    LinePosition, Margins, PageGeometry, PageLayout, PageSetup, Rect, SideFrames, Sides, Size,
    TWIPS_PER_POINT, TabKind, TabLeader, TabStop, Twips, WHITE,
};
pub use language::Language;
pub use number::{Displayed, NumberFormat, NumberKind, scan_number};
pub use power::{
    FieldSpan, MarkPlace, PowerMark, SeqField, SetAside, Shown, flag as power_flag, has_result,
    kind as power_kind, letter_number, number_in_style, number_style, power_fields, roman_numeral,
    set_aside_id,
};

/// The signature of the native format the application writes (v3,
/// `docs/Design/NATIVE_FORMAT_V3.md`).
pub const NATIVE_MAGIC: &str = native::MAGIC_V3;
/// Earlier native versions, which still load.
pub const NATIVE_MAGIC_V2: &str = "AMIPRO-LINUX/2";
const NATIVE_MAGIC_V1: &str = "AMIPRO-LINUX/1";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StyleId(pub u64);

macro_rules! object_ids {
    ($($(#[$meta:meta])* $name:ident),* $(,)?) => {$(
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u64);

        impl $name {
            /// Wraps a raw counter value, as stored in native files.
            pub const fn from_raw(raw: u64) -> Self {
                Self(raw)
            }

            /// The raw counter value.
            pub const fn raw(self) -> u64 {
                self.0
            }
        }
    )*};
}

object_ids!(
    /// A text stream: the main text, or the text of a frame, a table cell, or
    /// a floating header or footer.
    FlowId,
    /// A frame.
    FrameId,
    /// A page layout.
    LayoutId,
    /// A group of frames.
    GroupId,
    /// An embedded payload, such as the bytes of a picture.
    ObjectId,
    /// A paragraph. Layout caches use it; positions and commands use indexes.
    ParagraphId,
);

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphStyle {
    pub id: StyleId,
    pub name: String,
    pub parent: Option<StyleId>,
    pub font_family: String,
    pub font_size_points: f32,
    pub bold: bool,
    pub italic: bool,
    pub spacing: Spacing,
    pub indents: Indents,
    pub breaks: Breaks,
    pub alignment: Alignment,
    /// The style's other character attributes and colour (`[fnt]`).
    pub attributes: CharacterAttributes,
    /// Rules above and below the style's paragraphs (`[line]`).
    pub rules: ParagraphRules,
    /// The style Enter at a paragraph's end gives the new paragraph while
    /// Next style is on (`Breaks::next_style`): the name after `[nfmt]`.
    /// `None` is this style itself.
    pub next: Option<StyleId>,
    /// The style's own tabs, which replace its layout's while
    /// `Breaks::own_tabs` is on (decompile `10d0:1881`); kept when off.
    pub tabs: Vec<TabStop>,
    /// Hanging indent (`[algn]` 512): a left tab stop where the lines after
    /// the first start, All + Rest, ahead of the others (decompile).
    pub hanging: bool,
    /// Modify Style › Hyphenation (`[algn]` 256): the word that does not fit
    /// at a line's end may be split (decompile `1310:0000`).
    pub hyphenation: bool,
    /// Bullets & numbers: what stands before each paragraph (`[spec]`).
    pub bullets: Bullets,
    /// Where the definition lives: the document, the style sheet, or
    /// nowhere.
    pub origin: StyleOrigin,
    /// Modify Style › Alignment's Both sides (`[algn]` 16): the right
    /// indent follows All (`indents.right` holds it).
    pub both_sides: bool,
    /// The values of Ami Pro's style record that the model does not use,
    /// as read, so that a written style sheet says what the original said.
    pub kept: StyleKept,
}

/// What Ami Pro writes in a style record (`[tag]`) beyond the model's
/// fields (decompile `11c8:26b7`, the writer; `1058:0fac`, the reader).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleKept {
    /// `[tag]` value 2: the function key that applies the style (2–9 for
    /// F2–F9, 11 and 12 for F11 and F12; 0 none).
    pub function_key: u8,
    /// `[fnt]` value 4's face-kind bits (`0x4000` swiss, `0xC000` roman,
    /// `0x8000` modern).
    pub face_bits: u16,
    /// The dialogs' units: `[algn]` value 2, `[spc]` values 3 and 6.
    pub units: [u8; 3],
    /// `[spc]` value 1's Add in bits (16 Always, 32 When not at break, or
    /// neither, which adds as Always).
    pub add_in: u8,
    /// `[spc]` value 2 with Single spacing: the height the dialog showed
    /// last (layout does not use it).
    pub single_height: Twips,
    /// `[nfmt]`'s six values: Table format's cell format, decimal places,
    /// negative format, and the decimal, thousands, and currency symbols.
    pub number_format: NumberFormat,
    /// The next style's name as written when it names no style here (an
    /// outline style the sheet lacks, or nothing): `next` cannot hold it.
    pub next_name: Option<String>,
    /// `[algn]` value 1's bits 32, 64, and 128, which Ami Pro's reader
    /// clears and older files carry.
    pub alignment_bits: u16,
}

impl Default for StyleKept {
    /// What Ami Pro writes for a new style.
    fn default() -> Self {
        Self {
            function_key: 0,
            face_bits: 0,
            units: [1, 1, 1],
            add_in: 16,
            single_height: 0,
            number_format: NumberFormat::default(),
            next_name: None,
            alignment_bits: 0,
        }
    }
}

/// Where a style's definition lives, as Ami Pro's Style Management shows it
/// (user's guide, chapter 9; VM fixture ST1): in the document (created or
/// modified there, and written into a SAM as a `[tag]`), in the attached
/// style sheet, or nowhere: a name paragraphs still carry after its style
/// was removed or a new style sheet lacks it, which formats as Body Text
/// (Ami Pro shows the name red).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StyleOrigin {
    #[default]
    Document,
    Sheet,
    Missing,
}

/// Whether the text can be a cell's formula as stored: not empty, one line,
/// no control characters, at most 1,024 characters. Whether it compiles is
/// another matter: a file may hold a formula Ami Pro could not read (a stale
/// text), and a table's size can change under it.
pub(crate) fn formula_text_is_valid(text: &str) -> bool {
    !text.is_empty() && text.chars().count() <= 1024 && !text.chars().any(char::is_control)
}

/// The legacy key under which a document whose page layout is its style
/// sheet's keeps what that layout was when it was read, so that the SAM
/// writer stores the layout only once it changes. A command that makes
/// the layout the sheet's again removes it.
pub const SHEET_LAYOUT_KEY: &str = "sheet layout";

/// The legacy key that marks a document whose style sheet was not found
/// when it was read (its value is the sheet's name): the styles and layout
/// standing for the sheet's are defaults, so the SAM loss check reads a
/// written copy back without a sheet, as the document was read. A command
/// that gives the document a sheet removes it.
pub const STYLE_SHEET_MISSING_KEY: &str = "style sheet missing";

/// Ami Pro's hyphenation zone for documents that do not set one.
pub const DEFAULT_HYPHENATION_ZONE: u16 = 5;

/// The style sheet a document uses (`[sty]`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleSheetLink {
    /// The file name as Ami Pro writes it (`_DEFAULT.STY`).
    pub name: String,
    /// The page layout is the sheet's, not stored in the document (a SAM
    /// without `[lay]`), so another style sheet replaces it.
    pub layout_from_sheet: bool,
}

/// A style sheet's styles and standard page layout, as Use Another Style
/// Sheet applies them (`amipro_formats::read_style_sheet`).
#[derive(Clone, Debug, PartialEq)]
pub struct StyleSheet {
    pub name: String,
    /// IDs are the sheet's own; `next` names other styles of the sheet.
    pub styles: Vec<ParagraphStyle>,
    /// Its first page layout: only the page's settings are used.
    pub layout: Option<PageLayout>,
}

/// A style's Modify Style › Bullets & numbers (`[spec]`, ten values;
/// decompile `1058:0fac`, `1200:1384`, `1200:0af0`; VM fixtures BN1, BN2).
/// The text before a paragraph's first line holds bullet codes, number
/// codes, and plain text; the paragraph's text follows it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bullets {
    /// The text before the paragraph: U+0010–U+0019 bullets (a small dot,
    /// a dot, a small and a large square, an open square, a small and a
    /// large diamond, a small and a large ring, a check mark), U+0008 and
    /// U+000A–U+000F Ami Pro's bitmaps, U+001A arabic numbers, U+001B and
    /// U+001C upper and lower roman, U+001D and U+001E upper and lower
    /// letters, U+001F asterisks and U+0007 daggers (as many as the
    /// number), and any other character as itself. Empty: none.
    pub text: String,
    /// The outline level, 0 for none (value 2): a level's number counts
    /// every paragraph at that level, whatever its style, and restarts as
    /// `flags` say (`1200:1842`).
    pub level: u8,
    /// The text's attributes as `[fnt]` flags (value 7): 1 bold, 2 italic,
    /// 4 underline, 8 word underline, 16 upper case, 256 superscript.
    pub attributes: u16,
    /// Value 8: 2 restarts after a higher level, 4 after any other level
    /// or none (After intervening style), 8 makes the number codes count
    /// levels 1, 2, 3… (Cumulative), and 16 is Right align; the others are
    /// kept.
    pub flags: u16,
    /// Values 1, 4, 5, 6, 9, and 10, kept as written (their meaning is not
    /// known; value 4 is not the dialog's Space for).
    pub kept: [u32; 6],
}

impl Default for Bullets {
    /// None: what Ami Pro writes for a style without them
    /// (`0 0  0 1 1 0 0 0 0`).
    fn default() -> Self {
        Self {
            text: String::new(),
            level: 0,
            attributes: 0,
            flags: 0,
            kept: [0, 0, 1, 1, 0, 0],
        }
    }
}

impl Bullets {
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Whether the text counts the paragraphs (a number code).
    pub fn numbered(&self) -> bool {
        self.text
            .chars()
            .any(|code| matches!(code, '\u{7}' | '\u{1A}'..='\u{1F}'))
    }
}

/// A style's paragraph rules, Ami Pro's Modify Style › Lines (`[line]`;
/// decompile `1058:0fac`, `10d0:0d82`, `1288:0000`; VM fixtures NX1, NX2).
/// A rule takes `space` twips, its own thickness included, at the top of
/// the paragraph's first line (above) or the bottom of its last (below),
/// outside the line spacing and inside the space above and below.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParagraphRules {
    pub above: RuleSide,
    pub below: RuleSide,
    pub length: RuleLength,
    /// Length › Other, in twips; kept when another length is chosen.
    pub other_length: Twips,
    /// A Windows `COLORREF`, `0x00BBGGRR`.
    pub colour: Colour,
    /// What Ami Pro stores without a known meaning, kept for a writer:
    /// `[line]` flag bits other than 1, 2, 4, and 8, values 3 and 5, and
    /// value 10 (the dialog's unit, 1 inches).
    pub other_flags: u16,
    pub unknown: [u8; 2],
    pub unit: u8,
}

/// One side's rule. Ami Pro keeps the line and space of a side that is off.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleSide {
    pub on: bool,
    /// Ami Pro's line style, 1–12 (`rule_strokes`).
    pub line: u8,
    /// The height the rule takes, its thickness included.
    pub space: Twips,
}

/// How long a paragraph rule is drawn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleLength {
    /// The first line's text (above) or the last line's (below).
    Text,
    /// The column, whatever the indents.
    Margins,
    /// `ParagraphRules::other_length`, placed as the line is aligned.
    Other,
}

impl Default for ParagraphRules {
    /// No rules: what Ami Pro writes for a style without them
    /// (`8 0 1 0 1 1 1 10 10 1`).
    fn default() -> Self {
        let off = RuleSide {
            on: false,
            line: 1,
            space: 10,
        };
        Self {
            above: off,
            below: off,
            length: RuleLength::Margins,
            other_length: 0,
            colour: 0,
            other_flags: 0,
            unknown: [1, 1],
            unit: 1,
        }
    }
}

impl ParagraphRules {
    /// The height the rules add to a paragraph's first and last lines.
    pub fn space(&self) -> (Twips, Twips) {
        let side = |side: RuleSide| if side.on { side.space } else { 0 };
        (side(self.above), side(self.below))
    }

    /// The `[line]` flag word: 1 above, 2 below, 4 Length Text, 8 Margins,
    /// and the bits kept from the file.
    pub fn flags(&self) -> u16 {
        let length = match self.length {
            RuleLength::Text => 4,
            RuleLength::Margins => 8,
            RuleLength::Other => 0,
        };
        self.other_flags | u16::from(self.above.on) | u16::from(self.below.on) << 1 | length
    }
}

/// The strokes and gaps of Ami Pro's line style `line`, in twips from the
/// rule's top: strokes at even positions, gaps between (decompile
/// `1288:0000`, `1288:0800`). 1–7 are single lines of 10 to 240 twips; 8
/// double, 9 double heavy, 10 triple, 11 thick over thin, 12 thin over
/// thick. Others draw nothing.
pub fn rule_strokes(line: u8) -> &'static [Twips] {
    match line {
        1 => &[10],
        2 => &[20],
        3 => &[40],
        4 => &[80],
        5 => &[120],
        6 => &[160],
        7 => &[240],
        8 => &[20, 20, 20],
        9 => &[40, 20, 40],
        10 => &[20, 20, 40, 20, 20],
        11 => &[60, 20, 20],
        12 => &[20, 20, 60],
        _ => &[],
    }
}

/// Ami Pro's capitalisation attributes (Text › Caps). Each excludes the
/// others. The text keeps the letters as typed; only the display changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Caps {
    Upper,
    Lower,
    /// The first letter of each word upper case, the others lower case.
    Initial,
    /// Lower-case letters shown as capitals at 7/10 of the size.
    Small,
    /// The letters as typed, over a style that sets caps: a run's way to
    /// switch its style's caps off. Ami Pro's marks cannot say this (its
    /// caps off returns to the style); the app's Caps commands can.
    Normal,
}

/// The character attributes beyond face, size, bold, and italic that a
/// style sets for its text (`[fnt]` flags and colour;
/// `docs/research/SAM_FORMAT_NOTES.md`, "Style records").
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CharacterAttributes {
    pub underline: bool,
    /// Underlines words but not the spaces between them.
    pub word_underline: bool,
    pub double_underline: bool,
    pub strikethrough: bool,
    /// Drawn at 7/12 of the size, raised by 5/12 of the size. When both are
    /// set, superscript applies.
    pub superscript: bool,
    /// Drawn at 7/12 of the size, lowered by half the smaller font's ascent.
    pub subscript: bool,
    pub caps: Option<Caps>,
    /// A Windows `COLORREF`, `0x00BBGGRR`.
    pub colour: Colour,
    /// Protected text (Edit › Mark Text; `[fnt]` 4096 in a style): the
    /// caret steps over it and a click in it lands at its end, but Delete
    /// and Backspace remove it (VM fixture PT1).
    pub protected: bool,
    /// No hyphenation (Edit › Mark Text): never split at a line's end.
    pub no_hyphenation: bool,
    /// Overstrike: the character drawn over each character (Text ›
    /// Special Effects; VM fixture O1). A style never sets it.
    pub overstrike: Option<char>,
}

impl CharacterAttributes {
    pub fn is_default(self) -> bool {
        self == Self::default()
    }

    /// Superscript or subscript: text set at 7/12 of the size.
    pub fn is_script(self) -> bool {
        self.superscript || self.subscript
    }
}

/// How a paragraph's lines sit in their slot: `[algn]` flags 1, 2, 4, 8 in a
/// style (I1–I3) and Text › Alignment over one paragraph (O2). A justified
/// paragraph's last line is set left.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Alignment {
    #[default]
    Left,
    Right,
    Centre,
    Justify,
}

/// A paragraph style's break rules, as Ami Pro's Modify Style › Breaks sets
/// them and `[brk]` stores them (VM fixtures B1–B8). The bits are kept as
/// written, so the Next style switch (64) and the unknown 32 survive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Breaks(pub u16);

impl Default for Breaks {
    /// Ami Pro's default: a page or column may break within the paragraph.
    fn default() -> Self {
        Self(Self::WITHIN)
    }
}

impl Breaks {
    pub const PAGE_BEFORE: u16 = 1;
    pub const PAGE_AFTER: u16 = 2;
    pub const WITHIN: u16 = 4;
    pub const KEEP_WITH_PREVIOUS: u16 = 8;
    pub const KEEP_WITH_NEXT: u16 = 16;
    /// The style has its own tabs, replacing its layout's (decompile).
    pub const OWN_TABS: u16 = 32;
    pub const NEXT_STYLE: u16 = 64;
    pub const COLUMN_BEFORE: u16 = 128;
    pub const COLUMN_AFTER: u16 = 256;

    fn has(self, bit: u16) -> bool {
        self.0 & bit != 0
    }

    pub fn page_before(self) -> bool {
        self.has(Self::PAGE_BEFORE)
    }

    pub fn page_after(self) -> bool {
        self.has(Self::PAGE_AFTER)
    }

    pub fn column_before(self) -> bool {
        self.has(Self::COLUMN_BEFORE)
    }

    pub fn column_after(self) -> bool {
        self.has(Self::COLUMN_AFTER)
    }

    /// A page or column may break between the paragraph's lines.
    pub fn within(self) -> bool {
        self.has(Self::WITHIN)
    }

    pub fn keep_with_previous(self) -> bool {
        self.has(Self::KEEP_WITH_PREVIOUS)
    }

    pub fn keep_with_next(self) -> bool {
        self.has(Self::KEEP_WITH_NEXT)
    }

    /// Enter at the paragraph's end gives the new paragraph the style's
    /// next style (`ParagraphStyle::next`; VM fixture NX1).
    pub fn next_style(self) -> bool {
        self.has(Self::NEXT_STYLE)
    }

    pub fn own_tabs(self) -> bool {
        self.has(Self::OWN_TABS)
    }
}

/// A paragraph style's spacing, as Ami Pro's Modify Style › Spacing sets it
/// (`[spc]`; VM fixtures S1–S6).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Spacing {
    pub line: LineSpacing,
    /// Space above and below the paragraph in twips. Between two paragraphs
    /// both apply (S4).
    pub above: Twips,
    pub below: Twips,
    /// "Add in: Always" (or neither choice): the space above is added at
    /// the top of a page or column too, and the space below belongs to the
    /// paragraph's last line, which must fit with it. "When not at break"
    /// (false) skips the space above at a column top and counts the space
    /// below only before another line in the same column (S4, S5,
    /// decompile).
    pub add_at_break: bool,
    /// Text Tightness in percent: 90 tight, 100 normal, 115 loose (S6). Kept;
    /// not laid out yet.
    pub tightness: u16,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            line: LineSpacing::Single,
            above: 0,
            below: 0,
            add_at_break: false,
            tightness: 100,
        }
    }
}

impl Spacing {
    /// Space below only.
    pub fn below(below: Twips) -> Self {
        Self {
            below,
            ..Self::default()
        }
    }
}

/// The height of a paragraph's lines. Single lines take their fonts'
/// height; Custom the height stored; 1 1/2 and Double are computed from
/// each line's single height h, as h + ⌊h/2⌋ and 2h (decompile; LS1). The
/// height a style stores for 1 1/2 or Double (0 when Text › Spacing set it)
/// is the Spacing dialog's cache: kept for writing, not laid out. The extra
/// height is split equally above and below the text (S1–S3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineSpacing {
    Single,
    OneAndAHalf(Twips),
    Double(Twips),
    Custom(Twips),
}

impl LineSpacing {
    /// The fixed line height in twips, `None` for single spacing.
    pub fn height(self) -> Option<Twips> {
        match self {
            Self::Single => None,
            Self::OneAndAHalf(height) | Self::Double(height) | Self::Custom(height) => Some(height),
        }
    }

    /// Heights are positive, except 0 (not stored) for 1 1/2 and Double.
    pub fn is_valid(self) -> bool {
        match self {
            Self::Single => true,
            Self::OneAndAHalf(height) | Self::Double(height) => height >= 0,
            Self::Custom(height) => height > 0,
        }
    }

    /// The line height for a line whose single height is `single`, `None`
    /// for single spacing.
    pub fn height_for(self, single: f32) -> Option<f32> {
        match self {
            Self::Single => None,
            Self::OneAndAHalf(_) => Some(single + (single / 2.0).floor()),
            Self::Double(_) => Some(single * 2.0),
            Self::Custom(height) => Some(height as f32),
        }
    }
}

/// The single line heights Ami Pro gave this document's fonts: its
/// printer's metrics, which differ from printer to printer and from the
/// fonts' own (`docs/research/DECISIONS.md`, 2026-10-12, "Line heights
/// follow the printer"). The SAM importer reads them from Ami Pro's
/// paragraph caches; a font without one takes its own metrics.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LineHeights {
    heights: BTreeMap<FaceKey, Twips>,
}

/// A font as styles and runs name it: the face, compared without regard to
/// letter case, the size in twentieths of a point, and weight and slant.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FaceKey {
    pub family: String,
    pub size: Twips,
    pub bold: bool,
    pub italic: bool,
}

impl FaceKey {
    pub fn new(family: &str, size_points: f32, bold: bool, italic: bool) -> Self {
        Self {
            family: family.trim().to_lowercase(),
            size: points_to_twips(size_points),
            bold,
            italic,
        }
    }
}

impl LineHeights {
    pub fn get(&self, key: &FaceKey) -> Option<Twips> {
        self.heights.get(key).copied()
    }

    pub fn insert(&mut self, key: FaceKey, height: Twips) {
        self.heights.insert(key, height);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&FaceKey, Twips)> {
        self.heights.iter().map(|(key, height)| (key, *height))
    }

    pub fn len(&self) -> usize {
        self.heights.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heights.is_empty()
    }
}

/// A paragraph style's indents in twips, as Ami Pro's Modify Style ›
/// Alignment sets them: `all` from the left of the column, `first` and
/// `rest` added to it for the first line and the others, and `right` from
/// the right of the column (VM fixtures I1–I3).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Indents {
    pub all: Twips,
    pub first: Twips,
    pub rest: Twips,
    pub right: Twips,
}

impl Indents {
    /// The left indent of the first line (`first`) or of the others.
    pub fn left(self, first: bool) -> Twips {
        self.all + if first { self.first } else { self.rest }
    }

    pub fn is_zero(self) -> bool {
        self == Self::default()
    }
}

/// Formatting set on a paragraph itself, over its style's: Ami Pro's Text ›
/// Indention, Text › Spacing, and Text › Alignment (VM fixtures O2, O3, O6),
/// and its own tab ruler (`<:R…>`), which comes before the style's and the
/// column's tabs (decompile `10d0:1881`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParagraphFormat {
    pub indents: Option<Indents>,
    pub line_spacing: Option<LineSpacing>,
    pub alignment: Option<Alignment>,
    pub tabs: Option<Vec<TabStop>>,
}

impl ParagraphFormat {
    pub fn is_empty(self) -> bool {
        self == Self::default()
    }

    /// Puts this formatting over `style`. A 1 1/2 or Double line spacing
    /// without a stored height takes the style's when the style has the same
    /// spacing. The paragraph's tab ruler becomes the style's own tabs, which
    /// it outranks.
    pub fn apply(&self, style: &mut ParagraphStyle) {
        if let Some(tabs) = &self.tabs {
            style.tabs.clone_from(tabs);
            style.breaks = Breaks(style.breaks.0 | Breaks::OWN_TABS);
        }
        if let Some(indents) = self.indents {
            style.indents = indents;
        }
        if let Some(alignment) = self.alignment {
            style.alignment = alignment;
        }
        if let Some(line) = self.line_spacing {
            style.spacing.line = match (line, style.spacing.line) {
                (LineSpacing::OneAndAHalf(0), stored @ LineSpacing::OneAndAHalf(_))
                | (LineSpacing::Double(0), stored @ LineSpacing::Double(_)) => stored,
                _ => line,
            };
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Paragraph {
    pub id: ParagraphId,
    pub text: String,
    pub style: StyleId,
    pub format: ParagraphFormat,
    /// Sparse, non-overlapping byte ranges carrying direct character formatting.
    pub runs: Vec<CharacterRun>,
    /// One object per U+FFFC in `text`, in text order: the k-th U+FFFC is the
    /// k-th object. Objects occur only in the main flow.
    pub objects: Vec<InlineObject>,
    /// Zero-width legacy marks, sorted by byte offset.
    pub marks: Vec<LegacyMark>,
    /// The character format in force at the paragraph's end, after its last
    /// character: the format of its paragraph mark. Ami Pro sets an empty
    /// paragraph's line in this font (decompile `10d0:0592`; a `<:f200,,>`
    /// line in a 12 pt style is 10 pt high), and text typed into an empty
    /// paragraph takes it. Lines with text are measured by their characters
    /// alone (`docs/research/DECISIONS.md`, "The paragraph's end format").
    pub end_format: CharacterFormat,
    /// The layout cache Ami Pro stored with the paragraph, kept to be
    /// written back while the paragraph is as it was read.
    pub cache: Option<ParagraphCache>,
}

/// The layout cache Ami Pro stores with a paragraph (`<:#H,W>`): its height
/// and the width of the column it was laid out in. Ami Pro trusts a cache
/// whose width is the column's, so a SAM writer gives one back only while
/// the paragraph and its style are as they were when it was read
/// (`fingerprint`; VM fixtures K1–K6, `docs/research/SAM_FORMAT_NOTES.md`,
/// "Writing documents").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParagraphCache {
    pub height: i32,
    pub width: i32,
    /// [`Paragraph::content_fingerprint`] when the cache was read.
    pub fingerprint: u64,
}

/// A 64-bit FNV-1a hash of `bytes`, continuing from `hash`.
fn fnv1a(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

impl ParagraphStyle {
    /// The tab stops in effect for a paragraph whose style, with its own
    /// formatting over it ([`ParagraphFormat::apply`]), is `self`, in a box
    /// whose list is `box_tabs` ([`LayoutBox::tab_stops`]): its own ruler,
    /// else its style's own tabs while they are on, else the box's
    /// (`1148:14c5`; `docs/research/RULER_NOTES.md`). The list a ruler
    /// change copies.
    pub fn tabs_in_effect<'a>(&'a self, box_tabs: &'a [TabStop]) -> &'a [TabStop] {
        if self.breaks.own_tabs() {
            &self.tabs
        } else {
            box_tabs
        }
    }

    /// A hash of the style's definition, which a paragraph's
    /// [`Paragraph::content_fingerprint_with`] includes. Stable across
    /// builds.
    pub fn fingerprint(&self) -> u64 {
        let mut style = self.clone();
        style.id = StyleId(0);
        style.parent = None;
        style.origin = StyleOrigin::Document;
        fnv1a(FNV_OFFSET, format!("{style:?}").as_bytes())
    }
}

impl Paragraph {
    /// A hash of what sets the paragraph's height: its text, formatting,
    /// objects, marks, and its style's definition. Stable across builds.
    pub fn content_fingerprint(&self, style: &ParagraphStyle) -> u64 {
        self.content_fingerprint_with(style.fingerprint())
    }

    /// [`Self::content_fingerprint`] with the style's
    /// [`ParagraphStyle::fingerprint`] given, for many paragraphs of one
    /// style.
    pub fn content_fingerprint_with(&self, style: u64) -> u64 {
        let content = format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|",
            self.text, self.format, self.runs, self.objects, self.marks, self.end_format
        );
        fnv1a(fnv1a(FNV_OFFSET, content.as_bytes()), &style.to_le_bytes())
    }
}

/// The character that stands for an inline object in paragraph text.
pub const OBJECT_REPLACEMENT_CHARACTER: char = '\u{FFFC}';

/// What replaces U+FFFC in text that comes from input or the clipboard, so
/// that no object is ever invented.
pub const REPLACEMENT_CHARACTER: char = '\u{FFFD}';

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BreakKind {
    Page,
    Column,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeaderFooterKind {
    Header,
    Footer,
}

/// Which pages a floating header or footer applies to. Right pages are odd.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageSides {
    All,
    Right,
    Left,
}

/// An object that occupies one place in paragraph text, written there as
/// U+FFFC (`docs/research/DECISIONS.md`, Phase 2.2, D3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InlineObject {
    /// A page or column break. A page break may vertically centre the text
    /// before it and may switch to another layout from the next page; a
    /// column break does neither.
    Break {
        kind: BreakKind,
        centre: bool,
        layout: Option<LayoutId>,
        /// Ami Pro's break after a generated TOC (flag 0x02): the pages
        /// after it are numbered from the start again, as if the pages
        /// before did not exist (VM TI1T2, TI3T).
        restart: bool,
    },
    /// A floating header or footer, which applies from the next page. It owns
    /// its flow.
    FloatingHeaderFooter {
        kind: HeaderFooterKind,
        pages: PageSides,
        flow: FlowId,
    },
    /// The anchor of a With para above or Flow with text frame. It owns the
    /// frame.
    FrameAnchor(FrameId),
    /// A page-number field, which may occur in any flow (headers need it).
    PageNumber(PageNumberField),
    /// A bullet put into the text (Ami Pro's Edit › Insert › Bullet…,
    /// `<:B…>`): one of [`BULLET_CODES`], drawn as the style bullet with
    /// that code in the font where it stands
    /// (`docs/research/BULLETS_NOTES.md`).
    Bullet(char),
    /// A date or the time (Ami Pro's Insert Date/Time: `<:D…>`, `<:r…>`,
    /// `<:k…>`, `<:b…>`), shown as worked out when its line is laid out
    /// (`docs/research/FIELDS_NOTES.md`).
    Date(DateField),
    /// A Doc Info entry (Insert Doc Info Field, `<:d…>`), shown as worked
    /// out when its line is laid out.
    DocInfo(DocInfoField),
    /// A Seq power field (`<:X4,…>`), which shows its number.
    Seq(SeqField),
    /// A note (Ami Pro's Edit › Insert › Note, `<:N…>`): its author's
    /// initials, colour, time, and number, and its text, which it owns as a
    /// flow that is never laid out on the page. It takes no width; it may
    /// stand in any text (`docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`).
    Note { note: NoteInfo, flow: FlowId },
    /// A footnote's reference (Tools › Footnotes, `<:F`) in the main text.
    /// It owns the footnote's text as a flow, laid out in the page's
    /// footnote area (or after the text as endnotes); its number is worked
    /// out from its place.
    Footnote { flow: FlowId },
    /// A footnote's reference in a table cell (`<:xN,ROW>`): its number in
    /// the table and the row at the table's end that holds its text, as
    /// read or as the commands keep them.
    TableFootnote { number: u16, row: u16 },
}

/// A note's own fields (`<:NTIME,INITIALS,COLOUR,NUMBER,MAIN`; VM RNN1–RNN3).
/// Ami Pro's older short form `<:NTIME` has no initials, colour, or number:
/// they are `None` and it is written back so.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoteInfo {
    /// When the note was made or last changed, in Ami Pro's time base
    /// (seconds; [`fields::WallClock::from_ami_pro`]).
    pub time: u32,
    pub initials: Option<String>,
    pub colour: Option<Colour>,
    /// The note's number: 1, 2, 3 in the main text, counted separately
    /// elsewhere, 0 in headers, footers, and footnotes; Ami Pro renumbers on
    /// every insertion and removal, and so do the commands.
    pub number: Option<u16>,
}

impl NoteInfo {
    /// Ami Pro's default note colour, yellow.
    pub const YELLOW: Colour = 0x00_FFFF;
}

/// Ami Pro's Footnote Options (`[fopts]`; decompile `1058:228d`, dialog
/// `FOOTOPTSPROC`; VM RNF1–RNF4): flags, the starting number, the
/// separator's custom length and indent (twips), and the two continued
/// messages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootnoteOptions {
    pub flags: u8,
    pub start: u16,
    pub length: Twips,
    pub indent: Twips,
    pub continued_on: String,
    pub continued_from: String,
}

impl Default for FootnoteOptions {
    fn default() -> Self {
        Self {
            flags: 0,
            start: 1,
            length: 0,
            indent: 0,
            continued_on: String::new(),
            continued_from: String::new(),
        }
    }
}

impl FootnoteOptions {
    pub const ENDNOTES: u8 = 0x01;
    pub const RESET_EACH_PAGE: u8 = 0x02;
    pub const CUSTOM_LENGTH: u8 = 0x04;
    pub const CONTINUED_ON: u8 = 0x08;
    pub const CONTINUED_FROM: u8 = 0x10;
    pub const CONTINUED_ON_LEFT: u8 = 0x20;
    pub const CONTINUED_FROM_LEFT: u8 = 0x40;
    pub const REPEAT_NUMBER: u8 = 0x80;

    pub fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    pub fn set(&mut self, flag: u8, on: bool) {
        if on {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    pub fn endnotes(&self) -> bool {
        self.has(Self::ENDNOTES)
    }

    pub fn reset_each_page(&self) -> bool {
        self.has(Self::RESET_EACH_PAGE)
    }

    /// The "Continued on" message, when it is on and has text.
    pub fn continued_on_message(&self) -> Option<&str> {
        (self.has(Self::CONTINUED_ON) && !self.continued_on.is_empty())
            .then_some(self.continued_on.as_str())
    }

    /// The "Continued from" message, when it is on and has text.
    pub fn continued_from_message(&self) -> Option<&str> {
        (self.has(Self::CONTINUED_FROM) && !self.continued_from.is_empty())
            .then_some(self.continued_from.as_str())
    }
}

/// Ami Pro's Page › Line Numbering (`[lnopts]`; decompile `1368:0295`,
/// dialog 1130; VM fixtures LN1–LN20, `docs/research/COMMANDS_NOTES.md`
/// §8): the flag byte, the "Based on" style's name, and the count of
/// "Every N lines". Belongs to the document, not a page layout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineNumbering {
    pub flags: u8,
    pub style: String,
    pub interval: u16,
}

impl Default for LineNumbering {
    /// What Ami Pro writes for a new document: off, Every line, Body Text,
    /// 1 (LN1).
    fn default() -> Self {
        Self {
            flags: Self::EVERY_LINE,
            style: "Body Text".into(),
            interval: 1,
        }
    }
}

/// What Line Numbering numbers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineNumberingMode {
    None,
    /// Every line slot from the top margin to the bottom margin, at the
    /// "Based on" style's pitch (LN2, LN10, LN13).
    AllLines,
    /// The main text's lines that hold text, in the first column (LN3,
    /// LN9, LN20).
    TextLines,
}

/// Which counts Line Numbering shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineNumberingEvery {
    Line,
    OtherLine,
    FifthLine,
    /// Every N lines (the count, 1–99 in Ami Pro's dialog).
    Lines,
}

impl LineNumbering {
    pub const ALL_LINES: u8 = 0x01;
    pub const EVERY_LINE: u8 = 0x02;
    pub const EVERY_OTHER: u8 = 0x04;
    pub const EVERY_FIFTH: u8 = 0x08;
    pub const RESET_EACH_PAGE: u8 = 0x10;
    pub const EVERY_N: u8 = 0x20;
    pub const TEXT_LINES: u8 = 0x80;
    /// The dialog's limits of the count (LN18: 0 and 100 refused).
    pub const INTERVAL_RANGE: std::ops::RangeInclusive<u16> = 1..=99;

    /// On when either mode's bit is set (the drawing tests `& 0x81`); text
    /// lines win when both are (the dialog never writes both).
    pub fn mode(&self) -> LineNumberingMode {
        if self.flags & Self::TEXT_LINES != 0 {
            LineNumberingMode::TextLines
        } else if self.flags & Self::ALL_LINES != 0 {
            LineNumberingMode::AllLines
        } else {
            LineNumberingMode::None
        }
    }

    pub fn is_on(&self) -> bool {
        self.mode() != LineNumberingMode::None
    }

    /// The dialog's radio: the first of its bits set, else Every N (as the
    /// dialog opens on a value with none of them).
    pub fn every(&self) -> LineNumberingEvery {
        if self.flags & Self::EVERY_LINE != 0 {
            LineNumberingEvery::Line
        } else if self.flags & Self::EVERY_OTHER != 0 {
            LineNumberingEvery::OtherLine
        } else if self.flags & Self::EVERY_FIFTH != 0 {
            LineNumberingEvery::FifthLine
        } else {
            LineNumberingEvery::Lines
        }
    }

    pub fn reset_each_page(&self) -> bool {
        self.flags & Self::RESET_EACH_PAGE != 0
    }

    /// The flags as the dialog writes them for these choices (`1368:0295`).
    pub fn flags_for(mode: LineNumberingMode, every: LineNumberingEvery, reset: bool) -> u8 {
        let mode = match mode {
            LineNumberingMode::None => 0,
            LineNumberingMode::AllLines => Self::ALL_LINES,
            LineNumberingMode::TextLines => Self::TEXT_LINES,
        };
        let every = match every {
            LineNumberingEvery::Line => Self::EVERY_LINE,
            LineNumberingEvery::OtherLine => Self::EVERY_OTHER,
            LineNumberingEvery::FifthLine => Self::EVERY_FIFTH,
            LineNumberingEvery::Lines => Self::EVERY_N,
        };
        mode | every | if reset { Self::RESET_EACH_PAGE } else { 0 }
    }

    /// These settings with one choice changed, as the dialog would write
    /// them.
    pub fn with(&self, mode: LineNumberingMode, every: LineNumberingEvery, reset: bool) -> Self {
        Self {
            flags: Self::flags_for(mode, every, reset),
            ..self.clone()
        }
    }

    /// Whether count `count` (from 1) shows its number: every line all,
    /// every other the odd counts, every fifth and every N their multiples
    /// (LN4–LN6). A count of 0 is taken as 1.
    pub fn shows(&self, count: u32) -> bool {
        match self.every() {
            LineNumberingEvery::Line => true,
            LineNumberingEvery::OtherLine => count % 2 == 1,
            LineNumberingEvery::FifthLine => count % 5 == 0,
            LineNumberingEvery::Lines => count % u32::from(self.interval.max(1)) == 0,
        }
    }
}

/// Ami Pro's bullets in its Insert Bullet list order (decompile
/// `1560:0122`; VM fixture BL1): a small dot, a dot, a small and a large
/// square, an open square, a small and a large diamond, a small and a
/// large ring, a check mark, then its seven pictures (a pointing hand, a
/// box with a shadow, a box with a thick top, a checked box, a crossed
/// box, and two arrowheads). Style bullets (`Bullets::text`) use the same
/// codes.
pub const BULLET_CODES: [char; 17] = [
    '\u{10}', '\u{11}', '\u{12}', '\u{13}', '\u{14}', '\u{15}', '\u{16}', '\u{17}', '\u{18}',
    '\u{19}', '\u{8}', '\u{A}', '\u{B}', '\u{C}', '\u{D}', '\u{E}', '\u{F}',
];

/// Whether `code` is one of Ami Pro's bullets ([`BULLET_CODES`]).
pub fn is_bullet_code(code: char) -> bool {
    BULLET_CODES.contains(&code)
}

/// Ami Pro's page-number field (`<:P…>`; `docs/research/PAGE_LAYOUT_NOTES.md`,
/// "Page numbering"): a numbering style, the page it starts on, the number
/// that page shows, and text shown before the number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageNumberField {
    /// The style character as Ami Pro stores it: `1` figures, `I` and `i`
    /// roman numerals, `A` and `a` letters; any other character shows
    /// figures.
    pub style: char,
    /// Start on page: the first page that shows the field, or 0 for every
    /// page.
    pub start_page: u32,
    /// Start with number: the number the start page shows (page 1 without a
    /// start page), or 0 for the page's own number.
    pub start_number: u32,
    pub leading_text: String,
}

impl PageNumberField {
    /// The styles Ami Pro's Page Numbering offers, in its order.
    pub const STYLES: [char; 5] = ['1', 'I', 'i', 'A', 'a'];

    /// A field showing each page's number in figures, with no leading text.
    pub fn plain() -> Self {
        Self {
            style: '1',
            start_page: 0,
            start_number: 0,
            leading_text: String::new(),
        }
    }

    /// The number page `page` (1-based) shows, or `None` before the start
    /// page (`1308:0026`; Ami Pro computes in 16 bits, and a start page of
    /// 9999 shows on every page).
    pub fn number_on(&self, page: u32) -> Option<u32> {
        let page16 = page as u16;
        if self.start_page != 9999 && self.start_page > page {
            return None;
        }
        let number = match (self.start_number as u16, self.start_page as u16) {
            (0, _) => page16,
            (start, 0) => page16.wrapping_add(start).wrapping_sub(1),
            (start, first) => page16.wrapping_sub(first.wrapping_sub(start)),
        };
        Some(u32::from(number))
    }
}

impl InlineObject {
    /// The flow this object owns, if any. Deleting the object deletes it.
    pub fn owned_flow(&self) -> Option<FlowId> {
        match self {
            Self::FloatingHeaderFooter { flow, .. }
            | Self::Note { flow, .. }
            | Self::Footnote { flow } => Some(*flow),
            Self::Break { .. }
            | Self::TableFootnote { .. }
            | Self::FrameAnchor(_)
            | Self::PageNumber(_)
            | Self::Bullet(_)
            | Self::Date(_)
            | Self::DocInfo(_)
            | Self::Seq(_) => None,
        }
    }

    /// The frame this object anchors, if any. Deleting the object deletes it.
    pub fn anchored_frame(&self) -> Option<FrameId> {
        match self {
            Self::FrameAnchor(frame) => Some(*frame),
            _ => None,
        }
    }

    /// Whether the object may occur outside the main flow.
    pub fn allowed_in_any_flow(&self) -> bool {
        matches!(
            self,
            Self::PageNumber(_)
                | Self::Bullet(_)
                | Self::Date(_)
                | Self::DocInfo(_)
                | Self::Seq(_)
                | Self::Note { .. }
                | Self::TableFootnote { .. }
        )
    }
}

/// An inline control from a legacy file that the model does not map yet. It
/// is kept, zero-width, for a future writer: it moves like a run boundary,
/// is deleted with the text around it, and is never a caret stop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyMark {
    pub byte: usize,
    pub raw: String,
}

/// Direct character formatting over a paragraph's style. Each property is
/// optional: `None` is the style's (`docs/research/SAM_FORMAT_NOTES.md`,
/// "Character attributes" and "Font changes").
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharacterFormat {
    pub font_family: Option<String>,
    pub font_size_points: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub word_underline: Option<bool>,
    pub double_underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub superscript: Option<bool>,
    pub subscript: Option<bool>,
    pub caps: Option<Caps>,
    /// A Windows `COLORREF`, `0x00BBGGRR`.
    pub colour: Option<Colour>,
    /// Protected text and no hyphenation (`<+*>`, `<+.>`): off ends them,
    /// a style's protection included.
    pub protected: Option<bool>,
    pub no_hyphenation: Option<bool>,
    /// The overstrike character (`<:O+C>`), or none: no style has one.
    pub overstrike: Option<char>,
}

impl CharacterFormat {
    /// No direct formatting: everything the style's.
    pub const PLAIN: Self = Self {
        font_family: None,
        font_size_points: None,
        bold: None,
        italic: None,
        underline: None,
        word_underline: None,
        double_underline: None,
        strikethrough: None,
        superscript: None,
        subscript: None,
        caps: None,
        colour: None,
        protected: None,
        no_hyphenation: None,
        overstrike: None,
    };

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The font of text with this format in `style`, as printer line
    /// heights name it.
    pub fn face_key(&self, style: &ParagraphStyle) -> FaceKey {
        FaceKey::new(
            self.font_family.as_deref().unwrap_or(&style.font_family),
            self.font_size_points.unwrap_or(style.font_size_points),
            self.bold.unwrap_or(style.bold),
            self.italic.unwrap_or(style.italic),
        )
    }

    /// Whether the face, size, and colour are usable: a named face, a
    /// positive size, and a `COLORREF`.
    pub fn has_valid_font(&self) -> bool {
        !self
            .font_family
            .as_ref()
            .is_some_and(|family| family.is_empty())
            && !self
                .font_size_points
                .is_some_and(|size| !size.is_finite() || size <= 0.0)
            && !self.colour.is_some_and(|colour| colour > WHITE)
    }

    /// The attributes of text with this format in `style`.
    pub fn attributes(&self, style: &ParagraphStyle) -> CharacterAttributes {
        let base = style.attributes;
        CharacterAttributes {
            underline: self.underline.unwrap_or(base.underline),
            word_underline: self.word_underline.unwrap_or(base.word_underline),
            double_underline: self.double_underline.unwrap_or(base.double_underline),
            strikethrough: self.strikethrough.unwrap_or(base.strikethrough),
            superscript: self.superscript.unwrap_or(base.superscript),
            subscript: self.subscript.unwrap_or(base.subscript),
            caps: self.caps.or(base.caps).filter(|caps| *caps != Caps::Normal),
            colour: self.colour.unwrap_or(base.colour),
            protected: self.protected.unwrap_or(base.protected),
            no_hyphenation: self.no_hyphenation.unwrap_or(base.no_hyphenation),
            overstrike: self.overstrike.or(base.overstrike),
        }
    }

    /// Sets every property `other` sets.
    pub fn overlay(&mut self, other: &Self) {
        fn take<T: Clone>(target: &mut Option<T>, value: &Option<T>) {
            if value.is_some() {
                target.clone_from(value);
            }
        }
        take(&mut self.font_family, &other.font_family);
        take(&mut self.font_size_points, &other.font_size_points);
        take(&mut self.bold, &other.bold);
        take(&mut self.italic, &other.italic);
        take(&mut self.underline, &other.underline);
        take(&mut self.word_underline, &other.word_underline);
        take(&mut self.double_underline, &other.double_underline);
        take(&mut self.strikethrough, &other.strikethrough);
        take(&mut self.superscript, &other.superscript);
        take(&mut self.subscript, &other.subscript);
        take(&mut self.caps, &other.caps);
        take(&mut self.colour, &other.colour);
        take(&mut self.protected, &other.protected);
        take(&mut self.no_hyphenation, &other.no_hyphenation);
        take(&mut self.overstrike, &other.overstrike);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterRun {
    pub start: usize,
    pub end: usize,
    pub format: CharacterFormat,
}

impl Paragraph {
    /// Byte offsets of the U+FFFC characters in `text`, one per object.
    pub fn object_offsets(&self) -> impl Iterator<Item = usize> + '_ {
        self.text
            .match_indices(OBJECT_REPLACEMENT_CHARACTER)
            .map(|(offset, _)| offset)
    }

    /// The object whose U+FFFC starts at `byte`, if any.
    pub fn object_at(&self, byte: usize) -> Option<&InlineObject> {
        self.object_offsets()
            .position(|offset| offset == byte)
            .and_then(|index| self.objects.get(index))
    }

    /// The direct format of the character at `byte` (empty where no run
    /// covers it).
    pub fn format_at(&self, byte: usize) -> &CharacterFormat {
        static PLAIN: CharacterFormat = CharacterFormat::PLAIN;
        self.runs
            .iter()
            .find(|run| run.start <= byte && byte < run.end)
            .map_or(&PLAIN, |run| &run.format)
    }

    /// The fonts Ami Pro measures this paragraph's lines by: those of its
    /// characters (hard returns, soft hyphens, and objects show none), or,
    /// in a paragraph without text, its end format's (decompile `10d0:359b`,
    /// `10d0:0592`).
    pub fn fonts(&self, style: &ParagraphStyle) -> BTreeSet<FaceKey> {
        if self.text.is_empty() {
            return BTreeSet::from([self.end_format.face_key(style)]);
        }
        self.text
            .char_indices()
            .filter(|(_, character)| {
                !matches!(
                    *character,
                    '\u{2028}' | '\u{AD}' | OBJECT_REPLACEMENT_CHARACTER
                )
            })
            .map(|(byte, _)| self.format_at(byte).face_key(style))
            .collect()
    }

    /// Applies direct formatting to one UTF-8-safe byte range.
    pub fn apply_character_format(
        &mut self,
        start: usize,
        end: usize,
        format: &CharacterFormat,
    ) -> Result<(), DocumentError> {
        if format.is_empty() {
            return Err(DocumentError::Invalid(
                "character format change is empty".into(),
            ));
        }
        self.modify_character_format(start, end, |segment| segment.overlay(format))
    }

    /// Changes the direct formatting of one UTF-8-safe byte range: `change`
    /// sees the format of each stretch of the range in turn (empty where no
    /// run covers it) and may set or clear any property. Runs are split at
    /// the range's ends, emptied runs are dropped, and equal neighbours are
    /// merged.
    pub fn modify_character_format(
        &mut self,
        start: usize,
        end: usize,
        change: impl Fn(&mut CharacterFormat),
    ) -> Result<(), DocumentError> {
        if start >= end
            || end > self.text.len()
            || !self.text.is_char_boundary(start)
            || !self.text.is_char_boundary(end)
        {
            return Err(DocumentError::Invalid(
                "character format range has invalid UTF-8 boundaries".into(),
            ));
        }
        let mut boundaries = BTreeSet::from([0, start, end, self.text.len()]);
        for run in &self.runs {
            boundaries.insert(run.start);
            boundaries.insert(run.end);
        }
        let boundaries = boundaries.into_iter().collect::<Vec<_>>();
        let mut runs: Vec<CharacterRun> = Vec::new();
        for window in boundaries.windows(2) {
            let segment_start = window[0];
            let segment_end = window[1];
            if segment_start == segment_end {
                continue;
            }
            let mut segment_format = self
                .runs
                .iter()
                .find(|run| run.start <= segment_start && run.end >= segment_end)
                .map_or_else(CharacterFormat::default, |run| run.format.clone());
            if segment_start >= start && segment_end <= end {
                change(&mut segment_format);
            }
            if !segment_format.has_valid_font() {
                return Err(DocumentError::Invalid(
                    "character format change has invalid font properties".into(),
                ));
            }
            if segment_format.is_empty() {
                continue;
            }
            if let Some(previous) = runs
                .last_mut()
                .filter(|run| run.end == segment_start && run.format == segment_format)
            {
                previous.end = segment_end;
            } else {
                runs.push(CharacterRun {
                    start: segment_start,
                    end: segment_end,
                    format: segment_format,
                });
            }
        }
        self.runs = runs;
        Ok(())
    }
}

/// A text stream: an ordered list of paragraphs. Paragraphs are shared
/// between document roots, so an edit copies only the paragraphs it touches.
#[derive(Clone, Debug, Default)]
pub struct Flow {
    pub paragraphs: Vec<Arc<Paragraph>>,
}

impl Flow {
    pub fn len(&self) -> usize {
        self.paragraphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paragraphs.is_empty()
    }

    pub fn paragraph(&self, index: usize) -> Option<&Paragraph> {
        self.paragraphs.get(index).map(|paragraph| &**paragraph)
    }

    /// Returns a paragraph for mutation, copying it first if another root
    /// shares it.
    pub fn paragraph_mut(&mut self, index: usize) -> Option<&mut Paragraph> {
        self.paragraphs.get_mut(index).map(Arc::make_mut)
    }

    /// The flow's text with one line per paragraph.
    pub fn plain_text(&self) -> String {
        self.paragraphs
            .iter()
            .map(|paragraph| paragraph.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl PartialEq for Flow {
    fn eq(&self, other: &Self) -> bool {
        self.paragraphs.len() == other.paragraphs.len()
            && self
                .paragraphs
                .iter()
                .zip(&other.paragraphs)
                .all(|(left, right)| shared_eq(left, right))
    }
}

/// A document root.
///
/// Cloning is cheap: every part is shared through `Arc` until it changes.
/// Every flow, layout, and paragraph has an ID from one per-document counter
/// that only grows, so IDs are never reused within a document.
#[derive(Clone, Debug)]
pub struct Document {
    pub title: String,
    styles: Arc<BTreeMap<StyleId, ParagraphStyle>>,
    flows: Arc<BTreeMap<FlowId, Arc<Flow>>>,
    main_flow: FlowId,
    /// The first layout is the document's layout.
    layouts: Arc<Vec<Arc<PageLayout>>>,
    frames: Arc<BTreeMap<FrameId, Arc<Frame>>>,
    /// Every page frame (fixed and anchored), back to front.
    frame_stack: Arc<Vec<FrameId>>,
    objects: Arc<BTreeMap<ObjectId, EmbeddedObject>>,
    /// Document-level legacy sections, in order.
    legacy: Arc<LegacyData>,
    /// The printer's line heights by font (empty for new documents).
    line_heights: Arc<LineHeights>,
    /// Each face's Windows pitch and family byte, by lower-case family name
    /// (empty for new documents).
    face_kinds: Arc<BTreeMap<String, u8>>,
    /// The style sheet in use, if any.
    style_sheet: Option<StyleSheetLink>,
    /// The hyphenation zone (`[docopts]` value 1): a word is split only
    /// where the line has at least this many average characters left.
    hyphenation_zone: u16,
    /// The layout the first page uses when it is not the document's own
    /// (Ami Pro's `[l1]`: a layout inserted at the start of the document).
    start_layout: Option<LayoutId>,
    /// Ami Pro's Doc Info.
    info: Arc<DocumentInfo>,
    /// Ami Pro's Footnote Options (`[fopts]`).
    footnote_options: Arc<FootnoteOptions>,
    /// Ami Pro's Line Numbering (`[lnopts]`).
    line_numbering: Arc<LineNumbering>,
    /// The document's language (`[lang]`; decision SP6).
    language: Language,
    /// The grammar and style set the last grammar check used
    /// (`[GramStyle]`; empty: none chosen yet).
    grammar_set: String,
    /// The TOC levels and the TOC, Index options (`[toc]`, `[master]`).
    contents: Arc<contents::ContentsSettings>,
    /// The merge's data file and label values (`[recfile]`) and conditions.
    merge: Arc<merge::MergeSettings>,
    /// While View › Show Power Fields shows the codes in the text: what
    /// each field set aside, by the id its end mark carries
    /// (`power::SetAside`). Never saved: a save shows the results first.
    codes: Option<Arc<BTreeMap<u32, SetAside>>>,
    next_id: u64,
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.main_flow == other.main_flow
            && self.next_id == other.next_id
            && shared_eq(&self.styles, &other.styles)
            && (Arc::ptr_eq(&self.flows, &other.flows)
                || (self.flows.len() == other.flows.len()
                    && self.flows.iter().zip(other.flows.iter()).all(
                        |((left_id, left), (right_id, right))| {
                            left_id == right_id && shared_eq(left, right)
                        },
                    )))
            && (Arc::ptr_eq(&self.layouts, &other.layouts)
                || (self.layouts.len() == other.layouts.len()
                    && self
                        .layouts
                        .iter()
                        .zip(other.layouts.iter())
                        .all(|(left, right)| shared_eq(left, right))))
            && (Arc::ptr_eq(&self.frames, &other.frames)
                || (self.frames.len() == other.frames.len()
                    && self.frames.iter().zip(other.frames.iter()).all(
                        |((left_id, left), (right_id, right))| {
                            left_id == right_id && shared_eq(left, right)
                        },
                    )))
            && shared_eq(&self.frame_stack, &other.frame_stack)
            && shared_eq(&self.objects, &other.objects)
            && shared_eq(&self.legacy, &other.legacy)
            && shared_eq(&self.line_heights, &other.line_heights)
            && shared_eq(&self.face_kinds, &other.face_kinds)
            && self.style_sheet == other.style_sheet
            && self.hyphenation_zone == other.hyphenation_zone
            && shared_eq(&self.info, &other.info)
            && shared_eq(&self.footnote_options, &other.footnote_options)
            && shared_eq(&self.line_numbering, &other.line_numbering)
            && self.language == other.language
            && self.grammar_set == other.grammar_set
            && shared_eq(&self.contents, &other.contents)
            && shared_eq(&self.merge, &other.merge)
            && self.codes == other.codes
            && self.start_layout == other.start_layout
    }
}

/// Compares shared values, short-cutting when both sides share one value.
fn shared_eq<T: PartialEq + ?Sized>(left: &Arc<T>, right: &Arc<T>) -> bool {
    Arc::ptr_eq(left, right) || **left == **right
}

/// What [`Document::layout_changes`] found.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutChanges {
    /// Styles, page layouts, frames, the frame stack, embedded objects, line
    /// heights, or the set of flows changed: everything must be laid out
    /// again.
    pub structure: bool,
    /// A flow other than the main flow changed (header, footer, frame, or
    /// cell text).
    pub other_flows: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentError {
    Invalid(String),
    Io(String),
    Parse { line: usize, message: String },
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Io(message) => f.write_str(message),
            Self::Parse { line, message } => write!(f, "line {line}: {message}"),
        }
    }
}

impl std::error::Error for DocumentError {}

/// Points (as older native files store spacing) in twips.
pub(crate) fn points_to_twips(points: f32) -> Twips {
    if points.is_finite() {
        (points * 20.0).round().clamp(0.0, 1_000_000.0) as Twips
    } else {
        0
    }
}

/// `[fnt]` value 4's face-kind bits Ami Pro writes for a roman face
/// (Times New Roman) and a swiss face (Arial).
const ROMAN_FACE: u16 = 0xC000;
const SWISS_FACE: u16 = 0x4000;

/// The styles of a new document, in faces Ami Pro has, as its `_DEFAULT.STY`
/// pairs them: Times New Roman for text, Arial for headings (user decision,
/// 2026-10-13). The app shows them in their metric-compatible Liberation
/// substitutes.
pub fn default_styles() -> BTreeMap<StyleId, ParagraphStyle> {
    let normal = ParagraphStyle {
        id: StyleId(1),
        name: "Body Text".into(),
        parent: None,
        font_family: "Times New Roman".into(),
        font_size_points: 12.0,
        bold: false,
        italic: false,
        // As `_DEFAULT.STY`'s Body Text: no space above or below, so a new
        // document's header band holds two lines as in Ami Pro (VM HDR1;
        // user decision, 2026-10-18).
        spacing: Spacing::default(),
        indents: Indents::default(),
        breaks: Breaks::default(),
        alignment: Alignment::Left,
        attributes: CharacterAttributes::default(),
        rules: ParagraphRules::default(),
        next: None,
        tabs: Vec::new(),
        hanging: false,
        hyphenation: false,
        bullets: Bullets::default(),
        origin: StyleOrigin::Document,
        both_sides: false,
        kept: StyleKept {
            face_bits: ROMAN_FACE,
            ..StyleKept::default()
        },
    };
    let heading_one = ParagraphStyle {
        id: StyleId(2),
        name: "Heading 1".into(),
        parent: Some(normal.id),
        font_family: "Arial".into(),
        font_size_points: 20.0,
        bold: true,
        italic: false,
        spacing: Spacing::below(200),
        indents: Indents::default(),
        breaks: Breaks::default(),
        alignment: Alignment::Left,
        attributes: CharacterAttributes::default(),
        rules: ParagraphRules::default(),
        next: None,
        tabs: Vec::new(),
        hanging: false,
        hyphenation: false,
        bullets: Bullets::default(),
        origin: StyleOrigin::Document,
        both_sides: false,
        kept: StyleKept {
            face_bits: SWISS_FACE,
            ..StyleKept::default()
        },
    };
    let heading_two = ParagraphStyle {
        id: StyleId(3),
        name: "Heading 2".into(),
        parent: Some(normal.id),
        font_family: "Arial".into(),
        font_size_points: 16.0,
        bold: true,
        italic: false,
        spacing: Spacing::below(160),
        indents: Indents::default(),
        breaks: Breaks::default(),
        alignment: Alignment::Left,
        attributes: CharacterAttributes::default(),
        rules: ParagraphRules::default(),
        next: None,
        tabs: Vec::new(),
        hanging: false,
        hyphenation: false,
        bullets: Bullets::default(),
        origin: StyleOrigin::Document,
        both_sides: false,
        kept: StyleKept {
            face_bits: SWISS_FACE,
            ..StyleKept::default()
        },
    };
    BTreeMap::from([
        (normal.id, normal),
        (heading_one.id, heading_one),
        (heading_two.id, heading_two),
    ])
}

impl Default for Document {
    fn default() -> Self {
        let mut document = Self::empty("Untitled", default_styles());
        document.push_paragraph("", StyleId(1));
        document
    }
}

impl Document {
    /// A document with the given styles, the standard page layout with empty
    /// header and footer frames (as Ami Pro's new documents have, G00), and an
    /// empty main flow, which needs a paragraph before the document is valid.
    ///
    /// IDs are allocated in document order: the main flow, the layout and its
    /// header and footer frames, and then each paragraph as it is added.
    /// Importers and decoders build documents this way, so equal content gets
    /// equal IDs.
    pub fn empty(title: impl Into<String>, styles: BTreeMap<StyleId, ParagraphStyle>) -> Self {
        let mut document = Self {
            title: title.into(),
            styles: Arc::new(styles),
            flows: Arc::new(BTreeMap::new()),
            main_flow: FlowId(0),
            layouts: Arc::new(Vec::new()),
            frames: Arc::new(BTreeMap::new()),
            frame_stack: Arc::new(Vec::new()),
            objects: Arc::new(BTreeMap::new()),
            legacy: Arc::new(LegacyData::default()),
            line_heights: Arc::new(LineHeights::default()),
            face_kinds: Arc::default(),
            style_sheet: None,
            hyphenation_zone: DEFAULT_HYPHENATION_ZONE,
            start_layout: None,
            info: Arc::default(),
            footnote_options: Arc::default(),
            line_numbering: Arc::default(),
            language: Language::default(),
            grammar_set: String::new(),
            contents: Arc::default(),
            merge: Arc::default(),
            codes: None,
            next_id: 1,
        };
        let main_flow = FlowId(document.allocate_raw_id());
        Arc::make_mut(&mut document.flows).insert(main_flow, Arc::new(Flow::default()));
        document.main_flow = main_flow;
        let layout = document.new_standard_layout("Standard");
        Arc::make_mut(&mut document.layouts).push(Arc::new(layout));
        document
    }

    /// A Letter layout named `name` with new empty header and footer frames,
    /// not yet in the document.
    pub fn new_standard_layout(&mut self, name: impl Into<String>) -> PageLayout {
        let mut layout = PageLayout::letter(self.allocate_layout_id(), name);
        let header = self.create_text_frame(
            Placement::Header {
                side: PageSide::Right,
            },
            header_footer_style(true, &layout.right),
        );
        let footer = self.create_text_frame(
            Placement::Footer {
                side: PageSide::Right,
            },
            header_footer_style(false, &layout.right),
        );
        layout.header.right = Some(header);
        layout.footer.right = Some(footer);
        layout
    }

    /// Creates a text frame with an empty Body Text paragraph and adds it to
    /// the frame map only. The caller makes it reachable (frame stack, layout
    /// slot, or repeating list) in the same edit.
    pub fn create_text_frame(&mut self, placement: Placement, style: FrameStyle) -> FrameId {
        let id = FrameId(self.allocate_raw_id());
        let flow = self.create_flow(self.body_text_style());
        self.insert_frame(Frame {
            id,
            placement,
            style,
            group: None,
            content: FrameContent::Text(flow),
            legacy: LegacyData::default(),
        });
        id
    }

    /// The style new frame and header text starts in: Body Text if the
    /// document has it, otherwise its first style.
    pub fn body_text_style(&self) -> StyleId {
        self.styles
            .values()
            .find(|style| style.name.eq_ignore_ascii_case("Body Text"))
            .or_else(|| self.styles.values().next())
            .map_or(StyleId(1), |style| style.id)
    }

    /// The Table Text style, made when the first table is (user's guide:
    /// Ami Pro assigns Table Text to all table cells, generated from Body
    /// Text with no space above or below and no indent): stored in the
    /// document, with no function key.
    pub fn table_text_style(&mut self) -> StyleId {
        if let Some(style) = self
            .styles
            .values()
            .find(|style| style.name.eq_ignore_ascii_case("Table Text"))
        {
            return style.id;
        }
        let body = self.styles[&self.body_text_style()].clone();
        let id = StyleId(self.styles.keys().next_back().map_or(1, |id| id.0 + 1));
        let mut style = ParagraphStyle {
            id,
            name: "Table Text".into(),
            parent: None,
            indents: Indents::default(),
            next: None,
            origin: StyleOrigin::Document,
            ..body
        };
        style.spacing.above = 0;
        style.spacing.below = 0;
        style.kept.function_key = 0;
        self.styles_mut().insert(id, style);
        id
    }

    /// A new document with the default styles and the given main-flow
    /// paragraphs (one empty Body Text paragraph if there are none).
    pub fn with_paragraphs<I, T>(paragraphs: I) -> Self
    where
        I: IntoIterator<Item = (T, StyleId)>,
        T: Into<String>,
    {
        let mut document = Self::empty("Untitled", default_styles());
        for (text, style) in paragraphs {
            document.push_paragraph(text, style);
        }
        if document.paragraphs().is_empty() {
            document.push_paragraph("", StyleId(1));
        }
        document
    }

    /// A new document with the default styles and one Body Text paragraph per
    /// line of `text`.
    pub fn from_plain_text(text: &str) -> Self {
        Self::with_paragraphs(text.split('\n').map(|line| (line, StyleId(1))))
    }

    pub fn styles(&self) -> &BTreeMap<StyleId, ParagraphStyle> {
        &self.styles
    }

    /// Returns the styles for mutation, copying them first if shared.
    pub fn styles_mut(&mut self) -> &mut BTreeMap<StyleId, ParagraphStyle> {
        Arc::make_mut(&mut self.styles)
    }

    pub fn main_flow_id(&self) -> FlowId {
        self.main_flow
    }

    pub fn main_flow(&self) -> &Flow {
        self.flows
            .get(&self.main_flow)
            .expect("a document always has its main flow")
    }

    /// Returns the main flow for mutation, copying it first if shared.
    pub fn main_flow_mut(&mut self) -> &mut Flow {
        let id = self.main_flow;
        self.flow_mut(id)
            .expect("a document always has its main flow")
    }

    pub fn flow(&self, id: FlowId) -> Option<&Flow> {
        self.flows.get(&id).map(|flow| &**flow)
    }

    /// Returns a flow for mutation, copying it first if shared.
    pub fn flow_mut(&mut self, id: FlowId) -> Option<&mut Flow> {
        if !self.flows.contains_key(&id) {
            return None;
        }
        Arc::make_mut(&mut self.flows)
            .get_mut(&id)
            .map(Arc::make_mut)
    }

    /// Replaces a flow's paragraphs without copying the old ones first.
    pub fn replace_flow_paragraphs(
        &mut self,
        id: FlowId,
        paragraphs: Vec<Arc<Paragraph>>,
    ) -> Result<(), DocumentError> {
        let flows = Arc::make_mut(&mut self.flows);
        let flow = flows
            .get_mut(&id)
            .ok_or_else(|| DocumentError::Invalid(format!("missing flow {}", id.0)))?;
        *flow = Arc::new(Flow { paragraphs });
        Ok(())
    }

    /// Creates a flow with one empty paragraph in `style`, for an owner that
    /// the caller adds in the same edit.
    pub fn create_flow(&mut self, style: StyleId) -> FlowId {
        let id = FlowId(self.allocate_raw_id());
        let paragraph = Arc::new(self.new_paragraph("", style));
        Arc::make_mut(&mut self.flows).insert(
            id,
            Arc::new(Flow {
                paragraphs: vec![paragraph],
            }),
        );
        id
    }

    /// Removes a flow other than the main flow, for an edit that removes its
    /// owner.
    pub fn remove_flow(&mut self, id: FlowId) -> Option<Arc<Flow>> {
        if id == self.main_flow || !self.flows.contains_key(&id) {
            return None;
        }
        Arc::make_mut(&mut self.flows).remove(&id)
    }

    pub fn frames(&self) -> impl Iterator<Item = &Frame> {
        self.frames.values().map(|frame| &**frame)
    }

    pub fn frame(&self, id: FrameId) -> Option<&Frame> {
        self.frames.get(&id).map(|frame| &**frame)
    }

    /// Returns a frame for mutation, copying it first if shared.
    pub fn frame_mut(&mut self, id: FrameId) -> Option<&mut Frame> {
        if !self.frames.contains_key(&id) {
            return None;
        }
        Arc::make_mut(&mut self.frames)
            .get_mut(&id)
            .map(Arc::make_mut)
    }

    /// Adds or replaces a frame in the frame map. Reachability (the frame
    /// stack, a layout slot, or a repeating list) is the caller's.
    pub fn insert_frame(&mut self, frame: Frame) {
        Arc::make_mut(&mut self.frames).insert(frame.id, Arc::new(frame));
    }

    /// The table frame and the cell that hold frame `id`, if a cell does.
    pub fn cell_holding(&self, id: FrameId) -> Option<(FrameId, usize)> {
        if !self.frame(id)?.placement.is_in_cell() {
            return None;
        }
        self.frames.values().find_map(|frame| match &frame.content {
            FrameContent::Table(table) => table
                .cells
                .iter()
                .position(|cell| cell.frame == Some(id))
                .map(|cell| (frame.id, cell)),
            _ => None,
        })
    }

    /// A hash of what sets the height of row `row` of `table`: whether the
    /// table's rows are Automatic, and each cell over the row with its place,
    /// span, the widths of its columns and the gutters inside it, and its
    /// paragraphs ([`Paragraph::content_fingerprint`]). Ami Pro's stored
    /// height is the row's while this is what it was when read
    /// ([`Table::read_rows`]).
    pub fn table_row_fingerprint(&self, table: &Table, row: usize) -> u64 {
        let count = table.columns.len().max(1);
        let mut styles: BTreeMap<StyleId, u64> = BTreeMap::new();
        let mut hash = fnv1a(FNV_OFFSET, &[u8::from(table.row_options.automatic)]);
        for (index, cell) in table.cells.iter().enumerate() {
            let span = cell.span;
            let (top, column) = (index / count, index % count);
            if span.is_covered() || !(top..top + usize::from(span.rows.max(1))).contains(&row) {
                continue;
            }
            let columns =
                column..(column + usize::from(span.columns.max(1))).min(table.columns.len());
            let rows = top..(top + usize::from(span.rows.max(1))).min(table.rows.len());
            let shape = format!(
                "{index}|{span:?}|{:?}|{:?}|{:?}|",
                &table.columns[columns.clone()],
                columns
                    .clone()
                    .map(|k| table.column_gutter(k))
                    .collect::<Vec<_>>(),
                rows.clone()
                    .map(|k| table.row_gutter(k))
                    .collect::<Vec<_>>(),
            );
            hash = fnv1a(hash, shape.as_bytes());
            let CellContent::Text(flow) = cell.content;
            let paragraphs: &[Arc<Paragraph>] =
                self.flow(flow).map_or(&[], |flow| &flow.paragraphs);
            for paragraph in paragraphs {
                let style = *styles.entry(paragraph.style).or_insert_with(|| {
                    self.styles
                        .get(&paragraph.style)
                        .map_or(0, ParagraphStyle::fingerprint)
                });
                hash = fnv1a(
                    hash,
                    &paragraph.content_fingerprint_with(style).to_le_bytes(),
                );
            }
        }
        hash
    }

    /// Removes a frame from the frame map, the frame stack, any layout, and
    /// the cell that holds it, but not what it owns.
    pub fn remove_frame(&mut self, id: FrameId) -> Option<Arc<Frame>> {
        if let Some((table, cell)) = self.cell_holding(id)
            && let Some(table) = Arc::make_mut(&mut self.frames).get_mut(&table)
            && let FrameContent::Table(content) = &mut Arc::make_mut(table).content
        {
            content.cells[cell].frame = None;
        }
        let frame = Arc::make_mut(&mut self.frames).remove(&id)?;
        if self.frame_stack.contains(&id) {
            Arc::make_mut(&mut self.frame_stack).retain(|candidate| *candidate != id);
        }
        if self
            .layouts
            .iter()
            .any(|layout| layout.frames().any(|f| f == id))
        {
            for layout in Arc::make_mut(&mut self.layouts) {
                if layout.frames().any(|f| f == id) {
                    let layout = Arc::make_mut(layout);
                    for slot in [
                        &mut layout.header.right,
                        &mut layout.header.left,
                        &mut layout.footer.right,
                        &mut layout.footer.left,
                    ] {
                        if *slot == Some(id) {
                            *slot = None;
                        }
                    }
                    layout.repeating.retain(|candidate| *candidate != id);
                }
            }
        }
        Some(frame)
    }

    /// Page frames (fixed and anchored), back to front within each kind.
    /// Anchored frames are drawn below Where placed ones whatever their
    /// places here (VM fixtures FC7G, FC8), so only the order within a kind
    /// shows.
    pub fn frame_stack(&self) -> &[FrameId] {
        &self.frame_stack
    }

    pub fn frame_stack_mut(&mut self) -> &mut Vec<FrameId> {
        Arc::make_mut(&mut self.frame_stack)
    }

    pub fn objects(&self) -> &BTreeMap<ObjectId, EmbeddedObject> {
        &self.objects
    }

    /// Adds an embedded payload and returns its new ID.
    pub fn add_object(&mut self, bytes: Arc<[u8]>) -> ObjectId {
        let id = ObjectId(self.allocate_raw_id());
        Arc::make_mut(&mut self.objects).insert(id, EmbeddedObject { bytes });
        id
    }

    /// Adds an embedded payload under a known ID (for decoders).
    pub fn insert_object(&mut self, id: ObjectId, object: EmbeddedObject) {
        Arc::make_mut(&mut self.objects).insert(id, object);
    }

    /// Document-level legacy sections, in order.
    pub fn legacy(&self) -> &LegacyData {
        &self.legacy
    }

    pub fn legacy_mut(&mut self) -> &mut LegacyData {
        Arc::make_mut(&mut self.legacy)
    }

    /// The printer's single line heights by font.
    pub fn line_heights(&self) -> &LineHeights {
        &self.line_heights
    }

    pub fn set_line_heights(&mut self, heights: LineHeights) {
        self.line_heights = Arc::new(heights);
    }

    /// Each face's Windows pitch and family byte (`LOGFONT`'s
    /// `lfPitchAndFamily`: the low bits fixed or variable pitch, the high
    /// nibble roman, swiss, modern, script, or decorative), by lower-case
    /// family name, as Ami Pro stored it: what picks a substitute for a face
    /// that is not installed (`docs/research/SAM_FORMAT_NOTES.md`).
    pub fn face_kinds(&self) -> &BTreeMap<String, u8> {
        &self.face_kinds
    }

    pub fn set_face_kinds(&mut self, kinds: BTreeMap<String, u8>) {
        self.face_kinds = Arc::new(kinds);
    }

    /// The style sheet the document uses (`[sty]`), if any.
    pub fn style_sheet(&self) -> Option<&StyleSheetLink> {
        self.style_sheet.as_ref()
    }

    pub fn set_style_sheet(&mut self, link: Option<StyleSheetLink>) {
        self.style_sheet = link;
    }

    /// The hyphenation zone in average character widths (Ami Pro's default
    /// 5).
    pub fn hyphenation_zone(&self) -> u16 {
        self.hyphenation_zone
    }

    /// While power fields show their codes: what each field set aside.
    pub fn shown_codes(&self) -> Option<&BTreeMap<u32, SetAside>> {
        self.codes.as_deref()
    }

    pub fn set_shown_codes(&mut self, codes: Option<BTreeMap<u32, SetAside>>) {
        self.codes = codes.map(Arc::new);
    }

    /// Whether any text holds a date or a Doc Info entry, whose shown text
    /// depends on a [`FieldContext`].
    pub fn has_fields(&self) -> bool {
        self.flows.values().any(|flow| {
            flow.paragraphs.iter().any(|paragraph| {
                paragraph.objects.iter().any(|object| {
                    matches!(object, InlineObject::Date(_) | InlineObject::DocInfo(_))
                })
            })
        })
    }

    /// Ami Pro's Doc Info: description, keywords, fields, dates, and
    /// statistics.
    pub fn info(&self) -> &DocumentInfo {
        &self.info
    }

    pub fn info_mut(&mut self) -> &mut DocumentInfo {
        Arc::make_mut(&mut self.info)
    }

    pub fn footnote_options(&self) -> &FootnoteOptions {
        &self.footnote_options
    }

    pub fn set_footnote_options(&mut self, options: FootnoteOptions) {
        self.footnote_options = Arc::new(options);
    }

    /// Ami Pro's Line Numbering (`[lnopts]`).
    pub fn line_numbering(&self) -> &LineNumbering {
        &self.line_numbering
    }

    pub fn set_line_numbering(&mut self, numbering: LineNumbering) {
        self.line_numbering = Arc::new(numbering);
    }

    /// The document's language (`[lang]`).
    pub fn language(&self) -> &Language {
        &self.language
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// The grammar and style set's name (`[GramStyle]`), empty when none
    /// was chosen.
    pub fn grammar_set(&self) -> &str {
        &self.grammar_set
    }

    pub fn set_grammar_set(&mut self, name: impl Into<String>) {
        self.grammar_set = name.into();
    }

    /// The TOC levels and the TOC, Index options.
    pub fn contents(&self) -> &contents::ContentsSettings {
        &self.contents
    }

    pub fn set_contents(&mut self, settings: contents::ContentsSettings) {
        self.contents = Arc::new(settings);
    }

    /// The merge's data file, label values, and conditions.
    pub fn merge(&self) -> &merge::MergeSettings {
        &self.merge
    }

    pub fn set_merge(&mut self, settings: merge::MergeSettings) {
        self.merge = Arc::new(settings);
    }

    pub fn set_hyphenation_zone(&mut self, zone: u16) {
        self.hyphenation_zone = zone;
    }

    pub fn allocate_frame_id(&mut self) -> FrameId {
        FrameId(self.allocate_raw_id())
    }

    pub fn allocate_layout_id(&mut self) -> LayoutId {
        LayoutId(self.allocate_raw_id())
    }

    pub fn allocate_group_id(&mut self) -> GroupId {
        GroupId(self.allocate_raw_id())
    }

    pub fn allocate_flow_id(&mut self) -> FlowId {
        FlowId(self.allocate_raw_id())
    }

    /// Adds a flow under a known ID (for decoders and for copies made by
    /// commands). The caller adds its owner in the same edit.
    pub fn insert_flow(&mut self, id: FlowId, flow: Flow) {
        Arc::make_mut(&mut self.flows).insert(id, Arc::new(flow));
    }

    pub fn flows(&self) -> impl Iterator<Item = (FlowId, &Flow)> {
        self.flows.iter().map(|(id, flow)| (*id, &**flow))
    }

    /// The main flow's paragraphs.
    pub fn paragraphs(&self) -> &[Arc<Paragraph>] {
        &self.main_flow().paragraphs
    }

    /// Returns a main-flow paragraph for mutation, copying it first if shared.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds, like slice indexing.
    pub fn paragraph_mut(&mut self, index: usize) -> &mut Paragraph {
        self.main_flow_mut()
            .paragraph_mut(index)
            .expect("paragraph index out of bounds")
    }

    /// A new paragraph with a fresh ID, not yet in any flow.
    pub fn new_paragraph(&mut self, text: impl Into<String>, style: StyleId) -> Paragraph {
        Paragraph {
            format: ParagraphFormat::default(),
            id: self.allocate_paragraph_id(),
            text: text.into(),
            style,
            runs: Vec::new(),
            objects: Vec::new(),
            marks: Vec::new(),
            end_format: CharacterFormat::default(),
            cache: None,
        }
    }

    /// Appends a paragraph with a fresh ID to the main flow.
    pub fn push_paragraph(&mut self, text: impl Into<String>, style: StyleId) -> &mut Paragraph {
        let paragraph = Arc::new(self.new_paragraph(text, style));
        let flow = self.main_flow_mut();
        flow.paragraphs.push(paragraph);
        flow.paragraphs
            .last_mut()
            .map(Arc::make_mut)
            .expect("a paragraph was just pushed")
    }

    pub fn allocate_paragraph_id(&mut self) -> ParagraphId {
        ParagraphId(self.allocate_raw_id())
    }

    fn allocate_raw_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("the document ID counter overflowed");
        id
    }

    /// The value the next allocated ID will have.
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Makes later allocations start at `floor` or above. Undo history uses
    /// this so that IDs from undone edits are not handed out again.
    pub fn raise_id_counter(&mut self, floor: u64) {
        self.next_id = self.next_id.max(floor);
    }

    pub fn layout(&self, id: LayoutId) -> Option<&PageLayout> {
        self.layouts
            .iter()
            .find(|layout| layout.id == id)
            .map(|layout| &**layout)
    }

    /// Returns a layout for mutation, copying it first if shared.
    pub fn layout_mut(&mut self, id: LayoutId) -> Option<&mut PageLayout> {
        let index = self.layouts.iter().position(|layout| layout.id == id)?;
        Arc::make_mut(&mut self.layouts)
            .get_mut(index)
            .map(Arc::make_mut)
    }

    /// The ordered layouts for insertion and removal. The first stays the
    /// document's layout.
    pub fn layouts_mut(&mut self) -> &mut Vec<Arc<PageLayout>> {
        Arc::make_mut(&mut self.layouts)
    }

    pub fn layouts(&self) -> &[Arc<PageLayout>] {
        &self.layouts
    }

    /// The document's layout: the first one.
    pub fn document_layout(&self) -> &PageLayout {
        self.layouts
            .first()
            .expect("a document always has its document layout")
    }

    /// The layout the first page uses: the document's own, unless a layout
    /// was inserted at the document's start (Ami Pro's `[l1]`).
    pub fn start_layout(&self) -> &PageLayout {
        self.start_layout
            .and_then(|id| self.layout(id))
            .unwrap_or_else(|| self.document_layout())
    }

    /// The layout the first page uses when it is not the document's own.
    pub fn start_layout_id(&self) -> Option<LayoutId> {
        self.start_layout
    }

    /// Sets the layout the first page uses; the document's own layout, or
    /// `None`, means no other.
    pub fn set_start_layout(&mut self, layout: Option<LayoutId>) {
        self.start_layout = layout.filter(|id| *id != self.document_layout().id);
    }

    /// Returns the document's layout for mutation, copying it first if shared.
    pub fn document_layout_mut(&mut self) -> &mut PageLayout {
        Arc::make_mut(&mut self.layouts)
            .first_mut()
            .map(Arc::make_mut)
            .expect("a document always has its document layout")
    }

    /// The document layout's page geometry in points, for the current UI.
    pub fn page_setup(&self) -> PageSetup {
        self.document_layout().page_setup()
    }

    /// Sets the document layout's page geometry from points, rounding to
    /// twips.
    pub fn set_page_setup(&mut self, page: &PageSetup) -> Result<(), DocumentError> {
        let twips = |points: f32| -> Result<Twips, DocumentError> {
            let value = (points * TWIPS_PER_POINT).round();
            if value.is_finite() && value >= Twips::MIN as f32 && value <= Twips::MAX as f32 {
                Ok(value as Twips)
            } else {
                Err(DocumentError::Invalid("invalid page geometry".into()))
            }
        };
        let [top, right, bottom, left] = page.margin_points;
        let width = twips(page.width_points)?;
        let height = twips(page.height_points)?;
        let margins = Margins {
            top: twips(top)?,
            right: twips(right)?,
            bottom: twips(bottom)?,
            left: twips(left)?,
        };
        let geometry = &mut self.document_layout_mut().right;
        geometry.width = width;
        geometry.height = height;
        geometry.layout.margins = margins;
        Ok(())
    }

    /// What differs from `earlier` for layout (D5). Shared parts compare by
    /// pointer first, so a document derived from `earlier` by commands or
    /// restored from history compares in constant time per part. The main
    /// flow's paragraphs are left to the caller.
    pub fn layout_changes(&self, earlier: &Document) -> LayoutChanges {
        let same_flows = Arc::ptr_eq(&self.flows, &earlier.flows);
        LayoutChanges {
            structure: self.main_flow != earlier.main_flow
                || !shared_eq(&self.styles, &earlier.styles)
                || !shared_eq(&self.layouts, &earlier.layouts)
                || !shared_eq(&self.frames, &earlier.frames)
                || !shared_eq(&self.frame_stack, &earlier.frame_stack)
                || !shared_eq(&self.objects, &earlier.objects)
                || !shared_eq(&self.line_heights, &earlier.line_heights)
                || !shared_eq(&self.face_kinds, &earlier.face_kinds)
                || self.start_layout != earlier.start_layout
                || !shared_eq(&self.footnote_options, &earlier.footnote_options)
                || !(same_flows || self.flows.keys().eq(earlier.flows.keys())),
            other_flows: !same_flows
                && self
                    .flows
                    .iter()
                    .zip(earlier.flows.iter())
                    .any(|((id, flow), (_, before))| {
                        *id != self.main_flow && !shared_eq(flow, before)
                    }),
        }
    }

    /// Compares content without IDs: title, styles, layouts (without their
    /// frame IDs), the number of frames and payloads, and the main flow's
    /// paragraphs in order. Native v1 and v2 do not store IDs, so a document
    /// reloaded from them is equal to the saved one only this way.
    pub fn eq_ignoring_ids(&self, other: &Self) -> bool {
        let without_ids = |layout: &PageLayout| {
            let mut layout = layout.clone();
            layout.id = LayoutId(0);
            layout.header = SideFrames {
                right: layout.header.right.map(|_| FrameId(0)),
                left: layout.header.left.map(|_| FrameId(0)),
            };
            layout.footer = SideFrames {
                right: layout.footer.right.map(|_| FrameId(0)),
                left: layout.footer.left.map(|_| FrameId(0)),
            };
            layout.repeating.iter_mut().for_each(|id| *id = FrameId(0));
            layout
        };
        let same_layouts = self.layouts.len() == other.layouts.len()
            && self
                .layouts
                .iter()
                .zip(other.layouts.iter())
                .all(|(a, b)| without_ids(a) == without_ids(b));
        self.title == other.title
            && *self.styles == *other.styles
            && same_layouts
            && self.frames.len() == other.frames.len()
            && self.frame_stack.len() == other.frame_stack.len()
            && self.objects.len() == other.objects.len()
            && self.legacy == other.legacy
            && self.footnote_options == other.footnote_options
            && self.line_numbering == other.line_numbering
            && self.language == other.language
            && self.grammar_set == other.grammar_set
            && self.contents == other.contents
            && self.merge == other.merge
            && self.flows.len() == other.flows.len()
            && same_flow_content(self, self.main_flow(), other, other.main_flow())
    }

    /// Returns the editable body as one line per main-flow paragraph.
    pub fn plain_text(&self) -> String {
        self.main_flow().plain_text()
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        if let Some(start) = self.start_layout
            && (self.layout(start).is_none() || start == self.document_layout().id)
        {
            return Err(DocumentError::Invalid(
                "the start layout is an inserted layout of the document".into(),
            ));
        }
        if self
            .style_sheet
            .as_ref()
            .is_some_and(|link| link.name.trim().is_empty())
        {
            return Err(DocumentError::Invalid(
                "a style sheet link names its file".into(),
            ));
        }
        if let Some(family) = self
            .face_kinds
            .keys()
            .find(|family| family.is_empty() || family.to_lowercase() != **family)
        {
            return Err(DocumentError::Invalid(format!(
                "a face kind's family is lower case and named: {family:?}"
            )));
        }
        if let Some((key, _)) = self
            .line_heights
            .iter()
            .find(|(key, height)| key.size <= 0 || *height <= 0 || key.family.is_empty())
        {
            return Err(DocumentError::Invalid(format!(
                "invalid line height for {} at {} twentieths",
                key.family, key.size
            )));
        }
        for style in self.styles.values() {
            if style.font_size_points <= 0.0
                || !style.font_size_points.is_finite()
                || style.attributes.colour > WHITE
            {
                return Err(DocumentError::Invalid(format!(
                    "invalid size or colour for style {}",
                    style.name
                )));
            }
            if style
                .next
                .is_some_and(|next| !self.styles.contains_key(&next))
            {
                return Err(DocumentError::Invalid(format!(
                    "style {} names a missing next style",
                    style.name
                )));
            }
            let rules = style.rules;
            if rules.colour > WHITE
                || rules.other_length < 0
                || rules.above.space < 0
                || rules.below.space < 0
            {
                return Err(DocumentError::Invalid(format!(
                    "style {} has invalid paragraph rules",
                    style.name
                )));
            }
            let spacing = style.spacing;
            if spacing.above < 0 || spacing.below < 0 || !spacing.line.is_valid() {
                return Err(DocumentError::Invalid(format!(
                    "style {} has invalid spacing",
                    style.name
                )));
            }
            let indents = style.indents;
            if [indents.all, indents.first, indents.rest, indents.right]
                .iter()
                .any(|indent| *indent < 0)
            {
                return Err(DocumentError::Invalid(format!(
                    "style {} has a negative indent",
                    style.name
                )));
            }
            if let Some(parent) = style.parent {
                if parent == style.id || !self.styles.contains_key(&parent) {
                    return Err(DocumentError::Invalid(format!(
                        "invalid parent for style {}",
                        style.name
                    )));
                }
            }
        }

        let invalid = |message: String| Err(DocumentError::Invalid(message));
        let mut ids = BTreeSet::new();
        let mut claim = |raw: u64, kind: &str| -> Result<(), DocumentError> {
            if raw == 0 || raw >= self.next_id {
                return invalid(format!(
                    "{kind} ID {raw} is outside the document's ID counter"
                ));
            }
            if !ids.insert(raw) {
                return invalid(format!("{kind} ID {raw} is used more than once"));
            }
            Ok(())
        };

        // Layouts, and the frames each one reaches.
        if self.layouts.is_empty() {
            return invalid("a document requires a page layout".into());
        }
        let mut reached: BTreeMap<FrameId, usize> = BTreeMap::new();
        let mut reach = |frame: FrameId,
                         expected: &dyn Fn(Placement) -> bool,
                         slot: &str|
         -> Result<(), DocumentError> {
            let Some(found) = self.frames.get(&frame) else {
                return invalid(format!("{slot} names missing frame {}", frame.0));
            };
            if !expected(found.placement) {
                return invalid(format!(
                    "frame {} in {slot} has the wrong placement",
                    frame.0
                ));
            }
            *reached.entry(frame).or_default() += 1;
            Ok(())
        };
        for layout in self.layouts.iter() {
            claim(layout.id.0, "layout")?;
            layout.validate()?;
            let slot = |placement: Placement, kind: fn(PageSide) -> Placement, side| {
                placement == kind(side)
            };
            for (frame, side, header) in [
                (layout.header.right, PageSide::Right, true),
                (layout.header.left, PageSide::Left, true),
                (layout.footer.right, PageSide::Right, false),
                (layout.footer.left, PageSide::Left, false),
            ] {
                if let Some(frame) = frame {
                    let kind: fn(PageSide) -> Placement = if header {
                        |side| Placement::Header { side }
                    } else {
                        |side| Placement::Footer { side }
                    };
                    reach(
                        frame,
                        &|placement| slot(placement, kind, side),
                        "a header or footer slot",
                    )?;
                }
            }
            for frame in &layout.repeating {
                reach(
                    *frame,
                    &|placement| matches!(placement, Placement::Repeating { .. }),
                    "a repeating-frame list",
                )?;
            }
        }
        for frame in self.frame_stack.iter() {
            reach(*frame, &Placement::is_page_frame, "the frame stack")?;
        }

        // Frames, their groups, and their content.
        for id in self.objects.keys() {
            claim(id.0, "object")?;
        }
        let mut groups: BTreeMap<GroupId, Vec<FrameId>> = BTreeMap::new();
        let mut owners: BTreeMap<FlowId, usize> = BTreeMap::new();
        for (id, frame) in self.frames.iter() {
            claim(id.0, "frame")?;
            if frame.id != *id {
                return invalid(format!("frame {} is stored under ID {}", frame.id.0, id.0));
            }
            if reached.get(id) != Some(&1) {
                return invalid(format!("frame {} must be reachable exactly once", id.0));
            }
            frame
                .placement
                .validate()
                .or_else(|message| invalid(format!("frame {}: {message}", id.0)))?;
            frame.style.validate()?;
            match (frame.placement, frame.style.wrap) {
                (Placement::Inline { .. }, wrap) if wrap != Wrap::Around => {
                    return invalid(format!(
                        "frame {}: a Flow with text frame wraps around",
                        id.0
                    ));
                }
                (Placement::BelowParagraph { .. }, Wrap::Around) => {
                    return invalid(format!(
                        "frame {}: a With para above frame cannot wrap around",
                        id.0
                    ));
                }
                _ => {}
            }
            if let Some(group) = frame.group {
                groups.entry(group).or_default().push(*id);
            }
            match &frame.content {
                FrameContent::Table(table) => {
                    if table.columns.is_empty()
                        || table.rows.is_empty()
                        || table.columns.len().checked_mul(table.rows.len())
                            != Some(table.cells.len())
                    {
                        return invalid(format!(
                            "frame {}: a table needs rows times columns cells",
                            id.0
                        ));
                    }
                    table.validate_spacing().map_err(|message| {
                        DocumentError::Invalid(format!("frame {}: {message}", id.0))
                    })?;
                    table.validate_spans().map_err(|message| {
                        DocumentError::Invalid(format!("frame {}: {message}", id.0))
                    })?;
                }
                FrameContent::Picture(picture) => {
                    picture.validate().map_err(|message| {
                        DocumentError::Invalid(format!("frame {}: {message}", id.0))
                    })?;
                    for object in frame.objects() {
                        if !self.objects.contains_key(&object) {
                            return invalid(format!(
                                "frame {} refers to missing object {}",
                                id.0, object.0
                            ));
                        }
                    }
                }
                FrameContent::Text(_) | FrameContent::Legacy(_) => {}
            }
            for flow in frame.owned_flows() {
                *owners.entry(flow).or_default() += 1;
            }
        }
        // Frames in table cells: each named by exactly one cell, which joined
        // cells do not cover, and each cell's frame one of them.
        let mut held: BTreeMap<FrameId, usize> = BTreeMap::new();
        for (id, frame) in self.frames.iter() {
            let FrameContent::Table(table) = &frame.content else {
                continue;
            };
            for cell in &table.cells {
                if let Some(formula) = &cell.formula {
                    if !formula_text_is_valid(formula) {
                        return invalid(format!(
                            "frame {}: a cell's formula is one line of text",
                            id.0
                        ));
                    }
                    if cell.span.is_covered() {
                        return invalid(format!("frame {}: a covered cell holds a formula", id.0));
                    }
                }
                let Some(inner) = cell.frame else {
                    continue;
                };
                if cell.span.is_covered() {
                    return invalid(format!("frame {}: a covered cell holds a frame", id.0));
                }
                if !self
                    .frames
                    .get(&inner)
                    .is_some_and(|inner| inner.placement.is_in_cell())
                {
                    return invalid(format!(
                        "frame {}: a cell holds frame {}, which is not in a cell",
                        id.0, inner.0
                    ));
                }
                *held.entry(inner).or_default() += 1;
            }
        }
        for (id, frame) in self.frames.iter() {
            if !frame.placement.is_in_cell() {
                continue;
            }
            if held.get(id) != Some(&1) {
                return invalid(format!(
                    "frame {} is in a cell, so exactly one cell must hold it",
                    id.0
                ));
            }
            if frame.group.is_some() || matches!(frame.content, FrameContent::Table(_)) {
                return invalid(format!(
                    "frame {}: a frame in a cell is neither grouped nor a table",
                    id.0
                ));
            }
        }

        // Groups are runs in Ami Pro's list of Where placed frames; anchored
        // frames between them in the stack are in another list (FC8).
        let unanchored: Vec<FrameId> = self
            .frame_stack
            .iter()
            .copied()
            .filter(|id| {
                self.frames
                    .get(id)
                    .is_none_or(|frame| !frame.placement.is_anchored())
            })
            .collect();
        for (group, members) in &groups {
            claim(group.0, "group")?;
            let positions: Vec<usize> = members
                .iter()
                .filter_map(|member| unanchored.iter().position(|id| id == member))
                .collect();
            let pages: BTreeSet<u32> = members
                .iter()
                .filter_map(|member| match self.frames[member].placement {
                    Placement::Fixed { page, .. } => Some(page),
                    _ => None,
                })
                .collect();
            let contiguous = positions.len() == members.len()
                && positions
                    .iter()
                    .max()
                    .zip(positions.iter().min())
                    .is_some_and(|(max, min)| max - min + 1 == members.len());
            if members.len() < 2 || pages.len() != 1 || !contiguous {
                return invalid(format!(
                    "group {} needs two or more Where placed frames, on one page, adjacent in z-order",
                    group.0
                ));
            }
            if members
                .iter()
                .any(|member| !matches!(self.frames[member].placement, Placement::Fixed { .. }))
            {
                return invalid(format!(
                    "group {} has a frame that is not Where placed",
                    group.0
                ));
            }
        }

        // Flows, paragraphs, and inline objects.
        if !self.flows.contains_key(&self.main_flow) {
            return invalid("the main flow is missing".into());
        }
        let mut anchors: BTreeMap<FrameId, usize> = BTreeMap::new();
        for (id, flow) in self.flows.iter() {
            claim(id.0, "flow")?;
            if flow.paragraphs.is_empty() {
                return invalid(if *id == self.main_flow {
                    "a document requires at least one paragraph".into()
                } else {
                    format!("flow {} has no paragraphs", id.0)
                });
            }
            for paragraph in &flow.paragraphs {
                claim(paragraph.id.0, "paragraph")?;
                self.validate_paragraph(paragraph)?;
                for object in &paragraph.objects {
                    if *id != self.main_flow && !object.allowed_in_any_flow() {
                        return invalid(
                            "frame anchors, breaks, and floating headers occur only in the main flow"
                                .into(),
                        );
                    }
                    self.validate_object(object)?;
                    if let Some(owned) = object.owned_flow() {
                        *owners.entry(owned).or_default() += 1;
                    }
                    if let Some(frame) = object.anchored_frame() {
                        *anchors.entry(frame).or_default() += 1;
                    }
                }
            }
        }
        for (frame_id, frame) in self.frames.iter() {
            let count = anchors.get(frame_id).copied().unwrap_or(0);
            if frame.placement.is_anchored() != (count == 1) || count > 1 {
                return invalid(format!(
                    "frame {} needs exactly one anchor if and only if it is anchored",
                    frame_id.0
                ));
            }
        }
        if let Some((frame, _)) = anchors
            .iter()
            .find(|(frame, _)| !self.frames.contains_key(frame))
        {
            return invalid(format!("an anchor names missing frame {}", frame.0));
        }
        for (flow, count) in &owners {
            if *count > 1 {
                return invalid(format!("flow {} has more than one owner", flow.0));
            }
            if *flow == self.main_flow {
                return invalid("the main flow cannot have an owner".into());
            }
            if !self.flows.contains_key(flow) {
                return invalid(format!("an owner names missing flow {}", flow.0));
            }
        }
        if let Some(orphan) = self
            .flows
            .keys()
            .find(|id| **id != self.main_flow && !owners.contains_key(id))
        {
            return invalid(format!("flow {} has no owner", orphan.0));
        }
        Ok(())
    }

    fn validate_object(&self, object: &InlineObject) -> Result<(), DocumentError> {
        match object {
            InlineObject::Break {
                kind: BreakKind::Column,
                centre,
                layout,
                ..
            } if *centre || layout.is_some() => Err(DocumentError::Invalid(
                "a column break neither centres nor switches layouts".into(),
            )),
            InlineObject::Break {
                layout: Some(layout),
                ..
            } if self.layout(*layout).is_none() => Err(DocumentError::Invalid(format!(
                "a page break names missing layout {}",
                layout.0
            ))),
            InlineObject::FrameAnchor(frame) if !self.frame_stack.contains(frame) => {
                Err(DocumentError::Invalid(format!(
                    "an anchor names frame {} outside the frame stack",
                    frame.0
                )))
            }
            InlineObject::Bullet(code) if !is_bullet_code(*code) => Err(DocumentError::Invalid(
                format!("U+{:04X} is not one of Ami Pro's bullets", *code as u32),
            )),
            _ => Ok(()),
        }
    }

    fn validate_paragraph(&self, paragraph: &Paragraph) -> Result<(), DocumentError> {
        if !self.styles.contains_key(&paragraph.style) {
            return Err(DocumentError::Invalid(format!(
                "missing style {}",
                paragraph.style.0
            )));
        }
        if paragraph.text.contains(['\r', '\n']) {
            return Err(DocumentError::Invalid(
                "paragraph text contains an embedded line break".into(),
            ));
        }
        if paragraph
            .format
            .line_spacing
            .is_some_and(|line| !line.is_valid())
        {
            return Err(DocumentError::Invalid(
                "paragraph has an invalid line spacing".into(),
            ));
        }
        let placeholders = paragraph.object_offsets().count();
        if placeholders != paragraph.objects.len() {
            return Err(DocumentError::Invalid(format!(
                "paragraph has {placeholders} object characters but {} objects",
                paragraph.objects.len()
            )));
        }
        let mut previous_mark = 0;
        for mark in &paragraph.marks {
            if mark.byte < previous_mark
                || mark.byte > paragraph.text.len()
                || !paragraph.text.is_char_boundary(mark.byte)
                || mark.raw.is_empty()
            {
                return Err(DocumentError::Invalid(
                    "legacy mark is unsorted, empty, or off a UTF-8 boundary".into(),
                ));
            }
            previous_mark = mark.byte;
        }
        let mut previous_end = 0;
        for run in &paragraph.runs {
            if run.start >= run.end
                || run.end > paragraph.text.len()
                || !paragraph.text.is_char_boundary(run.start)
                || !paragraph.text.is_char_boundary(run.end)
            {
                return Err(DocumentError::Invalid(
                    "character run has invalid UTF-8 boundaries".into(),
                ));
            }
            if run.start < previous_end {
                return Err(DocumentError::Invalid(
                    "character runs overlap or are unsorted".into(),
                ));
            }
            if run.format.is_empty() {
                return Err(DocumentError::Invalid(
                    "character run has no formatting".into(),
                ));
            }
            if !run.format.has_valid_font() {
                return Err(DocumentError::Invalid(
                    "character run has invalid font properties".into(),
                ));
            }
            previous_end = run.end;
        }
        if !paragraph.end_format.has_valid_font() {
            return Err(DocumentError::Invalid(
                "paragraph end format has invalid font properties".into(),
            ));
        }
        Ok(())
    }

    /// Encodes the document as native v3. Payloads that no picture refers to
    /// are left out.
    pub fn encode_native(&self) -> Result<String, DocumentError> {
        self.validate()?;
        Ok(native::encode(self))
    }

    /// Removes embedded payloads that no picture refers to, as saving does.
    pub fn drop_unreferenced_objects(&mut self) {
        let referenced: BTreeSet<ObjectId> = self
            .frames
            .values()
            .flat_map(|frame| frame.objects())
            .collect();
        if self.objects.keys().any(|id| !referenced.contains(id)) {
            Arc::make_mut(&mut self.objects).retain(|id, _| referenced.contains(id));
        }
    }

    /// Decodes native v3, or v1 and v2. The older versions store one
    /// paragraph flow and the page in points; their IDs are assigned on load
    /// in document order.
    pub fn decode_native(input: &str) -> Result<Self, DocumentError> {
        const MAX_NATIVE_BYTES: usize = 64 * 1024 * 1024;
        if input.len() > MAX_NATIVE_BYTES {
            return Err(DocumentError::Invalid(
                "native document exceeds 64 MiB".into(),
            ));
        }
        if input.lines().next() == Some(NATIVE_MAGIC) {
            return native::decode(input);
        }
        let mut lines = input.lines();
        let signature = lines.next();
        if !matches!(signature, Some(NATIVE_MAGIC_V2 | NATIVE_MAGIC_V1)) {
            return Err(DocumentError::Parse {
                line: 1,
                message: "invalid native document signature".into(),
            });
        }
        let mut title = None;
        let mut page = None;
        let mut styles = BTreeMap::new();
        let mut paragraphs: Vec<(StyleId, String, Vec<CharacterRun>)> = Vec::new();
        for (index, line) in lines.enumerate() {
            let number = index + 2;
            let fields: Vec<&str> = line.split('\t').collect();
            let error = |message: &str| DocumentError::Parse {
                line: number,
                message: message.into(),
            };
            match fields.first().copied() {
                Some("title") if fields.len() == 2 => {
                    title = Some(unescape(fields[1]).map_err(|m| error(&m))?)
                }
                Some("page") if fields.len() == 7 => {
                    let values = fields[1..]
                        .iter()
                        .map(|v| v.parse::<f32>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| error("invalid page number"))?;
                    page = Some(PageSetup {
                        width_points: values[0],
                        height_points: values[1],
                        margin_points: [values[2], values[3], values[4], values[5]],
                    });
                }
                Some("style") if fields.len() == 9 => {
                    let id = StyleId(fields[1].parse().map_err(|_| error("invalid style id"))?);
                    let parent = match fields[3]
                        .parse::<u64>()
                        .map_err(|_| error("invalid parent id"))?
                    {
                        0 => None,
                        id => Some(StyleId(id)),
                    };
                    let style = ParagraphStyle {
                        id,
                        name: unescape(fields[2]).map_err(|m| error(&m))?,
                        parent,
                        font_family: unescape(fields[4]).map_err(|m| error(&m))?,
                        font_size_points: fields[5]
                            .parse()
                            .map_err(|_| error("invalid font size"))?,
                        bold: parse_bool(fields[6]).ok_or_else(|| error("invalid bold value"))?,
                        italic: parse_bool(fields[7])
                            .ok_or_else(|| error("invalid italic value"))?,
                        spacing: Spacing::below(points_to_twips(
                            fields[8].parse().map_err(|_| error("invalid spacing"))?,
                        )),
                        indents: Indents::default(),
                        breaks: Breaks::default(),
                        alignment: Alignment::Left,
                        attributes: CharacterAttributes::default(),
                        rules: ParagraphRules::default(),
                        next: None,
                        tabs: Vec::new(),
                        hanging: false,
                        hyphenation: false,
                        bullets: Bullets::default(),
                        origin: StyleOrigin::Document,
                        both_sides: false,
                        kept: StyleKept::default(),
                    };
                    if styles.insert(id, style).is_some() {
                        return Err(error("duplicate style id"));
                    }
                }
                Some("paragraph") if fields.len() == 3 => paragraphs.push((
                    StyleId(
                        fields[1]
                            .parse()
                            .map_err(|_| error("invalid paragraph style"))?,
                    ),
                    unescape(fields[2]).map_err(|m| error(&m))?,
                    Vec::new(),
                )),
                Some("run") if signature == Some(NATIVE_MAGIC_V2) && fields.len() == 10 => {
                    let paragraph_index = fields[1]
                        .parse::<usize>()
                        .map_err(|_| error("invalid run paragraph index"))?;
                    let start = fields[2]
                        .parse::<usize>()
                        .map_err(|_| error("invalid run start"))?;
                    let end = fields[3]
                        .parse::<usize>()
                        .map_err(|_| error("invalid run end"))?;
                    let has_family =
                        parse_bool(fields[4]).ok_or_else(|| error("invalid run font presence"))?;
                    let family = unescape(fields[5]).map_err(|m| error(&m))?;
                    let format = CharacterFormat {
                        font_family: has_family.then_some(family),
                        font_size_points: parse_optional_float(fields[6]).map_err(error)?,
                        bold: parse_optional_bool(fields[7])
                            .ok_or_else(|| error("invalid run bold value"))?,
                        italic: parse_optional_bool(fields[8])
                            .ok_or_else(|| error("invalid run italic value"))?,
                        underline: parse_optional_bool(fields[9])
                            .ok_or_else(|| error("invalid run underline value"))?,
                        ..CharacterFormat::default()
                    };
                    paragraphs
                        .get_mut(paragraph_index)
                        .ok_or_else(|| error("run references a missing paragraph"))?
                        .2
                        .push(CharacterRun { start, end, format });
                }
                _ => return Err(error("unknown or malformed record")),
            }
        }
        let mut document = Document::empty(title.unwrap_or_else(|| "Untitled".into()), styles);
        if let Some(page) = page {
            document.set_page_setup(&page)?;
        }
        for (style, text, runs) in paragraphs {
            document.push_paragraph(text, style).runs = runs;
        }
        document.validate()?;
        Ok(document)
    }

    pub fn save_native(&self, path: impl AsRef<Path>) -> Result<(), DocumentError> {
        let path = path.as_ref();
        let temporary = path.with_extension("amipro.tmp");
        fs::write(&temporary, self.encode_native()?)
            .map_err(|e| DocumentError::Io(e.to_string()))?;
        fs::rename(&temporary, path).map_err(|e| DocumentError::Io(e.to_string()))
    }

    pub fn load_native(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        let text = fs::read_to_string(path).map_err(|e| DocumentError::Io(e.to_string()))?;
        Self::decode_native(&text)
    }
}

/// The style of a layout's fixed header or footer frame: no lines, the
/// default Header/Footer panel margins (0.55 in toward the page edge, 0.05 in
/// toward the text, 1 in at the sides), and centre and right tabs at the
/// page's centre and right margin (G00, G18).
fn header_footer_style(header: bool, page: &PageGeometry) -> FrameStyle {
    let (top, bottom) = if header { (792, 72) } else { (72, 792) };
    let margins = Margins {
        top,
        right: 1_440,
        bottom,
        left: 1_440,
    };
    let text_width = page.width - margins.left - margins.right;
    let mut interior = LayoutBox::single_column(margins);
    interior.tabs = vec![vec![
        TabStop {
            kind: TabKind::Centre,
            position: page.width / 2 - margins.left,
            leader: TabLeader::None,
        },
        TabStop {
            kind: TabKind::Right,
            position: text_width,
            leader: TabLeader::None,
        },
    ]];
    FrameStyle {
        border: Border {
            sides: Sides::NONE,
            ..FrameStyle::default().border
        },
        interior,
        ..FrameStyle::default()
    }
}

/// Compares two flows' content without IDs, following owned flows.
fn same_flow_content(
    left_document: &Document,
    left: &Flow,
    right_document: &Document,
    right: &Flow,
) -> bool {
    left.len() == right.len()
        && left.paragraphs.iter().zip(&right.paragraphs).all(|(a, b)| {
            a.text == b.text
                && a.style == b.style
                && a.runs == b.runs
                && a.marks == b.marks
                && a.objects.len() == b.objects.len()
                && a.objects.iter().zip(&b.objects).all(|pair| match pair {
                    (
                        InlineObject::Break {
                            kind,
                            centre,
                            layout,
                            ..
                        },
                        InlineObject::Break {
                            kind: other_kind,
                            centre: other_centre,
                            layout: other_layout,
                            ..
                        },
                    ) => {
                        kind == other_kind
                            && centre == other_centre
                            && layout.is_some() == other_layout.is_some()
                    }
                    (
                        InlineObject::FloatingHeaderFooter { kind, pages, flow },
                        InlineObject::FloatingHeaderFooter {
                            kind: other_kind,
                            pages: other_pages,
                            flow: other_flow,
                        },
                    ) => {
                        kind == other_kind
                            && pages == other_pages
                            && match (left_document.flow(*flow), right_document.flow(*other_flow)) {
                                (Some(left), Some(right)) => {
                                    same_flow_content(left_document, left, right_document, right)
                                }
                                _ => false,
                            }
                    }
                    (
                        InlineObject::Note { note, flow },
                        InlineObject::Note {
                            note: other_note,
                            flow: other_flow,
                        },
                    ) => {
                        note == other_note
                            && match (left_document.flow(*flow), right_document.flow(*other_flow)) {
                                (Some(left), Some(right)) => {
                                    same_flow_content(left_document, left, right_document, right)
                                }
                                _ => false,
                            }
                    }
                    (
                        InlineObject::Footnote { flow },
                        InlineObject::Footnote { flow: other_flow },
                    ) => match (left_document.flow(*flow), right_document.flow(*other_flow)) {
                        (Some(left), Some(right)) => {
                            same_flow_content(left_document, left, right_document, right)
                        }
                        _ => false,
                    },
                    (left @ InlineObject::TableFootnote { .. }, right) => left == right,
                    _ => false,
                })
        })
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn parse_optional_bool(value: &str) -> Option<Option<bool>> {
    match value {
        "-" => Some(None),
        "0" => Some(Some(false)),
        "1" => Some(Some(true)),
        _ => None,
    }
}

fn parse_optional_float(value: &str) -> Result<Option<f32>, &'static str> {
    if value == "-" {
        Ok(None)
    } else {
        value.parse().map(Some).map_err(|_| "invalid run font size")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_numbering_reads_the_dialogs_flags() {
        // The values Ami Pro wrote for LN1–LN8, LN17, LN18.
        let of = |flags: u8| LineNumbering {
            flags,
            ..LineNumbering::default()
        };
        assert_eq!(LineNumbering::default().flags, 2);
        assert!(!of(2).is_on());
        assert_eq!(of(3).mode(), LineNumberingMode::AllLines);
        assert_eq!(of(130).mode(), LineNumberingMode::TextLines);
        assert_eq!(of(132).every(), LineNumberingEvery::OtherLine);
        assert_eq!(of(136).every(), LineNumberingEvery::FifthLine);
        assert_eq!(of(160).every(), LineNumberingEvery::Lines);
        assert!(of(146).reset_each_page() && of(19).reset_each_page());
        assert!(!of(24).is_on() && of(24).reset_each_page());
        assert_eq!(of(0).every(), LineNumberingEvery::Lines);
        use LineNumberingEvery as E;
        use LineNumberingMode as M;
        for (mode, every, reset, flags) in [
            (M::None, E::Line, false, 2),
            (M::AllLines, E::Line, false, 3),
            (M::TextLines, E::Line, false, 130),
            (M::TextLines, E::OtherLine, false, 132),
            (M::TextLines, E::FifthLine, false, 136),
            (M::TextLines, E::Lines, false, 160),
            (M::TextLines, E::Line, true, 146),
            (M::AllLines, E::Line, true, 19),
            (M::None, E::FifthLine, true, 24),
            (M::TextLines, E::Lines, true, 176),
        ] {
            assert_eq!(LineNumbering::flags_for(mode, every, reset), flags);
        }
        let shown = |flags: u8, interval: u16| {
            let numbering = LineNumbering {
                flags,
                interval,
                ..LineNumbering::default()
            };
            (1..=15).filter(|&k| numbering.shows(k)).collect::<Vec<_>>()
        };
        assert_eq!(shown(130, 1).len(), 15);
        assert_eq!(shown(132, 1), [1, 3, 5, 7, 9, 11, 13, 15]);
        assert_eq!(shown(136, 1), [5, 10, 15]);
        assert_eq!(shown(160, 7), [7, 14]);
        assert_eq!(shown(160, 0).len(), 15);
    }

    #[test]
    fn native_round_trip_preserves_styles_and_text() {
        let mut document = Document::empty("Test\tDocument", default_styles());
        document.push_paragraph("Hello\tworld \\ ok", StyleId(1));
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 0,
            end: 5,
            format: CharacterFormat {
                font_family: Some("Liberation Serif".into()),
                font_size_points: Some(14.0),
                bold: Some(true),
                italic: None,
                underline: Some(false),
                ..CharacterFormat::default()
            },
        });
        let encoded = document.encode_native().unwrap();
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
    }

    /// Notes, footnotes, table footnotes, and Footnote Options (the notes
    /// run): each note's fields, short form included, and the flows they
    /// own.
    #[test]
    fn native_round_trip_preserves_notes_and_footnotes() {
        let mut document =
            Document::with_paragraphs([("a\u{FFFC}b\u{FFFC}\u{FFFC}\u{FFFC}", StyleId(1))]);
        let note = document.create_flow(StyleId(1));
        let old_note = document.create_flow(StyleId(1));
        let footnote = document.create_flow(StyleId(1));
        document.paragraph_mut(0).objects.extend([
            InlineObject::Note {
                note: NoteInfo {
                    time: 1_793_037_315,
                    initials: Some("L,S\t".into()),
                    colour: Some(255),
                    number: Some(2),
                },
                flow: note,
            },
            InlineObject::Note {
                note: NoteInfo {
                    time: 7,
                    initials: None,
                    colour: None,
                    number: None,
                },
                flow: old_note,
            },
            InlineObject::Footnote { flow: footnote },
            InlineObject::TableFootnote { number: 1, row: 4 },
        ]);
        document.set_footnote_options(FootnoteOptions {
            flags: FootnoteOptions::ENDNOTES | FootnoteOptions::CONTINUED_ON,
            start: 3,
            length: 2_880,
            indent: 720,
            continued_on: "More\tbelow".into(),
            continued_from: String::new(),
        });
        document.set_line_numbering(LineNumbering {
            flags: LineNumbering::TEXT_LINES | LineNumbering::EVERY_N,
            style: "Title\t2".into(),
            interval: 7,
        });
        document.validate().unwrap();
        let encoded = document.encode_native().unwrap();
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        // A footnote stands only in the main text.
        let mut misplaced = document.clone();
        let flow = misplaced.flow_mut(note).unwrap();
        flow.paragraphs[0] = Arc::new(Paragraph {
            text: "\u{FFFC}".into(),
            objects: vec![InlineObject::Footnote { flow: footnote }],
            ..(*flow.paragraphs[0]).clone()
        });
        assert!(misplaced.validate().is_err());
    }

    /// Character attributes beyond the `run` record's (2026-10-12): each
    /// one an `sformat` record after its style or an `rformat` record after
    /// its run, including attributes switched off over a style.
    #[test]
    fn native_round_trip_preserves_character_attributes() {
        let mut styles = default_styles();
        styles.get_mut(&StyleId(2)).unwrap().attributes = CharacterAttributes {
            underline: true,
            word_underline: true,
            double_underline: true,
            strikethrough: true,
            superscript: true,
            subscript: true,
            caps: Some(Caps::Initial),
            colour: 0x0000_00FF,
            protected: true,
            no_hyphenation: false,
            overstrike: None,
        };
        styles.get_mut(&StyleId(3)).unwrap().attributes.caps = Some(Caps::Upper);
        let mut document = Document::empty("Attributes", styles);
        document.push_paragraph("Hello world, again", StyleId(1));
        let runs = &mut document.paragraph_mut(0).runs;
        runs.push(CharacterRun {
            start: 0,
            end: 5,
            format: CharacterFormat {
                word_underline: Some(true),
                double_underline: Some(false),
                strikethrough: Some(true),
                superscript: Some(false),
                subscript: Some(true),
                caps: Some(Caps::Small),
                colour: Some(0x00FF_8000),
                ..CharacterFormat::default()
            },
        });
        runs.push(CharacterRun {
            start: 6,
            end: 11,
            format: CharacterFormat {
                bold: Some(true),
                caps: Some(Caps::Lower),
                colour: Some(BLACK),
                ..CharacterFormat::default()
            },
        });
        // PT1 and O1: protected text, no hyphenation, and overstrike.
        runs.push(CharacterRun {
            start: 13,
            end: 18,
            format: CharacterFormat {
                protected: Some(true),
                no_hyphenation: Some(false),
                overstrike: Some('\t'),
                ..CharacterFormat::default()
            },
        });
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains(
            "\nstyle\t2\tHeading 1\t1\tArial\t20\t1\t0\t10\n\
             sformat\tunderline\t1\nsformat\twordunderline\t1\n\
             sformat\tdoubleunderline\t1\nsformat\tstrikethrough\t1\n\
             sformat\tsuperscript\t1\nsformat\tsubscript\t1\n\
             sformat\tprotected\t1\n\
             sformat\tcaps\tinitial\nsformat\tcolour\t255\n\
             skept\t0\t0\t16384\t1\t1\t1\t16\t0\t272\t1\t2\t.\t,\t$\t0\t\t0\nstyle\t3\t"
        ));
        assert!(encoded.contains(
            "\nrun\t0\t5\t0\t\t-\t-\t-\t-\n\
             rformat\twordunderline\t1\nrformat\tdoubleunderline\t0\n\
             rformat\tstrikethrough\t1\nrformat\tsuperscript\t0\n\
             rformat\tsubscript\t1\nrformat\tcaps\tsmall\n\
             rformat\tcolour\t16744448\n\
             run\t6\t11\t0\t\t-\t1\t-\t-\n\
             rformat\tcaps\tlower\nrformat\tcolour\t0\n\
             run\t13\t18\t0\t\t-\t-\t-\t-\n\
             rformat\tprotected\t1\nrformat\tnohyphenation\t0\n\
             rformat\toverstrike\t\\t\n"
        ));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
    }

    #[test]
    fn native_character_attribute_records_are_strict() {
        let mut document = Document::empty("Attributes", default_styles());
        document.push_paragraph("Hello", StyleId(1));
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 0,
            end: 5,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        });
        let encoded = document.encode_native().unwrap();
        let run = "\nrun\t0\t5\t0\t\t-\t1\t-\t-\n";
        let style = "\tTimes New Roman\t12\t0\t0\t0\n";
        assert!(encoded.contains(run) && encoded.contains(style));
        for (anchor, inserted, message) in [
            (
                run,
                "rformat\tcaps\tupper\nrformat\tcaps\tlower\n",
                "set twice",
            ),
            (run, "rformat\tunderline\t1\n", "run record"),
            (run, "rformat\tcolour\t16777216\n", "0x00BBGGRR"),
            (run, "rformat\tblink\t1\n", "unknown character attribute"),
            (run, "rformat\tcaps\tsmallest\n", "unknown caps"),
            (run, "rformat\toverstrike\txy\n", "one character"),
            (run, "rformat\toverstrike\t\n", "one character"),
            (
                run,
                "rformat\toverstrike\t/\nrformat\toverstrike\tx\n",
                "set twice",
            ),
            (run, "rformat\tprotected\tyes\n", ""),
            (style, "sformat\tstrikethrough\t2\n", ""),
            (
                style,
                "sformat\tsubscript\t1\nsformat\tsubscript\t0\n",
                "set twice",
            ),
        ] {
            let broken = encoded.replacen(anchor, &format!("{anchor}{inserted}"), 1);
            let error = Document::decode_native(&broken).unwrap_err().to_string();
            assert!(error.contains(message), "{inserted:?}: {error}");
        }
        let orphan = encoded.replacen("\nparagraph\t", "\nrformat\tcaps\tupper\nparagraph\t", 1);
        assert!(Document::decode_native(&orphan).is_err());
        let first_style = encoded.replacen("\nstyle\t", "\nsformat\tunderline\t1\nstyle\t", 1);
        assert!(Document::decode_native(&first_style).is_err());
    }

    #[test]
    fn native_round_trip_preserves_paragraph_rules() {
        let mut styles = default_styles();
        // A chapter title's grey rule above, as Ami Pro writes it
        // (`[line]` 9 0 1 12632256 0 6 12 160 100 2), with flag 16 kept.
        styles.get_mut(&StyleId(2)).unwrap().rules = ParagraphRules {
            above: RuleSide {
                on: true,
                line: 6,
                space: 160,
            },
            below: RuleSide {
                on: false,
                line: 12,
                space: 100,
            },
            length: RuleLength::Margins,
            other_length: 0,
            colour: 12_632_256,
            other_flags: 16,
            unknown: [1, 0],
            unit: 2,
        };
        styles.get_mut(&StyleId(3)).unwrap().rules = ParagraphRules {
            below: RuleSide {
                on: true,
                line: 11,
                space: 250,
            },
            length: RuleLength::Other,
            other_length: 2880,
            ..ParagraphRules::default()
        };
        let mut document = Document::empty("Rules", styles);
        document.push_paragraph("Title", StyleId(2));
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nsline\t25\t0\t1\t12632256\t0\t6\t12\t160\t100\t2\n"));
        assert!(encoded.contains("\nsline\t2\t2880\t1\t0\t1\t1\t11\t10\t250\t1\n"));
        assert_eq!(encoded.matches("sline").count(), 2, "Body Text has none");
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        assert_eq!(document.styles()[&StyleId(2)].rules.space(), (160, 0));
        assert_eq!(document.styles()[&StyleId(3)].rules.space(), (0, 250));

        let line = "sline\t25\t0\t1\t12632256\t0\t6\t12\t160\t100\t2\n";
        for (broken, message) in [
            (
                encoded.replacen(line, &format!("{line}{line}"), 1),
                "one sline",
            ),
            (encoded.replacen(line, "sline\t25\t0\t1\n", 1), ""),
            (
                encoded.replacen(line, &line.replace("12632256", "16777216"), 1),
                "COLORREF",
            ),
            (
                encoded.replacen(line, &line.replace("\t160\t", "\t-1\t"), 1),
                "rules",
            ),
            (
                encoded.replacen(line, &format!("{line}sformat\tunderline\t1\n"), 1),
                "before sline",
            ),
        ] {
            let error = Document::decode_native(&broken).unwrap_err().to_string();
            assert!(error.contains(message), "{broken:?}: {error}");
        }
        let orphan = encoded.replacen("\nstyle\t", &format!("\n{line}style\t"), 1);
        assert!(Document::decode_native(&orphan).is_err());
        assert!(rule_strokes(10).iter().sum::<Twips>() == 120 && rule_strokes(13).is_empty());
    }

    #[test]
    fn native_round_trip_preserves_tabs_leaders_and_rulers() {
        let mut styles = default_styles();
        let style = styles.get_mut(&StyleId(1)).unwrap();
        style.tabs = vec![
            TabStop::new(TabKind::Left, 3615),
            TabStop {
                leader: TabLeader::Hyphens,
                ..TabStop::new(TabKind::Decimal, 5040)
            },
        ];
        style.hanging = true;
        let mut document = Document::empty("Tabs", styles);
        document.push_paragraph("a\tb", StyleId(1));
        document.paragraph_mut(0).format.tabs = Some(vec![TabStop {
            leader: TabLeader::Line,
            ..TabStop::new(TabKind::Right, 4050)
        }]);
        document.push_paragraph("none", StyleId(1));
        document.paragraph_mut(1).format.tabs = Some(Vec::new());
        document.document_layout_mut().right.layout.tabs = vec![vec![TabStop {
            leader: TabLeader::Dots,
            ..TabStop::new(TabKind::Centre, 4680)
        }]];
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nstabs\tL:3615\tD:5040:h\nshanging\n"));
        assert!(encoded.contains("\npformat\ttabs\tR:4050:l\n"));
        assert!(encoded.contains("\npformat\ttabs\n"));
        assert!(encoded.contains("\ttabs\t0\tC:4680:d\n") || encoded.contains("tabs\t0\tC:4680:d"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        for (broken, message) in [
            (encoded.replacen("D:5040:h", "D:5040:x", 1), "tab leader"),
            (encoded.replacen("D:5040:h", "D:5040:h:d", 1), "tab leader"),
            (
                encoded.replacen("shanging\n", "shanging\nshanging\n", 1),
                "shanging",
            ),
            (encoded.replacen("shanging\n", "shanging\tyes\n", 1), ""),
            (
                encoded.replacen("stabs\tL:3615", "stabs\tQ:3615", 1),
                "tab kind",
            ),
        ] {
            let error = Document::decode_native(&broken).unwrap_err().to_string();
            assert!(error.contains(message), "{broken:?}: {error}");
        }
        // The ruler is the paragraph's: over the style it is its own tabs.
        let mut style = document.styles()[&StyleId(1)].clone();
        document.paragraphs()[0].format.apply(&mut style);
        assert!(style.breaks.own_tabs());
        assert_eq!(style.tabs.len(), 1);
    }

    #[test]
    fn native_round_trip_preserves_bullets_and_numbers() {
        let mut styles = default_styles();
        // Every code Ami Pro writes, the bitmaps at U+000A and U+000D
        // included, and plain text.
        let codes: String = ('\u{7}'..='\u{1F}')
            .filter(|code| *code != '\u{9}')
            .collect();
        styles.get_mut(&StyleId(1)).unwrap().bullets = Bullets {
            text: format!("{codes}Note:\t"),
            level: 5,
            attributes: 16 | 1,
            flags: 18,
            kept: [0, 28948, 1, 1, 0, 2],
        };
        styles.get_mut(&StyleId(2)).unwrap().hanging = true;
        styles.get_mut(&StyleId(2)).unwrap().bullets = Bullets {
            text: "\u{1A}.".into(),
            ..Bullets::default()
        };
        let mut document = Document::empty("Bullets", styles);
        document.push_paragraph("item", StyleId(1));
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nshanging\nsbullets\t\u{1A}.\t0\t0\t0\t0\t0\t1\t1\t0\t0\n"));
        assert_eq!(
            encoded.matches("\nsbullets\t").count(),
            2,
            "none for Heading 2"
        );
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        let line = "sbullets\t\u{1A}.\t0\t0\t0\t0\t0\t1\t1\t0\t0\n";
        for (broken, message) in [
            (
                encoded.replacen(line, &format!("{line}{line}"), 1),
                "sbullets",
            ),
            (
                encoded.replacen(line, &format!("{line}shanging\n"), 1),
                "before sbullets",
            ),
            (
                encoded.replacen(line, &line.replace("\t0\t0\n", "\t0\n"), 1),
                "",
            ),
            (
                encoded.replacen(line, &line.replacen("\t0\t", "\t256\t", 1), 1),
                "",
            ),
        ] {
            let error = Document::decode_native(&broken).unwrap_err().to_string();
            assert!(error.contains(message), "{broken:?}: {error}");
        }
        let orphan = encoded.replacen("\nstyle\t", &format!("\n{line}style\t"), 1);
        assert!(Document::decode_native(&orphan).is_err());
    }

    #[test]
    fn inserted_bullets_round_trip_in_native_files_and_are_strict() {
        // Every one of Ami Pro's bullets, in the main text and a header.
        let mut document = Document::default();
        let main = document.paragraph_mut(0);
        main.text = BULLET_CODES.iter().map(|_| "\u{FFFC} ").collect();
        main.objects = BULLET_CODES
            .iter()
            .map(|code| InlineObject::Bullet(*code))
            .collect();
        let header = document.document_layout().header.right.unwrap();
        let FrameContent::Text(flow) = document.frame(header).unwrap().content else {
            unreachable!();
        };
        let paragraph = document.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
        paragraph.text = "\u{FFFC}Header".into();
        paragraph.objects.push(InlineObject::Bullet('\u{A}'));
        document.validate().unwrap();
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\ninline\tbullet\t16\n"));
        assert!(encoded.contains("\ninline\tbullet\t10\n"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        for broken in ["65", "9", "x", "16\t1"] {
            let changed = encoded.replacen(
                "\ninline\tbullet\t16\n",
                &format!("\ninline\tbullet\t{broken}\n"),
                1,
            );
            assert!(Document::decode_native(&changed).is_err(), "{broken}");
        }
        // Validation refuses a code that is not a bullet.
        let mut wrong = document.clone();
        wrong.paragraph_mut(0).objects[0] = InlineObject::Bullet('A');
        assert!(wrong.validate().is_err());
    }

    #[test]
    fn dates_and_doc_info_entries_round_trip_in_native_files() {
        // Each date source, a time with its stored byte, and Doc Info
        // entries (one with a code Ami Pro shows nothing for), in the main
        // text and a header.
        let mut document = Document::default();
        let objects = vec![
            InlineObject::Date(DateField {
                source: DateSource::Today,
                format: b'b',
            }),
            InlineObject::Date(DateField {
                source: DateSource::LastSaved,
                format: b'd',
            }),
            InlineObject::Date(DateField {
                source: DateSource::Created,
                format: b'C',
            }),
            InlineObject::Date(DateField {
                source: DateSource::Time,
                format: 1,
            }),
            InlineObject::DocInfo(DocInfoField {
                code: 0x17,
                format: b'b',
            }),
            InlineObject::DocInfo(DocInfoField {
                code: 0x13,
                format: b'a',
            }),
        ];
        let main = document.paragraph_mut(0);
        main.text = objects.iter().map(|_| "\u{FFFC} ").collect();
        main.objects = objects;
        let header = document.document_layout().header.right.unwrap();
        let FrameContent::Text(flow) = document.frame(header).unwrap().content else {
            unreachable!();
        };
        let paragraph = document.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
        paragraph.text = "Printed \u{FFFC}".into();
        paragraph.objects.push(InlineObject::Date(DateField {
            source: DateSource::Today,
            format: b'a',
        }));
        document.validate().unwrap();
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\ninline\tdate\ttoday\t98\n"));
        assert!(encoded.contains("\ninline\tdate\ttime\t1\n"));
        assert!(encoded.contains("\ninline\tdocinfo\t23\t98\n"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        for broken in [
            "\ninline\tdate\tsoon\t98\n",
            "\ninline\tdate\ttoday\t256\n",
            "\ninline\tdate\ttoday\n",
            "\ninline\tdocinfo\t70000\t98\n",
        ] {
            let changed = encoded.replacen("\ninline\tdate\ttoday\t98\n", broken, 1);
            assert!(Document::decode_native(&changed).is_err(), "{broken}");
        }
    }

    #[test]
    fn doc_info_round_trips_in_native_files() {
        let mut document = Document::default();
        assert_eq!(*document.info(), DocumentInfo::default());
        assert!(!document.encode_native().unwrap().contains("docinfo"));
        let info = document.info_mut();
        *info = DocumentInfo::new(1_792_520_631);
        info.description = "Quarterly\treport <draft>".into();
        info.keywords = "budget, 1994".into();
        info.fields[0] = "Alpha".into();
        info.field_names[0] = "Client".into();
        info.field_names[7] = "Last one".into();
        info.saved = Some(1_792_521_067);
        info.editing_minutes = 7;
        info.statistics = Statistics {
            pages: 1,
            words: 6,
            characters: 23,
            size: 4,
            size_with_links: 4,
        };
        assert_eq!(document.info().field_name(0), "Client");
        assert_eq!(document.info().field_name(1), "Field2");
        assert!(document.info().runs_frame_macros());
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\ndocinfo\t1792520631\t1792521067\t1\t7\t1\t6\t23\t4\t4\t1\t"));
        assert!(encoded.contains("\ndocfield\t1\tClient\tAlpha\n"));
        assert!(encoded.contains("\ndocfield\t8\tLast one\t\n"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        for broken in ["\ndocfield\t9\tx\ty\n", "\ndocfield\t1\tx\n"] {
            let changed = encoded.replacen("\ndocfield\t1\tClient\tAlpha\n", broken, 1);
            assert!(Document::decode_native(&changed).is_err(), "{broken}");
        }
        let unsaved = encoded.replacen("\t1792521067\t", "\t\t", 1);
        assert_eq!(
            Document::decode_native(&unsaved).unwrap().info().saved,
            None
        );
    }

    #[test]
    fn native_round_trip_preserves_style_origins_and_the_style_sheet() {
        let mut styles = default_styles();
        styles.get_mut(&StyleId(2)).unwrap().origin = StyleOrigin::Sheet;
        styles.get_mut(&StyleId(3)).unwrap().origin = StyleOrigin::Missing;
        let mut document = Document::empty("Origins", styles);
        document.push_paragraph("x", StyleId(3));
        document.set_style_sheet(Some(StyleSheetLink {
            name: "_DEFAULT.STY".into(),
            layout_from_sheet: true,
        }));
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nstylesheet\t_DEFAULT.STY\t1\nstyle\t"));
        assert!(encoded.contains("\nsorigin\tsheet\n") && encoded.contains("\nsorigin\tmissing\n"));
        assert_eq!(
            encoded.matches("sorigin").count(),
            2,
            "none for document styles"
        );
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        for broken in [
            encoded.replacen("sorigin\tsheet", "sorigin\tbook", 1),
            encoded.replacen("sorigin\tsheet\n", "sorigin\tsheet\nsorigin\tsheet\n", 1),
            encoded.replacen("stylesheet\t_DEFAULT.STY\t1", "stylesheet\t\t1", 1),
            encoded.replacen("stylesheet\t_DEFAULT.STY\t1", "stylesheet\t_DEFAULT.STY", 1),
            encoded.replacen(
                "stylesheet\t_DEFAULT.STY\t1\n",
                "stylesheet\t_DEFAULT.STY\t1\nstylesheet\tx\t0\n",
                1,
            ),
        ] {
            assert!(Document::decode_native(&broken).is_err(), "{broken}");
        }
        // What Ami Pro's style record holds beyond the model (`skept`).
        let mut kept = document.clone();
        let style = kept.styles_mut().get_mut(&StyleId(1)).unwrap();
        style.both_sides = true;
        style.kept = StyleKept {
            function_key: 2,
            face_bits: 0xC000,
            units: [3, 1, 2],
            add_in: 32,
            single_height: 273,
            number_format: NumberFormat::from_values(
                &["280", "3", "2", ".", ",", ""].map(String::from),
            )
            .unwrap(),
            next_name: Some("Outline\tI".into()),
            alignment_bits: 224,
        };
        let encoded = kept.encode_native().unwrap();
        assert!(encoded.contains(
            "\nskept\t1\t2\t49152\t3\t1\t2\t32\t273\t280\t3\t2\t.\t,\t\t1\tOutline\\tI\t224\n"
        ));
        assert_eq!(Document::decode_native(&encoded).unwrap(), kept);
        let line = "skept\t1\t2\t49152\t3\t1\t2\t32\t273\t280\t3\t2\t.\t,\t\t1\tOutline\\tI\t224\n";
        for broken in [
            encoded.replacen(line, &format!("{line}{line}"), 1),
            encoded.replacen(line, &line.replacen("\t2\t49152", "\t256\t49152", 1), 1),
            encoded.replacen(line, &line.replacen("\t224\n", "\n", 1), 1),
        ] {
            assert!(Document::decode_native(&broken).is_err(), "{broken}");
        }
        // Hyphenation: the style's switch and the document's zone.
        let mut hyphenated = document.clone();
        hyphenated
            .styles_mut()
            .get_mut(&StyleId(1))
            .unwrap()
            .hyphenation = true;
        hyphenated.set_hyphenation_zone(3);
        let encoded = hyphenated.encode_native().unwrap();
        assert!(encoded.contains("\nhyphenzone\t3\n"));
        assert!(encoded.contains("\nshyphenation\n"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), hyphenated);
        for broken in [
            encoded.replacen("shyphenation\n", "shyphenation\nshyphenation\n", 1),
            encoded.replacen("shyphenation\n", "shyphenation\tyes\n", 1),
            encoded.replacen("hyphenzone\t3", "hyphenzone\t-1", 1),
            encoded.replacen("hyphenzone\t3\n", "hyphenzone\t3\nhyphenzone\t4\n", 1),
        ] {
            assert!(Document::decode_native(&broken).is_err(), "{broken}");
        }
        // Files without these records: styles stored in the document, no
        // style sheet, no hyphenation, and Ami Pro's zone of 5.
        let plain = Document::from_plain_text("x").encode_native().unwrap();
        let decoded = Document::decode_native(&plain).unwrap();
        assert!(decoded.style_sheet().is_none());
        assert_eq!(decoded.hyphenation_zone(), DEFAULT_HYPHENATION_ZONE);
        assert!(
            decoded
                .styles()
                .values()
                .all(|style| style.origin == StyleOrigin::Document)
        );
    }

    #[test]
    fn paragraph_caches_round_trip_in_native_files_and_are_strict() {
        let mut document = Document::from_plain_text("One\nTwo");
        let style = document.styles()[&document.paragraphs()[0].style].clone();
        let paragraph = document.paragraph_mut(0);
        paragraph.cache = Some(ParagraphCache {
            height: 284,
            width: 9_360,
            fingerprint: paragraph.content_fingerprint(&style),
        });
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("pcache\t284\t9360\t"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        let twice = encoded.replacen("pcache", "pcache\t1\t2\t0\npcache", 1);
        assert!(Document::decode_native(&twice).is_err());
        let bad = encoded.replace(
            &format!(
                "{:016x}",
                document.paragraphs()[0].cache.unwrap().fingerprint
            ),
            "not-hex",
        );
        assert!(Document::decode_native(&bad).is_err());
        // The fingerprint follows the paragraph and its style.
        let print = document.paragraphs()[0].content_fingerprint(&style);
        let mut changed = style.clone();
        changed.font_size_points = 14.0;
        assert_ne!(
            document.paragraphs()[0].content_fingerprint(&changed),
            print
        );
    }

    #[test]
    fn uneven_columns_are_kept_only_when_they_differ_ru2() {
        let mut layout = LayoutBox::single_column(Margins::uniform(1_440));
        layout.columns = 2;
        // RU2: two columns, the gap dragged to 5 in: 1440–7050, 7290–10800
        // on a Letter page, so 0–5610 and 5850–9360 in its text area.
        let ru2 = vec![
            ColumnEdges {
                left: 0,
                right: 5_610,
            },
            ColumnEdges {
                left: 5_850,
                right: 9_360,
            },
        ];
        layout.set_edges(ru2.clone(), 9_360);
        assert_eq!(layout.column_edges.as_deref(), Some(&ru2[..]));
        // Ami Pro stores no gutter: a file gives back the first gap.
        assert_eq!(layout.gutter, 240);
        assert!(layout.gaps_alike(9_360), "one gap");
        assert_eq!(layout.edges(9_360), ru2);
        layout.validate("RU2").unwrap();
        // Dragged back to even: no edges, the gap as the gutter.
        layout.set_edges(
            vec![
                ColumnEdges {
                    left: 0,
                    right: 4_500,
                },
                ColumnEdges {
                    left: 4_860,
                    right: 9_360,
                },
            ],
            9_360,
        );
        assert_eq!((layout.column_edges.clone(), layout.gutter), (None, 360));
        // Widths a twip apart are even too, as the importer reads them
        // (`uneven_edges`), ending short by the twip even columns leave.
        layout.set_edges(
            vec![
                ColumnEdges {
                    left: 0,
                    right: 5_086,
                },
                ColumnEdges {
                    left: 5_086,
                    right: 10_171,
                },
            ],
            10_171,
        );
        assert_eq!((layout.column_edges.clone(), layout.gutter), (None, 0));
        // Uneven columns keep their first gap as the gutter; one column
        // has Ami Pro's 240 (`1238:0000`).
        layout.set_edges(
            vec![
                ColumnEdges {
                    left: 0,
                    right: 4_050,
                },
                ColumnEdges {
                    left: 4_800,
                    right: 9_360,
                },
            ],
            9_360,
        );
        assert_eq!(layout.gutter, 750);
        layout.columns = 1;
        layout.column_edges = None;
        layout.gutter = 720;
        layout.settle_gutter();
        assert_eq!(layout.gutter, 240);
        layout.columns = 2;
        // Cols 3 spreads them with Ami Pro's whole twips (RU12).
        layout.columns = 3;
        layout.gutter = 240;
        let even: Vec<_> = layout
            .even_edges(9_360)
            .iter()
            .map(|edges| (edges.left, edges.right))
            .collect();
        assert_eq!(even, [(0, 2_960), (3_200, 6_160), (6_400, 9_360)]);
        // Overlapping or misnumbered edges are invalid.
        layout.columns = 2;
        layout.column_edges = Some(vec![ru2[1], ru2[0]]);
        assert!(layout.validate("bad").is_err());
        layout.column_edges = Some(ru2[..1].to_vec());
        assert!(layout.validate("bad").is_err());
    }

    #[test]
    fn uneven_columns_round_trip_in_native_files_and_are_strict() {
        let mut document = Document::from_plain_text("One");
        let layout = &mut document.document_layout_mut().right.layout;
        layout.columns = 2;
        layout.tabs = vec![Vec::new(), Vec::new()];
        layout.column_edges = Some(vec![
            ColumnEdges {
                left: 0,
                right: 4_050,
            },
            ColumnEdges {
                left: 4_800,
                right: 9_360,
            },
        ]);
        document.validate().unwrap();
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nedges\t0\t4050\t4800\t9360\ntabs\t0"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        // Once, after the box and before its tabs, a pair per column.
        let twice = encoded.replacen("\nedges", "\nedges\t0\t1\t2\t3\nedges", 1);
        assert!(Document::decode_native(&twice).is_err());
        let short = encoded.replace("\t4800\t9360\n", "\t4800\n");
        assert!(Document::decode_native(&short).is_err());
        let late = encoded
            .replace("\nedges\t0\t4050\t4800\t9360", "")
            .replace("\ntabs\t1", "\ntabs\t1\nedges\t0\t4050\t4800\t9360");
        assert!(Document::decode_native(&late).is_err());
        // Past the right margin is invalid.
        let wide = encoded.replace("\t9360\n", "\t9400\n");
        assert!(Document::decode_native(&wide).is_err());
    }

    #[test]
    fn the_start_layout_is_an_inserted_layout_and_strict_in_native_files() {
        let mut document = Document::from_plain_text("One");
        let own = document.document_layout().id;
        document.set_start_layout(Some(own));
        assert_eq!(
            document.start_layout_id(),
            None,
            "the own layout is no start layout"
        );
        let mut inserted = document.new_standard_layout("Inserted");
        inserted.right.layout.margins.left = 2_880;
        let inserted_id = inserted.id;
        document.layouts_mut().push(Arc::new(inserted));
        document.set_start_layout(Some(inserted_id));
        assert_eq!(document.start_layout().name, "Inserted");
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains(&format!("startlayout\t{}\n", inserted_id.raw())));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        // A start layout follows every layout and names one of them.
        let early = encoded.replacen(&format!("startlayout\t{}\n", inserted_id.raw()), "", 1);
        let early = early.replacen(
            "layout\t",
            &format!("startlayout\t{}\nlayout\t", inserted_id.raw()),
            2,
        );
        assert!(Document::decode_native(&early).is_err());
        let unknown = encoded.replacen(
            &format!("startlayout\t{}\n", inserted_id.raw()),
            "startlayout\t999\n",
            1,
        );
        assert!(Document::decode_native(&unknown).is_err());
    }

    #[test]
    fn native_round_trip_preserves_next_styles() {
        let mut styles = default_styles();
        let heading = styles.get_mut(&StyleId(2)).unwrap();
        heading.next = Some(StyleId(1));
        heading.breaks = Breaks(Breaks::WITHIN | Breaks::NEXT_STYLE);
        let mut document = Document::empty("Next", styles);
        document.push_paragraph("Title", StyleId(2));
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nsnext\t1\n"));
        assert_eq!(encoded.matches("snext").count(), 1);
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        for (broken, message) in [
            (
                encoded.replacen("snext\t1\n", "snext\t1\nsnext\t1\n", 1),
                "one snext",
            ),
            (
                encoded.replacen("snext\t1\n", "snext\t9\n", 1),
                "missing next style",
            ),
            (
                encoded.replacen("snext\t1\n", "snext\t1\nsformat\tunderline\t1\n", 1),
                "before sline and snext",
            ),
            (encoded.replacen("snext\t1\n", "snext\t0\n", 1), "positive"),
        ] {
            let error = Document::decode_native(&broken).unwrap_err().to_string();
            assert!(error.contains(message), "{broken:?}: {error}");
        }
    }

    #[test]
    fn native_round_trip_preserves_end_formats() {
        let mut document = Document::empty("End formats", default_styles());
        document.push_paragraph("", StyleId(1));
        document.push_paragraph("Text", StyleId(1));
        document.push_paragraph("", StyleId(1));
        document.paragraph_mut(0).end_format = CharacterFormat {
            font_size_points: Some(10.0),
            ..CharacterFormat::default()
        };
        document.paragraph_mut(1).runs.push(CharacterRun {
            start: 0,
            end: 4,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        });
        document.paragraph_mut(1).end_format = CharacterFormat {
            font_family: Some("Arial".into()),
            italic: Some(true),
            caps: Some(Caps::Upper),
            colour: Some(255),
            ..CharacterFormat::default()
        };
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nendformat\t0\t\t10\t-\t-\t-\n"));
        assert!(encoded.contains(
            "\nrun\t0\t4\t0\t\t-\t1\t-\t-\n\
             endformat\t1\tArial\t-\t-\t1\t-\n\
             rformat\tcaps\tupper\nrformat\tcolour\t255\n"
        ));
        // A paragraph without an end format writes no record.
        assert_eq!(encoded.matches("endformat").count(), 2);
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);

        // Files without end formats load with none.
        let older = encoded
            .lines()
            .filter(|line| !line.starts_with("endformat") && !line.starts_with("rformat\tcaps"))
            .filter(|line| !line.starts_with("rformat\tcolour"))
            .collect::<Vec<_>>()
            .join("\n");
        let decoded = Document::decode_native(&older).unwrap();
        assert!(decoded.paragraphs().iter().all(|p| p.end_format.is_empty()));
    }

    #[test]
    fn native_end_format_records_are_strict() {
        let mut document = Document::empty("End formats", default_styles());
        document.push_paragraph("Hello", StyleId(1));
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 0,
            end: 5,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        });
        document.paragraph_mut(0).end_format.bold = Some(true);
        document.paragraph_mut(0).marks.push(LegacyMark {
            byte: 5,
            raw: "<:s>".into(),
        });
        let encoded = document.encode_native().unwrap();
        let run = "\nrun\t0\t5\t0\t\t-\t1\t-\t-\n";
        let end = "endformat\t0\t\t-\t1\t-\t-\n";
        assert!(encoded.contains(&format!("{run}{end}mark")));
        for (broken, message) in [
            // Twice, before a run, after a mark, malformed, or invalid.
            (encoded.replacen(end, &format!("{end}{end}"), 1), "order"),
            (
                encoded.replacen(&format!("{run}{end}"), &format!("\n{end}{}", &run[1..]), 1),
                "order",
            ),
            (
                encoded
                    .replacen(end, "", 1)
                    .replacen("<:s>\n", &format!("<:s>\n{end}"), 1),
                "order",
            ),
            (encoded.replacen(end, "endformat\t0\t\t-\t1\t-\n", 1), ""),
            (
                encoded.replacen(end, "endformat\t0\t\t0\t1\t-\t-\n", 1),
                "end format",
            ),
            (
                encoded.replacen(end, &format!("{end}rformat\tunderline\t1\n"), 1),
                "run record",
            ),
        ] {
            let error = Document::decode_native(&broken).unwrap_err().to_string();
            assert!(error.contains(message), "{broken:?}: {error}");
        }
        // An rformat after the end format belongs to it.
        let attribute = encoded.replacen(end, &format!("{end}rformat\tcaps\tupper\n"), 1);
        let decoded = Document::decode_native(&attribute).unwrap();
        assert_eq!(decoded.paragraphs()[0].end_format.caps, Some(Caps::Upper));
        assert_eq!(decoded.paragraphs()[0].runs[0].format.caps, None);
    }

    #[test]
    fn paragraph_fonts_are_those_of_its_characters_or_its_end() {
        let style = default_styles()[&StyleId(1)].clone();
        let mut document = Document::empty("Fonts", default_styles());
        document.push_paragraph("ab\u{2028}c\u{AD}", StyleId(1));
        let paragraph = document.paragraph_mut(0);
        paragraph.runs.push(CharacterRun {
            start: 1,
            end: 2,
            format: CharacterFormat {
                font_size_points: Some(10.0),
                bold: Some(true),
                ..CharacterFormat::default()
            },
        });
        // Underline and the end format do not change a line's fonts.
        paragraph.runs.push(CharacterRun {
            start: 5,
            end: 6,
            format: CharacterFormat {
                underline: Some(true),
                ..CharacterFormat::default()
            },
        });
        paragraph.end_format.font_size_points = Some(30.0);
        let plain = FaceKey::new(&style.font_family, style.font_size_points, false, false);
        let small_bold = FaceKey::new(&style.font_family, 10.0, true, false);
        assert_eq!(
            paragraph.fonts(&style),
            BTreeSet::from([plain.clone(), small_bold])
        );
        paragraph.text.clear();
        paragraph.runs.clear();
        assert_eq!(
            paragraph.fonts(&style),
            BTreeSet::from([FaceKey::new(&style.font_family, 30.0, false, false)])
        );
        paragraph.end_format = CharacterFormat::default();
        assert_eq!(paragraph.fonts(&style), BTreeSet::from([plain]));
        // An invalid end format is refused.
        paragraph.end_format.font_size_points = Some(0.0);
        assert!(document.validate().is_err());
    }

    #[test]
    fn character_formats_resolve_over_their_style() {
        let mut style = default_styles()[&StyleId(1)].clone();
        style.attributes = CharacterAttributes {
            strikethrough: true,
            caps: Some(Caps::Small),
            colour: 0x0000_00FF,
            ..CharacterAttributes::default()
        };
        assert_eq!(
            CharacterFormat::default().attributes(&style),
            style.attributes
        );
        let format = CharacterFormat {
            strikethrough: Some(false),
            caps: Some(Caps::Upper),
            superscript: Some(true),
            ..CharacterFormat::default()
        };
        assert_eq!(
            format.attributes(&style),
            CharacterAttributes {
                strikethrough: false,
                caps: Some(Caps::Upper),
                superscript: true,
                colour: 0x0000_00FF,
                ..CharacterAttributes::default()
            }
        );
        // Normal switches the style's caps off.
        let normal = CharacterFormat {
            caps: Some(Caps::Normal),
            ..CharacterFormat::default()
        };
        assert_eq!(normal.attributes(&style).caps, None);
        let mut document = Document::empty("Normal caps", default_styles());
        document.push_paragraph("Hello", StyleId(1));
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 0,
            end: 5,
            format: normal,
        });
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nrformat\tcaps\tnormal\n"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        let mut document = Document::empty("Colour", default_styles());
        document.push_paragraph("Hello", StyleId(1));
        let paragraph = document.paragraph_mut(0);
        assert!(
            paragraph
                .apply_character_format(
                    0,
                    5,
                    &CharacterFormat {
                        colour: Some(0x0100_0000),
                        ..CharacterFormat::default()
                    }
                )
                .is_err()
        );
        paragraph
            .apply_character_format(
                0,
                2,
                &CharacterFormat {
                    caps: Some(Caps::Initial),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();
        paragraph
            .apply_character_format(
                1,
                5,
                &CharacterFormat {
                    colour: Some(0x00FF_0000),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();
        assert_eq!(
            paragraph
                .runs
                .iter()
                .map(|run| (run.start, run.end, run.format.caps, run.format.colour))
                .collect::<Vec<_>>(),
            [
                (0, 1, Some(Caps::Initial), None),
                (1, 2, Some(Caps::Initial), Some(0x00FF_0000)),
                (2, 5, None, Some(0x00FF_0000)),
            ]
        );
        document.validate().unwrap();
    }

    #[test]
    fn version_one_native_documents_remain_readable() {
        let input = "AMIPRO-LINUX/1\ntitle\tLegacy\npage\t612\t792\t72\t72\t72\t72\nstyle\t1\tBody Text\t0\tNoto Serif\t12\t0\t0\t6\nparagraph\t1\tHello\n";
        let document = Document::decode_native(input).unwrap();
        assert_eq!(document.title, "Legacy");
        assert_eq!(document.paragraphs()[0].text, "Hello");
        assert!(document.paragraphs()[0].runs.is_empty());
    }

    #[test]
    fn version_two_load_converts_the_page_to_twips_and_assigns_ids_in_order() {
        let input = "AMIPRO-LINUX/2\ntitle\tA4\npage\t595.3\t841.9\t56.7\t72\t56.7\t72\nstyle\t1\tBody Text\t0\tNoto Serif\t12\t0\t0\t6\nparagraph\t1\tone\nparagraph\t1\ttwo\n";
        let document = Document::decode_native(input).unwrap();
        let layout = document.document_layout();
        assert_eq!((layout.right.width, layout.right.height), (11_906, 16_838));
        assert_eq!(
            layout.right.layout.margins,
            Margins {
                top: 1_134,
                right: 1_440,
                bottom: 1_134,
                left: 1_440,
            }
        );
        assert_eq!(layout.name, "Standard");
        assert_eq!(document.main_flow_id(), FlowId::from_raw(1));
        assert_eq!(layout.id, LayoutId::from_raw(2));
        assert_eq!(
            document
                .paragraphs()
                .iter()
                .map(|paragraph| paragraph.id.raw())
                .collect::<Vec<_>>(),
            [9, 10],
            "after the main flow, the layout, and its header and footer frames, flows, and paragraphs"
        );
        assert_eq!(document.next_id(), 11);
        assert_eq!(document.page_setup().width_points, 595.3);
    }

    #[test]
    fn invalid_character_run_boundaries_are_rejected() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "A🙂".into();
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 1,
            end: 2,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        });
        assert!(matches!(
            document.validate(),
            Err(DocumentError::Invalid(_))
        ));
    }

    #[test]
    fn character_formatting_splits_and_merges_runs() {
        let mut document = Document::default();
        let mut paragraph = document.new_paragraph("one two three", StyleId(1));
        paragraph.runs.push(CharacterRun {
            start: 4,
            end: 7,
            format: CharacterFormat {
                italic: Some(true),
                ..CharacterFormat::default()
            },
        });

        paragraph
            .apply_character_format(
                4,
                13,
                &CharacterFormat {
                    bold: Some(true),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();

        assert_eq!(paragraph.runs.len(), 2);
        assert_eq!((paragraph.runs[0].start, paragraph.runs[0].end), (4, 7));
        assert_eq!(paragraph.runs[0].format.italic, Some(true));
        assert_eq!(paragraph.runs[0].format.bold, Some(true));
        assert_eq!((paragraph.runs[1].start, paragraph.runs[1].end), (7, 13));
        assert_eq!(paragraph.runs[1].format.italic, None);
        assert_eq!(paragraph.runs[1].format.bold, Some(true));
    }

    #[test]
    fn unknown_records_are_rejected() {
        let input = format!("{NATIVE_MAGIC}\nunknown\tvalue\n");
        assert!(matches!(
            Document::decode_native(&input),
            Err(DocumentError::Parse { .. })
        ));
    }

    #[test]
    fn native_file_save_and_reopen_is_exact() {
        let document = Document::default();
        let path = std::env::temp_dir().join(format!(
            "amipro-document-test-{}-{}.amipro",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        document.save_native(&path).unwrap();
        assert_eq!(Document::load_native(&path).unwrap(), document);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn mutation_copies_only_the_touched_paragraph() {
        let original = Document::with_paragraphs([("one", StyleId(1)), ("two", StyleId(1))]);
        let mut edited = original.clone();
        edited.paragraph_mut(1).text.push('!');

        assert_eq!(original.paragraphs()[1].text, "two");
        assert_eq!(edited.paragraphs()[1].text, "two!");
        assert!(Arc::ptr_eq(
            &original.paragraphs()[0],
            &edited.paragraphs()[0]
        ));
        assert_ne!(original, edited);
        edited.paragraph_mut(1).text.pop();
        assert_eq!(original, edited, "equal content compares equal");
    }

    #[test]
    fn ids_are_unique_and_below_the_counter() {
        let mut document = Document::with_paragraphs([("one", StyleId(1)), ("two", StyleId(1))]);
        let first = document.paragraphs()[0].id;
        document.paragraph_mut(1).id = first;
        assert_eq!(
            document.validate(),
            Err(DocumentError::Invalid(format!(
                "paragraph ID {} is used more than once",
                first.raw()
            )))
        );

        let mut document = Document::default();
        document.paragraph_mut(0).id = ParagraphId::from_raw(99);
        assert!(matches!(
            document.validate(),
            Err(DocumentError::Invalid(message)) if message.contains("outside")
        ));

        let mut document = Document::default();
        document.document_layout_mut().id = LayoutId::from_raw(document.main_flow_id().raw());
        assert!(matches!(
            document.validate(),
            Err(DocumentError::Invalid(message)) if message.contains("more than once")
        ));
    }

    #[test]
    fn the_id_counter_only_grows() {
        let mut document = Document::default();
        let next = document.next_id();
        document.raise_id_counter(next + 10);
        assert_eq!(document.new_paragraph("", StyleId(1)).id.raw(), next + 10);
        document.raise_id_counter(1);
        assert_eq!(document.next_id(), next + 11);
    }

    #[test]
    fn page_layout_geometry_is_validated_in_twips() {
        let mut document = Document::default();
        assert_eq!(document.page_setup().margin_points, [72.0; 4]);
        document.document_layout_mut().right.layout.margins.left = 6_000;
        document.document_layout_mut().right.layout.margins.right = 6_240;
        assert!(document.validate().is_err());
        assert!(
            Document::default()
                .set_page_setup(&PageSetup {
                    width_points: f32::NAN,
                    height_points: 792.0,
                    margin_points: [72.0; 4],
                })
                .is_err()
        );
    }

    #[test]
    fn content_comparison_ignores_ids() {
        let original = Document::with_paragraphs([("one", StyleId(1)), ("two", StyleId(2))]);
        let mut renumbered = Document::default();
        renumbered.raise_id_counter(50);
        renumbered.paragraph_mut(0).text = "one".into();
        renumbered.push_paragraph("two", StyleId(2));
        assert_ne!(original, renumbered);
        assert!(original.eq_ignoring_ids(&renumbered));
        renumbered.paragraph_mut(1).style = StyleId(1);
        assert!(!original.eq_ignoring_ids(&renumbered));
    }

    fn invalid(document: &Document) -> String {
        match document.validate() {
            Err(DocumentError::Invalid(message)) => message,
            other => panic!("expected an invalid document, got {other:?}"),
        }
    }

    /// A main-flow paragraph "a\u{FFFC}b" with a page break, plus a floating
    /// footer in the second paragraph that owns a flow.
    fn document_with_objects() -> Document {
        let mut document =
            Document::with_paragraphs([("a\u{FFFC}b", StyleId(1)), ("\u{FFFC}", StyleId(1))]);
        document.paragraph_mut(0).objects.push(InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: None,
            restart: false,
        });
        let flow = document.create_flow(StyleId(1));
        document
            .paragraph_mut(1)
            .objects
            .push(InlineObject::FloatingHeaderFooter {
                kind: HeaderFooterKind::Footer,
                pages: PageSides::All,
                flow,
            });
        document.validate().unwrap();
        document
    }

    #[test]
    fn object_placeholders_and_objects_must_agree() {
        let mut document = document_with_objects();
        document
            .paragraph_mut(0)
            .text
            .push(OBJECT_REPLACEMENT_CHARACTER);
        assert!(invalid(&document).contains("2 object characters but 1 objects"));

        let mut document = document_with_objects();
        document.paragraph_mut(0).objects.clear();
        assert!(invalid(&document).contains("1 object characters but 0 objects"));
    }

    #[test]
    fn objects_stay_in_the_main_flow_and_own_their_flows_once() {
        let document = document_with_objects();
        let flow = document.paragraphs()[1].objects[0].owned_flow().unwrap();

        let mut nested = document.clone();
        let paragraph = nested.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
        paragraph.text = "\u{FFFC}".into();
        paragraph.objects.push(InlineObject::Break {
            kind: BreakKind::Column,
            centre: false,
            layout: None,
            restart: false,
        });
        assert_eq!(
            invalid(&nested),
            "frame anchors, breaks, and floating headers occur only in the main flow"
        );

        let mut twice = document.clone();
        let footer = twice.paragraphs()[1].objects[0].clone();
        twice
            .paragraph_mut(0)
            .text
            .push(OBJECT_REPLACEMENT_CHARACTER);
        twice.paragraph_mut(0).objects.push(footer);
        assert!(invalid(&twice).contains("more than one owner"));

        let mut orphan = document.clone();
        orphan.paragraph_mut(1).text.clear();
        orphan.paragraph_mut(1).objects.clear();
        assert!(invalid(&orphan).contains("has no owner"));

        let mut missing = document.clone();
        assert!(missing.remove_flow(flow).is_some());
        assert!(invalid(&missing).contains("names missing flow"));

        let mut main = document.clone();
        let main_flow = main.main_flow_id();
        main.paragraph_mut(1).objects[0] = InlineObject::FloatingHeaderFooter {
            kind: HeaderFooterKind::Header,
            pages: PageSides::Left,
            flow: main_flow,
        };
        assert!(invalid(&main).contains("the main flow cannot have an owner"));
        assert!(
            main.remove_flow(main_flow).is_none(),
            "the main flow cannot be removed"
        );
    }

    #[test]
    fn breaks_are_validated() {
        let mut column = document_with_objects();
        column.paragraph_mut(0).objects[0] = InlineObject::Break {
            kind: BreakKind::Column,
            centre: true,
            layout: None,
            restart: false,
        };
        assert!(invalid(&column).contains("column break"));

        let mut missing = document_with_objects();
        missing.paragraph_mut(0).objects[0] = InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: Some(LayoutId::from_raw(77)),
            restart: false,
        };
        assert!(invalid(&missing).contains("missing layout 77"));

        let mut named = document_with_objects();
        let layout = named.document_layout().id;
        named.paragraph_mut(0).objects[0] = InlineObject::Break {
            kind: BreakKind::Page,
            centre: true,
            layout: Some(layout),
            restart: false,
        };
        assert!(named.validate().is_ok());
    }

    #[test]
    fn legacy_marks_are_sorted_zero_width_and_on_boundaries() {
        let mut document = Document::with_paragraphs([("é text", StyleId(1))]);
        document.paragraph_mut(0).marks = vec![
            LegacyMark {
                byte: 0,
                raw: "<:#320,7020>".into(),
            },
            LegacyMark {
                byte: 2,
                raw: "<:x\nspans lines>".into(),
            },
            LegacyMark {
                byte: 2,
                raw: "<:y>".into(),
            },
        ];
        assert!(document.validate().is_ok());
        document.paragraph_mut(0).marks[1].byte = 1;
        assert!(invalid(&document).contains("legacy mark"));
        document.paragraph_mut(0).marks[1].byte = 3;
        assert!(invalid(&document).contains("legacy mark"), "unsorted");
        document.paragraph_mut(0).marks.truncate(1);
        document.paragraph_mut(0).marks[0].raw.clear();
        assert!(invalid(&document).contains("legacy mark"), "empty");
    }

    #[test]
    fn special_characters_are_ordinary_text() {
        let document = Document::with_paragraphs([(
            "hard\u{2028}return soft\u{AD}hyphen no\u{A0}break",
            StyleId(1),
        )]);
        assert!(document.validate().is_ok());
        assert!(document.paragraphs()[0].objects.is_empty());
        let reopened = Document::decode_native(&document.encode_native().unwrap()).unwrap();
        assert_eq!(reopened, document);
    }

    /// Every kind of frame: a Where placed text frame on page 1, a group of
    /// two on page 2, a With para above frame and a Flow with text table
    /// anchored in the main text, a repeating embedded picture, and a
    /// page-number field in the header.
    struct FrameFixture {
        document: Document,
        fixed: FrameId,
        grouped: [FrameId; 2],
        below: FrameId,
        table: FrameId,
        picture: FrameId,
    }

    fn frame_fixture() -> FrameFixture {
        let mut document = Document::with_paragraphs([
            ("Anchor \u{FFFC} here", StyleId(1)),
            ("\u{FFFC}Table follows", StyleId(1)),
        ]);
        let rect = Rect::new(1_440, 1_440, 4_320, 2_880);
        let fixed =
            document.create_text_frame(Placement::Fixed { page: 1, rect }, FrameStyle::default());
        document.frame_stack_mut().push(fixed);
        let group = document.allocate_group_id();
        let grouped = [0, 1].map(|index| {
            let frame = document.create_text_frame(
                Placement::Fixed {
                    page: 2,
                    rect: rect.offset(0, 2_000 * index).unwrap(),
                },
                FrameStyle::default(),
            );
            document.frame_mut(frame).unwrap().group = Some(group);
            document.frame_stack_mut().push(frame);
            frame
        });
        let below = document.create_text_frame(
            Placement::BelowParagraph {
                x_offset: 720,
                size: Size {
                    width: 2_880,
                    height: 1_440,
                },
            },
            FrameStyle {
                wrap: Wrap::NoWrapAround,
                ..FrameStyle::default()
            },
        );
        document.frame_stack_mut().push(below);
        document
            .paragraph_mut(0)
            .objects
            .push(InlineObject::FrameAnchor(below));
        let cells = (0..2)
            .map(|_| TableCell {
                content: CellContent::Text(document.create_flow(StyleId(1))),
                span: CellSpan::SINGLE,
                frame: None,
                format: Default::default(),
                formula: None,
                legacy: LegacyData::default(),
            })
            .collect();
        let table = document.allocate_frame_id();
        document.insert_frame(Frame {
            id: table,
            placement: Placement::Inline {
                y_offset: 0,
                size: Size {
                    width: 2_880,
                    height: 500,
                },
            },
            style: FrameStyle::default(),
            group: None,
            content: FrameContent::Table(Table {
                columns: vec![1_440, 1_440],
                rows: vec![500],
                cell_margins: CellMargins::default(),
                spacing: Default::default(),
                row_options: Default::default(),
                layout: Default::default(),
                read_rows: Default::default(),
                cells,
                legacy: LegacyData::default(),
            }),
            legacy: LegacyData::default(),
        });
        document.frame_stack_mut().push(table);
        document
            .paragraph_mut(1)
            .objects
            .push(InlineObject::FrameAnchor(table));
        let object = document.add_object(Arc::from(&b"BM synthetic"[..]));
        let picture = document.allocate_frame_id();
        document.insert_frame(Frame {
            id: picture,
            placement: Placement::Repeating {
                pages: RepeatPages::All,
                rect,
            },
            style: FrameStyle::default(),
            group: None,
            content: FrameContent::Picture(Picture {
                source: PictureSource::Embedded(object),
                extension: ".bmp".into(),
                rendering: None,
                scaling: PictureScaling::imported(Size {
                    width: 1_440,
                    height: 1_440,
                }),
                pan: Offset::default(),
                description: String::new(),
                adjust: ImageAdjust::default(),
                legacy: LegacyData::default(),
            }),
            legacy: LegacyData::default(),
        });
        document.document_layout_mut().repeating.push(picture);
        let header = document.document_layout().header.right.unwrap();
        let FrameContent::Text(header_flow) = document.frame(header).unwrap().content else {
            panic!("the header frame holds text");
        };
        let field = document
            .flow_mut(header_flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap();
        field.text = "\u{FFFC}".into();
        field
            .objects
            .push(InlineObject::PageNumber(PageNumberField {
                leading_text: "Page ".into(),
                ..PageNumberField::plain()
            }));
        document.validate().unwrap();
        FrameFixture {
            document,
            fixed,
            grouped,
            below,
            table,
            picture,
        }
    }

    fn fails_with(document: &Document, expected: &str) {
        let message = invalid(document);
        assert!(
            message.contains(expected),
            "expected {expected:?} in {message:?}"
        );
    }

    #[test]
    fn page_numbers_start_on_their_page_with_their_number() {
        // `1308:0026`; the corpora's `<:P10,0,…>`, `<:P12,1,…>`, `<:P13,1,…>`.
        let field = |start_page, start_number| PageNumberField {
            start_page,
            start_number,
            ..PageNumberField::plain()
        };
        let numbers = |field: PageNumberField| {
            (1..=4)
                .map(|page| field.number_on(page))
                .collect::<Vec<_>>()
        };
        assert_eq!(numbers(field(0, 0)), [Some(1), Some(2), Some(3), Some(4)]);
        assert_eq!(numbers(field(2, 1)), [None, Some(1), Some(2), Some(3)]);
        assert_eq!(numbers(field(3, 1)), [None, None, Some(1), Some(2)]);
        assert_eq!(numbers(field(2, 0)), [None, Some(2), Some(3), Some(4)]);
        assert_eq!(numbers(field(0, 5)), [Some(5), Some(6), Some(7), Some(8)]);
        assert_eq!(numbers(field(2, 7)), [None, Some(7), Some(8), Some(9)]);
        assert_eq!(field(9999, 0).number_on(1), Some(1));
    }

    #[test]
    fn new_documents_have_the_standard_layout_with_empty_header_and_footer_frames() {
        let document = Document::default();
        let layout = document.document_layout();
        assert_eq!(layout.name, "Standard");
        assert_eq!((layout.right.width, layout.right.height), (12_240, 15_840));
        assert!(layout.left.is_none() && layout.repeating.is_empty());
        for (frame, placement, top) in [
            (
                layout.header.right,
                Placement::Header {
                    side: PageSide::Right,
                },
                792,
            ),
            (
                layout.footer.right,
                Placement::Footer {
                    side: PageSide::Right,
                },
                72,
            ),
        ] {
            let frame = document.frame(frame.unwrap()).unwrap();
            assert_eq!(frame.placement, placement);
            assert_eq!(frame.style.interior.margins.top, top);
            assert_eq!(
                frame.style.interior.tabs[0],
                [
                    TabStop::new(TabKind::Centre, 4_680),
                    TabStop::new(TabKind::Right, 9_360)
                ]
            );
            let FrameContent::Text(flow) = frame.content else {
                panic!("header and footer frames hold text");
            };
            assert_eq!(document.flow(flow).unwrap().plain_text(), "");
        }
        assert!(document.frame_stack().is_empty());
        assert!(document.validate().is_ok());
    }

    #[test]
    fn every_frame_is_reachable_exactly_once_in_its_own_layer() {
        let fixture = frame_fixture();

        let mut unreachable = fixture.document.clone();
        unreachable
            .frame_stack_mut()
            .retain(|id| *id != fixture.fixed);
        fails_with(&unreachable, "reachable exactly once");

        let mut twice = fixture.document.clone();
        twice.frame_stack_mut().push(fixture.fixed);
        fails_with(&twice, "reachable exactly once");

        let mut layout_and_stack = fixture.document.clone();
        layout_and_stack
            .document_layout_mut()
            .repeating
            .push(fixture.fixed);
        fails_with(&layout_and_stack, "wrong placement");

        let mut repeating_in_stack = fixture.document.clone();
        repeating_in_stack
            .document_layout_mut()
            .repeating
            .retain(|id| *id != fixture.picture);
        repeating_in_stack.frame_stack_mut().push(fixture.picture);
        fails_with(&repeating_in_stack, "wrong placement");

        let mut fixed_header = fixture.document.clone();
        fixed_header
            .frame_stack_mut()
            .retain(|id| *id != fixture.fixed);
        fixed_header.document_layout_mut().header.right = Some(fixture.fixed);
        fails_with(&fixed_header, "wrong placement");

        let mut missing = fixture.document.clone();
        missing.frame_stack_mut().push(FrameId::from_raw(9_999));
        fails_with(&missing, "missing frame 9999");
    }

    #[test]
    fn anchored_frames_have_exactly_one_anchor_in_the_main_flow() {
        let fixture = frame_fixture();

        let mut unanchored = fixture.document.clone();
        let paragraph = unanchored.paragraph_mut(0);
        paragraph.text = "Anchor  here".into();
        paragraph.objects.clear();
        fails_with(&unanchored, "exactly one anchor");

        let mut doubled = fixture.document.clone();
        let paragraph = doubled.paragraph_mut(0);
        paragraph.text.push('\u{FFFC}');
        paragraph
            .objects
            .push(InlineObject::FrameAnchor(fixture.below));
        fails_with(&doubled, "exactly one anchor");

        let mut fixed_anchor = fixture.document.clone();
        fixed_anchor.paragraph_mut(0).objects[0] = InlineObject::FrameAnchor(fixture.fixed);
        fails_with(&fixed_anchor, "exactly one anchor");

        let mut repeating_anchor = fixture.document.clone();
        repeating_anchor.paragraph_mut(0).objects[0] = InlineObject::FrameAnchor(fixture.picture);
        fails_with(&repeating_anchor, "outside the frame stack");

        let mut nested = fixture.document.clone();
        let FrameContent::Text(flow) = nested.frame(fixture.fixed).unwrap().content else {
            panic!("text frame");
        };
        let paragraph = nested.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
        paragraph.text = "\u{FFFC}".into();
        paragraph.objects.push(InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: None,
            restart: false,
        });
        fails_with(&nested, "only in the main flow");
    }

    #[test]
    fn frames_own_their_flows_once() {
        let fixture = frame_fixture();
        let FrameContent::Table(table) = &fixture.document.frame(fixture.table).unwrap().content
        else {
            panic!("table frame");
        };
        let CellContent::Text(cell_flow) = table.cells[0].content;
        let mut shared = fixture.document.clone();
        shared.frame_mut(fixture.fixed).unwrap().content = FrameContent::Text(cell_flow);
        fails_with(&shared, "more than one owner");

        let mut short = fixture.document.clone();
        let FrameContent::Table(table) = &mut short.frame_mut(fixture.table).unwrap().content
        else {
            panic!("table frame");
        };
        table.rows.push(500);
        fails_with(&short, "rows times columns");

        let mut missing_picture = fixture.document.clone();
        let FrameContent::Picture(picture) =
            &mut missing_picture.frame_mut(fixture.picture).unwrap().content
        else {
            panic!("picture frame");
        };
        picture.rendering = Some(ObjectId::from_raw(9_999));
        fails_with(&missing_picture, "missing object 9999");
    }

    /// Joined cells (Ami Pro's connected cells): a block covers exactly the
    /// cells marked covered, inside the grid; spans and cell margins survive
    /// native v3, and records written before them still load.
    #[test]
    fn joined_cells_cover_exactly_their_block_and_round_trip() {
        let fixture = frame_fixture();

        // Records without margins or spans load with the defaults.
        let old_form = fixture
            .document
            .encode_native()
            .unwrap()
            .lines()
            .map(|line| {
                let fields: Vec<&str> = line.split('\t').collect();
                match fields.first() {
                    Some(&"content") if fields.get(1) == Some(&"table") => fields[..4].join("\t"),
                    Some(&"cell") => fields[..2].join("\t"),
                    _ => line.to_owned(),
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(
            Document::decode_native(&old_form).unwrap(),
            fixture.document
        );

        let mut joined = fixture.document.clone();
        let table = table_of(&mut joined, fixture.table);
        table.cells[0].span = CellSpan {
            rows: 1,
            columns: 2,
        };
        table.cells[1].span = CellSpan::COVERED;
        table.cell_margins = CellMargins {
            horizontal: 86,
            vertical: 0,
        };
        joined.validate().unwrap();
        let reopened = Document::decode_native(&joined.encode_native().unwrap()).unwrap();
        assert_eq!(reopened, joined);

        let mut orphan = fixture.document.clone();
        table_of(&mut orphan, fixture.table).cells[1].span = CellSpan::COVERED;
        fails_with(&orphan, "lies in no joined cell");

        let mut too_wide = joined.clone();
        table_of(&mut too_wide, fixture.table).cells[0].span.columns = 3;
        fails_with(&too_wide, "reaches past the table");

        let mut uncovered = joined.clone();
        table_of(&mut uncovered, fixture.table).cells[1].span = CellSpan::SINGLE;
        fails_with(&uncovered, "exactly the covered cells");

        let mut negative = fixture.document.clone();
        table_of(&mut negative, fixture.table).cell_margins.vertical = -1;
        fails_with(&negative, "negative table gutters");
    }

    fn table_of(document: &mut Document, frame: FrameId) -> &mut Table {
        let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
            panic!("table frame");
        };
        table
    }

    /// A picture frame in the fixture table's second cell, at the stack's
    /// front as Ami Pro lists frames in cells first.
    fn with_cell_frame(document: &mut Document, table: FrameId) -> FrameId {
        let object = document.add_object(Arc::from(&b"BM cell"[..]));
        let frame = document.allocate_frame_id();
        document.insert_frame(Frame {
            id: frame,
            placement: Placement::InCell,
            style: FrameStyle::default(),
            group: None,
            content: FrameContent::Picture(Picture {
                source: PictureSource::Embedded(object),
                extension: ".bmp".into(),
                rendering: None,
                scaling: PictureScaling::imported(Size {
                    width: 1_440,
                    height: 1_440,
                }),
                pan: Offset::default(),
                description: String::new(),
                adjust: ImageAdjust::default(),
                legacy: LegacyData::default(),
            }),
            legacy: LegacyData::default(),
        });
        document.frame_stack_mut().insert(0, frame);
        table_of(document, table).cells[1].frame = Some(frame);
        frame
    }

    /// `CELL_FRAME_NOTES.md`: a frame in a cell is in the frame stack and
    /// held by exactly one cell; native v3 keeps the link.
    #[test]
    fn frames_in_cells_are_held_by_exactly_one_cell_and_round_trip() {
        let fixture = frame_fixture();
        let mut document = fixture.document.clone();
        let frame = with_cell_frame(&mut document, fixture.table);
        document.validate().unwrap();
        assert_eq!(document.cell_holding(frame), Some((fixture.table, 1)));
        assert_eq!(
            document.frame(fixture.table).unwrap().owned_frames(),
            [frame]
        );
        let reopened = Document::decode_native(&document.encode_native().unwrap()).unwrap();
        assert_eq!(reopened, document);

        let mut loose = document.clone();
        table_of(&mut loose, fixture.table).cells[1].frame = None;
        fails_with(&loose, "exactly one cell must hold it");
        let mut twice = document.clone();
        table_of(&mut twice, fixture.table).cells[0].frame = Some(frame);
        fails_with(&twice, "exactly one cell must hold it");
        let mut wrong = document.clone();
        table_of(&mut wrong, fixture.table).cells[0].frame = Some(fixture.fixed);
        fails_with(&wrong, "which is not in a cell");
        let mut grouped = document.clone();
        grouped.frame_mut(frame).unwrap().group = Some(GroupId(9_999));
        fails_with(&grouped, "neither grouped nor a table");

        // Removing the frame empties its cell.
        let mut removed = document.clone();
        removed.remove_frame(frame).unwrap();
        assert_eq!(table_of(&mut removed, fixture.table).cells[1].frame, None);
        removed.validate().unwrap();
    }

    #[test]
    fn table_spacing_validates_and_round_trips() {
        let fixture = frame_fixture();
        let mut spaced = fixture.document.clone();
        let table = table_of(&mut spaced, fixture.table);
        table.spacing.top = 300;
        table.spacing.left = 0;
        table.spacing.row_gutters.insert(0, 230);
        table.spacing.column_gutters.insert(1, 28);
        assert_eq!(table.row_gutter(0), 230);
        assert_eq!(table.column_gutter(0), 86);
        assert_eq!(table.column_gutter(1), 28);
        spaced.validate().unwrap();
        let reopened = Document::decode_native(&spaced.encode_native().unwrap()).unwrap();
        assert_eq!(reopened, spaced);
        // Ami Pro's spacing writes no new fields.
        let plain = fixture.document.encode_native().unwrap();
        let record = plain
            .lines()
            .find(|line| line.starts_with("content\ttable"))
            .unwrap();
        assert_eq!(record.split('\t').count(), 6, "{record}");

        let mut outside = fixture.document.clone();
        table_of(&mut outside, fixture.table)
            .spacing
            .column_gutters
            .insert(9, 10);
        fails_with(&outside, "a row or column the table does not have");
        let mut negative = fixture.document.clone();
        table_of(&mut negative, fixture.table).spacing.top = -1;
        fails_with(&negative, "negative table gutters or insets");
    }

    #[test]
    fn table_row_options_validate_and_round_trip() {
        let fixture = frame_fixture();
        let mut options = fixture.document.clone();
        let table = table_of(&mut options, fixture.table);
        assert_eq!(table.row_options, TableRows::default());
        assert!(table.row_options.automatic);
        let last = table.rows.len() - 1;
        table.row_options.automatic = false;
        table.row_options.span_pages = true;
        table.row_options.headings.insert(0);
        table.row_options.page_breaks.insert(last);
        options.validate().unwrap();
        let encoded = options.encode_native().unwrap();
        assert!(
            encoded.contains(&format!("tablerows\t0\t1\t0\t{last}\n")),
            "{encoded}"
        );
        assert_eq!(Document::decode_native(&encoded).unwrap(), options);
        // A new table's options write nothing.
        let plain = fixture.document.encode_native().unwrap();
        assert!(!plain.contains("tablerows"));

        for (bad, message) in [
            ("tablerows\t2\t1\t0\t-", "0 or 1"),
            ("tablerows\t0\t1\t0,0\t-", "each of the table's rows once"),
            ("tablerows\t0\t1\t99\t-", "each of the table's rows once"),
            ("tablerows\t0\t1\tx\t-", "malformed row list"),
            ("tablerows\t0\t1\t0", "fields"),
        ] {
            let text = encoded.replace(&format!("tablerows\t0\t1\t0\t{last}"), bad);
            let error = Document::decode_native(&text).unwrap_err().to_string();
            assert!(error.contains(message), "{bad}: {error}");
        }

        let mut outside = fixture.document.clone();
        table_of(&mut outside, fixture.table)
            .row_options
            .headings
            .insert(99);
        fails_with(&outside, "a heading or page break for a row");
    }

    /// `TABLE_EDIT_NOTES.md`: Modify Table Layout's defaults and options,
    /// rows' and columns' own sizes, and each cell's lines, line colour,
    /// fill, leader, and protection go through native v3; a file written
    /// before them infers the defaults from the sizes.
    #[test]
    fn table_layout_and_cell_formats_round_trip() {
        let fixture = frame_fixture();
        let mut document = fixture.document.clone();
        let table = table_of(&mut document, fixture.table);
        table.layout = TableLayout {
            row_height: 400,
            column_width: 2_000,
            center: true,
            honor_protection: true,
            fixed_sizes: true,
            own_heights: [0].into(),
            own_widths: [1].into(),
        };
        table.cells[0].format = CellFormat {
            lines: CellLines {
                left: 1,
                right: 2,
                top: 3,
                bottom: 12,
            },
            line_colour: 0x00_00FF,
            fill: Some(0),
            leader: CellLeader::Dashes,
            protected: true,
        };
        table.cells[1].format.leader = CellLeader::Line;
        document.validate().unwrap();
        let encoded = document.encode_native().unwrap();
        assert!(
            encoded.contains("tablelayout\t400\t2000\t1\t1\t1\t0\t1\n"),
            "{encoded}"
        );
        assert!(
            encoded.contains("cellformat\t49953\t0000ff\t000000\t2\t1\n"),
            "{encoded}"
        );
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);

        // Without the layout record the first row and column give the
        // defaults and the rest that differ are their own.
        let older: String = encoded
            .lines()
            .filter(|line| !line.starts_with("tablelayout"))
            .map(|line| format!("{line}\n"))
            .collect();
        let reopened = Document::decode_native(&older).unwrap();
        let FrameContent::Table(table) = &reopened.frame(fixture.table).unwrap().content else {
            panic!("a table");
        };
        assert_eq!(
            table.layout,
            TableLayout::inferred(&table.rows, &table.columns)
        );

        for (bad, message) in [
            (
                "cellformat\t49953\t0000ff\t000000\t4\t1",
                "leader is 0 to 3",
            ),
            (
                "cellformat\t49953\tblue\t000000\t2\t1",
                "six hexadecimal digits",
            ),
            (
                "cellformat\t49953\t0000ff\t000000\t2\t2",
                "protection is 0 or 1",
            ),
        ] {
            let text = encoded.replace("cellformat\t49953\t0000ff\t000000\t2\t1", bad);
            let error = Document::decode_native(&text).unwrap_err().to_string();
            assert!(error.contains(message), "{bad}: {error}");
        }
        let mut outside = fixture.document.clone();
        table_of(&mut outside, fixture.table)
            .layout
            .own_widths
            .insert(99);
        fails_with(&outside, "a size of its own for a row or column");
    }

    #[test]
    fn a_rows_fingerprint_follows_what_sets_its_height_and_round_trips() {
        let fixture = frame_fixture();
        let document = fixture.document.clone();
        let table = |document: &Document| match &document.frame(fixture.table).unwrap().content {
            FrameContent::Table(table) => table.clone(),
            _ => panic!("table frame"),
        };
        let print = |document: &Document| document.table_row_fingerprint(&table(document), 0);
        let read = print(&document);
        assert_eq!(read, print(&document.clone()));
        let CellContent::Text(first) = table(&document).cells[0].content;

        // The cell's text, its paragraphs' style, the column's width, and
        // Automatic each change it; the main text does not.
        let mut typed = document.clone();
        Arc::make_mut(&mut typed.flow_mut(first).unwrap().paragraphs[0])
            .text
            .push_str("more");
        assert_ne!(print(&typed), read);
        let mut restyled = document.clone();
        restyled
            .styles_mut()
            .get_mut(&StyleId(1))
            .unwrap()
            .spacing
            .below += 20;
        assert_ne!(print(&restyled), read);
        let mut wider = document.clone();
        table_of(&mut wider, fixture.table).columns[0] += 20;
        assert_ne!(print(&wider), read);
        let mut fixed = document.clone();
        table_of(&mut fixed, fixture.table).row_options.automatic = false;
        assert_ne!(print(&fixed), read);
        let mut main = document.clone();
        main.paragraph_mut(0).text.push_str("more");
        assert_eq!(print(&main), read);

        let mut cached = document.clone();
        table_of(&mut cached, fixture.table)
            .read_rows
            .insert(0, read);
        cached.validate().unwrap();
        let encoded = cached.encode_native().unwrap();
        let record = format!("readrow\t0\t{read:016x}");
        assert!(encoded.contains(&format!("{record}\n")), "{encoded}");
        assert_eq!(Document::decode_native(&encoded).unwrap(), cached);
        assert!(!document.encode_native().unwrap().contains("readrow"));
        for (bad, message) in [
            ("readrow\t0\txyz", "hexadecimal"),
            ("readrow\t1\t00", "once, in order"),
            (&format!("{record}\n{record}"), "once, in order"),
            ("readrow\t0", "fields"),
        ] {
            let text = encoded.replace(&record, bad);
            let error = Document::decode_native(&text).unwrap_err().to_string();
            assert!(error.contains(message), "{bad}: {error}");
        }

        let mut outside = document.clone();
        table_of(&mut outside, fixture.table)
            .read_rows
            .insert(9, read);
        fails_with(&outside, "a cached height for a row");
    }

    #[test]
    fn a_tables_heading_is_the_last_run_of_heading_rows_before_a_part() {
        let mut rows = TableRows::default();
        assert_eq!(rows.heading_before(11), None);
        rows.headings.extend([0, 1, 40]);
        assert_eq!(rows.heading_before(0), None);
        assert_eq!(rows.heading_before(1), Some(0..1));
        assert_eq!(rows.heading_before(11), Some(0..2));
        assert_eq!(rows.heading_before(40), Some(0..2));
        assert_eq!(rows.heading_before(43), Some(40..41));
        // A part starting with a heading row repeats the earlier one (TP2F).
        let mut rows = TableRows::default();
        rows.headings.extend([0, 11]);
        assert_eq!(rows.heading_before(11), Some(0..1));
    }

    #[test]
    fn groups_are_two_or_more_adjacent_where_placed_frames_on_one_page() {
        let fixture = frame_fixture();
        let group = fixture.document.frame(fixture.grouped[0]).unwrap().group;

        let mut single = fixture.document.clone();
        single.frame_mut(fixture.grouped[1]).unwrap().group = None;
        fails_with(&single, "two or more");

        let mut pages = fixture.document.clone();
        let Placement::Fixed { rect, .. } = pages.frame(fixture.grouped[1]).unwrap().placement
        else {
            panic!("fixed frame");
        };
        pages.frame_mut(fixture.grouped[1]).unwrap().placement = Placement::Fixed { page: 3, rect };
        fails_with(&pages, "on one page");

        // Apart among the Where placed frames: a Where placed frame between.
        let mut apart = fixture.document.clone();
        let stack = apart.frame_stack_mut();
        stack.retain(|id| *id != fixture.grouped[1]);
        stack.insert(0, fixture.grouped[1]);
        fails_with(&apart, "adjacent in z-order");
        // Anchored frames between them are in Ami Pro's other list (FC8).
        let mut between = fixture.document.clone();
        let stack = between.frame_stack_mut();
        stack.retain(|id| *id != fixture.grouped[1]);
        stack.push(fixture.grouped[1]);
        assert_eq!(between.validate(), Ok(()));

        let mut anchored = fixture.document.clone();
        anchored.frame_mut(fixture.below).unwrap().group = group;
        fails_with(&anchored, "group");
    }

    #[test]
    fn frame_and_layout_geometry_is_validated() {
        let fixture = frame_fixture();
        let set_fixed = |placement: Placement| {
            let mut document = fixture.document.clone();
            document.frame_mut(fixture.fixed).unwrap().placement = placement;
            document
        };
        fails_with(
            &set_fixed(Placement::Fixed {
                page: 0,
                rect: Rect::new(0, 0, 10, 10),
            }),
            "page 1 or later",
        );
        fails_with(
            &set_fixed(Placement::Fixed {
                page: 1,
                rect: Rect::new(10, 0, 0, 10),
            }),
            "empty or inverted",
        );

        let mut columns = fixture.document.clone();
        columns
            .frame_mut(fixture.fixed)
            .unwrap()
            .style
            .interior
            .columns = 9;
        fails_with(&columns, "1 to 8");
        let mut tabs = fixture.document.clone();
        tabs.frame_mut(fixture.fixed)
            .unwrap()
            .style
            .interior
            .columns = 2;
        fails_with(&tabs, "one tab list per column");
        let mut corners = fixture.document.clone();
        corners.frame_mut(fixture.fixed).unwrap().style.corners = 101;
        fails_with(&corners, "percentage");

        let mut inline_wrap = fixture.document.clone();
        inline_wrap.frame_mut(fixture.table).unwrap().style.wrap = Wrap::NoWrapAround;
        fails_with(&inline_wrap, "wraps around");
        let mut below_wrap = fixture.document.clone();
        below_wrap.frame_mut(fixture.below).unwrap().style.wrap = Wrap::Around;
        fails_with(&below_wrap, "cannot wrap around");

        let mut mirror = fixture.document.clone();
        mirror.document_layout_mut().mirror = true;
        fails_with(&mirror, "need left pages");
        let mut left_header = fixture.document.clone();
        let header = left_header.document_layout().header.right;
        left_header.document_layout_mut().header.left = header;
        fails_with(&left_header, "need left pages");
    }

    /// The frame fixture plus everything else v3 stores: a left page with
    /// its header, page lines, column lines and tabs, an inserted layout
    /// named by a break, a floating footer, runs, legacy marks, legacy data
    /// on every kind of object, a linked picture, and frame names.
    fn everything_document() -> Document {
        let mut document = frame_fixture().document;
        document.styles_mut().get_mut(&StyleId(1)).unwrap().indents = Indents {
            all: 720,
            first: 1_440,
            rest: 360,
            right: 2_160,
        };
        let text = "Bold é🙂 \u{FFFC}tail\twith\ttabs";
        let paragraph = document.paragraph_mut(0);
        paragraph.text = format!("{text} \u{FFFC}");
        paragraph.runs.push(CharacterRun {
            start: 0,
            end: 4,
            format: CharacterFormat {
                font_family: Some("Liberation Serif".into()),
                font_size_points: Some(13.5),
                bold: Some(true),
                italic: Some(false),
                underline: None,
                ..CharacterFormat::default()
            },
        });
        paragraph.marks = vec![
            LegacyMark {
                byte: 0,
                raw: "<:#284,9360>".into(),
            },
            LegacyMark {
                byte: 4,
                raw: "<:N multi\nline\tnote>".into(),
            },
        ];
        let footer_flow = document.create_flow(StyleId(2));
        let paragraph = document.paragraph_mut(0);
        paragraph.objects.insert(
            0,
            InlineObject::FloatingHeaderFooter {
                kind: HeaderFooterKind::Footer,
                pages: PageSides::Left,
                flow: footer_flow,
            },
        );
        let mut inserted = document.new_standard_layout("1");
        inserted.right.border = Some(Border {
            sides: Sides {
                left: true,
                right: false,
                top: true,
                bottom: false,
            },
            style: 3,
            colour: 255,
            position: LinePosition::Outside,
        });
        inserted.right.layout.columns = 2;
        inserted.right.layout.tabs = vec![
            vec![TabStop {
                kind: TabKind::Decimal,
                position: 720,
                leader: TabLeader::None,
            }],
            Vec::new(),
        ];
        inserted.right.layout.line_between = Some(ColumnLine {
            style: 2,
            colour: 0x00FF_0000,
        });
        inserted.right.layout.balance = true;
        inserted.legacy.push("lay.number", "513");
        let inserted_id = inserted.id;
        document.layouts_mut().push(Arc::new(inserted));
        // The document starts in the inserted layout (Ami Pro's `[l1]`).
        document.set_start_layout(Some(inserted_id));
        let second = document.paragraph_mut(1);
        second.text.push('\u{FFFC}');
        second.objects.push(InlineObject::Break {
            kind: BreakKind::Page,
            centre: true,
            layout: Some(inserted_id),
            restart: false,
        });
        let right = document.document_layout().right.clone();
        let header = document.document_layout().header.right.unwrap();
        let left_header = document.allocate_frame_id();
        let mut copy = document.frame(header).unwrap().clone();
        let left_flow = document.create_flow(StyleId(1));
        copy.id = left_header;
        copy.placement = Placement::Header {
            side: PageSide::Left,
        };
        copy.content = FrameContent::Text(left_flow);
        document.insert_frame(copy);
        let layout = document.document_layout_mut();
        layout.left = Some(right);
        layout.mirror = true;
        layout.header.left = Some(left_header);
        layout.footer_from_second_page = true;
        layout.paper = 4;
        layout.landscape = true;
        document.legacy_mut().push("section [ver]", "4");
        document
            .legacy_mut()
            .push("section [l1]", "line\twith\\escapes\r");
        let fixed = document.frame_stack()[0];
        let frame = document.frame_mut(fixed).unwrap();
        frame.style.name = Some("Frame3".into());
        frame.style.macro_name = Some("RUN.SMM".into());
        frame.style.shadow = Shadow {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
            colour: 0x00_80_80_80,
        };
        frame.legacy.push("frame.field8", "3");
        let table = document
            .frames()
            .find(|frame| matches!(frame.content, FrameContent::Table(_)))
            .unwrap()
            .id;
        let FrameContent::Table(table) = &mut document.frame_mut(table).unwrap().content else {
            unreachable!();
        };
        table.legacy.push("tbl.sizes", "1 2 3");
        table.cells[1]
            .legacy
            .push("cell", "0 1 0 0 0 0 0 0 1 0 0 0 0");
        let linked = document.allocate_frame_id();
        let rendering = document.add_object(Arc::from(vec![0x53, 0x53, 0, 255, 7]));
        document.insert_frame(Frame {
            id: linked,
            placement: Placement::Fixed {
                page: 3,
                rect: Rect::new(-100, 0, 500, 600),
            },
            style: FrameStyle::default(),
            group: None,
            content: FrameContent::Picture(Picture {
                source: PictureSource::Linked("L:\\IN\\PIC.BMP".into()),
                extension: ".bmp".into(),
                rendering: Some(rendering),
                scaling: PictureScaling {
                    sizing: PictureSizing::Custom,
                    keep_aspect: false,
                    percent: 63,
                    size: Size {
                        width: 1_440,
                        height: 360,
                    },
                    rotation: 90,
                },
                pan: Offset { x: 936, y: -144 },
                description: "A linked\tpicture, é 🙂".into(),
                adjust: ImageAdjust {
                    brightness: 30,
                    contrast: 70,
                    edge: 5,
                    smoothing: 6,
                    invert: true,
                    halftone: Halftone::Posterize,
                },
                legacy: LegacyData {
                    values: vec![LegacyValue {
                        key: "isd".into(),
                        value: ".X3 .bmp L:\\IN\\PIC.BMP 0 0 12".into(),
                    }],
                },
            }),
            legacy: LegacyData::default(),
        });
        document.frame_stack_mut().insert(0, linked);
        let drawing = document.allocate_frame_id();
        let mut legacy = LegacyData::default();
        legacy.push("section [drw]", "raw drawing data");
        document.insert_frame(Frame {
            id: drawing,
            placement: Placement::Repeating {
                pages: RepeatPages::RightLeft,
                rect: Rect::new(0, 0, 10, 10),
            },
            style: FrameStyle::default(),
            group: None,
            content: FrameContent::Legacy(legacy),
            legacy: LegacyData::default(),
        });
        document.document_layout_mut().repeating.push(drawing);
        document.validate().unwrap();
        document
    }

    /// A file written before pictures had a scaling record takes it from
    /// the kept `[isd]` values (PI8's), or Import Picture's when it has none.
    #[test]
    fn native_files_without_picture_scaling_read_it_from_the_kept_values() {
        let mut document = Document::from_plain_text("Text");
        let object = document.add_object(Arc::from(&b"BM"[..]));
        let frame = document.allocate_frame_id();
        let mut picture = Picture::new(
            PictureSource::Embedded(object),
            ".bmp",
            Size {
                width: 1_440,
                height: 1_440,
            },
        );
        picture.legacy.push(
            "isd",
            [
                ".X3", ".bmp", ".X3", "8", "1", "0", "0", "1440", "65176", "100", "0", "0",
            ]
            .into_iter()
            .chain([".bmp", "936", "65392", "0"])
            .collect::<Vec<_>>()
            .join("\n"),
        );
        document.insert_frame(Frame {
            id: frame,
            placement: Placement::Fixed {
                page: 1,
                rect: Rect::new(0, 0, 1_440, 1_440),
            },
            style: FrameStyle::default(),
            group: None,
            content: FrameContent::Picture(picture),
            legacy: LegacyData::default(),
        });
        document.frame_stack_mut().push(frame);
        let older: String = document
            .encode_native()
            .unwrap()
            .lines()
            .filter(|line| !line.starts_with("picturescaling\t"))
            .map(|line| format!("{line}\n"))
            .collect();
        let read = Document::decode_native(&older).unwrap();
        let FrameContent::Picture(picture) = &read.frame(frame).unwrap().content else {
            unreachable!();
        };
        assert_eq!(picture.scaling.sizing, PictureSizing::Custom);
        assert_eq!(picture.scaling.size.height, 360);
        assert_eq!(picture.pan, Offset { x: 936, y: 144 });
        assert!(picture.kept_is_current());
    }

    #[test]
    fn native_v3_round_trips_every_model_feature_exactly() {
        let document = everything_document();
        let encoded = document.encode_native().unwrap();
        assert!(encoded.starts_with("AMIPRO-LINUX/3\n"));
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);
        assert_eq!(
            Document::decode_native(&encoded)
                .unwrap()
                .encode_native()
                .unwrap(),
            encoded,
            "encoding is deterministic"
        );
    }

    #[test]
    fn line_heights_round_trip_in_native_v3() {
        let mut document = Document::from_plain_text("Text");
        let mut heights = LineHeights::default();
        heights.insert(FaceKey::new("Times New Roman", 12.0, false, false), 284);
        heights.insert(FaceKey::new("Arial", 10.0, true, false), 237);
        document.set_line_heights(heights);
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\nlineheight\tarial\t200\t1\t0\t237\n"));
        assert!(encoded.contains("\nlineheight\ttimes new roman\t240\t0\t0\t284\n"));
        let decoded = Document::decode_native(&encoded).unwrap();
        assert_eq!(decoded, document);
        assert_eq!(
            decoded
                .line_heights()
                .get(&FaceKey::new("TIMES NEW ROMAN", 12.0, false, false)),
            Some(284),
            "faces compare without letter case"
        );
        // Face kinds (pitch and family bytes) are kept the same way.
        let mut kinds = document.clone();
        kinds.set_face_kinds(BTreeMap::from([
            ("haettenschweiler".to_owned(), 0x22),
            ("times new roman".to_owned(), 0x12),
        ]));
        let with_kinds = kinds.encode_native().unwrap();
        assert!(
            with_kinds
                .contains("\nfacekind\thaettenschweiler\t34\nfacekind\ttimes new roman\t18\n")
        );
        assert_eq!(Document::decode_native(&with_kinds).unwrap(), kinds);
        let line = "facekind\thaettenschweiler\t34\n";
        for broken in [
            with_kinds.replacen(line, &format!("{line}{line}"), 1),
            with_kinds.replacen(line, "facekind\tHaettenschweiler\t34\n", 1),
            with_kinds.replacen(line, "facekind\thaettenschweiler\t256\n", 1),
            with_kinds.replacen(line, "facekind\t\t34\n", 1),
            with_kinds.replacen("\nlineheight\t", &format!("\n{line}lineheight\t"), 1),
        ] {
            assert!(Document::decode_native(&broken).is_err(), "{broken}");
        }
        // Files without the record load with none.
        let plain = Document::from_plain_text("Text").encode_native().unwrap();
        assert!(!plain.contains("lineheight"));
        assert!(
            Document::decode_native(&plain)
                .unwrap()
                .line_heights()
                .is_empty()
        );
        // Duplicates, invalid heights, and records out of order are refused.
        let line = "lineheight\tarial\t200\t1\t0\t237\n";
        let duplicated = encoded.replacen(line, &format!("{line}{line}"), 1);
        assert!(Document::decode_native(&duplicated).is_err());
        let zero = encoded.replace("\t237\n", "\t0\n");
        assert!(Document::decode_native(&zero).is_err());
        let mut late = Document::from_plain_text("Text");
        late.legacy_mut().push("key", "value");
        let late = late.encode_native().unwrap();
        let before_legacy = late.replacen("doclegacy", &format!("{line}doclegacy"), 1);
        assert!(Document::decode_native(&before_legacy).is_ok());
        let after = late.replacen("\nobject", &format!("\n{line}object"), 1);
        let after = if after == late {
            late.replacen("\nlayout", &format!("\n{line}layout"), 1)
        } else {
            after
        };
        assert!(Document::decode_native(&after).is_err());
    }

    #[test]
    fn paragraph_formats_round_trip_in_native_v3() {
        let mut document = Document::from_plain_text("Indented\nSpaced\nPlain");
        document.paragraph_mut(0).format.indents = Some(Indents {
            all: 360,
            first: 720,
            rest: 1_080,
            right: 1_440,
        });
        document.paragraph_mut(1).format.line_spacing = Some(LineSpacing::OneAndAHalf(0));
        document.paragraph_mut(1).format.alignment = Some(Alignment::Justify);
        let style = document.paragraphs()[2].style;
        document.styles_mut().get_mut(&style).unwrap().alignment = Alignment::Centre;
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\npformat\tindents\t360\t720\t1080\t1440\n"));
        assert!(encoded.contains("\npformat\tline\thalf\t0\n"));
        assert!(encoded.contains("\npformat\talign\tjustify\n"));
        assert_eq!(encoded.matches("pformat").count(), 3, "none for Plain");
        assert!(
            encoded
                .lines()
                .any(|line| line.starts_with("style\t") && line.ends_with("\t4\tcentre")),
            "a centred style writes its alignment as the twentieth field"
        );
        assert_eq!(Document::decode_native(&encoded).unwrap(), document);

        // Out of order, unknown, or invalid records are errors.
        let late = encoded.replacen(
            "pformat\tline\thalf\t0\n",
            "mark\t0\t<x>\npformat\tline\thalf\t0\n",
            1,
        );
        assert!(Document::decode_native(&late).is_err());
        let unknown = encoded.replacen("pformat\tline", "pformat\tcolour", 1);
        assert!(Document::decode_native(&unknown).is_err());
        let negative = encoded.replacen("\thalf\t0\n", "\tcustom\t0\n", 1);
        assert!(Document::decode_native(&negative).is_err());
    }

    #[test]
    fn style_indents_round_trip_and_old_style_records_load_without_them() {
        let document = everything_document();
        let encoded = document.encode_native().unwrap();
        assert!(encoded.contains("\tBody Text\t0\t"));
        assert!(encoded.lines().any(|line| line.starts_with("style\t1\t")
            && line.ends_with("\t720\t1440\t360\t2160\tsingle\t0\t0\t0\t100\t4")));
        // Spacing (S1–S6) round-trips; a record with only the indents (as
        // written for a while on 2026-10-12) still loads.
        let mut spaced = document.clone();
        spaced.styles_mut().get_mut(&StyleId(1)).unwrap().spacing = Spacing {
            line: LineSpacing::Custom(360),
            above: 144,
            below: 288,
            add_at_break: true,
            tightness: 90,
        };
        let encoded_spacing = spaced.encode_native().unwrap();
        assert_eq!(Document::decode_native(&encoded_spacing).unwrap(), spaced);
        assert!(encoded_spacing.contains("\tcustom\t360\t144\t1\t90\t4\n"));
        let indents_only = encoded.replace("\tsingle\t0\t0\t0\t100\t4\n", "\n");
        assert_eq!(Document::decode_native(&indents_only).unwrap(), document);
        // Break rules (B1–B8) round-trip with every bit as written; a record
        // without them (18 fields) loads with Ami Pro's default.
        let mut kept = document.clone();
        kept.styles_mut().get_mut(&StyleId(1)).unwrap().breaks =
            Breaks(Breaks::KEEP_WITH_NEXT | Breaks::NEXT_STYLE | 32);
        let encoded_breaks = kept.encode_native().unwrap();
        assert_eq!(Document::decode_native(&encoded_breaks).unwrap(), kept);
        let without_breaks = encoded.replace("\t100\t4\n", "\t100\n");
        assert_eq!(Document::decode_native(&without_breaks).unwrap(), document);
        assert!(Breaks::default().within());
        assert!(!Breaks(Breaks::KEEP_WITH_NEXT).within());
        // A style without indents keeps the nine-field record.
        let plain = Document::default().encode_native().unwrap();
        assert!(
            plain
                .lines()
                .filter(|line| line.starts_with("style\t"))
                .all(|line| line.split('\t').count() == 9)
        );
        let reopened = Document::decode_native(&plain).unwrap();
        assert!(
            reopened
                .styles()
                .values()
                .all(|style| style.indents.is_zero())
        );
        // Ten fields is neither.
        let broken = plain.replacen("Roman\t12\t0\t0\t0\n", "Roman\t12\t0\t0\t0\t1\n", 1);
        assert!(Document::decode_native(&broken).is_err());

        let mut negative = Document::default();
        negative
            .styles_mut()
            .get_mut(&StyleId(1))
            .unwrap()
            .indents
            .rest = -1;
        assert!(negative.validate().is_err());
        assert_eq!(
            Indents {
                all: 720,
                first: 1_440,
                rest: 360,
                right: 0
            }
            .left(true),
            2_160
        );
    }

    #[test]
    fn native_v3_leaves_out_unreferenced_payloads() {
        let mut document = everything_document();
        document.add_object(Arc::from(&b"orphan"[..]));
        let reopened = Document::decode_native(&document.encode_native().unwrap()).unwrap();
        assert_ne!(reopened, document);
        document.drop_unreferenced_objects();
        assert_eq!(reopened, document);
    }

    #[test]
    fn native_v3_follows_the_specification_example() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Hello".into();
        let header = document.document_layout().header.right.unwrap();
        let FrameContent::Text(flow) = document.frame(header).unwrap().content else {
            unreachable!();
        };
        let field = document.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
        field.text = "\u{FFFC}".into();
        field
            .objects
            .push(InlineObject::PageNumber(PageNumberField {
                leading_text: "Page ".into(),
                ..PageNumberField::plain()
            }));
        let specification = include_str!("../../../docs/Design/NATIVE_FORMAT_V3.md");
        let example = specification
            .split("```text\nAMIPRO-LINUX/3\n")
            .last()
            .and_then(|rest| rest.split("```").next())
            .expect("the specification has an example");
        assert_eq!(
            document.encode_native().unwrap(),
            format!("AMIPRO-LINUX/3\n{example}")
        );
    }

    #[test]
    fn native_v3_reads_page_number_records_written_before_the_style_character() {
        let specification = include_str!("../../../docs/Design/NATIVE_FORMAT_V3.md");
        let example = specification
            .split("```text\nAMIPRO-LINUX/3\n")
            .last()
            .and_then(|rest| rest.split("```").next())
            .expect("the specification has an example");
        let written = format!("AMIPRO-LINUX/3\n{example}");
        let current = "inline\tpagenumber\t1\t0\t0\tPage ";
        assert!(written.contains(current));
        let reopened = |record: &str| {
            Document::decode_native(&written.replace(current, record)).map(|document| {
                document
                    .flows()
                    .flat_map(|(_, flow)| flow.paragraphs.clone())
                    .flat_map(|paragraph| paragraph.objects.clone())
                    .find_map(|object| match object {
                        InlineObject::PageNumber(field) => Some(field),
                        _ => None,
                    })
            })
        };
        let field = |style, start_page, start_number, text: &str| PageNumberField {
            style,
            start_page,
            start_number,
            leading_text: text.into(),
        };
        assert_eq!(
            reopened("inline\tpagenumber\t10\t0\tPage ").unwrap(),
            Some(field('1', 0, 0, "Page "))
        );
        assert_eq!(
            reopened("inline\tpagenumber\t111\t15\t- ").unwrap(),
            Some(field('1', 11, 15, "- "))
        );
        assert_eq!(
            reopened("inline\tpagenumber\ti\t1\t1\t").unwrap(),
            Some(field('i', 1, 1, ""))
        );
        assert!(reopened("inline\tpagenumber\tii\t1\t1\t").is_err());
    }

    #[test]
    fn native_v3_rejects_malformed_input() {
        let encoded = everything_document().encode_native().unwrap();
        let lines: Vec<&str> = encoded.lines().collect();
        let mutate = |predicate: &dyn Fn(&str) -> bool, change: &dyn Fn(&str) -> String| {
            let mut changed = false;
            let text: Vec<String> = lines
                .iter()
                .map(|line| {
                    if !changed && predicate(line) {
                        changed = true;
                        change(line)
                    } else {
                        (*line).to_owned()
                    }
                })
                .collect();
            assert!(changed, "the mutation found its line");
            text.join("\n") + "\n"
        };
        let rejected = |input: String, why: &str| {
            assert!(
                Document::decode_native(&input).is_err(),
                "accepted input with {why}"
            );
        };
        rejected(
            mutate(&|line| line.starts_with("flow\t"), &|line| {
                format!("unknown\trecord\n{line}")
            }),
            "an unknown record",
        );
        rejected(
            mutate(&|line| line.starts_with("inline\tanchor"), &|_| {
                "inline\tanchor\t9999".into()
            }),
            "a dangling frame ID",
        );
        rejected(
            mutate(&|line| line.starts_with("inline\tfloating"), &|line| {
                line.rsplit_once('\t').unwrap().0.to_owned() + "\t9998"
            }),
            "a dangling flow ID",
        );
        rejected(
            mutate(&|line| line.starts_with("blob\t"), &|line| {
                format!("{line}!")
            }),
            "broken base64",
        );
        rejected(
            mutate(&|line| line.starts_with("object\t"), &|line| {
                let (head, _) = line.rsplit_once('\t').unwrap();
                format!("{head}\t3")
            }),
            "a wrong payload length",
        );
        rejected(
            mutate(&|line| line.starts_with("content\ttable"), &|_| {
                "content\ttable\t3\t1".into()
            }),
            "a wrong table shape",
        );
        rejected(
            mutate(&|line| line.starts_with("stack\t"), &|line| {
                format!("{line}\t{}", line.split('\t').nth(1).unwrap())
            }),
            "a frame stacked twice",
        );
        rejected(
            mutate(&|line| line.starts_with("tabs\t1"), &|_| String::new()),
            "a missing tab list",
        );
        rejected(
            mutate(&|line| line.starts_with("ids\t"), &|_| "ids\t3\t1".into()),
            "a counter below the stored IDs",
        );
        rejected(
            mutate(&|line| line.starts_with("paragraph\t"), &|line| {
                format!("{line}\\q")
            }),
            "a bad escape",
        );
        rejected(
            mutate(&|line| line.starts_with("layout\t"), &|line| {
                format!("flow\t1\n{line}")
            }),
            "a record out of order",
        );
        rejected(
            mutate(&|line| line.starts_with("look\t"), &|line| {
                line.replace("\t1\t", "\tyes\t")
            }),
            "a malformed boolean",
        );
        rejected(
            mutate(&|line| line.starts_with("mark\t"), &|_| {
                "mark\t9999\t<:x>".into()
            }),
            "a mark past its paragraph",
        );
        for length in (1..encoded.len()).step_by(97) {
            let truncated = &encoded[..encoded.floor_char_boundary(length)];
            let _ = Document::decode_native(truncated);
        }
    }

    #[test]
    fn versions_one_and_two_still_load_into_the_new_model() {
        let v2 = "AMIPRO-LINUX/2\ntitle\tOld\npage\t612\t792\t72\t72\t72\t72\nstyle\t1\tBody Text\t0\tNoto Serif\t12\t0\t0\t6\nparagraph\t1\tbold then plain\nrun\t0\t0\t4\t0\t\t-\t1\t-\t-\n";
        let document = Document::decode_native(v2).unwrap();
        assert_eq!(document.paragraphs()[0].runs[0].format.bold, Some(true));
        assert!(document.document_layout().header.right.is_some());
        let reopened = Document::decode_native(&document.encode_native().unwrap()).unwrap();
        assert_eq!(reopened, document, "v2 content survives a v3 save exactly");
    }

    #[test]
    fn an_empty_main_flow_is_invalid() {
        let document = Document::empty("Empty", default_styles());
        assert_eq!(
            document.validate(),
            Err(DocumentError::Invalid(
                "a document requires at least one paragraph".into()
            ))
        );
    }
}
