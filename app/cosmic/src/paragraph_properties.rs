//! Format › Paragraph… (`UI_SPEC.md` §39, CM3): Paragraph Properties in the
//! rail for the caret's or the selected paragraphs: Ami Pro's Text ›
//! Indention… and Spacing… as sections, with alignment, live; each change
//! one undo step, Revert to style per section. What SAM keeps per
//! paragraph is set here (`<:I…>`, `<:S…>`, `<+@ABC>`); Above, Below, and
//! the breaks are the style's and shown from it.

use std::collections::BTreeSet;

use amipro_document::{Alignment, Breaks, Indents, LineSpacing, Twips};
use amipro_editing::{Command, IndentChange};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::style_properties::{inches, parse_length};

pub const WIDTH: f32 = 300.0;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Section {
    Indention,
    Spacing,
    Alignment,
    Breaks,
}

impl Section {
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::Indention, "Indention"),
        (Self::Spacing, "Spacing"),
        (Self::Alignment, "Alignment"),
        (Self::Breaks, "Breaks"),
    ];
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Field {
    All,
    First,
    Rest,
    Right,
    Custom,
}

const FIELDS: [Field; 5] = [
    Field::All,
    Field::First,
    Field::Rest,
    Field::Right,
    Field::Custom,
];

/// The first selected paragraph's values: its own, else its style's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    pub style: String,
    pub paragraphs: usize,
    pub indents: Indents,
    pub own_indents: bool,
    pub spacing: Option<LineSpacing>,
    pub own_spacing: bool,
    pub alignment: Option<Alignment>,
    pub own_alignment: bool,
    pub above: Twips,
    pub below: Twips,
    pub breaks: u16,
}

