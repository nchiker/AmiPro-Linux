//! Bullets & numbers: what stands before a paragraph's first line, and how
//! paragraphs are numbered (decompile `1200:1384`, `1200:1489`,
//! `1200:159c`, `1200:0af0`, `1308:0caa`; VM fixtures BN1, BN2;
//! `docs/research/SAM_FORMAT_NOTES.md`, "Bullets and numbers").

use std::collections::BTreeMap;
use std::sync::Arc;

use amipro_document::{Bullets, Paragraph, ParagraphStyle, StyleId};

use crate::shaper::Shaper;

/// What a paragraph's first line draws before its text, from the line's
/// slot left edge; the text starts `width` after it.
#[derive(Clone, Debug, PartialEq)]
pub struct BulletBox {
    pub x: f32,
    pub width: f32,
    pub pieces: Vec<BulletPiece>,
    /// The text's attributes as `[fnt]` flags (`Bullets::attributes`).
    pub attributes: u16,
    /// Right align (`Bullets::flags` 16): the box ends a space before the
    /// first line's text when it is narrower (`1108:4107`).
    pub right_align: bool,
    /// The width of a space in the font the bullets' text is set in.
    pub space: f32,
    /// The text is set as superscript: 7/12 of the font's size, raised by
    /// 5/12 of it (`1200:168b`, `1108:4063`); never for asterisk or dagger
    /// numbers.
    pub superscript: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BulletPiece {
    /// Text drawn in the paragraph's font with the bullets' attributes; `x`
    /// from the box's left edge.
    Text { text: String, x: f32, width: f32 },
    /// One of Ami Pro's drawn bullets (`Bullets::text` codes U+0008 and
    /// U+000A–U+0019), in a box `size` twips high (the font's ascent).
    Shape {
        code: char,
        x: f32,
        width: f32,
        size: f32,
    },
}

/// A paragraph's numbers: its own (its style's level's count, or its run's
/// for a style without a level) and each outline level's count as of the
/// paragraph, which cumulative numbers show ("1.2.1").
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ParagraphNumber {
    pub own: u32,
    /// Levels 1–9; 0 where the level's count restarts before its next
    /// paragraph (Ami Pro shows 0 there).
    pub levels: [u32; 9],
}

impl ParagraphNumber {
    /// A paragraph numbered `own`, with no outline counts.
    pub fn own(own: u32) -> Self {
        Self {
            own,
            levels: [0; 9],
        }
    }
}

/// Whether a reset option of `flags` (`Bullets::flags`) restarts a level
/// after what was seen since its last paragraph (`1200:1842`): After higher
/// level (2) after a level above it, After intervening style (4) after any
/// other level or None; with neither it never restarts.
fn restarts(flags: u16, saw_higher: bool, saw_other: bool) -> bool {
    (flags & 2 != 0 && saw_higher) || (flags & 4 != 0 && saw_other)
}

/// The numbers of each paragraph of a flow, as Ami Pro counts them
/// (`1200:1842`; VM fixtures BN2, OL2): a style without an outline level
/// counts its unbroken run of paragraphs, so any other paragraph, an empty
/// one included, starts it again at 1; a level counts every paragraph at
/// that level, whatever its style, restarting by the reset options of the
/// style there (the first style with the level for a level the paragraph is
/// not at).
pub fn paragraph_numbers(
    paragraphs: &[Arc<Paragraph>],
    styles: &BTreeMap<StyleId, ParagraphStyle>,
) -> Vec<ParagraphNumber> {
    count_numbers(paragraphs, styles, None).0
}

/// The numbers of a flow that continues a master document's numbering:
/// each numbered style starts from its counter in `carried` (a member's
/// `[sequence]`), so that a level continues unless its reset options
/// restart it, and a style without a level continues when the flow starts
/// with it (`1200:19xx`, `10c8:2742`; `docs/research/MASTER_NOTES.md` §4).
pub fn paragraph_numbers_carried(
    paragraphs: &[Arc<Paragraph>],
    styles: &BTreeMap<StyleId, ParagraphStyle>,
    carried: Option<&amipro_document::contents::Sequence>,
) -> Vec<ParagraphNumber> {
    count_numbers(paragraphs, styles, carried).0
}

/// Each style with bullets or numbers and its counter at the end of the
/// flow, as a master run passes it on in `[sequence]` (`10c8:2b98`): a
/// level's count, or for a style without a level the run of paragraphs the
/// flow ends with (0 when another style ends it). In the styles' order.
pub fn end_counters<'s>(
    paragraphs: &[Arc<Paragraph>],
    styles: impl IntoIterator<Item = &'s ParagraphStyle>,
    all_styles: &BTreeMap<StyleId, ParagraphStyle>,
    carried: Option<&amipro_document::contents::Sequence>,
) -> Vec<(String, u32)> {
    let (_, counts, previous) = count_numbers(paragraphs, all_styles, carried);
    styles
        .into_iter()
        .filter(|style| !style.bullets.is_empty())
        .map(|style| {
            let level = style.bullets.level.min(9);
            let value = if level > 0 {
                counts[usize::from(level - 1)]
            } else {
                previous
                    .filter(|(id, _)| *id == style.id)
                    .map_or(0, |(_, number)| number)
            };
            (style.name.clone(), value)
        })
        .collect()
}

