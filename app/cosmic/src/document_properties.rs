//! Document Properties in the context rail (`UI_SPEC.md` §28, decision
//! FL3): Ami Pro's File › Doc Info… edited beside the document, as Page
//! Layout Properties edits a layout (§18). A typed text applies on Enter or
//! when its box loses focus, as one `SetDocumentInfo` command and one undo
//! step; the statistics are counted as the document changes, as Ami Pro's
//! Update counts them (`docs/research/FIELDS_NOTES.md`, "Doc Info").

use std::collections::BTreeSet;

use amipro_document::{DateSettings, Document, DocumentInfo, WallClock, format_date, format_time};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;

/// The panel's folding sections.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Section {
    Description,
    Fields,
    Statistics,
    History,
    File,
}

impl Section {
    pub const ALL: [(Self, &'static str); 5] = [
        (Self::Description, "Description"),
        (Self::Fields, "Fields"),
        (Self::Statistics, "Statistics"),
        (Self::History, "History"),
        (Self::File, "File"),
    ];
}

/// A text the user edits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Text {
    Description,
    Keywords,
    Value(usize),
    Name(usize),
}

#[derive(Clone, Debug)]
pub enum PropertiesAction {
    Section(Section),
    Text(Text, String),
    Commit,
    RunFrameMacros(bool),
    Back,
    Close,
}

pub enum Outcome {
    Stay,
    Back,
    Close,
    Apply(Box<DocumentInfo>),
}

/// What the panel shows besides the Doc Info: the file and this session.
pub struct Facts {
    pub file_name: String,
    pub folder: String,
    pub pages: u32,
    /// This session's last save, in Ami Pro's time base.
    pub saved: Option<u32>,
    pub session_minutes: u32,
}

/// The panel's state: the open sections, the texts as typed, and the
/// counts as of the last change.
pub struct DocumentProperties {
    open: BTreeSet<Section>,
    description: String,
    keywords: String,
    values: [String; 8],
    names: [String; 8],
    /// The Doc Info the texts came from.
    shown: DocumentInfo,
    words: u32,
    characters: u32,
}

/// The rail's width while the panel shows.
pub const WIDTH: f32 = 300.0;

/// Ami Pro's limits (`DESCRIPTIONPROC`, `OTHERFLDSDLGPROC`).
const DESCRIPTION_LIMIT: usize = 119;
const FIELD_LIMIT: usize = 31;

impl DocumentProperties {
    pub fn new(document: &Document, section: Section) -> Self {
        let mut state = Self {
            open: BTreeSet::from([section]),
            description: String::new(),
            keywords: String::new(),
            values: Default::default(),
            names: Default::default(),
            shown: DocumentInfo::default(),
            words: 0,
            characters: 0,
        };
        state.read(document.info());
        state.sync(document);
        state
    }

    fn read(&mut self, info: &DocumentInfo) {
        self.description = info.description.clone();
        self.keywords = info.keywords.clone();
        self.values = info.fields.clone();
        self.names = std::array::from_fn(|index| info.field_name(index));
        self.shown = info.clone();
    }

    /// After every message: the texts follow the Doc Info when it changed
    /// (an undo, another edit), and the counts are counted again.
    pub fn sync(&mut self, document: &Document) {
        if *document.info() != self.shown {
            self.read(document.info());
        }
        if self.open.contains(&Section::Statistics) {
            (self.words, self.characters) = amipro_document::count_words_and_characters(document);
        }
    }