impl Shown {
    fn text(&self, field: Field) -> String {
        match field {
            Field::All => inches(self.indents.all),
            Field::First => inches(self.indents.first),
            Field::Rest => inches(self.indents.rest),
            Field::Right => inches(self.indents.right),
            Field::Custom => match self.spacing {
                Some(LineSpacing::Custom(height)) => inches(height),
                _ => String::new(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphProperties {
    pub open: BTreeSet<Section>,
    pub shown: Shown,
    /// The boxes as typed, until Enter or leaving them.
    pub texts: [String; 5],
    /// The first paragraph the panel shows, to refresh the boxes when the
    /// caret moves to another.
    pub paragraph: Option<amipro_document::ParagraphId>,
}

impl ParagraphProperties {
    pub fn new(section: Section) -> Self {
        let shown = Shown::default();
        Self {
            open: BTreeSet::from([section]),
            texts: FIELDS.map(|field| shown.text(field)),
            shown,
            paragraph: None,
        }
    }

    /// Shows `shown` for `paragraph`; the boxes take its values unless they
    /// are the same paragraph's and the values did not change.
    pub fn sync(&mut self, shown: Shown, paragraph: Option<amipro_document::ParagraphId>) {
        if self.paragraph != paragraph || self.shown != shown {
            for (index, field) in FIELDS.into_iter().enumerate() {
                self.texts[index] = shown.text(field);
            }
        }
        self.paragraph = paragraph;
        self.shown = shown;
    }

    fn text(&self, field: Field) -> &str {
        &self.texts[FIELDS.iter().position(|known| *known == field).unwrap_or(0)]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParagraphAction {
    Section(Section),
    Text(Field, String),
    Submit(Field),
    /// Single, 1 1/2, Double, Custom.
    LineSpacing(usize),
    Align(Alignment),
    Revert(Section),
    /// The style's own settings: Style Properties.
    ModifyStyle,
    Close,
}

/// The command a submitted box makes for paragraphs `first..=last` of
/// `flow`, if its text is a length that changes something.
pub fn submitted(
    state: &ParagraphProperties,
    field: Field,
    flow: amipro_document::FlowId,
    first: usize,
    last: usize,
) -> Option<Command> {
    let value = parse_length(state.text(field))?;
    if state.text(field) == state.shown.text(field) {
        return None;
    }
    let change = |change: IndentChange| Command::SetParagraphIndents {
        flow,
        first,
        last,
        indents: change,
    };
    Some(match field {
        Field::All => change(IndentChange {
            all: Some(value),
            ..IndentChange::default()
        }),
        Field::First => change(IndentChange {
            first_line: Some(value),
            ..IndentChange::default()
        }),
        Field::Rest => change(IndentChange {
            rest: Some(value),
            ..IndentChange::default()
        }),
        Field::Right => change(IndentChange {
            right: Some(value),
            ..IndentChange::default()
        }),
        Field::Custom => Command::SetParagraphLineSpacing {
            flow,
            first,
            last,
            spacing: Some(LineSpacing::Custom(value.max(1))),
        },
    })
}

/// The line spacing of choice `index` (Single, 1 1/2, Double, Custom), a
/// Custom keeping the height shown or taking the style's single line.
pub fn line_spacing(index: usize, shown: &Shown) -> LineSpacing {
    match index {
        0 => LineSpacing::Single,
        1 => LineSpacing::OneAndAHalf(0),
        2 => LineSpacing::Double(0),
        _ => match shown.spacing {
            Some(LineSpacing::Custom(height)) => LineSpacing::Custom(height),
            _ => LineSpacing::Custom(240),
        },
    }
}

fn send(action: ParagraphAction) -> Message {
    Message::ParagraphProperties(action)
}

fn input(state: &ParagraphProperties, field: Field) -> Element<'static, Message> {
    widget::text_input("", state.text(field).to_owned())
        .width(Length::Fixed(72.0))
        .on_input(move |text| send(ParagraphAction::Text(field, text)))
        .on_submit(move |_| send(ParagraphAction::Submit(field)))
        .on_unfocus(send(ParagraphAction::Submit(field)))
        .into()
}

fn row(children: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    widget::row::with_children(children)
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

fn length_row(
    state: &ParagraphProperties,
    label: &'static str,
    field: Field,
) -> Element<'static, Message> {
    row(vec![
        widget::text::body(label).width(Length::Fixed(88.0)).into(),
        input(state, field),
        widget::text::caption("in.").into(),
    ])
}

fn origin(own: bool) -> Element<'static, Message> {
    widget::text::caption(if own {
        "Set for this paragraph"
    } else {
        "From the style"
    })
    .into()
}

fn revert(section: Section, own: bool) -> Element<'static, Message> {
    widget::button::text("Revert to style")
        .on_press_maybe(own.then_some(send(ParagraphAction::Revert(section))))
        .into()
}

fn indention(state: &ParagraphProperties) -> Vec<Element<'static, Message>> {
    vec![
        origin(state.shown.own_indents),
        length_row(state, "All", Field::All),
        length_row(state, "First", Field::First),
        length_row(state, "Rest", Field::Rest),
        length_row(state, "From right", Field::Right),
        widget::text::caption("First and Rest add to All.").into(),
        revert(Section::Indention, state.shown.own_indents),
    ]
}

fn spacing(state: &ParagraphProperties) -> Vec<Element<'static, Message>> {
    let line = match state.shown.spacing {
        None | Some(LineSpacing::Single) => 0,
        Some(LineSpacing::OneAndAHalf(_)) => 1,
        Some(LineSpacing::Double(_)) => 2,
        Some(LineSpacing::Custom(_)) => 3,
    };
    let choice = |label: &'static str, index: usize| -> Element<'static, Message> {
        widget::radio(label, index, Some(line), |index| {
            send(ParagraphAction::LineSpacing(index))
        })
        .into()
    };
    let mut custom = vec![
        widget::container(choice("Custom", 3))
            .width(Length::Fixed(88.0))
            .into(),
    ];
    if line == 3 {
        custom.push(input(state, Field::Custom));
        custom.push(widget::text::caption("in.").into());
    }
    vec![
        origin(state.shown.own_spacing),
        row(vec![
            choice("Single", 0),
            choice("1 1/2", 1),
            choice("Double", 2),
        ]),
        row(custom),
        widget::text::caption(format!(
            "Above {} in., below {} in.: the style's.",
            inches(state.shown.above),
            inches(state.shown.below)
        ))
        .into(),
        revert(Section::Spacing, state.shown.own_spacing),
    ]
}

fn alignment(state: &ParagraphProperties) -> Vec<Element<'static, Message>> {
    let current = state.shown.alignment;
    let align = |label: &'static str, alignment: Alignment| -> Element<'static, Message> {
        widget::radio(label, alignment, current, |alignment| {
            send(ParagraphAction::Align(alignment))
        })
        .into()
    };
    vec![
        origin(state.shown.own_alignment),
        row(vec![
            align("Left", Alignment::Left),
            align("Center", Alignment::Centre),
        ]),
        row(vec![
            align("Right", Alignment::Right),
            align("Justify", Alignment::Justify),
        ]),
        revert(Section::Alignment, state.shown.own_alignment),
    ]
}

fn breaks(state: &ParagraphProperties) -> Vec<Element<'static, Message>> {
    const BOXES: [(u16, &str); 5] = [
        (Breaks::PAGE_BEFORE, "Page break before"),
        (Breaks::PAGE_AFTER, "Page break after"),
        (Breaks::COLUMN_BEFORE, "Column break before"),
        (Breaks::COLUMN_AFTER, "Column break after"),
        (Breaks::WITHIN, "Allow break within"),
    ];
    let mut children: Vec<Element<'static, Message>> = BOXES
        .iter()
        .map(|(bit, label)| {
            widget::checkbox(state.shown.breaks & bit != 0)
                .label(*label)
                .into()
        })
        .collect();
    children.push(
        widget::text::caption(
            "Ami Pro keeps breaks in the style: change them in Style Properties.",
        )
        .into(),
    );
    children
}

