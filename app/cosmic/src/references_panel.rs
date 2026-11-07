//! The Notes, Bookmarks, and Footnotes panels in the rail (the notes run;
//! `docs/Design/UI_SPEC.md` §34, NR1, NR4, NR10): the document's notes,
//! read and edited in place, its bookmarks with Add, Go To, Rename, and
//! Remove, and Ami Pro's Footnote Options, live. Each change is one
//! command and one undo step; typing in a note joins its keys.

use amipro_document::bookmarks::{Bookmark, BookmarkTarget};
use amipro_document::{Colour, FootnoteOptions, NoteInfo};
use amipro_editing::TextPosition;
use cosmic::iced::Length;
use cosmic::iced::widget::text_editor;
use cosmic::widget;

use crate::Message;

pub const WIDTH: f32 = 300.0;

type Element<'a> = cosmic::Element<'a, Message>;

/// Which panel shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Notes,
    Bookmarks,
    Footnotes,
}

/// The note being edited, with its text in an editor.
pub struct NoteEditor {
    pub at: TextPosition,
    pub content: text_editor::Content,
}

/// What a panel keeps between messages.
pub struct ReferencePanel {
    pub kind: Kind,
    /// The note whose card is open for editing.
    pub editing: Option<NoteEditor>,
    /// Bookmarks: the name field, the selected row, and why a name was
    /// refused.
    pub name: String,
    pub selected: Option<String>,
    pub problem: Option<String>,
    /// Footnotes: the number fields as typed.
    pub start: String,
    pub length: String,
    pub indent: String,
}

impl ReferencePanel {
    pub fn new(kind: Kind, options: &FootnoteOptions) -> Self {
        let mut panel = Self {
            kind,
            editing: None,
            name: String::new(),
            selected: None,
            problem: None,
            start: String::new(),
            length: String::new(),
            indent: String::new(),
        };
        panel.sync_options(options);
        panel
    }

    /// The number fields from the document's options.
    pub fn sync_options(&mut self, options: &FootnoteOptions) {
        self.start = options.start.to_string();
        self.length = inches(options.length);
        self.indent = inches(options.indent);
    }

    /// Opens the note at `at` for editing, its text in the editor.
    pub fn edit_note(&mut self, at: TextPosition, text: &str) {
        if self
            .editing
            .as_ref()
            .is_some_and(|editing| editing.at == at)
        {
            return;
        }
        self.editing = Some(NoteEditor {
            at,
            content: text_editor::Content::with_text(text),
        });
    }
}

/// Twips as inches with two decimals, as Ami Pro's boxes show them.
pub fn inches(twips: i32) -> String {
    format!("{:.2}", twips as f32 / 1_440.0)
}

/// Inches typed (an optional `in` or `"`) as twips.
pub fn parse_inches(text: &str) -> Option<i32> {
    let text = text
        .trim()
        .trim_end_matches(['"', ' '])
        .trim_end_matches("in");
    let value: f32 = text.trim().parse().ok()?;
    (0.0..=22.0)
        .contains(&value)
        .then(|| (value * 1_440.0).round() as i32)
}

#[derive(Clone, Debug)]
pub enum ReferenceAction {
    Close,
    // Notes.
    EditNote(TextPosition),
    NoteEdit(text_editor::Action),
    GoToNote(TextPosition),
    RemoveNote(TextPosition),
    RemoveAllNotes,
    PreviousNote,
    NextNote,
    Initials(String),
    NoteColour(Colour),
    // Bookmarks.
    Name(String),
    Add,
    Select(String),
    GoTo(String),
    Rename,
    Remove,
    ShowBookmarks(bool),
    CrossReference { page: bool },
    // Footnotes.
    Flag(u8, bool),
    Start(String),
    Length(String),
    Indent(String),
    ContinuedOn(String),
    ContinuedFrom(String),
    NumberStyle,
    PreviousFootnote,
    NextFootnote,
}

fn send(action: ReferenceAction) -> Message {
    Message::References(action)
}

/// A note as the Notes panel lists it.
pub struct NoteRow {
    pub at: TextPosition,
    pub note: NoteInfo,
    pub text: String,
    pub page: Option<u32>,
    /// Its date and time as Ami Pro's title shows them (four-digit years).
    pub when: String,
}

/// A bookmark as the Bookmarks panel lists it.
pub struct BookmarkRow {
    pub bookmark: Bookmark,
    pub page: Option<u32>,
}

