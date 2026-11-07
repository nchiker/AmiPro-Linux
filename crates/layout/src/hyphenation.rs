//! Where words may be hyphenated, by Ami Pro's rules (decompile `1310:0000`,
//! `1310:02af`, `1310:03e0`, and the line builder `10d0:09b9`;
//! `docs/research/SAM_FORMAT_NOTES.md`, "Hyphenation"): only in styles with
//! Hyphenation on, only in a word's first 32 characters, never in a word
//! with a digit, a no-break space, or a discretionary hyphen (which is used
//! instead), never in a word of fewer than five letters, and never next to
//! text marked No hyphenation. The points themselves come from the shaper's
//! hyphenator (`Shaper::hyphenate`); the breaker takes one only where the
//! line has the document's hyphenation zone left.

use crate::shaper::ShapeRequest;

/// Characters Ami Pro hyphenates in a word.
const WINDOW: usize = 32;
/// Fewer letters than this are never hyphenated.
const SHORTEST: usize = 5;

/// Whether `character` keeps its word from being hyphenated: a digit, a
/// superscript digit or fraction, a no-break space, or a discretionary
/// hyphen.
fn stops_hyphenation(character: char) -> bool {
    character.is_ascii_digit()
        || matches!(
            character,
            '\u{B2}' | '\u{B3}' | '\u{B9}' | '\u{BC}' | '\u{BD}' | '\u{BE}' | '\u{A0}' | '\u{AD}'
        )
}

/// Whether `character` ends a word for hyphenation: a space, a tab, a hard
/// return, an object, or a hyphen.
fn separates(character: char) -> bool {
    (character.is_whitespace() && character != '\u{A0}')
        || matches!(character, '-' | '\u{2010}' | '\u{2028}' | '\u{FFFC}')
}

/// The byte offsets in `request.text` where a line may end with an added
/// hyphen, from `hyphenate`'s points within letter runs (offsets into the
/// run it is given); none unless the style has Hyphenation on.
pub fn hyphenation_points(
    request: &ShapeRequest<'_>,
    hyphenate: &mut dyn FnMut(&str) -> Vec<usize>,
) -> Vec<usize> {
    if !request.style.hyphenation {
        return Vec::new();
    }
    let text = request.text;
    let unmarked = |byte: usize| {
        !request
            .runs
            .iter()
            .find(|run| run.start <= byte && byte < run.end)
            .is_some_and(|run| run.format.attributes(request.style).no_hyphenation)
    };
    let mut points = Vec::new();
    let mut word_start = 0;
    let mut words = Vec::new();
    for (byte, character) in text.char_indices() {
        if separates(character) {
            if byte > word_start {
                words.push(word_start..byte);
            }
            word_start = byte + character.len_utf8();
        }
    }
    if text.len() > word_start {
        words.push(word_start..text.len());
    }
    for word in words {
        let slice = &text[word.clone()];
        if slice.chars().any(stops_hyphenation) {
            continue;
        }
        // Ami Pro looks at the word's first 32 characters.
        let window_end = slice
            .char_indices()
            .nth(WINDOW)
            .map_or(slice.len(), |(byte, _)| byte);
        // Each run of letters is hyphenated on its own.
        let mut letters_start = None;
        let mut runs = Vec::new();
        for (offset, character) in slice[..window_end].char_indices() {
            match (character.is_alphabetic(), letters_start) {
                (true, None) => letters_start = Some(offset),
                (false, Some(begin)) => {
                    runs.push(begin..offset);
                    letters_start = None;
                }
                _ => {}
            }
        }
        if let Some(begin) = letters_start {
            runs.push(begin..window_end);
        }
        for run in runs {
            let letters = &slice[run.clone()];
            if letters.chars().count() < SHORTEST {
                continue;
            }
            for point in hyphenate(letters) {
                if point == 0 || point >= letters.len() || !letters.is_char_boundary(point) {
                    continue;
                }
                let at = word.start + run.start + point;
                let before = text[..at]
                    .chars()
                    .next_back()
                    .map_or(at, |character| at - character.len_utf8());
                if unmarked(before) && unmarked(at) {
                    points.push(at);
                }
            }
        }
    }
    points.sort_unstable();
    points.dedup();
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{CharacterFormat, CharacterRun, LineHeights, StyleId, default_styles};

    /// Splits after every second letter, keeping two letters before and
    /// three after (TeX's English bounds), as `FixedShaper` does.
    fn every_second(word: &str) -> Vec<usize> {
        let count = word.chars().count();
        (2..count.saturating_sub(2)).step_by(2).collect()
    }

    fn points(text: &str, hyphenation: bool, runs: &[CharacterRun]) -> Vec<usize> {
        let mut style = default_styles()[&StyleId(1)].clone();
        style.hyphenation = hyphenation;
        let request = ShapeRequest {
            text,
            style: &style,
            runs,
            end_format: &CharacterFormat::default(),
            line_heights: &LineHeights::default(),
            language: "en",
        };
        hyphenation_points(&request, &mut every_second)
    }

    #[test]
    fn only_long_words_of_letters_in_hyphenated_styles() {
        assert_eq!(points("extensive", false, &[]), []);
        assert_eq!(points("extensive", true, &[]), [2, 4, 6]);
        // Short words, digits, no-break spaces, and discretionary hyphens.
        for text in ["word", "area51x", "abc\u{A0}defgh", "co\u{AD}operate"] {
            assert_eq!(points(text, true, &[]), [], "{text}");
        }
        // Punctuation ends a run of letters; hyphens and spaces end words.
        assert_eq!(points("(extensive)", true, &[]), [3, 5, 7]);
        assert_eq!(points("long-winded", true, &[]), [7]);
        // Only the first 32 characters count.
        let long = "a".repeat(40);
        assert!(points(&long, true, &[]).iter().all(|point| *point < 32));
        // Never next to text marked No hyphenation.
        let marked = [CharacterRun {
            start: 0,
            end: 5,
            format: CharacterFormat {
                no_hyphenation: Some(true),
                ..CharacterFormat::default()
            },
        }];
        assert_eq!(points("extensive", true, &marked), [6]);
    }
}
