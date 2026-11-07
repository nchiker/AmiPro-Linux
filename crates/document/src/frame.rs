//! Frames: "a mini-document within the main document" (Ami Pro 3.1 user's
//! guide), with their placement, appearance, interior layout, and content
//! (`docs/research/DECISIONS.md`, Phase 2.2, D4 and D7).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::Arc;

use crate::geometry::{
    BLACK, Border, Colour, LayoutBox, LegacyData, LinePosition, Margins, Rect, Sides, Size, Twips,
    WHITE,
};
use crate::{DocumentError, FlowId, FrameId, GroupId, ObjectId};

/// Which pages a header or footer frame belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageSide {
    Right,
    Left,
}

/// Which pages a repeating frame appears on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepeatPages {
    All,
    /// Ami Pro's "Repeat right/left".
    RightLeft,
}

/// Where a frame is placed (Ami Pro's Placement options).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Placement {
    /// "Where placed": on one page, at a fixed position. Ami Pro adds pages
    /// so that the page exists.
    Fixed { page: u32, rect: Rect },
    /// "With para above": below the paragraph that holds its anchor, offset
    /// from the left margin.
    BelowParagraph { x_offset: Twips, size: Size },
    /// "Flow with text": inline at its anchor, like a large character.
    /// `y_offset` is how far the frame's bottom hangs below the baseline (0:
    /// it stands on it). Ami Pro keeps the frame's top-to-baseline distance
    /// when the frame is resized (F18, F19).
    Inline { y_offset: Twips, size: Size },
    /// "Repeat all pages" or "Repeat right/left": part of a page layout.
    Repeating { pages: RepeatPages, rect: Rect },
    /// A page layout's fixed header; its rectangle is the top margin band.
    Header { side: PageSide },
    /// A page layout's fixed footer; its rectangle is the bottom margin band.
    Footer { side: PageSide },
    /// In a table cell (the cell names the frame, [`TableCell::frame`]): the
    /// frame covers the cell's text area wherever the table is laid out
    /// (`docs/research/CELL_FRAME_NOTES.md`; VM TF1–TF15).
    InCell,
}

impl Placement {
    /// Whether the frame is anchored to a mark in the main text.
    pub fn is_anchored(self) -> bool {
        matches!(self, Self::BelowParagraph { .. } | Self::Inline { .. })
    }

    /// Whether the frame belongs to the document's frame stack (rather than a
    /// page layout). Frames in table cells are there too, as Ami Pro keeps
    /// them with the page frames, in a list of their own (`1248:0347`).
    pub fn is_page_frame(self) -> bool {
        matches!(
            self,
            Self::Fixed { .. } | Self::BelowParagraph { .. } | Self::Inline { .. } | Self::InCell
        )
    }

    /// Whether the frame is in a table cell.
    pub fn is_in_cell(self) -> bool {
        self == Self::InCell
    }

    /// The frame's size where the placement holds it (not for header,
    /// footer, and cell frames, whose size comes from elsewhere).
    pub fn size(self) -> Option<Size> {
        match self {
            Self::Fixed { rect, .. } | Self::Repeating { rect, .. } => Some(Size {
                width: rect.width(),
                height: rect.height(),
            }),
            Self::BelowParagraph { size, .. } | Self::Inline { size, .. } => Some(size),
            Self::Header { .. } | Self::Footer { .. } | Self::InCell => None,
        }
    }

    pub(crate) fn validate(self) -> Result<(), &'static str> {
        match self {
            Self::Fixed { page, rect } => {
                if page == 0 {
                    return Err("fixed frames are on page 1 or later");
                }
                rect.is_valid()
                    .then_some(())
                    .ok_or("frame rectangle is empty or inverted")
            }
            Self::Repeating { rect, .. } => rect
                .is_valid()
                .then_some(())
                .ok_or("frame rectangle is empty or inverted"),
            Self::BelowParagraph { size, .. } | Self::Inline { size, .. } => size
                .is_valid()
                .then_some(())
                .ok_or("frame size must be positive"),
            Self::Header { .. } | Self::Footer { .. } | Self::InCell => Ok(()),
        }
    }
}

/// How text outside the frame behaves around it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Wrap {
    /// "Wrap around".
    Around,
    /// "No wrap around": text runs behind or through the frame.
    NoWrapAround,
    /// "No wrap beside": no text beside the frame.
    NoWrapBeside,
}

/// A frame's shadow, as extents on the sides it falls on (Normal 100 twips,
/// Deep 172).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Shadow {
    pub left: Twips,
    pub top: Twips,
    pub right: Twips,
    pub bottom: Twips,
    pub colour: Colour,
}

/// A frame's appearance and interior, everything but its placement, group,
/// and content. Commands set it as one value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameStyle {
    pub wrap: Wrap,
    pub opaque: bool,
    pub border: Border,
    pub shadow: Shadow,
    /// Rounded-corner percentage, 0 (square) to 100.
    pub corners: u8,
    pub background: Colour,
    pub interior: LayoutBox,
    pub name: Option<String>,
    pub macro_name: Option<String>,
}

impl Default for FrameStyle {
    /// Ami Pro's default text frame: opaque, wrap around, a thin black line
    /// on all sides, 0.10 in margins, one column.
    fn default() -> Self {
        Self {
            wrap: Wrap::Around,
            opaque: true,
            border: Border {
                sides: Sides::ALL,
                style: 1,
                colour: BLACK,
                position: LinePosition::Middle,
            },
            shadow: Shadow::default(),
            corners: 0,
            background: WHITE,
            interior: LayoutBox::single_column(Margins::uniform(144)),
            name: None,
            macro_name: None,
        }
    }
}

/// A line style's width in twips, as Ami Pro measures a frame's text area
/// with it (`1288:0800`); 0 for a style it does not know.
pub fn line_style_width(style: u16) -> Twips {
    match style {
        1 => 10,
        2 => 20,
        3 => 40,
        4 => 80,
        5 | 10 => 120,
        6 => 160,
        7 => 240,
        8 => 60,
        9 | 11 | 12 => 100,
        _ => 0,
    }
}

