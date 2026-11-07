//! Ami Pro's Text › Special Effects… and Text › Font… as Format menu
//! dialogs on the page surface (user decision, 2026-10-12: "Ami Pro's
//! items"). The layouts follow Ami Pro's dialogs (VM screenshots of O1 and
//! O4): Special Effects is a list of check boxes; Font has the face, the
//! size, a row of colour swatches, and Revert to style.

use amipro_document::Colour;
use amipro_editing::CharacterAttribute;
use cosmic::iced::{Background, Border, Color, Length};
use cosmic::{Element, widget};

use crate::Message;
use crate::colour_control::{Context, Target};
use crate::page_editing::FontChoice;
use crate::page_layout::display_family;

/// The Special Effects dialog's check boxes, in Ami Pro's order; the
/// fifth, Overstrike character, has a character box beside it.
pub const EFFECTS: [(CharacterAttribute, &str); 4] = [
    (CharacterAttribute::Superscript, "Superscript"),
    (CharacterAttribute::Subscript, "Subscript"),
    (CharacterAttribute::DoubleUnderline, "Double underline"),
    (CharacterAttribute::Strikethrough, "Strikethrough"),
];

/// The overstrike character the dialog offers when the text has none.
pub const DEFAULT_OVERSTRIKE: char = '/';

/// The Special Effects dialog: each box starts as the selection or caret
/// shows the effect, and OK switches only the boxes that changed, so that
/// an effect on part of a selection is left alone unless its box is used.
/// Overstrike starts with the text's character (VM fixture O1).
#[derive(Clone, Debug, PartialEq)]
pub struct SpecialEffects {
    pub initial: [bool; 4],
    pub chosen: [bool; 4],
    pub initial_overstrike: Option<char>,
    pub overstrike: bool,
    /// The character box: one character, as typed.
    pub overstrike_text: String,
}

impl SpecialEffects {
    pub fn new(initial: [bool; 4], overstrike: Option<char>) -> Self {
        Self {
            initial,
            chosen: initial,
            initial_overstrike: overstrike,
            overstrike: overstrike.is_some(),
            overstrike_text: overstrike.unwrap_or(DEFAULT_OVERSTRIKE).to_string(),
        }
    }

    /// The effects to switch, with their new state: overstrike when its box
    /// or its character changed and the box holds one character.
    pub fn changes(&self) -> Vec<(CharacterAttribute, bool)> {
        let mut changes: Vec<_> = EFFECTS
            .iter()
            .zip(self.initial.iter().zip(&self.chosen))
            .filter(|(_, (initial, chosen))| initial != chosen)
            .map(|((attribute, _), (_, chosen))| (*attribute, *chosen))
            .collect();
        let mut characters = self.overstrike_text.chars();
        let character = match (characters.next(), characters.next()) {
            (Some(character), None) if !character.is_whitespace() => Some(character),
            _ => None,
        };
        match (self.overstrike, character, self.initial_overstrike) {
            (true, Some(character), initial) if initial != Some(character) => {
                changes.push((CharacterAttribute::Overstrike(character), true));
            }
            (false, _, Some(initial)) => {
                changes.push((CharacterAttribute::Overstrike(initial), false));
            }
            _ => {}
        }
        changes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EffectsAction {
    Toggle(usize, bool),
    Overstrike(bool),
    /// The character box's text: the last character typed is kept.
    OverstrikeText(String),
    Apply,
    Cancel,
}

pub fn special_effects_dialog(state: &SpecialEffects) -> Element<'_, Message> {
    let boxes: Vec<Element<'_, Message>> = EFFECTS
        .iter()
        .enumerate()
        .map(|(index, (_, label))| {
            widget::checkbox(state.chosen[index])
                .label(*label)
                .on_toggle(move |on| Message::SpecialEffects(EffectsAction::Toggle(index, on)))
                .into()
        })
        .collect();
    let overstrike = widget::row::with_children(vec![
        widget::checkbox(state.overstrike)
            .label("Overstrike character:")
            .on_toggle(|on| Message::SpecialEffects(EffectsAction::Overstrike(on)))
            .into(),
        widget::text_input("", state.overstrike_text.clone())
            .width(Length::Fixed(40.0))
            .on_input(|text| Message::SpecialEffects(EffectsAction::OverstrikeText(text)))
            .on_submit(|_| Message::SpecialEffects(EffectsAction::Apply))
            .into(),
    ])
    .spacing(8)
    .align_y(cosmic::iced::Alignment::Center);
    let mut boxes = boxes;
    boxes.push(overstrike.into());
    widget::dialog()
        .title("Special Effects")
        .control(widget::column::with_children(boxes).spacing(8))
        .primary_action(
            widget::button::suggested("OK").on_press(Message::SpecialEffects(EffectsAction::Apply)),
        )
        .secondary_action(
            widget::button::standard("Cancel")
                .on_press(Message::SpecialEffects(EffectsAction::Cancel)),
        )
        .into()
}

/// Ami Pro's Font dialog colours, left to right (VM screenshot o4-fontdlg),
/// as `COLORREF`s (`0x00BBGGRR`).
pub const PALETTE: [(Colour, &str); 15] = [
    (0x0000_00FF, "Red"),
    (0x001E_42FF, "Red orange"),
    (0x0000_81FF, "Orange"),
    (0x0000_FFFF, "Yellow"),
    (0x0000_FF00, "Green"),
    (0x00B2_FF00, "Sea green"),
    (0x00FF_FF00, "Cyan"),
    (0x00FF_8000, "Sky blue"),
    (0x00FF_0000, "Blue"),
    (0x00FF_3F61, "Blue violet"),
    (0x00FF_0081, "Violet"),
    (0x00FF_00FF, "Magenta"),
    (0x0080_00FF, "Pink"),
    (0x00FF_FFFF, "White"),
    (0x0000_0000, "Black"),
];

/// The sizes the size list offers, in points; any other size can be typed.
pub const SIZES: [&str; 16] = [
    "6", "8", "9", "10", "11", "12", "14", "16", "18", "20", "22", "24", "28", "36", "48", "72",
];

/// The Font dialog's choices.
#[derive(Clone, Debug, PartialEq)]
pub struct FontDialog {
    /// The faces offered, sorted without regard to case.
    pub families: Vec<String>,
    pub family: usize,
    /// The size in points as typed.
    pub size: String,
    pub colour: Colour,
    /// Ami Pro's Revert to style: OK returns face, size, and colour to the
    /// style's.
    pub revert: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FontAction {
    Family(usize),
    SizePreset(usize),
    Size(String),
    Colour(Colour),
    Revert(bool),
    Apply,
    Cancel,
}

fn points_text(size: f32) -> String {
    if size.fract() == 0.0 {
        format!("{size:.0}")
    } else {
        format!("{size}")
    }
}

impl FontDialog {
    /// The dialog showing `family`, `size` (points), and `colour`, offering
    /// `families` and `family` itself.
    pub fn new(mut families: Vec<String>, family: &str, size: f32, colour: Colour) -> Self {
        if !families
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(family))
        {
            families.push(family.to_owned());
        }
        families.sort_by_key(|candidate| candidate.to_lowercase());
        families.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        let index = families
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(family))
            .unwrap_or(0);
        Self {
            families,
            family: index,
            size: points_text(size),
            colour,
            revert: false,
            error: None,
        }
    }

