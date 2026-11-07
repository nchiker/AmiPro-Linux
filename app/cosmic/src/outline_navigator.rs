//! The Outline navigator in the rail (`UI_SPEC.md` §40, OM1; run B, OB.6):
//! the main text's headings as a tree, indented by level. A click goes to
//! the heading; a drag moves its section to stand before the heading it is
//! dropped on (or at the end), one undo step; the chevrons contract and
//! expand sections in step with Outline mode (the tab's `OutlineState`,
//! OM2); with the navigator holding the keyboard (after a click on it),
//! Up and Down go to the heading before or after, Left contracts, Right
//! expands, and Alt+Up and Alt+Down move the section as anywhere else.

use cosmic::iced::Length;
use cosmic::{Element, widget};

use amipro_document::Document;
use amipro_editing::sections;

use crate::Message;
use crate::outline_view::OutlineState;

/// The navigator's width in the rail.
pub const WIDTH: f32 = 300.0;

/// The navigator's state: a drag under way, and whether it holds the keys.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutlineNavigator {
    /// The heading (paragraph index) a press began a drag on.
    pub pressed: Option<usize>,
    /// The row the pointer is over while dragging.
    pub over: Option<Target>,
    /// Up, Down, Left, and Right go to the navigator (after a click on it).
    pub keys: bool,
}

/// Where a drop puts a section: before a heading, or at the end.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    Before(usize),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NavigatorAction {
    Press(usize),
    Over(Target),
    Release(Target),
    /// The chevron: contract or expand that heading.
    Toggle(usize),
    Close,
}

/// One heading in the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub paragraph: usize,
    pub level: u8,
    pub text: String,
    /// Text under it (a section of more than the heading).
    pub has_children: bool,
    /// Its section is contracted.
    pub collapsed: bool,
}

/// The headings the tree lists: those Outline mode would show (Contract
/// and Show levels hide them in both), with their state.
pub fn entries(document: &Document, state: &OutlineState) -> Vec<Entry> {
    let main = document.main_flow_id();
    let levels = sections::levels(document, main);
    let hidden = state.hidden_flags(document);
    let Some(flow) = document.flow(main) else {
        return Vec::new();
    };
    levels
        .iter()
        .enumerate()
        .filter(|&(index, &level)| level > 0 && !hidden[index])
        .map(|(index, &level)| {
            let text: String = flow.paragraphs[index]
                .text
                .chars()
                .filter(|character| *character != amipro_document::OBJECT_REPLACEMENT_CHARACTER)
                .take(60)
                .collect();
            let has_children = sections::section(&levels, index).len() > 1;
            Entry {
                paragraph: index,
                level,
                text: if text.trim().is_empty() {
                    "(empty heading)".into()
                } else {
                    text
                },
                has_children,
                collapsed: has_children && hidden.get(index + 1).copied().unwrap_or(false),
            }
        })
        .collect()
}

/// The heading whose section holds main paragraph `paragraph`: the
/// nearest heading at or before it.
pub fn current(entries: &[Entry], paragraph: usize) -> Option<usize> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.paragraph <= paragraph)
        .map(|entry| entry.paragraph)
}

