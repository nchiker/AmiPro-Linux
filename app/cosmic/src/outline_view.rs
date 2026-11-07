//! View › Outline Mode (`UI_SPEC.md` §40, OM1; run B, OB.3;
//! `docs/research/OUTLINE_MODE_NOTES.md`).
//!
//! Outline mode shows the main text by outline level, as Ami Pro's: every
//! paragraph left aligned and without its style's space above and below, a
//! heading at level L one step (480 twips) further in per level, body text
//! half a step right of the heading above it, and an icon left of each
//! paragraph's first line: a plus on a heading with text under it, a minus
//! on one with nothing under it, a small box on body text (a bold plus, a
//! contracted heading, comes with OB.4). Like Draft it is a display copy laid
//! out at the window's width: the text, and so every position, is the
//! document's own, and nothing about the view is saved. The copy keeps the
//! document's pages (RB2: Ami Pro shows one continuous column; a page
//! break would make one page per break here).

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::sync::Arc;

use amipro_document::{Alignment, Breaks, Document, Indents, LineSpacing, ParagraphId, Twips};
use amipro_editing::sections;

/// One outline level's step (`1108:0f06`: 480 logical units).
pub const STEP: Twips = 480;
/// The page's left margin in Outline mode: room for the icons.
pub const MARGIN: Twips = 720;

/// The icon Ami Pro draws left of a paragraph's first line (`1108:12f7`,
/// `1108:13be`; VM OM1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutlineIcon {
    /// A heading with text under it that shows.
    Plus,
    /// A heading whose text under it is hidden (contracted; OB.4).
    Contracted,
    /// A heading with nothing under it.
    Minus,
    /// Body text.
    Body,
}

/// How far in paragraph `index`'s text starts in Outline mode: a heading at
/// level L at (L − 1) steps, body text half a step right of the heading
/// above it (none above: at the margin).
pub fn indent(levels: &[u8], index: usize) -> Twips {
    let level = levels[index];
    if level > 0 {
        return Twips::from(level - 1) * STEP;
    }
    match levels[..index].iter().rev().find(|&&above| above > 0) {
        Some(&above) => Twips::from(above - 1) * STEP + STEP / 2,
        None => 0,
    }
}

/// What Outline mode shows (OB.4; `1608:154c`): Show levels n (1–9, or
/// all), and Expand and Contract's marks on single paragraphs, which
/// outrank it. Per tab and never written (OM2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutlineState {
    /// Show levels: `None` is All (Ami Pro's 10).
    pub levels: Option<u8>,
    /// Shown whatever the levels (Expand; Ami Pro's flag 0x10).
    pub shown: BTreeSet<ParagraphId>,
    /// Hidden (Contract; Ami Pro's flag 0x20).
    pub hidden: BTreeSet<ParagraphId>,
}

impl OutlineState {
    /// Whether each main paragraph is hidden: by a Contract, or, unless an
    /// Expand shows it, by Show levels (deeper headings, and body text under
    /// any level but All).
    pub fn hidden_flags(&self, document: &Document) -> Vec<bool> {
        let main = document.main_flow_id();
        let levels = sections::levels(document, main);
        let Some(flow) = document.flow(main) else {
            return Vec::new();
        };
        flow.paragraphs
            .iter()
            .zip(&levels)
            .map(|(paragraph, &level)| {
                if self.hidden.contains(&paragraph.id) {
                    return true;
                }
                if self.shown.contains(&paragraph.id) {
                    return false;
                }
                match self.levels {
                    None => false,
                    Some(shown) => level == 0 || level > shown,
                }
            })
            .collect()
    }

    /// Show levels `levels` (`None` All): every Expand and Contract mark goes
    /// (`1608:1773`).
    pub fn show_levels(&mut self, levels: Option<u8>) {
        self.levels = levels;
        self.shown.clear();
        self.hidden.clear();
    }

    /// Contract on the heading at `index`: its whole section is hidden
    /// (`1608:0111`); body text has no section. Returns whether anything
    /// changed.
    pub fn contract(&mut self, document: &Document, index: usize) -> bool {
        self.mark(document, index, false)
    }

    /// Expand on the heading at `index`: its whole section shows, down to
    /// body text whatever the levels (`1608:00c4`).
    pub fn expand(&mut self, document: &Document, index: usize) -> bool {
        self.mark(document, index, true)
    }

