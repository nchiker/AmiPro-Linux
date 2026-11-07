//! The Contents & Index panel in the rail (the TOC, index, and glossary
//! run; `docs/Design/UI_SPEC.md` §36, TX1–TX5): Ami Pro's TOC Options, its
//! TOC, Index dialog, and its Mark Index Entry and TOC Entry dialogs as
//! three pages that act on the document live. Each change is one command
//! and one undo step.

use amipro_document::contents::{Leader, TOC_LEVELS, TocLevel};
use amipro_editing::TextPosition;
use cosmic::iced::Length;
use cosmic::widget;

use crate::Message;

pub const WIDTH: f32 = 300.0;

type Element<'a> = cosmic::Element<'a, Message>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Page {
    #[default]
    Contents,
    Index,
    Entries,
}

/// Where Generate Contents puts the TOC (TX2).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TocChoice {
    #[default]
    Start,
    Caret,
    NewDocument,
}

/// Where Generate Index puts the index (TX3).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum IndexChoice {
    #[default]
    NewDocument,
    End,
}

/// What the panel keeps between messages.
#[derive(Clone, Debug, Default)]
pub struct ContentsPanel {
    pub page: Page,
    pub toc_place: TocChoice,
    pub index_place: IndexChoice,
    /// Mark Index Entry's fields.
    pub primary: String,
    pub secondary: String,
    pub page_number: bool,
    pub other_on: bool,
    pub other: String,
    /// The mark the fields show (Next Mark, a click in the list), which
    /// Mark replaces and Remove takes out.
    pub current: Option<MarkAt>,
    /// Mark All's Exact case (TX4, on by default).
    pub exact_case: bool,
    /// TOC Entry's level and text.
    pub toc_level: String,
    pub toc_text: String,
    /// The last thing done, or why it could not be.
    pub message: Option<String>,
    /// The lists the drop-downs show, refreshed after every message: the
    /// styles that are no level, the document's primaries, and the
    /// secondaries under the Primary typed.
    pub other_styles: Vec<String>,
    pub primaries: Vec<String>,
    pub secondaries: Vec<String>,
}

/// A mark in the text: its flow, paragraph, and index among the
/// paragraph's marks, and the place it stands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkAt {
    pub at: TextPosition,
    pub mark: usize,
}

impl ContentsPanel {
    pub fn new(page: Page) -> Self {
        Self {
            page,
            page_number: true,
            exact_case: true,
            toc_level: "1".into(),
            ..Self::default()
        }
    }

    /// The fields for a new entry from `text` (Mark Index Entry's Primary
    /// takes the selection, trailing spaces trimmed; Page number on).
    pub fn new_entry(&mut self, text: &str) {
        self.primary = text.trim_end().chars().take(79).collect();
        self.secondary.clear();
        self.page_number = true;
        self.other_on = false;
        self.other.clear();
        self.current = None;
    }

    /// The entry the fields describe.
    pub fn entry(&self) -> amipro_document::contents::IndexEntry {
        amipro_document::contents::IndexEntry {
            primary: self.primary.chars().take(79).collect(),
            secondary: self.secondary.chars().take(79).collect(),
            other: if self.other_on {
                self.other.chars().take(79).collect()
            } else {
                String::new()
            },
            page_number: self.page_number,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ContentsAction {
    Close,
    Page(Page),
    // Contents.
    Promote(usize),
    Demote(usize),
    RemoveLevel(usize),
    AddLevel(usize),
    LevelPageNumber(usize, bool),
    LevelRightAlign(usize, bool),
    LevelLeader(usize, usize),
    TocPlace(TocChoice),
    GenerateToc,
    // Index.
    Separators(bool),
    Ranges(bool),
    IndexPlace(IndexChoice),
    GenerateIndex,
    // Entries.
    Primary(String),
    Secondary(String),
    PageNumber(bool),
    OtherOn(bool),
    Other(String),
    ChoosePrimary(usize),
    ChooseSecondary(usize),
    Mark,
    MarkAll,
    ExactCase(bool),
    Remove,
    NextMark,
    PreviousMark,
    NextSame,
    PreviousSame,
    GoTo(MarkAt),
    TocLevel(String),
    TocText(String),
    MarkToc,
}

fn send(action: ContentsAction) -> Message {
    Message::Contents(action)
}

/// An index mark as the Entries page lists it.
pub struct EntryRow {
    pub at: MarkAt,
    pub label: String,
    pub page: Option<String>,
}

/// What the panel shows besides its state.
pub struct Shown<'a> {
    pub levels: Vec<TocLevel>,
    /// Paragraph styles that are no level yet, for Add.
    pub other_styles: &'a [String],
    pub has_toc: bool,
    pub separators: bool,
    pub ranges: bool,
    pub entries: Vec<EntryRow>,
    pub primaries: &'a [String],
    pub secondaries: &'a [String],
    /// The example Mark Index Entry shows.
    pub example: String,
}

/// The leaders a level may have: with right aligned pages, none, dots,
/// hyphens, a line; with pages after the text, none or a comma.
pub fn leaders(right_align: bool) -> &'static [(Leader, &'static str)] {
    if right_align {
        &[
            (Leader::None, "None"),
            (Leader::Dots, "....."),
            (Leader::Hyphens, "-----"),
            (Leader::Line, "_____"),
        ]
    } else {
        &[(Leader::None, "None"), (Leader::Comma, ", (comma)")]
    }
}

