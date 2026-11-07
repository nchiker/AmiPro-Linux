//! Choosing colours (`docs/Design/UI_SPEC.md` §23, decisions C1–C4): the
//! colours a colour panel offers besides its picker. Ami Pro's 15 (its
//! colour bar, `character_dialogs::PALETTE`), the colours the document
//! already uses, every kind together and most used first, and Recent, one
//! list for the whole app kept in the settings. Colours are Windows
//! `COLORREF`s (`0x00BBGGRR`), as files store them.

use std::collections::BTreeMap;

use amipro_document::{Colour, Document, Sides};
use cosmic::iced::Color;

use crate::character_dialogs::PALETTE;

/// How many colours one of the panel's rows shows: as many as Ami Pro's
/// colour bar.
pub const ROW: usize = PALETTE.len();

/// A `COLORREF` as a display colour.
pub fn to_color(colour: Colour) -> Color {
    crate::character_dialogs::colour(colour)
}

/// A display colour as a `COLORREF`, each channel to the nearest of 256.
pub fn from_color(color: Color) -> Colour {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as Colour;
    channel(color.r) | channel(color.g) << 8 | channel(color.b) << 16
}

/// The picker's colour (hue, saturation, value) as a `COLORREF`.
pub fn from_hsv(hsv: cosmic::cosmic_theme::palette::Hsv) -> Colour {
    use cosmic::cosmic_theme::palette::{FromColor, Srgb};
    let rgb = Srgb::from_color(hsv);
    from_color(Color::from_rgb(rgb.red, rgb.green, rgb.blue))
}

/// `#RRGGBB` for a `COLORREF`.
pub fn hex(colour: Colour) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        colour & 0xFF,
        (colour >> 8) & 0xFF,
        (colour >> 16) & 0xFF
    )
}

/// A colour's name: Ami Pro's for one of its 15, else `#RRGGBB`.
pub fn name(colour: Colour) -> String {
    PALETTE
        .iter()
        .find(|(value, _)| *value == colour)
        .map_or_else(|| hex(colour), |(_, name)| (*name).to_owned())
}

/// The colours `document` shows, every kind together, most used first and
/// at most a row: the styles' text and rules weighed by the paragraphs in
/// them, each character run's, and each frame's and page layout's lines,
/// shadow, and background where they show.
pub fn document_colours(document: &Document) -> Vec<Colour> {
    // Each colour's weight and when it was first seen, for a stable order.
    let mut seen: BTreeMap<Colour, (usize, usize)> = BTreeMap::new();
    let mut add = |colour: Colour, weight: usize| {
        let order = seen.len();
        let entry = seen.entry(colour).or_insert((0, order));
        entry.0 += weight;
    };
    let mut in_style: BTreeMap<amipro_document::StyleId, usize> = BTreeMap::new();
    for (_, flow) in document.flows() {
        for paragraph in &flow.paragraphs {
            *in_style.entry(paragraph.style).or_default() += 1;
            for run in &paragraph.runs {
                if let Some(colour) = run.format.colour {
                    add(colour, 1);
                }
            }
            if let Some(colour) = paragraph.end_format.colour {
                add(colour, 1);
            }
        }
    }
    for (id, count) in &in_style {
        let Some(style) = document.styles().get(id) else {
            continue;
        };
        add(style.attributes.colour, *count);
        if style.rules.above.on || style.rules.below.on {
            add(style.rules.colour, *count);
        }
    }
    let shows = |sides: Sides| sides != Sides::NONE;
    for frame in document.frames() {
        let style = &frame.style;
        if shows(style.border.sides) {
            add(style.border.colour, 1);
        }
        let shadow = style.shadow;
        if shadow.left > 0 || shadow.top > 0 || shadow.right > 0 || shadow.bottom > 0 {
            add(shadow.colour, 1);
        }
        if style.opaque {
            add(style.background, 1);
        }
        if let Some(line) = style.interior.line_between {
            add(line.colour, 1);
        }
    }
    for layout in document.layouts() {
        for geometry in std::iter::once(&layout.right).chain(&layout.left) {
            if let Some(border) = geometry.border
                && shows(border.sides)
            {
                add(border.colour, 1);
            }
            if let Some(line) = geometry.layout.line_between {
                add(line.colour, 1);
            }
        }
    }
    let mut colours: Vec<(Colour, (usize, usize))> = seen.into_iter().collect();
    colours.sort_by_key(|(_, (weight, order))| (std::cmp::Reverse(*weight), *order));
    colours
        .into_iter()
        .take(ROW)
        .map(|(colour, _)| colour)
        .collect()
}

/// Recent: the colours last chosen anywhere in the app, newest first, one
/// list (C4), kept in the settings.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Recent(Vec<Colour>);

