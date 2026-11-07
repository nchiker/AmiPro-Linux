//! Line breaking over shaped words with a width per line
//! (`docs/research/LINE_BREAKING_SPIKE.md`).

use crate::shaper::ShapedWord;

/// Where one line ends.
#[derive(Clone, Debug, PartialEq)]
pub struct LineBreak {
    /// The index after the line's last word (trailing blanks included).
    pub end: usize,
    /// Width of the content: trailing blanks hang outside it; a hyphen shown
    /// at a soft hyphen is included.
    pub width: f32,
    pub hyphen: bool,
    /// The line ends at U+2028 (or another forced break the caller marks).
    pub forced: bool,
    /// One word wider than the line, placed alone.
    pub overflow: bool,
}

/// Breaks the next line greedily from `start`, within `limit` twips. A word
/// for which `forced(index)` is true ends the line after it. A line that ends
/// at a soft hyphen shows a hyphen, which must fit too. A line ends at a
/// hyphenator's point only where at least `zone` twips were left before the
/// word (Ami Pro's hyphenation zone, decompile `10d0:09b9`), unless the word
/// starts the line.
pub fn next_line(
    words: &[ShapedWord],
    start: usize,
    limit: f32,
    hyphen_width: f32,
    zone: f32,
    forced: impl Fn(usize) -> bool,
) -> LineBreak {
    let content_of = |end: usize| -> f32 { words[start..end].iter().map(|word| word.width).sum() };
    let mut end = start;
    let mut content = 0.0f32;
    let mut blanks = 0.0f32;
    let mut has_content = false;
    let mut is_forced = false;
    let mut overflow = false;
    while end < words.len() {
        let word = &words[end];
        // A hard return that opens a paragraph makes no empty line: Ami Pro
        // sets the text after it on the paragraph's first line (corpus
        // caches: 20 of 28 such one-line paragraphs one line high; Ami Pro's
        // display of a user document; the notes run, NK10).
        if start == 0 && end == 0 && word.forced_break && word.bytes.len() == '\u{2028}'.len_utf8()
        {
            end += 1;
            continue;
        }
        if word.blank {
            blanks += word.width;
            end += 1;
            if word.forced_break || forced(end - 1) {
                is_forced = true;
                break;
            }
            continue;
        }
        let candidate = content + blanks + word.width;
        // Blanks that open a paragraph are its text too: a word too wide to
        // follow them starts the next line (VM, `_PROPOS2.STY`'s New
        // document: a space and a column-wide frame take two lines).
        if candidate > limit && (has_content || blanks > 0.0) {
            break;
        }
        overflow |= candidate > limit;
        content = candidate;
        blanks = 0.0;
        has_content = true;
        end += 1;
        if word.forced_break || forced(end - 1) {
            is_forced = true;
            break;
        }
    }
    let mut hyphen = false;
    if !is_forced && end < words.len() {
        while let Some(last) = (start..end).rev().find(|index| !words[*index].blank) {
            if !words[last].soft_hyphen && !words[last].hyphenation {
                break;
            }
            if words[last].hyphenation {
                let mut first = last;
                while first > start && words[first - 1].hyphenation {
                    first -= 1;
                }
                if first > start && limit - content_of(first) < zone {
                    // Too little room for hyphenating: the word moves on.
                    end = first;
                    content = (start..first)
                        .rev()
                        .find(|index| !words[*index].blank)
                        .map_or(0.0, |previous| content_of(previous + 1));
                    break;
                }
            }
            let with_hyphen = content_of(last + 1) + hyphen_width;
            match (start..last).rev().find(|index| !words[*index].blank) {
                Some(previous) if with_hyphen > limit => {
                    end = previous + 1;
                    while end < last && words[end].blank {
                        end += 1;
                    }
                    content = content_of(previous + 1);
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
    LineBreak {
        end,
        width: content,
        hyphen,
        forced: is_forced,
        overflow,
    }
}

/// The visual order of words `range` on one line: rule L2 of the Unicode
/// bidi algorithm over the words' levels, for a paragraph of base level
/// `base` (0 left to right, 1 right to left). Trailing blanks are dropped.
pub fn visual_order(words: &[ShapedWord], range: std::ops::Range<usize>, base: u8) -> Vec<usize> {
    let mut end = range.end;
    while end > range.start && words[end - 1].blank {
        end -= 1;
    }
    let mut order: Vec<(usize, u8)> = (range.start..end)
        .map(|index| (index, words[index].level.max(base)))
        .collect();
    let highest = order.iter().map(|(_, level)| *level).max().unwrap_or(0);
    let lowest_odd = order
        .iter()
        .map(|(_, level)| *level)
        .filter(|level| level % 2 == 1)
        .min()
        .unwrap_or(u8::MAX);
    // From the highest level down to the lowest odd level, reverse every
    // maximal run of words at that level or higher.
    let mut level = highest;
    while level >= lowest_odd && level > 0 {
        let mut index = 0;
        while index < order.len() {
            if order[index].1 >= level {
                let run_end = (index..order.len())
                    .find(|candidate| order[*candidate].1 < level)
                    .unwrap_or(order.len());
                order[index..run_end].reverse();
                index = run_end;
            } else {
                index += 1;
            }
        }
        level -= 1;
    }
    order.into_iter().map(|(index, _)| index).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(width: f32, blank: bool) -> ShapedWord {
        ShapedWord {
            bytes: 0..1,
            width,
            ascent: 0.0,
            descent: 0.0,
            blank,
            level: 0,
            forced_break: false,
            soft_hyphen: false,
            hyphenation: false,
            object: None,
            tab: None,
        }
    }

    #[test]
    fn hyphenator_points_need_the_zone_left_before_the_word() {
        // "aa ex|ten|sive": a line ends inside the word only where the
        // line had `zone` twips left before it (decompile `10d0:09b9`).
        let piece = |width| ShapedWord {
            hyphenation: true,
            ..word(width, false)
        };
        let words = vec![
            word(100.0, false),
            word(10.0, true),
            piece(40.0),
            piece(60.0),
            word(80.0, false),
        ];
        // 110 before the word; 110 + 40 + 60 + hyphen 20 = 230 fits 240.
        let line = next_line(&words, 0, 240.0, 20.0, 100.0, |_| false);
        assert_eq!((line.end, line.hyphen, line.width), (4, true, 230.0));
        // With 130 left before the word the zone of 140 is not met: the
        // whole word moves to the next line.
        let line = next_line(&words, 0, 240.0, 20.0, 140.0, |_| false);
        assert_eq!((line.end, line.hyphen, line.width), (2, false, 100.0));
        // A word that starts its line is hyphenated whatever the zone.
        let line = next_line(&words, 2, 130.0, 20.0, 1_000.0, |_| false);
        assert_eq!((line.end, line.hyphen), (4, true));
    }

    #[test]
    fn greedy_lines_hang_blanks_and_back_off_hyphens() {
        let words = vec![
            word(100.0, false),
            word(10.0, true),
            word(100.0, false),
            word(10.0, true),
            ShapedWord {
                soft_hyphen: true,
                hyphenation: false,
                ..word(50.0, false)
            },
            word(50.0, false),
        ];
        let line = next_line(&words, 0, 215.0, 20.0, 0.0, |_| false);
        assert_eq!(
            (line.end, line.width),
            (4, 210.0),
            "the trailing blank hangs"
        );
        // 210 + 10 + 50 + hyphen 20 = 290 > 280: back off to the plain break.
        let line = next_line(&words, 0, 280.0, 20.0, 0.0, |_| false);
        assert_eq!((line.end, line.hyphen), (4, false));
        let line = next_line(&words, 0, 290.0, 20.0, 0.0, |_| false);
        assert_eq!((line.end, line.hyphen, line.width), (5, true, 290.0));
        let line = next_line(&words, 0, 50.0, 20.0, 0.0, |_| false);
        assert_eq!((line.end, line.overflow), (2, true));
        let line = next_line(&words, 0, 1_000.0, 20.0, 0.0, |index| index == 2);
        assert_eq!((line.end, line.forced), (3, true));
    }

    #[test]
    fn visual_order_reverses_right_to_left_runs() {
        let mut words: Vec<ShapedWord> = (0..5).map(|_| word(10.0, false)).collect();
        words[1].level = 1;
        words[2].level = 1;
        words[3].level = 2;
        assert_eq!(visual_order(&words, 0..5, 0), [0, 3, 2, 1, 4]);
        // In a right-to-left paragraph the whole line reverses, and the
        // level-2 word keeps its place inside the reversed run.
        assert_eq!(visual_order(&words, 0..5, 1), [4, 3, 2, 1, 0]);
        words[4].blank = true;
        assert_eq!(visual_order(&words, 0..5, 0), [0, 3, 2, 1]);
    }
}
