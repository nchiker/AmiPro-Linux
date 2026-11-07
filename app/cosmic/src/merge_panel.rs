//! The Merge panel in the rail (`docs/Design/UI_SPEC.md` §38, MG1, MG5,
//! MG6; laid out as Ami Pro's three steps since the user's MR2): 1 the
//! data file, 2 the merge document (its fields, the preview with a
//! record), 3 the merge (Merge to, and under Options the conditions,
//! labels, and one PDF per record), one step open at a time. Each change
//! to the document is one command and one undo step; the preview changes
//! nothing.

use std::sync::Arc;

use amipro_document::merge::{Condition, Join, Operator};
use cosmic::iced::Length;
use cosmic::widget;

use amipro_editing::merge::labels::{LabelSheet, SHEETS};

use crate::Message;
use crate::merge::MergeData;

pub const WIDTH: f32 = 340.0;

type Element<'a> = cosmic::Element<'a, Message>;

#[derive(Clone, Debug)]
pub enum MergeAction {
    Close,
    /// A step's title clicked: that step open (MR2).
    Step(Step),
    /// Step 3's Options ▸ folded open or shut.
    Options(bool),
    /// Open… a data file (the desktop's dialog).
    Open,
    /// The file chosen.
    Chosen(std::path::PathBuf),
    /// Inserts `MergeField NAME` at the caret.
    Insert(String),
    /// Inserts MergeRec, NextRec, or a Skip if… field.
    InsertMergeRec,
    InsertNextRec,
    InsertSkip,
    /// Preview: the chosen record's values in place.
    Preview(bool),
    Previous,
    Next,
    /// Conditions: a new row, a row's parts, a row removed.
    AddCondition,
    RemoveCondition(usize),
    /// A row's field, by its place among the data file's fields.
    ConditionField(usize, usize),
    ConditionOperator(usize, Operator),
    ConditionValue(usize, String),
    CommitValue(usize),
    ConditionJoin(usize, Join),
    WithConditions(bool),
    /// Merge to: a new tab with every record.
    ToNewDocument,
    /// Merge to: one PDF (the desktop's dialog).
    ToPdf,
    ToPdfChosen(std::path::PathBuf),
    /// Merge to: Ami Pro's Merge & save as (the desktop's dialog).
    ToAmiProFile,
    ToAmiProFileChosen(std::path::PathBuf),
    /// One PDF per record, named from a field (MG4): on, and the field.
    PdfEach(bool),
    PdfNameField(usize),
    /// The folder the PDFs go to.
    ToPdfFolderChosen(std::path::PathBuf),
    /// Merge to: the printer (the desktop's print dialog once).
    ToPrinter,
    /// Merge, view & print (MG4): each record in the document in turn.
    ViewAndPrint,
    PrintAndViewNext,
    SkipAndViewNext,
    PrintAll,
    CancelViewing,
    /// New Data File… (MG3): the names box, and the file to write.
    NewDataFile,
    NewDataNames(String),
    CreateDataFile,
    CancelNewData,
    NewDataFileChosen(std::path::PathBuf),
    /// Edit records… (MG3): the data file's tab as a grid.
    EditRecords,
    /// Labels (MG7): Merge as labels, the sheet, the start label, copies.
    Labels(bool),
    LabelSheet(usize),
    LabelStart(usize),
    LabelCopies(String),
    /// Start from (MG11): a merge letter, labels, or an envelope.
    StartFrom(StartFrom),
    /// One PDF per record written (how many), or why not.
    PdfsWritten {
        session_id: u64,
        result: Result<usize, String>,
    },
}

/// Ami Pro's three merge steps (MRGWELCOME's; MR2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    DataFile,
    MergeDocument,
    Merge,
}

/// MG11's merge sheets, until File › New's chooser gives them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartFrom {
    Letter,
    Labels,
    Envelope,
}