type Counts = (Vec<ParagraphNumber>, [u32; 9], Option<(StyleId, u32)>);

fn count_numbers(
    paragraphs: &[Arc<Paragraph>],
    styles: &BTreeMap<StyleId, ParagraphStyle>,
    carried: Option<&amipro_document::contents::Sequence>,
) -> Counts {
    // The reset options of the first style at each level.
    let mut level_flags = [0u16; 9];
    for level in 1..=9u8 {
        if let Some(style) = styles.values().find(|style| style.bullets.level == level) {
            level_flags[usize::from(level - 1)] = style.bullets.flags;
        }
    }
    let mut numbers = Vec::with_capacity(paragraphs.len());
    let mut previous: Option<(StyleId, u32)> = None;
    let mut counts = [0u32; 9];
    if let Some(carried) = carried {
        for style in styles.values() {
            let Some(value) = carried.style_value(&style.name) else {
                continue;
            };
            let level = style.bullets.level.min(9);
            if level > 0 {
                let count = &mut counts[usize::from(level - 1)];
                *count = (*count).max(value);
            } else if value > 0 {
                previous = Some((style.id, value));
            }
        }
    }
    let mut saw_higher = [false; 9];
    let mut saw_other = [false; 9];
    for paragraph in paragraphs {
        let style = styles.get(&paragraph.style);
        let level = style.map_or(0, |style| style.bullets.level.min(9));
        for other in 1..=9u8 {
            if other == level {
                continue;
            }
            let index = usize::from(other - 1);
            saw_other[index] = true;
            if level != 0 && level < other {
                saw_higher[index] = true;
            }
        }
        let own = if level > 0 {
            let index = usize::from(level - 1);
            let flags = style.map_or(0, |style| style.bullets.flags);
            if restarts(flags, saw_higher[index], saw_other[index]) {
                counts[index] = 0;
            }
            counts[index] += 1;
            saw_higher[index] = false;
            saw_other[index] = false;
            previous = None;
            counts[index]
        } else {
            let number = match (style, previous) {
                (Some(style), Some((id, number))) if id == style.id => number + 1,
                _ => 1,
            };
            previous = style.map(|style| (style.id, number));
            number
        };
        // Only what a paragraph shows is kept, so that an edit changes the
        // numbers of no paragraph that does not show one (incremental
        // layout reuses a tail of pages only when its numbers are equal).
        let bullets = style.map(|style| &style.bullets);
        let mut levels = [0u32; 9];
        if bullets.is_some_and(|bullets| bullets.flags & 8 != 0) {
            for (index, count) in levels.iter_mut().enumerate() {
                let at_level = usize::from(level) == index + 1;
                *count = if !at_level
                    && restarts(level_flags[index], saw_higher[index], saw_other[index])
                {
                    0
                } else {
                    counts[index]
                };
            }
        }
        let own = if bullets.is_some_and(Bullets::numbered) {
            own
        } else {
            0
        };
        numbers.push(ParagraphNumber { own, levels });
    }
    (numbers, counts, previous)
}

