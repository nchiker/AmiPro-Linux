//! The Table menu, its right-click items, and Insert › Table… on the page
//! surface (`UI_SPEC.md` §32, decisions TD1–TD10): when each command is on,
//! its check mark, and what it does, each one undo step.

use amipro_document::{FrameContent, FrameId, Placement, Table, Twips};
use amipro_editing::Command;
use amipro_editing::tables::{
    CellChange, CellRange, NewTable, TableEdit, TableStyle, TableTarget, block_at, expanded,
};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::command::CommandId;
use crate::page_editing::{PageAction, PageSelection};
use crate::{AmiProApp, Message, table_editing};

/// The table the caret or the cell selection is in, and the cells the
/// commands act on: the selection's (as Ami Pro takes them, `14a0:1052` for
/// those that grow to whole joined cells), or the caret's cell.
pub struct TableAt {
    pub frame: FrameId,
    pub table: Table,
    pub caret: (usize, usize),
    pub raw: CellRange,
    pub selected: bool,
}

impl TableAt {
    pub fn expanded(&self) -> CellRange {
        expanded(&self.table, &self.raw)
    }

    /// The whole rows of the selection, or the caret's row (with any joined
    /// cell it is in).
    pub fn rows(&self) -> std::ops::Range<usize> {
        expanded(
            &self.table,
            &CellRange {
                rows: self.raw.rows.clone(),
                columns: 0..self.table.columns.len(),
            },
        )
        .rows
    }

    pub fn columns(&self) -> std::ops::Range<usize> {
        expanded(
            &self.table,
            &CellRange {
                rows: 0..self.table.rows.len(),
                columns: self.raw.columns.clone(),
            },
        )
        .columns
    }
}

/// A new table's row height: one line of Table Text (or Body Text) at its
/// size, as Ami Pro measures one with its printer (284 for 12 point,
/// `1448:1b81`).
pub fn one_line(document: &amipro_document::Document) -> Twips {
    let styles = document.styles();
    let size = styles
        .values()
        .find(|style| style.name.eq_ignore_ascii_case("Table Text"))
        .or_else(|| styles.get(&document.body_text_style()))
        .map_or(12.0, |style| style.font_size_points);
    ((size * 20.0 * 71.0 / 60.0).round() as Twips).max(10)
}

impl AmiProApp {
    /// The caret's table and the cells a command takes.
    pub(crate) fn table_at(&self) -> Option<TableAt> {
        if !self.page_surface || self.frame_selection.is_some() {
            return None;
        }
        let (frame, row, column) = table_editing::caret_cell(&self.document, self.page_selection)?;
        let table = table_editing::table_of(&self.document, frame)?.clone();
        let (raw, selected) =
            match table_editing::cell_selection(&self.document, self.page_selection) {
                Some(cells) => (cells.range(), true),
                None => (CellRange::cell(row, column), false),
            };
        Some(TableAt {
            frame,
            table,
            caret: (row, column),
            raw,
            selected,
        })
    }

    /// Insert › Table…: the caret in the main text, outside a table, or one
    /// empty text frame selected (`1180:133d`).
    pub(crate) fn table_target(&self) -> Option<TableTarget> {
        if !self.page_surface {
            return None;
        }
        if let Some(selection) = &self.frame_selection {
            let frame = selection.single(&self.document)?;
            let FrameContent::Text(flow) = self.document.frame(frame)?.content else {
                return None;
            };
            let text = self.document.flow(flow)?;
            let empty = text.paragraphs.len() == 1 && text.paragraphs[0].text.is_empty();
            return empty.then_some(TableTarget::Frame(frame));
        }
        let at = self.page_selection.focus;
        (at.flow == self.document.main_flow_id()).then_some(TableTarget::Text(at))
    }

    /// Paragraphs of the main text selected, for Convert Text to Table.
    fn text_for_table(&self) -> Option<(usize, usize)> {
        let selection = self.page_selection;
        if !self.page_surface
            || self.frame_selection.is_some()
            || selection.is_caret()
            || selection.anchor.flow != self.document.main_flow_id()
            || selection.focus.flow != selection.anchor.flow
        {
            return None;
        }
        let (start, end) = selection.ordered();
        Some((start.paragraph, end.paragraph))
    }

    pub(crate) fn table_rule_enabled(&self, rule: crate::command::EnableRule) -> bool {
        use crate::command::EnableRule;
        match rule {
            EnableRule::TableTarget => self.table_target().is_some(),
            EnableRule::TextToTable => self.text_for_table().is_some(),
            _ => false,
        }
    }

