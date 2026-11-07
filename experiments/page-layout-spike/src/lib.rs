//! Phase 2.5a spike: break lines ourselves over COSMIC Text shaped words.
//!
//! COSMIC Text's `Buffer` wraps every line of a paragraph to one width. Ami
//! Pro's layout needs a width per line (text wrapping around frames), inline
//! boxes for Flow with text frames (U+FFFC), U+2028 as a forced break, and
//! U+00AD as a break point that shows a hyphen when used. This crate tests
//! whether the words of a `ShapeLine` are enough to do that ourselves.

use std::ops::Range;

use cosmic_text::{AttrsList, FontSystem, Hinting, ShapeLine, Shaping, Wrap};

pub const OBJECT: char = '\u{FFFC}';
pub const HARD_RETURN: char = '\u{2028}';
pub const SOFT_HYPHEN: char = '\u{AD}';

/// One shaped word in logical order: a break opportunity follows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    /// The span and word in the `ShapeLine`.
    pub span: usize,
    pub index: usize,
    pub bytes: Range<usize>,
    pub width: f32,
    pub blank: bool,
    pub rtl: bool,
    /// An inline object (U+FFFC), whose width is the object's.
    pub object: bool,
    /// Ends with U+2028: the line must end after it.
    pub forced_break: bool,
    /// Ends with U+00AD: breaking here shows a hyphen.
    pub soft_hyphen: bool,
}

/// A line chosen by [`break_lines`]: a range of words.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub words: Range<usize>,
    /// Width of the line's content, without hanging trailing blanks, with the
    /// hyphen if the line ends at a soft hyphen.
    pub width: f32,
    pub hyphen: bool,
    pub forced: bool,
    /// One word wider than the line, placed alone.
    pub overflow: bool,
}

/// Shapes `text` once, as one paragraph.
pub fn shape(font_system: &mut FontSystem, text: &str, attrs: &AttrsList) -> ShapeLine {
    ShapeLine::new(font_system, text, attrs, Shaping::Advanced, 8)
}

/// The shaped words in logical order. Objects take `object_width`.
pub fn logical_words(line: &ShapeLine, text: &str, font_size: f32, object_width: f32) -> Vec<Word> {
    let mut words = Vec::new();
    for (span_index, span) in line.spans.iter().enumerate() {
        for (word_index, word) in span.words.iter().enumerate() {
            let (Some(start), Some(end)) = (
                word.glyphs.iter().map(|glyph| glyph.start).min(),
                word.glyphs.iter().map(|glyph| glyph.end).max(),
            ) else {
                continue;
            };
            let slice = &text[start..end];
            let object = slice.contains(OBJECT);
            words.push(Word {
                span: span_index,
                index: word_index,
                bytes: start..end,
                width: if object {
                    object_width
                } else {
                    word.width(font_size)
                },
                blank: word.blank,
                rtl: span.level.is_rtl(),
                object,
                forced_break: slice.ends_with(HARD_RETURN),
                soft_hyphen: slice.ends_with(SOFT_HYPHEN),
            });
        }
    }
    words.sort_by_key(|word| word.bytes.start);
    words
}