    /// Applies a change; choosing a face, size, or colour clears Revert to
    /// style.
    pub fn update(&mut self, action: &FontAction) {
        if matches!(
            action,
            FontAction::Family(_)
                | FontAction::SizePreset(_)
                | FontAction::Size(_)
                | FontAction::Colour(_)
        ) {
            self.revert = false;
        }
        match action {
            FontAction::Family(index) if *index < self.families.len() => self.family = *index,
            FontAction::SizePreset(index) => {
                if let Some(size) = SIZES.get(*index) {
                    (*size).clone_into(&mut self.size);
                }
            }
            FontAction::Size(text) => text.clone_into(&mut self.size),
            FontAction::Colour(colour) => self.colour = *colour,
            FontAction::Revert(revert) => self.revert = *revert,
            FontAction::Family(_) | FontAction::Apply | FontAction::Cancel => {}
        }
        if !matches!(action, FontAction::Apply) {
            self.error = None;
        }
    }

    /// What OK applies: all three, as Ami Pro's Font dialog writes them, or
    /// the style's with Revert to style; or why the size is refused.
    pub fn choice(&self) -> Result<FontChoice, String> {
        if self.revert {
            return Ok(FontChoice::default());
        }
        let size = self
            .size
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|size| (1.0..=999.0).contains(size))
            .map(|size| (size * 20.0).round() / 20.0)
            .ok_or_else(|| "Enter a size from 1 to 999 points.".to_owned())?;
        Ok(FontChoice {
            family: self.families.get(self.family).cloned(),
            size: Some(size),
            colour: Some(self.colour),
        })
    }
}

