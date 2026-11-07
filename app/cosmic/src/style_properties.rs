//! Style Properties in the context rail (`UI_SPEC.md` §15, approved
//! 2026-10-13): a style's definition edited beside the document, as Frame
//! Properties edits a frame. The fields are Ami Pro's Modify Style panels
//! (§14; VM screenshots outside Git: `i1-modify`, `modalign`, `modspace`,
//! `modbreaks`, `modbn`, `modlines`) as folding sections. Each change
//! applies at once as one command and one undo step, so the document is
//! the sample; a typed value applies on Enter or when its box loses focus.
//! The panel follows the caret's paragraph style; a style picked from its
//! list stays until the caret moves to a paragraph in another style.
//!
//! Lengths are shown and typed in inches, as Ami Pro's dialog shows them
//! by default; a number may also end in `in`, `"`, `cm`, `mm`, or `pt`.

use std::collections::{BTreeMap, BTreeSet};

use amipro_document::{
    Alignment, Breaks, Caps, Colour, LineSpacing, NumberKind, ParagraphStyle, RuleLength, StyleId,
    TabKind, TabLeader, TabStop, Twips, number, rule_strokes,
};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::character_dialogs::SIZES;
use crate::colour_control::{Context, Target};
use crate::style_dialogs::listed_styles;

/// The panel's folding sections, in Ami Pro's panel order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Section {
    Font,
    Alignment,
    Tabs,
    Spacing,
    Breaks,
    Bullets,
    Lines,
    TableFormat,
}

impl Section {
    pub const ALL: [(Self, &'static str); 8] = [
        (Self::Font, "Font"),
        (Self::Alignment, "Alignment and indents"),
        (Self::Tabs, "Tabs"),
        (Self::Spacing, "Spacing"),
        (Self::Breaks, "Breaks and next style"),
        (Self::Bullets, "Bullets and numbers"),
        (Self::Lines, "Lines"),
        (Self::TableFormat, "Table format"),
    ];
}

/// A box the user types a value into; its text is kept as typed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Field {
    Size,
    All,
    First,
    Rest,
    Right,
    Tab(usize),
    Custom,
    Above,
    Below,
    RuleAbove,
    RuleBelow,
    Other,
    NumberBefore,
    NumberAfter,
    BulletText,
    /// Table format (TN6): the decimal places and the three symbols.
    Decimals,
    DecimalSymbol,
    ThousandsSymbol,
    CurrencySymbol,
}

/// Font panel check boxes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontAttribute {
    Bold,
    Italic,
    Underline,
    WordUnderline,
    DoubleUnderline,
    /// 1st line bold: exclusive with Bold (Ami Pro, `FUN_11c8_026f`).
    FirstLineBold,
}

/// Breaks panel check boxes: `Breaks` bits.
const BREAK_BOXES: [(u16, &str); 5] = [
    (Breaks::PAGE_BEFORE, "Page break before paragraph"),
    (Breaks::PAGE_AFTER, "Page break after paragraph"),
    (Breaks::COLUMN_BEFORE, "Column break before paragraph"),
    (Breaks::COLUMN_AFTER, "Column break after paragraph"),
    (Breaks::WITHIN, "Allow page/column break within"),
];

const CAPS: [(Option<Caps>, &str); 5] = [
    (None, "Normal"),
    (Some(Caps::Upper), "All caps"),
    (Some(Caps::Lower), "Lower case"),
    (Some(Caps::Initial), "Initial caps"),
    (Some(Caps::Small), "Small caps"),
];

pub(crate) const TAB_KINDS: [(TabKind, &str); 4] = [
    (TabKind::Left, "Left"),
    (TabKind::Centre, "Center"),
    (TabKind::Right, "Right"),
    (TabKind::Decimal, "Decimal"),
];

pub(crate) const LEADERS: [(TabLeader, &str); 4] = [
    (TabLeader::None, "No leader"),
    (TabLeader::Dots, "Dots"),
    (TabLeader::Hyphens, "Hyphens"),
    (TabLeader::Line, "Line"),
];

/// Ami Pro's twelve paragraph rule styles (`rule_strokes`).
pub(crate) const RULE_STYLES: [&str; 12] = [
    "Single, ½ pt",
    "Single, 1 pt",
    "Single, 2 pt",
    "Single, 4 pt",
    "Single, 6 pt",
    "Single, 8 pt",
    "Single, 12 pt",
    "Double, 1 pt",
    "Double, 2 and 1 pt",
    "Triple, 1 pt",
    "Double, 3 and 1 pt",
    "Double, 1 and 3 pt",
];

/// The number codes Ami Pro's Number box offers, with the sample it shows
/// (VM, 2026-10-13: 1, I, i, A, a, *, †).
const NUMBERS: [(char, &str); 7] = [
    ('\u{1A}', "1"),
    ('\u{1B}', "I"),
    ('\u{1C}', "i"),
    ('\u{1D}', "A"),
    ('\u{1E}', "a"),
    ('\u{1F}', "*"),
    ('\u{7}', "†"),
];

/// The bullet codes, in Ami Pro's list order, with their names
/// (`page_editing::BULLET_NAMES`).
const BULLETS: [(char, &str); 17] = crate::page_editing::BULLET_NAMES;

/// Bullets & numbers' attribute boxes: `Bullets::attributes` flags.
const BULLET_ATTRIBUTES: [(u16, &str); 6] = [
    (1, "Bold"),
    (2, "Italic"),
    (4, "Underline"),
    (8, "Word underline"),
    (16, "All caps"),
    (256, "Superscript"),
];

/// The text before a paragraph as the dialog's parts: Ami Pro writes the
/// number with what is typed around it, then the Text, then the bullet
/// (VM fixture BN4: Number 1, Text "Hi", and a bullet are `<*:>Hi<*0>`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BulletParts {
    /// Cumulative numbering's earlier levels' numbers ("1." of "1.1."),
    /// before the style's own number.
    pub prefix: String,
    /// The text before the number code, the code, and the text after it.
    pub number: Option<(String, char, String)>,
    pub text: Option<String>,
    pub bullet: Option<char>,
}

impl BulletParts {
    /// The parts of `text`, or `None` when it is not in Ami Pro's order (it
    /// is then kept as it is unless the panel is used).
    pub fn split(text: &str) -> Option<Self> {
        let is_bullet = |character: char| BULLETS.iter().any(|(code, _)| *code == character);
        let is_number = |character: char| NUMBERS.iter().any(|(code, _)| *code == character);
        let mut rest = text;
        let mut parts = Self::default();
        if let Some(last) = rest.chars().last()
            && is_bullet(last)
        {
            parts.bullet = Some(last);
            rest = &rest[..rest.len() - last.len_utf8()];
        }
        if rest.chars().any(is_bullet) {
            return None;
        }
        // Cumulative numbering ("1.1."): the numbers before the last are
        // the earlier levels' (`UI_SPEC.md` §22).
        if rest.chars().filter(|c| is_number(*c)).count() > 1 {
            let (last, _) = rest
                .char_indices()
                .rev()
                .find(|(_, c)| is_number(*c))
                .expect("two numbers");
            parts.prefix = rest[..last].to_owned();
            rest = &rest[last..];
        }
        if let Some((at, code)) = rest.char_indices().find(|(_, c)| is_number(*c)) {
            let before = &rest[..at];
            let after_code = &rest[at + code.len_utf8()..];
            // Punctuation right after the number belongs to it ("1."), the
            // rest is the Text.
            let split = after_code
                .char_indices()
                .find(|(_, c)| c.is_alphanumeric() || c.is_whitespace())
                .map_or(after_code.len(), |(at, _)| at);
            if before.chars().any(char::is_alphanumeric) {
                return None;
            }
            parts.number = Some((before.to_owned(), code, after_code[..split].to_owned()));
            rest = &after_code[split..];
        }
        if !rest.is_empty() {
            parts.text = Some(rest.to_owned());
        }
        Some(parts)
    }

