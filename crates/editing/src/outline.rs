//! Format › Promote and Demote (`docs/Design/UI_SPEC.md` §22, decisions O1
//! and O2): a paragraph takes the style one outline level up or down. Ami
//! Pro does this in Outline mode only (Alt+Left, Alt+Right; VM fixture
//! OL1); the app does it on the page. A list made with the Numbering
//! button nests as 1. → A. → i.: its depths are styles "List 1"…"List 9"
//! at the outline levels below the deepest level the document's other
//! styles use, because Ami Pro counts by level whatever the style
//! (`docs/research/OUTLINE_NOTES.md`). A bulleted list nests too (the
//! user, 2026-10-18): "Bullet 2"…"Bullet 9", further in, the bullets going
//! diamond, dot, small square; bullets count nothing, so these take no
//! outline level.

use std::collections::BTreeMap;

use amipro_document::{Bullets, Document, FlowId, ParagraphStyle, StyleId, StyleOrigin, Twips};

use crate::lists::{ListKind, shown_list};
use crate::{Applied, ChangeReport, EditError};

/// Which way a paragraph moves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelChange {
    /// One level up (a smaller level number).
    Promote,
    /// One level down.
    Demote,
}

/// The number format of each nested list depth (decision O2): 1. A. i. a.
/// 1) a) i) (1) (a).
const LIST_NUMBERS: [&str; 9] = [
    "\u{1A}.", "\u{1D}.", "\u{1C}.", "\u{1E}.", "\u{1A})", "\u{1E})", "\u{1C})", "(\u{1A})",
    "(\u{1E})",
];

/// How much further in each depth of a nested list stands.
const LIST_STEP: Twips = 360;

/// The bullet of each depth of a nested bulleted list, repeating: Ami
/// Pro's small diamond (Bullet 1's in `_DEFAULT.STY`), dot, and small
/// square.
const BULLET_SHAPES: [char; 3] = ['\u{15}', '\u{11}', '\u{12}'];

/// The depth (1–9) of a bulleted list style without an outline level:
/// "Bullet n", or 1 for any other (the document's Bullet 1 or Bullet, or
/// a bullet style of the user's).
pub fn bullet_depth(style: &ParagraphStyle) -> Option<u8> {
    if style.bullets.level != 0 || shown_list(style) != Some(ListKind::Bullets) {
        return None;
    }
    Some(
        style
            .name
            .strip_prefix("Bullet ")
            .and_then(|depth| depth.parse::<u8>().ok())
            .filter(|depth| (1..=9).contains(depth))
            .unwrap_or(1),
    )
}

/// The depth (1–9) of a nested list style: "List n" with an outline level
/// and a number.
pub fn list_depth(style: &ParagraphStyle) -> Option<u8> {
    let depth: u8 = style.name.strip_prefix("List ")?.parse().ok()?;
    ((1..=9).contains(&depth) && style.bullets.level > 0 && style.bullets.numbered())
        .then_some(depth)
}

/// Whether `style` makes a flat numbered list (the Numbering button's,
/// without an outline level).
fn is_flat_list(style: &ParagraphStyle) -> bool {
    style.bullets.level == 0 && shown_list(style) == Some(ListKind::Numbering)
}

/// The outline level a nested list's first depth takes: one below the
/// deepest level the document's other styles use, or List 1's when the
/// document has one.
fn list_base(styles: &BTreeMap<StyleId, ParagraphStyle>) -> u8 {
    if let Some(first) = styles.values().find(|style| list_depth(style) == Some(1)) {
        return first.bullets.level;
    }
    styles
        .values()
        .filter(|style| list_depth(style).is_none())
        .map(|style| style.bullets.level)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .min(9)
}

/// The styles being made by one command, so that later paragraphs reuse
/// them.
struct Styles {
    styles: BTreeMap<StyleId, ParagraphStyle>,
    made: bool,
}

impl Styles {
    fn next_id(&self) -> StyleId {
        StyleId(self.styles.keys().next_back().map_or(1, |id| id.0 + 1))
    }

    fn insert(&mut self, mut style: ParagraphStyle) -> StyleId {
        style.id = self.next_id();
        style.origin = StyleOrigin::Document;
        style.next = None;
        style.kept.next_name = None;
        style.kept.function_key = crate::styles::next_function_key(&self.styles);
        let id = style.id;
        self.styles.insert(id, style);
        self.made = true;
        id
    }