/// A `COLORREF` as an Iced colour.
pub fn colour(value: Colour) -> Color {
    Color::from_rgb8(
        (value & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        ((value >> 16) & 0xFF) as u8,
    )
}

pub fn font_dialog<'a>(state: &'a FontDialog, colours: Context<'a>) -> Element<'a, Message> {
    let enabled = !state.revert;
    let label = |text: &'static str| widget::text::body(text).width(Length::Fixed(56.0)).into();
    let face = widget::dropdown(&state.families, Some(state.family), |index| {
        Message::FontDialog(FontAction::Family(index))
    });
    let sizes = widget::dropdown(
        &SIZES[..],
        SIZES.iter().position(|size| *size == state.size.trim()),
        |index| Message::FontDialog(FontAction::SizePreset(index)),
    );
    let points = widget::text_input("Points", state.size.clone())
        .width(Length::Fixed(72.0))
        .on_input(|text| Message::FontDialog(FontAction::Size(text)))
        .on_submit(|_| Message::FontDialog(FontAction::Apply));
    let family = state
        .families
        .get(state.family)
        .map_or("", String::as_str)
        .to_owned();
    let preview_size = state
        .size
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|size| (1.0..=999.0).contains(size))
        .unwrap_or(12.0)
        .min(36.0);
    let preview = widget::text(family.clone())
        .font(cosmic::iced::Font {
            family: cosmic::iced::font::Family::Name(display_family(&family)),
            ..cosmic::iced::Font::default()
        })
        .size(preview_size * 4.0 / 3.0)
        .class(cosmic::theme::Text::Color(if enabled {
            colour(state.colour)
        } else {
            Color::from_rgb8(0x80, 0x80, 0x80)
        }));
    let mut controls: Vec<Element<'_, Message>> = vec![
        widget::row::with_children(vec![label("Face"), face.width(Length::Fill).into()])
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        widget::row::with_children(vec![
            label("Size"),
            sizes.width(Length::Fixed(88.0)).into(),
            points.into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
        colours.control_enabled(Target::FontText, state.colour, enabled),
        widget::checkbox(state.revert)
            .label("Revert to style")
            .on_toggle(|on| Message::FontDialog(FontAction::Revert(on)))
            .into(),
        widget::container(preview)
            .width(Length::Fill)
            .height(Length::Fixed(64.0))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(64.0))
            .class(cosmic::theme::Container::custom(|theme| {
                cosmic::iced::widget::container::Style {
                    background: Some(Background::Color(Color::WHITE)),
                    border: Border {
                        color: theme.cosmic().bg_divider().into(),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                }
            }))
            .into(),
    ];
    if let Some(error) = &state.error {
        controls.push(widget::text::caption(error.clone()).into());
    }
    widget::dialog()
        .title("Font")
        .control(widget::column::with_children(controls).spacing(12))
        .primary_action(
            widget::button::suggested("OK").on_press(Message::FontDialog(FontAction::Apply)),
        )
        .secondary_action(
            widget::button::standard("Cancel").on_press(Message::FontDialog(FontAction::Cancel)),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_effects_switch_only_the_boxes_that_changed() {
        let mut effects = SpecialEffects::new([true, false, false, true], None);
        assert!(effects.changes().is_empty());
        assert_eq!(effects.overstrike_text, "/");
        effects.chosen = [false, false, true, true];
        assert_eq!(
            effects.changes(),
            [
                (CharacterAttribute::Superscript, false),
                (CharacterAttribute::DoubleUnderline, true)
            ]
        );
        // Overstrike: on with the box's character; a changed character
        // replaces the text's; off removes it; an empty box changes nothing.
        let mut effects = SpecialEffects::new([false; 4], None);
        effects.overstrike = true;
        assert_eq!(
            effects.changes(),
            [(CharacterAttribute::Overstrike('/'), true)]
        );
        let mut effects = SpecialEffects::new([false; 4], Some('X'));
        assert!(effects.changes().is_empty());
        effects.overstrike_text = "=".into();
        assert_eq!(
            effects.changes(),
            [(CharacterAttribute::Overstrike('='), true)]
        );
        effects.overstrike = false;
        assert_eq!(
            effects.changes(),
            [(CharacterAttribute::Overstrike('X'), false)]
        );
        effects.overstrike = true;
        effects.overstrike_text.clear();
        assert!(effects.changes().is_empty());
    }

    #[test]
    fn the_font_dialog_offers_the_text_face_and_checks_the_size() {
        let mut dialog = FontDialog::new(
            vec![
                "Liberation Serif".into(),
                "arial".into(),
                "Noto Sans".into(),
            ],
            "Times New Roman",
            10.5,
            0x0000_00FF,
        );
        assert_eq!(
            dialog.families,
            ["arial", "Liberation Serif", "Noto Sans", "Times New Roman"]
        );
        assert_eq!(dialog.families[dialog.family], "Times New Roman");
        assert_eq!(dialog.size, "10.5");
        assert_eq!(
            dialog.choice(),
            Ok(FontChoice {
                family: Some("Times New Roman".into()),
                size: Some(10.5),
                colour: Some(0x0000_00FF),
            })
        );
        dialog.update(&FontAction::Family(0));
        dialog.update(&FontAction::SizePreset(8));
        dialog.update(&FontAction::Colour(PALETTE[8].0));
        let choice = dialog.choice().unwrap();
        assert_eq!(
            (choice.family.as_deref(), choice.size, choice.colour),
            (Some("arial"), Some(18.0), Some(0x00FF_0000))
        );
        dialog.update(&FontAction::Size("0".into()));
        assert!(dialog.choice().is_err());
        dialog.update(&FontAction::Revert(true));
        assert_eq!(dialog.choice(), Ok(FontChoice::default()));
        dialog.update(&FontAction::Colour(0));
        assert!(!dialog.revert, "a choice clears Revert to style");
        // Arial matches a face already offered, whatever its case.
        let dialog = FontDialog::new(vec!["arial".into()], "Arial", 12.0, 0);
        assert_eq!(dialog.families, ["arial"]);
    }
}