/// The current heading's row (and the end drop area under a drag): the
/// accent, faintly, as a selected list item.
fn highlight() -> cosmic::theme::Container<'static> {
    cosmic::theme::Container::custom(|theme| {
        let accent = theme.cosmic().accent_color();
        cosmic::iced::widget::container::Style {
            background: Some(
                cosmic::iced::Color::from_rgba(accent.red, accent.green, accent.blue, 0.2).into(),
            ),
            border: cosmic::iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
}

/// The line showing where a dragged section will go.
fn accent_line() -> cosmic::theme::Container<'static> {
    cosmic::theme::Container::custom(|theme| {
        let accent = theme.cosmic().accent_color();
        cosmic::iced::widget::container::Style {
            background: Some(cosmic::iced::Color::from(accent).into()),
            ..Default::default()
        }
    })
}

fn send(action: NavigatorAction) -> Message {
    Message::OutlineNavigator(action)
}

pub fn panel<'a>(
    state: &OutlineNavigator,
    entries: &[Entry],
    current: Option<usize>,
) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Outline").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close the Outline navigator")
            .on_press(send(NavigatorAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut rows: Vec<Element<'a, Message>> = vec![header.into()];
    if entries.is_empty() {
        rows.push(
            widget::text::body(
                "No headings: give a style an outline level (Format › Bullets and Numbering ▸ Promote, or Style Properties) to list its paragraphs here.",
            )
            .into(),
        );
    }
    let dragging = state.pressed.is_some();
    for entry in entries {
        let paragraph = entry.paragraph;
        let chevron: Element<'a, Message> = if entry.has_children {
            widget::button::icon(widget::icon::from_name(if entry.collapsed {
                "pan-end-symbolic"
            } else {
                "pan-down-symbolic"
            }))
            .extra_small()
            .name(if entry.collapsed {
                "Expand"
            } else {
                "Contract"
            })
            .on_press(send(NavigatorAction::Toggle(paragraph)))
            .into()
        } else {
            widget::space().width(Length::Fixed(24.0)).into()
        };
        let selected = current == Some(paragraph);
        let drop_here = dragging
            && state.over == Some(Target::Before(paragraph))
            && state.pressed != Some(paragraph);
        let label = widget::container(
            widget::text::body(entry.text.clone())
                .wrapping(cosmic::iced::widget::text::Wrapping::None),
        )
        .padding([4, 6])
        .width(Length::Fill)
        .class(if selected {
            highlight()
        } else {
            cosmic::theme::Container::Transparent
        });
        let label = widget::mouse_area(label)
            .on_press(send(NavigatorAction::Press(paragraph)))
            .on_release(send(NavigatorAction::Release(Target::Before(paragraph))))
            .on_enter(send(NavigatorAction::Over(Target::Before(paragraph))));
        let row = widget::row::with_children(vec![
            widget::space()
                .width(Length::Fixed(f32::from(entry.level - 1) * 16.0))
                .into(),
            chevron,
            label.into(),
        ])
        .align_y(cosmic::iced::Alignment::Center);
        if drop_here {
            // Where the section will go: a line above the row.
            rows.push(
                widget::container(widget::space().height(Length::Fixed(2.0)))
                    .width(Length::Fill)
                    .class(accent_line())
                    .into(),
            );
        }
        rows.push(row.into());
    }
    if !entries.is_empty() {
        let end = widget::mouse_area(
            widget::container(widget::text::caption(
                "Drag a heading onto another to move its section before it, or here to move it to the end.",
            ))
            .padding([8, 6])
            .width(Length::Fill)
            .class(if dragging && state.over == Some(Target::End) {
                highlight()
            } else {
                cosmic::theme::Container::Transparent
            }),
        )
        .on_release(send(NavigatorAction::Release(Target::End)))
        .on_enter(send(NavigatorAction::Over(Target::End)));
        rows.push(end.into());
    }
    widget::scrollable(
        widget::column::with_children(rows)
            .spacing(2)
            .padding([8, 8]),
    )
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tree_lists_the_headings_outline_mode_shows() {
        let document = crate::page_surface_app_tests::outline_fixture();
        let mut state = OutlineState::default();
        let listed = entries(&document, &state);
        let paragraphs: Vec<usize> = listed.iter().map(|entry| entry.paragraph).collect();
        assert_eq!(paragraphs, [0, 1, 4, 6, 7]);
        assert_eq!(listed[0].level, 1);
        assert!(listed[0].has_children && !listed[0].collapsed);
        assert_eq!(current(&listed, 3), Some(1), "Body under A is Subhead A's");
        assert_eq!(current(&listed, 5), Some(4));
        state.contract(&document, 0);
        let listed = entries(&document, &state);
        let paragraphs: Vec<usize> = listed.iter().map(|entry| entry.paragraph).collect();
        assert_eq!(paragraphs, [0, 6, 7]);
        assert!(listed[0].collapsed);
    }
}