    /// Whether a Table menu command is on (`1180:2153`, `1180:1e80`).
    pub(crate) fn table_command_enabled(&self, command: CommandId) -> bool {
        if matches!(
            command,
            CommandId::EditFormula
                | CommandId::QuickAddRow
                | CommandId::QuickAddColumn
                | CommandId::RecalculateTable
        ) {
            return self.formula_command_enabled(command);
        }
        let Some(at) = self.table_at() else {
            return false;
        };
        let table = &at.table;
        let page_table = self
            .document
            .frame(at.frame)
            .is_some_and(|frame| frame.placement.is_anchored());
        match command {
            // Select Column needs two rows, Select Row two columns.
            CommandId::SelectColumn => table.rows.len() >= 2,
            CommandId::SelectRow => table.columns.len() >= 2,
            CommandId::ConnectCells => {
                let (top, left, span) = block_at(table, at.caret.0, at.caret.1);
                if at.selected {
                    !at.expanded().is_single()
                } else {
                    (top, left) == at.caret && span != amipro_document::CellSpan::SINGLE
                }
            }
            // Headings are a page table's (`1180:20b1`).
            CommandId::TableHeadings => page_table,
            CommandId::DistributeRows => !table.row_options.automatic && !at.rows().is_empty(),
            CommandId::TableToText => page_table,
            CommandId::DeleteRow | CommandId::DeleteColumn | CommandId::DeleteTable => true,
            _ => true,
        }
    }

    /// The Table menu's check marks: Headings, the caret cell's leader, and
    /// Protect Cells, by the first row or cell of the range (`1180:1ce4`).
    pub(crate) fn table_menu_check(&self, command: CommandId) -> Option<bool> {
        use amipro_document::CellLeader;
        let leader = |leader| {
            Some(self.table_at().is_some_and(|at| {
                let (top, left, _) = block_at(&at.table, at.raw.rows.start, at.raw.columns.start);
                at.table.cells[top * at.table.columns.len() + left]
                    .format
                    .leader
                    == leader
            }))
        };
        match command {
            CommandId::TableHeadings => Some(
                self.table_at()
                    .is_some_and(|at| at.table.row_options.headings.contains(&at.raw.rows.start)),
            ),
            CommandId::ProtectCells => Some(self.table_at().is_some_and(|at| {
                let (top, left, _) = block_at(&at.table, at.raw.rows.start, at.raw.columns.start);
                at.table.cells[top * at.table.columns.len() + left]
                    .format
                    .protected
            })),
            CommandId::LeaderNone => leader(CellLeader::None),
            CommandId::LeaderDots => leader(CellLeader::Dots),
            CommandId::LeaderDashes => leader(CellLeader::Dashes),
            CommandId::LeaderLine => leader(CellLeader::Line),
            _ => None,
        }
    }

