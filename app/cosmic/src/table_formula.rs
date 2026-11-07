//! Formulas in the page surface's tables (run C, TD.7; `UI_SPEC.md` §41,
//! `docs/research/TABLE_DATA_NOTES.md` section 11.3): what the Formula
//! section shows for the caret's cell, the messages for a formula that
//! cannot be calculated (Ami Pro's 3125, 3126, 3127 and the REF word), and
//! the Table menu's formula commands.

use amipro_document::formula::{self, CellRef, Failure, Outcome};
use amipro_document::{CellContent, Document, FrameContent, FrameId, Table};
use amipro_editing::Command;
use amipro_editing::tables;
use cosmic::widget;

use crate::command::CommandId;
use crate::{AmiProApp, Message, table_editing};

/// The box Table Properties' Formula section edits the formula in.
pub const FORMULA_FIELD: &str = "table-formula";

/// Why a formula cell shows no number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Problem {
    /// Message 3125: the formula is not understood.
    Syntax,
    /// Message 3126.
    DivideByZero,
    /// Message 3127.
    Loop,
    /// The formula read a cell that is not a number: Ami Pro draws REF and
    /// says nothing.
    Ref,
}

/// Ami Pro's message for `problem` in the cell at `row`, `column` (0-based),
/// the cell's name built here as Ami Pro's text takes it.
pub fn message(problem: Problem, row: usize, column: usize) -> String {
    let name = CellRef::new(column, row).name();
    match problem {
        Problem::Syntax => format!("Cannot understand the formula in cell '{name}'."),
        Problem::DivideByZero => "There is a formula that tries to divide by zero.".into(),
        Problem::Loop => "This field contains a circular formula.".into(),
        Problem::Ref => format!("Cell {name} refers to a cell that is not a number (REF)."),
    }
}

/// Whether `text` is one of the messages (so a status line showing one can
/// be cleared when the cause is fixed).
pub fn is_message(text: &str) -> bool {
    text.starts_with("Cannot understand the formula in cell")
        || text == "There is a formula that tries to divide by zero."
        || text == "This field contains a circular formula."
}

fn options() -> formula::Options {
    formula::Options {
        extras: true,
        decimal: None,
    }
}

/// The calculated outcome of every formula cell of `table` (`None` for the
/// others), from the cells' stored text.
fn outcomes(document: &Document, table: &Table) -> Vec<Option<Outcome>> {
    let (dims, inputs) = formula::table_inputs(document, table);
    formula::calculate(dims, &inputs, options(), &|_, value| value)
}

fn problem_of(outcome: Outcome) -> Option<Problem> {
    match outcome {
        Outcome::Number(_) => None,
        Outcome::Ref => Some(Problem::Ref),
        Outcome::Failed(Failure::Syntax) => Some(Problem::Syntax),
        Outcome::Failed(Failure::DivideByZero) => Some(Problem::DivideByZero),
        Outcome::Failed(Failure::Loop) => Some(Problem::Loop),
    }
}

/// What the Formula section shows for one cell.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CellInfo {
    pub row: usize,
    pub column: usize,
    /// The cell's address, `C4`.
    pub name: String,
    /// The formula as typed.
    pub formula: Option<String>,
    /// The result as the cell's style displays it (`REF` for a formula that
    /// read a cell that is not a number; empty after a divide by zero or a
    /// loop, as Ami Pro draws them).
    pub result: String,
    pub problem: Option<Problem>,
}

impl CellInfo {
    pub fn message(&self) -> Option<String> {
        self.problem
            .map(|problem| message(problem, self.row, self.column))
    }
}

/// The cell at `row`, `column` of `table`.
pub fn cell_info(document: &Document, table: &Table, row: usize, column: usize) -> CellInfo {
    let columns = table.columns.len().max(1);
    let index = row * columns + column;
    let mut info = CellInfo {
        row,
        column,
        name: CellRef::new(column, row).name(),
        ..CellInfo::default()
    };
    let Some(cell) = table.cells.get(index) else {
        return info;
    };
    info.formula = cell.formula.clone();
    let CellContent::Text(flow) = cell.content;
    let paragraph = document
        .flow(flow)
        .and_then(|flow| flow.paragraphs.first().cloned());
    let text = paragraph
        .as_ref()
        .map_or_else(String::new, |paragraph| paragraph.text.clone());
    info.result = paragraph
        .as_ref()
        .and_then(|paragraph| document.styles().get(&paragraph.style))
        .and_then(|style| style.kept.number_format.display(&text))
        .map_or(text, |shown| shown.text);
    if cell.formula.is_some() {
        info.problem = outcomes(document, table)
            .get(index)
            .copied()
            .flatten()
            .and_then(problem_of);
        if info.problem == Some(Problem::Ref) {
            info.result = "REF".into();
        }
    }
    info
}