    pub fn join(&self) -> String {
        let mut text = self.prefix.clone();
        if let Some((before, code, after)) = &self.number {
            text.push_str(before);
            text.push(*code);
            text.push_str(after);
        }
        if let Some(own) = &self.text {
            text.push_str(own);
        }
        text.extend(self.bullet);
        text
    }
}

/// Cumulative numbering's numbers before a style's own at `level` (Ami
/// Pro: "incorporates the number, letter, or character specified in the
/// outline paragraph style set to the previous outline level"): each
/// earlier level's number and the text after it, from the first style at
/// that level, or "1." where none numbers.
fn cumulative_prefix(level: u8, styles: &BTreeMap<StyleId, ParagraphStyle>) -> String {
    (1..level)
        .map(|earlier| {
            styles
                .values()
                .find(|style| style.bullets.level == earlier)
                .and_then(|style| BulletParts::split(&style.bullets.text))
                .and_then(|parts| parts.number)
                .map_or_else(
                    || "\u{1A}.".to_owned(),
                    |(_, code, after)| format!("{code}{after}"),
                )
        })
        .collect()
}

/// The panel's state: the style shown and what is typed in its boxes. The
/// style's definition itself is the document's: every change applies.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleProperties {
    pub style: StyleId,
    /// The caret paragraph's style when last seen: when it changes, the
    /// panel follows it.
    pub caret_style: Option<StyleId>,
    /// The style as the panel last saw it in the document.
    pub draft: ParagraphStyle,
    /// What is typed in a box, as typed, until it applies.
    pub texts: BTreeMap<Field, String>,
    pub families: Vec<String>,
    /// The bullet parts, when the style's text is in Ami Pro's order.
    pub bullets: Option<BulletParts>,
    pub open: BTreeSet<Section>,
    /// Save as New Style: the name, while asked for.
    pub save_as: Option<String>,
    pub error: Option<String>,
}

/// What the panel asks the app to do after an action.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Stay,
    /// The style's new definition: one `ModifyStyle` command.
    Apply(Box<ParagraphStyle>),
    /// Make a style named `name` from `definition` (Save as New Style).
    SaveAs {
        name: String,
        based_on: StyleId,
        definition: Box<ParagraphStyle>,
    },
    /// ‹ Styles: back to the Styles list.
    Back,
    /// ×: close the rail.
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StyleAction {
    /// Show the listed style at this index (it stays until the caret moves
    /// to a paragraph in another style).
    Style(usize),
    Section(Section),
    Text(Field, String),
    Submit(Field),
    Hyphenation(bool),
    Face(usize),
    SizePreset(usize),
    Colour(Colour),
    Attribute(FontAttribute, bool),
    Caps(usize),
    Align(Alignment),
    BothSides(bool),
    Hanging(bool),
    OwnTabs(bool),
    TabKind(usize, usize),
    TabLeader(usize, usize),
    AddTab,
    RemoveTab(usize),
    LineSpacing(usize),
    AddInAlways(bool),
    Tightness(u16),
    Break(u16, bool),
    NextStyle(bool),
    NextStyleChoice(usize),
    Bullet(bool),
    BulletChoice(usize),
    Number(bool),
    NumberChoice(usize),
    BulletTextOn(bool),
    BulletAttribute(u16, bool),
    RightAlign(bool),
    /// Outline level: 0 for None, 1–9 (`UI_SPEC.md` §22).
    OutlineLevel(usize),
    /// Reset options: After higher level, After intervening style (each
    /// clears the other), Cumulative numbering.
    ResetHigher(bool),
    ResetIntervening(bool),
    Cumulative(bool),
    /// Table format: Cell format (General, Fixed Decimal, Currency,
    /// Percent), and its check boxes (`TABLE_DATA_NOTES.md` 10.2).
    CellFormat(usize),
    Thousands(bool),
    LeadingMinus(bool),
    TrailingMinus(bool),
    Parentheses(bool),
    Red(bool),
    SymbolLeading(bool),
    Rule(bool, bool),
    RuleStyle(bool, usize),
    RuleLength(usize),
    RuleColour(Colour),
    SaveAs,
    SaveAsName(String),
    SaveAsConfirm,
    SaveAsCancel,
    Back,
    Close,
}

/// Twips as inches with two decimals, as Ami Pro's dialog shows them.
pub fn inches(twips: Twips) -> String {
    format!("{:.2}", f64::from(twips) / 1440.0)
}

/// A typed length in twips, as Ami Pro's boxes read one (`lengths.rs`):
/// inches by default, or with `in`, `"`, `cm`, `mm`, or `pt`; from 0 to 22
/// inches.
pub fn parse_length(text: &str) -> Option<Twips> {
    crate::lengths::parse_length(text)
}

impl StyleProperties {
    /// The panel on `style`, with the caret in a paragraph of
    /// `caret_style`; `families` lists the faces offered.
    pub fn new(
        styles: &BTreeMap<StyleId, ParagraphStyle>,
        style: StyleId,
        caret_style: Option<StyleId>,
        mut families: Vec<String>,
    ) -> Option<Self> {
        let draft = styles.get(&style)?.clone();
        for style in styles.values() {
            if !families.contains(&style.font_family) {
                families.push(style.font_family.clone());
            }
        }
        families.sort_by_key(|family| family.to_lowercase());
        families.dedup();
        Some(Self {
            style,
            caret_style,
            bullets: BulletParts::split(&draft.bullets.text),
            draft,
            texts: BTreeMap::new(),
            families,
            open: BTreeSet::from([Section::Font]),
            save_as: None,
            error: None,
        })
    }

    /// Shows `style` (picked, or just made by Save as New Style).
    pub fn show(&mut self, styles: &BTreeMap<StyleId, ParagraphStyle>, style: StyleId) {
        let Some(definition) = styles.get(&style) else {
            return;
        };
        self.style = style;
        self.draft = definition.clone();
        self.bullets = BulletParts::split(&self.draft.bullets.text);
        self.texts.clear();
        self.error = None;
        self.save_as = None;
    }

    /// After every change to the document or the caret: follows the caret
    /// to a paragraph in another style, and takes the style's definition
    /// as the document has it (after an undo, say). Returns `false` when
    /// the style is gone and nothing can be shown.
    pub fn sync(
        &mut self,
        styles: &BTreeMap<StyleId, ParagraphStyle>,
        caret_style: Option<StyleId>,
    ) -> bool {
        if caret_style != self.caret_style {
            self.caret_style = caret_style;
            if let Some(caret) = caret_style
                && caret != self.style
            {
                self.show(styles, caret);
            }
        }
        match styles.get(&self.style) {
            Some(current) if *current == self.draft => true,
            Some(current) => {
                self.draft = current.clone();
                self.bullets = BulletParts::split(&self.draft.bullets.text);
                self.texts.clear();
                true
            }
            None => match caret_style.filter(|caret| styles.contains_key(caret)) {
                Some(caret) => {
                    self.show(styles, caret);
                    true
                }
                None => false,
            },
        }
    }

    /// The text a box shows: as typed, else the style's value.
    pub fn text(&self, field: Field) -> String {
        if let Some(text) = self.texts.get(&field) {
            return text.clone();
        }
        let draft = &self.draft;
        let rule_gap = |side: &amipro_document::RuleSide| {
            let thickness: Twips = rule_strokes(side.line).iter().sum();
            inches((side.space - thickness).max(0))
        };
        match field {
            Field::Size => format!("{}", draft.font_size_points),
            Field::All => inches(draft.indents.all),
            Field::First => inches(draft.indents.first),
            Field::Rest => inches(draft.indents.rest),
            Field::Right => inches(draft.indents.right),
            Field::Tab(index) => draft
                .tabs
                .get(index)
                .map_or_else(String::new, |stop| inches(stop.position)),
            Field::Custom => match draft.spacing.line {
                LineSpacing::Custom(height) => inches(height),
                _ => inches(288),
            },
            Field::Above => inches(draft.spacing.above),
            Field::Below => inches(draft.spacing.below),
            Field::RuleAbove => rule_gap(&draft.rules.above),
            Field::RuleBelow => rule_gap(&draft.rules.below),
            Field::Other => inches(draft.rules.other_length),
            Field::NumberBefore => self
                .bullets
                .as_ref()
                .and_then(|parts| parts.number.as_ref())
                .map_or_else(String::new, |(before, _, _)| before.clone()),
            Field::NumberAfter => self
                .bullets
                .as_ref()
                .and_then(|parts| parts.number.as_ref())
                .map_or_else(|| ".".into(), |(_, _, after)| after.clone()),
            Field::BulletText => self
                .bullets
                .as_ref()
                .and_then(|parts| parts.text.clone())
                .unwrap_or_default(),
            Field::Decimals => draft.kept.number_format.decimals.to_string(),
            Field::DecimalSymbol => draft.kept.number_format.decimal_symbol.clone(),
            Field::ThousandsSymbol => draft.kept.number_format.thousands_symbol.clone(),
            Field::CurrencySymbol => draft.kept.number_format.currency_symbol.clone(),
        }
    }

