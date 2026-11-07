//! Deterministic UI-2 chrome composition.
//!
//! This module describes which implemented commands appear in each responsive
//! action-bar state. Widget construction stays in `main.rs`; keeping the
//! composition pure makes command ownership and responsive grouping testable.

use crate::command::{self, COMMANDS, CommandId, Implementation};

pub const WIDE_STYLE_FIELD: u16 = 180;
pub const COMPACT_STYLE_FIELD: u16 = 144;

/// The widths the action bar is built with (`main.rs`, `action_bar_view`),
/// in logical pixels; a layout test checks them against the real widgets.
pub const ICON_BUTTON: f32 = 32.0;
pub const ICON_GAP: f32 = 4.0;
pub const GROUP_GAP: f32 = 8.0;
pub const DIVIDER: f32 = 1.0;
pub const BAR_PADDING: f32 = 8.0;
/// The `More ▾` menu root.
pub const MORE_WIDTH: f32 = 76.0;

/// One place in the action bar (`UI_SPEC.md` §12): a command button, the
/// paragraph-style field, or the line between two groups.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolbarItem {
    Command(CommandId),
    StyleField,
    Separator,
}

/// The approved Revision 2 bar (`UI_SPEC.md` §5 "Action bar"): Document,
/// History, Clipboard, Style, Character, Paragraph, and since decision F1
/// (§20) Draw Frame. The user's toolbar starts
/// as this, less what the build cannot run ([`standard_toolbar`]).
const REVISION_2_TOOLBAR: &[ToolbarItem] = &[
    ToolbarItem::Command(CommandId::NewDocument),
    ToolbarItem::Command(CommandId::Open),
    ToolbarItem::Command(CommandId::Save),
    ToolbarItem::Separator,
    ToolbarItem::Command(CommandId::Undo),
    ToolbarItem::Command(CommandId::Redo),
    ToolbarItem::Separator,
    ToolbarItem::Command(CommandId::Cut),
    ToolbarItem::Command(CommandId::Copy),
    ToolbarItem::Command(CommandId::Paste),
    ToolbarItem::Separator,
    ToolbarItem::StyleField,
    ToolbarItem::Separator,
    ToolbarItem::Command(CommandId::Bold),
    ToolbarItem::Command(CommandId::Italic),
    ToolbarItem::Command(CommandId::Underline),
    ToolbarItem::Separator,
    ToolbarItem::Command(CommandId::AlignLeft),
    ToolbarItem::Command(CommandId::Center),
    ToolbarItem::Command(CommandId::AlignRight),
    ToolbarItem::Command(CommandId::Justify),
    // Bullets and Numbering (`UI_SPEC.md` §21, B2), Promote and Demote
    // (§22, O4).
    ToolbarItem::Separator,
    ToolbarItem::Command(CommandId::Bullets),
    ToolbarItem::Command(CommandId::Numbering),
    ToolbarItem::Command(CommandId::Promote),
    ToolbarItem::Command(CommandId::Demote),
    // Ami Pro's frame SmartIcon, in its default set (`UI_SPEC.md` §20, F1).
    ToolbarItem::Separator,
    ToolbarItem::Command(CommandId::DrawFrame),
];

/// The Standard set in this build: the approved order without what the
/// build cannot run (Underline and alignment need the page surface) and
/// without the separators that would then stand alone.
pub fn standard_toolbar() -> Vec<ToolbarItem> {
    let mut items: Vec<ToolbarItem> = Vec::new();
    for item in REVISION_2_TOOLBAR {
        if !toolbar_item_available(*item) {
            continue;
        }
        if *item == ToolbarItem::Separator
            && items
                .last()
                .is_none_or(|last| *last == ToolbarItem::Separator)
        {
            continue;
        }
        items.push(*item);
    }
    if items.last() == Some(&ToolbarItem::Separator) {
        items.pop();
    }
    items
}

/// Whether `item` can stand on the toolbar in this build: the style field,
/// a separator, or an implemented command with an icon.
pub fn toolbar_item_available(item: ToolbarItem) -> bool {
    match item {
        ToolbarItem::StyleField | ToolbarItem::Separator => true,
        ToolbarItem::Command(command) => {
            command::metadata(command).implementation == Implementation::Implemented
                && action_icon_name(command).is_some()
        }
    }
}

/// What Customize Toolbar offers (`UI_SPEC.md` §12): the style field, then
/// every command that can stand on the bar, in menu order.
pub fn toolbar_choices() -> Vec<ToolbarItem> {
    let commands = command::MENU_LAYOUTS
        .iter()
        .flat_map(|(_, slots)| command::MenuSlot::commands(slots))
        .map(ToolbarItem::Command);
    std::iter::once(ToolbarItem::StyleField)
        .chain(commands)
        .filter(|item| toolbar_item_available(*item))
        .collect()
}

/// The name Customize Toolbar shows for `item`.
pub fn toolbar_item_label(item: ToolbarItem) -> &'static str {
    match item {
        ToolbarItem::Command(command) => command::metadata(command).label,
        ToolbarItem::StyleField => "Style list",
        ToolbarItem::Separator => "Separator",
    }
}

