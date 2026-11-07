//! The Glossary panel in the rail (the TOC, index, and glossary run;
//! `docs/Design/UI_SPEC.md` §36, TX7, TX8): the glossary data file's
//! records with a preview, Insert, Add from selection (Ami Pro's Mark
//! Glossary Record), Rename, Delete, and Data File….

use cosmic::iced::Length;
use cosmic::widget;

use crate::Message;

pub const WIDTH: f32 = 300.0;

type Element<'a> = cosmic::Element<'a, Message>;

/// A record as the panel lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordRow {
    pub name: String,
    /// Its text as plain text, for the preview.
    pub preview: String,
}

#[derive(Clone, Debug, Default)]
pub struct GlossaryPanel {
    /// The records of the data file as last read.
    pub records: Vec<RecordRow>,
    pub selected: Option<String>,
    /// Add's name and the selection's text it adds.
    pub name: String,
    /// Rename's new name.
    pub rename: String,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub enum GlossaryAction {
    Close,
    Select(String),
    Insert(String),
    Name(String),
    Add,
    RenameText(String),
    Rename,
    Delete,
    ChooseFile,
    NewFile,
    Complete(bool),
}

fn send(action: GlossaryAction) -> Message {
    Message::Glossary(action)
}

fn row(children: Vec<Element<'_>>) -> Element<'_> {
    widget::row::with_children(children)
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

/// What the panel shows besides its state.
pub struct Shown<'a> {
    pub file: &'a str,
    pub complete: bool,
    pub has_selection: bool,
}

pub fn panel<'a>(state: &'a GlossaryPanel, shown: Shown<'a>) -> Element<'a> {
    let mut body: Vec<Element<'a>> = Vec::new();
    body.push(
        widget::text::caption(if shown.file.is_empty() {
            "No data file chosen".to_owned()
        } else {
            format!(
                "Data file: {}",
                std::path::Path::new(shown.file).file_name().map_or_else(
                    || shown.file.to_owned(),
                    |name| name.to_string_lossy().into_owned()
                )
            )
        })
        .into(),
    );
    body.push(row(vec![
        widget::button::standard("Data File…")
            .on_press(send(GlossaryAction::ChooseFile))
            .into(),
        widget::button::standard("New…")
            .on_press(send(GlossaryAction::NewFile))
            .into(),
    ]));
    body.push(widget::text::heading("Add from selection").into());
    body.push(row(vec![
        widget::text_input("Record name", state.name.as_str())
            .on_input(|text| send(GlossaryAction::Name(text)))
            .on_submit(|_| send(GlossaryAction::Add))
            .width(Length::Fill)
            .into(),
        widget::button::standard("Add")
            .on_press_maybe(
                (shown.has_selection && !state.name.trim().is_empty() && !shown.file.is_empty())
                    .then(|| send(GlossaryAction::Add)),
            )
            .into(),
    ]));
    body.push(
        widget::checkbox(shown.complete)
            .label("Complete glossary names as you type")
            .on_toggle(|on| send(GlossaryAction::Complete(on)))
            .into(),
    );
    body.push(
        widget::text::heading(match state.records.len() {
            0 => "No records".to_owned(),
            1 => "1 record".to_owned(),
            count => format!("{count} records"),
        })
        .into(),
    );
    for record in &state.records {
        let selected = state.selected.as_deref() == Some(record.name.as_str());
        let mut card: Vec<Element<'a>> = vec![
            widget::button::custom(widget::text::body(if record.name.is_empty() {
                "***".to_owned()
            } else {
                record.name.clone()
            }))
            .class(if selected {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            })
            .width(Length::Fill)
            .on_press(send(GlossaryAction::Select(record.name.clone())))
            .into(),
        ];
        if selected {
            card.push(widget::text::caption(record.preview.clone()).into());
            card.push(row(vec![
                widget::button::suggested("Insert")
                    .on_press(send(GlossaryAction::Insert(record.name.clone())))
                    .into(),
                widget::space::horizontal().into(),
                widget::button::standard("Delete")
                    .on_press(send(GlossaryAction::Delete))
                    .into(),
            ]));
            card.push(row(vec![
                widget::text_input("New name", state.rename.as_str())
                    .on_input(|text| send(GlossaryAction::RenameText(text)))
                    .on_submit(|_| send(GlossaryAction::Rename))
                    .width(Length::Fill)
                    .into(),
                widget::button::standard("Rename")
                    .on_press_maybe(
                        (!state.rename.trim().is_empty()).then(|| send(GlossaryAction::Rename)),
                    )
                    .into(),
            ]));
        }
        body.push(
            widget::container(widget::column::with_children(card).spacing(4))
                .padding(4)
                .class(cosmic::theme::Container::Card)
                .into(),
        );
    }
    if let Some(message) = &state.message {
        body.push(widget::text::caption(message.clone()).into());
    }
    widget::column::with_children(vec![
        widget::row::with_children(vec![
            widget::text::heading("Glossary").into(),
            widget::space::horizontal().into(),
            widget::button::icon(widget::icon::from_name("window-close-symbolic"))
                .extra_small()
                .name("Close Glossary")
                .on_press(send(GlossaryAction::Close))
                .into(),
        ])
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

/// The word Ctrl+K expands (`1380:0000` mode 1): the run of letters and
/// digits holding the caret or ending at it, at most 63 characters; its
/// byte range in `text`.
pub fn word_at(text: &str, caret: usize) -> Option<std::ops::Range<usize>> {
    let caret = caret.min(text.len());
    let mut start = caret;
    for (at, character) in text[..caret].char_indices().rev() {
        if character.is_alphanumeric() {
            start = at;
        } else {
            break;
        }
    }
    let mut end = caret;
    for (at, character) in text[caret..].char_indices() {
        if character.is_alphanumeric() {
            end = caret + at + character.len_utf8();
        } else {
            break;
        }
    }
    (start < end && text[start..end].chars().count() <= 63).then_some(start..end)
}

#[cfg(test)]
mod tests {
    use super::word_at;

    #[test]
    fn ctrl_k_takes_the_letters_and_digits_at_the_caret() {
        assert_eq!(word_at("Say hi", 6), Some(4..6));
        assert_eq!(word_at("Say hi there", 5), Some(4..6));
        assert_eq!(word_at("a sy2.", 5), Some(2..5));
        assert_eq!(word_at("About()", 7), None, "punctuation ends a name");
        assert_eq!(word_at("", 0), None);
    }
}
