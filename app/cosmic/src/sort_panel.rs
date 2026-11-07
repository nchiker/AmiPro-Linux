//! The Sort panel in the rail (`UI_SPEC.md` §41, decision TN3; run C,
//! TD.8): Ami Pro's Tools › Sort… dialog (1077) as a panel that sorts the
//! selected paragraphs or a table's rows when Sort is pressed. Three
//! levels (field, Alphanumeric or Numeric, Word 1st, 2nd, or All), one
//! order, the delimiter (text only), and the paragraphs or rows in a
//! record. The settings are kept for the next sort, as Ami Pro's dialog
//! keeps its last ones. A refusal or a question (continue, cancel) shows
//! in the panel; nothing is modal.

use amipro_editing::sort::{SortKey, SortKind, SortSettings, SortWord};
use cosmic::iced::Length;
use cosmic::widget;

use crate::Message;

pub const WIDTH: f32 = 300.0;

type Element<'a> = cosmic::Element<'a, Message>;

/// One level's entries, as typed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Level {
    /// 1 to 99; empty leaves the level out.
    pub field: String,
    pub kind: SortKind,
    pub word: SortWord,
}

/// What the panel keeps between sorts (and between openings).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Form {
    pub levels: [Level; 3],
    pub descending: bool,
    /// Tab delimited (else the character below).
    pub tab_delimited: bool,
    pub delimiter: String,
    /// Paragraphs (rows) in a record.
    pub record: String,
}

impl Default for Form {
    fn default() -> Self {
        let level = |field: &str, word| Level {
            field: field.into(),
            kind: SortKind::Alphanumeric,
            word,
        };
        Self {
            levels: [
                level("1", SortWord::All),
                level("", SortWord::First),
                level("", SortWord::First),
            ],
            descending: false,
            tab_delimited: true,
            delimiter: String::new(),
            record: "1".into(),
        }
    }
}

impl Form {
    /// The settings the entries make, or what is wrong with them.
    pub fn settings(&self, in_table: bool) -> Result<SortSettings, String> {
        let mut keys = Vec::new();
        for (index, level) in self.levels.iter().enumerate() {
            let text = level.field.trim();
            if text.is_empty() {
                if index == 0 {
                    return Err("Level 1 needs a field number from 1 to 99.".into());
                }
                continue;
            }
            let field: u8 = text
                .parse()
                .ok()
                .filter(|field| (1..=SortKey::MAX_FIELD).contains(field))
                .ok_or_else(|| {
                    format!("Level {}'s field must be a number from 1 to 99.", index + 1)
                })?;
            keys.push(SortKey {
                field,
                kind: level.kind,
                word: level.word,
            });
        }
        let record: usize = self
            .record
            .trim()
            .parse()
            .ok()
            .filter(|record| (1..=99).contains(record))
            .ok_or("The record size must be from 1 to 99.")?;
        let delimiter = if in_table || self.tab_delimited {
            None
        } else {
            let mut characters = self.delimiter.chars();
            match (characters.next(), characters.next()) {
                (Some(character), None) => Some(character),
                _ => return Err("The field delimiter is one character.".into()),
            }
        };
        Ok(SortSettings {
            keys,
            descending: self.descending,
            delimiter,
            record,
            confirmed: false,
        })
    }
}

/// A question the panel waits to have answered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ask {
    /// 3112: a page or column break in the text.
    Break,
    /// 3184: connected cells in the rows.
    Connected,
}