/// What the panel keeps between messages.
#[derive(Clone, Debug, Default)]
pub struct MergePanel {
    pub data: Option<Arc<MergeData>>,
    /// The data file's field names as merge fields name them.
    pub keys: Vec<String>,
    pub reading: bool,
    /// The preview's record, among those the conditions select.
    pub position: usize,
    /// Condition values as typed, until committed.
    pub values: Vec<String>,
    pub message: Option<String>,
    /// One PDF per record, named from this field (its place among the
    /// data file's fields).
    pub pdf_each: bool,
    pub pdf_name_field: Option<usize>,
    /// New Data File…'s names as typed, while its box is open.
    pub new_names: Option<String>,
    /// The sheet chosen in the list (its place in `SHEETS`).
    pub sheet: Option<usize>,
    /// The first label used on the first sheet (1 for the sheet's first).
    pub start: usize,
    /// Each label N times, as typed.
    pub copies: String,
    /// Merge to Printer… waits for the desktop's print dialog.
    pub printing: bool,
    /// Opened by File › Merge… (not only kept behind the Envelope panel
    /// for its data file).
    pub shown: bool,
    /// The step open; `None` for the one the merge has reached
    /// ([`Shown::step`]).
    pub step: Option<Step>,
    /// Step 3's Options open.
    pub options: bool,
    /// The merge fields the merge document holds, counted when it changes
    /// (the view asks every frame).
    pub fields: usize,
    pub fields_of: Option<amipro_document::Document>,
}

impl MergePanel {
    /// Print each label N times: 1 to 99.
    pub fn copies(&self) -> usize {
        self.copies
            .trim()
            .parse::<usize>()
            .unwrap_or(1)
            .clamp(1, 99)
    }

    pub fn start(&self) -> usize {
        self.start.max(1)
    }
}

fn send(action: MergeAction) -> Message {
    Message::Merge(action)
}

fn header() -> Element<'static> {
    widget::row::with_children(vec![
        widget::text::heading("Merge").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Merge")
            .on_press(send(MergeAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn wide(label: &str, message: Option<Message>) -> Element<'static> {
    widget::button::custom(
        widget::container(widget::text::body(label.to_owned())).center_x(Length::Fill),
    )
    .class(cosmic::theme::Button::Standard)
    .width(Length::Fill)
    .on_press_maybe(message)
    .into()
}

/// What the panel shows besides its own state.
pub struct Shown<'a> {
    pub panel: &'a MergePanel,
    /// The data file as `[recfile]` names it.
    pub stored: Option<&'a str>,
    pub conditions: &'a [Condition],
    pub with_conditions: bool,
    /// The records the conditions select.
    pub selected: usize,
    pub preview: bool,
    /// The document's merge fields the data file lacks (MGM8).
    pub unknown: Vec<String>,
    /// Merge as labels: `[recfile]`'s As labels, and the sheet it matches.
    pub labels: Option<LabelSheet>,
    /// Merge, view & print: the record shown, of how many, and how many
    /// are to print.
    pub viewing: Option<(usize, usize, usize)>,
    /// The merge fields the document holds.
    pub fields: usize,
}

impl Shown<'_> {
    /// The step open: the one chosen, else the one the merge has reached
    /// (Ami Pro preselects step 1 without a data file): 1 without a data
    /// file, 2 while the document has no merge field, 3 after; 3 while
    /// viewing.
    pub fn step(&self) -> Step {
        if self.viewing.is_some() {
            return Step::Merge;
        }
        self.panel.step.unwrap_or(if self.panel.data.is_none() {
            Step::DataFile
        } else if self.fields == 0 {
            Step::MergeDocument
        } else {
            Step::Merge
        })
    }
}

const OPERATORS: [&str; 6] = ["=", "<", ">", "!=", "<=", ">="];

/// A step's title: its number, name, and summary; a click opens it.
fn step_title(
    number: &str,
    name: &str,
    summary: String,
    open: bool,
    step: Step,
) -> Element<'static> {
    widget::button::custom(
        widget::row::with_children(vec![
            widget::text::heading(format!("{number}  {name}")).into(),
            widget::space::horizontal().into(),
            widget::text::caption(summary)
                .wrapping(cosmic::iced::widget::text::Wrapping::None)
                .into(),
            widget::text::body(if open { "▾" } else { "▸" }).into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center),
    )
    .class(if open {
        cosmic::theme::Button::Standard
    } else {
        cosmic::theme::Button::Text
    })
    .padding([8, 10])
    .width(Length::Fill)
    .on_press(send(MergeAction::Step(step)))
    .into()
}

/// A step's body, set in under its title.
fn step_body<'a>(children: Vec<Element<'a>>) -> Element<'a> {
    widget::container(widget::column::with_children(children).spacing(10))
        .padding([4, 4, 8, 12])
        .width(Length::Fill)
        .into()
}