impl FrameStyle {
    /// How far the frame's text lies inside its edges on a frame `width` by
    /// `height` twips: its margins, and with rounded corners also their
    /// inset, never nearer an edge than its line (`1260:0827`, `1530:0892`,
    /// `1530:0c22`). The inset is (the shorter side less twice the line's
    /// width) × corner% × 3 / 2000, truncated, the same across and down but
    /// at 100 % (F01's 57, F03's and FC1's 38, FC2's 57; FC3's tiny frame,
    /// without left and right margins, keeps 10 twips: the line's width).
    pub fn text_insets(&self, width: Twips, height: Twips) -> Margins {
        let margins = self.interior.margins;
        if self.corners == 0 {
            return margins;
        }
        let line = line_style_width(self.border.style);
        let inset = |side: Twips| {
            if side < 2 * line {
                0
            } else {
                (i64::from(side - 2 * line) * i64::from(self.corners) * 3 / 2000) as Twips
            }
        };
        let (mut across, mut down) = (inset(width), inset(height));
        if self.corners != 100 {
            across = across.min(down);
            down = across;
        }
        let half = line / 2;
        let edge = |inset: Twips, margin: Twips| {
            let by_line = match self.border.position {
                LinePosition::Middle => half + margin / 2,
                LinePosition::Inside => margin,
                LinePosition::Outside => line,
                LinePosition::CloseToInside => half - margin / 4 + margin,
                LinePosition::CloseToOutside => margin / 4 + half,
            };
            (by_line.max(line) + inset).max(margin + inset)
        };
        Margins {
            top: edge(down, margins.top),
            right: edge(across, margins.right),
            bottom: edge(down, margins.bottom),
            left: edge(across, margins.left),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), DocumentError> {
        self.interior.validate("frame interior")?;
        if self.corners > 100 {
            return Err(DocumentError::Invalid(
                "rounded corners are a percentage up to 100".into(),
            ));
        }
        let shadow = self.shadow;
        if [shadow.left, shadow.top, shadow.right, shadow.bottom]
            .iter()
            .any(|extent| *extent < 0)
        {
            return Err(DocumentError::Invalid("negative shadow extent".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub id: FrameId,
    pub placement: Placement,
    pub style: FrameStyle,
    pub group: Option<GroupId>,
    pub content: FrameContent,
    pub legacy: LegacyData,
}

impl Frame {
    /// The flows this frame owns: its text, or its table cells' text.
    pub fn owned_flows(&self) -> Vec<FlowId> {
        match &self.content {
            FrameContent::Text(flow) => vec![*flow],
            FrameContent::Table(table) => table
                .cells
                .iter()
                .map(|cell| match cell.content {
                    CellContent::Text(flow) => flow,
                })
                .collect(),
            FrameContent::Picture(_) | FrameContent::Legacy(_) => Vec::new(),
        }
    }

    /// The frames this frame's table cells hold.
    pub fn owned_frames(&self) -> Vec<FrameId> {
        match &self.content {
            FrameContent::Table(table) => {
                table.cells.iter().filter_map(|cell| cell.frame).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The embedded payloads this frame refers to.
    pub fn objects(&self) -> Vec<ObjectId> {
        match &self.content {
            FrameContent::Picture(picture) => {
                let mut objects = Vec::new();
                if let PictureSource::Embedded(object) = picture.source {
                    objects.push(object);
                }
                objects.extend(picture.rendering);
                objects
            }
            _ => Vec::new(),
        }
    }
}

/// A frame holds exactly one of text, a table, or a picture (V21).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameContent {
    Text(FlowId),
    Table(Table),
    Picture(Picture),
    /// Content the model does not interpret yet (drawings, charts,
    /// equations), kept for a future writer.
    Legacy(LegacyData),
}

/// A table: column widths, row heights, gutters, and row-major cells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Table {
    /// Each column's text width.
    pub columns: Vec<Twips>,
    /// Each row's least text height; a row grows to fit its text.
    pub rows: Vec<Twips>,
    /// The table's default gutters, after each column and each row.
    pub cell_margins: CellMargins,
    /// The insets before the first row and column, and the rows and
    /// columns with gutters of their own.
    pub spacing: TableSpacing,
    /// How the rows take their height and continue across pages.
    pub row_options: TableRows,
    /// Modify Table Layout's defaults and options, and which rows and
    /// columns have a size of their own.
    pub layout: TableLayout,
    /// Ami Pro's row heights as a cache: the rows of an Automatic table
    /// whose stored height is Ami Pro's own measure of their text, each with
    /// its [`crate::Document::table_row_fingerprint`] when read. Ami Pro
    /// measures every Automatic row again when it opens a file, so a height
    /// it stored is its measure with its printer's fonts (VM RC1–RC3,
    /// `docs/research/TABLE_PAGES_NOTES.md`, "The row cache"); layout keeps
    /// that height while the row is as read (user decision 2026-10-19).
    pub read_rows: BTreeMap<usize, u64>,
    pub cells: Vec<TableCell>,
    pub legacy: LegacyData,
}

/// How a table's rows take their height and continue on later columns and
/// pages (`docs/research/TABLE_PAGES_NOTES.md`): Modify Table Layout's
/// Automatic and Rows span pages, Table › Headings, and page breaks in
/// rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableRows {
    /// Automatic row height (`[tbl]` value 7 bit 1): a row grows to fit its
    /// text; without it a row keeps its size and its text is cut (TP2E).
    pub automatic: bool,
    /// Rows span pages (bit `0x100`): a row may split between two of its
    /// lines at a column's end (TP5). Ami Pro offers it only with
    /// Automatic.
    pub span_pages: bool,
    /// Heading rows (`[h]` flag `0x10`): a part of the table after the
    /// first starts with the last run of them before its rows (TP2B, TP2F).
    pub headings: BTreeSet<usize>,
    /// Rows that start the next page (`[h]` flag `0x20`, TP6).
    pub page_breaks: BTreeSet<usize>,
}

impl Default for TableRows {
    /// A new table's: Automatic, nothing else.
    fn default() -> Self {
        Self {
            automatic: true,
            span_pages: false,
            headings: BTreeSet::new(),
            page_breaks: BTreeSet::new(),
        }
    }
}

impl TableRows {
    /// The heading a part of the table starting at row `first` repeats:
    /// the last run of consecutive heading rows before it, even when
    /// `first` is a heading row itself (`1468:044f`, `1468:05a8`; TP2F).
    pub fn heading_before(&self, first: usize) -> Option<Range<usize>> {
        let last = *self.headings.range(..first).next_back()?;
        let mut start = last;
        while start > 0 && self.headings.contains(&(start - 1)) {
            start -= 1;
        }
        Some(start..last + 1)
    }
}

/// Ami Pro's Modify Table Layout beyond the gutters and the row options
/// (`docs/research/TABLE_EDIT_NOTES.md`): the default row height and column
/// width (`[tbl]` values 3 and 5) that rows and columns without a size of
/// their own take, three options of value 7, and the rows and columns with
/// a size of their own (`[h]`/`[w]` flag 2: Column/Row Size, a dragged grid
/// line, or an Automatic row Ami Pro measured).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableLayout {
    pub row_height: Twips,
    pub column_width: Twips,
    /// Center table on page (value 7 bit `0x4`, VM TE13): a page table in
    /// the middle of its column.
    pub center: bool,
    /// Honor protection (bit `0x80`, TE12): the caret stays out of
    /// protected cells.
    pub honor_protection: bool,
    /// Disable mouse col/row sizing (bit `0x400`, TE15).
    pub fixed_sizes: bool,
    pub own_heights: BTreeSet<usize>,
    pub own_widths: BTreeSet<usize>,
}

impl Default for TableLayout {
    /// A new table's: one 12-point line, one inch (Create Table's Layout…),
    /// no option.
    fn default() -> Self {
        Self {
            row_height: 284,
            column_width: 1_440,
            center: false,
            honor_protection: false,
            fixed_sizes: false,
            own_heights: BTreeSet::new(),
            own_widths: BTreeSet::new(),
        }
    }
}

impl TableLayout {
    /// Defaults for a table read without them (native files written
    /// before 2026-10-24): its first row's height and first column's width,
    /// every row and column that differs counted as its own.
    pub fn inferred(rows: &[Twips], columns: &[Twips]) -> Self {
        let mut layout = Self::default();
        if let Some(height) = rows.first() {
            layout.row_height = *height;
        }
        if let Some(width) = columns.first() {
            layout.column_width = *width;
        }
        layout.own_heights = (0..rows.len())
            .filter(|row| rows[*row] != layout.row_height)
            .collect();
        layout.own_widths = (0..columns.len())
            .filter(|column| columns[*column] != layout.column_width)
            .collect();
        layout
    }
}

/// The table's default gutters: the space after each column's text
/// (`horizontal`) and after each row's (`vertical`), Ami Pro's Gutter size
/// (values 6 and 4 of a `[tbl]` header; TB1, V20, README31, TF14). Rows and
/// columns can have their own ([`TableSpacing`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellMargins {
    pub horizontal: Twips,
    pub vertical: Twips,
}

impl Default for CellMargins {
    /// What Ami Pro writes for a new table.
    fn default() -> Self {
        Self {
            horizontal: 86,
            vertical: 86,
        }
    }
}

/// Where a table's cells lie beyond their sizes and the default gutters
/// (`docs/research/CELL_FRAME_NOTES.md`, "Where the frame goes"): the text
/// of column c starts `left` plus each earlier column's width and gutter
/// in from the table's left, and of row r `top` plus each earlier row's
/// height and gutter down from its top (`[tbl]` header values 8 and 9,
/// VM TF14X; `[h]` and `[w]` records with flag 4, `1428:046b`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableSpacing {
    pub top: Twips,
    pub left: Twips,
    /// Rows with a gutter of their own, by index.
    pub row_gutters: BTreeMap<usize, Twips>,
    /// Columns with a gutter of their own, by index.
    pub column_gutters: BTreeMap<usize, Twips>,
}

impl Default for TableSpacing {
    /// Ami Pro's: 43 above and left, no gutters of their own.
    fn default() -> Self {
        Self {
            top: TableSpacing::INSET,
            left: TableSpacing::INSET,
            row_gutters: BTreeMap::new(),
            column_gutters: BTreeMap::new(),
        }
    }
}

impl TableSpacing {
    /// The inset Ami Pro gives every table, and reads when a file has none.
    pub const INSET: Twips = 43;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableCell {
    pub content: CellContent,
    pub span: CellSpan,
    /// The frame the cell holds (a picture, drawing, chart, or equation; an
    /// [`Placement::InCell`] frame), which covers its text area. Ami Pro
    /// keeps a cell's text and frame apart: it puts pictures only into
    /// cells without text and ignores typing in a frame's cell (TF12).
    pub frame: Option<FrameId>,
    /// The cell's lines, fill, leader, and protection (Table › Lines &
    /// Color, Leaders, Protect Cells).
    pub format: CellFormat,
    /// The formula as typed (Table › Edit Formula…, Quick Add), without the
    /// style prefix SAM stores it with: `@sum(A1..A4)`, `D4*E4`. The cell's
    /// text is its last result (stored plain, `number::NumberFormat::stored_text`);
    /// [`crate::formula`] compiles and calculates the text. SAM keeps it as
    /// the cell record's second body (F3 bit `0x40`).
    pub formula: Option<String>,
    pub legacy: LegacyData,
}

/// What Table › Leaders fills the rest of a cell with (cell record F3
/// `0xC000`: `0x4000` dots, `0x8000` dashes, `0xC000` a line; VM TE10–TE10D),
/// drawn on its last line by its alignment (`13e8:2422`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CellLeader {
    #[default]
    None,
    Dots,
    Dashes,
    Line,
}

impl CellLeader {
    /// The cell record's F3 bits.
    pub fn bits(self) -> u16 {
        match self {
            Self::None => 0,
            Self::Dots => 0x4000,
            Self::Dashes => 0x8000,
            Self::Line => 0xC000,
        }
    }

    pub fn from_bits(flags: u16) -> Self {
        match flags & 0xC000 {
            0x4000 => Self::Dots,
            0x8000 => Self::Dashes,
            0xC000 => Self::Line,
            _ => Self::None,
        }
    }
}

/// A cell's own look (`docs/research/TABLE_EDIT_NOTES.md`): its lines
/// (record F7), their colour (the app's own, TD8: Ami Pro's are black and
/// Save As SAM lists another), its fill (F3 `0x200`, F6, F10), its leader
/// (F3), and whether it is protected (F9, VM TE11).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellFormat {
    pub lines: CellLines,
    pub line_colour: Colour,
    pub fill: Option<Colour>,
    pub leader: CellLeader,
    pub protected: bool,
}

impl Default for CellFormat {
    fn default() -> Self {
        Self {
            lines: CellLines::default(),
            line_colour: BLACK,
            fill: None,
            leader: CellLeader::None,
            protected: false,
        }
    }
}

impl CellFormat {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// A cell's lines, Ami Pro's Table › Lines & Color (`TBLLINESPROC`,
/// `13a8:0000`): a line style (1–12, as paragraph rules, [`crate::rule_strokes`];
/// 0 none) for each side. Lines are black: the dialog has no line colour.
/// Each is drawn centred on the cell's box edge, the middle of the gutter
/// between two cells, or of the table's inset before the first row and
/// column (VM fixture TL1).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CellLines {
    pub left: u8,
    pub right: u8,
    pub top: u8,
    pub bottom: u8,
}

impl CellLines {
    /// The cell record's line word (its seventh value): left in bits 0–3,
    /// right 4–7, top 8–11, bottom 12–15 (`13e8:1232`; VM fixture TL1,
    /// All: `0x1111`).
    pub fn from_word(word: u16) -> Self {
        let side = |shift: u16| ((word >> shift) & 0xF) as u8;
        Self {
            left: side(0),
            right: side(4),
            top: side(8),
            bottom: side(12),
        }
    }