pub fn panel(state: &ParagraphProperties) -> Element<'_, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Paragraph Properties").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Paragraph Properties")
            .on_press(send(ParagraphAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let shown = &state.shown;
    let mut body: Vec<Element<'_, Message>> = vec![
        widget::text::body(if shown.paragraphs > 1 {
            format!(
                "{} paragraphs; the first in {}",
                shown.paragraphs, shown.style
            )
        } else {
            format!("A paragraph in {}", shown.style)
        })
        .into(),
    ];
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
            .on_press(send(ParagraphAction::Section(section)))
            .into(),
        );
        if open {
            let fields = match section {
                Section::Indention => indention(state),
                Section::Spacing => spacing(state),
                Section::Alignment => alignment(state),
                Section::Breaks => breaks(state),
            };
            body.push(
                widget::container(widget::column::with_children(fields).spacing(6))
                    .padding([0, 0, 8, 22])
                    .into(),
            );
        }
    }
    body.push(
        widget::button::standard("Modify Style…")
            .on_press(send(ParagraphAction::ModifyStyle))
            .into(),
    );
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

impl crate::AmiProApp {
    /// Format › Paragraph… (and Indention…, Spacing…): the panel at
    /// `section`, in place of the rail's other panels.
    pub(crate) fn open_paragraph_properties(&mut self, section: Section) {
        self.close_rail_panels();
        self.paragraph_properties = Some(ParagraphProperties::new(section));
        self.open_rail();
        self.sync_paragraph_properties();
    }

    /// Format › Increase Indent and Decrease Indent (Ctrl+], Ctrl+[): the
    /// selected paragraphs' All indent half an inch more or less, from the
    /// first one's, as one undo step.
    pub(crate) fn step_indent(&mut self, more: bool) {
        const STEP: Twips = 720;
        let session = &self.sessions[self.active_session];
        if session.frame_selection.is_some() {
            return;
        }
        let (start, end) = session.page_selection.ordered();
        let document = &session.document;
        let Some(paragraph) = document
            .flow(start.flow)
            .and_then(|flow| flow.paragraphs.get(start.paragraph))
        else {
            return;
        };
        let all = paragraph
            .format
            .indents
            .or(document
                .styles()
                .get(&paragraph.style)
                .map(|style| style.indents))
            .unwrap_or_default()
            .all;
        let next = if more {
            (all + STEP).min(31_680)
        } else {
            (all - STEP).max(0)
        };
        if next == all {
            return;
        }
        self.run_ruler_command(&Command::SetParagraphIndents {
            flow: start.flow,
            first: start.paragraph,
            last: end.paragraph,
            indents: IndentChange {
                all: Some(next),
                ..IndentChange::default()
            },
        });
    }