/// Greedy line breaking with a width per line. Blanks at a line end hang
/// outside the width; a word wider than its line is placed alone. A line that
/// ends at a soft hyphen shows a hyphen, which must fit too.
pub fn break_lines(
    words: &[Word],
    mut available: impl FnMut(usize) -> f32,
    hyphen_width: f32,
) -> Vec<Line> {
    let content_of =
        |range: Range<usize>| -> f32 { words[range].iter().map(|word| word.width).sum() };
    let mut lines = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let limit = available(lines.len());
        let mut end = start;
        let mut content = 0.0f32;
        let mut blanks = 0.0f32;
        let mut has_content = false;
        let mut forced = false;
        let mut overflow = false;
        while end < words.len() {
            let word = &words[end];
            if word.blank {
                blanks += word.width;
                end += 1;
                if word.forced_break {
                    forced = true;
                    break;
                }
                continue;
            }
            let candidate = content + blanks + word.width;
            if candidate > limit && has_content {
                break;
            }
            overflow |= candidate > limit;
            content = candidate;
            blanks = 0.0;
            has_content = true;
            end += 1;
            if word.forced_break {
                forced = true;
                break;
            }
        }
        let mut hyphen = false;
        if !forced && end < words.len() {
            while let Some(last) = (start..end).rev().find(|index| !words[*index].blank) {
                if !words[last].soft_hyphen {
                    break;
                }
                let with_hyphen = content_of(start..last + 1) + hyphen_width;
                let earlier = (start..last).rev().find(|index| !words[*index].blank);
                match earlier {
                    Some(previous) if with_hyphen > limit => {
                        // The hyphen does not fit: end after the earlier word.
                        end = previous + 1;
                        while end < last && words[end].blank {
                            end += 1;
                        }
                        content = content_of(start..previous + 1);
                    }
                    _ => {
                        hyphen = true;
                        overflow |= with_hyphen > limit;
                        content = with_hyphen;
                        end = last + 1;
                        break;
                    }
                }
            }
        }
        lines.push(Line {
            words: start..end,
            width: content,
            hyphen,
            forced,
            overflow,
        });
        start = end;
    }
    if lines.is_empty() {
        lines.push(Line {
            words: 0..0,
            width: 0.0,
            hyphen: false,
            forced: false,
            overflow: false,
        });
    }
    lines
}

/// A glyph placed on a line.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub bytes: Range<usize>,
    pub x: f32,
    pub width: f32,
}

/// Places a line's glyphs left to right, reversing right-to-left runs of
/// words (rule L2 for a left-to-right paragraph with levels 0 and 1).
/// Trailing blanks are not placed, as COSMIC Text does not place them.
pub fn place(line: &ShapeLine, words: &[Word], broken: &Line, font_size: f32) -> Vec<Placed> {
    let mut range: Vec<&Word> = words[broken.words.clone()].iter().collect();
    while range.last().is_some_and(|word| word.blank) {
        range.pop();
    }
    // Visual order of words.
    let mut visual: Vec<&Word> = Vec::with_capacity(range.len());
    let mut index = 0;
    while index < range.len() {
        if range[index].rtl {
            let run_end = (index..range.len())
                .find(|candidate| !range[*candidate].rtl)
                .unwrap_or(range.len());
            visual.extend(range[index..run_end].iter().rev());
            index = run_end;
        } else {
            visual.push(range[index]);
            index += 1;
        }
    }
    let mut placed = Vec::new();
    let mut x = 0.0;
    for word in visual {
        let shaped = &line.spans[word.span].words[word.index];
        if word.object {
            placed.push(Placed {
                bytes: word.bytes.clone(),
                x,
                width: word.width,
            });
            x += word.width;
            continue;
        }
        for glyph in &shaped.glyphs {
            let width = glyph.width(font_size);
            placed.push(Placed {
                bytes: glyph.start..glyph.end,
                x,
                width,
            });
            x += width;
        }
    }
    placed
}

/// COSMIC Text's own layout of the line at one width, for comparison.
pub fn reference_lines(line: &ShapeLine, font_size: f32, width: f32) -> Vec<Vec<Placed>> {
    line.layout(
        font_size,
        Some(width),
        Wrap::Word,
        None,
        None,
        Hinting::Disabled,
    )
    .into_iter()
    .map(|layout| {
        layout
            .glyphs
            .iter()
            .map(|glyph| Placed {
                bytes: glyph.start..glyph.end,
                x: glyph.x,
                width: glyph.w,
            })
            .collect()
    })
    .collect()
}

/// The byte range a placed line covers.
pub fn covered(placed: &[Placed]) -> Option<Range<usize>> {
    Some(
        placed.iter().map(|glyph| glyph.bytes.start).min()?
            ..placed.iter().map(|glyph| glyph.bytes.end).max()?,
    )
}