    /// Applies `action`; `styles` are the document's.
    pub fn update(
        &mut self,
        action: StyleAction,
        styles: &BTreeMap<StyleId, ParagraphStyle>,
    ) -> Outcome {
        self.error = None;
        match action {
            StyleAction::Style(index) => {
                if let Some((id, _)) = listed_styles(styles).get(index) {
                    self.show(styles, *id);
                }
                return Outcome::Stay;
            }
            StyleAction::Section(section) => {
                if !self.open.remove(&section) {
                    self.open.insert(section);
                }
                return Outcome::Stay;
            }
            StyleAction::Text(field, text) => {
                self.texts.insert(field, text);
                return Outcome::Stay;
            }
            StyleAction::Back => return Outcome::Back,
            StyleAction::Close => return Outcome::Close,
            StyleAction::SaveAs => {
                self.save_as = Some(String::new());
                return Outcome::Stay;
            }
            StyleAction::SaveAsName(name) => {
                self.save_as = Some(name);
                return Outcome::Stay;
            }
            StyleAction::SaveAsCancel => {
                self.save_as = None;
                return Outcome::Stay;
            }
            StyleAction::SaveAsConfirm => {
                let Some(name) = self.save_as.clone() else {
                    return Outcome::Stay;
                };
                return Outcome::SaveAs {
                    name,
                    based_on: self.style,
                    definition: Box::new(self.draft.clone()),
                };
            }
            StyleAction::Submit(field) => {
                let Some(text) = self.texts.get(&field).cloned() else {
                    return Outcome::Stay;
                };
                let mut bullets = self.bullets.clone();
                match apply_text(&mut self.draft, bullets.as_mut(), field, &text) {
                    Ok(changed) => {
                        self.texts.remove(&field);
                        if changed && let Some(parts) = &bullets {
                            self.draft.bullets.text = parts.join();
                        }
                        self.bullets = bullets;
                    }
                    Err(problem) => {
                        self.error = Some(problem);
                        return Outcome::Stay;
                    }
                }
            }
            other => self.edit(other, styles),
        }
        if styles.get(&self.style) == Some(&self.draft) {
            Outcome::Stay
        } else {
            Outcome::Apply(Box::new(self.draft.clone()))
        }
    }

    fn edit(&mut self, action: StyleAction, styles: &BTreeMap<StyleId, ParagraphStyle>) {
        let listed = listed_styles(styles);
        let mut bullets = self.bullets.clone();
        let mut bullets_changed = false;
        let draft = &mut self.draft;
        match action {
            StyleAction::Hyphenation(on) => draft.hyphenation = on,
            StyleAction::Face(index) => {
                if let Some(family) = self.families.get(index) {
                    draft.font_family = family.clone();
                }
            }
            StyleAction::SizePreset(index) => {
                if let Some(size) = SIZES.get(index).and_then(|size| size.parse().ok()) {
                    draft.font_size_points = size;
                    self.texts.remove(&Field::Size);
                }
            }
            StyleAction::Colour(colour) => draft.attributes.colour = colour,
            StyleAction::Attribute(attribute, on) => match attribute {
                FontAttribute::Bold => {
                    draft.bold = on;
                    if on {
                        draft.first_line_bold = false;
                    }
                }
                FontAttribute::FirstLineBold => {
                    draft.first_line_bold = on;
                    if on {
                        draft.bold = false;
                    }
                }
                FontAttribute::Italic => draft.italic = on,
                FontAttribute::Underline => draft.attributes.underline = on,
                FontAttribute::WordUnderline => draft.attributes.word_underline = on,
                FontAttribute::DoubleUnderline => draft.attributes.double_underline = on,
            },
            StyleAction::Caps(index) => {
                if let Some((caps, _)) = CAPS.get(index) {
                    draft.attributes.caps = *caps;
                }
            }
            StyleAction::Align(alignment) => draft.alignment = alignment,
            StyleAction::BothSides(on) => draft.both_sides = on,
            StyleAction::Hanging(on) => draft.hanging = on,
            StyleAction::OwnTabs(on) => set_bit(&mut draft.breaks.0, Breaks::OWN_TABS, on),
            StyleAction::TabKind(index, kind) => {
                if let (Some(stop), Some((kind, _))) =
                    (draft.tabs.get_mut(index), TAB_KINDS.get(kind))
                {
                    stop.kind = *kind;
                }
            }
            StyleAction::TabLeader(index, leader) => {
                if let (Some(stop), Some((leader, _))) =
                    (draft.tabs.get_mut(index), LEADERS.get(leader))
                {
                    stop.leader = *leader;
                }
            }
            StyleAction::AddTab => {
                // Ami Pro keeps at most 22 of a style's tabs.
                if draft.tabs.len() < 22 {
                    let position = draft.tabs.last().map_or(720, |stop| stop.position + 720);
                    draft.tabs.push(TabStop::new(TabKind::Left, position));
                    set_bit(&mut draft.breaks.0, Breaks::OWN_TABS, true);
                }
            }
            StyleAction::RemoveTab(index) => {
                if index < draft.tabs.len() {
                    draft.tabs.remove(index);
                    self.texts
                        .retain(|field, _| !matches!(field, Field::Tab(_)));
                }
            }
            StyleAction::LineSpacing(choice) => {
                draft.spacing.line = match choice {
                    0 => LineSpacing::Single,
                    1 => LineSpacing::OneAndAHalf(0),
                    2 => LineSpacing::Double(0),
                    _ => match draft.spacing.line {
                        LineSpacing::Custom(height) => LineSpacing::Custom(height),
                        _ => LineSpacing::Custom(288),
                    },
                };
            }
            StyleAction::AddInAlways(always) => draft.spacing.add_at_break = always,
            StyleAction::Tightness(percent) => draft.spacing.tightness = percent,
            StyleAction::Break(bit, on) => {
                set_bit(&mut draft.breaks.0, bit, on);
                // Keep With is offered only when breaks within are not
                // allowed (B1–B8).
                if bit == Breaks::WITHIN && on {
                    set_bit(&mut draft.breaks.0, Breaks::KEEP_WITH_PREVIOUS, false);
                    set_bit(&mut draft.breaks.0, Breaks::KEEP_WITH_NEXT, false);
                }
            }
            StyleAction::NextStyle(on) => {
                set_bit(&mut draft.breaks.0, Breaks::NEXT_STYLE, on);
            }
            StyleAction::NextStyleChoice(index) => {
                if let Some((id, _)) = listed.get(index) {
                    draft.next = (*id != draft.id).then_some(*id);
                    set_bit(&mut draft.breaks.0, Breaks::NEXT_STYLE, true);
                }
            }
            StyleAction::Bullet(on) => {
                let parts = bullets.get_or_insert_with(BulletParts::default);
                parts.bullet = on.then_some(BULLETS[0].0);
                bullets_changed = true;
            }
            StyleAction::BulletChoice(index) => {
                let parts = bullets.get_or_insert_with(BulletParts::default);
                parts.bullet = BULLETS.get(index).map(|(code, _)| *code);
                bullets_changed = true;
            }
            StyleAction::Number(on) => {
                let parts = bullets.get_or_insert_with(BulletParts::default);
                parts.number = on.then(|| (String::new(), NUMBERS[0].0, ".".into()));
                bullets_changed = true;
            }
            StyleAction::NumberChoice(index) => {
                let parts = bullets.get_or_insert_with(BulletParts::default);
                let (before, _, after) = parts
                    .number
                    .clone()
                    .unwrap_or_else(|| (String::new(), NUMBERS[0].0, ".".into()));
                if let Some((code, _)) = NUMBERS.get(index) {
                    parts.number = Some((before, *code, after));
                }
                bullets_changed = true;
            }
            StyleAction::BulletTextOn(on) => {
                let parts = bullets.get_or_insert_with(BulletParts::default);
                parts.text = on.then(|| parts.text.clone().unwrap_or_default());
                bullets_changed = true;
            }
            StyleAction::BulletAttribute(flag, on) => {
                set_bit(&mut draft.bullets.attributes, flag, on);
            }
            StyleAction::RightAlign(on) => set_bit(&mut draft.bullets.flags, 16, on),
            StyleAction::OutlineLevel(level) => {
                draft.bullets.level = level.min(9) as u8;
                if draft.bullets.flags & 8 != 0
                    && let Some(parts) = bullets.as_mut()
                {
                    parts.prefix = cumulative_prefix(draft.bullets.level, styles);
                    bullets_changed = true;
                }
            }
            StyleAction::ResetHigher(on) => {
                set_bit(&mut draft.bullets.flags, 2, on);
                if on {
                    set_bit(&mut draft.bullets.flags, 4, false);
                }
            }
            StyleAction::ResetIntervening(on) => {
                set_bit(&mut draft.bullets.flags, 4, on);
                if on {
                    set_bit(&mut draft.bullets.flags, 2, false);
                }
            }
            StyleAction::Cumulative(on) => {
                set_bit(&mut draft.bullets.flags, 8, on);
                if let Some(parts) = bullets.as_mut() {
                    parts.prefix = if on {
                        cumulative_prefix(draft.bullets.level, styles)
                    } else {
                        String::new()
                    };
                    bullets_changed = true;
                }
            }
            StyleAction::Rule(above, on) => {
                if above {
                    draft.rules.above.on = on;
                } else {
                    draft.rules.below.on = on;
                }
            }
            StyleAction::RuleStyle(above, index) => {
                let side = if above {
                    &mut draft.rules.above
                } else {
                    &mut draft.rules.below
                };
                // The gap shown stays; the space grows with the line.
                let old: Twips = rule_strokes(side.line).iter().sum();
                side.line = u8::try_from(index + 1).unwrap_or(1);
                let new: Twips = rule_strokes(side.line).iter().sum();
                side.space = (side.space - old).max(0) + new;
                side.on = true;
            }
            StyleAction::RuleLength(index) => {
                draft.rules.length = match index {
                    0 => RuleLength::Text,
                    1 => RuleLength::Margins,
                    _ => RuleLength::Other,
                };
            }
            StyleAction::RuleColour(colour) => draft.rules.colour = colour,
            StyleAction::CellFormat(index) => {
                if let Some(kind) = CELL_FORMATS.get(index) {
                    draft.kept.number_format.kind = *kind;
                }
            }
            StyleAction::Thousands(on) => set_flag(draft, number::THOUSANDS, on),
            // Leading and Trailing minus exclude each other (Ami Pro's
            // dialog); Parentheses and Red stand alone.
            StyleAction::LeadingMinus(on) => {
                set_flag(draft, number::LEADING_MINUS, on);
                if on {
                    set_flag(draft, number::TRAILING_MINUS, false);
                }
            }
            StyleAction::TrailingMinus(on) => {
                set_flag(draft, number::TRAILING_MINUS, on);
                if on {
                    set_flag(draft, number::LEADING_MINUS, false);
                }
            }
            StyleAction::Parentheses(on) => set_flag(draft, number::PARENTHESES, on),
            StyleAction::Red(on) => set_flag(draft, number::RED, on),
            StyleAction::SymbolLeading(on) => set_flag(draft, number::SYMBOL_LEADING, on),
            _ => {}
        }
        if bullets_changed && let Some(parts) = &bullets {
            draft.bullets.text = parts.join();
        }
        if bullets_changed {
            self.bullets = bullets;
        }
    }
}

