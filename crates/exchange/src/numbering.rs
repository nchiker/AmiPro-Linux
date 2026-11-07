//! Ami Pro's bullets and numbers as the formats' lists (EX8): live lists
//! that renumber when edited in the other program, showing exactly Ami
//! Pro's numbers (`amipro_layout::paragraph_numbers`, `1200:1842`).
//!
//! Every style with an outline level shares one list, as Ami Pro counts a
//! level whatever the style; a style without a level has a list of its own.
//! Word and LibreOffice count a list instance on their own terms (a level
//! restarts after a higher one, or never), so the paragraphs are walked
//! with that counting beside Ami Pro's: where the two would show different
//! numbers (Ami Pro's After intervening style, a style's run starting
//! again), a new instance starts with Ami Pro's numbers; where no instance
//! can show them (two number formats in one level's place, daggers), the
//! paragraph shows its number as text.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use amipro_document::{Bullets, Paragraph, ParagraphStyle, StyleId};
use amipro_layout::{ParagraphNumber, number_text, paragraph_numbers};

use crate::{CharProps, List, ListInstance, ListLevel, NumberFormat, Underline};

/// The lists of a document and where each style's paragraphs belong.
#[derive(Clone, Debug, Default)]
pub struct Lists {
    pub lists: Vec<List>,
    pub instances: Vec<ListInstance>,
    /// Each style with bullets or numbers: its list and level (0-based).
    pub styles: BTreeMap<StyleId, (usize, u8)>,
}

/// One level's parts: literal text and the numbers it shows, as Ami Pro's
/// bullet text holds them.
#[derive(Clone, Debug, PartialEq)]
enum Piece {
    Text(String),
    /// A number code: its format, and which level's count it shows
    /// (cumulative codes count levels 1, 2, 3… in turn), or the paragraph's
    /// own.
    Number(NumberFormat, Option<u8>),
    /// A code no list can show (asterisks, daggers).
    Unlistable,
}

/// The pieces of `bullets`' text.
fn pieces(bullets: &Bullets) -> Vec<Piece> {
    let cumulative = bullets.flags & 8 != 0;
    let mut level = 0u8;
    let mut out = Vec::new();
    for code in bullets.text.chars() {
        let format = match code {
            '\u{1A}' => Some(NumberFormat::Decimal),
            '\u{1B}' => Some(NumberFormat::UpperRoman),
            '\u{1C}' => Some(NumberFormat::LowerRoman),
            '\u{1D}' => Some(NumberFormat::UpperLetter),
            '\u{1E}' => Some(NumberFormat::LowerLetter),
            _ => None,
        };
        if let Some(format) = format {
            level += 1;
            out.push(Piece::Number(format, cumulative.then_some(level)));
        } else if matches!(code, '\u{1F}' | '\u{7}') {
            out.push(Piece::Unlistable);
        } else {
            let text = bullet_character(code).map_or_else(|| code.to_string(), String::from);
            match out.last_mut() {
                Some(Piece::Text(last)) => last.push_str(&text),
                _ => out.push(Piece::Text(text)),
            }
        }
    }
    out
}

/// The character standing for one of Ami Pro's drawn bullets
/// (`BULLET_CODES`), the nearest Unicode has.
pub fn bullet_character(code: char) -> Option<&'static str> {
    Some(match code {
        '\u{10}' => "\u{00B7}",
        '\u{11}' => "\u{2022}",
        '\u{12}' => "\u{25AA}",
        '\u{13}' => "\u{25A0}",
        '\u{14}' => "\u{25A1}",
        '\u{15}' => "\u{2666}",
        '\u{16}' => "\u{25C6}",
        '\u{17}' => "\u{25E6}",
        '\u{18}' => "\u{25CB}",
        '\u{19}' => "\u{2713}",
        '\u{8}' => "\u{261E}",
        '\u{A}' => "\u{2751}",
        '\u{B}' => "\u{2750}",
        '\u{C}' => "\u{2611}",
        '\u{D}' => "\u{2612}",
        '\u{E}' => "\u{27A2}",
        '\u{F}' => "\u{27A4}",
        _ => return None,
    })
}

/// The text Ami Pro shows before a paragraph of `style` numbered `number`
/// (`1200:0af0`): bullets as their characters, number codes as the
/// paragraph's own number or, cumulative, each level's.
pub fn shown_text(bullets: &Bullets, number: &ParagraphNumber) -> String {
    let cumulative = bullets.flags & 8 != 0;
    let mut level = 0usize;
    let mut out = String::new();
    for code in bullets.text.chars() {
        let value = if cumulative && matches!(code, '\u{1A}'..='\u{1E}') {
            level += 1;
            number.levels.get(level - 1).copied().unwrap_or(0)
        } else {
            number.own
        };
        match number_text(code, value) {
            Some(text) => out.push_str(&text),
            None => out.push_str(bullet_character(code).unwrap_or(&code.to_string())),
        }
    }
    out
}

