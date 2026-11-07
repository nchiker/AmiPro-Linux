//! Tools › Sort… on the page surface (`UI_SPEC.md` §41, TN3; run C,
//! TD.8): what the panel sorts (the selected paragraphs, the whole text
//! when nothing is selected, or a table's rows), the command it runs, and
//! what it shows afterwards. One undo step; nothing is modal.

use std::ops::Range;

use amipro_document::FlowId;
use amipro_editing::sort::{
    SORT_ACROSS_BREAK, SORT_CONNECTED, SORT_FOOTNOTES, SORT_NOTHING, SortSettings,
};
use amipro_editing::tables::{FORMULA_IN_SORT, TableEdit};
use amipro_editing::{Command, TextPosition};
use cosmic::iced::Task;

use crate::page_editing::{self, PageSelection};
use crate::sort_panel::{Ask, Notice, Shown, SortAction, SortPanel};
use crate::{AmiProApp, Message};

/// What a sort acts on.
#[derive(Clone, Debug, PartialEq)]
pub enum Scope {
    /// Paragraphs `first..=last` of `flow`; `whole` when nothing was
    /// selected (Ami Pro asks "sort the entire stream?").
    Text {
        flow: FlowId,
        first: usize,
        last: usize,
        whole: bool,
    },
    /// A table's rows: the selected ones, or (`None`) all below the
    /// headings.
    Rows {
        frame: amipro_document::FrameId,
        rows: Option<Range<usize>>,
    },
    Nothing,
}

impl AmiProApp {
    /// What the panel would sort now, and the line that says so.
    pub(crate) fn sort_scope(&self) -> (Scope, Shown) {
        let nothing = |text: &str, in_table| {
            (
                Scope::Nothing,
                Shown {
                    scope: text.into(),
                    can_sort: false,
                    in_table,
                },
            )
        };
        if !self.page_surface || self.frame_selection.is_some() {
            return nothing("Select text, or put the caret in a table.", false);
        }
        if let Some(at) = self.table_at() {
            let count = at.table.rows.len();
            let selected_cells = at.raw.rows.len() * at.raw.columns.len();
            if at.selected && selected_cells > 1 {
                let rows = at.rows();
                if rows.len() < 2 {
                    return nothing(
                        "Select two or more rows, or put the caret in the table to sort it all.",
                        true,
                    );
                }
                let text = format!("Sorting table rows {}–{}", rows.start + 1, rows.end);
                return (
                    Scope::Rows {
                        frame: at.frame,
                        rows: Some(rows),
                    },
                    Shown {
                        scope: text,
                        can_sort: true,
                        in_table: true,
                    },
                );
            }
            let first = (0..count)
                .find(|row| !at.table.row_options.headings.contains(row))
                .unwrap_or(count);
            if count - first < 2 {
                return nothing("The table has fewer than two rows to sort.", true);
            }
            let text = if first == 0 {
                format!("Sorting the whole table ({count} rows)")
            } else {
                format!(
                    "Sorting the whole table ({} rows below the headings)",
                    count - first
                )
            };
            return (
                Scope::Rows {
                    frame: at.frame,
                    rows: None,
                },
                Shown {
                    scope: text,
                    can_sort: true,
                    in_table: true,
                },
            );
        }
        let selection = self.page_selection;
        if selection.anchor.flow != selection.focus.flow {
            return nothing("Select text, or put the caret in a table.", false);
        }
        let flow = selection.focus.flow;
        let Some(length) = self.document.flow(flow).map(|flow| flow.paragraphs.len()) else {
            return nothing("Select text, or put the caret in a table.", false);
        };
        let (start, end) = selection.ordered();
        let (first, last, whole) = if selection.is_caret() {
            (0, length - 1, true)
        } else {
            let last = if end.byte == 0 && end.paragraph > start.paragraph {
                end.paragraph - 1
            } else {
                end.paragraph
            };
            (start.paragraph, last, false)
        };
        let count = last + 1 - first;
        if count < 2 {
            return nothing("Select two or more paragraphs to sort.", false);
        }
        let text = if whole {
            format!("Nothing is selected: sorting all {count} paragraphs of the text")
        } else {
            format!("Sorting {count} paragraphs")
        };
        (
            Scope::Text {
                flow,
                first,
                last,
                whole,
            },
            Shown {
                scope: text,
                can_sort: true,
                in_table: false,
            },
        )
    }

    /// Tools › Sort…: the panel, with the settings of the last sort.
    pub(crate) fn open_sort(&mut self) {
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.sort_panel = Some(SortPanel {
            form: session.sort_form.clone(),
            notice: None,
        });
        self.open_rail();
    }

    pub(crate) fn close_sort(&mut self) {
        let session = &mut self.sessions[self.active_session];
        if let Some(panel) = session.sort_panel.take() {
            session.sort_form = panel.form;
            if self.rail_open() {
                self.toggle_rail();
            }
        }
    }