    pub fn is_none(self) -> bool {
        self == Self::default()
    }

    /// The line word Ami Pro writes.
    pub fn word(self) -> u16 {
        u16::from(self.left & 0xF)
            | u16::from(self.right & 0xF) << 4
            | u16::from(self.top & 0xF) << 8
            | u16::from(self.bottom & 0xF) << 12
    }
}

impl TableCell {
    /// The cell's lines.
    pub fn lines(&self) -> CellLines {
        self.format.lines
    }

    /// The cell's fill colour, Lines & Color's Fill color.
    pub fn fill(&self) -> Option<Colour> {
        self.format.fill
    }
}

impl CellFormat {
    /// The look a cell record as Ami Pro wrote it holds (`ROW COL F3 …
    /// F12`): lines in F7 (`13e8:1232`; VM TL1, All: `0x1111`), the fill in
    /// F10 while F3 has `0x200`, black too (TL4: yellow; VM TF0X, a corpus
    /// letter's black heading row; without the bit nothing is drawn, TF0Y),
    /// the leader in F3, protection in F9 (TE11).
    pub fn from_record(record: &[i64]) -> Self {
        let value = |index: usize| record.get(index).copied().unwrap_or(0);
        Self {
            lines: CellLines::from_word(value(6) as u16),
            line_colour: BLACK,
            fill: (value(2) & 0x200 != 0).then(|| value(9) as Colour & WHITE),
            leader: CellLeader::from_bits(value(2) as u16),
            protected: value(8) != 0,
        }
    }
}

/// The rows and columns a cell covers (Ami Pro's connected cells): 1 × 1 for
/// an ordinary cell, more for the top-left cell of a joined block, and 0 × 0
/// for a cell inside a block, which shows nothing of its own.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellSpan {
    pub rows: u16,
    pub columns: u16,
}

