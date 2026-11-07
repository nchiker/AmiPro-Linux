//! Making and editing tables (`docs/Design/UI_SPEC.md` §32), by Ami Pro's
//! rules (`docs/research/TABLE_EDIT_NOTES.md`): Tools › Tables…, the Table
//! menu's structure and cell commands, Modify Table Layout, Column/Row
//! Size, Lines & Color, and the run's additions (TD4, TD7). Every command is
//! one step of the history, which Ami Pro's are not (TD10).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use amipro_document::{
    Border, CellContent, CellFormat, CellLeader, CellMargins, CellSpan, Colour, Document, FlowId,
    Frame, FrameContent, FrameId, FrameStyle, InlineObject, LayoutBox, LegacyData, LinePosition,
    Margins, Paragraph, Placement, Shadow, Sides, Size, StyleId, Table, TableCell, TableLayout,
    TableRows, TableSpacing, Twips, Wrap, line_style_width,
};

use amipro_document::formula;
use amipro_document::number::NumberFormat;

use crate::{Applied, ChangeReport, Command, EditError, TextPosition};

/// A rectangle of cells, rows and columns half-open.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellRange {
    pub rows: Range<usize>,
    pub columns: Range<usize>,
}

impl CellRange {
    pub fn cell(row: usize, column: usize) -> Self {
        Self {
            rows: row..row + 1,
            columns: column..column + 1,
        }
    }

    /// The rectangle with corners at two cells, in any order.
    pub fn spanning(a: (usize, usize), b: (usize, usize)) -> Self {
        Self {
            rows: a.0.min(b.0)..a.0.max(b.0) + 1,
            columns: a.1.min(b.1)..a.1.max(b.1) + 1,
        }
    }

    pub fn contains(&self, row: usize, column: usize) -> bool {
        self.rows.contains(&row) && self.columns.contains(&column)
    }

    pub fn is_single(&self) -> bool {
        self.rows.len() == 1 && self.columns.len() == 1
    }
}

/// Where a new table goes: at a position in the main text (its anchor, in
/// the caret's paragraph: TE1, TE20), or into an empty text frame, which
/// becomes the table's (TE19).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableTarget {
    Text(TextPosition),
    Frame(FrameId),
}

/// Tools › Tables… and its Layout…: the grid and Modify Table Layout's
/// values for a new table. The caller works out a fitting column width
/// (the column or frame less the inset and gutters, divided by the
/// columns) and one Table Text line's height, which need the page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewTable {
    pub columns: usize,
    pub rows: usize,
    pub column_width: Twips,
    pub row_height: Twips,
    pub cell_margins: CellMargins,
    pub automatic: bool,
    pub span_pages: bool,
    pub center: bool,
    pub honor_protection: bool,
    pub fixed_sizes: bool,
    /// Line around table, its style (1–12).
    pub line_around: Option<u16>,
}

impl NewTable {
    /// Ami Pro's limits (`CREATETBLPROC`): 1–250 columns, 1–4000 rows.
    pub const MAX_COLUMNS: usize = 250;
    pub const MAX_ROWS: usize = 4_000;

    /// Create Table's values without Layout…: the given grid, the columns
    /// fitting `width`, one line of `row_height`, Ami Pro's gutters, inset,
    /// and Automatic.
    pub fn fitting(columns: usize, rows: usize, width: Twips, row_height: Twips) -> Self {
        let margins = CellMargins::default();
        Self {
            columns,
            rows,
            column_width: fit_width(width, columns, margins.horizontal),
            row_height,
            cell_margins: margins,
            automatic: true,
            span_pages: false,
            center: false,
            honor_protection: false,
            fixed_sizes: false,
            line_around: None,
        }
    }
}

/// The column width that fills `width` with `columns` columns: the width
/// less the 43-twip inset and each column's gutter (at least 86), divided
/// (`1478:1054`; TE1: 3019 for three columns of 9360, Ami Pro 3017 after its
/// screen rounding).
pub fn fit_width(width: Twips, columns: usize, gutter: Twips) -> Twips {
    let columns = columns.max(1) as Twips;
    let room = width - TableSpacing::INSET;
    ((room - columns * gutter.max(86)) / columns).max(1)
}

/// Modify Table Layout's values (`TBLLAYOUTPROC`), and Line around table's
/// style for a page table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableDefaults {
    pub column_width: Twips,
    pub column_gutter: Twips,
    pub row_height: Twips,
    pub row_gutter: Twips,
    pub automatic: bool,
    pub span_pages: bool,
    pub center: bool,
    pub honor_protection: bool,
    pub fixed_sizes: bool,
    pub line_around: Option<u16>,
    /// The rows' heights as laid out, which they keep when Automatic goes
    /// off (TE17); empty to keep the stored ones.
    pub measured_rows: Vec<Twips>,
}

/// Column/Row Size: the sizes and gutters to give the rows and columns
/// (each `None` left as it is, Ami Pro's unchecked boxes).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TrackSizes {
    pub width: Option<Twips>,
    pub column_gutter: Option<Twips>,
    pub height: Option<Twips>,
    pub row_gutter: Option<Twips>,
}

/// Lines & Color's line positions: each `Some(true)` sets the style,
/// `Some(false)` removes the line, `None` keeps it (Ami Pro's grey box).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LinesChange {
    pub all: Option<bool>,
    pub outline: Option<bool>,
    pub left: Option<bool>,
    pub right: Option<bool>,
    pub top: Option<bool>,
    pub bottom: Option<bool>,
    pub style: u8,
}

/// What a cell command changes in each cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellChange {
    Lines(LinesChange),
    /// The lines' colour (TD8, the app's own).
    LineColour(Colour),
    Fill(Option<Colour>),
    Leader(CellLeader),
    Protect(bool),
}

