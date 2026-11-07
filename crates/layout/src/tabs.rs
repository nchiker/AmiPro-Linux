//! Tab stops: which stops a paragraph's tabs use, and how wide each tab is
//! on its line (decompile `10d0:1881`, `10d0:49b8`, `10d0:3ef4`; VM
//! fixture TB2; `docs/research/SAM_FORMAT_NOTES.md`, "Tabs").

use amipro_document::{Indents, ParagraphStyle, TabKind, TabLeader, TabStop};

use crate::shaper::ShapedWord;

/// A right stop this close to the column's right edge moves to it.
const RIGHT_EDGE_SNAP: f32 = 280.0;

/// The tab stops of a paragraph in a column `column_width` twips wide,
/// positioned from the column's left edge: the style's own tabs while it
/// has them on, else the column's; with Hanging indent, a left stop where
/// the lines after the first start (All + Rest), ahead of the others, which
/// are dropped up to it.
pub fn tab_stops(
    style: &ParagraphStyle,
    indents: Indents,
    column_tabs: &[TabStop],
    column_width: f32,
) -> Vec<TabStop> {
    let mut stops: Vec<TabStop> = style
        .tabs_in_effect(column_tabs)
        .iter()
        .map(|stop| {
            let mut stop = *stop;
            if stop.kind == TabKind::Right && stop.position as f32 + RIGHT_EDGE_SNAP >= column_width
            {
                stop.position = column_width.round() as i32;
            }
            stop
        })
        .collect();
    stops.sort_by_key(|stop| stop.position);
    if style.hanging {
        let hanging = indents.all + indents.rest;
        stops.retain(|stop| stop.position > hanging);
        if hanging > 0 {
            stops.insert(0, TabStop::new(TabKind::Left, hanging));
        }
    }
    stops
}

/// Sizes the tabs of the line that starts at word `start`, set in a slot
/// `width` twips wide whose left edge is `origin` twips from the column's:
/// each tab reaches the first stop past where it starts, a left stop's text
/// starting there, a right stop's ending there, a centre stop's centred on
/// it, and a decimal stop's decimal point there (right with no point). A
/// tab with no stop left in the slot runs to the slot's end, so the text
/// after it starts the next line, and draws no leader. Words after a
/// forced break are left for their own line.
pub fn size_tabs(
    words: &mut [ShapedWord],
    start: usize,
    origin: f32,
    width: f32,
    stops: &[TabStop],
    text: &str,
) {
    let mut position = 0.0f32;
    for index in start..words.len() {
        if words[index].tab.is_some() {
            let at = origin + position;
            let stop = stops
                .iter()
                .find(|stop| stop.position as f32 > at + 0.5)
                .filter(|stop| (stop.position as f32) <= origin + width + 0.5);
            let Some(stop) = stop else {
                words[index].width = (width - position).max(0.0);
                words[index].tab = Some(TabLeader::None);
                return;
            };
            let reach = stop.position as f32 - at;
            let segment = || segment_after(words, index, text, false);
            let tab = match stop.kind {
                TabKind::Left => reach,
                TabKind::Right => reach - segment(),
                TabKind::Centre => reach - segment() / 2.0,
                TabKind::Decimal => reach - segment_after(words, index, text, true),
            };
            words[index].width = tab.max(0.0);
            words[index].tab = Some(stop.leader);
        }
        position += words[index].width;
        if words[index].forced_break {
            return;
        }
    }
}