/// Buttons that wrap onto as many lines as the panel needs.
fn button_rows<'a>(buttons: Vec<Element<'a>>) -> Element<'a> {
    widget::row::with_children(buttons)
        .spacing(6)
        .wrap()
        .vertical_spacing(6)
        .into()
}

fn standard(label: &str, message: Option<Message>) -> Element<'static> {
    widget::button::standard(label.to_owned())
        .on_press_maybe(message)
        .into()
}

pub fn panel<'a>(shown: Shown<'a>) -> Element<'a> {
    let state = shown.panel;
    let data = state.data.as_deref();
    let step = shown.step();
    let count = shown.selected;
    let total = data.map_or(0, |data| data.table.records.len());
    let mut body: Vec<Element<'a>> = Vec::new();

    // 1 Data file.
    let file_name = data.map(|data| {
        data.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    body.push(step_title(
        "1",
        "Data file",
        match (&file_name, state.reading) {
            (_, true) => "Reading…".into(),
            (Some(name), _) => name.clone(),
            (None, _) => "None yet".into(),
        },
        step == Step::DataFile,
        Step::DataFile,
    ));
    if step == Step::DataFile {
        let about = match (data, shown.stored, state.reading) {
            (_, _, true) => "Reading…".to_string(),
            (Some(data), _, _) => format!(
                "{} · {}{} · {} records · {} fields",
                file_name.clone().unwrap_or_default(),
                data.kind.name(),
                if data.kind.editable() {
                    ""
                } else {
                    " (read only)"
                },
                data.table.records.len(),
                data.table.names.len()
            ),
            (None, Some(stored), _) if !stored.trim().is_empty() => {
                format!("{} (not found)", stored.trim())
            }
            _ => {
                "Choose the file that holds the names and addresses, or make a new one.".to_string()
            }
        };
        let mut children: Vec<Element<'a>> = vec![widget::text::body(about).into()];
        children.push(wide("Open…", Some(send(MergeAction::Open))));
        children.push(wide("New Data File…", Some(send(MergeAction::NewDataFile))));
        if let Some(names) = &state.new_names {
            children.push(
                widget::text::caption("The new data file's field names, separated by commas:")
                    .into(),
            );
            children.push(
                widget::text_input("Title, First, Last, Company, City", names.clone())
                    .on_input(|text| send(MergeAction::NewDataNames(text)))
                    .on_submit(|_| send(MergeAction::CreateDataFile))
                    .width(Length::Fill)
                    .into(),
            );
            children.push(button_rows(vec![
                widget::button::suggested("Create…")
                    .on_press(send(MergeAction::CreateDataFile))
                    .into(),
                standard("Cancel", Some(send(MergeAction::CancelNewData))),
            ]));
        }
        children.push(wide(
            "Edit Records",
            data.filter(|data| data.kind.editable())
                .map(|_| send(MergeAction::EditRecords)),
        ));
        children.push(
            widget::text::caption(
                "Edit Records opens the data file as a table: type in the cells; Tab in the last cell adds a record.",
            )
            .into(),
        );
        body.push(step_body(children));
    }

    // 2 Merge document.
    body.push(step_title(
        "2",
        "Merge document",
        match shown.fields {
            0 => "No fields yet".into(),
            1 => "1 field".into(),
            fields => format!("{fields} fields"),
        },
        step == Step::MergeDocument,
        Step::MergeDocument,
    ));
    if step == Step::MergeDocument {
        let mut children: Vec<Element<'a>> = Vec::new();
        if data.is_some() {
            children.push(
                widget::text::caption("Click a field to put it in the document at the caret:")
                    .into(),
            );
            children.push(button_rows(
                state
                    .keys
                    .iter()
                    .map(|name| standard(name, Some(send(MergeAction::Insert(name.clone())))))
                    .collect(),
            ));
        } else {
            children
                .push(widget::text::caption("Choose a data file first to see its fields.").into());
        }
        children.push(widget::text::caption("Record numbers and skipping:").into());
        children.push(button_rows(vec![
            standard("MergeRec", Some(send(MergeAction::InsertMergeRec))),
            standard("NextRec", Some(send(MergeAction::InsertNextRec))),
            standard("Skip if…", Some(send(MergeAction::InsertSkip))),
        ]));
        if !shown.unknown.is_empty() {
            children.push(
                widget::text::caption(format!(
                    "Not in the data file, so they merge empty: {}.",
                    shown.unknown.join(", ")
                ))
                .into(),
            );
        }
        let position = state.position.min(count.saturating_sub(1));
        children.push(
            widget::row::with_children(vec![
                widget::checkbox(shown.preview)
                    .label("Preview a record")
                    .on_toggle(|on| send(MergeAction::Preview(on)))
                    .into(),
                widget::space::horizontal().into(),
                standard(
                    "◀",
                    (shown.preview && position > 0).then(|| send(MergeAction::Previous)),
                ),
                widget::text::body(if count == 0 {
                    "0 of 0".to_string()
                } else {
                    format!("{} of {count}", position + 1)
                })
                .into(),
                standard(
                    "▶",
                    (shown.preview && position + 1 < count).then(|| send(MergeAction::Next)),
                ),
            ])
            .spacing(6)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
        children.push(widget::text::caption("Or start a new merge document from:").into());
        children.push(button_rows(vec![
            standard(
                "Letter",
                Some(send(MergeAction::StartFrom(StartFrom::Letter))),
            ),
            standard(
                "Labels",
                Some(send(MergeAction::StartFrom(StartFrom::Labels))),
            ),
            standard(
                "Envelope",
                Some(send(MergeAction::StartFrom(StartFrom::Envelope))),
            ),
        ]));
        body.push(step_body(children));
    }

    // 3 Merge.
    body.push(step_title(
        "3",
        "Merge",
        if data.is_some() {
            format!("{count} of {total} records")
        } else {
            String::new()
        },
        step == Step::Merge,
        Step::Merge,
    ));
    if step == Step::Merge {
        let mut children: Vec<Element<'a>> = Vec::new();
        let ready = data.is_some() && count > 0;
        match shown.viewing {
            Some((position, of, to_print)) => {
                children.push(
                    widget::text::body(format!(
                        "Viewing record {} of {of} · {to_print} to print",
                        position + 1
                    ))
                    .into(),
                );
                children.push(wide(
                    "Print and View Next",
                    Some(send(MergeAction::PrintAndViewNext)),
                ));
                children.push(wide(
                    "Skip and View Next",
                    Some(send(MergeAction::SkipAndViewNext)),
                ));
                children.push(wide("Print All", Some(send(MergeAction::PrintAll))));
                children.push(wide("Cancel", Some(send(MergeAction::CancelViewing))));
                children.push(
                    widget::text::caption(
                        "Edits made while viewing carry to later records. The records to print go to the printer together at the end.",
                    )
                    .into(),
                );
            }
            None => {
                children.push(
                    widget::text::body(if data.is_none() {
                        "Choose a data file in step 1 first.".to_string()
                    } else {
                        format!("{count} of {total} records merge. Merge to:")
                    })
                    .into(),
                );
                children.push(wide(
                    "New Document",
                    ready.then(|| send(MergeAction::ToNewDocument)),
                ));
                children.push(wide(
                    if state.pdf_each {
                        "PDFs, One per Record…"
                    } else {
                        "PDF…"
                    },
                    ready.then(|| send(MergeAction::ToPdf)),
                ));
                children.push(wide(
                    "Printer…",
                    (ready && !state.printing).then(|| send(MergeAction::ToPrinter)),
                ));
                children.push(wide(
                    "View & Print",
                    (ready && shown.labels.is_none()).then(|| send(MergeAction::ViewAndPrint)),
                ));
                // As labels dims Ami Pro's view and save choices.
                children.push(wide(
                    "Ami Pro File…",
                    (ready && shown.labels.is_none()).then(|| send(MergeAction::ToAmiProFile)),
                ));
                children.push(
                    widget::button::custom(
                        widget::row::with_children(vec![
                            widget::text::heading("Options").into(),
                            widget::space::horizontal().into(),
                            widget::text::caption(options_summary(&shown)).into(),
                            widget::text::body(if state.options { "▾" } else { "▸" }).into(),
                        ])
                        .spacing(8)
                        .align_y(cosmic::iced::Alignment::Center),
                    )
                    .class(cosmic::theme::Button::Text)
                    .padding([6, 4])
                    .width(Length::Fill)
                    .on_press(send(MergeAction::Options(!state.options)))
                    .into(),
                );
                if state.options {
                    children.extend(options(&shown, count, total));
                }
            }
        }
        body.push(step_body(children));
    }
    // The last message first, where it is seen (a refusal at the panel's
    // foot went unnoticed in the user's first use).
    if let Some(message) = &state.message {
        body.insert(0, widget::text::body(message.clone()).into());
    }
    widget::column::with_children(vec![
        header(),
        crate::rail_scroller(widget::column::with_children(body).spacing(6).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

/// What Options holds that is on, for its title.
fn options_summary(shown: &Shown<'_>) -> String {
    let mut on: Vec<&str> = Vec::new();
    if shown.with_conditions && !shown.conditions.is_empty() {
        on.push("conditions");
    }
    if shown.labels.is_some() {
        on.push("labels");
    }
    if shown.panel.pdf_each {
        on.push("PDF per record");
    }
    on.join(", ")
}

/// Step 3's Options: the conditions (MG6), Merge as labels (MG7), and one
/// PDF per record (MG4).
fn options<'a>(shown: &Shown<'a>, count: usize, total: usize) -> Vec<Element<'a>> {
    let state = shown.panel;
    let data = state.data.as_deref();
    let mut children: Vec<Element<'a>> = Vec::new();
    children.push(
        widget::row::with_children(vec![
            widget::checkbox(shown.with_conditions)
                .label("With conditions")
                .on_toggle(|on| send(MergeAction::WithConditions(on)))
                .into(),
            widget::space::horizontal().into(),
            widget::text::caption(format!("{count} of {total} match")).into(),
        ])
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    );
    let names = &state.keys;
    for (index, condition) in shown.conditions.iter().enumerate() {
        let field = names
            .iter()
            .position(|name| amipro_editing::merge::data::field_key(&condition.field) == *name);
        let operator = OPERATORS
            .iter()
            .position(|symbol| *symbol == condition.operator.symbol());
        let value = state
            .values
            .get(index)
            .cloned()
            .unwrap_or_else(|| condition.value.clone());
        children.push(
            widget::row::with_children(vec![
                widget::text::body("If").into(),
                widget::dropdown(names.as_slice(), field, move |chosen| {
                    send(MergeAction::ConditionField(index, chosen))
                })
                .width(Length::Fill)
                .into(),
                widget::dropdown(&OPERATORS, operator, move |chosen| {
                    send(MergeAction::ConditionOperator(index, Operator::ALL[chosen]))
                })
                .width(Length::Fixed(64.0))
                .into(),
                widget::button::icon(widget::icon::from_name("list-remove-symbolic"))
                    .name("Remove condition")
                    .on_press(send(MergeAction::RemoveCondition(index)))
                    .into(),
            ])
            .spacing(6)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
        children.push(
            widget::row::with_children(vec![
                widget::text_input("value", value)
                    .on_input(move |text| send(MergeAction::ConditionValue(index, text)))
                    .on_submit(move |_| send(MergeAction::CommitValue(index)))
                    .width(Length::Fill)
                    .into(),
                standard(
                    if condition.join == Join::And {
                        "And"
                    } else {
                        "Or"
                    },
                    Some(send(MergeAction::ConditionJoin(
                        index,
                        if condition.join == Join::And {
                            Join::Or
                        } else {
                            Join::And
                        },
                    ))),
                ),
            ])
            .spacing(6)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
    }
    children.push(standard(
        "+ Condition",
        data.map(|_| send(MergeAction::AddCondition)),
    ));
    children.push(widget::divider::horizontal::light().into());
    children.push(
        widget::checkbox(shown.labels.is_some())
            .label("Merge as labels")
            .on_toggle(|on| send(MergeAction::Labels(on)))
            .into(),
    );
    if let Some(sheet) = &shown.labels {
        let names: &'static [String] = sheet_names();
        let chosen = state
            .sheet
            .or_else(|| SHEETS.iter().position(|listed| listed == sheet));
        children.push(
            widget::dropdown(names, chosen, |chosen| {
                send(MergeAction::LabelSheet(chosen))
            })
            .width(Length::Fill)
            .into(),
        );
        if chosen.is_none() {
            children.push(widget::text::caption("Custom: the page is one label.").into());
        }
        children.push(widget::text::caption("Click the label to start at:").into());
        children.push(sheet_picker(sheet, state.start()));
        children.push(
            widget::row::with_children(vec![
                widget::text::body(format!("Start at label {}", state.start())).into(),
                widget::space::horizontal().into(),
                widget::text::body("Copies").into(),
                widget::text_input("1", state.copies.clone())
                    .on_input(|text| send(MergeAction::LabelCopies(text)))
                    .width(Length::Fixed(56.0))
                    .into(),
            ])
            .spacing(6)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
    }
    children.push(widget::divider::horizontal::light().into());
    children.push(
        widget::checkbox(state.pdf_each)
            .label("One PDF per record")
            .on_toggle(|on| send(MergeAction::PdfEach(on)))
            .into(),
    );
    if state.pdf_each {
        children.push(
            widget::row::with_children(vec![
                widget::text::body("Named by").into(),
                widget::dropdown(state.keys.as_slice(), state.pdf_name_field, |chosen| {
                    send(MergeAction::PdfNameField(chosen))
                })
                .width(Length::Fill)
                .into(),
            ])
            .spacing(6)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
    }
    children
}

/// The sheet list's names (MG7), made once.
fn sheet_names() -> &'static [String] {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| SHEETS.iter().map(LabelSheet::name).collect())
}

/// The sheet drawn small: a click on a label makes it the first one used.
fn sheet_picker(sheet: &LabelSheet, start: usize) -> Element<'static> {
    let across = sheet.across.clamp(1, 8) as usize;
    let down = sheet.down.clamp(1, 20) as usize;
    let size = (240.0 / across as f32).min(14.0 * 2.0);
    let height =
        (size * sheet.pitch_down as f32 / sheet.pitch_across.max(1) as f32).clamp(6.0, 24.0);
    let mut rows: Vec<Element<'static>> = Vec::new();
    for row in 0..down {
        let mut cells: Vec<Element<'static>> = Vec::new();
        for column in 0..across {
            let label = row * across + column + 1;
            let used = label >= start;
            cells.push(
                widget::button::custom(
                    widget::space()
                        .width(Length::Fixed(size - 4.0))
                        .height(Length::Fixed(height - 4.0)),
                )
                .class(if label == start {
                    cosmic::theme::Button::Suggested
                } else if used {
                    cosmic::theme::Button::Standard
                } else {
                    cosmic::theme::Button::Text
                })
                .padding(1)
                .on_press(send(MergeAction::LabelStart(label)))
                .into(),
            );
        }
        rows.push(widget::row::with_children(cells).spacing(2).into());
    }
    widget::column::with_children(rows).spacing(2).into()
}
