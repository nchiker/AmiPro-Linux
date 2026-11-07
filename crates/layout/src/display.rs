//! How a paragraph's text is shown: its runs over its style, with Ami Pro's
//! caps, superscript, subscript, and small caps, which change the letters
//! and sizes drawn but not the text (`docs/research/SAM_FORMAT_NOTES.md`,
//! "Character attributes"). Shapers and the page surface share it, so that
//! what is measured is what is drawn.

use std::borrow::Cow;
use std::ops::Range;

use amipro_document::{
    Caps, CharacterAttributes, CharacterFormat, CharacterRun, ParagraphStyle, TWIPS_PER_POINT,
};

/// A stretch of a paragraph shown in one font with one set of attributes.
#[derive(Clone, Debug, PartialEq)]
pub struct DisplaySpan<'a> {
    /// Bytes of the paragraph's text, which are the same bytes of the
    /// display text.
    pub bytes: Range<usize>,
    /// The face as the style or run names it.
    pub family: &'a str,
    /// The size drawn, in points: 7/12 of `full_size` for superscript and
    /// subscript, and 7/10 of that for small caps' lower-case letters.
    pub size: f32,
    /// The size before those reductions, in points. Lines take their height
    /// from it (decompile: the line's metrics are scaled back to full size).
    pub full_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub attributes: CharacterAttributes,
}

impl DisplaySpan<'_> {
    /// Whether the span is drawn smaller than its line's metrics assume.
    pub fn is_reduced(&self) -> bool {
        self.size != self.full_size
    }

    /// How far the span's baseline sits above the line's, in twips, given
    /// the ascent of its own (reduced) font: a superscript is raised by 5/12
    /// of the full size, and a subscript lowered by half its font's ascent
    /// (decompile).
    pub fn baseline_shift(&self, reduced_ascent: f32) -> f32 {
        if self.attributes.superscript {
            (points_to_twips(self.full_size) * 5 / 12) as f32
        } else if self.attributes.subscript {
            -(reduced_ascent / 2.0).trunc()
        } else {
            0.0
        }
    }
}

fn points_to_twips(points: f32) -> i32 {
    (points * TWIPS_PER_POINT).round() as i32
}

/// The format in force at each byte: the runs over plain text.
fn formats<'a>(
    text: &str,
    runs: &'a [CharacterRun],
) -> impl Iterator<Item = (Range<usize>, Option<&'a CharacterFormat>)> {
    let mut pieces = Vec::new();
    let mut offset = 0;
    for run in runs
        .iter()
        .filter(|run| run.start < run.end && run.end <= text.len())
    {
        if run.start < offset {
            continue;
        }
        if run.start > offset {
            pieces.push((offset..run.start, None));
        }
        pieces.push((run.start..run.end, Some(&run.format)));
        offset = run.end;
    }
    if offset < text.len() {
        pieces.push((offset..text.len(), None));
    }
    pieces.into_iter()
}

/// The paragraph's text with its caps applied. Every character keeps its
/// UTF-8 length, so byte offsets stay valid: a character whose other case
/// is longer or shorter (ß, whose capital is "SS") is shown as typed.
pub fn display_text<'t>(
    text: &'t str,
    style: &ParagraphStyle,
    runs: &[CharacterRun],
) -> Cow<'t, str> {
    let pieces: Vec<_> = formats(text, runs)
        .filter_map(|(bytes, format)| {
            let caps = format.map_or(style.attributes.caps, |format| {
                format.attributes(style).caps
            })?;
            Some((bytes, caps))
        })
        .collect();
    if pieces.is_empty() {
        return Cow::Borrowed(text);
    }
    let mut shown = text.to_owned();
    for (bytes, caps) in pieces {
        let changed = apply_caps(text, bytes.clone(), caps);
        shown.replace_range(bytes, &changed);
    }
    Cow::Owned(shown)
}