/// The cell formats Modify Style offers, in order (`[nfmt]` value 2: 1–4).
const CELL_FORMATS: [NumberKind; 4] = [
    NumberKind::General,
    NumberKind::Fixed,
    NumberKind::Currency,
    NumberKind::Percent,
];

fn set_flag(draft: &mut ParagraphStyle, flag: u32, on: bool) {
    if on {
        draft.kept.number_format.flags |= flag;
    } else {
        draft.kept.number_format.flags &= !flag;
    }
}

fn set_bit(value: &mut u16, bit: u16, on: bool) {
    if on {
        *value |= bit;
    } else {
        *value &= !bit;
    }
}

/// A typed value into the draft; `Err` says why it is not taken (the box
/// keeps the text). `Ok(true)` when the bullet parts changed.
fn apply_text(
    draft: &mut ParagraphStyle,
    bullets: Option<&mut BulletParts>,
    field: Field,
    text: &str,
) -> Result<bool, String> {
    let length =
        || parse_length(text).ok_or_else(|| "Type a length from 0 to 22 inches".to_owned());
    match field {
        Field::Size => {
            let size: f32 = text
                .trim()
                .parse()
                .ok()
                .filter(|size: &f32| (1.0..=999.0).contains(size))
                .ok_or("Type a point size from 1 to 999")?;
            draft.font_size_points = size;
        }
        Field::All => draft.indents.all = length()?,
        Field::First => draft.indents.first = length()?,
        Field::Rest => draft.indents.rest = length()?,
        Field::Right => draft.indents.right = length()?,
        Field::Tab(index) => {
            let position = length()?;
            if let Some(stop) = draft.tabs.get_mut(index) {
                stop.position = position;
            }
        }
        Field::Custom => {
            let height = length()?;
            if height == 0 {
                return Err("A custom line height is more than 0".into());
            }
            draft.spacing.line = LineSpacing::Custom(height);
        }
        Field::Above => draft.spacing.above = length()?,
        Field::Below => draft.spacing.below = length()?,
        Field::RuleAbove | Field::RuleBelow => {
            let gap = length()?;
            let side = if field == Field::RuleAbove {
                &mut draft.rules.above
            } else {
                &mut draft.rules.below
            };
            side.space = gap + rule_strokes(side.line).iter().sum::<Twips>();
        }
        Field::Other => draft.rules.other_length = length()?,
        Field::Decimals => {
            draft.kept.number_format.decimals = text
                .trim()
                .parse()
                .ok()
                .filter(|places| *places <= 15)
                .ok_or("Type a number of decimal places from 0 to 15")?;
        }
        Field::DecimalSymbol | Field::ThousandsSymbol => {
            if text.chars().count() > 1 || text.chars().any(|c| (c as u32) < 0x20) {
                return Err("Type one character, or none".into());
            }
            let format = &mut draft.kept.number_format;
            if field == Field::DecimalSymbol {
                format.decimal_symbol = text.to_owned();
            } else {
                format.thousands_symbol = text.to_owned();
            }
        }
        Field::CurrencySymbol => {
            if text.chars().count() > 4 || text.chars().any(|c| (c as u32) < 0x20) {
                return Err("Type a currency symbol of up to four characters".into());
            }
            draft.kept.number_format.currency_symbol = text.to_owned();
        }
        Field::NumberBefore | Field::NumberAfter | Field::BulletText => {
            let Some(parts) = bullets else {
                return Ok(false);
            };
            if text.chars().any(|character| (character as u32) < 0x20) {
                return Err("Control characters are not allowed here".into());
            }
            match field {
                Field::BulletText => parts.text = Some(text.to_owned()),
                _ => {
                    let (before, code, after) = parts
                        .number
                        .clone()
                        .unwrap_or_else(|| (String::new(), NUMBERS[0].0, String::new()));
                    parts.number = Some(if field == Field::NumberBefore {
                        (text.to_owned(), code, after)
                    } else {
                        (before, code, text.to_owned())
                    });
                }
            }
            return Ok(true);
        }
    }
    Ok(false)
}