/// The first of the table's cells that cannot be calculated and that Ami
/// Pro tells about (3125, 3126, 3127; not REF): the preferred cell if it is
/// one, else the first in reading order.
pub fn first_problem(
    document: &Document,
    frame: FrameId,
    prefer: Option<(usize, usize)>,
) -> Option<String> {
    let FrameContent::Table(table) = &document.frame(frame)?.content else {
        return None;
    };
    if !table.cells.iter().any(|cell| cell.formula.is_some()) {
        return None;
    }
    let columns = table.columns.len().max(1);
    let found: Vec<(usize, Problem)> = outcomes(document, table)
        .into_iter()
        .enumerate()
        .filter_map(|(index, outcome)| {
            let problem = problem_of(outcome?)?;
            (problem != Problem::Ref).then_some((index, problem))
        })
        .collect();
    let preferred = prefer.and_then(|(row, column)| {
        found
            .iter()
            .find(|(index, _)| *index == row * columns + column)
    });
    let (index, problem) = preferred.or_else(|| found.first())?;
    Some(message(*problem, index / columns, index % columns))
}

impl crate::DocumentSession {
    /// The status line tells about a cell that cannot be calculated (TN8: no
    /// modal box), and forgets such a message once the cause is fixed.
    pub(crate) fn report_formulas(&mut self, frame: FrameId, prefer: Option<(usize, usize)>) {
        match first_problem(&self.document, frame, prefer) {
            Some(text) => self.status = text,
            None if is_message(&self.status) => self.status = "Modified".into(),
            None => {}
        }
    }

    /// Recalculates the formulas that read the cell the caret left after
    /// typing in it (TN5). `join` amends the result into the undo step the
    /// typing (and the number rewrite) made. A recalculation that changes
    /// nothing is not an edit.
    pub(crate) fn recalculate_cell(&mut self, flow: amipro_document::FlowId, join: bool) {
        let Some((frame, row, column)) = tables::cell_of_flow(&self.document, flow) else {
            return;
        };
        let has_formula = table_editing::table_of(&self.document, frame)
            .is_some_and(|table| table.cells.iter().any(|cell| cell.formula.is_some()));
        if !has_formula {
            return;
        }
        let command = Command::RecalculateTable {
            frame,
            changed: vec![(row, column)],
        };
        let previous = self.document.clone();
        let cursor = self.history_caret();
        let result = if join {
            self.history.amend(&mut self.document, cursor, &command)
        } else {
            self.history.execute(&mut self.document, cursor, &command)
        };
        // NOTHING_RECALCULATED (and any other refusal) leaves the document
        // as it was.
        if let Ok(executed) = result
            && !executed.report.is_empty()
        {
            let selection = self.page_selection;
            self.page_selection = crate::page_editing::valid_selection(&self.document, selection);
            self.refresh_rich_layout();
            crate::synchronize_editor_text(&mut self.editor, &previous, &self.document);
            self.sync_editor_to_page();
            self.dirty = true;
            self.typing_run = None;
        }
        self.report_formulas(frame, None);
    }

    /// Why a refused formula command was refused, as a message.
    fn refusal(reason: &str, row: usize, column: usize) -> String {
        match reason {
            tables::FORMULA_REFUSED => message(Problem::Syntax, row, column),
            tables::NOTHING_TO_ADD => "Nothing to add: there is no cell before this one.".into(),
            tables::NO_FORMULA => "The cell holds no formula.".into(),
            other => format!("Formula not set: {other}"),
        }
    }
}

/// The text the status bar shows when typing in a formula cell is refused.
pub const HOLDS_A_FORMULA: &str =
    "This cell holds a formula: use Table › Edit Formula… to change it";

impl AmiProApp {
    /// Paste of rich text or a picture, and Cut, in a formula cell's result
    /// do nothing but say how to change the formula (the same refusal as
    /// typing; a paste of cells stays a table command).
    pub(crate) fn refuse_in_formula_cell(&mut self) -> bool {
        if !self.page_surface {
            return false;
        }
        let session = &mut self.sessions[self.active_session];
        if crate::page_editing::formula_cell_refuses(
            &session.document,
            session.page_selection,
            &crate::page_editing::PageAction::Delete,
        ) {
            session.status = HOLDS_A_FORMULA.into();
            return true;
        }
        false
    }

    /// The caret cell's formula section, when the caret is in one cell.
    pub(crate) fn caret_cell_info(&self, at: &crate::table_commands::TableAt) -> CellInfo {
        cell_info(&self.document, &at.table, at.caret.0, at.caret.1)
    }