/// A level of a list from a style's bullets: Word's level text uses the
/// level's own format for every number it shows, so a level is listable
/// only when each number piece shows a level whose format it has.
fn level_of(style: &ParagraphStyle, level: u8) -> Option<ListLevel> {
    let bullets = &style.bullets;
    let pieces = pieces(bullets);
    if pieces.contains(&Piece::Unlistable) {
        return None;
    }
    let numbers: Vec<_> = pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Number(format, shown) => Some((*format, *shown)),
            _ => None,
        })
        .collect();
    // Own numbers shown twice, or a cumulative code past the paragraph's
    // level, cannot be a list's text.
    if numbers.iter().filter(|(_, shown)| shown.is_none()).count() > 1
        || numbers
            .iter()
            .any(|(_, shown)| shown.is_some_and(|shown| shown > level + 1))
    {
        return None;
    }
    let format = numbers
        .iter()
        .find(|(_, shown)| shown.is_none_or(|shown| shown == level + 1))
        .map_or(NumberFormat::Bullet, |(format, _)| *format);
    let mut text = String::new();
    for piece in &pieces {
        match piece {
            Piece::Text(literal) => text.push_str(&literal.replace('%', "%%")),
            Piece::Number(_, Some(shown)) => text.push_str(&format!("%{shown}")),
            Piece::Number(_, None) => text.push_str(&format!("%{}", level + 1)),
            Piece::Unlistable => {}
        }
    }
    let format = if numbers.is_empty() && text.is_empty() {
        NumberFormat::None
    } else {
        format
    };
    let indents = style.indents;
    Some(ListLevel {
        format,
        text,
        start: 1,
        restart_after_higher: bullets.flags & 2 != 0,
        font: None,
        props: bullet_props(bullets.attributes),
        right_align: bullets.flags & 16 != 0,
        indent_left: indents.all + indents.rest,
        hanging: indents.rest - indents.first,
    })
}

/// The bullets' text attributes (`Bullets::attributes`, `[fnt]` flags).
fn bullet_props(attributes: u16) -> CharProps {
    let set = |bit: u16| (attributes & bit != 0).then_some(true);
    CharProps {
        bold: set(1),
        italic: set(2),
        underline: if attributes & 8 != 0 {
            Some(Underline::Words)
        } else if attributes & 4 != 0 {
            Some(Underline::Single)
        } else {
            None
        },
        caps: set(16),
        script: (attributes & 256 != 0).then_some(crate::Script::Super),
        ..CharProps::default()
    }
}

/// A list's level that shows nothing, for the places no style fills.
fn empty_level() -> ListLevel {
    ListLevel {
        format: NumberFormat::None,
        text: String::new(),
        start: 1,
        restart_after_higher: true,
        font: None,
        props: CharProps::default(),
        right_align: false,
        indent_left: 0,
        hanging: 0,
    }
}

impl Lists {
    /// The lists of `styles`: one for the levelled styles (each level from
    /// the first style at it), one per style without a level.
    pub fn plan(styles: &BTreeMap<StyleId, ParagraphStyle>) -> Self {
        let mut lists = Self::default();
        let levelled: Vec<_> = styles
            .values()
            .filter(|style| style.bullets.level > 0 && !style.bullets.is_empty())
            .collect();
        if !levelled.is_empty() {
            let mut levels = vec![empty_level(); 9];
            let mut defined = [None::<StyleId>; 9];
            for style in &levelled {
                let level = style.bullets.level.min(9) - 1;
                if defined[usize::from(level)].is_none()
                    && let Some(definition) = level_of(style, level)
                {
                    levels[usize::from(level)] = definition;
                    defined[usize::from(level)] = Some(style.id);
                }
            }
            let outline = lists.lists.len();
            lists.lists.push(List { levels });
            for style in &levelled {
                let level = style.bullets.level.min(9) - 1;
                let shared = defined[usize::from(level)].is_some_and(|first| {
                    first == style.id || level_of(style, level) == level_of(&styles[&first], level)
                });
                if shared {
                    lists.styles.insert(style.id, (outline, level));
                } else if let Some(definition) = level_of(style, level) {
                    // A list of its own, at its level.
                    let mut levels = vec![empty_level(); 9];
                    levels[usize::from(level)] = definition;
                    lists.styles.insert(style.id, (lists.lists.len(), level));
                    lists.lists.push(List { levels });
                }
            }
        }
        for style in styles.values() {
            if style.bullets.level == 0
                && !style.bullets.is_empty()
                && let Some(definition) = level_of(style, 0)
            {
                lists.styles.insert(style.id, (lists.lists.len(), 0));
                lists.lists.push(List {
                    levels: vec![definition],
                });
            }
        }
        lists
    }