impl CellSpan {
    pub const SINGLE: Self = Self {
        rows: 1,
        columns: 1,
    };
    pub const COVERED: Self = Self {
        rows: 0,
        columns: 0,
    };

    pub fn is_covered(self) -> bool {
        self == Self::COVERED
    }
}

impl Default for CellSpan {
    fn default() -> Self {
        Self::SINGLE
    }
}

impl Table {
    /// The gutter after column `column`: its own, or the table's.
    pub fn column_gutter(&self, column: usize) -> Twips {
        self.spacing
            .column_gutters
            .get(&column)
            .copied()
            .unwrap_or(self.cell_margins.horizontal)
    }

    /// The gutter after row `row`: its own, or the table's.
    pub fn row_gutter(&self, row: usize) -> Twips {
        self.spacing
            .row_gutters
            .get(&row)
            .copied()
            .unwrap_or(self.cell_margins.vertical)
    }

    /// The text area of cell `index` (joined cells together) from the
    /// table's top-left, by the stored column widths and row heights:
    /// Ami Pro's grid (`docs/research/CELL_FRAME_NOTES.md`), on which a
    /// frame in the cell lies. Layout grows rows to their text; this does
    /// not.
    pub fn cell_text_area(&self, index: usize) -> Option<Rect> {
        let count = self.columns.len();
        let cell = self.cells.get(index)?;
        if count == 0 || cell.span.is_covered() {
            return None;
        }
        let (row, column) = (index / count, index % count);
        let span = |start: usize, length: u16, sizes: &[Twips], gutter: &dyn Fn(usize) -> Twips| {
            let end = (start + usize::from(length.max(1))).min(sizes.len());
            let offset: Twips = (0..start).map(|k| sizes[k] + gutter(k)).sum();
            let extent: Twips = (start..end).map(|k| sizes[k]).sum::<Twips>()
                + (start..end.saturating_sub(1)).map(gutter).sum::<Twips>();
            (offset, extent)
        };
        let (x, width) = span(column, cell.span.columns, &self.columns, &|k| {
            self.column_gutter(k)
        });
        let (y, height) = span(row, cell.span.rows, &self.rows, &|k| self.row_gutter(k));
        let (left, top) = (self.spacing.left + x, self.spacing.top + y);
        Some(Rect::new(left, top, left + width, top + height))
    }

    /// Checks the gutters and insets: none negative, own gutters only for
    /// rows and columns the table has.
    pub fn validate_spacing(&self) -> Result<(), &'static str> {
        let spacing = &self.spacing;
        if self.cell_margins.horizontal < 0
            || self.cell_margins.vertical < 0
            || spacing.top < 0
            || spacing.left < 0
            || spacing.row_gutters.values().any(|gutter| *gutter < 0)
            || spacing.column_gutters.values().any(|gutter| *gutter < 0)
        {
            return Err("negative table gutters or insets");
        }
        if spacing
            .row_gutters
            .keys()
            .any(|row| *row >= self.rows.len())
            || spacing
                .column_gutters
                .keys()
                .any(|column| *column >= self.columns.len())
        {
            return Err("a gutter for a row or column the table does not have");
        }
        if self
            .layout
            .own_heights
            .last()
            .is_some_and(|row| *row >= self.rows.len())
            || self
                .layout
                .own_widths
                .last()
                .is_some_and(|column| *column >= self.columns.len())
        {
            return Err("a size of its own for a row or column the table does not have");
        }
        let options = &self.row_options;
        if options
            .headings
            .iter()
            .chain(&options.page_breaks)
            .any(|row| *row >= self.rows.len())
        {
            return Err("a heading or page break for a row the table does not have");
        }
        if self
            .read_rows
            .last_key_value()
            .is_some_and(|(row, _)| *row >= self.rows.len())
        {
            return Err("a cached height for a row the table does not have");
        }
        Ok(())
    }

    /// Checks that joined blocks lie inside the grid, do not overlap, and
    /// cover exactly the cells marked covered.
    pub fn validate_spans(&self) -> Result<(), &'static str> {
        let columns = self.columns.len();
        if columns == 0 {
            return Err("a table has at least one column");
        }
        let mut covered_by = vec![false; self.cells.len()];
        for (index, cell) in self.cells.iter().enumerate() {
            let span = cell.span;
            if span.is_covered() {
                continue;
            }
            if span.rows == 0 || span.columns == 0 {
                return Err("a cell spans at least one row and one column");
            }
            let (row, column) = (index / columns, index % columns);
            if row + usize::from(span.rows) > self.rows.len()
                || column + usize::from(span.columns) > columns
            {
                return Err("a joined cell reaches past the table");
            }
            for inner_row in row..row + usize::from(span.rows) {
                for inner_column in column..column + usize::from(span.columns) {
                    let inner = inner_row * columns + inner_column;
                    if inner == index {
                        continue;
                    }
                    if !self.cells[inner].span.is_covered() || covered_by[inner] {
                        return Err("a joined cell must cover exactly the covered cells");
                    }
                    covered_by[inner] = true;
                }
            }
        }
        if self
            .cells
            .iter()
            .zip(&covered_by)
            .any(|(cell, covered)| cell.span.is_covered() && !covered)
        {
            return Err("a covered cell lies in no joined cell");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellContent {
    Text(FlowId),
}

/// A picture in a frame (`docs/research/PICTURE_NOTES.md`): its bytes or
/// the file it links to, and how it is sized, rotated, and moved in the
/// frame (Ami Pro's Graphics Scaling and picture mode; `[isd]` values 4–11
/// and the pan).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Picture {
    pub source: PictureSource,
    /// The source file's extension, such as `.bmp`.
    pub extension: String,
    /// Ami Pro's own rendering of the picture: a screen image it keeps as a
    /// cache (`1168:2016`), drawn only for kinds the app cannot read yet.
    pub rendering: Option<ObjectId>,
    pub scaling: PictureScaling,
    /// How far the picture is moved in its frame from the picture area's
    /// top-left corner (picture mode), y growing downward.
    pub pan: Offset,
    /// A description of the picture (alt text). The native format keeps it;
    /// Ami Pro's files cannot.
    pub description: String,
    /// Image Processing (`AmiImage`): non-destructive adjustments of a
    /// bitmap, drawn live; the bytes are never changed. A gray-scale TIFF
    /// (`.tgf`) keeps them in `[isd]` as Ami Pro does; any other bitmap is
    /// baked in when saved as SAM.
    pub adjust: ImageAdjust,
    /// What Ami Pro stored that the model does not type: the `[isd]` values
    /// as read (key `isd`), written back only while they still say what the
    /// typed fields say, with the driver's saved state.
    pub legacy: LegacyData,
}