/// `number` in the format of code `code` (U+001A–U+001F, U+0007), or
/// `None` for a code that is not a number format.
pub fn number_text(code: char, number: u32) -> Option<String> {
    let count = number.min(100) as usize;
    Some(match code {
        '\u{1A}' => number.to_string(),
        '\u{1B}' => amipro_document::roman_numeral(number).to_uppercase(),
        '\u{1C}' => amipro_document::roman_numeral(number),
        '\u{1D}' => amipro_document::letter_number(number).to_uppercase(),
        '\u{1E}' => amipro_document::letter_number(number),
        '\u{1F}' => "*".repeat(count),
        '\u{7}' => "\u{2020}".repeat(count),
        _ => return None,
    })
}

/// The ascent and average character width, in twips, that size Ami Pro's
/// drawn bullets in a font of `size_points` (Windows' text metrics for the
/// usual faces).
pub fn bullet_metrics(size_points: f32) -> (f32, f32) {
    let size = size_points * 20.0;
    (size * 0.9, size * 0.4)
}

/// The width of a bullet put into the text (`<:B…>`) in a font of
/// `size_points`: the style bullet's with the same code (`10d0:3b66`,
/// `1200:159c`; `docs/research/BULLETS_NOTES.md`).
pub fn inline_bullet_width(code: char, size_points: f32) -> f32 {
    let (ascent, average) = bullet_metrics(size_points);
    shape_width(code, ascent, average).unwrap_or(0.0)
}

/// The width of drawn bullet `code` in a font whose ascent is `ascent`
/// (`1200:159c`); `None` for a code that is not a drawn bullet.
fn shape_width(code: char, ascent: f32, average: f32) -> Option<f32> {
    Some(match code {
        '\u{8}' | '\u{A}'..='\u{F}' | '\u{14}' => ascent,
        '\u{10}' => ascent / 4.0,
        '\u{11}' | '\u{13}' | '\u{15}' | '\u{17}' => ascent / 2.0,
        '\u{12}' => ascent / 3.0,
        '\u{16}' | '\u{18}' => ascent * 3.0 / 4.0,
        '\u{19}' => average,
        _ => return None,
    })
}