/// Copied cells (`1240:2008`): rows of cells relative to the block's
/// top-left, each its paragraphs and look.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CellBlock {
    pub rows: Vec<Vec<CellData>>,
    /// The (row, column) of the block's top-left cell where it was copied
    /// from: a pasted formula shifts its relative names by the distance
    /// from there (`1420:04a8`, mode 3).
    pub origin: (usize, usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CellData {
    pub paragraphs: Vec<Paragraph>,
    pub format: CellFormat,
    /// The cell's formula as typed, if it has one.
    pub formula: Option<String>,
}

impl CellBlock {
    /// The cells of `range` in `table` of `document` (covered cells empty).
    pub fn copy(document: &Document, table: &Table, range: &CellRange) -> Self {
        let columns = table.columns.len();
        let rows = range
            .rows
            .clone()
            .map(|row| {
                range
                    .columns
                    .clone()
                    .map(|column| {
                        let cell = &table.cells[row * columns + column];
                        if cell.span.is_covered() {
                            return CellData::default();
                        }
                        let CellContent::Text(flow) = cell.content;
                        CellData {
                            paragraphs: document
                                .flow(flow)
                                .map(|text| {
                                    text.paragraphs
                                        .iter()
                                        .map(|paragraph| (**paragraph).clone())
                                        .collect()
                                })
                                .unwrap_or_default(),
                            format: cell.format,
                            formula: cell.formula.clone(),
                        }
                    })
                    .collect()
            })
            .collect();
        Self {
            rows,
            origin: (range.rows.start, range.columns.start),
        }
    }

    /// Tabbed text as cells: a row per line, a cell per tab-separated part,
    /// in `style` (Ami Pro: Body Text becomes Table Text, `13b0:3b6e`).
    pub fn from_text(document: &mut Document, text: &str, style: StyleId) -> Self {
        let rows = text
            .trim_end_matches(['\n', '\r'])
            .split('\n')
            .map(|line| {
                line.trim_end_matches('\r')
                    .split('\t')
                    .map(|part| CellData {
                        paragraphs: vec![document.new_paragraph(part, style)],
                        format: CellFormat::default(),
                        formula: None,
                    })
                    .collect()
            })
            .collect();
        Self {
            rows,
            origin: (0, 0),
        }
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    pub fn width(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }

    /// The cells as text: tabs between cells, a line per row, no line end
    /// after the last (Ami Pro's text form, `1240:2636`).
    pub fn text(&self) -> String {
        self.rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| {
                        cell.paragraphs
                            .iter()
                            .map(|paragraph| paragraph.text.replace('\u{FFFC}', ""))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Ready-made looks from Ami Pro's lines and fills (TD7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableStyle {
    /// Every cell's sides.
    Grid,
    /// A line around the cells, none inside.
    Outline,
    /// The first row shaded and a heading, a line under it.
    HeaderRow,
    /// Every other row shaded, lines around every cell.
    BandedRows,
    /// No lines, no fills.
    Plain,
}

/// One change to a table.
#[derive(Clone, Debug, PartialEq)]
pub enum TableEdit {
    /// `count` rows at the table's defaults before row `at`, with the lines
    /// and fill of row `like` (Insert Column/Row, `1438:0000`).
    InsertRows {
        at: usize,
        count: usize,
        like: Option<usize>,
    },
    /// `count` columns before column `at`. When the table would be wider
    /// than `room`, its columns narrow in proportion (TD4).
    InsertColumns {
        at: usize,
        count: usize,
        like: Option<usize>,
        room: Option<Twips>,
    },
    DeleteRows(Range<usize>),
    DeleteColumns(Range<usize>),
    SetDefaults(TableDefaults),
    SetSizes {
        rows: Range<usize>,
        columns: Range<usize>,
        sizes: TrackSizes,
    },
    /// The rows' and columns' own sizes and gutters removed: the table's
    /// defaults again.
    ResetSizes {
        rows: Range<usize>,
        columns: Range<usize>,
    },
    /// The rows' heights or columns' widths made equal within their total.
    Distribute {
        rows: Option<Range<usize>>,
        columns: Option<Range<usize>>,
    },
    Connect(CellRange),
    /// The joined cell that starts at (row, column) split again; its cells
    /// show their own text (TE9).
    Disconnect {
        row: usize,
        column: usize,
    },
    Headings {
        rows: Range<usize>,
        on: bool,
    },
    FormatCells {
        cells: CellRange,
        change: CellChange,
    },
    /// The cells' text and pictures removed, rows and columns kept (Delete
    /// with cells selected, `1480:01b6`).
    ClearCells(CellRange),
    /// Rows moved (or copied) to stand before row `to` of the table as it
    /// was (drag and drop, Alt+Up/Down).
    MoveRows {
        rows: Range<usize>,
        to: usize,
        copy: bool,
    },
    MoveColumns {
        columns: Range<usize>,
        to: usize,
        copy: bool,
    },
    ApplyStyle(TableStyle),
    /// The rows below the table's leading heading rows sorted by their text
    /// in `column` (numbers by value, other text without regard to case,
    /// equal rows keeping their order): Table › Sort Ascending and Sort
    /// Descending (MR1; Ami Pro's Tools › Sort is not built).
    SortRows {
        column: usize,
        descending: bool,
    },
    /// Tools › Sort… in a table (TD.8, `sort`): whole rows in records, the
    /// `rows` given or (`None`) the table's rows below its headings.
    SortRecords {
        rows: Option<Range<usize>>,
        settings: crate::sort::SortSettings,
    },
    /// Copied cells pasted over the cells from `at` (`13b0:4065`): over a
    /// selected `area` that is a whole multiple of the block, the block
    /// repeats; columns past the last are dropped; rows are added to a
    /// table in the text when `add_rows`, else the paste stops at the last.
    PasteCells {
        at: (usize, usize),
        block: CellBlock,
        area: Option<CellRange>,
        add_rows: bool,
    },
}

/// Ami Pro's default inset; also the least the insets shrink back to.
const INSET: Twips = TableSpacing::INSET;

fn table_of(document: &Document, frame: FrameId) -> Result<&Table, EditError> {
    match document.frame(frame).map(|frame| &frame.content) {
        Some(FrameContent::Table(table)) => Ok(table),
        Some(_) => Err("the frame holds no table"),
        None => Err("frame does not exist"),
    }
}

/// The new root with `table` in `frame`, the frame's size following its
/// grid for a table in the text (Ami Pro: the inset and each size and
/// gutter, TE1–TE7).
fn put(next: &mut Document, frame: FrameId, table: Table) {
    let size = grid_size(&table);
    if let Some(slot) = next.frame_mut(frame) {
        let margins = slot.style.interior.margins;
        if let Placement::Inline { size: stored, .. }
        | Placement::BelowParagraph { size: stored, .. } = &mut slot.placement
        {
            *stored = Size {
                width: size.width + margins.left + margins.right,
                height: size.height + margins.top + margins.bottom,
            };
        }
        slot.content = FrameContent::Table(table);
    }
}

/// A table's grid: its insets and each row's and column's size and gutter.
pub fn grid_size(table: &Table) -> Size {
    let width = table.spacing.left
        + (0..table.columns.len())
            .map(|column| table.columns[column] + table.column_gutter(column))
            .sum::<Twips>();
    let height = table.spacing.top
        + (0..table.rows.len())
            .map(|row| table.rows[row] + table.row_gutter(row))
            .sum::<Twips>();
    Size { width, height }
}

fn new_cell(next: &mut Document, style: StyleId, report: &mut ChangeReport) -> TableCell {
    let flow = next.create_flow(style);
    report.added_flows.push(flow);
    TableCell {
        content: CellContent::Text(flow),
        span: CellSpan::SINGLE,
        frame: None,
        format: CellFormat::default(),
        formula: None,
        legacy: LegacyData::default(),
    }
}

fn table_text(next: &mut Document, report: &mut ChangeReport) -> StyleId {
    let styles = next.styles().len();
    let style = next.table_text_style();
    report.styles |= next.styles().len() != styles;
    style
}

/// Shifts the keys of a set at or past `at` by `delta`, dropping those in
/// `removed`.
fn shift_set(
    set: &BTreeSet<usize>,
    at: usize,
    delta: isize,
    removed: &Range<usize>,
) -> BTreeSet<usize> {
    set.iter()
        .filter(|index| !removed.contains(index))
        .map(|index| shifted(*index, at, delta))
        .collect()
}

fn shift_map<V: Clone>(
    map: &BTreeMap<usize, V>,
    at: usize,
    delta: isize,
    removed: &Range<usize>,
) -> BTreeMap<usize, V> {
    map.iter()
        .filter(|(index, _)| !removed.contains(index))
        .map(|(index, value)| (shifted(*index, at, delta), value.clone()))
        .collect()
}

fn shifted(index: usize, at: usize, delta: isize) -> usize {
    if index >= at {
        (index as isize + delta) as usize
    } else {
        index
    }
}

/// Each joined block: its top-left cell's (row, column) and span.
fn blocks(table: &Table) -> Vec<(usize, usize, CellSpan)> {
    let columns = table.columns.len().max(1);
    table
        .cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| !cell.span.is_covered() && cell.span != CellSpan::SINGLE)
        .map(|(index, cell)| (index / columns, index % columns, cell.span))
        .collect()
}

/// The block that covers (row, column): its top-left and span.
pub fn block_at(table: &Table, row: usize, column: usize) -> (usize, usize, CellSpan) {
    blocks(table)
        .into_iter()
        .find(|(top, left, span)| {
            (*top..top + usize::from(span.rows)).contains(&row)
                && (*left..left + usize::from(span.columns)).contains(&column)
        })
        .unwrap_or((row, column, CellSpan::SINGLE))
}

/// A range grown until no joined block crosses its edge (`14a0:1052`).
pub fn expanded(table: &Table, range: &CellRange) -> CellRange {
    let mut range = range.clone();
    loop {
        let mut grown = range.clone();
        for (top, left, span) in blocks(table) {
            let rows = top..top + usize::from(span.rows);
            let columns = left..left + usize::from(span.columns);
            let meets = rows.start < range.rows.end
                && range.rows.start < rows.end
                && columns.start < range.columns.end
                && range.columns.start < columns.end;
            if meets {
                grown.rows = grown.rows.start.min(rows.start)..grown.rows.end.max(rows.end);
                grown.columns =
                    grown.columns.start.min(columns.start)..grown.columns.end.max(columns.end);
            }
        }
        if grown == range {
            return range;
        }
        range = grown;
    }
}

fn protected_at(table: &Table, row: usize, column: usize) -> bool {
    let columns = table.columns.len();
    let (top, left, _) = block_at(table, row, column);
    table
        .cells
        .get(top * columns + left)
        .is_some_and(|cell| cell.format.protected)
}

/// Whether Honor protection keeps the caret and commands out of a cell
/// (`1460:04a6`).
pub fn keeps_out(table: &Table, row: usize, column: usize) -> bool {
    table.layout.honor_protection && protected_at(table, row, column)
}

fn check_range(table: &Table, range: &CellRange) -> Result<(), EditError> {
    if range.rows.is_empty()
        || range.columns.is_empty()
        || range.rows.end > table.rows.len()
        || range.columns.end > table.columns.len()
    {
        return Err("cells outside the table");
    }
    Ok(())
}

/// Makes a table (`UI_SPEC.md` §32): in the text, an anchored table frame
/// whose anchor goes at the caret as Ami Pro's does (flags `0x1800C4`, the
/// default frame without shadow or margins, Line around as its line), or in
/// an empty text frame, which keeps its rectangle and look (TE19). The caret
/// goes to the first cell.
pub(crate) fn insert_table(
    document: &Document,
    target: TableTarget,
    spec: &NewTable,
) -> Result<Applied, EditError> {
    if !(1..=NewTable::MAX_COLUMNS).contains(&spec.columns)
        || !(1..=NewTable::MAX_ROWS).contains(&spec.rows)
    {
        return Err("a table has 1 to 250 columns and 1 to 4000 rows");
    }
    if spec.column_width < 1 || spec.row_height < 1 {
        return Err("a table's columns and rows have a size");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let style = table_text(&mut next, &mut report);
    let cells = (0..spec.columns * spec.rows)
        .map(|_| new_cell(&mut next, style, &mut report))
        .collect::<Vec<_>>();
    let CellContent::Text(first_flow) = cells[0].content;
    let table = Table {
        columns: vec![spec.column_width; spec.columns],
        rows: vec![spec.row_height; spec.rows],
        cell_margins: spec.cell_margins,
        spacing: TableSpacing::default(),
        row_options: TableRows {
            automatic: spec.automatic,
            span_pages: spec.span_pages && spec.automatic,
            ..TableRows::default()
        },
        layout: TableLayout {
            row_height: spec.row_height,
            column_width: spec.column_width,
            center: spec.center,
            honor_protection: spec.honor_protection,
            fixed_sizes: spec.fixed_sizes,
            ..TableLayout::default()
        },
        read_rows: BTreeMap::new(),
        cells,
        legacy: LegacyData::default(),
    };
    let caret = TextPosition {
        flow: first_flow,
        paragraph: 0,
        byte: 0,
    };
    match target {
        TableTarget::Frame(frame) => {
            let holder = document.frame(frame).ok_or("frame does not exist")?;
            let FrameContent::Text(flow) = holder.content else {
                return Err("a table goes into an empty text frame");
            };
            let text = document.flow(flow).ok_or("frame text does not exist")?;
            if text.paragraphs.len() > 1
                || text
                    .paragraphs
                    .first()
                    .is_some_and(|paragraph| !paragraph.text.is_empty())
            {
                return Err("a table goes into an empty text frame");
            }
            // Rows span pages and Line around do not apply to a frame table.
            let mut table = table;
            table.row_options.span_pages = false;
            if next.remove_flow(flow).is_some() {
                report.removed_flows.push(flow);
            }
            if let Some(slot) = next.frame_mut(frame) {
                slot.content = FrameContent::Table(table);
                slot.legacy.values.retain(|value| value.key != "txt");
            }
            report.frames.insert(frame);
            Ok(Applied {
                document: next,
                caret: Some(caret),
                report,
            })
        }
        TableTarget::Text(at) => {
            let mut style = FrameStyle {
                wrap: Wrap::Around,
                opaque: true,
                border: Border {
                    sides: Sides::ALL,
                    style: spec.line_around.unwrap_or(1),
                    colour: amipro_document::BLACK,
                    position: LinePosition::Middle,
                },
                shadow: Shadow {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                    ..Shadow::default()
                },
                corners: 0,
                interior: LayoutBox::single_column(Margins::uniform(0)),
                ..FrameStyle::default()
            };
            set_line_around(&mut style, spec.line_around);
            let size = grid_size(&table);
            let margin = style.interior.margins.left;
            let placement = Placement::Inline {
                y_offset: 0,
                size: Size {
                    width: size.width + 2 * margin,
                    height: size.height + 2 * margin,
                },
            };
            let id = next.allocate_frame_id();
            // Ami Pro marks a table made in the text (`0x100000`, TE1's
            // `0x1800C4`; With para above tables lack it), a bit the model
            // keeps as read.
            let mut legacy = LegacyData::default();
            legacy.push("frame.flags.unknown", "0x100000");
            next.insert_frame(Frame {
                id,
                placement,
                style,
                group: None,
                content: FrameContent::Table(table),
                legacy,
            });
            next.frame_stack_mut().push(id);
            report.frames.insert(id);
            let mut applied = crate::frames::attach_anchor(next, at, id, report)?;
            applied.caret = Some(caret);
            Ok(applied)
        }
    }
}

/// Line around table: the frame's own line on all sides in `style`, its
/// margins the line's width (`14b0:133e`; VM TE14), or no line and no
/// margins.
fn set_line_around(style: &mut FrameStyle, line: Option<u16>) {
    match line {
        Some(line) => {
            style.border.sides = Sides::ALL;
            style.border.style = line;
            style.interior.margins = Margins::uniform(line_style_width(line));
        }
        None => {
            style.border.sides = Sides::NONE;
            style.interior.margins = Margins::uniform(0);
        }
    }
}

/// Whether a table frame has Line around table.
pub fn line_around(frame: &Frame) -> Option<u16> {
    (frame.style.border.sides != Sides::NONE).then_some(frame.style.border.style)
}

pub(crate) fn edit_table(
    document: &Document,
    frame: FrameId,
    edit: &TableEdit,
) -> Result<Applied, EditError> {
    let table = table_of(document, frame)?.clone();
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    report.frames.insert(frame);
    let mut caret = None;
    // Whether the results of the formulas are worked out again after the
    // edit (TN5), and whether those that became REF keep their old result
    // (the rewrite of rows and columns does not touch it, VM TDF3).
    let mut recalculate: Option<bool> = None;
    let table = match edit {
        TableEdit::InsertRows { at, count, like } => {
            let mut table = insert_rows(&mut next, table, *at, *count, *like, &mut report)?;
            rewrite_formulas(
                &mut table,
                formula::Change::InsertRows {
                    at: *at,
                    count: *count,
                },
            );
            recalculate = Some(true);
            table
        }
        TableEdit::InsertColumns {
            at,
            count,
            like,
            room,
        } => {
            let mut table =
                insert_columns(&mut next, table, *at, *count, *like, *room, &mut report)?;
            rewrite_formulas(
                &mut table,
                formula::Change::InsertColumns {
                    at: *at,
                    count: *count,
                },
            );
            recalculate = Some(true);
            table
        }
        TableEdit::DeleteRows(rows) => {
            if rows.is_empty() || rows.end > table.rows.len() {
                return Err("rows outside the table");
            }
            if rows.len() == table.rows.len() {
                return crate::frames::delete_frame(document, frame);
            }
            let mut table = delete_rows(&mut next, table, rows.clone(), &mut report);
            rewrite_formulas(
                &mut table,
                formula::Change::DeleteRows {
                    at: rows.start,
                    count: rows.len(),
                },
            );
            recalculate = Some(true);
            let row = rows.start.min(table.rows.len() - 1);
            caret = cell_caret(&table, row, 0);
            table
        }
        TableEdit::DeleteColumns(columns) => {
            if columns.is_empty() || columns.end > table.columns.len() {
                return Err("columns outside the table");
            }
            if columns.len() == table.columns.len() {
                return crate::frames::delete_frame(document, frame);
            }
            let mut table = delete_columns(&mut next, table, columns.clone(), &mut report);
            rewrite_formulas(
                &mut table,
                formula::Change::DeleteColumns {
                    at: columns.start,
                    count: columns.len(),
                },
            );
            recalculate = Some(true);
            let column = columns.start.min(table.columns.len() - 1);
            caret = cell_caret(&table, 0, column);
            table
        }
        TableEdit::SetDefaults(defaults) => {
            let table = set_defaults(table, defaults)?;
            if let Some(slot) = next.frame_mut(frame)
                && matches!(
                    slot.placement,
                    Placement::Inline { .. } | Placement::BelowParagraph { .. }
                )
            {
                set_line_around(&mut slot.style, defaults.line_around);
            }
            table
        }
        TableEdit::SetSizes {
            rows,
            columns,
            sizes,
        } => set_sizes(table, rows.clone(), columns.clone(), *sizes)?,
        TableEdit::ResetSizes { rows, columns } => {
            let mut table = table;
            let count = table.rows.len();
            for row in rows.clone().filter(|row| *row < count) {
                table.layout.own_heights.remove(&row);
                table.spacing.row_gutters.remove(&row);
                if !table.row_options.automatic {
                    table.rows[row] = table.layout.row_height;
                }
            }
            let count = table.columns.len();
            for column in columns.clone().filter(|column| *column < count) {
                table.layout.own_widths.remove(&column);
                table.spacing.column_gutters.remove(&column);
                table.columns[column] = table.layout.column_width;
            }
            table
        }
        TableEdit::Distribute { rows, columns } => {
            distribute(table, rows.clone(), columns.clone())?
        }
        TableEdit::Connect(range) => connect(table, range)?,
        TableEdit::Disconnect { row, column } => {
            let mut table = table;
            let count = table.columns.len();
            let index = row * count + column;
            let span = table.cells.get(index).ok_or("cell outside the table")?.span;
            if span.is_covered() || span == CellSpan::SINGLE {
                return Err("the cell is not joined");
            }
            for inner in *row..row + usize::from(span.rows) {
                for other in *column..column + usize::from(span.columns) {
                    table.cells[inner * count + other].span = CellSpan::SINGLE;
                }
            }
            table
        }
        TableEdit::Headings { rows, on } => {
            let mut table = table;
            if rows.end > table.rows.len() {
                return Err("rows outside the table");
            }
            // Whole joined blocks (`1458:0e56`).
            let range = expanded(
                &table,
                &CellRange {
                    rows: rows.clone(),
                    columns: 0..table.columns.len(),
                },
            );
            for row in range.rows {
                if *on {
                    table.row_options.headings.insert(row);
                } else {
                    table.row_options.headings.remove(&row);
                }
            }
            table
        }
        TableEdit::FormatCells { cells, change } => format_cells(table, cells, *change)?,
        TableEdit::ClearCells(range) => {
            recalculate = Some(false);
            clear_cells(&mut next, table, range, &mut report)?
        }
        TableEdit::MoveRows { rows, to, copy } => {
            recalculate = Some(false);
            move_rows(&mut next, table, rows.clone(), *to, *copy, &mut report)?
        }
        TableEdit::MoveColumns { columns, to, copy } => {
            recalculate = Some(false);
            move_columns(&mut next, table, columns.clone(), *to, *copy, &mut report)?
        }
        TableEdit::ApplyStyle(style) => apply_style(table, *style),
        TableEdit::SortRows { column, descending } => {
            recalculate = Some(false);
            sort_rows(&mut next, table, *column, *descending, &mut report)?
        }
        TableEdit::SortRecords { rows, settings } => {
            recalculate = Some(false);
            sort_records(&mut next, table, rows.clone(), settings, &mut report)?
        }
        TableEdit::PasteCells {
            at,
            block,
            area,
            add_rows,
        } => {
            recalculate = Some(false);
            paste_cells(
                &mut next,
                table,
                *at,
                block,
                area.as_ref(),
                *add_rows,
                &mut report,
            )?
        }
    };
    if let Some(keep_ref) = recalculate {
        recalculate_cells(&mut next, &table, &Scope::All, keep_ref, &mut report)?;
    }
    table.validate_spans()?;
    table.validate_spacing()?;
    put(&mut next, frame, table);
    Ok(Applied {
        document: next,
        caret,
        report,
    })
}

/// The start of the text of the cell that shows at (row, column).
pub fn cell_caret(table: &Table, row: usize, column: usize) -> Option<TextPosition> {
    let (top, left, _) = block_at(table, row, column);
    let CellContent::Text(flow) = table.cells.get(top * table.columns.len() + left)?.content;
    Some(TextPosition {
        flow,
        paragraph: 0,
        byte: 0,
    })
}

/// The look a new cell takes from the cell beside it: its lines and fill
/// (Ami Pro: "the lines and color specified for the selected cells"; its
/// leader and protection are not copied).
fn like_format(format: &CellFormat) -> CellFormat {
    CellFormat {
        lines: format.lines,
        line_colour: format.line_colour,
        fill: format.fill,
        ..CellFormat::default()
    }
}

fn insert_rows(
    next: &mut Document,
    mut table: Table,
    at: usize,
    count: usize,
    like: Option<usize>,
    report: &mut ChangeReport,
) -> Result<Table, EditError> {
    if count == 0 || at > table.rows.len() {
        return Err("rows outside the table");
    }
    if table.rows.len() + count > NewTable::MAX_ROWS {
        return Err("a table has at most 4000 rows");
    }
    let columns = table.columns.len();
    let style = table_text(next, report);
    let none = 0..0;
    // A block the new rows fall inside grows by them; its new cells are
    // covered (`1438:1636`).
    let spans: Vec<(usize, usize, CellSpan)> = blocks(&table);
    let mut new_cells = Vec::with_capacity(count * columns);
    for _ in 0..count {
        for column in 0..columns {
            let mut cell = new_cell(next, style, report);
            if let Some(like) = like.and_then(|row| table.cells.get(row * columns + column)) {
                cell.format = like_format(&like.format);
            }
            new_cells.push(cell);
        }
    }
    let tail = table.cells.split_off(at * columns);
    table.cells.extend(new_cells);
    table.cells.extend(tail);
    for (top, left, span) in spans {
        let bottom = top + usize::from(span.rows);
        if top < at && at < bottom {
            table.cells[top * columns + left].span.rows += count as u16;
            for row in at..at + count {
                for column in left..left + usize::from(span.columns) {
                    table.cells[row * columns + column].span = CellSpan::COVERED;
                }
            }
        }
    }
    let height = table.layout.row_height;
    table
        .rows
        .splice(at..at, std::iter::repeat_n(height, count));
    let delta = count as isize;
    table.spacing.row_gutters = shift_map(&table.spacing.row_gutters, at, delta, &none);
    table.layout.own_heights = shift_set(&table.layout.own_heights, at, delta, &none);
    table.row_options.headings = shift_set(&table.row_options.headings, at, delta, &none);
    table.row_options.page_breaks = shift_set(&table.row_options.page_breaks, at, delta, &none);
    table.read_rows = shift_map(&table.read_rows, at, delta, &none);
    Ok(table)
}

fn insert_columns(
    next: &mut Document,
    mut table: Table,
    at: usize,
    count: usize,
    like: Option<usize>,
    room: Option<Twips>,
    report: &mut ChangeReport,
) -> Result<Table, EditError> {
    if count == 0 || at > table.columns.len() {
        return Err("columns outside the table");
    }
    if table.columns.len() + count > NewTable::MAX_COLUMNS {
        return Err("a table has at most 250 columns");
    }
    let old = table.columns.len();
    let new = old + count;
    let style = table_text(next, report);
    let spans = blocks(&table);
    let mut cells = Vec::with_capacity(table.rows.len() * new);
    let mut old_cells = std::mem::take(&mut table.cells).into_iter();
    for _ in 0..table.rows.len() {
        let row_cells: Vec<TableCell> = old_cells.by_ref().take(old).collect();
        let mut added = Vec::with_capacity(count);
        for _ in 0..count {
            let mut cell = new_cell(next, style, report);
            if let Some(like) = like.and_then(|column| row_cells.get(column)) {
                cell.format = like_format(&like.format);
            }
            added.push(cell);
        }
        let mut row_cells = row_cells;
        let tail = row_cells.split_off(at);
        cells.extend(row_cells);
        cells.extend(added);
        cells.extend(tail);
    }
    table.cells = cells;
    for (top, left, span) in spans {
        let right = left + usize::from(span.columns);
        if left < at && at < right {
            table.cells[top * new + left].span.columns += count as u16;
            for row in top..top + usize::from(span.rows) {
                for column in at..at + count {
                    table.cells[row * new + column].span = CellSpan::COVERED;
                }
            }
        }
    }
    let width = table.layout.column_width;
    table
        .columns
        .splice(at..at, std::iter::repeat_n(width, count));
    let none = 0..0;
    let delta = count as isize;
    table.spacing.column_gutters = shift_map(&table.spacing.column_gutters, at, delta, &none);
    table.layout.own_widths = shift_set(&table.layout.own_widths, at, delta, &none);
    // TD4: a table wider than its room keeps its width, its columns
    // narrowed in proportion (Ami Pro refuses the column instead).
    if let Some(room) = room {
        let width = grid_size(&table).width;
        if width > room {
            let gutters: Twips = (0..table.columns.len())
                .map(|column| table.column_gutter(column))
                .sum();
            let texts: Twips = table.columns.iter().sum();
            let wanted = (room - table.spacing.left - gutters).max(table.columns.len() as Twips);
            let scale = wanted as f64 / texts.max(1) as f64;
            for size in &mut table.columns {
                *size = ((*size as f64 * scale).floor() as Twips).max(1);
            }
            table.layout.column_width =
                ((table.layout.column_width as f64 * scale).floor() as Twips).max(1);
            // Every column now has the size it was given.
            let default = table.layout.column_width;
            table.layout.own_widths = (0..table.columns.len())
                .filter(|column| table.columns[*column] != default)
                .collect();
        }
    }
    Ok(table)
}

/// Removes rows; a joined block loses them, and when its first row goes
/// the first row left takes its contents (`1430:0f40`).
fn delete_rows(
    next: &mut Document,
    mut table: Table,
    rows: Range<usize>,
    report: &mut ChangeReport,
) -> Table {
    let columns = table.columns.len();
    for (top, left, span) in blocks(&table) {
        let bottom = top + usize::from(span.rows);
        let overlap = rows.start.max(top)..rows.end.min(bottom);
        if overlap.is_empty() {
            continue;
        }
        let left_rows = span.rows as usize - overlap.len();
        let owner = top * columns + left;
        if rows.contains(&top) && left_rows > 0 {
            // The new top-left: the first row after the deleted ones.
            let new_top = rows.end;
            let target = new_top * columns + left;
            table.cells.swap(owner, target);
            table.cells[target].span = CellSpan {
                rows: left_rows as u16,
                columns: span.columns,
            };
            table.cells[owner].span = CellSpan::COVERED;
        } else if left_rows > 0 {
            table.cells[owner].span.rows = left_rows as u16;
        }
    }
    // Blocks reduced to one cell are ordinary cells again; a block whose
    // rows all go goes with them.
    let removed: Vec<TableCell> = table
        .cells
        .drain(rows.start * columns..rows.end * columns)
        .collect();
    for cell in removed {
        drop_cell(next, cell, report);
    }
    table.rows.drain(rows.clone());
    let delta = -(rows.len() as isize);
    let at = rows.end;
    table.spacing.row_gutters = shift_map(&table.spacing.row_gutters, at, delta, &rows);
    table.layout.own_heights = shift_set(&table.layout.own_heights, at, delta, &rows);
    table.row_options.headings = shift_set(&table.row_options.headings, at, delta, &rows);
    table.row_options.page_breaks = shift_set(&table.row_options.page_breaks, at, delta, &rows);
    table.read_rows = shift_map(&table.read_rows, at, delta, &rows);
    tidy_spans(&mut table);
    table
}

fn delete_columns(
    next: &mut Document,
    mut table: Table,
    columns: Range<usize>,
    report: &mut ChangeReport,
) -> Table {
    let count = table.columns.len();
    for (top, left, span) in blocks(&table) {
        let right = left + usize::from(span.columns);
        let overlap = columns.start.max(left)..columns.end.min(right);
        if overlap.is_empty() {
            continue;
        }
        let left_columns = span.columns as usize - overlap.len();
        let owner = top * count + left;
        if columns.contains(&left) && left_columns > 0 {
            let target = top * count + columns.end;
            table.cells.swap(owner, target);
            table.cells[target].span = CellSpan {
                rows: span.rows,
                columns: left_columns as u16,
            };
            table.cells[owner].span = CellSpan::COVERED;
        } else if left_columns > 0 {
            table.cells[owner].span.columns = left_columns as u16;
        }
    }
    let mut kept = Vec::with_capacity(table.cells.len());
    for (index, cell) in std::mem::take(&mut table.cells).into_iter().enumerate() {
        if columns.contains(&(index % count)) {
            drop_cell(next, cell, report);
        } else {
            kept.push(cell);
        }
    }
    table.cells = kept;
    table.columns.drain(columns.clone());
    let delta = -(columns.len() as isize);
    let at = columns.end;
    table.spacing.column_gutters = shift_map(&table.spacing.column_gutters, at, delta, &columns);
    table.layout.own_widths = shift_set(&table.layout.own_widths, at, delta, &columns);
    tidy_spans(&mut table);
    table
}

/// After rows or columns go: a block of one cell is an ordinary cell, and
/// a covered cell no block covers shows its own text again.
fn tidy_spans(table: &mut Table) {
    let columns = table.columns.len();
    let mut covered = vec![false; table.cells.len()];
    for (top, left, span) in blocks(table) {
        let rows = (top + usize::from(span.rows)).min(table.rows.len());
        let right = (left + usize::from(span.columns)).min(columns);
        let span = CellSpan {
            rows: (rows - top) as u16,
            columns: (right - left) as u16,
        };
        table.cells[top * columns + left].span = if span.rows <= 1 && span.columns <= 1 {
            CellSpan::SINGLE
        } else {
            span
        };
        for row in top..rows {
            for column in left..right {
                if (row, column) != (top, left) && span != CellSpan::SINGLE {
                    covered[row * columns + column] = true;
                }
            }
        }
    }
    for (cell, covered) in table.cells.iter_mut().zip(covered) {
        if cell.span.is_covered() && !covered {
            cell.span = CellSpan::SINGLE;
        }
    }
}

fn drop_cell(next: &mut Document, cell: TableCell, report: &mut ChangeReport) {
    if let Some(frame) = cell.frame {
        crate::frames::delete_frame_tree(next, frame, report);
    }
    let CellContent::Text(flow) = cell.content;
    if next.remove_flow(flow).is_some() {
        report.removed_flows.push(flow);
    }
}

fn set_defaults(mut table: Table, defaults: &TableDefaults) -> Result<Table, EditError> {
    if defaults.column_width < 1 || defaults.row_height < 10 {
        return Err("a column is at least a twip and a row 10 twips");
    }
    if defaults.column_gutter < 0 || defaults.row_gutter < 0 {
        return Err("gutters are not negative");
    }
    // Rows and columns without their own size take the new defaults
    // (TE16); Automatic off keeps each row as laid out (TE17).
    if table.row_options.automatic && !defaults.automatic {
        for (row, height) in defaults.measured_rows.iter().enumerate() {
            if let Some(slot) = table.rows.get_mut(row) {
                *slot = *height;
                table.layout.own_heights.insert(row);
            }
        }
    }
    for column in 0..table.columns.len() {
        if !table.layout.own_widths.contains(&column) {
            table.columns[column] = defaults.column_width;
        }
    }
    for row in 0..table.rows.len() {
        if !table.layout.own_heights.contains(&row) {
            table.rows[row] = defaults.row_height;
        }
    }
    table.layout.column_width = defaults.column_width;
    table.layout.row_height = defaults.row_height;
    table.cell_margins = CellMargins {
        horizontal: defaults.column_gutter,
        vertical: defaults.row_gutter,
    };
    table.row_options.automatic = defaults.automatic;
    // Automatic rows fit their text: none keeps a height of its own.
    if defaults.automatic {
        table.layout.own_heights.clear();
    }
    // Rows span pages only with Automatic (`TBLLAYOUTPROC`).
    table.row_options.span_pages = defaults.span_pages && defaults.automatic;
    table.layout.center = defaults.center;
    table.layout.honor_protection = defaults.honor_protection;
    table.layout.fixed_sizes = defaults.fixed_sizes;
    Ok(table)
}

fn set_sizes(
    mut table: Table,
    rows: Range<usize>,
    columns: Range<usize>,
    sizes: TrackSizes,
) -> Result<Table, EditError> {
    if rows.end > table.rows.len() || columns.end > table.columns.len() {
        return Err("rows or columns outside the table");
    }
    if [sizes.width, sizes.height]
        .into_iter()
        .flatten()
        .any(|size| size < 1)
        || [sizes.column_gutter, sizes.row_gutter]
            .into_iter()
            .flatten()
            .any(|gutter| gutter < 0)
    {
        return Err("sizes are positive and gutters not negative");
    }
    for column in columns {
        if let Some(width) = sizes.width {
            table.columns[column] = width;
            table.layout.own_widths.insert(column);
        }
        if let Some(gutter) = sizes.column_gutter {
            table.spacing.column_gutters.insert(column, gutter);
        }
    }
    for row in rows {
        // Ami Pro's Height is off while Automatic is on.
        if let Some(height) = sizes.height.filter(|_| !table.row_options.automatic) {
            table.rows[row] = height;
            table.layout.own_heights.insert(row);
        }
        if let Some(gutter) = sizes.row_gutter {
            table.spacing.row_gutters.insert(row, gutter);
        }
    }
    Ok(table)
}

fn distribute(
    mut table: Table,
    rows: Option<Range<usize>>,
    columns: Option<Range<usize>>,
) -> Result<Table, EditError> {
    if let Some(columns) = columns {
        if columns.end > table.columns.len() || columns.is_empty() {
            return Err("columns outside the table");
        }
        let total: Twips = table.columns[columns.clone()].iter().sum();
        let count = columns.len() as Twips;
        for (offset, column) in columns.clone().enumerate() {
            // The remainder to the first columns, so the total stays.
            let extra = Twips::from((offset as Twips) < total % count);
            table.columns[column] = total / count + extra;
            table.layout.own_widths.insert(column);
        }
    }
    if let Some(rows) = rows {
        if rows.end > table.rows.len() || rows.is_empty() {
            return Err("rows outside the table");
        }
        if table.row_options.automatic {
            return Err("rows of an Automatic table fit their text");
        }
        let total: Twips = table.rows[rows.clone()].iter().sum();
        let count = rows.len() as Twips;
        for (offset, row) in rows.clone().enumerate() {
            let extra = Twips::from((offset as Twips) < total % count);
            table.rows[row] = total / count + extra;
            table.layout.own_heights.insert(row);
        }
    }
    Ok(table)
}

/// Connect Cells (`1458:04df`): the range grown to whole blocks, the blocks
/// inside it dissolved, one block made; the hidden cells keep their text
/// (TE8).
fn connect(mut table: Table, range: &CellRange) -> Result<Table, EditError> {
    check_range(&table, range)?;
    let range = expanded(&table, range);
    if range.is_single() {
        return Err("Connect Cells needs more than one cell");
    }
    let columns = table.columns.len();
    if range.rows.clone().any(|row| {
        range
            .columns
            .clone()
            .any(|column| keeps_out(&table, row, column))
    }) {
        return Err("a protected cell cannot be connected");
    }
    let headings = range
        .rows
        .clone()
        .filter(|row| table.row_options.headings.contains(row))
        .count();
    if headings != 0 && headings != range.rows.len() {
        return Err("heading rows connect only with heading rows");
    }
    for row in range.rows.clone() {
        for column in range.columns.clone() {
            table.cells[row * columns + column].span = CellSpan::COVERED;
        }
    }
    table.cells[range.rows.start * columns + range.columns.start].span = CellSpan {
        rows: range.rows.len() as u16,
        columns: range.columns.len() as u16,
    };
    Ok(table)
}

/// Lines & Color, Leaders, and Protect Cells over a range of cells.
fn format_cells(
    mut table: Table,
    range: &CellRange,
    change: CellChange,
) -> Result<Table, EditError> {
    check_range(&table, range)?;
    let columns = table.columns.len();
    // Protect works whatever Honor protection says; the others skip
    // protected cells under it (`1460:0000`, `13e8:2790`, `14b0:067c`).
    let honoured = !matches!(change, CellChange::Protect(_));
    let mut owners = BTreeSet::new();
    for row in range.rows.clone() {
        for column in range.columns.clone() {
            let (top, left, _) = block_at(&table, row, column);
            owners.insert((top, left));
        }
    }
    let skip = |table: &Table, row: usize, column: usize| honoured && keeps_out(table, row, column);
    if let CellChange::Lines(lines) = change {
        return Ok(set_lines(table, range, lines, &owners, honoured));
    }
    for (row, column) in owners {
        if skip(&table, row, column) {
            continue;
        }
        let format = &mut table.cells[row * columns + column].format;
        match change {
            CellChange::Lines(_) => unreachable!(),
            CellChange::LineColour(colour) => format.line_colour = colour,
            CellChange::Fill(fill) => format.fill = fill,
            CellChange::Leader(leader) => format.leader = leader,
            CellChange::Protect(on) => format.protected = on,
        }
    }
    Ok(table)
}

/// Lines & Color's line positions over a range (`14b0:067c`): each side of
/// each cell set, removed, or kept, Outline on the range's edge; a joined
/// block's lines in its top-left cell; a line wider than its gutter or
/// inset widening it (flag 4), the insets shrinking back towards 43
/// (`14b0:09c8`, `14b0:13a1`).
fn set_lines(
    mut table: Table,
    range: &CellRange,
    change: LinesChange,
    owners: &BTreeSet<(usize, usize)>,
    honoured: bool,
) -> Table {
    let columns = table.columns.len();
    let style = change.style.clamp(1, 12);
    let decide = |side: Option<bool>, outer: bool, current: u8| -> u8 {
        let outline = if outer { change.outline } else { None };
        if change.all == Some(true) || side == Some(true) || outline == Some(true) {
            style
        } else if change.all == Some(false) || side == Some(false) || outline == Some(false) {
            0
        } else {
            current
        }
    };
    for &(row, column) in owners {
        if honoured && keeps_out(&table, row, column) {
            continue;
        }
        let span = table.cells[row * columns + column].span;
        let bottom = row + usize::from(span.rows.max(1)) - 1;
        let right = column + usize::from(span.columns.max(1)) - 1;
        let lines = &mut table.cells[row * columns + column].format.lines;
        lines.left = decide(change.left, column == range.columns.start, lines.left);
        lines.right = decide(change.right, right + 1 >= range.columns.end, lines.right);
        lines.top = decide(change.top, row == range.rows.start, lines.top);
        lines.bottom = decide(change.bottom, bottom + 1 >= range.rows.end, lines.bottom);
    }
    widen_for_lines(&mut table);
    table
}

/// Gutters and insets as wide as the lines in them (`14b0:09c8`): never
/// narrowed but the insets, which return towards 43 when their lines go.
fn widen_for_lines(table: &mut Table) {
    let columns = table.columns.len();
    let rows = table.rows.len();
    let width = |style: u8| line_style_width(u16::from(style));
    let mut top_inset = INSET;
    let mut left_inset = INSET;
    for (index, cell) in table.cells.iter().enumerate() {
        if cell.span.is_covered() {
            continue;
        }
        let (row, column) = (index / columns, index % columns);
        let lines = cell.format.lines;
        if row == 0 {
            top_inset = top_inset.max(width(lines.top));
        }
        if column == 0 {
            left_inset = left_inset.max(width(lines.left));
        }
    }
    table.spacing.top = top_inset.max(table.spacing.top.min(top_inset));
    table.spacing.left = left_inset.max(table.spacing.left.min(left_inset));
    for index in 0..table.cells.len() {
        let cell = &table.cells[index];
        if cell.span.is_covered() {
            continue;
        }
        let (row, column) = (index / columns, index % columns);
        let span = cell.span;
        let lines = cell.format.lines;
        let bottom = row + usize::from(span.rows.max(1)) - 1;
        let right = column + usize::from(span.columns.max(1)) - 1;
        let widen_column = |column: usize, line: u8, table: &mut Table| {
            if column + 1 < columns && width(line) > table.column_gutter(column) {
                table.spacing.column_gutters.insert(column, width(line));
            }
        };
        if column > 0 {
            widen_column(column - 1, lines.left, table);
        }
        widen_column(right, lines.right, table);
        let widen_row = |row: usize, line: u8, table: &mut Table| {
            if row + 1 < rows && width(line) > table.row_gutter(row) {
                table.spacing.row_gutters.insert(row, width(line));
            }
        };
        if row > 0 {
            widen_row(row - 1, lines.top, table);
        }
        widen_row(bottom, lines.bottom, table);
    }
}

fn clear_cells(
    next: &mut Document,
    mut table: Table,
    range: &CellRange,
    report: &mut ChangeReport,
) -> Result<Table, EditError> {
    check_range(&table, range)?;
    let columns = table.columns.len();
    let style = table_text(next, report);
    for row in range.rows.clone() {
        for column in range.columns.clone() {
            if keeps_out(&table, row, column) {
                continue;
            }
            let cell = &mut table.cells[row * columns + column];
            if let Some(frame) = cell.frame.take() {
                crate::frames::delete_frame_tree(next, frame, report);
            }
            let CellContent::Text(flow) = cell.content;
            // Del on a calculated cell leaves no formula and a plain cell (TDF1).
            let had_formula = cell.formula.take().is_some();
            let empty = next.flow(flow).is_some_and(|text| {
                text.paragraphs.len() == 1 && text.paragraphs[0].text.is_empty()
            });
            if !empty || had_formula {
                let paragraph: Paragraph = next.new_paragraph("", style);
                let id = paragraph.id;
                next.replace_flow_paragraphs(flow, vec![std::sync::Arc::new(paragraph)])
                    .map_err(|_| "cell text does not exist")?;
                report.flows.insert(flow);
                report.inserted.push(id);
            }
        }
    }
    Ok(table)
}

/// Moves (or copies) rows to stand before row `to`, with their sizes,
/// gutters, headings, and page breaks; a joined block may not be cut.
fn move_rows(
    next: &mut Document,
    table: Table,
    rows: Range<usize>,
    to: usize,
    copy: bool,
    report: &mut ChangeReport,
) -> Result<Table, EditError> {
    if rows.is_empty() || rows.end > table.rows.len() || to > table.rows.len() {
        return Err("rows outside the table");
    }
    if !copy && (rows.start..=rows.end).contains(&to) {
        return Ok(table);
    }
    let columns = table.columns.len();
    let whole = CellRange {
        rows: rows.clone(),
        columns: 0..columns,
    };
    if expanded(&table, &whole) != whole {
        return Err("rows inside a joined cell move with the whole cell");
    }
    // Each row's data, in order: the row as the table has it.
    let mut order: Vec<Option<usize>> = (0..table.rows.len()).map(Some).collect();
    let moving: Vec<Option<usize>> = rows.clone().map(Some).collect();
    let insert_at = if copy || to < rows.start {
        to
    } else {
        to - rows.len()
    };
    if !copy {
        order.drain(rows.clone());
    }
    let marked: Vec<Option<usize>> = moving
        .iter()
        .map(|row| {
            if copy {
                row.map(|row| row + usize::MAX / 2)
            } else {
                *row
            }
        })
        .collect();
    order.splice(insert_at..insert_at, marked);
    let order: Vec<(usize, bool)> = order
        .iter()
        .map(|entry| {
            let entry = entry.expect("every row has a source");
            if entry >= usize::MAX / 2 {
                (entry - usize::MAX / 2, true)
            } else {
                (entry, false)
            }
        })
        .collect();
    Ok(reorder_rows(next, table, &order, report))
}

/// `table` with its rows in `order`: each entry the old row it comes from
/// and whether it is a copy (new cell texts), every row's own size,
/// gutter, heading, and page break going with it.
fn reorder_rows(
    next: &mut Document,
    mut table: Table,
    order: &[(usize, bool)],
    report: &mut ChangeReport,
) -> Table {
    let columns = table.columns.len();
    let style = table_text(next, report);
    let old = table.clone();
    let dims = formula::Dims {
        columns: columns.max(1),
        rows: order.len().max(1),
    };
    let mut cells = Vec::with_capacity(order.len() * columns);
    let mut sizes = Vec::new();
    let mut gutters = BTreeMap::new();
    let mut own = BTreeSet::new();
    let mut headings = BTreeSet::new();
    let mut breaks = BTreeSet::new();
    for (new_row, &(row, copied)) in order.iter().enumerate() {
        for column in 0..columns {
            let cell = &old.cells[row * columns + column];
            let mut cell = if copied {
                copy_cell(next, cell, style, report)
            } else {
                cell.clone()
            };
            shift_formula(&mut cell, new_row as isize - row as isize, 0, dims);
            cells.push(cell);
        }
        sizes.push(old.rows[row]);
        if let Some(gutter) = old.spacing.row_gutters.get(&row) {
            gutters.insert(new_row, *gutter);
        }
        if old.layout.own_heights.contains(&row) {
            own.insert(new_row);
        }
        if old.row_options.headings.contains(&row) {
            headings.insert(new_row);
        }
        if old.row_options.page_breaks.contains(&row) {
            breaks.insert(new_row);
        }
    }
    table.cells = cells;
    table.rows = sizes;
    table.spacing.row_gutters = gutters;
    table.layout.own_heights = own;
    table.row_options.headings = headings;
    table.row_options.page_breaks = breaks;
    table.read_rows.clear();
    table
}

/// Two cells' texts in sort order: numbers by value, other text without
/// regard to case (as the record editor sorted, MR1).
pub fn sort_order(a: &str, b: &str) -> std::cmp::Ordering {
    match (
        crate::merge::conditions::ami_pro_number(a),
        crate::merge::conditions::ami_pro_number(b),
    ) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

fn sort_rows(
    next: &mut Document,
    table: Table,
    column: usize,
    descending: bool,
    report: &mut ChangeReport,
) -> Result<Table, EditError> {
    let columns = table.columns.len();
    if column >= columns {
        return Err("column outside the table");
    }
    let count = table.rows.len();
    let first = (0..count)
        .find(|row| !table.row_options.headings.contains(row))
        .unwrap_or(count);
    if count - first < 2 {
        return Ok(table);
    }
    if table.cells[first * columns..]
        .iter()
        .any(|cell| cell.span != CellSpan::SINGLE)
    {
        return Err("rows with joined cells cannot be sorted");
    }
    // A cell holding a formula stops the sort (message 3113, notes 10.7).
    if table.cells[first * columns..]
        .iter()
        .any(|cell| cell.formula.is_some())
    {
        return Err(FORMULA_IN_SORT);
    }
    let key = |row: usize| {
        let CellContent::Text(flow) = table.cells[row * columns + column].content;
        next.flow(flow)
            .map(|flow| {
                flow.paragraphs
                    .iter()
                    .map(|paragraph| paragraph.text.replace('\u{FFFC}', ""))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    };
    let keys: Vec<String> = (first..count).map(key).collect();
    let mut rest: Vec<usize> = (first..count).collect();
    rest.sort_by(|a, b| {
        let order = sort_order(&keys[a - first], &keys[b - first]);
        if descending { order.reverse() } else { order }
    });
    let order: Vec<(usize, bool)> = (0..first).chain(rest).map(|row| (row, false)).collect();
    Ok(reorder_rows(next, table, &order, report))
}

/// Tools › Sort… on table rows: a record is `settings.record` rows (the
/// last may be short) and its fields are its cells, row after row.
fn sort_records(
    next: &mut Document,
    mut table: Table,
    rows: Option<Range<usize>>,
    settings: &crate::sort::SortSettings,
    report: &mut ChangeReport,
) -> Result<Table, EditError> {
    use crate::sort;
    if settings.keys.is_empty() || settings.keys.len() > 3 || settings.record == 0 {
        return Err("the sort needs one to three keys and a record size");
    }
    let columns = table.columns.len();
    let count = table.rows.len();
    let range = match rows {
        Some(rows) => rows.start..rows.end.min(count),
        None => {
            let first = (0..count)
                .find(|row| !table.row_options.headings.contains(row))
                .unwrap_or(count);
            first..count
        }
    };
    if range.len() < 2 {
        return Err(sort::SORT_NOTHING);
    }
    let cell_flows = |table: &Table, rows: Range<usize>| -> Vec<amipro_document::FlowId> {
        table.cells[rows.start * columns..rows.end * columns]
            .iter()
            .map(|cell| {
                let CellContent::Text(flow) = cell.content;
                flow
            })
            .collect()
    };
    // Message 3193: footnotes anywhere in the table refuse the sort.
    for flow in cell_flows(&table, 0..count) {
        let has_footnote = next.flow(flow).is_some_and(|flow| {
            flow.paragraphs.iter().any(|paragraph| {
                paragraph
                    .objects
                    .iter()
                    .any(|object| matches!(object, InlineObject::TableFootnote { .. }))
            })
        });
        if has_footnote {
            return Err(sort::SORT_FOOTNOTES);
        }
    }
    // Message 3113: a formula in a sorted row.
    if table.cells[range.start * columns..range.end * columns]
        .iter()
        .any(|cell| cell.formula.is_some())
    {
        return Err(FORMULA_IN_SORT);
    }
    // Message 3184: connected cells ask; Continue disconnects them.
    if table.cells.iter().any(|cell| cell.span != CellSpan::SINGLE) {
        if !settings.confirmed {
            return Err(sort::SORT_CONNECTED);
        }
        for cell in &mut table.cells {
            cell.span = CellSpan::SINGLE;
        }
    }
    let text_of = |next: &Document, flow| -> String {
        next.flow(flow)
            .map(|flow| {
                flow.paragraphs
                    .iter()
                    .map(|paragraph| paragraph.text.replace('\u{FFFC}', ""))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    };
    let starts: Vec<usize> = range.clone().step_by(settings.record).collect();
    let keys: Vec<Vec<sort::Key>> = starts
        .iter()
        .map(|&start| {
            let end = (start + settings.record).min(range.end);
            let fields: Vec<String> = cell_flows(&table, start..end)
                .into_iter()
                .map(|flow| text_of(next, flow))
                .collect();
            sort::keys_of_fields(&fields, settings)
        })
        .collect();
    let order = sort::sorted_order(&keys, settings.descending);
    let mut rows_order: Vec<usize> = (0..range.start).collect();
    for index in order {
        let start = starts[index];
        rows_order.extend(start..(start + settings.record).min(range.end));
    }
    rows_order.extend(range.end..count);
    if rows_order.iter().enumerate().all(|(new, old)| new == *old) {
        return Err(sort::SORT_NOTHING);
    }
    let order: Vec<(usize, bool)> = rows_order.into_iter().map(|row| (row, false)).collect();
    Ok(reorder_rows(next, table, &order, report))
}

fn move_columns(
    next: &mut Document,
    mut table: Table,
    moved: Range<usize>,
    to: usize,
    copy: bool,
    report: &mut ChangeReport,
) -> Result<Table, EditError> {
    let count = table.columns.len();
    if moved.is_empty() || moved.end > count || to > count {
        return Err("columns outside the table");
    }
    if !copy && (moved.start..=moved.end).contains(&to) {
        return Ok(table);
    }
    let whole = CellRange {
        rows: 0..table.rows.len(),
        columns: moved.clone(),
    };
    if expanded(&table, &whole) != whole {
        return Err("columns inside a joined cell move with the whole cell");
    }
    if table.columns.len() + if copy { moved.len() } else { 0 } > NewTable::MAX_COLUMNS {
        return Err("a table has at most 250 columns");
    }
    let mut order: Vec<(usize, bool)> = (0..count).map(|column| (column, false)).collect();
    let insert_at = if copy || to < moved.start {
        to
    } else {
        to - moved.len()
    };
    if !copy {
        order.drain(moved.clone());
    }
    order.splice(
        insert_at..insert_at,
        moved.clone().map(|column| (column, copy)),
    );
    let style = table_text(next, report);
    let old = table.clone();
    let mut cells = Vec::with_capacity(order.len() * table.rows.len());
    let dims = formula::Dims {
        columns: order.len().max(1),
        rows: old.rows.len().max(1),
    };
    for row in 0..old.rows.len() {
        for (new_column, (column, copied)) in order.iter().enumerate() {
            let cell = &old.cells[row * count + column];
            let mut cell = if *copied {
                copy_cell(next, cell, style, report)
            } else {
                cell.clone()
            };
            shift_formula(&mut cell, 0, new_column as isize - *column as isize, dims);
            cells.push(cell);
        }
    }
    table.cells = cells;
    table.columns = order
        .iter()
        .map(|(column, _)| old.columns[*column])
        .collect();
    table.spacing.column_gutters = order
        .iter()
        .enumerate()
        .filter_map(|(new, (column, _))| {
            old.spacing
                .column_gutters
                .get(column)
                .map(|gutter| (new, *gutter))
        })
        .collect();
    table.layout.own_widths = order
        .iter()
        .enumerate()
        .filter(|(_, (column, _))| old.layout.own_widths.contains(column))
        .map(|(new, _)| new)
        .collect();
    table.read_rows.clear();
    Ok(table)
}

/// A copy of a cell with its own text flow (a frame it holds is not
/// copied: Ami Pro copies cells' text).
fn copy_cell(
    next: &mut Document,
    cell: &TableCell,
    style: StyleId,
    report: &mut ChangeReport,
) -> TableCell {
    let CellContent::Text(flow) = cell.content;
    let copy = next.create_flow(style);
    report.added_flows.push(copy);
    let originals: Vec<Paragraph> = next
        .flow(flow)
        .map(|text| {
            text.paragraphs
                .iter()
                .map(|paragraph| (**paragraph).clone())
                .collect()
        })
        .unwrap_or_default();
    let paragraphs: Vec<std::sync::Arc<Paragraph>> = originals
        .into_iter()
        .map(|paragraph| {
            let id = next.new_paragraph("", paragraph.style).id;
            std::sync::Arc::new(Paragraph { id, ..paragraph })
        })
        .collect();
    if !paragraphs.is_empty() {
        let _ = next.replace_flow_paragraphs(copy, paragraphs);
    }
    TableCell {
        content: CellContent::Text(copy),
        span: cell.span,
        frame: None,
        format: cell.format,
        formula: cell.formula.clone(),
        legacy: LegacyData::default(),
    }
}

/// The table styles (TD7) as Ami Pro's lines and fills.
fn apply_style(mut table: Table, style: TableStyle) -> Table {
    const SHADE: Colour = 0x00E1_E1E1;
    const BAND: Colour = 0x00F7_F7F7;
    let columns = table.columns.len();
    let rows = table.rows.len();
    for index in 0..table.cells.len() {
        let (row, column) = (index / columns, index % columns);
        let span = table.cells[index].span;
        let bottom = row + usize::from(span.rows.max(1)) - 1;
        let right = column + usize::from(span.columns.max(1)) - 1;
        let format = &mut table.cells[index].format;
        let all = amipro_document::CellLines {
            left: 1,
            right: 1,
            top: 1,
            bottom: 1,
        };
        match style {
            TableStyle::Grid => {
                format.lines = all;
                format.fill = None;
            }
            TableStyle::Outline => {
                format.lines = amipro_document::CellLines {
                    left: if column == 0 { 2 } else { 0 },
                    right: if right + 1 == columns { 2 } else { 0 },
                    top: if row == 0 { 2 } else { 0 },
                    bottom: if bottom + 1 == rows { 2 } else { 0 },
                };
                format.fill = None;
            }
            TableStyle::HeaderRow => {
                format.lines = amipro_document::CellLines {
                    bottom: if row == 0 { 2 } else { 0 },
                    ..Default::default()
                };
                format.fill = (row == 0).then_some(SHADE);
            }
            TableStyle::BandedRows => {
                format.lines = all;
                format.fill = (row % 2 == 1).then_some(BAND);
            }
            TableStyle::Plain => {
                format.lines = Default::default();
                format.fill = None;
            }
        }
    }
    if style == TableStyle::HeaderRow && rows > 1 {
        table.row_options.headings.insert(0);
    }
    widen_for_lines(&mut table);
    table
}

/// The flow a table cell's text is in, and the table and cell.
pub fn cell_of_flow(document: &Document, flow: FlowId) -> Option<(FrameId, usize, usize)> {
    document.frames().find_map(|frame| {
        let FrameContent::Table(table) = &frame.content else {
            return None;
        };
        let columns = table.columns.len().max(1);
        table
            .cells
            .iter()
            .position(|cell| cell.content == CellContent::Text(flow))
            .map(|index| (frame.id, index / columns, index % columns))
    })
}

/// Convert Text to Table (TD7): the main text's paragraphs `first..=last`
/// become the rows of a table, a column for each tab-separated part, in a
/// paragraph of their own where they stood; the columns fill `width`. One
/// step.
pub(crate) fn text_to_table(
    document: &Document,
    first: usize,
    last: usize,
    width: Twips,
    row_height: Twips,
) -> Result<Applied, EditError> {
    let main = document.main_flow_id();
    let paragraphs = &document.main_flow().paragraphs;
    if first > last || last >= paragraphs.len() {
        return Err("paragraphs outside the text");
    }
    if paragraphs[first..=last].iter().any(|paragraph| {
        paragraph
            .objects
            .iter()
            .any(|object| object.anchored_frame().is_some())
    }) {
        return Err("text holding frames or tables does not become a table");
    }
    let rows: Vec<Vec<String>> = paragraphs[first..=last]
        .iter()
        .map(|paragraph| paragraph.text.split('\t').map(str::to_owned).collect())
        .collect();
    let columns = rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(1)
        .clamp(1, NewTable::MAX_COLUMNS);
    // The paragraphs go, leaving one empty paragraph in the first's style.
    let end_text = &paragraphs[last].text;
    let cleared = crate::replace_range(
        document.clone(),
        TextPosition {
            flow: main,
            paragraph: first,
            byte: 0,
        },
        TextPosition {
            flow: main,
            paragraph: last,
            byte: end_text.len(),
        },
        "",
        Vec::new(),
        None,
    )?;
    let at = TextPosition {
        flow: main,
        paragraph: first,
        byte: 0,
    };
    let spec = NewTable::fitting(columns, rows.len(), width, row_height);
    let made = insert_table(&cleared.document, TableTarget::Text(at), &spec)?;
    let mut next = made.document;
    let frame = *next.frame_stack().last().ok_or("no table was made")?;
    let style = table_text(&mut next, &mut ChangeReport::default());
    let table = table_of(&next, frame)?.clone();
    for (row, parts) in rows.iter().enumerate() {
        for (column, part) in parts.iter().enumerate().take(columns) {
            let CellContent::Text(flow) = table.cells[row * columns + column].content;
            let mut paragraph = next.new_paragraph(part.clone(), style);
            paragraph.style = style;
            let _ = next.replace_flow_paragraphs(flow, vec![std::sync::Arc::new(paragraph)]);
        }
    }
    let mut report = cleared.report;
    report.merge(made.report);
    Ok(Applied {
        document: next,
        caret: made.caret,
        report,
    })
}

/// Convert Table to Text (TD7): a table in the text becomes paragraphs where
/// its anchor was, a row each, its cells' text separated by tabs (a cell's
/// paragraphs joined by spaces). One step.
pub(crate) fn table_to_text(document: &Document, frame: FrameId) -> Result<Applied, EditError> {
    let table = table_of(document, frame)?;
    let at = crate::frames::anchor_position(document, frame)
        .ok_or("only a table in the text becomes text")?;
    let columns = table.columns.len();
    let text: Vec<String> = (0..table.rows.len())
        .map(|row| {
            (0..columns)
                .filter(|column| !table.cells[row * columns + column].span.is_covered())
                .map(|column| {
                    let CellContent::Text(flow) = table.cells[row * columns + column].content;
                    document
                        .flow(flow)
                        .map(|text| {
                            text.paragraphs
                                .iter()
                                .map(|paragraph| paragraph.text.replace('\u{FFFC}', ""))
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect();
    // The anchor goes with its table, and the rows take its place.
    let end = TextPosition {
        byte: at.byte + amipro_document::OBJECT_REPLACEMENT_CHARACTER.len_utf8(),
        ..at
    };
    crate::replace_range(
        document.clone(),
        at,
        end,
        &text.join("\n"),
        Vec::new(),
        None,
    )
}

/// Pastes a block of cells (`13b0:4065`, `14c8:044f`): each target cell's
/// text and look replaced by the block's, repeated over a selected area
/// that is a whole multiple of it; protected cells under Honor protection
/// and covered cells are passed over.
fn paste_cells(
    next: &mut Document,
    mut table: Table,
    at: (usize, usize),
    block: &CellBlock,
    area: Option<&CellRange>,
    add_rows: bool,
    report: &mut ChangeReport,
) -> Result<Table, EditError> {
    let (height, width) = (block.height(), block.width());
    if height == 0 || width == 0 {
        return Err("nothing to paste");
    }
    // The area the block fills: a selection it repeats over, or its own
    // size from the cell.
    let (rows, columns) = match area {
        Some(area) if !area.is_single() => {
            let fits = |extent: usize, size: usize| size == 1 || extent % size == 0;
            if !(fits(area.rows.len(), height) && fits(area.columns.len(), width))
                || (area.rows.len() < height || area.columns.len() < width)
            {
                return Err("the selected cells do not match the copied cells");
            }
            (area.rows.clone(), area.columns.clone())
        }
        _ => (at.0..at.0 + height, at.1..at.1 + width),
    };
    if rows.start > table.rows.len() || columns.start >= table.columns.len() {
        return Err("cells outside the table");
    }
    if rows.end > table.rows.len() && add_rows {
        let missing = rows.end - table.rows.len();
        let last = table.rows.len() - 1;
        let end = table.rows.len();
        table = insert_rows(next, table, end, missing, Some(last), report)?;
    }
    let count = table.columns.len();
    let style = table_text(next, report);
    let dims = formula::Dims {
        columns: count.max(1),
        rows: table.rows.len().max(1),
    };
    for row in rows.clone().filter(|row| *row < table.rows.len()) {
        for column in columns.clone().filter(|column| *column < count) {
            let index = row * count + column;
            if table.cells[index].span.is_covered() || keeps_out(&table, row, column) {
                continue;
            }
            let Some(data) = block
                .rows
                .get((row - rows.start) % height)
                .and_then(|cells| cells.get((column - columns.start) % width))
            else {
                continue;
            };
            let cell = &mut table.cells[index];
            if let Some(frame) = cell.frame.take() {
                crate::frames::delete_frame_tree(next, frame, report);
            }
            cell.format = data.format;
            // The formula moves by the distance between where the cell came
            // from and where it lands (a repeated block per tile).
            cell.formula = data.formula.clone();
            let from_row = block.origin.0 + (row - rows.start) % height;
            let from_column = block.origin.1 + (column - columns.start) % width;
            shift_formula(
                cell,
                row as isize - from_row as isize,
                column as isize - from_column as isize,
                dims,
            );
            let CellContent::Text(flow) = cell.content;
            let source = if data.paragraphs.is_empty() {
                vec![next.new_paragraph("", style)]
            } else {
                data.paragraphs.clone()
            };
            let paragraphs: Vec<std::sync::Arc<Paragraph>> = source
                .into_iter()
                .map(|paragraph| {
                    let id = next.new_paragraph("", paragraph.style).id;
                    report.inserted.push(id);
                    std::sync::Arc::new(Paragraph { id, ..paragraph })
                })
                .collect();
            next.replace_flow_paragraphs(flow, paragraphs)
                .map_err(|_| "cell text does not exist")?;
            report.flows.insert(flow);
        }
    }
    Ok(table)
}

// ---- Formulas (TD.6; `docs/research/TABLE_DATA_NOTES.md` sections 10 and 11) ----

/// Message 3125 refused the formula: nothing was stored.
pub const FORMULA_REFUSED: &str = "cannot understand the formula";
/// Message 3113: a cell holding a formula stops a sort.
pub const FORMULA_IN_SORT: &str = "a cell holds a formula";
/// Quick Add on the first column or row, or Edit Formula's Del on a cell
/// without a formula: nothing to do.
pub const NOTHING_TO_ADD: &str = "nothing to add";
pub const NO_FORMULA: &str = "the cell holds no formula";
/// A recalculation that changes no result.
pub const NOTHING_RECALCULATED: &str = "no result changes";

/// What the engine accepts in the document: Ami Pro's set and the
/// native-only extras (Save As lists them).
fn formula_options() -> formula::Options {
    formula::Options {
        extras: true,
        decimal: None,
    }
}

fn table_dims(table: &Table) -> formula::Dims {
    formula::Dims {
        columns: table.columns.len().max(1),
        rows: table.rows.len().max(1),
    }
}

/// The names of every formula in the table follow a change of its shape.
fn rewrite_formulas(table: &mut Table, change: formula::Change) {
    let dims = table_dims(table);
    for cell in &mut table.cells {
        if let Some(text) = &cell.formula
            && let Some(rewritten) = formula::rewrite(text, change, dims, formula_options())
        {
            cell.formula = Some(rewritten);
        }
    }
}

/// A formula that moves by an offset: relative parts shift, `$` parts stay,
/// a name leaving the table becomes REF (mode 3 of `1420:04a8`).
fn shift_formula(cell: &mut TableCell, rows: isize, columns: isize, dims: formula::Dims) {
    if rows == 0 && columns == 0 {
        return;
    }
    if let Some(text) = &cell.formula
        && let Some(rewritten) = formula::rewrite(
            text,
            formula::Change::Shift { rows, columns },
            dims,
            formula_options(),
        )
    {
        cell.formula = Some(rewritten);
    }
}

/// Which results a recalculation writes.
enum Scope {
    /// Every formula cell.
    All,
    /// The formula cells that read the `changed` cells, and `plus` itself.
    From {
        changed: Vec<usize>,
        plus: Vec<usize>,
    },
}

/// The number format of the style of the cell's first paragraph.
fn cell_number_format(document: &Document, cell: &TableCell) -> NumberFormat {
    let CellContent::Text(flow) = cell.content;
    document
        .flow(flow)
        .and_then(|flow| flow.paragraphs.first())
        .and_then(|paragraph| document.styles().get(&paragraph.style))
        .map(|style| style.kept.number_format.clone())
        .unwrap_or_default()
}

/// Runs a command on `next` and keeps its report.
fn step(
    next: &mut Document,
    report: &mut ChangeReport,
    command: &Command,
) -> Result<(), EditError> {
    let applied = crate::apply_command(next, command)?;
    *next = applied.document;
    report.merge(applied.report);
    Ok(())
}

/// Works out the formulas' results of `table` (its cells' flows are in
/// `next`) and writes those of the scope that differ into the cells' text,
/// each rounded as its style stores it. Results Ami Pro could not read
/// (a formula it did not understand) stay as read. With `keep_ref`, a
/// result that is REF keeps its old text (after a row or column was
/// inserted or deleted Ami Pro rewrites the formula and does not touch the
/// stored result, VM TDF3). Returns whether any text changed.
fn recalculate_cells(
    next: &mut Document,
    table: &Table,
    scope: &Scope,
    keep_ref: bool,
    report: &mut ChangeReport,
) -> Result<bool, EditError> {
    if !table.cells.iter().any(|cell| cell.formula.is_some()) {
        return Ok(false);
    }
    let (dims, inputs) = formula::table_inputs(next, table);
    let formats: Vec<NumberFormat> = table
        .cells
        .iter()
        .map(|cell| cell_number_format(next, cell))
        .collect();
    let round = |index: usize, value: f64| {
        let format = &formats[index];
        format.scan(&format.stored_text(value)).unwrap_or(value)
    };
    let outcomes = formula::calculate(dims, &inputs, formula_options(), &round);
    let targets: BTreeSet<usize> = match scope {
        Scope::All => (0..inputs.len()).collect(),
        Scope::From { changed, plus } => {
            let mut from = changed.clone();
            from.extend(plus);
            let mut found: BTreeSet<usize> =
                formula::dependants(dims, &inputs, &from, formula_options())
                    .into_iter()
                    .collect();
            found.extend(plus);
            found
        }
    };
    let mut any = false;
    for index in targets {
        let Some(Some(outcome)) = outcomes.get(index).copied() else {
            continue;
        };
        if matches!(outcome, formula::Outcome::Failed(formula::Failure::Syntax))
            || (keep_ref && outcome == formula::Outcome::Ref)
        {
            continue;
        }
        let text = outcome.stored_text(&formats[index]);
        let CellContent::Text(flow) = table.cells[index].content;
        let Some(paragraph) = next.flow(flow).and_then(|flow| flow.paragraphs.first()) else {
            continue;
        };
        if paragraph.text != text {
            let at = |byte| TextPosition {
                flow,
                paragraph: 0,
                byte,
            };
            let end = paragraph.text.len();
            step(
                next,
                report,
                &Command::ReplaceText {
                    start: at(0),
                    end: at(end),
                    text,
                },
            )?;
            any = true;
        }
    }
    Ok(any)
}

/// The cell's result is right-aligned (`<+A>`), as every calculated cell's
/// body is (TDF1).
fn align_result(
    next: &mut Document,
    table: &Table,
    index: usize,
    report: &mut ChangeReport,
) -> Result<(), EditError> {
    let CellContent::Text(flow) = table.cells[index].content;
    let needs = next
        .flow(flow)
        .and_then(|flow| flow.paragraphs.first())
        .is_some_and(|paragraph| paragraph.format.alignment.is_none());
    if needs {
        step(
            next,
            report,
            &Command::SetParagraphAlignment {
                flow,
                first: 0,
                last: 0,
                alignment: Some(amipro_document::Alignment::Right),
            },
        )?;
    }
    Ok(())
}

/// Edit Formula's OK, and Del on a calculated cell (`None`).
pub(crate) fn set_cell_formula(
    document: &Document,
    frame: FrameId,
    row: usize,
    column: usize,
    text: Option<&str>,
) -> Result<Applied, EditError> {
    let mut table = table_of(document, frame)?.clone();
    let columns = table.columns.len();
    if row >= table.rows.len() || column >= columns {
        return Err("cell outside the table");
    }
    let index = row * columns + column;
    if table.cells[index].span.is_covered() {
        return Err("the cell is joined into another");
    }
    if keeps_out(&table, row, column) {
        return Err("the cell is protected");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    report.frames.insert(frame);
    match text {
        Some(text) => {
            // The text is stored as typed; whether Ami Pro reads it is the
            // compiler's answer (3125 otherwise, and nothing is stored).
            if text.chars().count() > 1024
                || text.chars().any(char::is_control)
                || formula::Formula::compile(text, table_dims(&table), formula_options()).is_err()
            {
                return Err(FORMULA_REFUSED);
            }
            table.cells[index].formula = Some(text.to_owned());
            align_result(&mut next, &table, index, &mut report)?;
            let scope = Scope::From {
                changed: Vec::new(),
                plus: vec![index],
            };
            recalculate_cells(&mut next, &table, &scope, false, &mut report)?;
        }
        None => {
            if table.cells[index].formula.take().is_none() {
                return Err(NO_FORMULA);
            }
            let style = table_text(&mut next, &mut report);
            let CellContent::Text(flow) = table.cells[index].content;
            let paragraph: Paragraph = next.new_paragraph("", style);
            let id = paragraph.id;
            next.replace_flow_paragraphs(flow, vec![std::sync::Arc::new(paragraph)])
                .map_err(|_| "cell text does not exist")?;
            report.flows.insert(flow);
            report.inserted.push(id);
            let scope = Scope::From {
                changed: vec![index],
                plus: Vec::new(),
            };
            recalculate_cells(&mut next, &table, &scope, false, &mut report)?;
        }
    }
    put(&mut next, frame, table);
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// The text Quick Add writes (notes 10.4): the range ends at the cell next
/// to the caret cell and starts after the nearest cell to hold text that is
/// not a number and has no formula (or a joined cell); with none it starts
/// at the table's edge; with the neighbour itself the stopper, the range is
/// that one cell. The first column (Row) or row (Column) has nothing to add.
pub fn quick_add_text(
    document: &Document,
    table: &Table,
    row: usize,
    column: usize,
    along_row: bool,
) -> Result<String, EditError> {
    let columns = table.columns.len();
    if row >= table.rows.len() || column >= columns {
        return Err("cell outside the table");
    }
    let position = if along_row { column } else { row };
    if position == 0 {
        return Err(NOTHING_TO_ADD);
    }
    let (_, inputs) = formula::table_inputs(document, table);
    let at = |step: usize| {
        if along_row {
            (row, step)
        } else {
            (step, column)
        }
    };
    let stops = |step: usize| {
        let (r, c) = at(step);
        let index = r * columns + c;
        table.cells[index].span != CellSpan::SINGLE || matches!(inputs[index], formula::Input::Text)
    };
    let first = match (0..position).rev().find(|step| stops(*step)) {
        Some(stopper) if stopper + 1 == position => stopper,
        Some(stopper) => stopper + 1,
        None => 0,
    };
    let name = |step: usize| {
        let (r, c) = at(step);
        formula::CellRef::new(c, r).name()
    };
    Ok(format!("@sum({}..{})", name(first), name(position - 1)))
}

pub(crate) fn quick_add(
    document: &Document,
    frame: FrameId,
    row: usize,
    column: usize,
    along_row: bool,
) -> Result<Applied, EditError> {
    let table = table_of(document, frame)?;
    let text = quick_add_text(document, table, row, column, along_row)?;
    set_cell_formula(document, frame, row, column, Some(&text))
}

/// The results of a table's formulas worked out again: all of them
/// (`changed` empty), or those reading the changed cells.
pub fn recalculate_table(
    document: &Document,
    frame: FrameId,
    changed: &[(usize, usize)],
) -> Result<Applied, EditError> {
    let table = table_of(document, frame)?;
    let columns = table.columns.len();
    let mut cells = Vec::with_capacity(changed.len());
    for (row, column) in changed {
        if *row >= table.rows.len() || *column >= columns {
            return Err("cell outside the table");
        }
        cells.push(row * columns + column);
    }
    let scope = if cells.is_empty() {
        Scope::All
    } else {
        Scope::From {
            changed: cells,
            plus: Vec::new(),
        }
    };
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    report.frames.insert(frame);
    if !recalculate_cells(&mut next, table, &scope, false, &mut report)? {
        return Err(NOTHING_RECALCULATED);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}