/// The name an item is saved under in the user's configuration: the
/// command's registry name (`NewDocument`), `Style`, or `Separator`.
pub fn toolbar_item_key(item: ToolbarItem) -> String {
    match item {
        ToolbarItem::Command(command) => format!("{command:?}"),
        ToolbarItem::StyleField => "Style".to_owned(),
        ToolbarItem::Separator => "Separator".to_owned(),
    }
}

/// Reads a saved toolbar. Names this build does not know or cannot show
/// (an unimplemented command, a command saved twice) are left out, so a
/// configuration written by a newer or differently built version still
/// gives a working bar.
pub fn toolbar_from_keys(keys: &[String]) -> Vec<ToolbarItem> {
    let mut items: Vec<ToolbarItem> = Vec::new();
    for key in keys {
        let item = match key.as_str() {
            "Style" => Some(ToolbarItem::StyleField),
            "Separator" => Some(ToolbarItem::Separator),
            name => COMMANDS
                .iter()
                .find(|command| format!("{:?}", command.id) == name)
                .map(|command| ToolbarItem::Command(command.id)),
        };
        if let Some(item) = item
            && toolbar_item_available(item)
            && (item == ToolbarItem::Separator || !items.contains(&item))
        {
            items.push(item);
        }
    }
    items
}

/// The Customize Toolbar dialog's selections: a choice in the list of what
/// can be added and an item of the toolbar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToolbarEditor {
    pub choice: Option<ToolbarItem>,
    pub item: Option<usize>,
}

/// One change made in Customize Toolbar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolbarEdit {
    SelectChoice(ToolbarItem),
    SelectItem(usize),
    /// Adds the selected choice after the selected item (or at the end).
    Add,
    /// Adds a separator after the selected item (or at the end).
    AddSeparator,
    Remove,
    MoveUp,
    MoveDown,
    /// Back to the Standard set.
    Reset,
}

impl ToolbarEditor {
    /// Whether `choice` can be added: each command and the style field
    /// stand on the bar once.
    pub fn can_add(toolbar: &[ToolbarItem], choice: ToolbarItem) -> bool {
        choice == ToolbarItem::Separator || !toolbar.contains(&choice)
    }