// ----- View: the rail, about 300 px wide -----

/// The rail's width while Style Properties shows (`UI_SPEC.md` §15).
pub const WIDTH: f32 = 300.0;

fn send(action: StyleAction) -> Message {
    Message::StyleProperties(action)
}

fn input(state: &StyleProperties, field: Field, width: f32) -> Element<'static, Message> {
    widget::text_input("", state.text(field))
        .width(Length::Fixed(width))
        .on_input(move |text| send(StyleAction::Text(field, text)))
        .on_submit(move |_| send(StyleAction::Submit(field)))
        .on_unfocus(send(StyleAction::Submit(field)))
        .into()
}

fn check(
    on: bool,
    text: &'static str,
    action: impl Fn(bool) -> StyleAction + 'static,
) -> Element<'static, Message> {
    widget::checkbox(on)
        .label(text)
        .on_toggle(move |on| send(action(on)))
        .into()
}

fn row(children: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    widget::row::with_children(children)
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

/// Radio buttons side by side, in columns `width` wide.
fn choices(children: Vec<Element<'static, Message>>, width: f32) -> Element<'static, Message> {
    widget::row::with_children(
        children
            .into_iter()
            .map(|child| widget::container(child).width(Length::Fixed(width)).into())
            .collect::<Vec<_>>(),
    )
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn column<'a>(children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    widget::column::with_children(children).spacing(6).into()
}

fn caption(text: &'static str) -> Element<'static, Message> {
    widget::text::caption(text).into()
}

/// A labelled length box: "All [0.00] in."
fn length_row(
    state: &StyleProperties,
    text: &'static str,
    field: Field,
) -> Element<'static, Message> {
    row(vec![
        widget::text::body(text).width(Length::Fixed(88.0)).into(),
        input(state, field, 72.0),
        caption("in."),
    ])
}

fn dropdown<T: AsRef<str> + Send + Sync + Clone + 'static>(
    labels: Vec<T>,
    selected: Option<usize>,
    on_select: impl Fn(usize) -> StyleAction + Send + Sync + 'static,
) -> widget::Dropdown<'static, T, Message, Message> {
    widget::dropdown(labels, selected, move |index| send(on_select(index)))
}

fn font<'a>(state: &StyleProperties, colours: Context<'a>) -> Vec<Element<'a, Message>> {
    let draft = &state.draft;
    let family = state
        .families
        .iter()
        .position(|family| *family == draft.font_family);
    let size = SIZES
        .iter()
        .position(|size| size.parse::<f32>().ok() == Some(draft.font_size_points));
    vec![
        row(vec![
            widget::text::body("Face").width(Length::Fixed(36.0)).into(),
            dropdown(state.families.clone(), family, StyleAction::Face)
                .width(Length::Fill)
                .into(),
        ]),
        row(vec![
            widget::text::body("Size").width(Length::Fixed(36.0)).into(),
            dropdown(SIZES.to_vec(), size, StyleAction::SizePreset)
                .width(Length::Fixed(80.0))
                .into(),
            input(state, Field::Size, 56.0),
            caption("pt"),
        ]),
        caption("Colour"),
        colours.control(Target::StyleText, draft.attributes.colour),
        check(draft.bold, "Bold", |on| {
            StyleAction::Attribute(FontAttribute::Bold, on)
        }),
        check(draft.italic, "Italic", |on| {
            StyleAction::Attribute(FontAttribute::Italic, on)
        }),
        check(draft.attributes.underline, "Underline", |on| {
            StyleAction::Attribute(FontAttribute::Underline, on)
        }),
        check(draft.attributes.word_underline, "Word underline", |on| {
            StyleAction::Attribute(FontAttribute::WordUnderline, on)
        }),
        check(
            draft.attributes.double_underline,
            "Double underline",
            |on| StyleAction::Attribute(FontAttribute::DoubleUnderline, on),
        ),
        row(vec![
            widget::text::body("Caps").width(Length::Fixed(88.0)).into(),
            dropdown(
                CAPS.iter().map(|(_, name)| *name).collect(),
                CAPS.iter()
                    .position(|(caps, _)| *caps == draft.attributes.caps),
                StyleAction::Caps,
            )
            .width(Length::Fill)
            .into(),
        ]),
        check(draft.first_line_bold, "1st line bold", |on| {
            StyleAction::Attribute(FontAttribute::FirstLineBold, on)
        }),
    ]
}

fn alignment(state: &StyleProperties) -> Vec<Element<'static, Message>> {
    let draft = &state.draft;
    let align = |text: &'static str, alignment: Alignment| -> Element<'static, Message> {
        widget::radio(text, alignment, Some(draft.alignment), |alignment| {
            send(StyleAction::Align(alignment))
        })
        .into()
    };
    vec![
        choices(
            vec![
                align("Left", Alignment::Left),
                align("Center", Alignment::Centre),
            ],
            96.0,
        ),
        choices(
            vec![
                align("Right", Alignment::Right),
                align("Justify", Alignment::Justify),
            ],
            96.0,
        ),
        length_row(state, "All", Field::All),
        length_row(state, "First", Field::First),
        length_row(state, "Rest", Field::Rest),
        length_row(state, "From right", Field::Right),
        check(draft.both_sides, "Both sides", StyleAction::BothSides),
        check(draft.hanging, "Hanging indent", StyleAction::Hanging),
    ]
}

fn tabs(state: &StyleProperties) -> Vec<Element<'static, Message>> {
    let draft = &state.draft;
    let own = draft.breaks.0 & Breaks::OWN_TABS != 0;
    let mut children = vec![check(own, "Use style tabs", StyleAction::OwnTabs)];
    if own {
        for (index, stop) in draft.tabs.iter().enumerate() {
            children.push(row(vec![
                dropdown(
                    TAB_KINDS.iter().map(|(_, name)| *name).collect(),
                    TAB_KINDS.iter().position(|(kind, _)| *kind == stop.kind),
                    move |kind| StyleAction::TabKind(index, kind),
                )
                .width(Length::Fixed(96.0))
                .into(),
                input(state, Field::Tab(index), 60.0),
                caption("in."),
                widget::button::icon(widget::icon::from_name("list-remove-symbolic"))
                    .extra_small()
                    .name("Remove this tab")
                    .tooltip("Remove this tab")
                    .on_press(send(StyleAction::RemoveTab(index)))
                    .into(),
            ]));
            children.push(
                dropdown(
                    LEADERS.iter().map(|(_, name)| *name).collect(),
                    LEADERS
                        .iter()
                        .position(|(leader, _)| *leader == stop.leader),
                    move |leader| StyleAction::TabLeader(index, leader),
                )
                .width(Length::Fixed(160.0))
                .into(),
            );
        }
        children.push(
            widget::button::standard("Add tab")
                .on_press_maybe((draft.tabs.len() < 22).then_some(send(StyleAction::AddTab)))
                .into(),
        );
    }
    children
}

fn spacing(state: &StyleProperties) -> Vec<Element<'static, Message>> {
    let draft = &state.draft;
    let line = match draft.spacing.line {
        LineSpacing::Single => 0,
        LineSpacing::OneAndAHalf(_) => 1,
        LineSpacing::Double(_) => 2,
        LineSpacing::Custom(_) => 3,
    };
    let choice = |text: &'static str, index: usize| -> Element<'static, Message> {
        widget::radio(text, index, Some(line), |index| {
            send(StyleAction::LineSpacing(index))
        })
        .into()
    };
    let tight = |text: &'static str, percent: u16| -> Element<'static, Message> {
        widget::radio(text, percent, Some(draft.spacing.tightness), |percent| {
            send(StyleAction::Tightness(percent))
        })
        .into()
    };
    let add_in = draft.spacing.add_at_break;
    let mut custom = vec![
        widget::container(choice("Custom", 3))
            .width(Length::Fixed(88.0))
            .into(),
    ];
    if line == 3 {
        custom.push(input(state, Field::Custom, 72.0));
        custom.push(caption("in."));
    }
    vec![
        choices(
            vec![choice("Single", 0), choice("1 1/2", 1), choice("Double", 2)],
            80.0,
        ),
        row(custom),
        length_row(state, "Above", Field::Above),
        length_row(state, "Below", Field::Below),
        caption("Add in"),
        widget::radio("Always", true, Some(add_in), |always| {
            send(StyleAction::AddInAlways(always))
        })
        .into(),
        widget::radio("When not at break", false, Some(add_in), |always| {
            send(StyleAction::AddInAlways(always))
        })
        .into(),
        caption("Text tightness"),
        choices(
            vec![tight("90%", 90), tight("100%", 100), tight("115%", 115)],
            80.0,
        ),
    ]
}