/// What the panels show besides their state.
pub struct Shown<'a> {
    pub notes: Vec<NoteRow>,
    pub initials: &'a str,
    pub colour: Colour,
    pub bookmarks: Vec<BookmarkRow>,
    pub show_bookmarks: bool,
    pub options: &'a FootnoteOptions,
    pub footnotes: usize,
}

fn header(title: &'static str, close: &'static str) -> Element<'static> {
    widget::row::with_children(vec![
        widget::text::heading(title).into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name(close)
            .on_press(send(ReferenceAction::Close))
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

fn wrap<'a>(title: Element<'a>, body: Vec<Element<'a>>) -> Element<'a> {
    widget::column::with_children(vec![
        title,
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

pub fn panel<'a>(state: &'a ReferencePanel, shown: Shown<'a>) -> Element<'a> {
    match state.kind {
        Kind::Notes => notes_panel(state, shown),
        Kind::Bookmarks => bookmarks_panel(state, shown),
        Kind::Footnotes => footnotes_panel(state, shown),
    }
}

fn notes_panel<'a>(state: &'a ReferencePanel, shown: Shown<'a>) -> Element<'a> {
    let mut body: Vec<Element<'a>> = Vec::new();
    // Ami Pro's User Setup: the initials and colour new notes take.
    let mut you: Vec<Element<'a>> = vec![
        widget::text::body("You").width(Length::Fixed(36.0)).into(),
        widget::text_input("Initials", shown.initials.to_owned())
            .on_input(|text| send(ReferenceAction::Initials(text)))
            .width(Length::Fixed(64.0))
            .into(),
    ];
    for colour in NOTE_COLOURS {
        you.push(crate::colour_control::swatch(
            colour,
            colour == shown.colour,
            send(ReferenceAction::NoteColour(colour)),
        ));
    }
    body.push(
        widget::row::with_children(you)
            .spacing(2)
            .align_y(cosmic::iced::Alignment::Center)
            .wrap()
            .into(),
    );
    body.push(row(vec![
        widget::button::text("‹ Previous")
            .on_press_maybe((!shown.notes.is_empty()).then(|| send(ReferenceAction::PreviousNote)))
            .into(),
        widget::space::horizontal().into(),
        widget::text::caption(match shown.notes.len() {
            0 => "No notes".to_owned(),
            1 => "1 note".to_owned(),
            count => format!("{count} notes"),
        })
        .into(),
        widget::space::horizontal().into(),
        widget::button::text("Next ›")
            .on_press_maybe((!shown.notes.is_empty()).then(|| send(ReferenceAction::NextNote)))
            .into(),
    ]));
    let any = !shown.notes.is_empty();
    if !any {
        body.push(
            widget::text::caption("Insert › Note (Ctrl+Alt+M) puts a note at the caret.").into(),
        );
    }
    for note in shown.notes {
        let editing = state
            .editing
            .as_ref()
            .filter(|editing| editing.at == note.at);
        let title = format!(
            "{}{}  {}{}",
            note.note.initials.as_deref().unwrap_or(""),
            note.note
                .number
                .filter(|number| *number > 0)
                .map_or_else(String::new, |number| number.to_string()),
            note.when,
            note.page
                .map_or_else(String::new, |page| format!("  p. {page}")),
        );
        let swatch = crate::colour_control::swatch(
            note.note.colour.unwrap_or(NoteInfo::YELLOW),
            false,
            send(ReferenceAction::GoToNote(note.at)),
        );
        let mut card: Vec<Element<'a>> = vec![row(vec![
            swatch,
            widget::button::text(title)
                .on_press(send(ReferenceAction::GoToNote(note.at)))
                .into(),
        ])];
        match editing {
            Some(editing) => {
                card.push(
                    text_editor(&editing.content)
                        .on_action(|action| send(ReferenceAction::NoteEdit(action)))
                        .height(Length::Shrink)
                        .into(),
                );
            }
            None => card.push(
                widget::button::custom(widget::text::body(if note.text.is_empty() {
                    "(empty)".to_owned()
                } else {
                    note.text.clone()
                }))
                .class(cosmic::theme::Button::Text)
                .width(Length::Fill)
                .on_press(send(ReferenceAction::EditNote(note.at)))
                .into(),
            ),
        }
        card.push(row(vec![
            widget::space::horizontal().into(),
            widget::button::text("Remove")
                .on_press(send(ReferenceAction::RemoveNote(note.at)))
                .into(),
        ]));
        body.push(
            widget::container(widget::column::with_children(card).spacing(4))
                .padding(6)
                .class(cosmic::theme::Container::Card)
                .into(),
        );
    }
    if any {
        body.push(
            widget::button::standard("Remove All Notes")
                .on_press(send(ReferenceAction::RemoveAllNotes))
                .into(),
        );
    }
    wrap(header("Notes", "Close Notes"), body)
}

/// The note colours offered: Ami Pro's User Setup palette.
pub const NOTE_COLOURS: [Colour; 8] = [
    0x00_FFFF, 0x00_80FF, 0x00_00FF, 0xFF_00FF, 0xFF_0000, 0xFF_FF00, 0x00_FF00, 0xC0_C0C0,
];

fn kind_label(bookmark: &Bookmark) -> &'static str {
    match bookmark.target {
        BookmarkTarget::Text { .. } => "text",
        BookmarkTarget::Cells { .. } => "cells",
        BookmarkTarget::Cell { .. } => "cell",
        BookmarkTarget::Frame(_) => "frame",
    }
}