    fn mark(&mut self, document: &Document, index: usize, show: bool) -> bool {
        let main = document.main_flow_id();
        let levels = sections::levels(document, main);
        if levels.get(index).is_none_or(|&level| level == 0) {
            return false;
        }
        let before = self.hidden_flags(document);
        let Some(flow) = document.flow(main) else {
            return false;
        };
        let section = sections::section(&levels, index);
        for paragraph in &flow.paragraphs[section.start + 1..section.end] {
            if show {
                self.hidden.remove(&paragraph.id);
                self.shown.insert(paragraph.id);
            } else {
                self.shown.remove(&paragraph.id);
                self.hidden.insert(paragraph.id);
            }
        }
        self.hidden_flags(document) != before
    }

    /// Forgets marks of paragraphs no longer in the document.
    pub fn retain(&mut self, document: &Document) {
        let Some(flow) = document.flow(document.main_flow_id()) else {
            return;
        };
        let ids: BTreeSet<ParagraphId> = flow.paragraphs.iter().map(|p| p.id).collect();
        self.shown.retain(|id| ids.contains(id));
        self.hidden.retain(|id| ids.contains(id));
    }
}

/// Each main-flow paragraph's icon, with `hidden` from
/// [`OutlineState::hidden_flags`]: a heading whose next paragraph is hidden
/// shows contracted.
pub fn icons(document: &Document, hidden: &[bool]) -> Vec<OutlineIcon> {
    let levels = sections::levels(document, document.main_flow_id());
    (0..levels.len())
        .map(|index| {
            if levels[index] == 0 {
                OutlineIcon::Body
            } else if sections::section(&levels, index).len() > 1 {
                if hidden.get(index + 1).copied().unwrap_or(false) {
                    OutlineIcon::Contracted
                } else {
                    OutlineIcon::Plus
                }
            } else {
                OutlineIcon::Minus
            }
        })
        .collect()
}

/// The nearest shown paragraph to `index`, looking forward first when
/// `forward` (the caret leaves hidden text the way it was going).
pub fn shown_near(hidden: &[bool], index: usize, forward: bool) -> Option<usize> {
    if !hidden.get(index).copied().unwrap_or(false) {
        return Some(index);
    }
    let after = (index + 1..hidden.len()).find(|&next| !hidden[next]);
    let before = (0..index).rev().find(|&previous| !hidden[previous]);
    if forward {
        after.or(before)
    } else {
        before.or(after)
    }
}

/// `_OUTLINE.STY`'s font for each level, as shipped (read; `1098:0333`):
/// face, size in points, bold. Level 0 is body text.
pub fn outline_style_font(level: u8) -> (&'static str, f32, bool) {
    match level {
        0 => ("Arial", 12.0, false),
        1 => ("Times New Roman", 18.0, true),
        2 => ("Times New Roman", 14.0, false),
        3 => ("Times New Roman", 12.0, false),
        _ => ("Arial", 12.0, false),
    }
}

/// `document` as Outline mode lays it out at `width` twips: Draft's
/// window-wide pages with room for the icons, every style left aligned with
/// no space above or below and no breaks before or after, and each main
/// paragraph indented by its level.
pub fn outline_document(
    document: &Document,
    width: Twips,
    hidden: &[bool],
    outline_styles: bool,
) -> Document {
    let mut copy = match crate::view_modes::window_pages(document, width, MARGIN) {
        Cow::Borrowed(document) => document.clone(),
        Cow::Owned(document) => document,
    };
    for style in copy.styles_mut().values_mut() {
        style.alignment = Alignment::Left;
        style.spacing.above = 0;
        style.spacing.below = 0;
        style.indents = Indents::default();
        style.breaks = Breaks(style.breaks.0 & !(Breaks::PAGE_BEFORE | Breaks::PAGE_AFTER));
        // Use Outline Styles: the level's outline style's font, for display
        // only (the paragraph keeps its style; OB.5).
        if outline_styles {
            let (face, size, bold) = outline_style_font(style.bullets.level);
            style.font_family = face.into();
            style.font_size_points = size;
            style.bold = bold;
            style.italic = false;
        }
    }
    // Ami Pro paints Outline mode through its Draft path: no line numbers.
    if copy.line_numbering().is_on() {
        let mut numbering = copy.line_numbering().clone();
        numbering.flags &= !(amipro_document::LineNumbering::ALL_LINES
            | amipro_document::LineNumbering::TEXT_LINES);
        copy.set_line_numbering(numbering);
    }
    let main = copy.main_flow_id();
    let levels = sections::levels(&copy, main);
    if let Some(flow) = copy.flow_mut(main) {
        for (index, paragraph) in flow.paragraphs.iter_mut().enumerate() {
            let all = indent(&levels, index);
            let paragraph = Arc::make_mut(paragraph);
            // A hidden paragraph keeps its place (so every position stays
            // the document's) but shows as an empty line one twip high.
            if hidden.get(index).copied().unwrap_or(false) {
                paragraph.text.clear();
                paragraph.runs.clear();
                paragraph.objects.clear();
                paragraph.marks.clear();
                paragraph.format.line_spacing = Some(LineSpacing::Custom(1));
            }
            paragraph.format.indents = Some(Indents {
                all,
                first: 0,
                rest: 0,
                right: 0,
            });
            paragraph.format.alignment = None;
        }
    }
    copy
}