fn breaks(
    state: &StyleProperties,
    styles: &BTreeMap<StyleId, ParagraphStyle>,
) -> Vec<Element<'static, Message>> {
    let draft = &state.draft;
    let bits = draft.breaks.0;
    let mut children: Vec<Element<'static, Message>> = BREAK_BOXES
        .iter()
        .map(|(bit, text)| {
            let bit = *bit;
            check(bits & bit != 0, text, move |on| StyleAction::Break(bit, on))
        })
        .collect();
    let listed = listed_styles(styles);
    let next = draft.next.unwrap_or(draft.id);
    children.push(check(
        bits & Breaks::NEXT_STYLE != 0,
        "Next style",
        StyleAction::NextStyle,
    ));
    children.push(
        dropdown(
            listed.iter().map(|(_, name)| name.clone()).collect(),
            listed.iter().position(|(id, _)| *id == next),
            StyleAction::NextStyleChoice,
        )
        .width(Length::Fill)
        .into(),
    );
    let within = bits & Breaks::WITHIN != 0;
    let keep = |bit: u16, text: &'static str| -> Element<'static, Message> {
        let mut checkbox = widget::checkbox(bits & bit != 0).label(text);
        if !within {
            checkbox = checkbox.on_toggle(move |on| send(StyleAction::Break(bit, on)));
        }
        checkbox.into()
    };
    children.push(caption("Keep with (when breaks within are off)"));
    children.push(keep(Breaks::KEEP_WITH_PREVIOUS, "Previous paragraph"));
    children.push(keep(Breaks::KEEP_WITH_NEXT, "Next paragraph"));
    children
}

fn bullets(state: &StyleProperties) -> Vec<Element<'static, Message>> {
    let draft = &state.draft;
    let Some(parts) = &state.bullets else {
        return vec![
            caption(
                "This style's text before each paragraph is not in the order this panel writes; it is kept as it is.",
            ),
            check(false, "Start over with no bullet or number", |on| {
                StyleAction::Bullet(!on)
            }),
        ];
    };
    let number = parts.number.as_ref();
    let mut children = vec![check(number.is_some(), "Number", StyleAction::Number)];
    if number.is_some() {
        children.push(caption("Text before, number, text after"));
        children.push(row(vec![
            input(state, Field::NumberBefore, 40.0),
            dropdown(
                NUMBERS.iter().map(|(_, sample)| *sample).collect(),
                number.and_then(|(_, code, _)| NUMBERS.iter().position(|(c, _)| c == code)),
                StyleAction::NumberChoice,
            )
            .width(Length::Fixed(72.0))
            .into(),
            input(state, Field::NumberAfter, 40.0),
        ]));
    }
    children.push(check(
        parts.text.is_some(),
        "Text",
        StyleAction::BulletTextOn,
    ));
    if parts.text.is_some() {
        children.push(input(state, Field::BulletText, 200.0));
    }
    children.push(check(parts.bullet.is_some(), "Bullet", StyleAction::Bullet));
    if let Some(bullet) = parts.bullet {
        children.push(
            dropdown(
                BULLETS.iter().map(|(_, name)| *name).collect(),
                BULLETS.iter().position(|(code, _)| *code == bullet),
                StyleAction::BulletChoice,
            )
            .width(Length::Fill)
            .into(),
        );
    }
    children.push(check(
        draft.bullets.flags & 16 != 0,
        "Right align",
        StyleAction::RightAlign,
    ));
    // Outline levels (`UI_SPEC.md` §22): Ami Pro's Outline Styles per style.
    children.push(caption("Outline level"));
    children.push(
        dropdown(
            vec!["None", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
            Some(usize::from(draft.bullets.level.min(9))),
            StyleAction::OutlineLevel,
        )
        .width(Length::Fill)
        .into(),
    );
    if draft.bullets.level > 0 {
        children.push(caption("Reset options"));
        children.push(check(
            draft.bullets.flags & 2 != 0,
            "After higher level",
            StyleAction::ResetHigher,
        ));
        children.push(check(
            draft.bullets.flags & 4 != 0,
            "After intervening style",
            StyleAction::ResetIntervening,
        ));
        children.push(check(
            draft.bullets.flags & 8 != 0,
            "Cumulative numbering (1.1.1)",
            StyleAction::Cumulative,
        ));
    }
    children.push(caption("Attributes"));
    for (flag, text) in BULLET_ATTRIBUTES {
        children.push(check(
            draft.bullets.attributes & flag != 0,
            text,
            move |on| StyleAction::BulletAttribute(flag, on),
        ));
    }
    // Ami Pro's Space for is the first-line indent, Alignment's First
    // (decompile: both boxes write the same field; VM fixture BL3).
    children.push(length_row(state, "Space for", Field::First));
    children.push(caption(
        "Room for the bullet or number: the first line's indent, as First in Alignment.",
    ));
    children
}

fn lines<'a>(state: &StyleProperties, colours: Context<'a>) -> Vec<Element<'a, Message>> {
    let rules = state.draft.rules;
    let side = |above: bool, text: &'static str| -> Vec<Element<'static, Message>> {
        let rule = if above { rules.above } else { rules.below };
        let field = if above {
            Field::RuleAbove
        } else {
            Field::RuleBelow
        };
        vec![
            check(rule.on, text, move |on| StyleAction::Rule(above, on)),
            dropdown(
                RULE_STYLES.to_vec(),
                usize::from(rule.line.clamp(1, 12)).checked_sub(1),
                move |index| StyleAction::RuleStyle(above, index),
            )
            .width(Length::Fill)
            .into(),
            length_row(state, "Spacing", field),
        ]
    };
    let length = match rules.length {
        RuleLength::Text => 0,
        RuleLength::Margins => 1,
        RuleLength::Other => 2,
    };
    let choice = |text: &'static str, index: usize| -> Element<'static, Message> {
        widget::radio(text, index, Some(length), |index| {
            send(StyleAction::RuleLength(index))
        })
        .into()
    };
    let mut children: Vec<Element<'a, Message>> = side(true, "Line above");
    children.extend(side(false, "Line below"));
    children.push(caption("Length"));
    children.push(choices(
        vec![choice("Text", 0), choice("Margins", 1), choice("Other", 2)],
        84.0,
    ));
    if length == 2 {
        children.push(length_row(state, "Other length", Field::Other));
    }
    children.push(caption("Colour"));
    children.push(colours.control(Target::StyleRule, rules.colour));
    children
}

