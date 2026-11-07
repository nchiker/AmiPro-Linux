//! Insert › Special Character… (`UI_SPEC.md` §39, CM12): the characters of
//! Ami Pro's ANSI table (Windows-1252, the user's guide's Appendix C) above
//! plain ASCII, in a grid with the recently inserted ones first; a click
//! inserts the character at the caret, over a selection, as one undo step.
//! Everything listed is stored by SAM as itself.

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;

/// Recent characters kept.
pub const RECENT: usize = 16;

/// Windows-1252's characters from 0x80 to 0xFF that are defined, as Unicode.
pub fn characters() -> Vec<char> {
    const HIGH: [char; 32] = [
        '€', '\0', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\0', 'Ž', '\0', '\0',
        '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\0', 'ž', 'Ÿ',
    ];
    HIGH.into_iter()
        .filter(|character| *character != '\0')
        .chain((0xA1..=0xFF_u32).filter_map(char::from_u32))
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterAction {
    Insert(char),
    Cancel,
}

/// `character` goes to the front of the recent ones, once.
pub fn remember(recent: &mut Vec<char>, character: char) {
    recent.retain(|known| *known != character);
    recent.insert(0, character);
    recent.truncate(RECENT);
}

fn send(action: CharacterAction) -> Message {
    Message::SpecialCharacter(action)
}

fn grid(characters: &[char]) -> Element<'static, Message> {
    const COLUMNS: usize = 16;
    widget::column::with_children(
        characters
            .chunks(COLUMNS)
            .map(|row| {
                widget::row::with_children(
                    row.iter()
                        .map(|character| {
                            widget::button::custom(
                                widget::text::body(character.to_string())
                                    .width(Length::Fill)
                                    .center(),
                            )
                            .class(cosmic::theme::Button::Text)
                            .width(28)
                            .height(28)
                            .padding(0)
                            .on_press(send(CharacterAction::Insert(*character)))
                            .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(2)
                .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(2)
    .into()
}

pub fn dialog(recent: &[char]) -> Element<'static, Message> {
    let mut children: Vec<Element<'static, Message>> = Vec::new();
    if !recent.is_empty() {
        children.push(widget::text::caption_heading("Recent").into());
        children.push(grid(recent));
        children.push(widget::text::caption_heading("All").into());
    }
    children.push(grid(&characters()));
    widget::dialog()
        .title("Special Character")
        .body("Click a character to insert it at the caret.")
        .control(widget::column::with_children(children).spacing(8))
        .secondary_action(
            widget::button::standard("Cancel").on_press(send(CharacterAction::Cancel)),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_windows_1252_above_ascii_and_recent_ones_lead() {
        let all = characters();
        assert_eq!(all.len(), 27 + 95);
        assert!(all.contains(&'€') && all.contains(&'©') && all.contains(&'ÿ'));
        assert!(!all.contains(&'\u{A0}'));
        let mut recent = Vec::new();
        for character in "©é©".chars() {
            remember(&mut recent, character);
        }
        assert_eq!(recent, ['©', 'é']);
    }
}