/// How a gray-scale image is halftoned for printing (`AmiImage`; the codes
/// are those stored in the state, IMV1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Halftone {
    Fastest = 1,
    BestPictureQuality = 2,
    Automatic = 3,
    Posterize = 4,
    PrinterDriver = 5,
}

impl Halftone {
    /// The menu's order is not the codes' order; this is the codes'.
    pub const ALL: [Self; 5] = [
        Self::Fastest,
        Self::BestPictureQuality,
        Self::Automatic,
        Self::Posterize,
        Self::PrinterDriver,
    ];

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| *mode as u8 == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fastest => "Fastest",
            Self::BestPictureQuality => "Best picture quality",
            Self::Automatic => "Automatically selected",
            Self::Posterize => "Posterize",
            Self::PrinterDriver => "Printer driver",
        }
    }
}

/// Image Processing's settings (`amiimage.dll`, notes 11.3 and 12.3): four
/// 0 to 100 values, Invert, and the halftone mode (a print setting).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageAdjust {
    pub brightness: u8,
    pub contrast: u8,
    pub edge: u8,
    pub smoothing: u8,
    pub invert: bool,
    pub halftone: Halftone,
}

impl Default for ImageAdjust {
    /// The dialog's neutral state: 50, 50, 0, 0, not inverted, Fastest (as
    /// the VM's untouched image stores it).
    fn default() -> Self {
        Self {
            brightness: 50,
            contrast: 50,
            edge: 0,
            smoothing: 0,
            invert: false,
            halftone: Halftone::Fastest,
        }
    }
}

/// The count of 16-bit words of the `.tgf` driver state in `[isd]`.
pub const TGF_STATE_WORDS: usize = 32;

impl ImageAdjust {
    /// Whether the pixels are drawn as stored (the halftone mode changes
    /// printing only).
    pub fn is_neutral(&self) -> bool {
        self.brightness == 50
            && self.contrast == 50
            && self.edge == 0
            && self.smoothing == 0
            && !self.invert
    }

    /// The same with the values kept in 0 to 100.
    pub fn clamped(mut self) -> Self {
        self.brightness = self.brightness.min(100);
        self.contrast = self.contrast.min(100);
        self.edge = self.edge.min(100);
        self.smoothing = self.smoothing.min(100);
        self
    }

    /// The state Ami Pro keeps for a `.tgf` frame (`1018:1691`, IMV1):
    /// `DB DB`, the record count 8, records of (tag word, length word,
    /// data) for version 1, `10 03`, brightness, contrast, edge,
    /// smoothing, invert, halftone, then `DF DF`; as 16-bit words, the
    /// rest zero.
    pub fn tgf_words(&self) -> [u16; TGF_STATE_WORDS] {
        let mut bytes: Vec<u8> = vec![0xDB, 0xDB, 8, 0];
        let mut record = |tag: u16, data: &[u8]| {
            bytes.extend_from_slice(&tag.to_le_bytes());
            bytes.extend_from_slice(&(data.len() as u16).to_le_bytes());
            bytes.extend_from_slice(data);
        };
        record(1, &[1]);
        record(2, &[0x10, 0x03]);
        record(3, &[self.brightness]);
        record(4, &[self.contrast]);
        record(5, &[self.edge]);
        record(6, &[self.smoothing]);
        record(7, &[u8::from(self.invert)]);
        record(8, &[self.halftone as u8]);
        bytes.extend_from_slice(&[0xDF, 0xDF]);
        let mut words = [0u16; TGF_STATE_WORDS];
        for (word, pair) in words.iter_mut().zip(bytes.chunks(2)) {
            *word = u16::from_le_bytes([pair[0], *pair.get(1).unwrap_or(&0)]);
        }
        words
    }

    /// The settings a `.tgf` state says, `None` when it is not Ami Pro's
    /// blob (the version, the record layout, and the end marker).
    pub fn from_tgf_words(words: &[u16]) -> Option<Self> {
        let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        if bytes.get(..4)? != [0xDB, 0xDB, 8, 0] {
            return None;
        }
        let mut at = 4;
        let mut found: [Option<Vec<u8>>; 9] = [const { None }; 9];
        for _ in 0..8 {
            let tag = usize::from(u16::from_le_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]));
            let len = usize::from(u16::from_le_bytes([
                *bytes.get(at + 2)?,
                *bytes.get(at + 3)?,
            ]));
            let data = bytes.get(at + 4..at + 4 + len)?;
            if !(1..=8).contains(&tag) {
                return None;
            }
            found[tag] = Some(data.to_vec());
            at += 4 + len;
        }
        if bytes.get(at..at + 2)? != [0xDF, 0xDF] {
            return None;
        }
        let byte = |tag: usize| found[tag].as_ref().and_then(|data| data.first().copied());
        let adjust = Self {
            brightness: byte(3)?,
            contrast: byte(4)?,
            edge: byte(5)?,
            smoothing: byte(6)?,
            invert: byte(7)? != 0,
            halftone: Halftone::from_code(byte(8)?)?,
        };
        (adjust == adjust.clamped()).then_some(adjust)
    }
}

/// The largest picture Ami Pro draws, 22 in (`1168:2dd2`, `FRAMESCALEPROC`).
pub const PICTURE_MAX: Twips = 31_680;

/// A picture's Graphics Scaling (`FRAMESCALEPROC`; `[isd]` values 4 and
/// 6–11).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PictureScaling {
    pub sizing: PictureSizing,
    /// Maintain aspect ratio (`[isd]` flag 0x10).
    pub keep_aspect: bool,
    /// Percentage's percent of the natural size.
    pub percent: u16,
    /// Custom's size. In the other modes it is the size the picture last
    /// had (Ami Pro's stored rectangle), which [`PictureScaling::drawn_size`]
    /// recomputes, as Ami Pro does every time it draws (`1168:2dd2`).
    pub size: Size,
    /// Degrees, 0 to 359 (bitmaps and drawings).
    pub rotation: u16,
}

/// How a picture is sized in its frame: Ami Pro's Graphics Scaling options
/// (`[isd]` value 4's low bits 1, 2, 4, 8).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PictureSizing {
    /// At its natural size.
    Original,
    /// The picture area, or within it with the aspect ratio kept. What Ami
    /// Pro gives an imported picture.
    FitInFrame,
    /// The natural size times [`PictureScaling::percent`].
    Percentage,
    /// [`PictureScaling::size`].
    Custom,
}