    /// A Table menu label that follows the caret: Disconnect Cells in a
    /// joined cell (AMIPROUI 2378).
    pub(crate) fn table_menu_label(&self, command: CommandId) -> Option<&'static str> {
        (command == CommandId::ConnectCells
            && self.table_at().is_some_and(|at| {
                !at.selected
                    && block_at(&at.table, at.caret.0, at.caret.1).2
                        != amipro_document::CellSpan::SINGLE
            }))
        .then_some("Disconnect Cells")
    }

    /// The width a table's columns may take: the text column it stands in
    /// for a table in the text, the frame's inside for a frame table.
    fn table_room(&self, frame: FrameId) -> Option<Twips> {
        let holder = self.document.frame(frame)?;
        if let Placement::Fixed { rect, .. } = holder.placement {
            let margins = holder.style.interior.margins;
            return Some(rect.width() - margins.left - margins.right);
        }
        Some(self.caret_column_width())
    }

    /// The width of the text column the caret is in (the page's first
    /// column when the caret's line is not laid out).
    fn caret_column_width(&self) -> Twips {
        let layout = self.pages.layout();
        let focus = self.page_selection.focus;
        let page = layout
            .pages
            .iter()
            .find(|page| page.number == self.page_selection.page)
            .or_else(|| layout.pages.first());
        let Some(page) = page else {
            return 9_360;
        };
        let line_x = page
            .lines
            .iter()
            .find(|line| line.flow == focus.flow && line.paragraph_index == focus.paragraph)
            .map(|line| line.x);
        page.columns
            .iter()
            .find(|column| line_x.is_some_and(|x| x >= column.x && x < column.x + column.width))
            .or_else(|| page.columns.first())
            .map_or(9_360, |column| column.width.round() as Twips)
    }

    /// Runs a Table menu command (`UI_SPEC.md` §32).
    pub(crate) fn run_table_command(&mut self, command: CommandId) {
        if command == CommandId::Table {
            self.open_insert_table();
            return;
        }
        if command == CommandId::TextToTable {
            if let Some((first, last)) = self.text_for_table() {
                let width = self.caret_column_width();
                let row_height = one_line(&self.document);
                let session = &mut self.sessions[self.active_session];
                if session.execute_page_command(&Command::TextToTable {
                    first,
                    last,
                    width,
                    row_height,
                }) {
                    session.status = "Text converted to a table".into();
                }
            }
            return;
        }
        match command {
            CommandId::QuickAddRow | CommandId::QuickAddColumn => {
                self.apply_formula(crate::table_formula::FormulaChange::QuickAdd {
                    along_row: command == CommandId::QuickAddRow,
                });
                return;
            }
            CommandId::RecalculateTable => {
                self.recalculate_table();
                return;
            }
            _ => {}
        }
        let Some(at) = self.table_at() else {
            return;
        };
        let frame = at.frame;
        let table = &at.table;
        let rows = at.rows();
        let columns = at.columns();
        let cells = at.raw.clone();
        let edit = |edit: TableEdit| Command::EditTable { frame, edit };
        let command_to_run: Option<(Command, &str)> = match command {
            CommandId::InsertRowAbove => Some((
                edit(TableEdit::InsertRows {
                    at: rows.start,
                    count: rows.len(),
                    like: Some(rows.start),
                }),
                "Rows inserted",
            )),
            CommandId::InsertRowBelow => Some((
                edit(TableEdit::InsertRows {
                    at: rows.end,
                    count: rows.len(),
                    like: Some(rows.end - 1),
                }),
                "Rows inserted",
            )),
            CommandId::InsertColumnLeft | CommandId::InsertColumnRight => {
                let left = command == CommandId::InsertColumnLeft;
                Some((
                    edit(TableEdit::InsertColumns {
                        at: if left { columns.start } else { columns.end },
                        count: columns.len(),
                        like: Some(if left { columns.start } else { columns.end - 1 }),
                        room: self.table_room(frame),
                    }),
                    "Columns inserted",
                ))
            }
            CommandId::DeleteRow => {
                Some((edit(TableEdit::DeleteRows(rows.clone())), "Rows deleted"))
            }
            CommandId::DeleteColumn => Some((
                edit(TableEdit::DeleteColumns(columns.clone())),
                "Columns deleted",
            )),
            CommandId::DeleteTable => Some((Command::DeleteFrame { frame }, "Table deleted")),
            CommandId::SelectColumn | CommandId::SelectRow | CommandId::SelectTable => {
                let range = at.expanded();
                let (anchor, focus) = match command {
                    CommandId::SelectColumn => (
                        (0, range.columns.start),
                        (table.rows.len() - 1, range.columns.end - 1),
                    ),
                    CommandId::SelectRow => (
                        (range.rows.start, 0),
                        (range.rows.end - 1, table.columns.len() - 1),
                    ),
                    _ => ((0, 0), (table.rows.len() - 1, table.columns.len() - 1)),
                };
                drop(self.update_page(PageAction::SelectCells {
                    frame,
                    anchor,
                    focus,
                }));
                None
            }
            CommandId::ConnectCells => {
                if at.selected {
                    Some((edit(TableEdit::Connect(at.expanded())), "Cells connected"))
                } else {
                    let (row, column) = at.caret;
                    Some((
                        edit(TableEdit::Disconnect { row, column }),
                        "Cells disconnected",
                    ))
                }
            }
            CommandId::TableHeadings => {
                let on = !table.row_options.headings.contains(&rows.start);
                Some((
                    edit(TableEdit::Headings {
                        rows: rows.clone(),
                        on,
                    }),
                    if on {
                        "Heading rows"
                    } else {
                        "Not heading rows"
                    },
                ))
            }
            CommandId::LeaderNone
            | CommandId::LeaderDots
            | CommandId::LeaderDashes
            | CommandId::LeaderLine => {
                use amipro_document::CellLeader;
                let leader = match command {
                    CommandId::LeaderDots => CellLeader::Dots,
                    CommandId::LeaderDashes => CellLeader::Dashes,
                    CommandId::LeaderLine => CellLeader::Line,
                    _ => CellLeader::None,
                };
                Some((
                    edit(TableEdit::FormatCells {
                        cells,
                        change: CellChange::Leader(leader),
                    }),
                    "Leaders set",
                ))
            }
            CommandId::ProtectCells => {
                let on = self.table_menu_check(CommandId::ProtectCells) != Some(true);
                Some((
                    edit(TableEdit::FormatCells {
                        cells,
                        change: CellChange::Protect(on),
                    }),
                    if on {
                        "Cells protected"
                    } else {
                        "Cells unprotected"
                    },
                ))
            }
            CommandId::DistributeRows => Some((
                edit(TableEdit::Distribute {
                    rows: Some(if at.selected {
                        rows.clone()
                    } else {
                        0..table.rows.len()
                    }),
                    columns: None,
                }),
                "Rows distributed",
            )),
            CommandId::DistributeColumns => Some((
                edit(TableEdit::Distribute {
                    rows: None,
                    columns: Some(if at.selected {
                        columns.clone()
                    } else {
                        0..table.columns.len()
                    }),
                }),
                "Columns distributed",
            )),
            CommandId::SortAscending | CommandId::SortDescending => Some((
                edit(TableEdit::SortRows {
                    column: at.caret.1,
                    descending: command == CommandId::SortDescending,
                }),
                "Rows sorted",
            )),
            CommandId::TableStyleGrid
            | CommandId::TableStyleOutline
            | CommandId::TableStyleHeaderRow
            | CommandId::TableStyleBandedRows
            | CommandId::TableStylePlain => {
                let style = match command {
                    CommandId::TableStyleGrid => TableStyle::Grid,
                    CommandId::TableStyleOutline => TableStyle::Outline,
                    CommandId::TableStyleHeaderRow => TableStyle::HeaderRow,
                    CommandId::TableStyleBandedRows => TableStyle::BandedRows,
                    _ => TableStyle::Plain,
                };
                Some((edit(TableEdit::ApplyStyle(style)), "Table style applied"))
            }
            CommandId::TableToText => {
                Some((Command::TableToText { frame }, "Table converted to text"))
            }
            CommandId::TableProperties => {
                self.open_table_properties();
                None
            }
            _ => None,
        };
        let Some((command, done)) = command_to_run else {
            return;
        };
        let caret_flow = self.page_selection.focus.flow;
        let page = self.page_selection.page;
        let session = &mut self.sessions[self.active_session];
        if session.execute_page_command(&command) {
            session.status = done.into();
            session.report_formulas(frame, None);
            // The caret stays in its cell when the cell is still there.
            if session.document.flow(caret_flow).is_some() {
                session.page_selection = crate::page_editing::valid_selection(
                    &session.document,
                    PageSelection::caret(
                        amipro_editing::TextPosition {
                            flow: caret_flow,
                            paragraph: 0,
                            byte: 0,
                        },
                        page,
                    ),
                );
                session.sync_editor_to_page();
            }
        }
    }

    /// Insert › Table…'s dialog (`UI_SPEC.md` §32).
    fn open_insert_table(&mut self) {
        let Some(target) = self.table_target() else {
            return;
        };
        let width = match target {
            TableTarget::Frame(frame) => self.table_room(frame).unwrap_or(9_360),
            TableTarget::Text(_) => self.caret_column_width(),
        };
        let row_height = one_line(&self.document);
        let remembered = self.preferences.insert_table;
        let session = &mut self.sessions[self.active_session];
        session.insert_table_dialog = Some(InsertTableDialog::new(
            target, width, row_height, remembered,
        ));
    }

    pub(crate) fn update_insert_table(&mut self, action: InsertTableAction) {
        let session = &mut self.sessions[self.active_session];
        let Some(dialog) = session.insert_table_dialog.as_mut() else {
            return;
        };
        match dialog.update(action) {
            InsertTableOutcome::Stay => {}
            InsertTableOutcome::Cancel => session.insert_table_dialog = None,
            InsertTableOutcome::Insert(target, spec) => {
                session.insert_table_dialog = None;
                let size = (spec.columns, spec.rows);
                if session.execute_page_command(&Command::InsertTable {
                    target,
                    table: spec,
                }) {
                    session.frame_selection = None;
                    session.status = format!("Table of {} × {} inserted", size.0, size.1);
                    self.preferences.insert_table = Some(size);
                }
            }
        }
    }
}