    /// The panel follows the caret; it closes with the rail or a frame
    /// selection.
    pub(crate) fn sync_paragraph_properties(&mut self) {
        if self.paragraph_properties.is_none() {
            return;
        }
        if !self.rail_open() || self.frame_selection.is_some() {
            self.paragraph_properties = None;
            return;
        }
        let session = &self.sessions[self.active_session];
        let (start, end) = session.page_selection.ordered();
        let document = &session.document;
        let Some(paragraph) = document
            .flow(start.flow)
            .and_then(|flow| flow.paragraphs.get(start.paragraph))
        else {
            return;
        };
        let style = document.styles().get(&paragraph.style);
        let format = &paragraph.format;
        let shown = Shown {
            style: style.map_or_else(String::new, |style| style.name.clone()),
            paragraphs: end.paragraph.saturating_sub(start.paragraph) + 1,
            indents: format
                .indents
                .or(style.map(|style| style.indents))
                .unwrap_or_default(),
            own_indents: format.indents.is_some(),
            spacing: format
                .line_spacing
                .or(style.map(|style| style.spacing.line)),
            own_spacing: format.line_spacing.is_some(),
            alignment: format.alignment.or(style.map(|style| style.alignment)),
            own_alignment: format.alignment.is_some(),
            above: style.map_or(0, |style| style.spacing.above),
            below: style.map_or(0, |style| style.spacing.below),
            breaks: style.map_or(0, |style| style.breaks.0),
        };
        let id = paragraph.id;
        if let Some(state) = self.sessions[self.active_session]
            .paragraph_properties
            .as_mut()
        {
            state.sync(shown, Some(id));
        }
    }

    pub(crate) fn update_paragraph_properties(&mut self, action: ParagraphAction) {
        let session = &mut self.sessions[self.active_session];
        let Some(state) = session.paragraph_properties.as_mut() else {
            return;
        };
        let (start, end) = session.page_selection.ordered();
        let (flow, first, last) = (start.flow, start.paragraph, end.paragraph);
        let command = match action {
            ParagraphAction::Section(section) => {
                if !state.open.remove(&section) {
                    state.open.insert(section);
                }
                None
            }
            ParagraphAction::Text(field, text) => {
                if let Some(index) = FIELDS.iter().position(|known| *known == field) {
                    state.texts[index] = text;
                }
                None
            }
            ParagraphAction::Submit(field) => {
                let command = submitted(state, field, flow, first, last);
                if command.is_none() {
                    // A text that is not a length goes back to the value.
                    let index = FIELDS.iter().position(|known| *known == field).unwrap_or(0);
                    state.texts[index] = state.shown.text(field);
                }
                command
            }
            ParagraphAction::LineSpacing(index) => Some(Command::SetParagraphLineSpacing {
                flow,
                first,
                last,
                spacing: Some(line_spacing(index, &state.shown)),
            }),
            ParagraphAction::Align(alignment) => Some(Command::SetParagraphAlignment {
                flow,
                first,
                last,
                alignment: Some(alignment),
            }),
            ParagraphAction::Revert(Section::Indention) => {
                Some(Command::RevertParagraphIndents { flow, first, last })
            }
            ParagraphAction::Revert(Section::Spacing) => Some(Command::SetParagraphLineSpacing {
                flow,
                first,
                last,
                spacing: None,
            }),
            ParagraphAction::Revert(Section::Alignment) => Some(Command::SetParagraphAlignment {
                flow,
                first,
                last,
                alignment: None,
            }),
            ParagraphAction::Revert(Section::Breaks) => None,
            ParagraphAction::ModifyStyle => {
                session.paragraph_properties = None;
                self.open_style_properties(None);
                return;
            }
            ParagraphAction::Close => {
                session.paragraph_properties = None;
                return;
            }
        };
        if let Some(command) = command {
            self.run_ruler_command(&command);
            self.sync_paragraph_properties();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_length_makes_one_indent_or_spacing_command() {
        let mut state = ParagraphProperties::new(Section::Indention);
        state.sync(Shown::default(), None);
        let flow = amipro_document::Document::default().main_flow_id();
        assert_eq!(state.text(Field::All), "0.00");
        // Unchanged or not a length: nothing.
        assert_eq!(submitted(&state, Field::All, flow, 0, 0), None);
        state.texts[0] = "x".into();
        assert_eq!(submitted(&state, Field::All, flow, 0, 0), None);
        state.texts[0] = "0.5".into();
        assert_eq!(
            submitted(&state, Field::All, flow, 0, 2),
            Some(Command::SetParagraphIndents {
                flow,
                first: 0,
                last: 2,
                indents: IndentChange {
                    all: Some(720),
                    ..IndentChange::default()
                },
            })
        );
        state.texts[4] = "0.25".into();
        assert_eq!(
            submitted(&state, Field::Custom, flow, 1, 1),
            Some(Command::SetParagraphLineSpacing {
                flow,
                first: 1,
                last: 1,
                spacing: Some(LineSpacing::Custom(360)),
            })
        );
        assert_eq!(line_spacing(2, &state.shown), LineSpacing::Double(0));
        assert_eq!(line_spacing(3, &state.shown), LineSpacing::Custom(240));
    }
}