impl Recent {
    /// The list the settings kept, newest first, without repeats, at most
    /// two rows; anything else in them is dropped.
    pub fn from_numbers(numbers: &[u32]) -> Self {
        let mut recent = Self::default();
        for colour in numbers.iter().rev() {
            if *colour <= 0x00FF_FFFF {
                recent.push(*colour);
            }
        }
        recent
    }

    pub fn to_numbers(&self) -> Vec<u32> {
        self.0.clone()
    }

    /// `colour` becomes the newest.
    pub fn push(&mut self, colour: Colour) {
        self.0.retain(|kept| *kept != colour);
        self.0.insert(0, colour);
        self.0.truncate(2 * ROW);
    }

    /// The newest colours not among `shown` (the rows above), a row at most.
    pub fn shown(&self, shown: &[Colour]) -> Vec<Colour> {
        self.0
            .iter()
            .filter(|colour| !shown.contains(colour))
            .take(ROW)
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{CharacterFormat, CharacterRun, ColumnLine, FrameStyle, Placement, Rect};

    #[test]
    fn colours_convert_both_ways_and_are_named() {
        let navy: Colour = 0x0080_3311;
        assert_eq!(hex(navy), "#113380");
        assert_eq!(from_color(to_color(navy)), navy);
        assert_eq!(name(0x0000_00FF), "Red");
        assert_eq!(name(navy), "#113380");
    }

    #[test]
    fn a_document_offers_its_colours_most_used_first() {
        let mut document = Document::from_plain_text("One\nTwo\nThree");
        // A red word, twice, and a frame with a blue background, a green
        // shadow, a grey line, and an orange line between columns.
        for index in 0..2 {
            document.paragraph_mut(index).runs.push(CharacterRun {
                start: 0,
                end: 1,
                format: CharacterFormat {
                    colour: Some(0x0000_00FF),
                    ..CharacterFormat::default()
                },
            });
        }
        let defaults = FrameStyle::default();
        let style = FrameStyle {
            background: 0x00FF_0000,
            border: amipro_document::Border {
                colour: 0x0080_8080,
                ..defaults.border
            },
            shadow: amipro_document::Shadow {
                right: 60,
                colour: 0x0000_FF00,
                ..defaults.shadow
            },
            interior: amipro_document::LayoutBox {
                columns: 2,
                tabs: vec![Vec::new(), Vec::new()],
                line_between: Some(ColumnLine {
                    style: 1,
                    colour: 0x0000_81FF,
                }),
                ..defaults.interior.clone()
            },
            ..defaults
        };
        let frame = document.create_text_frame(
            Placement::Fixed {
                page: 1,
                rect: Rect::new(1_440, 1_440, 4_320, 4_320),
            },
            style,
        );
        document.frame_stack_mut().push(frame);
        let colours = document_colours(&document);
        // Body Text's black, in the three paragraphs and the frame's and
        // the header's and footer's; red twice; then the frame's in order.
        assert_eq!(colours[0], 0);
        assert_eq!(colours[1], 0x0000_00FF);
        for colour in [0x0080_8080, 0x0000_FF00, 0x00FF_0000, 0x0000_81FF] {
            assert!(colours.contains(&colour), "{colours:?}");
        }
        // A transparent frame's background and an absent shadow do not
        // show.
        let mut plain = Document::from_plain_text("Text");
        let defaults = FrameStyle::default();
        let style = FrameStyle {
            opaque: false,
            background: 0x0012_3456,
            shadow: amipro_document::Shadow {
                colour: 0x0065_4321,
                ..defaults.shadow
            },
            ..defaults
        };
        let frame = plain.create_text_frame(
            Placement::Fixed {
                page: 1,
                rect: Rect::new(1_440, 1_440, 4_320, 4_320),
            },
            style,
        );
        plain.frame_stack_mut().push(frame);
        let colours = document_colours(&plain);
        assert!(!colours.contains(&0x0012_3456));
        assert!(!colours.contains(&0x0065_4321));
    }

    #[test]
    fn recent_keeps_the_newest_first_without_repeats() {
        let mut recent = Recent::default();
        for colour in [1, 2, 3, 2] {
            recent.push(colour);
        }
        assert_eq!(recent.to_numbers(), [2, 3, 1]);
        assert_eq!(recent.shown(&[3]), [2, 1], "shown above, left out");
        assert_eq!(Recent::from_numbers(&recent.to_numbers()), recent);
        assert_eq!(
            Recent::from_numbers(&[5, 0x0100_0000, 5, 6]).to_numbers(),
            [5, 6],
            "repeats and what is not a colour dropped"
        );
        for colour in 0..100 {
            recent.push(colour);
        }
        assert_eq!(recent.to_numbers().len(), 2 * ROW);
        assert_eq!(recent.shown(&[]).len(), ROW);
    }
}