/// The width of the text after tab `tab` up to the next tab or forced
/// break; with `decimal`, only up to the first decimal point in it (all of
/// it when there is none, which aligns the text right).
fn segment_after(words: &[ShapedWord], tab: usize, text: &str, decimal: bool) -> f32 {
    let mut width = 0.0;
    for word in &words[tab + 1..] {
        if word.tab.is_some() {
            break;
        }
        let slice = &text[word.bytes.clone()];
        if decimal && let Some(point) = slice.find('.') {
            // A point inside a word: its share of the word's width.
            let characters = slice.chars().count().max(1) as f32;
            return width + word.width * slice[..point].chars().count() as f32 / characters;
        }
        width += word.width;
        if word.forced_break {
            break;
        }
    }
    width
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{Breaks, StyleId, default_styles};

    fn word(text: &str, bytes: std::ops::Range<usize>, width: f32) -> ShapedWord {
        ShapedWord {
            bytes: bytes.clone(),
            width,
            ascent: 0.0,
            descent: 0.0,
            blank: text[bytes.clone()].chars().all(|c| c == ' '),
            level: 0,
            forced_break: false,
            soft_hyphen: false,
            hyphenation: false,
            object: None,
            tab: (&text[bytes] == "\t").then_some(TabLeader::None),
        }
    }

    /// Words of `text` split at tabs and spaces, each character 100 twips.
    fn words(text: &str) -> Vec<ShapedWord> {
        let mut words = Vec::new();
        let mut start = 0;
        for (at, character) in text.char_indices() {
            if character == '\t' || character == ' ' {
                if at > start {
                    words.push(word(text, start..at, 100.0 * (at - start) as f32));
                }
                let width = if character == ' ' { 100.0 } else { 0.0 };
                words.push(word(text, at..at + 1, width));
                start = at + 1;
            }
        }
        if start < text.len() {
            words.push(word(
                text,
                start..text.len(),
                100.0 * (text.len() - start) as f32,
            ));
        }
        words
    }

    fn tb2_stops() -> Vec<TabStop> {
        // VM fixture TB2: left 1440 with hyphens, centre 3600, right 5760
        // with dots, decimal 7920 with a line.
        vec![
            TabStop {
                leader: TabLeader::Hyphens,
                ..TabStop::new(TabKind::Left, 1440)
            },
            TabStop::new(TabKind::Centre, 3600),
            TabStop {
                leader: TabLeader::Dots,
                ..TabStop::new(TabKind::Right, 5760)
            },
            TabStop {
                leader: TabLeader::Line,
                ..TabStop::new(TabKind::Decimal, 7920)
            },
        ]
    }

    /// Where each word of `text` starts after sizing its tabs.
    fn starts(text: &str, origin: f32) -> (Vec<ShapedWord>, Vec<f32>) {
        let mut words = words(text);
        size_tabs(&mut words, 0, origin, 9360.0, &tb2_stops(), text);
        let mut x = 0.0;
        let starts = words
            .iter()
            .map(|word| {
                let start = x;
                x += word.width;
                start
            })
            .collect();
        (words, starts)
    }

    #[test]
    fn tabs_reach_their_stops_by_kind_tb2() {
        // "L\tleft": the text starts at the left stop, after hyphens.
        let (words, at) = starts("L\tleft", 0.0);
        assert_eq!(at[2], 1440.0);
        assert_eq!(words[1].tab, Some(TabLeader::Hyphens));
        // "\t\tcentred": centred on 3600.
        let (_, at) = starts("\t\tcentred", 0.0);
        assert_eq!(at[2], 3600.0 - 350.0);
        // A right stop: the text ends there.
        let (words, at) = starts("\t\t\tright", 0.0);
        assert_eq!(at[3] + 500.0, 5760.0);
        assert_eq!(words[2].tab, Some(TabLeader::Dots));
        // A decimal stop: the point stands there; without one, right.
        let (_, at) = starts("\t\t\t\t123.45", 0.0);
        assert_eq!(at[4] + 300.0, 7920.0);
        let (_, at) = starts("\t\t\t\tno point", 0.0);
        assert_eq!(at[6] + 500.0, 7920.0);
        // No stop left: the tab runs to the slot's end, without a leader.
        let (words, at) = starts("x\t\t\t\t\tbeyond", 0.0);
        assert_eq!(at[6], 9360.0);
        assert_eq!(words[5].tab, Some(TabLeader::None));
        // Stops are from the column's edge: an indented slot starts later.
        let (_, at) = starts("\tleft", 720.0);
        assert_eq!(at[1], 1440.0 - 720.0);
        // A stop the text has passed is skipped.
        let (_, at) = starts("abcdefghijklmnop\tx", 0.0);
        assert_eq!(at[2], 3600.0 - 50.0, "past 1440: the centre stop");
    }

    #[test]
    fn stops_come_from_the_style_or_the_column_with_hanging_indent() {
        let mut style = default_styles()[&StyleId(1)].clone();
        let column = vec![
            TabStop::new(TabKind::Left, 720),
            TabStop::new(TabKind::Right, 9200),
        ];
        let stops = tab_stops(&style, Indents::default(), &column, 9360.0);
        assert_eq!(stops[1].position, 9360, "a right stop near the edge snaps");
        style.tabs = vec![TabStop::new(TabKind::Left, 2000)];
        assert_eq!(
            tab_stops(&style, Indents::default(), &column, 9360.0)[0].position,
            720
        );
        style.breaks = Breaks(Breaks::WITHIN | Breaks::OWN_TABS);
        assert_eq!(
            tab_stops(&style, Indents::default(), &column, 9360.0),
            [TabStop::new(TabKind::Left, 2000)]
        );
        style.breaks = Breaks::default();
        style.hanging = true;
        let indents = Indents {
            all: 360,
            first: 0,
            rest: 720,
            right: 0,
        };
        let stops = tab_stops(&style, indents, &column, 9360.0);
        assert_eq!(
            stops,
            [
                TabStop::new(TabKind::Left, 1080),
                TabStop::new(TabKind::Right, 9360)
            ]
        );
    }
}