/// An offset in twips, y growing downward.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Offset {
    pub x: Twips,
    pub y: Twips,
}

impl PictureScaling {
    /// What Ami Pro's Import Picture gives a picture: Fit in frame with the
    /// aspect ratio kept, 100 %, not rotated (PI1–PI4).
    pub fn imported(size: Size) -> Self {
        Self {
            sizing: PictureSizing::FitInFrame,
            keep_aspect: true,
            percent: 100,
            size,
            rotation: 0,
        }
    }

    /// The picture's size in a picture area of `area` when its natural size
    /// is `natural`, as Ami Pro computes it every time it draws
    /// (`1168:2dd2`): Original the natural size; Percentage the natural
    /// size times the percent, truncated; Fit in frame the area, or with the
    /// aspect ratio kept the natural size scaled by the smaller of the two
    /// ratios, truncated; Custom the stored size. An axis below 0 or past
    /// 22 in is 22 in.
    pub fn drawn_size(&self, natural: Size, area: Size) -> Size {
        let scale = |value: Twips, by: i64, of: i64| (i64::from(value) * by / of.max(1)) as Twips;
        let size = match self.sizing {
            PictureSizing::Original => natural,
            PictureSizing::Percentage => {
                let percent = i64::from(self.percent);
                Size {
                    width: scale(natural.width, percent, 100),
                    height: scale(natural.height, percent, 100),
                }
            }
            PictureSizing::FitInFrame if !self.keep_aspect => area,
            PictureSizing::FitInFrame => {
                let width = i64::from(natural.width.max(1));
                let height = i64::from(natural.height.max(1));
                let (by, of) = if i64::from(area.height) * 32_000 / height
                    < i64::from(area.width) * 32_000 / width
                {
                    (i64::from(area.height), height)
                } else {
                    (i64::from(area.width), width)
                };
                Size {
                    width: scale(natural.width, by, of),
                    height: scale(natural.height, by, of),
                }
            }
            PictureSizing::Custom => self.size,
        };
        let axis = |value: Twips| {
            if (0..=PICTURE_MAX).contains(&value) {
                value
            } else {
                PICTURE_MAX
            }
        };
        Size {
            width: axis(size.width),
            height: axis(size.height),
        }
    }

    /// `[isd]` value 4: the sizing's bit and 0x10 for the aspect ratio.
    pub fn flags(&self) -> u16 {
        let sizing = match self.sizing {
            PictureSizing::Original => 1,
            PictureSizing::FitInFrame => 2,
            PictureSizing::Percentage => 4,
            PictureSizing::Custom => 8,
        };
        sizing | if self.keep_aspect { 0x10 } else { 0 }
    }

    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self.percent == 0 {
            return Err("a picture's percentage is 0");
        }
        if self.rotation >= 360 {
            return Err("a picture's rotation is 360 degrees or more");
        }
        if self.sizing == PictureSizing::Custom
            && !((1..=PICTURE_MAX).contains(&self.size.width)
                && (1..=PICTURE_MAX).contains(&self.size.height))
        {
            return Err("a picture's custom size is outside 1 to 31,680 twips");
        }
        Ok(())
    }
}

/// A word as Ami Pro writes it, unsigned: −544 is 64992.
fn signed_word(value: &str) -> Option<Twips> {
    let value: i32 = value.trim().parse().ok()?;
    match value {
        32_768..=65_535 => Some(value - 65_536),
        -32_768..=32_767 => Some(value),
        _ => None,
    }
}

impl Picture {
    /// A picture as Ami Pro's Import Picture leaves it
    /// ([`PictureScaling::imported`]), not moved.
    pub fn new(source: PictureSource, extension: impl Into<String>, size: Size) -> Self {
        Self {
            source,
            extension: extension.into(),
            rendering: None,
            scaling: PictureScaling::imported(size),
            pan: Offset::default(),
            description: String::new(),
            adjust: ImageAdjust::default(),
            legacy: LegacyData::default(),
        }
    }

    /// The scaling and pan the kept `[isd]` values say (`11c8:2e29`):
    /// value 4 the flags, 6–9 the rectangle (twips, y upward, written as
    /// unsigned words), 10 the percent, 11 the rotation, 12 the count of the
    /// driver's state words, then the driver's type and the pan (none in
    /// older versions' records). `None` when there are none or they do not
    /// read.
    pub fn kept_scaling(&self) -> Option<(PictureScaling, Offset)> {
        let values: Vec<&str> = self.legacy.get("isd")?.split('\n').collect();
        let flags: u16 = values.get(3)?.trim().parse().ok()?;
        let sizing = match flags & 0xf {
            1 => PictureSizing::Original,
            2 => PictureSizing::FitInFrame,
            4 => PictureSizing::Percentage,
            8 => PictureSizing::Custom,
            _ => return None,
        };
        let word = |index: usize| values.get(index).and_then(|value| signed_word(value));
        let (left, top, right, bottom) = (word(5)?, word(6)?, word(7)?, word(8)?);
        let percent: u16 = values.get(9)?.trim().parse().ok()?;
        let rotation: u16 = values.get(10)?.trim().parse().ok()?;
        // Ami Pro 3.1's layout names the driver after its state and then
        // gives the pan; older versions' records (the corpus's `.G0x`
        // pictures) end otherwise and have no pan.
        let after = values
            .get(11)
            .and_then(|count| count.trim().parse::<usize>().ok())
            .and_then(|count| 12usize.checked_add(count))
            .filter(|after| {
                values
                    .get(*after)
                    .is_some_and(|value| value.starts_with('.'))
            });
        let pan = match after {
            Some(after) => Offset {
                x: word(after + 1)?,
                y: -word(after + 2)?,
            },
            None => Offset::default(),
        };
        let scaling = PictureScaling {
            sizing,
            keep_aspect: flags & 0x10 != 0,
            percent,
            size: Size {
                width: right - left,
                height: top - bottom,
            },
            rotation,
        };
        Some((scaling, pan))
    }

    /// Whether the kept `[isd]` values still say this picture's scaling and
    /// pan, so a writer may write them back as read.
    pub fn kept_is_current(&self) -> bool {
        self.kept_scaling() == Some((self.scaling, self.pan))
    }

    /// Whether this is a gray-scale TIFF in Ami Pro's own form (`.tgf`):
    /// the TIFF's bytes and the adjustments in the driver state.
    pub fn is_gray_tiff(&self) -> bool {
        self.format() == PictureFormat::GrayTiff
    }

