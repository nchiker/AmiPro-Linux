//! The Fields panel in the context rail (`UI_SPEC.md` §28, decision FL2):
//! inserting dates and times (Ami Pro's Insert Date/Time), Doc Info entries
//! (Insert Doc Info Field), and power fields (Insert Power Fields), and
//! changing the field at the caret, the document live beside it. Each
//! insertion and each change is one undo step.

use std::collections::BTreeSet;

use amipro_document::{
    DATE_FORMATS, DateField, DateSource, DocInfoEntry, DocInfoField, FieldContext, InlineObject,
    PowerMark, format_date, power_flag,
};
use amipro_editing::TextPosition;
use amipro_editing::fields::FieldAt;
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;

/// The Insert section's pages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    DateTime,
    DocInfo,
    Power,
    /// Opened on the field at the caret (Edit Field…).
    ThisField,
}

/// Ami Pro's Insert Date/Time choices (`INSVARDLGPROC`).
pub const DATE_KINDS: [&str; 5] = [
    "Today's date (as text)",
    "System date (updates)",
    "System time (updates)",
    "Date of last revision",
    "Date created",
];

/// Ami Pro's power fields as its Insert Power Fields list has them
/// (MACTOKEN.SAM's class 125, `IF`, `Call`, and the starred functions):
/// the name, what choosing it puts in the code, a syntax line in our words,
/// and whether the app works it out (FL1).
pub const POWER_FIELDS: [(&str, &str, &str, bool); 41] = [
    ("Bookmark", "", "A bookmark's name: its text", false),
    (
        "Button",
        "Button \"Text\" ",
        "Button \"text\" action: shows the text",
        true,
    ),
    (
        "Call",
        "Call ",
        "Call file!macro(): runs a macro (not here)",
        false,
    ),
    (
        "CreateDate",
        "CreateDate",
        "CreateDate [%D… or %T…]: when the document was created",
        true,
    ),
    (
        "Decide",
        "Decide(\"\")",
        "Decide(\"question\"): asks yes or no when updated",
        true,
    ),
    (
        "Defined",
        "Defined ",
        "Defined name: 1 if the name is a variable or a bookmark",
        true,
    ),
    (
        "Description",
        "Description",
        "Description: the Doc Info description",
        true,
    ),
    (
        "EditDate",
        "EditDate",
        "EditDate [%D… or %T…]: when the document was last saved",
        true,
    ),
    (
        "Exec",
        "Exec(\"\")",
        "Exec(program, …): runs a program (not here)",
        false,
    ),
    (
        "FileSize",
        "FileSize",
        "FileSize: the document's size in K",
        true,
    ),
    (
        "FormatNum$",
        "FormatNum$(\"\",\"\",2,)",
        "FormatNum$(before, after, decimals, number)",
        true,
    ),
    (
        "IF",
        "IF  ENDIF",
        "IF condition result [ELSEIF …] [ELSE result] ENDIF",
        true,
    ),
    (
        "Include",
        "Include \"\"",
        "Include \"file\": another document's text, read when updated",
        true,
    ),
    (
        "Index",
        "Index \"\"",
        "Index \"entry\" [# \"subentry\"]: an index entry (no result)",
        true,
    ),
    (
        "LCase$",
        "LCase$(\"\")",
        "LCase$(text): in small letters",
        true,
    ),
    (
        "Left$",
        "Left$(\"\",)",
        "Left$(text, count): the first characters",
        true,
    ),
    (
        "Len",
        "Len(\"\")",
        "Len(text): the number of characters",
        true,
    ),
    (
        "MergeField",
        "MergeField ",
        "MergeField name: a merge value",
        true,
    ),
    (
        "MergeRec",
        "MergeRec",
        "MergeRec: the merge record, 0 outside a merge",
        true,
    ),
    (
        "Message",
        "Message(\"\")",
        "Message(\"text\"): shows a message (not here)",
        false,
    ),
    (
        "Mid$",
        "Mid$(\"\",,)",
        "Mid$(text, start, count): characters from the middle",
        true,
    ),
    (
        "NextRec",
        "NextRec",
        "NextRec: the next merge record",
        false,
    ),
    (
        "Now",
        "Now()",
        "Now() [%D… or %T…]: the date and time now",
        true,
    ),
    (
        "NumChars",
        "NumChars",
        "NumChars: the characters in the document",
        true,
    ),
    ("NumEdits", "NumEdits", "NumEdits: the revisions", true),
    ("NumPages", "NumPages", "NumPages: the pages", true),
    (
        "NumWords",
        "NumWords",
        "NumWords: the words in the document",
        true,
    ),
    ("PageRef", "PageRef ", "PageRef bookmark: its page", false),
    (
        "PrintEscape",
        "PrintEscape \"\"",
        "PrintEscape \"codes\": sent to the printer",
        false,
    ),
    (
        "Query$",
        "Query$(\"\")",
        "Query$(\"question\" [, \"answer\"]): asks for text when updated",
        true,
    ),
    (
        "Right$",
        "Right$(\"\",)",
        "Right$(text, count): the last characters",
        true,
    ),
    (
        "Seq",
        "Seq Name",
        "Seq name [=n] [+n] [-n] [%N…]: numbers in order",
        true,
    ),
    (
        "Set",
        "Set ",
        "Set name value: a variable (no result)",
        true,
    ),
    ("Skip", "Skip", "Skip: skips a merge record", false),
    (
        "StrCat$",
        "StrCat$(\"\",\"\")",
        "StrCat$(text, text, …): joined",
        true,
    ),
    (
        "StrField$",
        "StrField$(\"\",,\",\")",
        "StrField$(text, n, separator): the n-th part",
        true,
    ),
    (
        "TOC",
        "TOC 1 \"\"",
        "TOC level \"text\": a contents entry (no result)",
        true,
    ),
    (
        "TotalEditingTime",
        "TotalEditingTime",
        "TotalEditingTime: minutes of editing",
        true,
    ),
    ("UCase$", "UCase$(\"\")", "UCase$(text): in capitals", true),
    (
        "User",
        "User ",
        "User anything: a place for macros (no result)",
        true,
    ),
    (
        "Void",
        "Void ",
        "Void expression: works it out, shows nothing",
        true,
    ),
];