    /// Applies `edit` to `toolbar`; the selection follows the item moved or
    /// added.
    pub fn apply(&mut self, toolbar: &mut Vec<ToolbarItem>, edit: ToolbarEdit) {
        let insert_at = |selected: Option<usize>, toolbar: &[ToolbarItem]| {
            selected.map_or(toolbar.len(), |index| (index + 1).min(toolbar.len()))
        };
        match edit {
            ToolbarEdit::SelectChoice(choice) => self.choice = Some(choice),
            ToolbarEdit::SelectItem(index) => {
                self.item = (index < toolbar.len()).then_some(index);
            }
            ToolbarEdit::Add => {
                if let Some(choice) = self.choice
                    && Self::can_add(toolbar, choice)
                {
                    let at = insert_at(self.item, toolbar);
                    toolbar.insert(at, choice);
                    self.item = Some(at);
                    // It leaves the list of what can be added.
                    self.choice = None;
                }
            }
            ToolbarEdit::AddSeparator => {
                let at = insert_at(self.item, toolbar);
                toolbar.insert(at, ToolbarItem::Separator);
                self.item = Some(at);
            }
            ToolbarEdit::Remove => {
                if let Some(index) = self.item.filter(|index| *index < toolbar.len()) {
                    toolbar.remove(index);
                    self.item = (!toolbar.is_empty()).then(|| index.min(toolbar.len() - 1));
                }
            }
            ToolbarEdit::MoveUp => {
                if let Some(index) = self.item.filter(|index| (1..toolbar.len()).contains(index)) {
                    toolbar.swap(index, index - 1);
                    self.item = Some(index - 1);
                }
            }
            ToolbarEdit::MoveDown => {
                if let Some(index) = self.item.filter(|index| index + 1 < toolbar.len()) {
                    toolbar.swap(index, index + 1);
                    self.item = Some(index + 1);
                }
            }
            ToolbarEdit::Reset => {
                *toolbar = standard_toolbar();
                self.item = None;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionBarLayout {
    /// The groups shown, in order: runs of items between separators.
    pub visible_groups: Vec<Vec<ToolbarItem>>,
    /// The commands of the groups that did not fit, for `More`.
    pub overflow_commands: Vec<CommandId>,
    pub style_width: u16,
}

/// The toolbar's items split at its separators, empty groups dropped.
pub fn toolbar_groups(items: &[ToolbarItem]) -> Vec<Vec<ToolbarItem>> {
    items
        .split(|item| *item == ToolbarItem::Separator)
        .filter(|group| !group.is_empty())
        .map(<[ToolbarItem]>::to_vec)
        .collect()
}

fn group_width(group: &[ToolbarItem], style_width: u16) -> f32 {
    let items: f32 = group
        .iter()
        .map(|item| match item {
            ToolbarItem::StyleField => f32::from(style_width),
            _ => ICON_BUTTON,
        })
        .sum();
    items + ICON_GAP * group.len().saturating_sub(1) as f32
}

/// The width the bar needs for `groups`, with `More` when `more`.
pub fn bar_width(groups: &[Vec<ToolbarItem>], style_width: u16, more: bool) -> f32 {
    let mut elements: Vec<f32> = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        if index > 0 {
            elements.push(DIVIDER);
        }
        elements.push(group_width(group, style_width));
    }
    if more {
        if !elements.is_empty() {
            elements.push(DIVIDER);
        }
        elements.push(MORE_WIDTH);
    }
    let gaps = GROUP_GAP * elements.len().saturating_sub(1) as f32;
    2.0 * BAR_PADDING + elements.iter().sum::<f32>() + gaps
}

/// Fits `items` into a bar `width` wide. The style field narrows first;
/// then whole groups move to `More` from the end of the bar, so the order
/// is the priority, except the group holding the style field, which a menu
/// cannot hold (`UI_SPEC.md` §5, §12; user, 2026-10-12: More appeared on a
/// mid-size window with room to spare under fixed width thresholds).
pub fn action_bar_layout(items: &[ToolbarItem], width: f32) -> ActionBarLayout {
    let groups = toolbar_groups(items);
    for style_width in [WIDE_STYLE_FIELD, COMPACT_STYLE_FIELD] {
        if bar_width(&groups, style_width, false) <= width {
            return ActionBarLayout {
                visible_groups: groups,
                overflow_commands: Vec::new(),
                style_width,
            };
        }
    }
    let style_width = COMPACT_STYLE_FIELD;
    let mut visible = groups.clone();
    let mut overflow: Vec<Vec<ToolbarItem>> = Vec::new();
    while bar_width(&visible, style_width, true) > width {
        let Some(index) = visible
            .iter()
            .rposition(|group| !group.contains(&ToolbarItem::StyleField))
        else {
            break;
        };
        overflow.insert(0, visible.remove(index));
    }
    // Groups keep their order in `More` as on the bar.
    overflow.sort_by_key(|group| groups.iter().position(|candidate| candidate == group));
    ActionBarLayout {
        visible_groups: visible,
        overflow_commands: overflow
            .iter()
            .flatten()
            .filter_map(|item| match item {
                ToolbarItem::Command(command) => Some(*command),
                _ => None,
            })
            .collect(),
        style_width,
    }
}

/// Names of the icons the app draws itself, which the COSMIC theme does
/// not have (`app_icon_svg`).
pub const APP_ICON_BULLETS: &str = "amipro-list-bullets-symbolic";
pub const APP_ICON_NUMBERS: &str = "amipro-list-numbers-symbolic";
pub const APP_ICON_INSERT_BULLET: &str = "amipro-insert-bullet-symbolic";

/// The SVG of one of the app's own icons, by name.
/// The draw bar's icons (`UI_SPEC.md` §33), the app's own.
pub const DRAW_ICON_SELECT: &str = "draw-select-symbolic";
pub const DRAW_ICON_HAND: &str = "draw-hand-symbolic";
pub const DRAW_ICON_LINE: &str = "draw-line-symbolic";
pub const DRAW_ICON_POLYLINE: &str = "draw-polyline-symbolic";
pub const DRAW_ICON_POLYGON: &str = "draw-polygon-symbolic";
pub const DRAW_ICON_RECTANGLE: &str = "draw-rectangle-symbolic";
pub const DRAW_ICON_ROUNDED: &str = "draw-rounded-symbolic";
pub const DRAW_ICON_ELLIPSE: &str = "draw-ellipse-symbolic";
pub const DRAW_ICON_ARC: &str = "draw-arc-symbolic";
pub const DRAW_ICON_TEXT: &str = "draw-text-symbolic";
pub const DRAW_ICON_SELECT_ALL: &str = "draw-select-all-symbolic";
pub const DRAW_ICON_GROUP: &str = "draw-group-symbolic";
pub const DRAW_ICON_FRONT: &str = "draw-front-symbolic";
pub const DRAW_ICON_BACK: &str = "draw-back-symbolic";
pub const DRAW_ICON_ROTATE: &str = "draw-rotate-symbolic";
pub const DRAW_ICON_FLIP_HORIZONTAL: &str = "draw-flip-horizontal-symbolic";
pub const DRAW_ICON_FLIP_VERTICAL: &str = "draw-flip-vertical-symbolic";
pub const DRAW_ICON_GRID: &str = "draw-grid-symbolic";
pub const DRAW_ICON_SNAP: &str = "draw-snap-symbolic";
pub const DRAW_ICON_EXTRACT: &str = "draw-extract-symbolic";
pub const DRAW_ICON_APPLY: &str = "draw-apply-symbolic";

pub fn app_icon_svg(name: &str) -> Option<&'static [u8]> {
    match name {
        DRAW_ICON_SELECT => Some(include_bytes!("../icons/draw-select-symbolic.svg")),
        DRAW_ICON_HAND => Some(include_bytes!("../icons/draw-hand-symbolic.svg")),
        DRAW_ICON_LINE => Some(include_bytes!("../icons/draw-line-symbolic.svg")),
        DRAW_ICON_POLYLINE => Some(include_bytes!("../icons/draw-polyline-symbolic.svg")),
        DRAW_ICON_POLYGON => Some(include_bytes!("../icons/draw-polygon-symbolic.svg")),
        DRAW_ICON_RECTANGLE => Some(include_bytes!("../icons/draw-rectangle-symbolic.svg")),
        DRAW_ICON_ROUNDED => Some(include_bytes!("../icons/draw-rounded-symbolic.svg")),
        DRAW_ICON_ELLIPSE => Some(include_bytes!("../icons/draw-ellipse-symbolic.svg")),
        DRAW_ICON_ARC => Some(include_bytes!("../icons/draw-arc-symbolic.svg")),
        DRAW_ICON_TEXT => Some(include_bytes!("../icons/draw-text-symbolic.svg")),
        DRAW_ICON_SELECT_ALL => Some(include_bytes!("../icons/draw-select-all-symbolic.svg")),
        DRAW_ICON_GROUP => Some(include_bytes!("../icons/draw-group-symbolic.svg")),
        DRAW_ICON_FRONT => Some(include_bytes!("../icons/draw-front-symbolic.svg")),
        DRAW_ICON_BACK => Some(include_bytes!("../icons/draw-back-symbolic.svg")),
        DRAW_ICON_ROTATE => Some(include_bytes!("../icons/draw-rotate-symbolic.svg")),
        DRAW_ICON_FLIP_HORIZONTAL => {
            Some(include_bytes!("../icons/draw-flip-horizontal-symbolic.svg"))
        }
        DRAW_ICON_FLIP_VERTICAL => Some(include_bytes!("../icons/draw-flip-vertical-symbolic.svg")),
        DRAW_ICON_GRID => Some(include_bytes!("../icons/draw-grid-symbolic.svg")),
        DRAW_ICON_SNAP => Some(include_bytes!("../icons/draw-snap-symbolic.svg")),
        DRAW_ICON_EXTRACT => Some(include_bytes!("../icons/draw-extract-symbolic.svg")),
        DRAW_ICON_APPLY => Some(include_bytes!("../icons/draw-apply-symbolic.svg")),
        APP_ICON_BULLETS => Some(include_bytes!("../icons/list-bullets-symbolic.svg")),
        APP_ICON_NUMBERS => Some(include_bytes!("../icons/list-numbers-symbolic.svg")),
        APP_ICON_INSERT_BULLET => Some(include_bytes!("../icons/insert-bullet-symbolic.svg")),
        _ => None,
    }
}

pub const fn action_icon_name(command: CommandId) -> Option<&'static str> {
    match command {
        CommandId::NewDocument => Some("document-new-symbolic"),
        CommandId::Open => Some("folder-open-symbolic"),
        CommandId::Save => Some("media-floppy-symbolic"),
        CommandId::Undo => Some("edit-undo-symbolic"),
        CommandId::Redo => Some("edit-redo-symbolic"),
        CommandId::Cut => Some("edit-cut-symbolic"),
        CommandId::Copy => Some("edit-copy-symbolic"),
        CommandId::Paste => Some("edit-paste-symbolic"),
        CommandId::Bold => Some("format-text-bold-symbolic"),
        CommandId::Italic => Some("format-text-italic-symbolic"),
        // Offered by Customize Toolbar once implemented (`UI_SPEC.md` §12).
        CommandId::SaveAs => Some("document-save-as-symbolic"),
        CommandId::CloseDocument => Some("window-close-symbolic"),
        CommandId::Print => Some("document-print-symbolic"),
        CommandId::Quit => Some("application-exit-symbolic"),
        CommandId::Delete => Some("edit-delete-symbolic"),
        CommandId::SelectAll => Some("edit-select-all-symbolic"),
        CommandId::Find => Some("edit-find-symbolic"),
        CommandId::GoToPage => Some("go-jump-symbolic"),
        CommandId::PreviousPage => Some("go-up-symbolic"),
        CommandId::NextPage => Some("go-down-symbolic"),
        CommandId::StylesPanel => Some("view-list-symbolic"),
        CommandId::ZoomIn => Some("zoom-in-symbolic"),
        CommandId::ZoomOut => Some("zoom-out-symbolic"),
        CommandId::ActualSize => Some("zoom-original-symbolic"),
        CommandId::FitPageWidth => Some("zoom-fit-best-symbolic"),
        CommandId::FullScreen => Some("view-fullscreen-symbolic"),
        CommandId::Underline => Some("format-text-underline-symbolic"),
        CommandId::ClearDirectFormatting => Some("edit-clear-all-symbolic"),
        CommandId::AlignLeft => Some("format-justify-left-symbolic"),
        CommandId::Center => Some("format-justify-center-symbolic"),
        CommandId::AlignRight => Some("format-justify-right-symbolic"),
        CommandId::Justify => Some("format-justify-fill-symbolic"),
        // The app's own icons (`app/cosmic/icons`): the theme has none.
        CommandId::Bullets => Some(APP_ICON_BULLETS),
        CommandId::Numbering => Some(APP_ICON_NUMBERS),
        CommandId::InsertBullet => Some(APP_ICON_INSERT_BULLET),
        CommandId::Promote => Some("format-indent-less-symbolic"),
        CommandId::Demote => Some("format-indent-more-symbolic"),
        CommandId::IncreaseIndent => Some("format-indent-more-symbolic"),
        CommandId::DecreaseIndent => Some("format-indent-less-symbolic"),
        CommandId::FrameProperties => Some("document-properties-symbolic"),
        CommandId::DrawFrame => Some("insert-rectangle-symbolic"),
        CommandId::BringToFront => Some("go-top-symbolic"),
        CommandId::SendToBack => Some("go-bottom-symbolic"),
        CommandId::PreviousDocument => Some("go-previous-symbolic"),
        CommandId::NextDocument => Some("go-next-symbolic"),
        CommandId::About => Some("help-about-symbolic"),
        CommandId::NewFromStyleSheet => Some("folder-templates-symbolic"),
        CommandId::FastFormat => Some("color-select-symbolic"),
        CommandId::ShowTabsReturns => Some("format-text-direction-ltr-symbolic"),
        _ => None,
    }
}

pub fn command_tooltip(command: CommandId) -> String {
    let metadata = command::metadata(command);
    metadata.shortcut.map_or_else(
        || metadata.label.to_owned(),
        |shortcut| {
            let shortcut = shortcut.key_bind().to_string().replace(" + ", "+");
            format!("{} ({shortcut})", metadata.label)
        },
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusBarModel {
    pub page: String,
    pub message: Option<String>,
    pub source_state: Option<&'static str>,
}

/// The words in `text`: runs between spaces, hard returns, and inline
/// objects that hold at least one letter or digit.
pub fn count_words(text: &str) -> usize {
    text.split(|character: char| {
        character.is_whitespace() || character == '\u{FFFC}' || character == '\u{2028}'
    })
    .filter(|word| word.chars().any(char::is_alphanumeric))
    .count()
}

/// The status bar's word count (`UI_SPEC.md` §5 "Status bar"): the
/// document's words, or the selection's of them.
pub fn word_count_label(total: usize, selected: Option<usize>) -> String {
    let total_text = grouped(total);
    match selected {
        Some(selected) => format!("{} of {total_text} words", grouped(selected)),
        None if total == 1 => "1 word".to_owned(),
        None => format!("{total_text} words"),
    }
}

/// Thousands separated by commas.
fn grouped(number: usize) -> String {
    let digits = number.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The main text's word count, kept per paragraph so that an edit recounts
/// only the paragraphs it changed.
#[derive(Clone, Debug, Default)]
pub struct WordCount {
    paragraphs: Vec<(std::sync::Arc<amipro_document::Paragraph>, usize)>,
    total: usize,
}

impl WordCount {
    pub fn total(&self) -> usize {
        self.total
    }

    /// Brings the count up to `document`'s main text.
    pub fn refresh(&mut self, document: &amipro_document::Document) {
        let previous = std::mem::take(&mut self.paragraphs);
        self.paragraphs = document
            .paragraphs()
            .iter()
            .enumerate()
            .map(|(index, paragraph)| {
                let count = previous
                    .get(index)
                    .filter(|(cached, _)| std::sync::Arc::ptr_eq(cached, paragraph))
                    .map_or_else(|| count_words(&paragraph.text), |(_, count)| *count);
                (std::sync::Arc::clone(paragraph), count)
            })
            .collect();
        self.total = self.paragraphs.iter().map(|(_, count)| count).sum();
    }
}

/// The status bar: the caret's page of the laid-out pages, a message, and
/// whether an imported source is protected.
pub fn status_bar_model(
    page: u32,
    pages: usize,
    raw_message: &str,
    imported_source: bool,
) -> StatusBarModel {
    let message = raw_message.trim();
    let message = (!message.is_empty()
        && !message.starts_with("Modified")
        && !message.starts_with("Ready —"))
    .then(|| message.to_owned());
    let pages = pages.max(1);
    StatusBarModel {
        page: format!("Page {} of {pages}", (page as usize).clamp(1, pages)),
        message,
        source_state: imported_source.then_some("Imported source protected"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bar with the default build's Standard groups, for the responsive
    /// rules whatever the build.
    const FIVE_GROUPS: &[ToolbarItem] = &[
        ToolbarItem::Command(CommandId::NewDocument),
        ToolbarItem::Command(CommandId::Open),
        ToolbarItem::Command(CommandId::Save),
        ToolbarItem::Separator,
        ToolbarItem::Command(CommandId::Undo),
        ToolbarItem::Command(CommandId::Redo),
        ToolbarItem::Separator,
        ToolbarItem::Command(CommandId::Cut),
        ToolbarItem::Command(CommandId::Copy),
        ToolbarItem::Command(CommandId::Paste),
        ToolbarItem::Separator,
        ToolbarItem::StyleField,
        ToolbarItem::Separator,
        ToolbarItem::Command(CommandId::Bold),
        ToolbarItem::Command(CommandId::Italic),
    ];

    #[test]
    fn the_standard_set_is_the_approved_order_less_what_the_build_cannot_run() {
        let standard = standard_toolbar();
        if cfg!(feature = "page-surface") {
            assert_eq!(standard, REVISION_2_TOOLBAR);
        } else {
            assert_eq!(standard, FIVE_GROUPS);
        }
        assert_ne!(standard.first(), Some(&ToolbarItem::Separator));
        assert_ne!(standard.last(), Some(&ToolbarItem::Separator));
        assert!(standard.iter().all(|item| toolbar_item_available(*item)));
    }

    #[test]
    fn a_saved_toolbar_reads_back_without_what_this_build_cannot_show() {
        let keys: Vec<String> = standard_toolbar()
            .iter()
            .map(|item| toolbar_item_key(*item))
            .collect();
        assert_eq!(keys[..4], ["NewDocument", "Open", "Save", "Separator"]);
        assert!(keys.contains(&"Style".to_owned()));
        assert_eq!(toolbar_from_keys(&keys), standard_toolbar());
        // Unknown names, commands not implemented here, and a second copy
        // of a command or the style field are left out; separators repeat.
        let saved: Vec<String> = [
            "Save",
            "Teleport",
            "RevertToSaved",
            "Separator",
            "Save",
            "Style",
            "Separator",
            "Style",
            "Bold",
        ]
        .map(str::to_owned)
        .to_vec();
        assert_eq!(
            toolbar_from_keys(&saved),
            [
                ToolbarItem::Command(CommandId::Save),
                ToolbarItem::Separator,
                ToolbarItem::StyleField,
                ToolbarItem::Separator,
                ToolbarItem::Command(CommandId::Bold),
            ]
        );
        assert!(toolbar_from_keys(&[]).is_empty());
    }

    #[test]
    fn customize_toolbar_offers_the_style_field_and_implemented_commands_with_icons() {
        let choices = toolbar_choices();
        assert_eq!(choices[0], ToolbarItem::StyleField);
        for item in &standard_toolbar() {
            if *item != ToolbarItem::Separator {
                assert!(choices.contains(item), "{item:?}");
            }
        }
        for choice in &choices {
            assert!(toolbar_item_available(*choice));
            assert_eq!(choices.iter().filter(|other| *other == choice).count(), 1);
        }
        assert!(!choices.contains(&ToolbarItem::Separator));
        // Print is offered where it is implemented (the page surface, since
        // the print and PDF run); Revert to Saved is not yet anywhere.
        assert_eq!(
            choices.contains(&ToolbarItem::Command(CommandId::Print)),
            cfg!(feature = "page-surface")
        );
        assert!(!choices.contains(&ToolbarItem::Command(CommandId::RevertToSaved)));
        assert!(choices.contains(&ToolbarItem::Command(CommandId::SaveAs)));
        assert_eq!(
            choices.contains(&ToolbarItem::Command(CommandId::ZoomIn)),
            cfg!(feature = "page-surface")
        );
        // Every command with an icon has a symbolic one.
        for command in COMMANDS {
            if let Some(icon) = action_icon_name(command.id) {
                assert!(icon.ends_with("-symbolic"), "{:?}", command.id);
            }
        }
    }

    #[test]
    fn customize_toolbar_adds_after_the_selection_moves_removes_and_resets() {
        let save_as = ToolbarItem::Command(CommandId::SaveAs);
        let mut toolbar = standard_toolbar();
        let mut editor = ToolbarEditor::default();
        // Nothing chosen: Add does nothing.
        editor.apply(&mut toolbar, ToolbarEdit::Add);
        assert_eq!(toolbar, standard_toolbar());
        // After the selected item.
        editor.apply(&mut toolbar, ToolbarEdit::SelectChoice(save_as));
        editor.apply(&mut toolbar, ToolbarEdit::SelectItem(2));
        editor.apply(&mut toolbar, ToolbarEdit::Add);
        assert_eq!(toolbar[3], save_as);
        assert_eq!(editor.item, Some(3));
        // Once only.
        assert_eq!(editor.choice, None);
        assert!(!ToolbarEditor::can_add(&toolbar, save_as));
        editor.apply(&mut toolbar, ToolbarEdit::SelectChoice(save_as));
        editor.apply(&mut toolbar, ToolbarEdit::Add);
        assert_eq!(toolbar.iter().filter(|item| **item == save_as).count(), 1);
        // Moving keeps the selection on the item.
        editor.apply(&mut toolbar, ToolbarEdit::MoveUp);
        assert_eq!((toolbar[2], editor.item), (save_as, Some(2)));
        editor.apply(&mut toolbar, ToolbarEdit::MoveDown);
        editor.apply(&mut toolbar, ToolbarEdit::MoveDown);
        assert_eq!((toolbar[4], editor.item), (save_as, Some(4)));
        // A separator after it, then remove both.
        editor.apply(&mut toolbar, ToolbarEdit::AddSeparator);
        assert_eq!((toolbar[5], editor.item), (ToolbarItem::Separator, Some(5)));
        editor.apply(&mut toolbar, ToolbarEdit::Remove);
        editor.apply(&mut toolbar, ToolbarEdit::SelectItem(4));
        editor.apply(&mut toolbar, ToolbarEdit::Remove);
        assert_eq!(toolbar, standard_toolbar());
        // The ends: no move past them; with nothing selected Add appends.
        editor.apply(&mut toolbar, ToolbarEdit::SelectItem(0));
        editor.apply(&mut toolbar, ToolbarEdit::MoveUp);
        assert_eq!(toolbar, standard_toolbar());
        let past_the_end = toolbar.len();
        editor.apply(&mut toolbar, ToolbarEdit::SelectItem(past_the_end));
        assert_eq!(editor.item, None);
        editor.apply(&mut toolbar, ToolbarEdit::SelectChoice(save_as));
        editor.apply(&mut toolbar, ToolbarEdit::Add);
        assert_eq!(toolbar.last(), Some(&save_as));
        // Removing everything leaves an empty bar; Reset brings the Standard.
        while !toolbar.is_empty() {
            editor.apply(&mut toolbar, ToolbarEdit::SelectItem(0));
            editor.apply(&mut toolbar, ToolbarEdit::Remove);
        }
        assert_eq!(editor.item, None);
        assert!(action_bar_layout(&toolbar, 900.0).visible_groups.is_empty());
        editor.apply(&mut toolbar, ToolbarEdit::Reset);
        assert_eq!(toolbar, standard_toolbar());
    }

    #[test]
    fn the_standard_bar_fits_wide_windows_and_moves_its_last_groups_when_narrow() {
        // No group of the Standard bar moved to More on a mid-size window
        // (the user's report, 2026-10-12). With the page surface's list and
        // level buttons (B2, O4, the user knowing the cost) it needs about
        // 1,000 px with the wide style field and about 970 with the compact
        // one; below that its last two groups (the lists and Draw Frame) move
        // to More, which needs room of its own. Without the page surface it
        // needs about 600.
        let standard = standard_toolbar();
        let groups = toolbar_groups(&standard);
        let needed = bar_width(&groups, WIDE_STYLE_FIELD, false);
        let compact = bar_width(&groups, COMPACT_STYLE_FIELD, false);
        assert!(needed < 1_024.0, "{needed}");
        for width in [needed, 1_024.0, 1_280.0, 1_920.0] {
            let layout = action_bar_layout(&standard, width);
            assert_eq!(layout.visible_groups, groups);
            assert!(layout.overflow_commands.is_empty(), "at {width}");
            assert_eq!(layout.style_width, WIDE_STYLE_FIELD, "at {width}");
        }
        let layout = action_bar_layout(&standard, compact);
        assert!(layout.overflow_commands.is_empty(), "at {compact}");
        assert_eq!(layout.style_width, COMPACT_STYLE_FIELD);
        if compact > 900.0 {
            let layout = action_bar_layout(&standard, 900.0);
            assert_eq!(layout.visible_groups, groups[..groups.len() - 2]);
            assert_eq!(
                layout.overflow_commands,
                [
                    CommandId::Bullets,
                    CommandId::Numbering,
                    CommandId::Promote,
                    CommandId::Demote,
                    CommandId::DrawFrame
                ]
            );
        } else {
            let layout = action_bar_layout(&standard, 900.0);
            assert!(layout.overflow_commands.is_empty());
        }
    }

    #[test]
    fn narrow_bars_narrow_the_style_field_then_move_whole_groups_from_the_end() {
        let groups = toolbar_groups(FIVE_GROUPS);
        let wide = bar_width(&groups, WIDE_STYLE_FIELD, false);
        let compact = bar_width(&groups, COMPACT_STYLE_FIELD, false);
        let layout = action_bar_layout(FIVE_GROUPS, wide - 1.0);
        assert!(layout.overflow_commands.is_empty());
        assert_eq!(layout.style_width, COMPACT_STYLE_FIELD);
        // Too narrow for everything. More is wider than the two-button
        // character group, so moving that group alone never fits: the
        // clipboard group goes with it.
        let layout = action_bar_layout(FIVE_GROUPS, compact - 1.0);
        assert_eq!(
            layout.overflow_commands,
            [
                CommandId::Cut,
                CommandId::Copy,
                CommandId::Paste,
                CommandId::Bold,
                CommandId::Italic
            ]
        );
        assert!(bar_width(&layout.visible_groups, COMPACT_STYLE_FIELD, true) <= compact - 1.0);
        // With a larger last group, that group alone moves first.
        let mut longer = FIVE_GROUPS.to_vec();
        longer.extend([
            ToolbarItem::Separator,
            ToolbarItem::Command(CommandId::Undo),
            ToolbarItem::Command(CommandId::Redo),
            ToolbarItem::Command(CommandId::Cut),
            ToolbarItem::Command(CommandId::Copy),
        ]);
        let fits_standard = bar_width(&groups, COMPACT_STYLE_FIELD, true);
        let layout = action_bar_layout(&longer, fits_standard);
        assert_eq!(
            layout.overflow_commands,
            [
                CommandId::Undo,
                CommandId::Redo,
                CommandId::Cut,
                CommandId::Copy
            ]
        );
        // Narrower: the clipboard group follows, and More keeps bar order.
        let kept = [groups[0].clone(), groups[1].clone(), groups[3].clone()];
        let layout = action_bar_layout(FIVE_GROUPS, bar_width(&kept, COMPACT_STYLE_FIELD, true));
        assert_eq!(layout.visible_groups, kept);
        assert_eq!(
            layout.overflow_commands,
            [
                CommandId::Cut,
                CommandId::Copy,
                CommandId::Paste,
                CommandId::Bold,
                CommandId::Italic
            ]
        );
        // The style field's group never moves; groups are never split.
        let layout = action_bar_layout(FIVE_GROUPS, 10.0);
        assert_eq!(layout.visible_groups, [vec![ToolbarItem::StyleField]]);
        for group in &layout.visible_groups {
            assert!(groups.contains(group));
        }
    }

    #[test]
    fn action_bar_and_more_never_advertise_future_commands() {
        for width in [300.0, 500.0, 760.0, 1_000.0, 1_920.0] {
            let layout = action_bar_layout(&standard_toolbar(), width);
            let commands = layout
                .visible_groups
                .iter()
                .flatten()
                .filter_map(|item| match item {
                    ToolbarItem::Command(command) => Some(command),
                    _ => None,
                })
                .chain(layout.overflow_commands.iter());
            for command in commands {
                assert_eq!(
                    command::metadata(*command).implementation,
                    Implementation::Implemented
                );
                assert_ne!(*command, CommandId::Print);
            }
        }
    }

    #[test]
    fn icon_controls_have_symbolic_icons_and_registry_derived_tooltips() {
        for item in REVISION_2_TOOLBAR {
            let ToolbarItem::Command(command) = item else {
                continue;
            };
            assert!(action_icon_name(*command).is_some_and(|name| name.ends_with("-symbolic")));
            assert!(command_tooltip(*command).starts_with(command::metadata(*command).label));
        }
        assert_eq!(command_tooltip(CommandId::Save), "Save (Ctrl+S)");
    }

    #[test]
    fn status_model_is_quiet_shows_the_caret_page_and_has_no_style_owner() {
        let edited = status_bar_model(3, 7, "Modified", false);
        assert_eq!(edited.page, "Page 3 of 7");
        assert_eq!(edited.message, None);
        assert_eq!(edited.source_state, None);
        assert_eq!(status_bar_model(9, 7, "", false).page, "Page 7 of 7");
        assert_eq!(status_bar_model(0, 0, "", false).page, "Page 1 of 1");

        let imported =
            status_bar_model(1, 18, "Imported 315 paragraphs; Save As is required", true);
        assert_eq!(
            imported.message.as_deref(),
            Some("Imported 315 paragraphs; Save As is required")
        );
        assert_eq!(imported.source_state, Some("Imported source protected"));
        let rendered_fields = format!(
            "{} {:?} {:?}",
            imported.page, imported.message, imported.source_state
        );
        assert!(!rendered_fields.contains("Style:"));
    }

    #[test]
    fn words_are_counted_and_labelled_with_thousands() {
        assert_eq!(count_words("Two words"), 2);
        assert_eq!(count_words("  café naïve\u{2028}東京 — 3.1 \u{FFFC} "), 4);
        assert_eq!(count_words(""), 0);
        assert_eq!(word_count_label(0, None), "0 words");
        assert_eq!(word_count_label(1, None), "1 word");
        assert_eq!(word_count_label(2_134, None), "2,134 words");
        assert_eq!(
            word_count_label(1_234_567, Some(12)),
            "12 of 1,234,567 words"
        );
    }

    #[test]
    fn the_word_count_recounts_only_changed_paragraphs() {
        let mut document = amipro_document::Document::from_plain_text("One two three\nFour five");
        let mut count = WordCount::default();
        count.refresh(&document);
        assert_eq!(count.total(), 5);
        document.paragraph_mut(1).text = "Four five six".into();
        let unchanged = std::sync::Arc::clone(&document.paragraphs()[0]);
        count.refresh(&document);
        assert_eq!(count.total(), 6);
        assert!(std::sync::Arc::ptr_eq(&count.paragraphs[0].0, &unchanged));
    }
}