fn bookmarks_panel<'a>(state: &'a ReferencePanel, shown: Shown<'a>) -> Element<'a> {
    let mut body: Vec<Element<'a>> = Vec::new();
    let exists = shown
        .bookmarks
        .iter()
        .any(|row| row.bookmark.name.eq_ignore_ascii_case(state.name.trim()));
    let default = if exists {
        widget::button::suggested("Go To").on_press(send(ReferenceAction::GoTo(state.name.clone())))
    } else {
        widget::button::suggested("Add")
            .on_press_maybe((!state.name.trim().is_empty()).then(|| send(ReferenceAction::Add)))
    };
    body.push(row(vec![
        widget::text_input("Name", state.name.clone())
            .on_input(|text| send(ReferenceAction::Name(text)))
            .on_submit(move |_| {
                send(if exists {
                    ReferenceAction::GoTo(String::new())
                } else {
                    ReferenceAction::Add
                })
            })
            .width(Length::Fill)
            .into(),
        default.into(),
    ]));
    if let Some(problem) = &state.problem {
        body.push(widget::text::caption(problem.clone()).into());
    }
    if shown.bookmarks.is_empty() {
        body.push(
            widget::text::caption(
                "Select text, cells, or a frame (or place the caret), type a name, and Add.",
            )
            .into(),
        );
    }
    let mut list: Vec<Element<'a>> = Vec::new();
    for entry in &shown.bookmarks {
        let name = entry.bookmark.name.clone();
        let selected = state.selected.as_deref() == Some(name.as_str());
        let line = format!(
            "{}   {}{}",
            name,
            kind_label(&entry.bookmark),
            entry
                .page
                .map_or_else(String::new, |page| format!("   p. {page}")),
        );
        list.push(
            widget::button::custom(widget::text::body(line))
                .class(if selected {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Text
                })
                .width(Length::Fill)
                .on_press(send(if selected {
                    ReferenceAction::GoTo(name.clone())
                } else {
                    ReferenceAction::Select(name.clone())
                }))
                .into(),
        );
    }
    body.push(widget::column::with_children(list).spacing(2).into());
    let chosen = state.selected.is_some();
    body.push(row(vec![
        widget::button::standard("Go To")
            .on_press_maybe(
                state
                    .selected
                    .clone()
                    .map(|name| send(ReferenceAction::GoTo(name))),
            )
            .into(),
        widget::button::standard("Rename")
            .on_press_maybe(
                (chosen && !state.name.trim().is_empty() && !exists)
                    .then(|| send(ReferenceAction::Rename)),
            )
            .into(),
        widget::button::standard("Remove")
            .on_press_maybe(chosen.then(|| send(ReferenceAction::Remove)))
            .into(),
    ]));
    body.push(
        widget::checkbox(shown.show_bookmarks)
            .label("Show bookmarks on the page")
            .on_toggle(|on| send(ReferenceAction::ShowBookmarks(on)))
            .into(),
    );
    body.push(row(vec![
        widget::text::body("Insert a reference to").into(),
    ]));
    body.push(row(vec![
        widget::button::standard("Its page")
            .on_press_maybe(chosen.then(|| send(ReferenceAction::CrossReference { page: true })))
            .into(),
        widget::button::standard("Its text")
            .on_press_maybe(chosen.then(|| send(ReferenceAction::CrossReference { page: false })))
            .into(),
    ]));
    wrap(header("Bookmarks", "Close Bookmarks"), body)
}