impl Ask {
    pub fn text(self) -> &'static str {
        match self {
            Ask::Break => "The text has a page or column break. Sort across it?",
            Ask::Connected => "The rows have connected cells. Sorting disconnects them. Continue?",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notice {
    Info(String),
    Refused(String),
    Ask(Ask),
}

/// The open panel.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SortPanel {
    pub form: Form,
    pub notice: Option<Notice>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SortAction {
    Field(usize, String),
    Kind(usize, SortKind),
    Word(usize, SortWord),
    Descending(bool),
    TabDelimited(bool),
    Delimiter(String),
    Record(String),
    Sort,
    /// Continue, to the question asked.
    Continue,
    /// Cancel the question.
    CancelAsk,
    Close,
}

fn send(action: SortAction) -> Message {
    Message::Sort(action)
}

/// What the panel shows besides its state.
pub struct Shown {
    /// "Sorting 12 paragraphs", "Sorting table rows 2–9", or why not.
    pub scope: String,
    pub can_sort: bool,
    pub in_table: bool,
}

fn header() -> Element<'static> {
    widget::row::with_children(vec![
        widget::text::heading("Sort").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Sort")
            .on_press(send(SortAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn radio<'a, T: Copy + Eq + 'a>(
    label: &'static str,
    value: T,
    selected: T,
    action: impl Fn(T) -> SortAction + 'a,
) -> Element<'a> {
    widget::radio(
        widget::text::body(label),
        value,
        Some(selected),
        move |value| send(action(value)),
    )
    .into()
}

fn wide(label: &str, suggested: bool, message: Option<Message>) -> Element<'static> {
    widget::button::custom(
        widget::container(widget::text::body(label.to_owned())).center_x(Length::Fill),
    )
    .class(if suggested {
        cosmic::theme::Button::Suggested
    } else {
        cosmic::theme::Button::Standard
    })
    .width(Length::Fill)
    .on_press_maybe(message)
    .into()
}

fn level<'a>(index: usize, level: &'a Level) -> Element<'a> {
    widget::column::with_children(vec![
        widget::row::with_children(vec![
            widget::text::body(format!("Level {}", index + 1))
                .width(Length::Fixed(60.0))
                .into(),
            widget::text::caption("Field").into(),
            widget::text_input(if index == 0 { "1" } else { "none" }, level.field.as_str())
                .on_input(move |text| send(SortAction::Field(index, text)))
                .on_submit(|_| send(SortAction::Sort))
                .width(Length::Fixed(56.0))
                .into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
        widget::row::with_children(vec![
            radio(
                "Alphanumeric",
                SortKind::Alphanumeric,
                level.kind,
                move |kind| SortAction::Kind(index, kind),
            ),
            radio("Numeric", SortKind::Numeric, level.kind, move |kind| {
                SortAction::Kind(index, kind)
            }),
        ])
        .spacing(12)
        .into(),
        widget::row::with_children(vec![
            widget::text::caption("Word").into(),
            radio("1st", SortWord::First, level.word, move |word| {
                SortAction::Word(index, word)
            }),
            radio("2nd", SortWord::Second, level.word, move |word| {
                SortAction::Word(index, word)
            }),
            radio("All", SortWord::All, level.word, move |word| {
                SortAction::Word(index, word)
            }),
        ])
        .spacing(10)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    ])
    .spacing(4)
    .into()
}

pub fn panel<'a>(state: &'a SortPanel, shown: &Shown) -> Element<'a> {
    let form = &state.form;
    let mut body: Vec<Element<'a>> = vec![widget::text::caption(shown.scope.clone()).into()];
    for (index, entry) in form.levels.iter().enumerate() {
        body.push(level(index, entry));
        body.push(widget::divider::horizontal::default().into());
    }
    body.push(
        widget::row::with_children(vec![
            widget::text::heading("Order").into(),
            radio("Ascending", false, form.descending, SortAction::Descending),
            radio("Descending", true, form.descending, SortAction::Descending),
        ])
        .spacing(10)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    );
    if shown.in_table {
        body.push(
            widget::text::caption("Whole rows move; the delimiter is not used in a table.").into(),
        );
    } else {
        body.push(widget::text::heading("Delimiter").into());
        body.push(
            widget::row::with_children(vec![
                radio("Tab", true, form.tab_delimited, SortAction::TabDelimited),
                radio(
                    "Character",
                    false,
                    form.tab_delimited,
                    SortAction::TabDelimited,
                ),
                widget::text_input("", form.delimiter.as_str())
                    .on_input(|text| send(SortAction::Delimiter(text)))
                    .on_submit(|_| send(SortAction::Sort))
                    .width(Length::Fixed(40.0))
                    .into(),
            ])
            .spacing(10)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
    }
    body.push(
        widget::row::with_children(vec![
            widget::text::body(if shown.in_table {
                "Rows in a record"
            } else {
                "Paragraphs in a record"
            })
            .into(),
            widget::text_input("1", form.record.as_str())
                .on_input(|text| send(SortAction::Record(text)))
                .on_submit(|_| send(SortAction::Sort))
                .width(Length::Fixed(56.0))
                .into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    );
    body.push(wide(
        "Sort",
        true,
        shown.can_sort.then(|| send(SortAction::Sort)),
    ));
    match &state.notice {
        Some(Notice::Ask(ask)) => {
            body.push(widget::text::body(ask.text()).into());
            body.push(
                widget::row::with_children(vec![
                    wide("Continue", true, Some(send(SortAction::Continue))),
                    wide("Cancel", false, Some(send(SortAction::CancelAsk))),
                ])
                .spacing(6)
                .into(),
            );
        }
        Some(Notice::Refused(text) | Notice::Info(text)) => {
            body.push(widget::text::body(text.clone()).into());
        }
        None => {}
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entries_make_the_settings_or_say_what_is_wrong() {
        let mut form = Form::default();
        let settings = form.settings(false).unwrap();
        assert_eq!(settings.keys.len(), 1);
        assert_eq!(settings.keys[0].word, SortWord::All);
        assert_eq!(settings.delimiter, None);
        form.levels[1].field = "2".into();
        form.levels[1].kind = SortKind::Numeric;
        assert_eq!(form.settings(false).unwrap().keys.len(), 2);
        form.levels[2].field = "100".into();
        assert!(form.settings(false).is_err());
        form.levels[2].field = String::new();
        form.tab_delimited = false;
        assert!(form.settings(false).is_err(), "a character is needed");
        form.delimiter = ",".into();
        assert_eq!(form.settings(false).unwrap().delimiter, Some(','));
        assert_eq!(
            form.settings(true).unwrap().delimiter,
            None,
            "not in tables"
        );
        form.levels[0].field = String::new();
        assert!(form.settings(false).is_err());
    }
}
