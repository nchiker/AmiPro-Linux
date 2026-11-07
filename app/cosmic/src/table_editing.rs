//! Tables on the page surface (`UI_SPEC.md` §32): cell selections, moving
//! between cells, and the commands the table keys send. A cell selection is
//! a page selection whose anchor and focus are in two cells of one table, so
//! it needs no state of its own; it is always a rectangle (`14a0:0000`).

use amipro_document::{Document, FlowId, FrameContent, FrameId, Table};
use amipro_editing::TextPosition;
use amipro_editing::tables::{CellRange, TableEdit, block_at, cell_caret, keeps_out};

use crate::page_editing::PageSelection;

/// The cell a flow is the text of: its table and (row, column).
pub fn cell_of(document: &Document, flow: FlowId) -> Option<(FrameId, usize, usize)> {
    amipro_editing::tables::cell_of_flow(document, flow)
}

pub fn table_of(document: &Document, frame: FrameId) -> Option<&Table> {
    match &document.frame(frame)?.content {
        FrameContent::Table(table) => Some(table),
        _ => None,
    }
}

/// A rectangle of selected cells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellSelection {
    pub frame: FrameId,
    pub anchor: (usize, usize),
    pub focus: (usize, usize),
}

impl CellSelection {
    /// The cells it covers, as Protect, Leaders, and Lines & Color take them.
    pub fn range(&self) -> CellRange {
        CellRange::spanning(self.anchor, self.focus)
    }

    /// Grown to whole joined cells, as Connect, Headings, and the Select
    /// commands take it (`14a0:1052`).
    pub fn expanded(&self, table: &Table) -> CellRange {
        amipro_editing::tables::expanded(table, &self.range())
    }
}

/// The selected cells, when the selection's ends are in two cells of one
/// table.
pub fn cell_selection(document: &Document, selection: PageSelection) -> Option<CellSelection> {
    if selection.anchor.flow == selection.focus.flow {
        return None;
    }
    let (frame, row, column) = cell_of(document, selection.anchor.flow)?;
    let (other, focus_row, focus_column) = cell_of(document, selection.focus.flow)?;
    (frame == other).then_some(CellSelection {
        frame,
        anchor: (row, column),
        focus: (focus_row, focus_column),
    })
}

/// The caret's table and cell, or the selection's focus cell.
pub fn caret_cell(
    document: &Document,
    selection: PageSelection,
) -> Option<(FrameId, usize, usize)> {
    cell_of(document, selection.focus.flow)
}

/// The selection after the focus moves to `target`: a drag or Shift from one
/// cell into another of the same table selects cells (TD3); otherwise as
/// [`PageSelection::moved`].
pub fn extended(
    document: &Document,
    selection: PageSelection,
    target: TextPosition,
    page: u32,
    extend: bool,
) -> PageSelection {
    if extend
        && target.flow != selection.anchor.flow
        && let (Some((frame, ..)), Some((other, ..))) = (
            cell_of(document, selection.anchor.flow),
            cell_of(document, target.flow),
        )
        && frame == other
    {
        return PageSelection {
            anchor: selection.anchor,
            focus: target,
            page,
        };
    }
    selection.moved(target, page, extend)
}

/// The start of the cell that shows at (row, column), as a caret position.
pub fn cell_start(
    document: &Document,
    frame: FrameId,
    row: usize,
    column: usize,
) -> Option<TextPosition> {
    cell_caret(table_of(document, frame)?, row, column)
}

/// Where Tab and Shift+Tab go (`1408:0b30`): the next or previous cell row
/// by row, past covered cells and, under Honor protection, protected ones.
pub enum Step {
    Cell(TextPosition),
    /// Tab in the last cell: a row is added (TD5).
    AddRow,
    /// Shift+Tab in the first cell: nothing.
    Stay,
}

pub fn step(document: &Document, frame: FrameId, row: usize, column: usize, forward: bool) -> Step {
    let Some(table) = table_of(document, frame) else {
        return Step::Stay;
    };
    let (columns, rows) = (table.columns.len(), table.rows.len());
    let (top, left, _) = block_at(table, row, column);
    let mut index = (top * columns + left) as isize;
    loop {
        index += if forward { 1 } else { -1 };
        if index < 0 {
            return Step::Stay;
        }
        if index as usize >= rows * columns {
            return Step::AddRow;
        }
        let (row, column) = (index as usize / columns, index as usize % columns);
        let (owner_row, owner_column, _) = block_at(table, row, column);
        if (owner_row, owner_column) != (row, column) || keeps_out(table, row, column) {
            continue;
        }
        if let Some(position) = cell_caret(table, row, column) {
            return Step::Cell(position);
        }
    }
}

/// Ctrl+arrows (`1408:085a`, `1408:07d6`): a cell up, down, left, or right,
/// past joined cells, staying in the table.
pub fn neighbour(
    document: &Document,
    frame: FrameId,
    (row, column): (usize, usize),
    (down, right): (isize, isize),
) -> Option<(usize, usize)> {
    let table = table_of(document, frame)?;
    let (top, left, span) = block_at(table, row, column);
    let mut target = (row as isize, column as isize);
    if down > 0 {
        target.0 = (top + usize::from(span.rows.max(1))) as isize;
    } else if down < 0 {
        target.0 = top as isize - 1;
    }
    if right > 0 {
        target.1 = (left + usize::from(span.columns.max(1))) as isize;
    } else if right < 0 {
        target.1 = left as isize - 1;
    }
    let (rows, columns) = (table.rows.len() as isize, table.columns.len() as isize);
    if !(0..rows).contains(&target.0) || !(0..columns).contains(&target.1) {
        return None;
    }
    let (row, column) = (target.0 as usize, target.1 as usize);
    let (top, left, _) = block_at(table, row, column);
    Some((top, left))
}

/// The commands Tab in the last cell sends: a row after the last with the
/// last row's lines and fill.
pub fn add_row(document: &Document, frame: FrameId) -> Option<TableEdit> {
    let table = table_of(document, frame)?;
    let last = table.rows.len().checked_sub(1)?;
    Some(TableEdit::InsertRows {
        at: last + 1,
        count: 1,
        like: Some(last),
    })
}