    /// Whether Edit Formula…, Quick Add, and Recalculate Table are on for
    /// the caret's table (`UI_SPEC.md` §41): one cell, not kept out by
    /// Honor protection; Quick Add Row needs a cell to the left, Column one
    /// above; Recalculate Table a formula in the table.
    pub(crate) fn formula_command_enabled(&self, command: CommandId) -> bool {
        let Some(at) = self.table_at() else {
            return false;
        };
        let table = &at.table;
        let single = !at.selected || at.expanded().is_single();
        let (row, column) = at.caret;
        let index = row * table.columns.len().max(1) + column;
        let writable = single
            && !tables::keeps_out(table, row, column)
            && table
                .cells
                .get(index)
                .is_some_and(|cell| !cell.span.is_covered() && cell.frame.is_none());
        match command {
            CommandId::EditFormula => writable,
            CommandId::QuickAddRow => writable && column > 0,
            CommandId::QuickAddColumn => writable && row > 0,
            CommandId::RecalculateTable => table.cells.iter().any(|cell| cell.formula.is_some()),
            _ => false,
        }
    }

    /// Table › Edit Formula…: the Formula section of Table Properties open,
    /// its box focused with the formula selected.
    pub(crate) fn edit_formula(&mut self) -> cosmic::Task<cosmic::Action<Message>> {
        use crate::table_properties::Section;
        let Some(at) = self.table_at() else {
            return cosmic::Task::none();
        };
        if !self.formula_command_enabled(CommandId::EditFormula) {
            return cosmic::Task::none();
        }
        let open_here = self
            .table_properties
            .as_ref()
            .is_some_and(|state| state.frame == at.frame);
        if !open_here {
            self.open_table_properties();
        }
        let session = &mut self.sessions[self.active_session];
        if let Some(state) = session.table_properties.as_mut() {
            // The other sections fold so the Formula section shows without
            // scrolling; they open again from their headings.
            state.open = [Section::Formula].into();
            state.formula = None;
            state.error = None;
        }
        if !self.rail_open() {
            self.open_rail();
        }
        let id = widget::Id::new(FORMULA_FIELD);
        widget::text_input::focus::<Message>(id.clone())
            .chain(widget::text_input::select_all::<Message>(id))
            .chain(cosmic::Task::done(Message::Repaint))
            .map(cosmic::Action::from)
    }

    /// Sets, removes, or Quick Adds the formula of the caret's cell as one
    /// undo step, with the message of a refusal in the status line and the
    /// Formula section.
    pub(crate) fn apply_formula(&mut self, change: FormulaChange) {
        let Some(at) = self.table_at() else {
            return;
        };
        let (row, column) = at.caret;
        let frame = at.frame;
        let (command, done) = match change {
            FormulaChange::Set(text) => (
                Command::SetCellFormula {
                    frame,
                    row,
                    column,
                    formula: Some(text),
                },
                "Formula set",
            ),
            FormulaChange::Remove => (
                Command::SetCellFormula {
                    frame,
                    row,
                    column,
                    formula: None,
                },
                "Formula removed",
            ),
            FormulaChange::QuickAdd { along_row } => (
                Command::QuickAdd {
                    frame,
                    row,
                    column,
                    along_row,
                },
                "Quick Add",
            ),
        };
        let page = self.page_selection.page;
        let session = &mut self.sessions[self.active_session];
        if !session.execute_page_command(&command) {
            let reason = session
                .status
                .strip_prefix("Edit failed: ")
                .unwrap_or("")
                .to_owned();
            let text = crate::DocumentSession::refusal(&reason, row, column);
            session.status = text.clone();
            if let Some(state) = session.table_properties.as_mut() {
                state.error = Some(text);
            }
            return;
        }
        session.status = done.into();
        if let Some(state) = session.table_properties.as_mut() {
            state.formula = None;
        }
        // The caret stays in its cell.
        session.page_selection = crate::page_editing::valid_selection(
            &session.document,
            crate::page_editing::PageSelection::caret(session.page_selection.focus, page),
        );
        session.sync_editor_to_page();
        session.report_formulas(frame, Some((row, column)));
    }

    /// Table › Recalculate Table: every formula worked out again as one
    /// undo step.
    pub(crate) fn recalculate_table(&mut self) {
        let Some(at) = self.table_at() else {
            return;
        };
        let session = &mut self.sessions[self.active_session];
        let command = Command::RecalculateTable {
            frame: at.frame,
            changed: Vec::new(),
        };
        let selection = session.page_selection;
        match amipro_editing::apply(&session.document, &command) {
            Ok(_) => {
                if session.execute_page_command(&command) {
                    session.page_selection =
                        crate::page_editing::valid_selection(&session.document, selection);
                    session.sync_editor_to_page();
                    session.status = "Table recalculated".into();
                }
            }
            Err(_) => session.status = "Every result is already up to date".into(),
        }
        session.report_formulas(at.frame, None);
    }
}

/// What Table Properties' Formula section (or Quick Add) asks of the caret's
/// cell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormulaChange {
    Set(String),
    Remove,
    QuickAdd { along_row: bool },
}