    /// The list instance and level of each paragraph of a flow, and the
    /// text of those no instance can number (`None` where the paragraph
    /// shows nothing before its text).
    pub fn number_flow(
        &mut self,
        paragraphs: &[Arc<Paragraph>],
        styles: &BTreeMap<StyleId, ParagraphStyle>,
    ) -> Vec<Numbered> {
        let numbers = paragraph_numbers(paragraphs, styles);
        // Each list's current instance in this flow and its counts.
        let mut current: HashMap<usize, (usize, [Option<u32>; 9])> = HashMap::new();
        let mut out = Vec::with_capacity(paragraphs.len());
        for (paragraph, number) in paragraphs.iter().zip(&numbers) {
            let Some(style) = styles.get(&paragraph.style) else {
                out.push(Numbered::None);
                continue;
            };
            if style.bullets.is_empty() {
                out.push(Numbered::None);
                continue;
            }
            let shown = shown_text(&style.bullets, number);
            let Some(&(list, level)) = self.styles.get(&style.id) else {
                out.push(Numbered::Text(shown));
                continue;
            };
            let numbered = style.bullets.numbered();
            // What the current instance would show here.
            let fits = |instance: &ListInstance, counts: &[Option<u32>; 9]| {
                let mut counts = *counts;
                advance(&self.lists[list], instance, &mut counts, level, numbered);
                self.text(list, instance, &counts, level) == shown
            };
            let reused = current.get(&list).and_then(|(index, counts)| {
                fits(&self.instances[*index], counts).then_some(*index)
            });
            let index = match reused {
                Some(index) => index,
                None => {
                    // A new instance starting where Ami Pro's numbers are.
                    let mut starts = [None; 9];
                    if numbered {
                        starts[usize::from(level)] = Some(number.own.max(1));
                        for (higher, count) in number.levels.iter().enumerate() {
                            if higher < usize::from(level) && *count > 0 {
                                starts[higher] = Some(*count);
                            }
                        }
                    }
                    let instance = ListInstance { list, starts };
                    if !fits(&instance, &[None; 9]) {
                        out.push(Numbered::Text(shown));
                        continue;
                    }
                    self.instances.push(instance);
                    current.insert(list, (self.instances.len() - 1, [None; 9]));
                    self.instances.len() - 1
                }
            };
            let (_, counts) = current.get_mut(&list).expect("an instance is current");
            advance(
                &self.lists[list],
                &self.instances[index],
                counts,
                level,
                numbered,
            );
            out.push(Numbered::List(index, level, shown));
        }
        out
    }

    /// The text an instance shows at `level` with `counts`.
    fn text(
        &self,
        list: usize,
        instance: &ListInstance,
        counts: &[Option<u32>; 9],
        level: u8,
    ) -> String {
        let levels = &self.lists[list].levels;
        let definition = &levels[usize::from(level)];
        let mut out = String::new();
        let mut chars = definition.text.chars().peekable();
        while let Some(character) = chars.next() {
            if character != '%' {
                out.push(character);
                continue;
            }
            match chars.next() {
                Some('%') => out.push('%'),
                Some(digit @ '1'..='9') => {
                    let shown = usize::from(digit as u8 - b'1');
                    let value = counts[shown]
                        .or(instance.starts[shown])
                        .unwrap_or(levels.get(shown).map_or(1, |level| level.start));
                    let format = levels
                        .get(shown)
                        .map_or(NumberFormat::Decimal, |level| level.format);
                    out.push_str(&format_number(format, value));
                }
                _ => {}
            }
        }
        out
    }
}

/// What a paragraph shows before its text.
#[derive(Clone, Debug, PartialEq)]
pub enum Numbered {
    None,
    /// A list instance and a level (0-based), and the text it shows.
    List(usize, u8, String),
    /// Text, where no instance can show it.
    Text(String),
}

/// Counts one paragraph at `level`, as Word and LibreOffice count: the
/// level's count goes up (or starts at the instance's start), and lower
/// levels that restart after a higher one start again.
fn advance(
    list: &List,
    instance: &ListInstance,
    counts: &mut [Option<u32>; 9],
    level: u8,
    numbered: bool,
) {
    let index = usize::from(level);
    if numbered {
        counts[index] = Some(match counts[index] {
            Some(count) => count + 1,
            None => instance.starts[index].unwrap_or(list.levels[index].start),
        });
    }
    for (count, level) in counts.iter_mut().zip(&list.levels).skip(index + 1) {
        if level.restart_after_higher {
            *count = None;
        }
    }
}

/// `number` in `format`.
pub fn format_number(format: NumberFormat, number: u32) -> String {
    match format {
        NumberFormat::Decimal => number.to_string(),
        NumberFormat::UpperRoman => amipro_document::roman_numeral(number).to_uppercase(),
        NumberFormat::LowerRoman => amipro_document::roman_numeral(number),
        NumberFormat::UpperLetter => amipro_document::letter_number(number).to_uppercase(),
        NumberFormat::LowerLetter => amipro_document::letter_number(number),
        NumberFormat::Bullet | NumberFormat::None => String::new(),
    }
}