/// The time formats Insert Power Fields offers (`%T1` to `%T9`).
const TIME_FORMATS: [&str; 9] = [
    "13:05", "1:05 PM", "01:05 PM", "1:05 P", "01:05 P", "1:05 pm", "01:05 pm", "1:05 p", "01:05 p",
];

/// The number styles Insert Power Fields offers for Seq (`%N1` to `%N5`).
const NUMBER_STYLES: [&str; 5] = ["1, 2, 3", "I, II, III", "i, ii, iii", "A, B, C", "a, b, c"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Section {
    Insert,
    ThisField,
}

#[derive(Clone, Debug)]
pub enum FieldsAction {
    Section(Section),
    Page(Page),
    DateKind(usize),
    DateFormat(usize),
    DocEntry(usize),
    Code(String),
    Pick(usize),
    Option(usize),
    Lock(bool),
    AutoRun(bool),
    Insert,
    SaveName(String),
    Save,
    Saved(usize),
    ThisFormat(usize),
    ThisCode(String),
    ThisSubmit,
    ThisLock(bool),
    ThisAutoRun(bool),
    ThisUpdate,
    Back,
    Close,
}

pub enum Outcome {
    Stay,
    Back,
    Close,
    /// Today's date: plain text.
    InsertText,
    InsertObject(InlineObject),
    InsertPower {
        code: String,
        flags: u16,
    },
    SetObject(TextPosition, InlineObject),
    SetMarks(Box<amipro_editing::Command>),
    UpdateThis,
    Save(String, String),
}

/// The panel's state.
pub struct FieldsPanel {
    open: BTreeSet<Section>,
    pub page: Page,
    date_kind: usize,
    date_format: usize,
    doc_entry: usize,
    code: String,
    pick: Option<usize>,
    lock: bool,
    auto_run: bool,
    save_name: String,
    /// The field at the caret, and its code as typed.
    this: Option<FieldAt>,
    this_code: String,
}

pub const WIDTH: f32 = 300.0;

impl FieldsPanel {
    pub fn new(page: Page, at_caret: Option<FieldAt>) -> Self {
        let mut open = BTreeSet::from([Section::Insert]);
        if page == Page::ThisField || at_caret.is_some() {
            open.insert(Section::ThisField);
        }
        let mut panel = Self {
            open,
            page: if page == Page::ThisField {
                Page::Power
            } else {
                page
            },
            // Ami Pro's defaults: Today's date, format a (AMIPRO.INI
            // `datetype`, `datestyle`).
            date_kind: 0,
            date_format: 0,
            doc_entry: 0,
            code: String::new(),
            pick: None,
            lock: false,
            auto_run: false,
            save_name: String::new(),
            this: None,
            this_code: String::new(),
        };
        panel.sync(at_caret);
        panel
    }

    /// After every message: This field follows the caret.
    pub fn sync(&mut self, at_caret: Option<FieldAt>) {
        if at_caret != self.this {
            self.this_code = match &at_caret {
                Some(FieldAt::Power { span, .. }) => span.mark.code.clone(),
                Some(FieldAt::Object {
                    object: InlineObject::Seq(seq),
                    ..
                }) => seq.code.clone(),
                _ => String::new(),
            };
            self.this = at_caret;
        }
    }

    pub fn update(&mut self, action: FieldsAction, saved: &[(String, String)]) -> Outcome {
        match action {
            FieldsAction::Section(section) => {
                if !self.open.remove(&section) {
                    self.open.insert(section);
                }
            }
            FieldsAction::Page(page) => self.page = page,
            FieldsAction::DateKind(kind) => self.date_kind = kind.min(DATE_KINDS.len() - 1),
            FieldsAction::DateFormat(format) => self.date_format = format.min(20),
            FieldsAction::DocEntry(entry) => self.doc_entry = entry.min(21),
            FieldsAction::Code(code) => self.code = code,
            FieldsAction::Pick(index) => {
                if let Some((_, insert, _, _)) = POWER_FIELDS.get(index) {
                    self.pick = Some(index);
                    // Choosing a field writes it into the code (Bookmark
                    // inserts a name where the code is typed).
                    if !insert.is_empty() {
                        self.code = (*insert).into();
                    }
                }
            }
            FieldsAction::Option(index) => self.code = with_option(&self.code, self.pick, index),
            FieldsAction::Lock(on) => self.lock = on,
            FieldsAction::AutoRun(on) => self.auto_run = on,
            FieldsAction::Insert => return self.insertion(),
            FieldsAction::SaveName(name) => self.save_name = name,
            FieldsAction::Save => {
                let name = self.save_name.trim();
                if !name.is_empty() && !self.code.trim().is_empty() {
                    return Outcome::Save(name.into(), self.code.clone());
                }
            }
            FieldsAction::Saved(index) => {
                if let Some((_, code)) = saved.get(index) {
                    self.code = code.clone();
                }
            }
            FieldsAction::ThisFormat(index) => return self.this_format(index),
            FieldsAction::ThisCode(code) => self.this_code = code,
            FieldsAction::ThisSubmit => return self.this_marks(|mark, code| mark.code = code),
            FieldsAction::ThisLock(on) => {
                return self.this_marks(move |mark, _| set_flag(mark, power_flag::LOCK, on));
            }
            FieldsAction::ThisAutoRun(on) => {
                return self.this_marks(move |mark, _| set_flag(mark, power_flag::AUTO_RUN, on));
            }
            FieldsAction::ThisUpdate => return Outcome::UpdateThis,
            FieldsAction::Back => return Outcome::Back,
            FieldsAction::Close => return Outcome::Close,
        }
        Outcome::Stay
    }

    fn insertion(&self) -> Outcome {
        match self.page {
            Page::DateTime => {
                let format = DATE_FORMATS[self.date_format];
                let source = match self.date_kind {
                    0 => return Outcome::InsertText,
                    1 => DateSource::Today,
                    // The time keeps the list's position, as Ami Pro's does.
                    2 => {
                        return Outcome::InsertObject(InlineObject::Date(DateField {
                            source: DateSource::Time,
                            format: self.date_format as u8,
                        }));
                    }
                    3 => DateSource::LastSaved,
                    _ => DateSource::Created,
                };
                Outcome::InsertObject(InlineObject::Date(DateField { source, format }))
            }
            Page::DocInfo => match doc_info_entries().get(self.doc_entry) {
                // Document Description is a power field (`1338:039d`).
                Some(None) => Outcome::InsertPower {
                    code: "Description".into(),
                    flags: 0,
                },
                Some(Some(entry)) => Outcome::InsertObject(InlineObject::DocInfo(DocInfoField {
                    code: entry.code(),
                    format: DATE_FORMATS[self.date_format],
                })),
                None => Outcome::Stay,
            },
            Page::Power | Page::ThisField => {
                if self.code.trim().is_empty() {
                    return Outcome::Stay;
                }
                let mut flags = 0;
                if self.lock {
                    flags |= power_flag::LOCK;
                }
                if self.auto_run {
                    flags |= power_flag::AUTO_RUN;
                }
                Outcome::InsertPower {
                    code: self.code.trim().into(),
                    flags,
                }
            }
        }
    }

    /// Takes another panel's choices (for a preview of what it would
    /// insert).
    pub fn clone_choice_from(&mut self, other: &Self) {
        self.page = other.page;
        self.date_kind = other.date_kind;
        self.date_format = other.date_format;
        self.doc_entry = other.doc_entry;
        self.code = other.code.clone();
        self.pick = other.pick;
    }

    /// Today's date formatted as the date kind would show it now.
    pub fn today_text(&self, context: &FieldContext) -> String {
        format_date(
            &context.now,
            DATE_FORMATS[self.date_format],
            &context.settings,
        )
    }

    fn this_format(&self, index: usize) -> Outcome {
        match &self.this {
            Some(FieldAt::Object {
                at,
                object: InlineObject::Date(date),
            }) if date.source != DateSource::Time => Outcome::SetObject(
                *at,
                InlineObject::Date(DateField {
                    format: DATE_FORMATS[index.min(20)],
                    ..*date
                }),
            ),
            Some(FieldAt::Object {
                at,
                object: InlineObject::DocInfo(entry),
            }) => Outcome::SetObject(
                *at,
                InlineObject::DocInfo(DocInfoField {
                    format: DATE_FORMATS[index.min(20)],
                    ..*entry
                }),
            ),
            _ => Outcome::Stay,
        }
    }

    fn this_marks(&self, change: impl FnOnce(&mut PowerMark, String)) -> Outcome {
        match &self.this {
            Some(FieldAt::Power { flow, span }) => {
                let mut mark = span.mark.clone();
                change(&mut mark, self.this_code.trim().to_owned());
                if mark == span.mark {
                    return Outcome::Stay;
                }
                Outcome::SetMarks(Box::new(amipro_editing::Command::SetFieldMarks {
                    flow: *flow,
                    start: span.start,
                    end: span.end,
                    mark,
                }))
            }
            Some(FieldAt::Object {
                at,
                object: InlineObject::Seq(seq),
            }) => {
                let mut mark = PowerMark {
                    end: false,
                    kind: amipro_document::power_kind::SEQ,
                    value: seq.value,
                    code: seq.code.clone(),
                };
                change(&mut mark, self.this_code.trim().to_owned());
                // A Seq's value is its number: only its code changes (FX).
                Outcome::SetObject(
                    *at,
                    InlineObject::Seq(amipro_document::SeqField {
                        code: mark.code,
                        value: seq.value,
                    }),
                )
            }
            _ => Outcome::Stay,
        }
    }
}

fn set_flag(mark: &mut PowerMark, flag: u16, on: bool) {
    if on {
        mark.value |= flag;
    } else {
        mark.value &= !flag;
    }
}

/// Insert Doc Info Field's 22 entries: the 21 objects and, fourth, the
/// description, which is a power field.
fn doc_info_entries() -> Vec<Option<DocInfoEntry>> {
    let mut entries: Vec<Option<DocInfoEntry>> =
        DocInfoEntry::LISTED.iter().copied().map(Some).collect();
    entries.insert(3, None);
    entries
}

/// The options a field offers (`~1` dates and times, `~2` number styles):
/// labels, and what each adds to the code.
fn options(pick: Option<usize>, context: &FieldContext) -> Vec<(String, String)> {
    let name = pick
        .and_then(|index| POWER_FIELDS.get(index))
        .map(|field| field.0);
    match name {
        Some("CreateDate" | "EditDate" | "Now") => DATE_FORMATS
            .iter()
            .map(|letter| {
                (
                    format_date(&context.now, *letter, &context.settings),
                    format!("%D{}", char::from(*letter)),
                )
            })
            .chain(
                TIME_FORMATS
                    .iter()
                    .enumerate()
                    .map(|(index, label)| ((*label).into(), format!("%T{}", index + 1))),
            )
            .collect(),
        Some("Seq") => NUMBER_STYLES
            .iter()
            .enumerate()
            .map(|(index, label)| ((*label).into(), format!("%N{}", index + 1)))
            .collect(),
        _ => Vec::new(),
    }
}

/// The code with option `index`'s format in place of any it had.
fn with_option(code: &str, pick: Option<usize>, index: usize) -> String {
    let Some((_, format)) = options(pick, &FieldContext::default())
        .into_iter()
        .nth(index)
    else {
        return code.into();
    };
    let base = code.split('%').next().unwrap_or(code).trim_end();
    format!("{base} {format}")
}

fn send(action: FieldsAction) -> Message {
    Message::FieldsPanel(action)
}

fn row(children: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    widget::row::with_children(children)
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

fn label(text: &'static str) -> Element<'static, Message> {
    widget::text::body(text).width(Length::Fixed(64.0)).into()
}

/// What the panel shows besides its state: the context, the previews the
/// app worked out, and the saved fields.
pub struct Shown<'a> {
    pub context: FieldContext,
    /// What the Insert section's choice would show.
    pub preview: String,
    /// What the field at the caret shows.
    pub this_shows: Option<String>,
    pub doc_info: Vec<String>,
    pub saved: &'a [(String, String)],
}

pub fn panel<'a>(state: &FieldsPanel, shown: Shown<'a>) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Fields").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(FieldsAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Fields")
            .on_press(send(FieldsAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut body: Vec<Element<'a, Message>> = Vec::new();
    let folder = |section: Section, title: &'static str| -> Element<'a, Message> {
        let open = state.open.contains(&section);
        widget::button::custom(
            widget::row::with_children(vec![
                widget::icon::from_name(if open {
                    "pan-down-symbolic"
                } else {
                    "pan-end-symbolic"
                })
                .size(16)
                .icon()
                .into(),
                widget::text::heading(title).into(),
            ])
            .spacing(6)
            .align_y(cosmic::iced::Alignment::Center),
        )
        .class(cosmic::theme::Button::Text)
        .padding([4, 0])
        .width(Length::Fill)
        .on_press(send(FieldsAction::Section(section)))
        .into()
    };
    body.push(folder(Section::Insert, "Insert"));
    if state.open.contains(&Section::Insert) {
        let mut rows: Vec<Element<'a, Message>> = vec![page_selector(state.page)];
        match state.page {
            Page::DateTime => {
                rows.push(row(vec![
                    label("Choice"),
                    widget::dropdown(DATE_KINDS.to_vec(), Some(state.date_kind), |kind| {
                        send(FieldsAction::DateKind(kind))
                    })
                    .width(Length::Fill)
                    .into(),
                ]));
                let samples: Vec<String> = DATE_FORMATS
                    .iter()
                    .map(|letter| format_date(&shown.context.now, *letter, &shown.context.settings))
                    .collect();
                let mut format = widget::dropdown(samples, Some(state.date_format), |format| {
                    send(FieldsAction::DateFormat(format))
                })
                .width(Length::Fill);
                if state.date_kind == 2 {
                    // The time has one format, Windows' (Ami Pro dims the list).
                    format = widget::dropdown(vec!["Windows' time".to_owned()], Some(0), |_| {
                        send(FieldsAction::DateFormat(0))
                    })
                    .width(Length::Fill);
                }
                rows.push(row(vec![label("Format"), format.into()]));
            }
            Page::DocInfo => {
                rows.push(
                    widget::dropdown(shown.doc_info.clone(), Some(state.doc_entry), |entry| {
                        send(FieldsAction::DocEntry(entry))
                    })
                    .width(Length::Fill)
                    .into(),
                );
                rows.push(
                    widget::text::caption("Dates take the format chosen under Date and time.")
                        .into(),
                );
            }
            Page::Power | Page::ThisField => {
                rows.push(row(vec![
                    label("Field"),
                    widget::text_input("NumPages", state.code.clone())
                        .on_input(|code| send(FieldsAction::Code(code)))
                        .on_submit(|_| send(FieldsAction::Insert))
                        .width(Length::Fill)
                        .into(),
                ]));
                let names: Vec<String> = POWER_FIELDS
                    .iter()
                    .map(|(name, _, _, runs)| {
                        if *runs {
                            (*name).to_owned()
                        } else {
                            format!("{name} (kept, not run)")
                        }
                    })
                    .collect();
                rows.push(row(vec![
                    label("Fields"),
                    widget::dropdown(names, state.pick, |index| send(FieldsAction::Pick(index)))
                        .width(Length::Fill)
                        .into(),
                ]));
                let offered = options(state.pick, &shown.context);
                if !offered.is_empty() {
                    let labels: Vec<String> = offered.into_iter().map(|(label, _)| label).collect();
                    rows.push(row(vec![
                        label("Options"),
                        widget::dropdown(labels, None, |index| send(FieldsAction::Option(index)))
                            .width(Length::Fill)
                            .into(),
                    ]));
                }
                if let Some((_, _, syntax, _)) =
                    state.pick.and_then(|index| POWER_FIELDS.get(index))
                {
                    rows.push(widget::text::caption(*syntax).into());
                }
                rows.push(row(vec![
                    widget::checkbox(state.lock)
                        .label("Lock")
                        .on_toggle(|on| send(FieldsAction::Lock(on)))
                        .into(),
                    widget::checkbox(state.auto_run)
                        .label("Update when opened")
                        .on_toggle(|on| send(FieldsAction::AutoRun(on)))
                        .into(),
                ]));
                if !shown.saved.is_empty() {
                    let names: Vec<String> =
                        shown.saved.iter().map(|(name, _)| name.clone()).collect();
                    rows.push(row(vec![
                        label("Saved"),
                        widget::dropdown(names, None, |index| send(FieldsAction::Saved(index)))
                            .width(Length::Fill)
                            .into(),
                    ]));
                }
                rows.push(row(vec![
                    widget::text_input("Name to save it under", state.save_name.clone())
                        .on_input(|name| send(FieldsAction::SaveName(name)))
                        .width(Length::Fill)
                        .into(),
                    widget::button::standard("Save")
                        .on_press(send(FieldsAction::Save))
                        .into(),
                ]));
            }
        }
        rows.push(row(vec![
            label("Shows"),
            widget::text::body(shown.preview.clone())
                .width(Length::Fill)
                .into(),
        ]));
        rows.push(
            widget::row::with_children(vec![
                widget::space::horizontal().into(),
                widget::button::suggested("Insert")
                    .on_press(send(FieldsAction::Insert))
                    .into(),
            ])
            .into(),
        );
        body.push(
            widget::container(widget::column::with_children(rows).spacing(6))
                .padding([0, 0, 8, 22])
                .into(),
        );
    }
    body.push(folder(Section::ThisField, "This field"));
    if state.open.contains(&Section::ThisField) {
        let mut rows: Vec<Element<'a, Message>> = Vec::new();
        match &state.this {
            None => rows
                .push(widget::text::caption("Put the caret in a field to change it here.").into()),
            Some(field) => {
                rows.push(widget::text::body(crate::field_commands::field_name(field)).into());
                match field {
                    FieldAt::Object { object, .. } => {
                        let format = match object {
                            InlineObject::Date(date) if date.source != DateSource::Time => {
                                Some(date.format)
                            }
                            InlineObject::DocInfo(entry) if matches!(entry.code, 4 | 5) => {
                                Some(entry.format)
                            }
                            _ => None,
                        };
                        if let Some(format) = format {
                            let samples: Vec<String> = DATE_FORMATS
                                .iter()
                                .map(|letter| {
                                    format_date(
                                        &shown.context.now,
                                        *letter,
                                        &shown.context.settings,
                                    )
                                })
                                .collect();
                            rows.push(row(vec![
                                label("Format"),
                                widget::dropdown(
                                    samples,
                                    DATE_FORMATS.iter().position(|letter| *letter == format),
                                    |index| send(FieldsAction::ThisFormat(index)),
                                )
                                .width(Length::Fill)
                                .into(),
                            ]));
                        }
                        if matches!(object, InlineObject::Seq(_)) {
                            rows.push(code_row(state));
                        }
                    }
                    FieldAt::Power { span, .. } => {
                        rows.push(code_row(state));
                        rows.push(row(vec![
                            widget::checkbox(span.mark.locked())
                                .label("Lock")
                                .on_toggle(|on| send(FieldsAction::ThisLock(on)))
                                .into(),
                            widget::checkbox(span.mark.auto_run())
                                .label("Update when opened")
                                .on_toggle(|on| send(FieldsAction::ThisAutoRun(on)))
                                .into(),
                        ]));
                    }
                }
                if let Some(shows) = &shown.this_shows {
                    rows.push(row(vec![
                        label("Shows"),
                        widget::text::body(shows.clone()).width(Length::Fill).into(),
                    ]));
                }
                rows.push(
                    widget::row::with_children(vec![
                        widget::space::horizontal().into(),
                        widget::button::standard("Update")
                            .on_press(send(FieldsAction::ThisUpdate))
                            .into(),
                    ])
                    .into(),
                );
            }
        }
        body.push(
            widget::container(widget::column::with_children(rows).spacing(6))
                .padding([0, 0, 8, 22])
                .into(),
        );
    }
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(6).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

fn code_row(state: &FieldsPanel) -> Element<'static, Message> {
    row(vec![
        label("Code"),
        widget::text_input("", state.this_code.clone())
            .on_input(|code| send(FieldsAction::ThisCode(code)))
            .on_submit(|_| send(FieldsAction::ThisSubmit))
            .on_unfocus(send(FieldsAction::ThisSubmit))
            .width(Length::Fill)
            .into(),
    ])
}

/// The three Insert pages in a list, as the rail's other choices are.
fn page_selector(current: Page) -> Element<'static, Message> {
    const PAGES: [Page; 3] = [Page::DateTime, Page::DocInfo, Page::Power];
    let chosen = PAGES.iter().position(|page| *page == current).unwrap_or(2);
    row(vec![
        label("Insert"),
        widget::dropdown(
            vec!["Date and time", "Document info", "Power field"],
            Some(chosen),
            |index| send(FieldsAction::Page(PAGES[index])),
        )
        .width(Length::Fill)
        .into(),
    ])
}