fn footnotes_panel<'a>(state: &'a ReferencePanel, shown: Shown<'a>) -> Element<'a> {
    let options = shown.options;
    let endnotes = options.endnotes();
    let flag = |label: &'static str, bit: u8, enabled: bool| -> Element<'a> {
        let mut check = widget::checkbox(options.has(bit)).label(label);
        if enabled {
            check = check.on_toggle(move |on| send(ReferenceAction::Flag(bit, on)));
        }
        check.into()
    };
    let field =
        |value: &str, action: fn(String) -> ReferenceAction, enabled: bool| -> Element<'a> {
            let mut input = widget::text_input("", value.to_owned()).width(Length::Fixed(64.0));
            if enabled {
                input = input.on_input(move |text| send(action(text)));
            }
            input.into()
        };
    // A message's place: left, or right (Ami Pro's default).
    let side = |left_bit: u8| -> Element<'a> {
        let left = options.has(left_bit);
        row(vec![
            widget::radio("Left", true, Some(left), move |value| {
                send(ReferenceAction::Flag(left_bit, value))
            })
            .into(),
            widget::radio("Right", false, Some(left), move |value| {
                send(ReferenceAction::Flag(left_bit, value))
            })
            .into(),
        ])
    };
    let mut body: Vec<Element<'a>> = vec![
        flag("Make endnotes", FootnoteOptions::ENDNOTES, true),
        flag(
            "Reset number on each page",
            FootnoteOptions::RESET_EACH_PAGE,
            !endnotes,
        ),
        row(vec![
            widget::text::body("Starting number").into(),
            field(&state.start, ReferenceAction::Start, !endnotes),
        ]),
        widget::text::heading("Separator line").into(),
        row(vec![
            widget::radio(
                "Margins",
                false,
                Some(options.has(FootnoteOptions::CUSTOM_LENGTH)),
                |_| send(ReferenceAction::Flag(FootnoteOptions::CUSTOM_LENGTH, false)),
            )
            .into(),
            widget::radio(
                "Custom",
                true,
                Some(options.has(FootnoteOptions::CUSTOM_LENGTH)),
                |_| send(ReferenceAction::Flag(FootnoteOptions::CUSTOM_LENGTH, true)),
            )
            .into(),
            field(&state.length, ReferenceAction::Length, !endnotes),
            widget::text::caption("in").into(),
        ]),
        row(vec![
            widget::text::body("Indent from left").into(),
            field(&state.indent, ReferenceAction::Indent, !endnotes),
            widget::text::caption("in").into(),
        ]),
        widget::text::heading("Continued messages").into(),
        flag(
            "Continued on message",
            FootnoteOptions::CONTINUED_ON,
            !endnotes,
        ),
        widget::text_input("Continued on next page", options.continued_on.clone())
            .on_input(|text| send(ReferenceAction::ContinuedOn(text)))
            .width(Length::Fill)
            .into(),
        side(FootnoteOptions::CONTINUED_ON_LEFT),
        flag(
            "Continued from message",
            FootnoteOptions::CONTINUED_FROM,
            !endnotes,
        ),
        widget::text_input(
            "Continued from previous page",
            options.continued_from.clone(),
        )
        .on_input(|text| send(ReferenceAction::ContinuedFrom(text)))
        .width(Length::Fill)
        .into(),
        side(FootnoteOptions::CONTINUED_FROM_LEFT),
        flag(
            "Repeat footnote number",
            FootnoteOptions::REPEAT_NUMBER,
            !endnotes && !options.reset_each_page() && options.has(FootnoteOptions::CONTINUED_FROM),
        ),
        widget::button::standard("Number style…")
            .on_press(send(ReferenceAction::NumberStyle))
            .into(),
        row(vec![
            widget::button::text("‹ Previous")
                .on_press_maybe(
                    (shown.footnotes > 0).then(|| send(ReferenceAction::PreviousFootnote)),
                )
                .into(),
            widget::space::horizontal().into(),
            widget::text::caption(match shown.footnotes {
                0 => "No footnotes".to_owned(),
                1 => "1 footnote".to_owned(),
                count => format!("{count} footnotes"),
            })
            .into(),
            widget::space::horizontal().into(),
            widget::button::text("Next ›")
                .on_press_maybe((shown.footnotes > 0).then(|| send(ReferenceAction::NextFootnote)))
                .into(),
        ]),
    ];
    if endnotes {
        body.insert(
            1,
            widget::text::caption("Every footnote follows the text, from a new page.").into(),
        );
    }
    wrap(header("Footnotes", "Close Footnotes"), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inches_as_ami_pros_boxes_show_them() {
        assert_eq!(inches(2_880), "2.00");
        assert_eq!(parse_inches("0.50 in"), Some(720));
        assert_eq!(parse_inches("2\""), Some(2_880));
        assert_eq!(parse_inches("x"), None);
    }
}