    fn named(&self, name: &str) -> Option<&ParagraphStyle> {
        self.styles
            .values()
            .find(|style| style.name.eq_ignore_ascii_case(name))
    }

    /// A name not taken: `name`, else `name 2`, `name 3`…
    fn free_name(&self, name: &str) -> String {
        (1..)
            .map(|n| {
                if n == 1 {
                    name.to_owned()
                } else {
                    format!("{name} {n}")
                }
            })
            .find(|candidate| self.named(candidate).is_none())
            .expect("an unused name")
    }

    /// Nested list depth `depth`'s style, made from `from` (the list's
    /// style at a neighbouring depth, or the flat list's) when the
    /// document has none.
    fn list(&mut self, depth: u8, from: &ParagraphStyle) -> StyleId {
        let name = format!("List {depth}");
        if let Some(style) = self
            .named(&name)
            .filter(|style| list_depth(style) == Some(depth))
        {
            return style.id;
        }
        let base = list_base(&self.styles);
        let from_depth = list_depth(from).unwrap_or(1);
        let mut style = from.clone();
        style.name = self.free_name(&name);
        style.indents.all =
            (from.indents.all + (Twips::from(depth) - Twips::from(from_depth)) * LIST_STEP).max(0);
        style.bullets = Bullets {
            text: LIST_NUMBERS[usize::from(depth - 1)].into(),
            level: (base + depth - 1).min(9),
            // After higher level and Right align.
            flags: 2 | 16,
            ..Bullets::default()
        };
        self.insert(style)
    }

    /// Nested bulleted list depth `depth`'s style: the document's "Bullet
    /// n" for it, else at depth 1 the document's bullet style, else one
    /// made from `from` (the list's style at a neighbouring depth), further
    /// in or out by the step, with that depth's bullet.
    fn bullet(&mut self, depth: u8, from: &ParagraphStyle) -> StyleId {
        let name = format!("Bullet {depth}");
        if let Some(style) = self
            .named(&name)
            .filter(|style| bullet_depth(style) == Some(depth))
        {
            return style.id;
        }
        if depth == 1
            && let Some(style) = ListKind::Bullets.names().iter().find_map(|name| {
                self.named(name)
                    .filter(|style| bullet_depth(style).is_some())
            })
        {
            return style.id;
        }
        let from_depth = bullet_depth(from).unwrap_or(1);
        let mut style = from.clone();
        style.name = self.free_name(&name);
        style.indents.all =
            (from.indents.all + (Twips::from(depth) - Twips::from(from_depth)) * LIST_STEP).max(0);
        let shape = BULLET_SHAPES[usize::from(depth - 1) % BULLET_SHAPES.len()];
        style.bullets.text = style
            .bullets
            .text
            .chars()
            .map(|code| {
                if amipro_document::is_bullet_code(code) {
                    shape
                } else {
                    code
                }
            })
            .collect();
        self.insert(style)
    }

    /// The style at outline `level` for a paragraph in `from`: one already
    /// used in the document first, else the first at the level, else
    /// Ami Pro's Outline# (a copy at the level; OL1).
    fn at_level(
        &mut self,
        level: u8,
        from: &ParagraphStyle,
        used: &BTreeMap<StyleId, usize>,
    ) -> StyleId {
        let candidates: Vec<&ParagraphStyle> = self
            .styles
            .values()
            .filter(|style| style.bullets.level == level && list_depth(style).is_none())
            .collect();
        if let Some(style) = candidates.iter().max_by_key(|style| {
            (
                used.get(&style.id).copied().unwrap_or(0),
                std::cmp::Reverse(style.id),
            )
        }) {
            return style.id;
        }
        let mut style = from.clone();
        style.name = self.free_name(&format!("Outline{level}"));
        style.bullets.level = level;
        self.insert(style)
    }
}