/// Insert › Table…'s dialog: columns and rows, a grid to pick them from,
/// and Layout… (Ami Pro's Create Table with its Modify Table Layout).
#[derive(Clone, Debug, PartialEq)]
pub struct InsertTableDialog {
    pub target: TableTarget,
    /// The width the columns fill without Layout…'s width.
    pub room: Twips,
    pub row_height: Twips,
    pub columns: String,
    pub rows: String,
    pub layout: bool,
    /// Layout…'s column width; empty: fit the room.
    pub column_width: String,
    pub automatic: bool,
    pub height: String,
    pub center: bool,
    pub line_around: bool,
    pub honor_protection: bool,
    pub span_pages: bool,
    pub fixed_sizes: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InsertTableAction {
    Columns(String),
    Rows(String),
    Pick(usize, usize),
    ToggleLayout,
    ColumnWidth(String),
    Height(String),
    Automatic(bool),
    Center(bool),
    LineAround(bool),
    HonorProtection(bool),
    SpanPages(bool),
    FixedSizes(bool),
    Insert,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InsertTableOutcome {
    Stay,
    Cancel,
    Insert(TableTarget, NewTable),
}

/// The grid of sizes the dialog offers to pick from.
const GRID: (usize, usize) = (8, 6);

impl InsertTableDialog {
    /// Ami Pro's 5 × 5, or the size inserted last.
    pub fn new(
        target: TableTarget,
        room: Twips,
        row_height: Twips,
        remembered: Option<(usize, usize)>,
    ) -> Self {
        let (columns, rows) = remembered.unwrap_or((5, 5));
        Self {
            target,
            room,
            row_height,
            columns: columns.to_string(),
            rows: rows.to_string(),
            layout: false,
            column_width: String::new(),
            automatic: true,
            height: crate::style_properties::inches(row_height),
            center: false,
            line_around: false,
            honor_protection: false,
            span_pages: false,
            fixed_sizes: false,
            error: None,
        }
    }

    fn count(text: &str, max: usize) -> Option<usize> {
        text.trim()
            .parse()
            .ok()
            .filter(|count| (1..=max).contains(count))
    }

    /// The table the values describe, or why not.
    pub fn table(&self) -> Result<NewTable, String> {
        let columns =
            Self::count(&self.columns, NewTable::MAX_COLUMNS).ok_or("Columns are 1 to 250.")?;
        let rows = Self::count(&self.rows, NewTable::MAX_ROWS).ok_or("Rows are 1 to 4000.")?;
        let mut spec = NewTable::fitting(columns, rows, self.room, self.row_height);
        if self.layout {
            if !self.column_width.trim().is_empty() {
                spec.column_width = crate::lengths::parse_length(&self.column_width)
                    .filter(|width| *width >= 1)
                    .ok_or("Type a column width such as 1.00, 2.5 cm, or 72 pt.")?;
            }
            spec.automatic = self.automatic;
            if !self.automatic {
                spec.row_height = crate::lengths::parse_length(&self.height)
                    .filter(|height| *height >= 10)
                    .ok_or("Type a row height of at least 0.01 in.")?;
            }
            spec.center = self.center;
            spec.line_around = self.line_around.then_some(1);
            spec.honor_protection = self.honor_protection;
            spec.span_pages = self.span_pages && self.automatic;
            spec.fixed_sizes = self.fixed_sizes;
        }
        Ok(spec)
    }

    pub fn update(&mut self, action: InsertTableAction) -> InsertTableOutcome {
        self.error = None;
        match action {
            InsertTableAction::Columns(text) => self.columns = text,
            InsertTableAction::Rows(text) => self.rows = text,
            InsertTableAction::Pick(columns, rows) => {
                self.columns = columns.to_string();
                self.rows = rows.to_string();
            }
            InsertTableAction::ToggleLayout => self.layout = !self.layout,
            InsertTableAction::ColumnWidth(text) => self.column_width = text,
            InsertTableAction::Height(text) => self.height = text,
            InsertTableAction::Automatic(on) => self.automatic = on,
            InsertTableAction::Center(on) => self.center = on,
            InsertTableAction::LineAround(on) => self.line_around = on,
            InsertTableAction::HonorProtection(on) => self.honor_protection = on,
            InsertTableAction::SpanPages(on) => self.span_pages = on,
            InsertTableAction::FixedSizes(on) => self.fixed_sizes = on,
            InsertTableAction::Insert => match self.table() {
                Ok(spec) => return InsertTableOutcome::Insert(self.target, spec),
                Err(reason) => self.error = Some(reason),
            },
            InsertTableAction::Cancel => return InsertTableOutcome::Cancel,
        }
        InsertTableOutcome::Stay
    }

    pub fn view(&self) -> Element<'static, Message> {
        let send = Message::InsertTable;
        let chosen = (
            Self::count(&self.columns, NewTable::MAX_COLUMNS).unwrap_or(0),
            Self::count(&self.rows, NewTable::MAX_ROWS).unwrap_or(0),
        );
        // A grid of squares: a click picks that many columns and rows.
        let grid = widget::column::with_children(
            (1..=GRID.1)
                .map(|row| {
                    widget::row::with_children(
                        (1..=GRID.0)
                            .map(|column| {
                                let on = column <= chosen.0 && row <= chosen.1;
                                widget::button::custom(
                                    widget::container(widget::space())
                                        .width(Length::Fixed(14.0))
                                        .height(Length::Fixed(14.0)),
                                )
                                .class(if on {
                                    cosmic::theme::Button::Suggested
                                } else {
                                    cosmic::theme::Button::Standard
                                })
                                .padding(0)
                                .on_press(send(InsertTableAction::Pick(column, row)))
                                .into()
                            })
                            .collect::<Vec<Element<'static, Message>>>(),
                    )
                    .spacing(2)
                    .into()
                })
                .collect::<Vec<Element<'static, Message>>>(),
        )
        .spacing(2);
        let field = |label: &'static str, value: String, on: fn(String) -> InsertTableAction| {
            widget::row::with_children(vec![
                widget::text::body(label).width(Length::Fixed(110.0)).into(),
                widget::text_input("", value)
                    .on_input(move |text| send(on(text)))
                    .on_submit(move |_| send(InsertTableAction::Insert))
                    .width(Length::Fixed(80.0))
                    .into(),
            ])
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .into()
        };
        let mut controls: Vec<Element<'static, Message>> = vec![
            widget::row::with_children(vec![
                grid.into(),
                widget::column::with_children(vec![
                    field("Columns", self.columns.clone(), InsertTableAction::Columns),
                    field("Rows", self.rows.clone(), InsertTableAction::Rows),
                    widget::text::caption(format!("{} × {}", chosen.0, chosen.1)).into(),
                ])
                .spacing(8)
                .into(),
            ])
            .spacing(16)
            .into(),
        ];
        if self.layout {
            let check = |label: &'static str, on: bool, action: fn(bool) -> InsertTableAction| {
                widget::checkbox(on)
                    .label(label)
                    .on_toggle(move |on| send(action(on)))
                    .into()
            };
            controls.push(widget::text::heading("Layout").into());
            controls.push(field(
                "Column width",
                self.column_width.clone(),
                InsertTableAction::ColumnWidth,
            ));
            controls.push(widget::text::caption("Empty: the columns fill the space.").into());
            controls.push(check(
                "Automatic row height",
                self.automatic,
                InsertTableAction::Automatic,
            ));
            if !self.automatic {
                controls.push(field(
                    "Row height",
                    self.height.clone(),
                    InsertTableAction::Height,
                ));
            }
            controls.push(check(
                "Center table on page",
                self.center,
                InsertTableAction::Center,
            ));
            controls.push(check(
                "Line around table",
                self.line_around,
                InsertTableAction::LineAround,
            ));
            controls.push(check(
                "Honor protection",
                self.honor_protection,
                InsertTableAction::HonorProtection,
            ));
            if self.automatic {
                controls.push(check(
                    "Rows span pages",
                    self.span_pages,
                    InsertTableAction::SpanPages,
                ));
            }
            controls.push(check(
                "Disable mouse sizing",
                self.fixed_sizes,
                InsertTableAction::FixedSizes,
            ));
        }
        if let Some(error) = &self.error {
            controls.push(widget::text::caption(error.clone()).into());
        }
        widget::dialog()
            .title("Insert Table")
            .body(match self.target {
                TableTarget::Text(_) => "A table at the insertion point.",
                TableTarget::Frame(_) => "A table in the selected frame.",
            })
            .control(widget::column::with_children(controls).spacing(8))
            .primary_action(
                widget::button::suggested("Insert").on_press(send(InsertTableAction::Insert)),
            )
            .secondary_action(
                widget::button::standard("Cancel").on_press(send(InsertTableAction::Cancel)),
            )
            .tertiary_action(
                widget::button::text(if self.layout { "Less" } else { "Layout…" })
                    .on_press(send(InsertTableAction::ToggleLayout)),
            )
            .into()
    }
}