/// Table format (TN6, `UI_SPEC.md` §41): Cell format, decimal places and
/// symbols, thousands, the negative forms, and the currency symbol, with a
/// sample of the format on a number.
fn table_format(state: &StyleProperties) -> Vec<Element<'static, Message>> {
    let format = &state.draft.kept.number_format;
    let kind = CELL_FORMATS.iter().position(|kind| *kind == format.kind);
    let sample = |value: &str| {
        format
            .display(value)
            .map_or_else(|| value.to_owned(), |shown| shown.text)
    };
    vec![
        row(vec![
            widget::text::body("Cell format")
                .width(Length::Fixed(88.0))
                .into(),
            dropdown(
                vec!["General", "Fixed Decimal", "Currency", "Percent"],
                kind,
                StyleAction::CellFormat,
            )
            .width(Length::Fill)
            .into(),
        ]),
        row(vec![
            widget::text::body("Decimal places")
                .width(Length::Fixed(88.0))
                .into(),
            input(state, Field::Decimals, 48.0),
            caption("symbol"),
            input(state, Field::DecimalSymbol, 36.0),
        ]),
        row(vec![
            check(format.thousands(), "Thousands", StyleAction::Thousands),
            input(state, Field::ThousandsSymbol, 36.0),
        ]),
        caption("Negative"),
        check(
            format.leading_minus(),
            "Leading minus",
            StyleAction::LeadingMinus,
        ),
        check(
            format.trailing_minus(),
            "Trailing minus",
            StyleAction::TrailingMinus,
        ),
        check(
            format.parentheses(),
            "Parentheses",
            StyleAction::Parentheses,
        ),
        check(format.red(), "Red", StyleAction::Red),
        caption("Currency"),
        row(vec![
            widget::text::body("Symbol")
                .width(Length::Fixed(88.0))
                .into(),
            input(state, Field::CurrencySymbol, 48.0),
        ]),
        choices(
            vec![
                widget::radio("Leading", true, Some(format.symbol_leading()), |on| {
                    send(StyleAction::SymbolLeading(on))
                })
                .into(),
                widget::radio("Trailing", false, Some(format.symbol_leading()), |on| {
                    send(StyleAction::SymbolLeading(on))
                })
                .into(),
            ],
            100.0,
        ),
        widget::text::caption(format!(
            "Sample: {}   {}",
            sample("1234.5"),
            sample("-1234.5")
        ))
        .into(),
    ]
}