/// Ami Pro's caps over `text[bytes]` (decompile, `14f0:01c4`): upper and
/// small caps show capitals, lower case small letters, and initial caps a
/// capital at the start of each word and small letters after it. A word is
/// a run of letters; an apostrophe between two letters stays in the word.
fn apply_caps(text: &str, bytes: Range<usize>, caps: Caps) -> String {
    let mut shown = String::with_capacity(bytes.len());
    let mut at_start = !text[..bytes.start]
        .chars()
        .next_back()
        .is_some_and(char::is_alphabetic);
    let mut previous = text[..bytes.start].chars().next_back();
    let mut chars = text[bytes].chars().peekable();
    while let Some(character) = chars.next() {
        let next = chars.peek().copied();
        let shown_as = match caps {
            Caps::Upper | Caps::Small => upper(character),
            Caps::Lower => lower(character),
            Caps::Normal => character,
            Caps::Initial => {
                if character == '\'' {
                    at_start = !(previous.is_some_and(char::is_alphabetic)
                        && next.is_some_and(char::is_alphabetic));
                    character
                } else if character.is_alphabetic() {
                    let shown = if at_start {
                        upper(character)
                    } else {
                        lower(character)
                    };
                    at_start = false;
                    shown
                } else {
                    at_start = true;
                    character
                }
            }
        };
        shown.push(shown_as);
        previous = Some(character);
    }
    shown
}

/// One character in another case, when that is one character of the same
/// UTF-8 length. The micro sign stays: it is a symbol, not a letter, in Ami
/// Pro's character table.
fn same_length(character: char, mut cased: impl Iterator<Item = char>) -> char {
    match (cased.next(), cased.next()) {
        (Some(other), None)
            if other.len_utf8() == character.len_utf8() && character != '\u{B5}' =>
        {
            other
        }
        _ => character,
    }
}

fn upper(character: char) -> char {
    same_length(character, character.to_uppercase())
}

fn lower(character: char) -> char {
    same_length(character, character.to_lowercase())
}

/// Whether small caps show `character` at the smaller size: lower-case
/// letters and spaces (Ami Pro's character table, `& 10`).
fn small_in_small_caps(character: char) -> bool {
    character.is_lowercase() || matches!(character, ' ' | '\t' | '\u{A0}')
}