    /// The adjustments the kept `[isd]` driver state says (IMV1): its 32
    /// words after the count in value 12, `None` when there is none or it
    /// is not Ami Pro's blob.
    pub fn kept_adjust(&self) -> Option<ImageAdjust> {
        let values: Vec<&str> = self.legacy.get("isd")?.split('\n').collect();
        let count: usize = values.get(11)?.trim().parse().ok()?;
        if count != TGF_STATE_WORDS || !values.get(12 + count)?.trim().eq_ignore_ascii_case(".tgf")
        {
            return None;
        }
        let words: Option<Vec<u16>> = values[12..12 + count]
            .iter()
            .map(|value| value.trim().parse().ok())
            .collect();
        ImageAdjust::from_tgf_words(&words?)
    }

    /// Whether the kept driver state still says the adjustments (true when
    /// nothing was kept to say otherwise).
    pub fn kept_adjust_is_current(&self) -> bool {
        self.kept_adjust().is_none_or(|kept| kept == self.adjust)
    }

    /// Where the picture is placed in its frame: its pan, except for an
    /// equation, whose pan word holds the baseline (`[isd]` carries it
    /// there, `OBJECTS_NOTES.md` §12.2) and which is always drawn at the
    /// picture area's corner.
    pub fn placed_pan(&self) -> Offset {
        if self.format() == PictureFormat::Equation {
            Offset::default()
        } else {
            self.pan
        }
    }

    /// What kind of picture this is, from its extension and, for charts,
    /// the driver Ami Pro named after its state.
    pub fn format(&self) -> PictureFormat {
        if self.legacy.get("isd").is_some_and(|values| {
            values
                .split('\n')
                .any(|value| value.eq_ignore_ascii_case(".sct"))
        }) {
            return PictureFormat::Chart;
        }
        PictureFormat::from_extension(&self.extension)
    }

    /// Whether Ami Pro's files can hold this picture as it is: one Ami Pro
    /// stored with one of its drivers (a linked TIFF it converts on loading
    /// is a bitmap to it), or one of the kinds its drivers take (P8).
    pub fn ami_pro_stores(&self) -> bool {
        let driver = self.legacy.get("isd").and_then(|values| {
            let values: Vec<&str> = values.split('\n').collect();
            let count: usize = values.get(11)?.trim().parse().ok()?;
            values.get(12 + count).map(|value| value.trim())
        });
        driver.map_or_else(
            || self.format().ami_pro_stores(),
            |driver| PictureFormat::from_extension(driver).ami_pro_stores(),
        )
    }

    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        self.scaling.validate()?;
        if self.pan.x.abs() > 4 * PICTURE_MAX || self.pan.y.abs() > 4 * PICTURE_MAX {
            return Err("a picture is moved too far in its frame");
        }
        Ok(())
    }
}

/// The kinds of picture: those Ami Pro stores (its drivers, `[AmiVISD]`)
/// and those the app also takes (decision P1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PictureFormat {
    /// A Windows bitmap (`AmiPaint`).
    Bitmap,
    /// A Windows metafile (`AmiWMF`).
    Metafile,
    /// An enhanced metafile (Word's and LibreOffice's drawings), from an
    /// imported document; Save As SAM writes it as a bitmap.
    Emf,
    /// An Ami Pro drawing (`AmiDraw`).
    Drawing,
    /// A gray-scale TIFF in Ami Pro's own form (`AmiImage`, driver
    /// `.tgf`): the TIFF's bytes unchanged, its adjustments in `[isd]`.
    GrayTiff,
    /// An Ami Pro chart, stored as a drawing (`AmiChart`).
    Chart,
    /// An equation (`AmiEqn`).
    Equation,
    /// An OLE object (`AmiOLE`).
    Ole,
    /// Encapsulated PostScript with its preview.
    Eps,
    Png,
    Jpeg,
    Gif,
    Tiff,
    Pcx,
    Svg,
    Other,
}

impl PictureFormat {
    pub fn from_extension(extension: &str) -> Self {
        let extension = extension.trim_start_matches('.').to_ascii_lowercase();
        match extension.as_str() {
            "bmp" | "dib" => Self::Bitmap,
            "wmf" => Self::Metafile,
            "emf" => Self::Emf,
            "sdw" => Self::Drawing,
            "tgf" => Self::GrayTiff,
            "sct" => Self::Chart,
            "tex" => Self::Equation,
            "ole" => Self::Ole,
            "eps" => Self::Eps,
            "png" => Self::Png,
            "jpg" | "jpeg" | "jpe" => Self::Jpeg,
            "gif" => Self::Gif,
            "tif" | "tiff" => Self::Tiff,
            "pcx" => Self::Pcx,
            "svg" => Self::Svg,
            _ => Self::Other,
        }
    }

    /// Whether Ami Pro's files can store pictures of this kind as they are
    /// (its drivers' types); the others are written as bitmaps (P8).
    pub fn ami_pro_stores(self) -> bool {
        matches!(
            self,
            Self::Bitmap
                | Self::Metafile
                | Self::GrayTiff
                | Self::Drawing
                | Self::Chart
                | Self::Equation
                | Self::Ole
                | Self::Eps
        )
    }

    /// The kind's name, as Frame Properties and the boxes show it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Bitmap => "Windows Bitmap",
            Self::Metafile => "Windows Metafile",
            Self::Emf => "Enhanced Metafile",
            Self::Drawing => "Drawing",
            Self::GrayTiff => "Gray-scale TIFF",
            Self::Chart => "Chart",
            Self::Equation => "Equation",
            Self::Ole => "OLE object",
            Self::Eps => "EPS picture",
            Self::Png => "PNG image",
            Self::Jpeg => "JPEG image",
            Self::Gif => "GIF image",
            Self::Tiff => "TIFF image",
            Self::Pcx => "PCX image",
            Self::Svg => "SVG image",
            Self::Other => "Picture",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PictureSource {
    Embedded(ObjectId),
    /// A file the picture is read from when drawn: a path as Ami Pro stored
    /// it, or one the user linked (decision P2).
    Linked(String),
}

/// Bytes embedded in the document, shared between roots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddedObject {
    pub bytes: Arc<[u8]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(record: &str) -> CellFormat {
        let values: Vec<i64> = record
            .split_whitespace()
            .filter_map(|value| value.parse().ok())
            .collect();
        CellFormat::from_record(&values)
    }

    #[test]
    fn a_cells_leader_and_protection_come_from_its_record() {
        // VM fixtures TE10, TE10B, TE10C (Leaders on B2) and TE11 (Protect).
        assert_eq!(cell("1 1 16384 0 0 0 0 1 0 0 0 0").leader, CellLeader::Dots);
        assert_eq!(
            cell("1 1 32768 0 0 0 0 1 0 0 0 0").leader,
            CellLeader::Dashes
        );
        assert_eq!(cell("1 1 49152 0 0 0 0 1 0 0 0 0").leader, CellLeader::Line);
        let protected = cell("1 1 0 0 0 0 0 1 1 0 0 0");
        assert!(protected.protected);
        assert_eq!(protected.leader, CellLeader::None);
        for leader in [
            CellLeader::None,
            CellLeader::Dots,
            CellLeader::Dashes,
            CellLeader::Line,
        ] {
            assert_eq!(CellLeader::from_bits(leader.bits() | 0x384), leader);
        }
        let lines = CellLines {
            left: 1,
            right: 2,
            top: 3,
            bottom: 12,
        };
        assert_eq!(CellLines::from_word(lines.word()), lines);
    }