pub const RIGHT_LEADERS: [&str; 4] = ["None", ".....", "-----", "_____"];
pub const AFTER_LEADERS: [&str; 2] = ["None", ", (comma)"];

fn header() -> Element<'static> {
    widget::row::with_children(vec![
        widget::text::heading("Contents & Index").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Contents & Index")
            .on_press(send(ContentsAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn row(children: Vec<Element<'_>>) -> Element<'_> {
    widget::row::with_children(children)
        .spacing(6)
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

fn pages(page: Page) -> Element<'static> {
    let button = |label: &'static str, this: Page| -> Element<'static> {
        widget::button::custom(widget::container(widget::text::body(label)).center_x(Length::Fill))
            .class(if page == this {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            })
            .width(Length::Fill)
            .on_press(send(ContentsAction::Page(this)))
            .into()
    };
    widget::row::with_children(vec![
        button("Contents", Page::Contents),
        button("Index", Page::Index),
        button("Entries", Page::Entries),
    ])
    .spacing(2)
    .into()
}

fn radio<'a, T: Copy + Eq + 'a>(
    label: &'static str,
    value: T,
    selected: T,
    action: impl Fn(T) -> ContentsAction + 'a,
) -> Element<'a> {
    widget::radio(
        widget::text::body(label),
        value,
        Some(selected),
        move |value| send(action(value)),
    )
    .into()
}

pub fn panel<'a>(state: &'a ContentsPanel, shown: Shown<'a>) -> Element<'a> {
    let mut body: Vec<Element<'a>> = vec![pages(state.page)];
    match state.page {
        Page::Contents => contents_page(state, &shown, &mut body),
        Page::Index => index_page(state, &shown, &mut body),
        Page::Entries => entries_page(state, shown, &mut body),
    }
    if let Some(message) = &state.message {
        body.push(widget::text::caption(message.clone()).into());
    }
    widget::column::with_children(vec![
        header(),
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

fn contents_page<'a>(state: &'a ContentsPanel, shown: &Shown<'a>, body: &mut Vec<Element<'a>>) {
    body.push(widget::text::heading("Levels").into());
    if shown.levels.is_empty() {
        body.push(
            widget::text::caption(
                "No style is a TOC level yet: add one below, or mark entries on the Entries page.",
            )
            .into(),
        );
    }
    let count = shown.levels.len();
    for (index, level) in shown.levels.iter().enumerate() {
        let mut card: Vec<Element<'a>> = vec![row(vec![
            widget::text::body(format!("{}  {}", index + 1, level.style))
                .width(Length::Fill)
                .into(),
            widget::button::icon(widget::icon::from_name("go-up-symbolic"))
                .extra_small()
                .name("Promote")
                .on_press_maybe((index > 0).then(|| send(ContentsAction::Promote(index))))
                .into(),
            widget::button::icon(widget::icon::from_name("go-down-symbolic"))
                .extra_small()
                .name("Demote")
                .on_press_maybe((index + 1 < count).then(|| send(ContentsAction::Demote(index))))
                .into(),
            widget::button::icon(widget::icon::from_name("list-remove-symbolic"))
                .extra_small()
                .name("Remove level")
                .on_press(send(ContentsAction::RemoveLevel(index)))
                .into(),
        ])];
        card.push(
            widget::checkbox(level.page_number)
                .label("Page number")
                .on_toggle(move |on| send(ContentsAction::LevelPageNumber(index, on)))
                .into(),
        );
        if level.page_number {
            card.push(
                widget::checkbox(level.right_align)
                    .label("Right align page number")
                    .on_toggle(move |on| send(ContentsAction::LevelRightAlign(index, on)))
                    .into(),
            );
            let choices = leaders(level.right_align);
            let selected = choices
                .iter()
                .position(|(leader, _)| *leader == level.leader);
            let names: &'static [&'static str] = if level.right_align {
                &RIGHT_LEADERS
            } else {
                &AFTER_LEADERS
            };
            card.push(row(vec![
                widget::text::body(if level.right_align {
                    "Leader"
                } else {
                    "Separator"
                })
                .into(),
                widget::dropdown(names, selected, move |choice| {
                    send(ContentsAction::LevelLeader(index, choice))
                })
                .width(Length::Fill)
                .into(),
            ]));
        }
        body.push(
            widget::container(widget::column::with_children(card).spacing(4))
                .padding(6)
                .class(cosmic::theme::Container::Card)
                .into(),
        );
    }
    if count < TOC_LEVELS && !shown.other_styles.is_empty() {
        body.push(row(vec![
            widget::text::body("Add").into(),
            widget::dropdown(shown.other_styles, None, |index| {
                send(ContentsAction::AddLevel(index))
            })
            .placeholder("a style as the next level")
            .width(Length::Fill)
            .into(),
        ]));
    }
    body.push(widget::text::heading("Place").into());
    body.push(radio(
        "Start of this document",
        TocChoice::Start,
        state.toc_place,
        ContentsAction::TocPlace,
    ));
    body.push(radio(
        "At the caret",
        TocChoice::Caret,
        state.toc_place,
        ContentsAction::TocPlace,
    ));
    body.push(radio(
        "New document",
        TocChoice::NewDocument,
        state.toc_place,
        ContentsAction::TocPlace,
    ));
    body.push(wide(
        if shown.has_toc && state.toc_place != TocChoice::NewDocument {
            "Update Contents"
        } else {
            "Generate Contents"
        },
        Some(send(ContentsAction::GenerateToc)),
    ));
    if shown.has_toc {
        body.push(
            widget::text::caption(
                "Update replaces the document's table of contents where it stands. Ctrl+click an entry to go to its heading.",
            )
            .into(),
        );
    }
}

fn index_page<'a>(state: &'a ContentsPanel, shown: &Shown<'a>, body: &mut Vec<Element<'a>>) {
    body.push(
        widget::checkbox(shown.separators)
            .label("Alphabetical separators")
            .on_toggle(|on| send(ContentsAction::Separators(on)))
            .into(),
    );
    body.push(
        widget::checkbox(shown.ranges)
            .label("Join pages into ranges (3–5)")
            .on_toggle(|on| send(ContentsAction::Ranges(on)))
            .into(),
    );
    body.push(widget::text::heading("Place").into());
    body.push(radio(
        "New document",
        IndexChoice::NewDocument,
        state.index_place,
        ContentsAction::IndexPlace,
    ));
    body.push(radio(
        "End of this document",
        IndexChoice::End,
        state.index_place,
        ContentsAction::IndexPlace,
    ));
    body.push(wide(
        "Generate Index",
        (!shown.entries.is_empty()).then(|| send(ContentsAction::GenerateIndex)),
    ));
    body.push(
        widget::text::caption(match shown.entries.len() {
            0 => "No index entries yet: mark them on the Entries page.".to_owned(),
            1 => "1 index entry".to_owned(),
            count => format!("{count} index entries"),
        })
        .into(),
    );
}

fn entries_page<'a>(state: &'a ContentsPanel, shown: Shown<'a>, body: &mut Vec<Element<'a>>) {
    body.push(widget::text::heading("Index entry").into());
    body.push(row(vec![
        widget::text::body("Primary")
            .width(Length::Fixed(72.0))
            .into(),
        widget::text_input("", state.primary.as_str())
            .on_input(|text| send(ContentsAction::Primary(text)))
            .on_submit(|_| send(ContentsAction::Mark))
            .width(Length::Fill)
            .into(),
    ]));
    if !shown.primaries.is_empty() {
        body.push(
            widget::dropdown(shown.primaries, None, |index| {
                send(ContentsAction::ChoosePrimary(index))
            })
            .placeholder("Entries in this document")
            .width(Length::Fill)
            .into(),
        );
    }
    body.push(row(vec![
        widget::text::body("Secondary")
            .width(Length::Fixed(72.0))
            .into(),
        widget::text_input("", state.secondary.as_str())
            .on_input(|text| send(ContentsAction::Secondary(text)))
            .on_submit(|_| send(ContentsAction::Mark))
            .width(Length::Fill)
            .into(),
    ]));
    if !shown.secondaries.is_empty() {
        body.push(
            widget::dropdown(shown.secondaries, None, |index| {
                send(ContentsAction::ChooseSecondary(index))
            })
            .placeholder("Under this primary")
            .width(Length::Fill)
            .into(),
        );
    }
    body.push(
        widget::checkbox(state.page_number)
            .label("Page number")
            .on_toggle(|on| send(ContentsAction::PageNumber(on)))
            .into(),
    );
    body.push(row(vec![
        widget::checkbox(state.other_on)
            .label("Other")
            .on_toggle(|on| send(ContentsAction::OtherOn(on)))
            .into(),
        widget::text_input(" See also …", state.other.as_str())
            .on_input(|text| send(ContentsAction::Other(text)))
            .width(Length::Fill)
            .into(),
    ]));
    body.push(widget::text::caption(shown.example.clone()).into());
    let has_primary = !state.primary.trim().is_empty();
    body.push(row(vec![
        widget::button::standard(if state.current.is_some() {
            "Change"
        } else {
            "Mark"
        })
        .on_press_maybe(has_primary.then(|| send(ContentsAction::Mark)))
        .into(),
        widget::button::standard("Mark All")
            .on_press_maybe(has_primary.then(|| send(ContentsAction::MarkAll)))
            .into(),
        widget::button::standard("Remove")
            .on_press_maybe(
                state
                    .current
                    .is_some()
                    .then(|| send(ContentsAction::Remove)),
            )
            .into(),
    ]));
    body.push(
        widget::checkbox(state.exact_case)
            .label("Mark All: exact case")
            .on_toggle(|on| send(ContentsAction::ExactCase(on)))
            .into(),
    );
    let any = !shown.entries.is_empty();
    body.push(row(vec![
        widget::button::text("‹ Previous")
            .on_press_maybe(any.then(|| send(ContentsAction::PreviousMark)))
            .into(),
        widget::space::horizontal().into(),
        widget::button::text("Next ›")
            .on_press_maybe(any.then(|| send(ContentsAction::NextMark)))
            .into(),
    ]));
    body.push(row(vec![
        widget::button::text("‹ Previous same")
            .on_press_maybe((any && has_primary).then(|| send(ContentsAction::PreviousSame)))
            .into(),
        widget::space::horizontal().into(),
        widget::button::text("Next same ›")
            .on_press_maybe((any && has_primary).then(|| send(ContentsAction::NextSame)))
            .into(),
    ]));
    body.push(widget::text::heading("TOC entry").into());
    body.push(row(vec![
        widget::text::body("Level").into(),
        widget::text_input("1", state.toc_level.as_str())
            .on_input(|text| send(ContentsAction::TocLevel(text)))
            .width(Length::Fixed(40.0))
            .into(),
    ]));
    body.push(
        widget::text_input("Text to appear in the TOC", state.toc_text.as_str())
            .on_input(|text| send(ContentsAction::TocText(text)))
            .on_submit(|_| send(ContentsAction::MarkToc))
            .width(Length::Fill)
            .into(),
    );
    let level_ok = state
        .toc_level
        .trim()
        .parse::<u8>()
        .is_ok_and(|level| (1..=TOC_LEVELS as u8).contains(&level));
    body.push(
        widget::button::standard("Mark TOC Entry")
            .on_press_maybe(
                (level_ok && !state.toc_text.is_empty()).then(|| send(ContentsAction::MarkToc)),
            )
            .into(),
    );
    body.push(
        widget::text::heading(match shown.entries.len() {
            0 => "No index entries".to_owned(),
            1 => "1 index entry".to_owned(),
            count => format!("{count} index entries"),
        })
        .into(),
    );
    for entry in shown.entries {
        let selected = state.current == Some(entry.at);
        let label = match &entry.page {
            Some(page) => format!("{}  p. {page}", entry.label),
            None => entry.label.clone(),
        };
        body.push(
            widget::button::custom(widget::text::body(label))
                .class(if selected {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Text
                })
                .width(Length::Fill)
                .on_press(send(ContentsAction::GoTo(entry.at)))
                .into(),
        );
    }
}

/// The example Mark Index Entry shows (dialog 1109's Example): the entry
/// as the index will show it.
pub fn example(entry: &amipro_document::contents::IndexEntry) -> String {
    if entry.primary.trim().is_empty() {
        return String::new();
    }
    let page = if entry.page_number { ", ##" } else { "" };
    if entry.secondary.is_empty() {
        format!("{}{page}{}", entry.primary, entry.other)
    } else {
        format!(
            "{}\n     {}{page}{}",
            entry.primary, entry.secondary, entry.other
        )
    }
}