    pub fn update(&mut self, action: PropertiesAction, document: &Document) -> Outcome {
        match action {
            PropertiesAction::Section(section) => {
                if !self.open.remove(&section) {
                    self.open.insert(section);
                    self.sync(document);
                }
                Outcome::Stay
            }
            PropertiesAction::Text(text, value) => {
                match text {
                    Text::Description => self.description = value,
                    Text::Keywords => self.keywords = value,
                    Text::Value(index) => self.values[index] = value,
                    Text::Name(index) => self.names[index] = value,
                }
                Outcome::Stay
            }
            PropertiesAction::Commit => {
                let mut info = document.info().clone();
                info.description = self.description.clone();
                info.keywords = self.keywords.clone();
                info.fields = self.values.clone();
                for (index, name) in self.names.iter().enumerate() {
                    // A field's default name is stored as Ami Pro writes it.
                    info.field_names[index] = if name.trim().is_empty() {
                        format!("Field{}", index + 1)
                    } else {
                        name.clone()
                    };
                }
                // Names only Ami Pro's defaults: as read, so that nothing
                // changes until something is typed.
                let unchanged_names =
                    (0..8).all(|index| document.info().field_name(index) == info.field_name(index));
                if unchanged_names {
                    info.field_names = document.info().field_names.clone();
                }
                if info == *document.info() {
                    Outcome::Stay
                } else {
                    Outcome::Apply(Box::new(info))
                }
            }
            PropertiesAction::RunFrameMacros(on) => {
                let mut info = document.info().clone();
                info.flags = (info.flags & !1) | u32::from(on);
                Outcome::Apply(Box::new(info))
            }
            PropertiesAction::Back => Outcome::Back,
            PropertiesAction::Close => Outcome::Close,
        }
    }
}

fn send(action: PropertiesAction) -> Message {
    Message::DocumentProperties(action)
}

fn input(value: &str, text: Text, limit: usize) -> Element<'static, Message> {
    widget::text_input("", value.to_owned())
        .width(Length::Fill)
        .on_input(move |typed: String| {
            send(PropertiesAction::Text(
                text,
                typed.chars().take(limit).collect(),
            ))
        })
        .on_submit(|_| send(PropertiesAction::Commit))
        .on_unfocus(send(PropertiesAction::Commit))
        .into()
}

fn line(label: &str, value: String) -> Element<'static, Message> {
    widget::row::with_children(vec![
        widget::text::body(label.to_owned())
            .width(Length::Fixed(96.0))
            .into(),
        widget::text::body(value).width(Length::Fill).into(),
    ])
    .spacing(6)
    .into()
}

/// A stored time as the panel shows it: the date in words and the time,
/// the year as it is (Ami Pro's dialog shows 2026 as "126").
fn when(seconds: Option<u32>) -> String {
    match seconds {
        Some(seconds) => {
            let clock = WallClock::from_ami_pro(seconds);
            format!(
                "{}  {}",
                format_date(&clock, b'd', &DateSettings::US),
                format_time(&clock, &DateSettings::US).trim()
            )
        }
        None => "Not yet".into(),
    }
}