    #[test]
    fn a_cells_lines_and_fill_come_from_its_record() {
        // VM fixtures TL1 (Lines & Color › All) and TL4 (Fill color yellow).
        let all = cell("0 0 0 0 0 0 4369 1 0 0 0 0");
        assert_eq!(
            all.lines,
            CellLines {
                left: 1,
                right: 1,
                top: 1,
                bottom: 1
            }
        );
        assert_eq!(all.fill, None);
        let filled = cell("0 1 512 0 0 9 0 1 0 65535 0 0");
        assert!(filled.lines.is_none());
        assert_eq!(filled.fill, Some(0x0000_FFFF));
        // The bit alone is the fill, black too (TF0X); the colour without
        // it is none (TF0Y).
        assert_eq!(cell("1 1 512 0 0 0 0 1 0 0 0 0").fill, Some(0));
        assert_eq!(cell("1 2 0 0 0 6 0 1 0 255 0 0").fill, None);
        // Top and bottom only (the corpora's 4352), and a cell made here.
        assert_eq!(
            CellLines::from_word(0x1100),
            CellLines {
                left: 0,
                right: 0,
                top: 1,
                bottom: 1
            }
        );
        assert!(cell("").lines.is_none());
    }

    /// `1168:2dd2` against the stored rectangles of VM fixtures PI1 (Fit in
    /// frame, aspect kept: a 1,800 × 1,200 bitmap in a 1,108 × 544 picture
    /// area stores 816 × 544), PI5 (Original), PI6 (Percentage 100), and
    /// PI7 (Custom 1.00 × 0.25 in); past 22 in an axis is 22 in.
    #[test]
    fn pictures_are_sized_as_ami_pro_sizes_them_pi1_pi5_pi6_pi7() {
        let natural = Size {
            width: 1_800,
            height: 1_200,
        };
        let area = Size {
            width: 1_108,
            height: 544,
        };
        let mut scaling = PictureScaling::imported(area);
        assert_eq!(
            scaling.drawn_size(natural, area),
            Size {
                width: 816,
                height: 544
            }
        );
        scaling.keep_aspect = false;
        assert_eq!(scaling.drawn_size(natural, area), area);
        scaling.sizing = PictureSizing::Original;
        assert_eq!(scaling.drawn_size(natural, area), natural);
        scaling.sizing = PictureSizing::Percentage;
        assert_eq!(scaling.drawn_size(natural, area), natural);
        scaling.percent = 63;
        assert_eq!(
            scaling.drawn_size(natural, area),
            Size {
                width: 1_134,
                height: 756
            }
        );
        scaling.percent = 2_000;
        assert_eq!(scaling.drawn_size(natural, area).width, 31_680);
        scaling.sizing = PictureSizing::Custom;
        scaling.size = Size {
            width: 1_440,
            height: 360,
        };
        assert_eq!(scaling.drawn_size(natural, area), scaling.size);
        assert_eq!(scaling.flags(), 8);
        scaling.keep_aspect = true;
        assert_eq!(scaling.flags(), 24);
        // A tall area: the width decides.
        let tall = Size {
            width: 900,
            height: 5_000,
        };
        assert_eq!(
            PictureScaling::imported(tall).drawn_size(natural, tall),
            Size {
                width: 900,
                height: 600
            }
        );
    }

    /// The kept `[isd]` values give the scaling and pan (PI8: Custom 1.00 ×
    /// 0.25 in, moved 936 right and 144 down, written `936 65392`), and
    /// still say them until a change.
    #[test]
    fn kept_isd_values_give_the_scaling_and_pan_pi8() {
        let mut values = vec![
            ".X3", ".bmp", ".X3", "8", "1", "0", "0", "1440", "65176", "100",
        ];
        values.extend(["0", "2", "7", "9", ".bmp", "936", "65392", "0"]);
        let mut picture = Picture::new(
            PictureSource::Embedded(ObjectId(1)),
            ".bmp",
            Size {
                width: 1,
                height: 1,
            },
        );
        picture.legacy.push("isd", values.join("\n"));
        let (scaling, pan) = picture.kept_scaling().unwrap();
        assert_eq!(scaling.sizing, PictureSizing::Custom);
        assert!(!scaling.keep_aspect);
        assert_eq!(
            scaling.size,
            Size {
                width: 1_440,
                height: 360
            }
        );
        assert_eq!(pan, Offset { x: 936, y: 144 });
        assert!(!picture.kept_is_current());
        picture.scaling = scaling;
        picture.pan = pan;
        assert!(picture.kept_is_current());
        picture.pan.x += 72;
        assert!(!picture.kept_is_current());
        assert_eq!(picture.format(), PictureFormat::Bitmap);
        // A chart is a drawing whose driver is AmiChart.
        picture.legacy = LegacyData::default();
        picture.legacy.push(
            "isd",
            ".X3\n.sdw\n\n18\n1\n0\n0\n9\n9\n100\n0\n0\n.sct\n0\n0\n0",
        );
        picture.extension = ".sdw".into();
        assert_eq!(picture.format(), PictureFormat::Chart);
        assert!(PictureFormat::Chart.ami_pro_stores());
        assert!(!PictureFormat::from_extension(".PNG").ami_pro_stores());
    }

    /// `1530:0892` and `1530:0c22` against the VM fixtures' stored column
    /// edges: F01 (2,880 × 2,160, 18 %) 57, FC1 (3,600 × 1,440) 38, FC2
    /// (2,472 × 2,142) 57, and FC3 (50 × 33, no left or right margins) 10,
    /// the line's width; square frames keep their margins.
    #[test]
    fn text_insets_are_ami_pros_f01_fc1_fc2_fc3() {
        let style = FrameStyle {
            corners: 18,
            ..FrameStyle::default()
        };
        assert_eq!(style.text_insets(2_880, 2_160).left, 144 + 57);
        assert_eq!(style.text_insets(3_600, 1_440).right, 144 + 38);
        assert_eq!(style.text_insets(2_472, 2_142).left, 144 + 57);
        let mut tiny = style.clone();
        tiny.interior.margins.left = 0;
        tiny.interior.margins.right = 0;
        let insets = tiny.text_insets(50, 33);
        assert_eq!((insets.left, insets.right, insets.top), (10, 10, 144));
        let square = FrameStyle::default();
        assert_eq!(square.text_insets(50, 33), square.interior.margins);
        // At 100 % each axis keeps its own inset.
        let round = FrameStyle {
            corners: 100,
            ..FrameStyle::default()
        };
        let insets = round.text_insets(4_000, 2_000);
        assert_eq!(insets.left - 144, (4_000 - 20) * 300 / 2000);
        assert_eq!(insets.top - 144, (2_000 - 20) * 300 / 2000);
        assert_eq!(line_style_width(7), 240);
        assert_eq!(line_style_width(13), 0);
    }
}