impl AmiProApp {
    /// Table Properties in the rail (`UI_SPEC.md` §32, TD2): it replaces the
    /// other rail panels and follows the caret's table.
    pub(crate) fn open_table_properties(&mut self) {
        let Some(at) = self.table_at() else {
            return;
        };
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.table_properties = Some(crate::table_properties::TableProperties::new(at.frame));
        self.open_rail();
    }

    /// The rows' heights as laid out, which Automatic off keeps (TE17).
    fn laid_out_rows(&self, frame: FrameId) -> Vec<Twips> {
        let Some(table) = table_editing::table_of(&self.document, frame) else {
            return Vec::new();
        };
        let mut heights = table.rows.clone();
        for page in &self.pages.layout().pages {
            for frame_box in page
                .frames
                .iter()
                .filter(|frame_box| frame_box.frame == frame)
            {
                if let amipro_layout::FrameBoxKind::Table { part, .. } = &frame_box.kind {
                    for (offset, row) in part.rows.clone().enumerate() {
                        if let (Some(height), Some(slot)) =
                            (part.heights.get(offset), heights.get_mut(row))
                        {
                            *slot = height.round() as Twips;
                        }
                    }
                }
            }
        }
        heights
    }

    pub(crate) fn update_table_properties(&mut self, action: crate::table_properties::TableAction) {
        use crate::table_properties::Outcome;
        let Some(at) = self.table_at() else {
            let session = &mut self.sessions[self.active_session];
            session.table_properties = None;
            return;
        };
        let line = crate::table_properties::frame_line(&self.document, at.frame);
        let measured = self.laid_out_rows(at.frame);
        let cell_formula = at.table.cells[at.caret.0 * at.table.columns.len().max(1) + at.caret.1]
            .formula
            .clone();
        let session = &mut self.sessions[self.active_session];
        let Some(state) = session.table_properties.as_mut() else {
            return;
        };
        let restoring = action == crate::table_properties::TableAction::FormulaRestore;
        match state.update(action, &at, line, measured, cell_formula.as_deref()) {
            Outcome::Stay => {
                // Esc gives the old formula back and the refusal goes.
                if restoring && crate::table_formula::is_message(&session.status) {
                    session.status = "Modified".into();
                }
            }
            Outcome::Formula(change) => self.apply_formula(change),
            Outcome::Back => session.table_properties = None,
            Outcome::Close => {
                session.table_properties = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            Outcome::Run(command, done) => self.run_table_edit(&command, done),
        }
    }

    /// Runs a table command from the rail or a colour panel, the caret kept
    /// in its cell.
    fn run_table_edit(&mut self, command: &Command, done: &str) {
        let caret_flow = self.page_selection.focus.flow;
        let selection = self.page_selection;
        let session = &mut self.sessions[self.active_session];
        let table_frame = table_editing::caret_cell(&session.document, selection).map(|c| c.0);
        if session.execute_page_command(command) {
            session.status = done.into();
            if let Some(frame) = table_frame {
                session.report_formulas(frame, None);
            }
            // The cells stay selected while they are there.
            let keep = session.document.flow(selection.anchor.flow).is_some()
                && session.document.flow(selection.focus.flow).is_some();
            session.page_selection = if keep {
                selection
            } else {
                crate::page_editing::valid_selection(
                    &session.document,
                    PageSelection::caret(
                        amipro_editing::TextPosition {
                            flow: caret_flow,
                            paragraph: 0,
                            byte: 0,
                        },
                        selection.page,
                    ),
                )
            };
            session.sync_editor_to_page();
        }
    }

    /// A colour chosen in Table Properties' panel: the cells' fill or their
    /// lines' colour, one undo step.
    pub(crate) fn table_colour(
        &mut self,
        target: crate::colour_control::Target,
        colour: amipro_document::Colour,
    ) {
        let Some(at) = self.table_at() else {
            return;
        };
        let Some(change) = crate::table_properties::colour_change(target, colour) else {
            return;
        };
        let command = Command::EditTable {
            frame: at.frame,
            edit: TableEdit::FormatCells {
                cells: at.raw.clone(),
                change,
            },
        };
        self.run_table_edit(&command, "Cells coloured");
    }

    /// Table Properties follows the caret's table and closes when the caret
    /// leaves tables.
    pub(crate) fn sync_table_properties(&mut self) {
        if self.table_properties.is_none() {
            return;
        }
        let frame = self.table_at().map(|at| at.frame);
        let session = &mut self.sessions[self.active_session];
        match frame {
            None => session.table_properties = None,
            Some(frame) => {
                if let Some(state) = session.table_properties.as_mut()
                    && state.frame != frame
                {
                    *state = crate::table_properties::TableProperties::new(frame);
                }
            }
        }
    }

    pub(crate) fn table_properties_view(&self) -> Option<(Element<'_, Message>, f32)> {
        let state = self.table_properties.as_ref()?;
        let at = self.table_at()?;
        let line = crate::table_properties::frame_line(&self.document, at.frame);
        let page_table = self
            .document
            .frame(at.frame)
            .is_some_and(|frame| frame.placement.is_anchored());
        let cell = self.caret_cell_info(&at);
        Some((
            crate::table_properties::panel(
                state,
                &at,
                line,
                page_table,
                self.colour_context(),
                &cell,
            ),
            crate::table_properties::WIDTH,
        ))
    }
}

impl AmiProApp {
    /// Copy and Cut of selected cells (`1240:2008`): the cells kept for a
    /// paste in a table, their text (tabs between cells, a line per row) for
    /// the clipboard. `None` without a cell selection.
    pub(crate) fn copy_cells(&mut self) -> Option<String> {
        let at = self.table_at().filter(|at| at.selected)?;
        let block = amipro_editing::tables::CellBlock::copy(&self.document, &at.table, &at.raw);
        let text = block.text();
        let whole = at.raw.rows == (0..at.table.rows.len())
            && at.raw.columns == (0..at.table.columns.len());
        self.cell_clipboard = Some((text.clone(), block, whole));
        Some(text)
    }