fn minutes(total: u32) -> String {
    match (total / 60, total % 60) {
        (0, 1) => "1 minute".into(),
        (0, minutes) => format!("{minutes} minutes"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

fn thousands(number: u32) -> String {
    let digits = number.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

pub fn panel<'a>(
    state: &DocumentProperties,
    document: &Document,
    facts: Facts,
) -> Element<'a, Message> {
    let info = document.info();
    let header = widget::row::with_children(vec![
        widget::text::heading("Document Properties").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(PropertiesAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Document Properties")
            .on_press(send(PropertiesAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut body: Vec<Element<'a, Message>> = Vec::new();
    for (section, title) in Section::ALL {
        let open = state.open.contains(&section);
        body.push(
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
            .on_press(send(PropertiesAction::Section(section)))
            .into(),
        );
        if !open {
            continue;
        }
        let rows: Vec<Element<'a, Message>> = match section {
            Section::Description => vec![
                input(&state.description, Text::Description, DESCRIPTION_LIMIT),
                widget::text::caption(format!(
                    "{} of {DESCRIPTION_LIMIT} characters",
                    state.description.chars().count()
                ))
                .into(),
                widget::text::body("Keywords").into(),
                input(&state.keywords, Text::Keywords, 499),
            ],
            Section::Fields => (0..8)
                .flat_map(|index| {
                    [
                        widget::row::with_children(vec![
                            widget::container(input(&state.names[index], Text::Name(index), FIELD_LIMIT))
                                .width(Length::Fixed(110.0))
                                .into(),
                            input(&state.values[index], Text::Value(index), FIELD_LIMIT),
                        ])
                        .spacing(6)
                        .into(),
                    ]
                })
                .chain(std::iter::once(
                    widget::text::caption(
                        "The name on the left, the value on the right; Ami Pro keeps 31 characters of each.",
                    )
                    .into(),
                ))
                .collect(),
            Section::Statistics => vec![
                line("Pages", thousands(facts.pages)),
                line("Words", thousands(state.words)),
                line("Characters", thousands(state.characters)),
                line("Size", format!("{} K", info.statistics.size)),
                widget::text::caption("Counted as you type and written at each save.").into(),
            ],
            Section::History => vec![
                line("Created", when(info.created)),
                line("Last saved", when(facts.saved.or(info.saved))),
                line("Revisions", info.revisions.to_string()),
                line(
                    "Editing time",
                    minutes(info.editing_minutes.saturating_add(facts.session_minutes)),
                ),
            ],
            Section::File => {
                let mut rows = vec![
                    line(
                        "File",
                        if facts.file_name.is_empty() {
                            "Untitled".into()
                        } else {
                            facts.file_name.clone()
                        },
                    ),
                    line("Folder", facts.folder.clone()),
                    line(
                        "Style sheet",
                        document
                            .style_sheet()
                            .map_or_else(|| "None".into(), |link| link.name.clone()),
                    ),
                    widget::checkbox(info.runs_frame_macros())
                        .label("Run frame macros")
                        .on_toggle(|on| send(PropertiesAction::RunFrameMacros(on)))
                        .into(),
                    widget::text::caption("Kept for Ami Pro: this app runs no macros.").into(),
                ];
                if !info.annotation_lock.is_empty() {
                    rows.push(
                        widget::text::caption(format!(
                            "Locked for annotations by {}.",
                            info.annotation_lock
                        ))
                        .into(),
                    );
                }
                rows
            }
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> Document {
        let mut document = Document::from_plain_text("One two three");
        *document.info_mut() = DocumentInfo::new(1_792_520_631);
        document
    }

    #[test]
    fn typed_texts_apply_as_one_change_on_commit() {
        let document = document();
        let mut state = DocumentProperties::new(&document, Section::Description);
        state.update(
            PropertiesAction::Text(Text::Description, "Quarterly".into()),
            &document,
        );
        state.update(
            PropertiesAction::Text(Text::Name(0), "Client".into()),
            &document,
        );
        state.update(
            PropertiesAction::Text(Text::Value(0), "Alpha".into()),
            &document,
        );
        let Outcome::Apply(info) = state.update(PropertiesAction::Commit, &document) else {
            panic!("a change");
        };
        assert_eq!(info.description, "Quarterly");
        assert_eq!(info.field_name(0), "Client");
        assert_eq!(info.fields[0], "Alpha");
        // Untouched names stay as read; committing nothing changes nothing.
        let mut fresh = DocumentProperties::new(&document, Section::Fields);
        assert!(matches!(
            fresh.update(PropertiesAction::Commit, &document),
            Outcome::Stay
        ));
    }

    #[test]
    fn the_statistics_count_as_ami_pro_counts_and_times_show_their_year() {
        let document = document();
        let state = DocumentProperties::new(&document, Section::Statistics);
        assert_eq!((state.words, state.characters), (3, 13));
        assert_eq!(
            when(Some(1_792_520_631)),
            "Tuesday, October 20, 2026  11:23 AM"
        );
        assert_eq!(minutes(134), "2 h 14 min");
        assert_eq!(thousands(1204), "1,204");
    }
}