/// The bullet box of a paragraph set in `style` with paragraph number
/// `number`, or `None` when the style has no bullets: number codes become
/// numbers, drawn codes shapes, and other characters text, which the
/// shaper measures in the style's font with the bullets' bold and italic
/// (and capitals with Upper case).
pub fn bullet_box(
    bullets: &Bullets,
    number: ParagraphNumber,
    style: &ParagraphStyle,
    shaper: &mut dyn Shaper,
) -> Option<BulletBox> {
    if bullets.is_empty() {
        return None;
    }
    let mut font = style.clone();
    font.bold |= bullets.attributes & 1 != 0;
    font.italic |= bullets.attributes & 2 != 0;
    let upper = bullets.attributes & 16 != 0;
    // Ami Pro clears the script bits for asterisk and dagger numbers.
    let superscript = bullets.attributes & 256 != 0
        && !matches!(bullets.text.chars().next(), Some('\u{1F}' | '\u{7}'));
    let mut text_font = font.clone();
    if superscript {
        text_font.font_size_points *= 7.0 / 12.0;
    }
    // Right align's space, in the font the text is set in (VM fixture BL3).
    let space = shaper.measure(" ", &text_font);
    let (ascent, average) = bullet_metrics(style.font_size_points);
    let mut pieces = Vec::new();
    let mut text = String::new();
    let mut x = 0.0f32;
    let mut flush = |text: &mut String, x: &mut f32, pieces: &mut Vec<BulletPiece>| {
        if text.is_empty() {
            return;
        }
        let shown = if upper {
            text.to_uppercase()
        } else {
            std::mem::take(text)
        };
        text.clear();
        let width = shaper.measure(&shown, &text_font);
        pieces.push(BulletPiece::Text {
            text: shown,
            x: *x,
            width,
        });
        *x += width;
    };
    // Cumulative numbering (`1200:1384`): the k-th number code counts level
    // k, and a full stop right after the last level's code is not shown
    // (VM fixture OL2: "1.1.1." shows "1.1.1").
    let cumulative = bullets.flags & 8 != 0;
    let mut codes = 0usize;
    let mut after_last_level = false;
    for code in bullets.text.chars() {
        let value = if cumulative {
            number.levels.get(codes).copied().unwrap_or(number.own)
        } else {
            number.own
        };
        if let Some(shown) = number_text(code, value) {
            codes += 1;
            text.push_str(&shown);
            after_last_level = cumulative && codes == usize::from(bullets.level);
            continue;
        }
        let hidden = after_last_level && code == '.';
        after_last_level = false;
        if hidden {
            continue;
        }
        if let Some(width) = shape_width(code, ascent, average) {
            flush(&mut text, &mut x, &mut pieces);
            pieces.push(BulletPiece::Shape {
                code,
                x,
                width,
                size: ascent,
            });
            x += width;
        } else if code >= ' ' {
            text.push(code);
        }
    }
    flush(&mut text, &mut x, &mut pieces);
    Some(BulletBox {
        x: 0.0,
        width: x,
        pieces,
        attributes: bullets.attributes,
        right_align: bullets.flags & 16 != 0,
        space,
        superscript,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shaper::FixedShaper;
    use amipro_document::{Document, default_styles};

    fn numbered(level: u8, text: &str) -> Bullets {
        Bullets {
            text: text.into(),
            level,
            ..Bullets::default()
        }
    }

    #[test]
    fn numbers_count_runs_and_outline_levels_bn2() {
        // VM fixture BN2: Num, Num, Body, Num, Num, Num0, Num, (empty),
        // Num, Lvl2, Lvl2, Num, Lvl2, Lvl6, Lvl2.
        let mut styles = default_styles();
        let body = StyleId(1);
        let mut add = |id: u64, bullets: Bullets| {
            let mut style = styles[&body].clone();
            style.id = StyleId(id);
            style.bullets = bullets;
            styles.insert(style.id, style);
        };
        add(10, numbered(0, "\u{1A}."));
        add(11, numbered(0, "\u{1A}."));
        add(12, numbered(2, "\u{1D}."));
        add(13, numbered(6, "\u{1A})"));
        let sequence = [10, 10, 1, 10, 10, 11, 10, 1, 10, 12, 12, 10, 12, 13, 12];
        let mut document = Document::empty("Numbers", styles);
        for id in sequence {
            document.push_paragraph("x", StyleId(id));
        }
        let numbers = paragraph_numbers(
            document
                .flow(document.main_flow_id())
                .unwrap()
                .paragraphs
                .as_slice(),
            document.styles(),
        );
        let own: Vec<u32> = numbers.iter().map(|number| number.own).collect();
        assert_eq!(own, [1, 2, 0, 1, 2, 1, 1, 0, 1, 1, 2, 1, 3, 1, 4]);
    }

    fn outline(level: u8, text: &str, flags: u16) -> Bullets {
        Bullets {
            text: text.into(),
            level,
            flags,
            ..Bullets::default()
        }
    }

    /// Numbers of `sequence` (style IDs) with `bullets` added as styles 20…
    fn numbers_of(bullets: &[Bullets], sequence: &[u64]) -> Vec<ParagraphNumber> {
        let mut styles = default_styles();
        let body = StyleId(1);
        for (index, bullets) in bullets.iter().enumerate() {
            let mut style = styles[&body].clone();
            style.id = StyleId(20 + index as u64);
            style.name = format!("Level {index}");
            style.bullets = bullets.clone();
            styles.insert(style.id, style);
        }
        let mut document = Document::empty("Levels", styles);
        for id in sequence {
            document.push_paragraph("x", StyleId(*id));
        }
        paragraph_numbers(
            document
                .flow(document.main_flow_id())
                .unwrap()
                .paragraphs
                .as_slice(),
            document.styles(),
        )
    }

    #[test]
    fn a_master_documents_member_continues_its_numbered_styles_md3() {
        use amipro_document::contents::{Counter, Sequence};
        let mut styles = default_styles();
        let body = StyleId(1);
        for (id, name, bullets) in [
            (10, "Number List", numbered(0, "\u{1A}.")),
            (11, "Heading", numbered(1, "\u{1A}.")),
        ] {
            let mut style = styles[&body].clone();
            style.id = StyleId(id);
            style.name = name.into();
            style.bullets = bullets;
            styles.insert(style.id, style);
        }
        let mut document = Document::empty("Member", styles);
        for id in [10, 10, 1, 11] {
            document.push_paragraph("x", StyleId(id));
        }
        let carried = Sequence {
            styles: vec![
                Counter {
                    value: 3,
                    name: "Number List".into(),
                },
                Counter {
                    value: 2,
                    name: "Heading".into(),
                },
            ],
            ..Sequence::default()
        };
        let paragraphs = document.paragraphs();
        let numbers = paragraph_numbers_carried(paragraphs, document.styles(), Some(&carried));
        let own: Vec<u32> = numbers.iter().map(|number| number.own).collect();
        // The list continues at 4, 5; the level continues at 3.
        assert_eq!(own, [4, 5, 0, 3]);
        // At the end: the level's count; the list ended by Body Text.
        let ends = end_counters(
            paragraphs,
            document.styles().values(),
            document.styles(),
            Some(&carried),
        );
        assert!(ends.contains(&("Heading".to_string(), 3)));
        assert!(ends.contains(&("Number List".to_string(), 0)));
        // Without a carry the list starts at 1.
        assert_eq!(paragraph_numbers(paragraphs, document.styles())[0].own, 1);
    }

    #[test]
    fn reset_options_restart_a_level_as_ami_pro_does_1200_1842() {
        // Levels 1 and 2 numbered; Body Text (1) between. After higher
        // level: level 2 restarts after level 1 only. After intervening
        // style: after anything else. Neither: never.
        let sequence = [20, 21, 21, 1, 21, 20, 21];
        let own = |flags: u16| -> Vec<u32> {
            numbers_of(
                &[outline(1, "\u{1A}.", 2), outline(2, "\u{1D}.", flags)],
                &sequence,
            )
            .iter()
            .map(|number| number.own)
            .collect()
        };
        assert_eq!(own(2), [1, 1, 2, 0, 3, 2, 1]);
        assert_eq!(own(4), [1, 1, 2, 0, 1, 2, 1]);
        assert_eq!(own(0), [1, 1, 2, 0, 3, 2, 4]);
        // A level counts every paragraph at it, whatever the style: an
        // unnumbered heading at level 2 takes a number from the list.
        let heading_between = numbers_of(
            &[outline(2, "\u{1A}.", 0), outline(2, "", 0)],
            &[20, 21, 20],
        );
        assert_eq!(heading_between[2].own, 3);
    }

    #[test]
    fn cumulative_numbers_count_each_level_and_hide_the_last_full_stop_ol2() {
        // OL2: Quick Numbering's 1, 1.1, 1.1.1 on Title (1) and Subhead
        // (2): 1, 1.1, 1.2, 2, 2.1.
        let title = outline(1, "\u{1A}.", 2 | 8);
        let subhead = outline(2, "\u{1A}.\u{1A}.", 2 | 8);
        let numbers = numbers_of(&[title.clone(), subhead.clone()], &[20, 21, 21, 20, 21]);
        let style = default_styles()[&StyleId(1)].clone();
        let mut shaper = FixedShaper;
        let shown: Vec<String> = numbers
            .iter()
            .zip([&title, &subhead, &subhead, &title, &subhead])
            .map(|(number, bullets)| {
                let bullet = bullet_box(bullets, *number, &style, &mut shaper).unwrap();
                bullet
                    .pieces
                    .iter()
                    .map(|piece| match piece {
                        BulletPiece::Text { text, .. } => text.clone(),
                        BulletPiece::Shape { .. } => String::new(),
                    })
                    .collect()
            })
            .collect();
        assert_eq!(shown, ["1", "1.1", "1.2", "2", "2.1"]);
        // Without Cumulative every code shows the style's own level.
        let plain = outline(2, "\u{1A}.\u{1A}.", 2);
        let numbers = numbers_of(&[title, plain.clone()], &[20, 21]);
        let bullet = bullet_box(&plain, numbers[1], &style, &mut shaper).unwrap();
        assert!(matches!(&bullet.pieces[..], [BulletPiece::Text { text, .. }] if text == "1.1."));
    }

    #[test]
    fn number_formats_and_letters_past_z() {
        assert_eq!(number_text('\u{1A}', 12).unwrap(), "12");
        assert_eq!(number_text('\u{1B}', 14).unwrap(), "XIV");
        assert_eq!(number_text('\u{1C}', 9).unwrap(), "ix");
        assert_eq!(number_text('\u{1D}', 3).unwrap(), "C");
        assert_eq!(number_text('\u{1E}', 27).unwrap(), "aa");
        assert_eq!(number_text('\u{1E}', 28).unwrap(), "ab");
        assert_eq!(number_text('\u{1E}', 703).unwrap(), "aaa");
        assert_eq!(number_text('\u{1F}', 3).unwrap(), "***");
        assert_eq!(number_text('\u{7}', 2).unwrap(), "\u{2020}\u{2020}");
        assert_eq!(number_text('\u{11}', 1), None);
    }

    #[test]
    fn bullet_boxes_hold_shapes_numbers_and_text_bn1() {
        let style = default_styles()[&StyleId(1)].clone();
        let mut shaper = FixedShaper;
        // A dot: half the ascent wide.
        let dot = bullet_box(
            &numbered(0, "\u{11}"),
            ParagraphNumber::own(0),
            &style,
            &mut shaper,
        )
        .unwrap();
        assert_eq!(dot.width, 240.0 * 0.9 / 2.0);
        assert!(matches!(
            dot.pieces[..],
            [BulletPiece::Shape { code: '\u{11}', .. }]
        ));
        // "3." measured as text.
        let number = bullet_box(
            &numbered(0, "\u{1A}."),
            ParagraphNumber::own(3),
            &style,
            &mut shaper,
        )
        .unwrap();
        assert!(
            matches!(&number.pieces[..], [BulletPiece::Text { text, width, .. }] if text == "3." && *width == 240.0)
        );
        // "Note:" in capitals with Upper case.
        let note = Bullets {
            attributes: 16,
            ..numbered(0, "Note:")
        };
        let note = bullet_box(&note, ParagraphNumber::own(0), &style, &mut shaper).unwrap();
        assert!(matches!(&note.pieces[..], [BulletPiece::Text { text, .. }] if text == "NOTE:"));
        assert!(
            bullet_box(
                &Bullets::default(),
                ParagraphNumber::own(0),
                &style,
                &mut shaper
            )
            .is_none()
        );
    }
}