/// Promote or Demote paragraphs `first..=last` of `flow`.
pub(crate) fn change_level(
    document: &Document,
    flow_id: FlowId,
    first: usize,
    last: usize,
    change: LevelChange,
) -> Result<Applied, EditError> {
    let flow = document.flow(flow_id).ok_or("flow does not exist")?;
    if first > last || last >= flow.len() {
        return Err("paragraph out of bounds");
    }
    let mut used: BTreeMap<StyleId, usize> = BTreeMap::new();
    for paragraph in document.paragraphs() {
        *used.entry(paragraph.style).or_default() += 1;
    }
    let mut styles = Styles {
        styles: document.styles().clone(),
        made: false,
    };
    let style_of = |index: usize| flow.paragraphs[index].style;
    let mut targets: BTreeMap<usize, StyleId> = BTreeMap::new();
    for index in first..=last {
        if targets.contains_key(&index) {
            continue;
        }
        let Some(style) = styles.styles.get(&style_of(index)).cloned() else {
            continue;
        };
        if let Some(depth) = list_depth(&style) {
            // A nested list moves one depth; the first does not promote.
            let depth = match change {
                LevelChange::Promote if depth > 1 => depth - 1,
                LevelChange::Demote if depth < 9 && list_base(&styles.styles) + depth <= 9 => {
                    depth + 1
                }
                _ => continue,
            };
            targets.insert(index, styles.list(depth, &style));
        } else if let Some(depth) = bullet_depth(&style) {
            // A bulleted list moves one depth; the first does not promote.
            let depth = match change {
                LevelChange::Promote if depth > 1 => depth - 1,
                LevelChange::Demote if depth < 9 => depth + 1,
                _ => continue,
            };
            targets.insert(index, styles.bullet(depth, &style));
        } else if is_flat_list(&style) {
            // Demoting an item of a flat numbered list makes it nested: its
            // unbroken run goes to depth 1, the demoted items to depth 2.
            if change == LevelChange::Promote || list_base(&styles.styles) >= 9 {
                continue;
            }
            let id = style.id;
            let mut start = index;
            while start > 0 && style_of(start - 1) == id {
                start -= 1;
            }
            let mut end = index;
            while end + 1 < flow.len() && style_of(end + 1) == id {
                end += 1;
            }
            let top = styles.list(1, &style);
            let top_style = styles.styles[&top].clone();
            let nested = styles.list(2, &top_style);
            for member in start..=end {
                let target = if (first..=last).contains(&member) {
                    nested
                } else {
                    top
                };
                targets.insert(member, target);
            }
        } else {
            let level = style.bullets.level;
            let level = if level > 0 {
                match change {
                    LevelChange::Promote if level > 1 => level - 1,
                    LevelChange::Demote if level < 9 => level + 1,
                    _ => continue,
                }
            } else {
                // None: the previous levelled paragraph's level, or one below
                // it (OL1); level 1 when there is none.
                let previous = (0..index)
                    .rev()
                    .find_map(|other| {
                        styles
                            .styles
                            .get(&style_of(other))
                            .map(|style| style.bullets.level)
                            .filter(|level| *level > 0)
                    })
                    .unwrap_or(0);
                match change {
                    LevelChange::Promote => previous.max(1),
                    LevelChange::Demote => (previous + 1).min(9),
                }
            };
            targets.insert(index, styles.at_level(level, &style, &used));
        }
    }
    let mut next = document.clone();
    let made = styles.made;
    if made {
        *next.styles_mut() = styles.styles;
    }
    let mut report = ChangeReport {
        styles: made,
        ..ChangeReport::default()
    };
    for (index, style) in targets {
        let paragraph = next
            .flow_mut(flow_id)
            .and_then(|flow| flow.paragraph_mut(index))
            .ok_or("paragraph out of bounds")?;
        if paragraph.style != style {
            paragraph.style = style;
            report.changed.push(paragraph.id);
        }
    }
    if !report.changed.is_empty() || made {
        report.flows.insert(flow_id);
    }
    next.validate()
        .map_err(|_| "the styles would not be valid")?;
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, History, apply};
    use amipro_document::default_styles;

    /// A document with Title (level 1), Subhead (level 2), Number List
    /// (flat), and Body Text, and paragraphs in the given styles.
    fn document(names: &[&str]) -> Document {
        let mut styles = default_styles();
        let body = styles
            .values()
            .find(|style| style.name == "Body Text")
            .unwrap()
            .clone();
        let mut add = |id: u64, name: &str, bullets: Bullets| {
            styles.insert(
                StyleId(id),
                ParagraphStyle {
                    id: StyleId(id),
                    name: name.into(),
                    bullets,
                    origin: StyleOrigin::Sheet,
                    ..body.clone()
                },
            );
        };
        add(
            100,
            "Title",
            Bullets {
                level: 1,
                ..Bullets::default()
            },
        );
        add(
            101,
            "Subhead",
            Bullets {
                level: 2,
                ..Bullets::default()
            },
        );
        add(
            102,
            "Number List",
            Bullets {
                text: "\u{1A}.".into(),
                flags: 16,
                ..Bullets::default()
            },
        );
        let mut document = Document::empty("Outline", styles);
        for name in names {
            let id = document
                .styles()
                .values()
                .find(|style| style.name == *name)
                .unwrap()
                .id;
            document.push_paragraph(*name, id);
        }
        document
    }

    fn names(document: &Document) -> Vec<String> {
        document
            .paragraphs()
            .iter()
            .map(|paragraph| document.styles()[&paragraph.style].name.clone())
            .collect()
    }

    fn change(document: &Document, first: usize, last: usize, change: LevelChange) -> Document {
        apply(
            document,
            &Command::ChangeLevel {
                flow: document.main_flow_id(),
                first,
                last,
                change,
            },
        )
        .unwrap()
        .document
    }

    #[test]
    fn headings_and_none_move_by_ami_pros_rule_ol1() {
        // OL1: Title, Subhead, Body Text, Subhead. Demote the last: no style
        // at level 3, so Outline3, a copy of Subhead at level 3.
        let document = document(&["Title", "Subhead", "Body Text", "Subhead"]);
        let demoted = change(&document, 3, 3, LevelChange::Demote);
        assert_eq!(names(&demoted)[3], "Outline3");
        let outline3 = &demoted.styles()[&demoted.paragraphs()[3].style];
        assert_eq!(outline3.bullets.level, 3);
        assert_eq!(outline3.origin, StyleOrigin::Document);
        assert_eq!(
            outline3.font_family,
            document.styles()[&StyleId(101)].font_family
        );
        assert_ne!(outline3.kept.function_key, 0);
        // Body Text after a Subhead: promoted to Subhead, demoted to the
        // existing Outline3.
        assert_eq!(
            names(&change(&demoted, 2, 2, LevelChange::Promote))[2],
            "Subhead"
        );
        assert_eq!(
            names(&change(&demoted, 2, 2, LevelChange::Demote))[2],
            "Outline3"
        );
        // Level 1 does not promote.
        assert_eq!(
            names(&change(&document, 0, 0, LevelChange::Promote))[0],
            "Title"
        );
        // Subhead promoted is Title.
        assert_eq!(
            names(&change(&document, 1, 1, LevelChange::Promote))[1],
            "Title"
        );
    }

    #[test]
    fn a_numbered_list_nests_as_one_a_i_o2() {
        // A flat list of four under a Title; demote the second and third.
        let document = document(&[
            "Title",
            "Number List",
            "Number List",
            "Number List",
            "Number List",
            "Body Text",
        ]);
        let mut history = History::new();
        let mut nested = document.clone();
        let flow = nested.main_flow_id();
        history
            .execute(
                &mut nested,
                (),
                &Command::ChangeLevel {
                    flow,
                    first: 2,
                    last: 3,
                    change: LevelChange::Demote,
                },
            )
            .unwrap();
        assert_eq!(
            names(&nested),
            ["Title", "List 1", "List 2", "List 2", "List 1", "Body Text"]
        );
        let style = |name: &str| {
            nested
                .styles()
                .values()
                .find(|style| style.name == name)
                .unwrap()
                .clone()
        };
        // Below Subhead (2), the deepest heading level: levels 3 and 4.
        assert_eq!(style("List 1").bullets.level, 3);
        assert_eq!(style("List 2").bullets.level, 4);
        assert_eq!(style("List 1").bullets.text, "\u{1A}.");
        assert_eq!(style("List 2").bullets.text, "\u{1D}.");
        assert_eq!(style("List 2").bullets.flags, 2 | 16);
        assert_eq!(
            style("List 2").indents.all,
            style("List 1").indents.all + 360
        );
        // Demote again: i.; promote back; List 1 does not promote.
        let deeper = change(&nested, 2, 2, LevelChange::Demote);
        assert_eq!(names(&deeper)[2], "List 3");
        let style3 = deeper
            .styles()
            .values()
            .find(|style| style.name == "List 3")
            .unwrap();
        assert_eq!(style3.bullets.text, "\u{1C}.");
        assert_eq!(
            names(&change(&deeper, 2, 2, LevelChange::Promote))[2],
            "List 2"
        );
        assert_eq!(
            names(&change(&nested, 1, 1, LevelChange::Promote))[1],
            "List 1"
        );
        // One undo step takes the made styles away.
        history.undo(&mut nested, ());
        assert_eq!(nested, document);
    }

    /// A document with Title, Subhead, Number List, Body Text, and a
    /// bullet style named `bullet` as `_DEFAULT.STY`'s Bullet 1 (a small
    /// diamond, 0.20 in), with paragraphs in the given styles.
    fn bulleted(bullet: &str, names: &[&str]) -> Document {
        let mut document = document(&[]);
        let body = document.styles()[&document.body_text_style()].clone();
        let style = ParagraphStyle {
            id: StyleId(103),
            name: bullet.into(),
            ..crate::lists::default_list_style(ListKind::Bullets, &body)
        };
        document.styles_mut().insert(style.id, style);
        for name in names {
            let id = document
                .styles()
                .values()
                .find(|style| style.name == *name)
                .unwrap()
                .id;
            document.push_paragraph(*name, id);
        }
        document
    }

    #[test]
    fn a_bulleted_list_nests_as_diamond_dot_square() {
        // The user, 2026-10-18: bullets nest as numbered lists do.
        let document = bulleted(
            "Bullet 1",
            &["Bullet 1", "Bullet 1", "Bullet 1", "Body Text"],
        );
        let mut history = History::new();
        let mut nested = document.clone();
        let flow = nested.main_flow_id();
        history
            .execute(
                &mut nested,
                (),
                &Command::ChangeLevel {
                    flow,
                    first: 1,
                    last: 1,
                    change: LevelChange::Demote,
                },
            )
            .unwrap();
        assert_eq!(
            names(&nested),
            ["Bullet 1", "Bullet 2", "Bullet 1", "Body Text"]
        );
        let style = |document: &Document, name: &str| {
            document
                .styles()
                .values()
                .find(|style| style.name == name)
                .unwrap()
                .clone()
        };
        let first = style(&nested, "Bullet 1");
        let second = style(&nested, "Bullet 2");
        assert_eq!(second.bullets.text, "\u{11}", "a dot");
        assert_eq!(second.bullets.level, 0, "bullets count nothing");
        assert_eq!(second.indents.all, first.indents.all + 360);
        assert_eq!(second.origin, StyleOrigin::Document);
        assert_ne!(second.kept.function_key, 0);
        assert_eq!(bullet_depth(&second), Some(2));
        // Deeper: a small square, then the diamond again; back up; Bullet
        // 1 does not promote, and the next item reuses Bullet 2.
        let third = change(&nested, 1, 1, LevelChange::Demote);
        assert_eq!(names(&third)[1], "Bullet 3");
        assert_eq!(style(&third, "Bullet 3").bullets.text, "\u{12}");
        assert_eq!(
            style(&third, "Bullet 3").indents.all,
            first.indents.all + 720
        );
        let fourth = change(&third, 1, 1, LevelChange::Demote);
        assert_eq!(style(&fourth, "Bullet 4").bullets.text, "\u{15}");
        assert_eq!(
            names(&change(&third, 1, 1, LevelChange::Promote))[1],
            "Bullet 2"
        );
        let up = change(&nested, 1, 1, LevelChange::Promote);
        assert_eq!(names(&up)[1], "Bullet 1");
        assert_eq!(up.styles().len(), nested.styles().len());
        assert_eq!(
            names(&change(&nested, 0, 0, LevelChange::Promote))[0],
            "Bullet 1"
        );
        let both = change(&nested, 2, 2, LevelChange::Demote);
        assert_eq!(names(&both)[2], "Bullet 2");
        assert_eq!(both.styles().len(), nested.styles().len());
        // One undo step takes the made style away.
        history.undo(&mut nested, ());
        assert_eq!(nested, document);

        // A sheet's Bullet: its second depth is Bullet 2, and back again.
        let plain = bulleted("Bullet", &["Bullet", "Bullet"]);
        let nested = change(&plain, 1, 1, LevelChange::Demote);
        assert_eq!(names(&nested), ["Bullet", "Bullet 2"]);
        assert_eq!(
            names(&change(&nested, 1, 1, LevelChange::Promote))[1],
            "Bullet"
        );
    }
}