    /// Paste in or into a table (`13b0:4065`, `13b0:3b6e`): the cells copied
    /// here over the cells from the caret's (or repeated over the selected
    /// ones), tabbed or several lines of text as cells; a whole table copied
    /// here pasted in the text as a new table. One undo step. False when
    /// the paste is ordinary text.
    pub(crate) fn paste_cells(&mut self, text: &str) -> bool {
        let copied = self
            .cell_clipboard
            .as_ref()
            .filter(|(copied, ..)| {
                copied.as_str() == text.trim_end_matches(['\n', '\r']) || copied == text
            })
            .map(|(_, block, whole)| (block.clone(), *whole));
        let Some(at) = self.table_at() else {
            // A whole table copied, pasted in the main text: a new table.
            if let Some((block, true)) = copied
                && let Some(TableTarget::Text(position)) = self.table_target()
                && self.frame_selection.is_none()
            {
                let width = self.caret_column_width();
                let spec = NewTable::fitting(
                    block.width(),
                    block.height(),
                    width,
                    one_line(&self.document),
                );
                let session = &mut self.sessions[self.active_session];
                if !session.execute_page_command(&Command::InsertTable {
                    target: TableTarget::Text(position),
                    table: spec,
                }) {
                    return false;
                }
                let Some(frame) = session.document.frame_stack().last().copied() else {
                    return true;
                };
                session.amend_page_command(&Command::EditTable {
                    frame,
                    edit: TableEdit::PasteCells {
                        at: (0, 0),
                        block,
                        area: None,
                        add_rows: true,
                    },
                });
                session.status = "Table pasted".into();
                return true;
            }
            return false;
        };
        let block = match copied {
            Some((block, _)) => block,
            // Text with tabs or several lines fills cells, a line a row
            // (Ami Pro asks before putting several lines in one cell).
            None if text.contains('\t') || text.trim_end().contains('\n') => {
                let mut scratch = self.document.clone();
                let style = scratch.table_text_style();
                let block = amipro_editing::tables::CellBlock::from_text(&mut scratch, text, style);
                // Paragraph ids come from the document when pasted.
                block
            }
            None => return false,
        };
        let page_table = self
            .document
            .frame(at.frame)
            .is_some_and(|frame| frame.placement.is_anchored());
        let (top, left, _) = block_at(&at.table, at.raw.rows.start, at.raw.columns.start);
        let command = Command::EditTable {
            frame: at.frame,
            edit: TableEdit::PasteCells {
                at: (top, left),
                block,
                area: at.selected.then(|| at.raw.clone()),
                add_rows: page_table,
            },
        };
        let session = &mut self.sessions[self.active_session];
        match amipro_editing::apply(&session.document, &command) {
            Ok(_) => {
                session.execute_page_command(&command);
                session.status = "Cells pasted".into();
            }
            Err(reason) => {
                session.status = format!("Cells not pasted: {reason}");
            }
        }
        true
    }
}

impl AmiProApp {
    /// Selected cells dropped on a cell (`14e0:0000`): whole rows or columns
    /// move (or are copied) to stand before the row or column dropped on,
    /// after it when dragged down or right; a block of cells is moved over
    /// the cells from the one dropped on, never adding rows or columns. One
    /// undo step.
    pub(crate) fn drop_cells(&mut self, frame: FrameId, to: (usize, usize), copy: bool) {
        let Some(at) = self
            .table_at()
            .filter(|at| at.selected && at.frame == frame)
        else {
            return;
        };
        let table = &at.table;
        let range = at.expanded();
        let whole_rows = range.columns == (0..table.columns.len());
        let whole_columns = range.rows == (0..table.rows.len());
        let command = if whole_rows && !whole_columns {
            if range.rows.contains(&to.0) && !copy {
                return;
            }
            let place = if to.0 < range.rows.start {
                to.0
            } else {
                to.0 + 1
            };
            Command::EditTable {
                frame,
                edit: TableEdit::MoveRows {
                    rows: range.rows.clone(),
                    to: place,
                    copy,
                },
            }
        } else if whole_columns && !whole_rows {
            if range.columns.contains(&to.1) && !copy {
                return;
            }
            let place = if to.1 < range.columns.start {
                to.1
            } else {
                to.1 + 1
            };
            Command::EditTable {
                frame,
                edit: TableEdit::MoveColumns {
                    columns: range.columns.clone(),
                    to: place,
                    copy,
                },
            }
        } else if whole_rows && whole_columns {
            // The whole table cannot be dropped in itself.
            return;
        } else {
            if (range.rows.start, range.columns.start) == to {
                return;
            }
            let block = amipro_editing::tables::CellBlock::copy(&self.document, table, &range);
            let paste = Command::EditTable {
                frame,
                edit: TableEdit::PasteCells {
                    at: to,
                    block,
                    area: None,
                    add_rows: false,
                },
            };
            if copy {
                paste
            } else {
                Command::Several(vec![
                    Command::EditTable {
                        frame,
                        edit: TableEdit::ClearCells(range.clone()),
                    },
                    paste,
                ])
            }
        };
        let session = &mut self.sessions[self.active_session];
        if session.execute_page_command(&command) {
            session.status = if copy { "Cells copied" } else { "Cells moved" }.into();
        }
    }
}