    pub(crate) fn update_sort(&mut self, action: SortAction) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        let Some(panel) = session.sort_panel.as_mut() else {
            return Task::none();
        };
        match action {
            SortAction::Close => self.close_sort(),
            SortAction::Sort => self.run_sort(false),
            SortAction::Continue => self.run_sort(true),
            SortAction::CancelAsk => panel.notice = None,
            other => {
                let form = &mut panel.form;
                panel.notice = None;
                match other {
                    SortAction::Field(level, text) => form.levels[level].field = text,
                    SortAction::Kind(level, kind) => form.levels[level].kind = kind,
                    SortAction::Word(level, word) => form.levels[level].word = word,
                    SortAction::Descending(on) => form.descending = on,
                    SortAction::TabDelimited(on) => form.tab_delimited = on,
                    SortAction::Delimiter(text) => {
                        form.delimiter = text.chars().take(1).collect();
                        form.tab_delimited = false;
                    }
                    SortAction::Record(text) => form.record = text,
                    _ => {}
                }
                session.sort_form = form.clone();
            }
        }
        Task::none()
    }

    /// Sort: the command for the panel's scope and settings, refused or
    /// asked about first without touching the document.
    fn run_sort(&mut self, confirmed: bool) {
        let (scope, shown) = self.sort_scope();
        let session = &mut self.sessions[self.active_session];
        let Some(panel) = session.sort_panel.as_mut() else {
            return;
        };
        session.sort_form = panel.form.clone();
        let settings = match panel.form.settings(shown.in_table) {
            Ok(settings) => SortSettings {
                confirmed,
                ..settings
            },
            Err(text) => {
                panel.notice = Some(Notice::Refused(text));
                return;
            }
        };
        let command = match &scope {
            Scope::Text {
                flow, first, last, ..
            } => Command::SortParagraphs {
                flow: *flow,
                first: *first,
                last: *last,
                settings,
            },
            Scope::Rows { frame, rows } => Command::EditTable {
                frame: *frame,
                edit: TableEdit::SortRecords {
                    rows: rows.clone(),
                    settings,
                },
            },
            Scope::Nothing => {
                panel.notice = Some(Notice::Refused(shown.scope));
                return;
            }
        };
        if let Err(error) = amipro_editing::apply(&session.document, &command) {
            panel.notice = Some(match error {
                SORT_ACROSS_BREAK => Notice::Ask(Ask::Break),
                SORT_CONNECTED => Notice::Ask(Ask::Connected),
                SORT_NOTHING => Notice::Info("Already in that order.".into()),
                FORMULA_IN_SORT => Notice::Refused(
                    "A cell in the rows holds a formula, so they cannot be sorted.".into(),
                ),
                SORT_FOOTNOTES => {
                    Notice::Refused("A table with footnotes cannot be sorted.".into())
                }
                other => Notice::Refused(format!("Not sorted: {other}.")),
            });
            return;
        }
        let page = session.page_selection.page;
        if !session.execute_page_command(&command) {
            return;
        }
        let done = match &scope {
            Scope::Text {
                flow, first, last, ..
            } => {
                // The sorted paragraphs stay selected, as in Ami Pro.
                let end = session
                    .document
                    .flow(*flow)
                    .and_then(|flow| flow.paragraphs.get(*last))
                    .map_or(0, |paragraph| paragraph.text.len());
                let at = |paragraph, byte| TextPosition {
                    flow: *flow,
                    paragraph,
                    byte,
                };
                session.page_selection = page_editing::valid_selection(
                    &session.document,
                    PageSelection {
                        anchor: at(*first, 0),
                        focus: at(*last, end),
                        page,
                    },
                );
                session.sync_editor_to_page();
                format!("Sorted {} paragraphs", last + 1 - first)
            }
            Scope::Rows { frame, rows } => {
                let first = rows.as_ref().map_or(0, |rows| rows.start);
                if let Some(flow) = crate::table_editing::table_of(&session.document, *frame)
                    .and_then(|table| table.cells.get(first * table.columns.len()))
                    .map(|cell| {
                        let amipro_document::CellContent::Text(flow) = cell.content;
                        flow
                    })
                {
                    session.page_selection = page_editing::valid_selection(
                        &session.document,
                        PageSelection::caret(
                            TextPosition {
                                flow,
                                paragraph: 0,
                                byte: 0,
                            },
                            page,
                        ),
                    );
                    session.sync_editor_to_page();
                }
                session.report_formulas(*frame, None);
                "Rows sorted".to_owned()
            }
            Scope::Nothing => String::new(),
        };
        session.status.clone_from(&done);
        if let Some(panel) = session.sort_panel.as_mut() {
            panel.notice = Some(Notice::Info(done));
        }
    }
}