/// The rail's content: header, the style list, the folding sections,
/// Hyphenation, and Save as New Style.
pub fn panel<'a>(
    state: &StyleProperties,
    styles: &BTreeMap<StyleId, ParagraphStyle>,
    colours: Context<'a>,
) -> Element<'a, Message> {
    let listed = listed_styles(styles);
    let header = widget::row::with_children(vec![
        widget::text::heading("Style Properties").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(StyleAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Style Properties")
            .tooltip(crate::chrome::command_tooltip(
                crate::CommandId::StylesPanel,
            ))
            .on_press(send(StyleAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut body: Vec<Element<'a, Message>> = vec![row(vec![
        widget::text::body("Style").into(),
        dropdown(
            listed.iter().map(|(_, name)| name.clone()).collect(),
            listed.iter().position(|(id, _)| *id == state.style),
            StyleAction::Style,
        )
        .width(Length::Fill)
        .into(),
    ])];
    if let Some(problem) = &state.error {
        body.push(widget::text::caption(problem.clone()).into());
    }
    for (section, title) in Section::ALL {
        let open = state.open.contains(&section);
        body.push(
            widget::button::custom(
                widget::row::with_children(vec![
                    widget::icon::from_name(if open {
                        "pan-down-symbolic"
                    } else {
                        "pan-end-symbolic"
                    })
                    .size(16)
                    .icon()
                    .into(),
                    widget::text::heading(title).into(),
                ])
                .spacing(6)
                .align_y(cosmic::iced::Alignment::Center),
            )
            .class(cosmic::theme::Button::Text)
            .padding([4, 0])
            .width(Length::Fill)
            .on_press(send(StyleAction::Section(section)))
            .into(),
        );
        if open {
            let fields = match section {
                Section::Font => font(state, colours),
                Section::Alignment => alignment(state),
                Section::Tabs => tabs(state),
                Section::Spacing => spacing(state),
                Section::Breaks => breaks(state, styles),
                Section::Bullets => bullets(state),
                Section::Lines => lines(state, colours),
                Section::TableFormat => table_format(state),
            };
            body.push(
                widget::container(column(fields))
                    .padding([0, 0, 8, 22])
                    .into(),
            );
        }
    }
    body.push(check(
        state.draft.hyphenation,
        "Hyphenation",
        StyleAction::Hyphenation,
    ));
    match &state.save_as {
        None => body.push(
            widget::button::standard("Save as New Style…")
                .on_press(send(StyleAction::SaveAs))
                .into(),
        ),
        Some(name) => {
            let problem = amipro_editing::style_name_problem(styles, name, None);
            body.push(caption("New style name"));
            body.push(
                widget::text_input("Name", name.clone())
                    .on_input(|name| send(StyleAction::SaveAsName(name)))
                    .on_submit(|_| send(StyleAction::SaveAsConfirm))
                    .width(Length::Fill)
                    .into(),
            );
            if let Some(problem) = problem.filter(|_| !name.is_empty()) {
                body.push(widget::text::caption(crate::style_dialogs::sentence(problem)).into());
            }
            body.push(row(vec![
                widget::button::suggested("Save")
                    .on_press_maybe(
                        (problem.is_none() && !name.is_empty())
                            .then_some(send(StyleAction::SaveAsConfirm)),
                    )
                    .into(),
                widget::button::standard("Cancel")
                    .on_press(send(StyleAction::SaveAsCancel))
                    .into(),
            ]));
        }
    }
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(6).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::default_styles;

    #[test]
    fn lengths_are_read_in_inches_or_with_a_unit() {
        assert_eq!(parse_length("0.5"), Some(720));
        assert_eq!(parse_length(" 1 in"), Some(1440));
        assert_eq!(parse_length("2.54cm"), Some(1440));
        assert_eq!(parse_length("12pt"), Some(240));
        assert_eq!(parse_length("-1"), None);
        assert_eq!(parse_length("23"), None);
        assert_eq!(parse_length("x"), None);
        assert_eq!(inches(1008), "0.70");
    }

    #[test]
    fn bullet_text_splits_in_ami_pros_order_bn4() {
        let parts = BulletParts::split("\u{1A}Hi\u{10}").unwrap();
        assert_eq!(parts.number, Some((String::new(), '\u{1A}', String::new())));
        assert_eq!(parts.text.as_deref(), Some("Hi"));
        assert_eq!(parts.bullet, Some('\u{10}'));
        assert_eq!(parts.join(), "\u{1A}Hi\u{10}");
        for text in ["\u{1A}.", "(\u{1D})", "Note", "\u{10}", "", "[\u{1A}] "] {
            assert_eq!(BulletParts::split(text).unwrap().join(), text, "{text:?}");
        }
        // A bullet before text is not Ami Pro's order: kept as it is.
        assert_eq!(BulletParts::split("\u{10}Note"), None);
    }

    fn panel_on(style: u64) -> (StyleProperties, BTreeMap<StyleId, ParagraphStyle>) {
        let styles = default_styles();
        let state = StyleProperties::new(
            &styles,
            StyleId(style),
            Some(StyleId(style)),
            vec!["Arial".into()],
        )
        .unwrap();
        (state, styles)
    }

    /// Applies an outcome as the app does.
    fn apply(outcome: Outcome, styles: &mut BTreeMap<StyleId, ParagraphStyle>) {
        if let Outcome::Apply(style) = outcome {
            styles.insert(style.id, *style);
        }
    }

    #[test]
    fn first_line_bold_and_bold_exclude_each_other_and_each_applies_at_once() {
        let (mut state, mut styles) = panel_on(1);
        let outcome = state.update(StyleAction::Attribute(FontAttribute::Bold, true), &styles);
        apply(outcome, &mut styles);
        assert!(state.sync(&styles, Some(StyleId(1))));
        let outcome = state.update(
            StyleAction::Attribute(FontAttribute::FirstLineBold, true),
            &styles,
        );
        assert!(
            matches!(&outcome, Outcome::Apply(style) if style.first_line_bold && !style.bold),
            "checking 1st line bold unchecks Bold"
        );
        apply(outcome, &mut styles);
        assert!(state.sync(&styles, Some(StyleId(1))));
        let outcome = state.update(StyleAction::Attribute(FontAttribute::Bold, true), &styles);
        assert!(
            matches!(&outcome, Outcome::Apply(style) if style.bold && !style.first_line_bold),
            "checking Bold unchecks 1st line bold"
        );
    }

    #[test]
    fn choices_apply_at_once_and_typed_values_on_submit() {
        let (mut state, mut styles) = panel_on(1);
        // A check box applies at once.
        let outcome = state.update(StyleAction::Attribute(FontAttribute::Bold, true), &styles);
        assert!(matches!(&outcome, Outcome::Apply(style) if style.bold));
        apply(outcome, &mut styles);
        assert!(state.sync(&styles, Some(StyleId(1))));
        // Typing waits; Enter (or leaving the box) applies.
        assert_eq!(
            state.update(StyleAction::Text(Field::All, "0.5".into()), &styles),
            Outcome::Stay
        );
        assert_eq!(state.text(Field::All), "0.5");
        let outcome = state.update(StyleAction::Submit(Field::All), &styles);
        assert!(matches!(&outcome, Outcome::Apply(style) if style.indents.all == 720));
        apply(outcome, &mut styles);
        assert_eq!(state.text(Field::All), "0.50");
        // A value that does not read keeps its box and says why.
        state.update(StyleAction::Text(Field::First, "abc".into()), &styles);
        assert_eq!(
            state.update(StyleAction::Submit(Field::First), &styles),
            Outcome::Stay
        );
        assert!(state.error.is_some());
        assert_eq!(state.text(Field::First), "abc");
        // Leaving a box unchanged applies nothing.
        assert_eq!(
            state.update(StyleAction::Submit(Field::Rest), &styles),
            Outcome::Stay
        );
    }

    #[test]
    fn the_panel_follows_the_caret_unless_a_style_was_picked() {
        let (mut state, styles) = panel_on(1);
        // The caret moves to a heading: the panel follows.
        state.sync(&styles, Some(StyleId(2)));
        assert_eq!(state.style, StyleId(2));
        // A style picked from the list stays while the caret stays in
        // paragraphs of the style it was in.
        let listed = listed_styles(&styles);
        let third = listed.iter().position(|(id, _)| *id == StyleId(3)).unwrap();
        state.update(StyleAction::Style(third), &styles);
        state.sync(&styles, Some(StyleId(2)));
        assert_eq!(state.style, StyleId(3));
        // ... until the caret reaches another style's paragraph.
        state.sync(&styles, Some(StyleId(1)));
        assert_eq!(state.style, StyleId(1));
        // An undo elsewhere is taken up: the draft is the document's.
        let mut changed = styles.clone();
        changed.get_mut(&StyleId(1)).unwrap().font_size_points = 30.0;
        state.sync(&changed, Some(StyleId(1)));
        assert_eq!(state.draft.font_size_points, 30.0);
    }

    #[test]
    fn outline_level_reset_options_and_cumulative_numbers_set_the_style_ol2() {
        // A level-1 style numbering "I." first, then style 1 at level 2
        // with Number "1." made cumulative: its text becomes "I.1." (Ami
        // Pro takes the previous level's number), and back.
        let (mut state, mut styles) = panel_on(1);
        let mut top = styles[&StyleId(1)].clone();
        top.id = StyleId(90);
        top.name = "Top".into();
        top.bullets = amipro_document::Bullets {
            text: "\u{1B}.".into(),
            level: 1,
            flags: 2,
            ..amipro_document::Bullets::default()
        };
        styles.insert(top.id, top);
        let run = |state: &mut StyleProperties,
                   styles: &mut BTreeMap<StyleId, ParagraphStyle>,
                   action: StyleAction| {
            let outcome = state.update(action, styles);
            apply(outcome, styles);
            state.sync(styles, Some(StyleId(1)));
        };
        run(&mut state, &mut styles, StyleAction::Number(true));
        run(
            &mut state,
            &mut styles,
            StyleAction::Text(Field::NumberAfter, ".".into()),
        );
        run(
            &mut state,
            &mut styles,
            StyleAction::Submit(Field::NumberAfter),
        );
        run(&mut state, &mut styles, StyleAction::OutlineLevel(2));
        run(&mut state, &mut styles, StyleAction::ResetHigher(true));
        assert_eq!(styles[&StyleId(1)].bullets.level, 2);
        assert_eq!(styles[&StyleId(1)].bullets.flags & 6, 2);
        // The other reset option clears this one.
        run(&mut state, &mut styles, StyleAction::ResetIntervening(true));
        assert_eq!(styles[&StyleId(1)].bullets.flags & 6, 4);
        run(&mut state, &mut styles, StyleAction::ResetHigher(true));
        run(&mut state, &mut styles, StyleAction::Cumulative(true));
        let style = &styles[&StyleId(1)];
        assert_eq!(style.bullets.flags & 8, 8);
        assert_eq!(style.bullets.text, "\u{1B}.\u{1A}.");
        // The panel reads the cumulative text back as its parts.
        let parts = BulletParts::split(&style.bullets.text).unwrap();
        assert_eq!(parts.prefix, "\u{1B}.");
        assert_eq!(parts.number, Some((String::new(), '\u{1A}', ".".into())));
        run(&mut state, &mut styles, StyleAction::Cumulative(false));
        assert_eq!(styles[&StyleId(1)].bullets.text, "\u{1A}.");
        assert_eq!(styles[&StyleId(1)].bullets.flags & 8, 0);
    }

    #[test]
    fn sections_set_the_style_fields() {
        let (mut state, mut styles) = panel_on(1);
        for action in [
            StyleAction::LineSpacing(3),
            StyleAction::Text(Field::Custom, "0.25".into()),
            StyleAction::Submit(Field::Custom),
            StyleAction::Text(Field::Below, "6pt".into()),
            StyleAction::Submit(Field::Below),
            StyleAction::AddInAlways(false),
            StyleAction::Tightness(90),
            StyleAction::Break(Breaks::WITHIN, false),
            StyleAction::Break(Breaks::KEEP_WITH_NEXT, true),
            StyleAction::Number(true),
            StyleAction::BulletTextOn(true),
            StyleAction::Text(Field::BulletText, "Hi".into()),
            StyleAction::Submit(Field::BulletText),
            StyleAction::Bullet(true),
            StyleAction::Rule(false, true),
            StyleAction::RuleStyle(false, 2),
            StyleAction::Text(Field::RuleBelow, "0.1".into()),
            StyleAction::Submit(Field::RuleBelow),
            StyleAction::AddTab,
            StyleAction::TabKind(0, 2),
            StyleAction::Hyphenation(true),
        ] {
            let outcome = state.update(action, &styles);
            apply(outcome, &mut styles);
            state.sync(&styles, Some(StyleId(1)));
        }
        let style = &styles[&StyleId(1)];
        assert_eq!(style.spacing.line, LineSpacing::Custom(360));
        assert_eq!(style.spacing.below, 120);
        assert!(!style.spacing.add_at_break);
        assert_eq!(style.spacing.tightness, 90);
        assert!(style.breaks.keep_with_next());
        assert_eq!(style.bullets.text, "\u{1A}.Hi\u{10}");
        assert!(style.rules.below.on);
        assert_eq!(style.rules.below.line, 3);
        assert_eq!(style.rules.below.space, 144 + 40);
        assert_eq!(style.tabs, [TabStop::new(TabKind::Right, 720)]);
        assert!(style.breaks.0 & Breaks::OWN_TABS != 0);
        assert!(style.hyphenation);
        // Allowing breaks within clears Keep With.
        apply(
            state.update(StyleAction::Break(Breaks::WITHIN, true), &styles),
            &mut styles,
        );
        assert!(!styles[&StyleId(1)].breaks.keep_with_next());
    }

    #[test]
    fn save_as_new_style_asks_for_the_style() {
        let (mut state, styles) = panel_on(1);
        state.update(StyleAction::SaveAs, &styles);
        state.update(StyleAction::SaveAsName("Big".into()), &styles);
        let Outcome::SaveAs { name, based_on, .. } =
            state.update(StyleAction::SaveAsConfirm, &styles)
        else {
            panic!("Save as New Style asks for the style");
        };
        assert_eq!((name.as_str(), based_on), ("Big", StyleId(1)));
        assert_eq!(state.update(StyleAction::Back, &styles), Outcome::Back);
    }
}