impl crate::AmiProApp {
    /// The Outline bar, in the toolbar's place in Outline mode (Ami Pro's
    /// Outline SmartIcons; VM OM1): Show levels 1–9 and All, Promote,
    /// Demote, Move Up, Move Down, Expand, Contract, and Layout Mode.
    pub(crate) fn outline_bar_view(&self) -> cosmic::Element<'_, crate::Message> {
        use cosmic::iced::Length;
        use cosmic::widget;

        use crate::{CommandId, Message};
        let levels = self.outline_state.levels;
        let level =
            |label: &'static str, id: CommandId, on: bool| -> cosmic::Element<'_, Message> {
                widget::tooltip(
                    widget::button::custom(widget::text::body(label).center())
                        .class(crate::toggle_button_class(on))
                        .selected(on)
                        .width(32)
                        .height(32)
                        .on_press(Message::Command(id)),
                    widget::text::body(crate::command::metadata(id).label),
                    widget::tooltip::Position::Bottom,
                )
                .into()
            };
        let command = |id: CommandId, icon: &'static str| -> cosmic::Element<'_, Message> {
            widget::button::icon(widget::icon::from_name(icon))
                .name(crate::command::metadata(id).label)
                .tooltip(crate::command::metadata(id).label)
                .width(32)
                .height(32)
                .on_press_maybe(self.command_enabled(id).then_some(Message::Command(id)))
                .into()
        };
        let divider = || -> cosmic::Element<'_, Message> {
            widget::divider::vertical::default()
                .height(24)
                .width(crate::chrome::DIVIDER)
                .into()
        };
        const LEVELS: [(&str, CommandId); 9] = [
            ("1", CommandId::ShowLevel1),
            ("2", CommandId::ShowLevel2),
            ("3", CommandId::ShowLevel3),
            ("4", CommandId::ShowLevel4),
            ("5", CommandId::ShowLevel5),
            ("6", CommandId::ShowLevel6),
            ("7", CommandId::ShowLevel7),
            ("8", CommandId::ShowLevel8),
            ("9", CommandId::ShowLevel9),
        ];
        let mut items: Vec<cosmic::Element<'_, Message>> = LEVELS
            .iter()
            .zip(1u8..)
            .map(|((label, id), n)| level(label, *id, levels == Some(n)))
            .collect();
        items.push(level("All", CommandId::ShowAllLevels, levels.is_none()));
        items.push(divider());
        items.push(command(CommandId::Promote, "go-previous-symbolic"));
        items.push(command(CommandId::Demote, "go-next-symbolic"));
        items.push(command(CommandId::MoveUp, "go-up-symbolic"));
        items.push(command(CommandId::MoveDown, "go-down-symbolic"));
        items.push(divider());
        items.push(command(CommandId::OutlineExpand, "list-add-symbolic"));
        items.push(command(CommandId::OutlineContract, "list-remove-symbolic"));
        items.push(widget::space().width(Length::Fill).into());
        items.push(
            widget::button::standard("Layout Mode")
                .on_press(Message::Command(CommandId::OutlineMode))
                .into(),
        );
        widget::container(
            widget::row::with_children(items)
                .spacing(crate::chrome::ICON_GAP)
                .align_y(cosmic::iced::Alignment::Center),
        )
        .padding([4.0, crate::chrome::BAR_PADDING])
        .height(crate::ACTION_BAR_HEIGHT)
        .width(Length::Fill)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use amipro_document::StyleId;

    use super::*;

    fn outline() -> Document {
        let mut document = Document::from_plain_text("Title\nSub\nBody\nSub two\nTitle two");
        let body = document.body_text_style();
        for (id, level) in [(StyleId(901), 1), (StyleId(902), 2)] {
            let mut style = document.styles()[&body].clone();
            style.id = id;
            style.name = format!("Level {level}");
            style.bullets.level = level;
            style.alignment = Alignment::Centre;
            style.spacing.above = 288;
            document.styles_mut().insert(id, style);
        }
        for (index, style) in [(0, 901), (1, 902), (3, 902), (4, 901)] {
            document.paragraph_mut(index).style = StyleId(style);
        }
        document
    }

    #[test]
    fn levels_indent_and_icons_follow_ami_pros_outline() {
        let document = outline();
        let levels = sections::levels(&document, document.main_flow_id());
        assert_eq!(levels, [1, 2, 0, 2, 1]);
        let indents: Vec<_> = (0..levels.len())
            .map(|index| indent(&levels, index))
            .collect();
        assert_eq!(indents, [0, 480, 720, 480, 0]);
        assert_eq!(indent(&[0, 1], 0), 0, "body text before any heading");
        use OutlineIcon::*;
        assert_eq!(
            icons(&document, &[false; 5]),
            [Plus, Plus, Body, Minus, Minus]
        );
    }

    #[test]
    fn the_copy_keeps_the_text_and_lays_paragraphs_out_by_level() {
        let document = outline();
        let copy = outline_document(&document, 9_000, &[false; 5], false);
        assert_eq!(copy.plain_text(), document.plain_text());
        assert_eq!(copy.document_layout().right.width, 9_000);
        assert_eq!(copy.document_layout().right.layout.margins.left, MARGIN);
        let style = &copy.styles()[&StyleId(901)];
        assert_eq!(style.alignment, Alignment::Left);
        assert_eq!(style.spacing.above, 0);
        assert_eq!(copy.paragraphs()[2].format.indents.unwrap().all, 720);
        // The document itself is untouched.
        assert_eq!(
            document.styles()[&StyleId(901)].alignment,
            Alignment::Centre
        );
        assert!(document.paragraphs()[2].format.indents.is_none());
    }

    #[test]
    fn contract_expand_and_show_levels_hide_as_ami_pro_does() {
        // Title, Sub, Body, Sub two, Title two.
        let document = outline();
        let mut state = OutlineState::default();
        assert_eq!(state.hidden_flags(&document), [false; 5]);
        assert!(state.contract(&document, 0), "the first title's section");
        assert_eq!(
            state.hidden_flags(&document),
            [false, true, true, true, false]
        );
        use OutlineIcon::*;
        assert_eq!(
            icons(&document, &state.hidden_flags(&document)),
            // The hidden Sub's own icon is never drawn.
            [Contracted, Contracted, Body, Minus, Minus]
        );
        assert!(!state.contract(&document, 2), "body text has no section");
        assert!(state.expand(&document, 0));
        assert_eq!(state.hidden_flags(&document), [false; 5]);
        // Show 1: Titles only; Expand shows a section down to body text.
        state.show_levels(Some(1));
        assert_eq!(
            state.hidden_flags(&document),
            [false, true, true, true, false]
        );
        state.expand(&document, 0);
        assert_eq!(state.hidden_flags(&document), [false; 5]);
        // Show 2: headings, no body text; a new choice clears the marks.
        state.show_levels(Some(2));
        assert_eq!(
            state.hidden_flags(&document),
            [false, false, true, false, false]
        );
        state.show_levels(None);
        assert!(state.shown.is_empty() && state.hidden.is_empty());
        assert_eq!(shown_near(&[false, true, true, false], 1, true), Some(3));
        assert_eq!(shown_near(&[false, true, true, false], 2, false), Some(0));
        assert_eq!(shown_near(&[false, true], 0, true), Some(0));
    }

    #[test]
    fn hidden_paragraphs_keep_their_places_as_empty_lines() {
        let document = outline();
        let copy = outline_document(&document, 9_000, &[false, true, true, false, false], false);
        assert_eq!(copy.paragraphs().len(), document.paragraphs().len());
        assert_eq!(copy.paragraphs()[1].text, "");
        assert_eq!(
            copy.paragraphs()[1].format.line_spacing,
            Some(LineSpacing::Custom(1))
        );
        assert_eq!(copy.paragraphs()[3].text, "Sub two");
        assert_eq!(document.paragraphs()[1].text, "Sub");
    }

    #[test]
    fn outline_styles_give_each_level_its_font_for_display_only() {
        let document = outline();
        let copy = outline_document(&document, 9_000, &[false; 5], true);
        let title = &copy.styles()[&StyleId(901)];
        assert_eq!(
            (
                title.font_family.as_str(),
                title.font_size_points,
                title.bold
            ),
            ("Times New Roman", 18.0, true)
        );
        let sub = &copy.styles()[&StyleId(902)];
        assert_eq!(
            (sub.font_family.as_str(), sub.font_size_points),
            ("Times New Roman", 14.0)
        );
        let body = &copy.styles()[&document.body_text_style()];
        assert_eq!(body.font_family, "Arial");
        assert_ne!(
            document.styles()[&StyleId(902)].font_size_points,
            14.0,
            "the document keeps its own"
        );
    }
}