/// The entries' labels as Insert Doc Info Field lists them, with their
/// values now (`shown` text for each object entry).
pub fn doc_info_labels(
    names: impl Fn(usize) -> String,
    value: impl Fn(&InlineObject) -> String,
) -> Vec<String> {
    doc_info_entries()
        .into_iter()
        .map(|entry| match entry {
            None => "Document Description (a power field)".to_owned(),
            Some(entry) => {
                let label = match entry {
                    DocInfoEntry::Field(index) => names(usize::from(index)),
                    other => crate::field_commands::doc_info_label(other),
                };
                let shown = value(&InlineObject::DocInfo(DocInfoField {
                    code: entry.code(),
                    format: b'a',
                }));
                if shown.is_empty() {
                    label
                } else {
                    format!("{label}: {shown}")
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertions_are_ami_pros() {
        let mut panel = FieldsPanel::new(Page::DateTime, None);
        // Today's date is text; System date the object with the format.
        assert!(matches!(
            panel.update(FieldsAction::Insert, &[]),
            Outcome::InsertText
        ));
        panel.update(FieldsAction::DateKind(1), &[]);
        panel.update(FieldsAction::DateFormat(1), &[]);
        assert!(matches!(
            panel.update(FieldsAction::Insert, &[]),
            Outcome::InsertObject(InlineObject::Date(DateField {
                source: DateSource::Today,
                format: b'b'
            }))
        ));
        // The time keeps the list's position (FD1: `<:b<*!>>`).
        panel.update(FieldsAction::DateKind(2), &[]);
        assert!(matches!(
            panel.update(FieldsAction::Insert, &[]),
            Outcome::InsertObject(InlineObject::Date(DateField {
                source: DateSource::Time,
                format: 1
            }))
        ));
        // Document Description is a power field; Filename an entry.
        panel.update(FieldsAction::Page(Page::DocInfo), &[]);
        panel.update(FieldsAction::DocEntry(3), &[]);
        assert!(matches!(
            panel.update(FieldsAction::Insert, &[]),
            Outcome::InsertPower { code, .. } if code == "Description"
        ));
        panel.update(FieldsAction::DocEntry(0), &[]);
        assert!(matches!(
            panel.update(FieldsAction::Insert, &[]),
            Outcome::InsertObject(InlineObject::DocInfo(DocInfoField { code: 1, .. }))
        ));
        // A power field with its format and flags.
        panel.update(FieldsAction::Page(Page::Power), &[]);
        panel.update(FieldsAction::Pick(3), &[]);
        panel.update(FieldsAction::Option(1), &[]);
        panel.update(FieldsAction::Lock(true), &[]);
        let Outcome::InsertPower { code, flags } = panel.update(FieldsAction::Insert, &[]) else {
            panic!("a power field");
        };
        assert_eq!(code, "CreateDate %Db");
        assert_eq!(flags, power_flag::LOCK);
        // Saved fields fill the code.
        let saved = [("Greeting".to_owned(), "\"Dear {Name}\"".to_owned())];
        panel.update(FieldsAction::Saved(0), &saved);
        assert!(matches!(
            panel.update(FieldsAction::Insert, &saved),
            Outcome::InsertPower { code, .. } if code == "\"Dear {Name}\""
        ));
        assert_eq!(POWER_FIELDS.iter().filter(|field| !field.3).count(), 8);
    }
}