/// The paragraph's display spans in text order, covering every byte once.
pub fn display_spans<'a>(
    text: &str,
    style: &'a ParagraphStyle,
    runs: &'a [CharacterRun],
) -> Vec<DisplaySpan<'a>> {
    let mut spans = Vec::new();
    for (bytes, format) in formats(text, runs) {
        let attributes = format.map_or(style.attributes, |format| format.attributes(style));
        let family = format
            .and_then(|format| format.font_family.as_deref())
            .unwrap_or(&style.font_family);
        let full_size = format
            .and_then(|format| format.font_size_points)
            .unwrap_or(style.font_size_points);
        // Sizes are whole twips, reduced with integer arithmetic as Ami
        // Pro does (decompile, `10d0:0309`).
        let mut twips = points_to_twips(full_size);
        if attributes.is_script() {
            twips = twips * 7 / 12;
        }
        let span = DisplaySpan {
            bytes: bytes.clone(),
            family,
            size: if attributes.is_script() {
                twips as f32 / TWIPS_PER_POINT
            } else {
                full_size
            },
            full_size,
            bold: format.and_then(|format| format.bold).unwrap_or(style.bold),
            italic: format
                .and_then(|format| format.italic)
                .unwrap_or(style.italic),
            attributes,
        };
        if attributes.caps != Some(Caps::Small) {
            spans.push(span);
            continue;
        }
        // Small caps: lower-case letters and spaces at 7/10, the rest as
        // they are.
        let small = (twips * 7 / 10) as f32 / TWIPS_PER_POINT;
        let mut start = bytes.start;
        let mut current = None;
        for (offset, character) in text[bytes.clone()].char_indices() {
            let reduced = small_in_small_caps(character);
            if current.is_some_and(|current| current != reduced) {
                let at = bytes.start + offset;
                spans.push(DisplaySpan {
                    bytes: start..at,
                    size: if current == Some(true) {
                        small
                    } else {
                        span.size
                    },
                    ..span.clone()
                });
                start = at;
            }
            current = Some(reduced);
        }
        spans.push(DisplaySpan {
            bytes: start..bytes.end,
            size: if current == Some(true) {
                small
            } else {
                span.size
            },
            ..span
        });
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{StyleId, default_styles};

    fn run(start: usize, end: usize, format: CharacterFormat) -> CharacterRun {
        CharacterRun { start, end, format }
    }

    fn caps(caps: Caps) -> CharacterFormat {
        CharacterFormat {
            caps: Some(caps),
            ..CharacterFormat::default()
        }
    }

    #[test]
    fn caps_change_the_letters_shown_but_not_their_bytes() {
        let style = default_styles()[&StyleId(1)].clone();
        let shown = |text: &str, format: CharacterFormat| {
            display_text(text, &style, &[run(0, text.len(), format)]).into_owned()
        };
        assert_eq!(
            shown("Größe straße µm", caps(Caps::Upper)),
            "GRÖßE STRAßE µM"
        );
        assert_eq!(shown("ÉCOLE Normale", caps(Caps::Lower)), "école normale");
        assert_eq!(shown("ÉCOLE Normale", caps(Caps::Small)), "ÉCOLE NORMALE");
        // Initial caps lower the rest of each word; digits and punctuation
        // end words, and an apostrophe between letters does not (decompile).
        assert_eq!(
            shown(
                "don't stop McDONALD's 3rd 'quoted' élan",
                caps(Caps::Initial)
            ),
            "Don't Stop Mcdonald's 3Rd 'Quoted' Élan"
        );
        // A word that starts before the run keeps going.
        let text = "mcdonald";
        assert_eq!(
            display_text(text, &style, &[run(2, text.len(), caps(Caps::Initial))]),
            "mcdonald"
        );
        // A style's caps apply where no run sets others.
        let mut upper = style.clone();
        upper.attributes.caps = Some(Caps::Upper);
        assert_eq!(
            display_text("title case", &upper, &[run(0, 5, caps(Caps::Initial))]),
            "Title CASE"
        );
        assert!(matches!(
            display_text("plain", &style, &[]),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn scripts_and_small_caps_are_drawn_smaller_than_their_line() {
        let mut style = default_styles()[&StyleId(1)].clone();
        style.font_size_points = 12.0;
        let runs = [
            run(
                1,
                3,
                CharacterFormat {
                    superscript: Some(true),
                    ..CharacterFormat::default()
                },
            ),
            run(
                3,
                5,
                CharacterFormat {
                    subscript: Some(true),
                    font_size_points: Some(11.0),
                    ..CharacterFormat::default()
                },
            ),
            run(5, 13, caps(Caps::Small)),
        ];
        let text = "x23ab Small C";
        let spans: Vec<_> = display_spans(text, &style, &runs)
            .into_iter()
            .map(|span| (&text[span.bytes.clone()], span.size, span.full_size))
            .collect();
        // 240 × 7 / 12 = 140 twips; 220 × 7 / 12 = 128; 240 × 7 / 10 = 168.
        assert_eq!(
            spans,
            [
                ("x", 12.0, 12.0),
                ("23", 7.0, 12.0),
                ("ab", 6.4, 11.0),
                (" ", 8.4, 12.0),
                ("S", 12.0, 12.0),
                ("mall ", 8.4, 12.0),
                ("C", 12.0, 12.0),
            ]
        );
        let spans = display_spans(text, &style, &runs);
        assert!(!spans[0].is_reduced() && spans[1].is_reduced());
        assert_eq!(spans[1].baseline_shift(100.0), 100.0, "5/12 of 240 twips");
        assert_eq!(
            spans[2].baseline_shift(101.0),
            -50.0,
            "half the small ascent"
        );
        assert_eq!(spans[4].baseline_shift(100.0), 0.0);
        let covered: usize = spans.iter().map(|span| span.bytes.len()).sum();
        assert_eq!(covered, text.len());
    }
}
