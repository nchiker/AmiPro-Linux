//! Tools › Preferences… (Ctrl+,; `UI_SPEC.md` §39, CM6): one dialog with
//! pages, in place of Ami Pro's User Setup and its Options and Paths
//! dialogs. Every value applies as it is set and is kept in the COSMIC
//! configuration (never the user's in tests).
//!
//! UI review 2026-11: each page is in titled sections; General holds what
//! new documents, saving, and editing do; View holds the window's parts
//! (the toolbar's own page went: its switch was the View page's), the
//! Custom Zoom, and the marks on the page in View › Show's order.

use std::path::PathBuf;

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::{Message, SaveFormat};

/// Undo levels Preferences offers; the app has always kept 100.
pub const UNDO_LEVELS: std::ops::RangeInclusive<usize> = 1..=1000;
/// Recent files the File menu lists.
pub const RECENT_FILES: std::ops::RangeInclusive<usize> = 0..=9;
/// View › Custom Zoom's percentages (Ami Pro's 10–400 from the app's
/// smallest zoom, CK9).
pub const CUSTOM_ZOOM: std::ops::RangeInclusive<usize> = 25..=400;
/// Recent files kept, whatever the File menu lists.
pub const RECENT_KEPT: usize = 9;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Page {
    #[default]
    General,
    Folders,
    View,
}

impl Page {
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::General, "General"),
        (Self::Folders, "Folders"),
        (Self::View, "View"),
    ];
}

/// The View page's settings: the View menu's own toggles, in one place:
/// the window's parts, then View › Show's marks in its order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewSetting {
    Toolbar,
    Ruler,
    VerticalRuler,
    StatusBar,
    TabsReturns,
    ColumnGuides,
    MarginsInColour,
    Pictures,
    TableAddresses,
    FieldShading,
    Notes,
    Bookmarks,
}

impl ViewSetting {
    pub const ALL: [(Self, &'static str); 12] = [
        (Self::Toolbar, "Toolbar"),
        (Self::Ruler, "Ruler"),
        (Self::VerticalRuler, "Vertical ruler"),
        (Self::StatusBar, "Status bar"),
        (Self::TabsReturns, "Tabs and returns"),
        (Self::ColumnGuides, "Column guides"),
        (Self::MarginsInColour, "Margins in colour"),
        (Self::Pictures, "Pictures"),
        (Self::TableAddresses, "Table row/column headings"),
        (Self::FieldShading, "Field shading"),
        (Self::Notes, "Notes"),
        (Self::Bookmarks, "Bookmarks"),
    ];

    /// A part of the window (the rest are marks on the page).
    pub const fn is_window(self) -> bool {
        matches!(
            self,
            Self::Toolbar | Self::Ruler | Self::VerticalRuler | Self::StatusBar
        )
    }

    /// Whether the text editor (no page surface) has this setting.
    pub const fn in_text_editor(self) -> bool {
        matches!(self, Self::Toolbar)
    }
}

/// The zoom every document opens at (Ami Pro's `initview`: Custom at the
/// Custom zoom percentage from the factory, or Standard, Full Page; Page
/// width is the app's own).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OpenZoom {
    #[default]
    Custom,
    ActualSize,
    PageWidth,
    FullPage,
}

impl OpenZoom {
    pub const ALL: [Self; 4] = [
        Self::Custom,
        Self::ActualSize,
        Self::PageWidth,
        Self::FullPage,
    ];

    /// The value kept in the configuration.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::ActualSize => "actual_size",
            Self::PageWidth => "page_width",
            Self::FullPage => "full_page",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|zoom| zoom.key() == key)
    }

    /// The choice as the View page lists it; Custom names its percentage.
    pub fn label(self, custom: u16) -> String {
        match self {
            Self::Custom => format!("Custom ({custom}%)"),
            Self::ActualSize => "100%".to_owned(),
            Self::PageWidth => "Page width".to_owned(),
            Self::FullPage => "Full page".to_owned(),
        }
    }
}

/// The height of every page's area; a longer page scrolls.
const PAGE_HEIGHT: f32 = 430.0;

/// The minutes between recovery copies that Preferences offers (P6.3), and
/// the default.
pub const RECOVERY_INTERVALS: [u32; 4] = [1, 2, 5, 10];
pub const RECOVERY_DEFAULT: u32 = 2;

/// The dialog while it is open: its page and the number fields as typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreferencesState {
    pub page: Page,
    pub undo_levels: String,
    pub recent_files: String,
    pub custom_zoom: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PreferencesAction {
    Page(Page),
    /// Typed undo levels; a number in [`UNDO_LEVELS`] applies at once.
    UndoLevels(String),
    /// Typed Custom Zoom; a number in [`CUSTOM_ZOOM`] applies at once.
    CustomZoom(String),
    /// The zoom documents open at.
    OpenZoom(OpenZoom),
    /// Typed recent files; a number in [`RECENT_FILES`] applies at once.
    RecentFiles(String),
    /// The format Save As offers for a document with no file yet.
    SaveFormat(SaveFormat),
    KeepBackup(bool),
    /// Check for updates when the program starts (P6.2).
    CheckUpdates(bool),
    /// Keep recovery copies while editing (P6.3).
    KeepRecovery(bool),
    /// The minutes between recovery copies, one of
    /// [`RECOVERY_INTERVALS`].
    RecoveryMinutes(u32),
    /// The style sheet File › New… selects when none was used yet.
    DefaultSheet(String),
    /// Folders › Add…: the desktop's folder chooser.
    AddFolder,
    FolderChosen(Option<PathBuf>),
    RemoveFolder(usize),
    /// Folders › Documents › Choose…: the desktop's folder chooser.
    ChooseDocuments,
    DocumentsChosen(Option<PathBuf>),
    /// Folders › Documents › Clear: the desktop's own default again.
    ClearDocuments,
    View(ViewSetting, bool),
    /// View › Customize Toolbar…: the toolbar editor, in place of this
    /// dialog.
    CustomizeToolbar,
    Close,
}

/// What the pages show, read from the preferences.
pub struct Shown<'a> {
    pub save_format: SaveFormat,
    pub keep_backup: bool,
    pub check_updates: bool,
    pub keep_recovery: bool,
    pub recovery_minutes: u32,
    pub default_sheet: &'a str,
    /// The remembered style-sheet folders, which Remove can take.
    pub style_folders: &'a [PathBuf],
    /// The Documents folder the file choosers start in (DF1), when set.
    pub documents_folder: Option<&'a std::path::Path>,
    /// Folders from `AMIPRO_STYLES`, listed but not removable here.
    pub environment_folders: Vec<PathBuf>,
    /// In [`ViewSetting::ALL`]'s order.
    pub view: [bool; 12],
    pub open_zoom: OpenZoom,
    /// The Custom zoom percentage in effect (the field may hold a typed
    /// number out of range).
    pub custom_zoom: u16,
    /// The text editor has only the toolbar of the View page.
    pub page_surface: bool,
}

/// A number typed in a field, when it is one in `range`.
pub fn number_in(text: &str, range: &std::ops::RangeInclusive<usize>) -> Option<usize> {
    text.trim()
        .parse::<usize>()
        .ok()
        .filter(|value| range.contains(value))
}

fn send(action: PreferencesAction) -> Message {
    Message::Preferences(action)
}

fn number_field<'a>(
    label: &'static str,
    text: &'a str,
    range: &std::ops::RangeInclusive<usize>,
    on_input: fn(String) -> PreferencesAction,
) -> Element<'a, Message> {
    let mut column: Vec<Element<'a, Message>> = vec![
        widget::row::with_children(vec![
            widget::text::body(label).width(Length::Fill).into(),
            widget::text_input("", text.to_owned())
                .on_input(move |text| send(on_input(text)))
                .width(80)
                .into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    ];
    if number_in(text, range).is_none() {
        column.push(
            widget::text::caption(format!(
                "Type a number from {} to {}.",
                range.start(),
                range.end()
            ))
            .into(),
        );
    }
    widget::column::with_children(column).spacing(4).into()
}

fn toggle_row<'a>(
    label: &'static str,
    on: bool,
    message: fn(bool) -> Message,
) -> Element<'a, Message> {
    widget::row::with_children(vec![
        widget::text::body(label).width(Length::Fill).into(),
        widget::toggler(on).on_toggle(message).into(),
    ])
    .spacing(8)
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

/// A titled group of a page's settings.
fn section<'a>(title: &'static str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut children: Vec<Element<'a, Message>> = vec![widget::text::heading(title).into()];
    children.extend(rows);
    widget::column::with_children(children).spacing(10).into()
}

/// The minutes between recovery copies: one button for each interval.
fn recovery_interval_row<'a>(shown: &Shown<'_>) -> Element<'a, Message> {
    let mut buttons: Vec<Element<'a, Message>> = vec![
        widget::text::body("Write a copy every")
            .width(Length::Fill)
            .into(),
    ];
    for minutes in RECOVERY_INTERVALS {
        let label = if minutes == 1 {
            "1 min".to_owned()
        } else {
            format!("{minutes} min")
        };
        let button = if minutes == shown.recovery_minutes {
            widget::button::suggested(label)
        } else {
            widget::button::standard(label)
        };
        buttons.push(
            button
                .on_press_maybe(
                    shown
                        .keep_recovery
                        .then(|| send(PreferencesAction::RecoveryMinutes(minutes))),
                )
                .into(),
        );
    }
    widget::row::with_children(buttons)
        .spacing(4)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

/// Save new documents as: one button for each format Save As writes, on
/// a row of their own under the label (beside it, the label was squeezed
/// under the buttons).
fn save_format_row<'a>(shown: &Shown<'_>) -> Element<'a, Message> {
    let mut buttons: Vec<Element<'a, Message>> = Vec::new();
    for format in SaveFormat::ALL {
        let label = match format {
            SaveFormat::Sam => "Ami Pro (.SAM)",
            SaveFormat::Native => "Native (.amipro)",
            SaveFormat::Markdown => "Markdown (.md)",
        };
        let button = if format == shown.save_format {
            widget::button::suggested(label)
        } else {
            widget::button::standard(label)
        };
        buttons.push(
            button
                .on_press(send(PreferencesAction::SaveFormat(format)))
                .into(),
        );
    }
    widget::column::with_children(vec![
        widget::text::body("Save new documents as").into(),
        widget::row::with_children(buttons)
            .spacing(4)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
    ])
    .spacing(6)
    .into()
}

fn general<'a>(state: &'a PreferencesState, shown: &Shown<'_>) -> Vec<Element<'a, Message>> {
    let mut saving = vec![
        toggle_row(
            "Keep the original as NAME.SAM.bak when saving over an Ami Pro document",
            shown.keep_backup,
            |on| send(PreferencesAction::KeepBackup(on)),
        ),
        toggle_row(
            "Keep recovery copies while editing",
            shown.keep_recovery,
            |on| send(PreferencesAction::KeepRecovery(on)),
        ),
    ];
    // The interval only matters with recovery copies on.
    if shown.keep_recovery {
        saving.push(recovery_interval_row(shown));
    }
    vec![
        section(
            "New documents",
            vec![
                widget::row::with_children(vec![
                    widget::text::body("Style sheet File › New… selects first")
                        .width(Length::Fill)
                        .into(),
                    widget::text_input("_DEFAULT.STY", shown.default_sheet.to_owned())
                        .on_input(|name| send(PreferencesAction::DefaultSheet(name)))
                        .width(160)
                        .into(),
                ])
                .spacing(8)
                .align_y(cosmic::iced::Alignment::Center)
                .into(),
                save_format_row(shown),
            ],
        ),
        section("Saving", saving),
        section(
            "Editing",
            vec![
                number_field(
                    "Undo levels",
                    &state.undo_levels,
                    &UNDO_LEVELS,
                    PreferencesAction::UndoLevels,
                ),
                number_field(
                    "Recent files in the File menu",
                    &state.recent_files,
                    &RECENT_FILES,
                    PreferencesAction::RecentFiles,
                ),
            ],
        ),
        section(
            "Updates",
            vec![toggle_row(
                "Check for updates when the program starts (once a day at most)",
                shown.check_updates,
                |on| send(PreferencesAction::CheckUpdates(on)),
            )],
        ),
    ]
}

/// The Documents row's text: the folder, or what unset means.
pub fn documents_text(folder: Option<&std::path::Path>) -> String {
    folder.map_or_else(
        || "Not set — the desktop's own default".to_owned(),
        |folder| folder.display().to_string(),
    )
}

/// The characters of a path a Folders row shows; a longer one loses its
/// middle, keeping where it starts and the folder's own name.
const PATH_CHARS: usize = 36;

/// `text` in at most `max` characters: whole when it fits, else its start
/// and its end around an ellipsis, more of the end (the folder's name).
pub fn elide_middle(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max || max < 5 {
        return text.to_owned();
    }
    let keep = max - 1;
    let head = keep * 2 / 5;
    let tail = keep - head;
    let start: String = text.chars().take(head).collect();
    let end: String = text.chars().skip(count - tail).collect();
    format!("{start}…{end}")
}

/// One folder in a Folders list: its icon, the path (middle elided, the
/// whole path in a tooltip), a note under it, and its button on the right.
fn folder_row<'a>(
    path: &str,
    note: Option<&'static str>,
    button: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let shown = elide_middle(path, PATH_CHARS);
    let mut text: Vec<Element<'a, Message>> = vec![
        widget::text::body(shown)
            .wrapping(cosmic::iced::widget::text::Wrapping::None)
            .into(),
    ];
    if let Some(note) = note {
        text.push(widget::text::caption(note).into());
    }
    let text: Element<'a, Message> = widget::column::with_children(text).spacing(2).into();
    let text = widget::tooltip(
        text,
        widget::text::body(path.to_owned()),
        widget::tooltip::Position::Bottom,
    );
    let mut children: Vec<Element<'a, Message>> = vec![
        widget::icon::from_name("folder-symbolic").size(16).into(),
        widget::container(text).width(Length::Fill).into(),
    ];
    if let Some(button) = button {
        children.push(button);
    }
    widget::row::with_children(children)
        .spacing(12)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

/// A Folders row's button: one width, so the buttons line up.
fn row_button<'a>(label: &'static str, message: Option<Message>) -> Element<'a, Message> {
    widget::button::standard(label)
        .on_press_maybe(message)
        .width(Length::Fixed(96.0))
        .into()
}

fn folders<'a>(shown: &Shown<'_>) -> Vec<Element<'a, Message>> {
    // Documents: the folder (or what unset means), Choose… and Clear.
    let documents_path = documents_text(shown.documents_folder);
    let documents = widget::list_column()
        .add(folder_row(
            &documents_path,
            None,
            Some(row_button(
                "Choose…",
                Some(send(PreferencesAction::ChooseDocuments)),
            )),
        ))
        .into_element();
    let mut documents_rows = vec![
        widget::text::caption(
            "Where Open, Save As, Insert File, Import, Export, and Insert Image start \
             when the document has no folder of its own.",
        )
        .into(),
        documents,
    ];
    if shown.documents_folder.is_some() {
        documents_rows.push(
            widget::row::with_children(vec![
                widget::space().width(Length::Fill).into(),
                widget::button::text("Use the Desktop's Default")
                    .on_press(send(PreferencesAction::ClearDocuments))
                    .into(),
            ])
            .into(),
        );
    }
    // Style-sheet folders: the environment's first, marked and fixed, then
    // the user's, each with Remove.
    let mut list = widget::list_column();
    let mut any = false;
    for folder in &shown.environment_folders {
        any = true;
        list = list.add(folder_row(
            &folder.display().to_string(),
            Some("From AMIPRO_STYLES: always searched, not removable"),
            None,
        ));
    }
    for (index, folder) in shown.style_folders.iter().enumerate() {
        any = true;
        list = list.add(folder_row(
            &folder.display().to_string(),
            None,
            Some(row_button(
                "Remove",
                Some(send(PreferencesAction::RemoveFolder(index))),
            )),
        ));
    }
    let mut styles_rows: Vec<Element<'a, Message>> = vec![
        widget::text::caption(
            "Searched for a document's style sheet and listed by Use Another Style Sheet.",
        )
        .into(),
    ];
    if any {
        styles_rows.push(list.into_element());
    } else {
        styles_rows.push(widget::text::body("No folders yet.").into());
    }
    styles_rows.push(
        widget::row::with_children(vec![
            widget::space().width(Length::Fill).into(),
            widget::button::standard("Add Folder…")
                .on_press(send(PreferencesAction::AddFolder))
                .into(),
        ])
        .into(),
    );
    vec![
        section("Documents", documents_rows),
        section("Style-sheet folders", styles_rows),
    ]
}

fn view_page<'a>(state: &'a PreferencesState, shown: &Shown<'_>) -> Vec<Element<'a, Message>> {
    let row = |setting: ViewSetting, label: &'static str, on: bool| -> Element<'a, Message> {
        let mut children: Vec<Element<'a, Message>> =
            vec![widget::text::body(label).width(Length::Fill).into()];
        // The toolbar's buttons are edited beside its switch.
        if setting == ViewSetting::Toolbar {
            children.push(
                widget::button::standard("Customize Toolbar…")
                    .on_press(send(PreferencesAction::CustomizeToolbar))
                    .into(),
            );
        }
        children.push(
            widget::toggler(on)
                .on_toggle(move |on| send(PreferencesAction::View(setting, on)))
                .into(),
        );
        widget::row::with_children(children)
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .into()
    };
    let rows: Vec<(ViewSetting, &'static str, bool)> = ViewSetting::ALL
        .iter()
        .zip(shown.view)
        .map(|((setting, label), on)| (*setting, *label, on))
        .filter(|(setting, _, _)| shown.page_surface || setting.in_text_editor())
        .collect();
    let window: Vec<Element<'a, Message>> = rows
        .iter()
        .filter(|(setting, _, _)| setting.is_window())
        .map(|(setting, label, on)| row(*setting, label, *on))
        .collect();
    let mut page = vec![section("Window", window)];
    if shown.page_surface {
        let mut choices: Vec<Element<'a, Message>> = Vec::new();
        for zoom in OpenZoom::ALL {
            let label = zoom.label(shown.custom_zoom);
            let button = if zoom == shown.open_zoom {
                widget::button::suggested(label)
            } else {
                widget::button::standard(label)
            };
            choices.push(
                button
                    .on_press(send(PreferencesAction::OpenZoom(zoom)))
                    .into(),
            );
        }
        page.push(section(
            "Zoom",
            vec![
                // As Save new documents as: the label over its choices.
                widget::column::with_children(vec![
                    widget::text::body("Open documents at").into(),
                    widget::row::with_children(choices)
                        .spacing(4)
                        .align_y(cosmic::iced::Alignment::Center)
                        .into(),
                ])
                .spacing(6)
                .into(),
                number_field(
                    "Custom zoom (%)",
                    &state.custom_zoom,
                    &CUSTOM_ZOOM,
                    PreferencesAction::CustomZoom,
                ),
                widget::text::caption(
                    "New, opened, and imported documents start at this zoom; \
                     View › Custom Zoom uses the percentage. A zoom you choose \
                     for one document stays with it.",
                )
                .into(),
            ],
        ));
        page.push(section(
            "Show on the page",
            rows.iter()
                .filter(|(setting, _, _)| !setting.is_window())
                .map(|(setting, label, on)| row(*setting, label, *on))
                .collect(),
        ));
    }
    page
}

pub fn dialog<'a>(state: &'a PreferencesState, shown: &Shown<'_>) -> Element<'a, Message> {
    let tabs = widget::row::with_children(
        Page::ALL
            .iter()
            .map(|(page, label)| {
                widget::button::text(*label)
                    .class(if *page == state.page {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Text
                    })
                    .on_press(send(PreferencesAction::Page(*page)))
                    .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(4);
    let page = match state.page {
        Page::General => general(state, shown),
        Page::Folders => folders(shown),
        Page::View => view_page(state, shown),
    };
    widget::dialog()
        .title("Preferences")
        .control(
            widget::column::with_children(vec![
                tabs.into(),
                widget::divider::horizontal::default().into(),
                // One height for every page, scrolled when a page is
                // longer: the tabs no longer jump, and the dialog fits the
                // 640-pixel minimum window (UI review 2026-11).
                widget::scrollable(
                    widget::column::with_children(page)
                        .spacing(20)
                        .padding([0, 16, 0, 0])
                        .width(Length::Fill),
                )
                .height(Length::Fixed(PAGE_HEIGHT))
                .into(),
            ])
            .spacing(12)
            .width(Length::Fixed(480.0)),
        )
        .primary_action(widget::button::suggested("Close").on_press(send(PreferencesAction::Close)))
        .into()
}

/// Recent files, newest first: `path` goes to the top, once, and the list
/// keeps [`RECENT_KEPT`].
pub fn remember(recent: &mut Vec<PathBuf>, path: &std::path::Path) {
    recent.retain(|known| known != path);
    recent.insert(0, path.to_path_buf());
    recent.truncate(RECENT_KEPT);
}

/// A recent file as the File menu lists it: its number and name.
pub fn recent_label(index: usize, path: &std::path::Path) -> String {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    format!("{} {name}", index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documents_row_says_when_nothing_is_set() {
        assert_eq!(documents_text(None), "Not set — the desktop's own default");
        assert_eq!(documents_text(Some(std::path::Path::new("/d"))), "/d");
    }

    #[test]
    fn long_paths_lose_their_middle_and_keep_the_folders_name() {
        assert_eq!(elide_middle("/home/me/styles", 44), "/home/me/styles");
        let long = "/home/someone/Documents/Lotus/AmiPro/Styles/Letterheads/Company";
        let shown = elide_middle(long, 30);
        assert_eq!(shown.chars().count(), 30);
        assert!(shown.starts_with("/home/some"), "{shown}");
        assert!(shown.ends_with("heads/Company"), "{shown}");
        assert!(shown.contains('…'));
        // Characters, not bytes.
        let accented = "/é".repeat(40);
        assert_eq!(elide_middle(&accented, 21).chars().count(), 21);
    }

    #[test]
    fn numbers_apply_only_inside_their_range() {
        assert_eq!(number_in(" 25 ", &UNDO_LEVELS), Some(25));
        assert_eq!(number_in("0", &UNDO_LEVELS), None);
        assert_eq!(number_in("1001", &UNDO_LEVELS), None);
        assert_eq!(number_in("x", &UNDO_LEVELS), None);
        assert_eq!(number_in("0", &RECENT_FILES), Some(0));
        assert_eq!(number_in("10", &RECENT_FILES), None);
    }

    #[test]
    fn recent_files_go_to_the_top_once_and_the_list_is_kept_short() {
        let mut recent = Vec::new();
        for index in 0..12 {
            remember(&mut recent, &PathBuf::from(format!("/d/{index}.SAM")));
        }
        assert_eq!(recent.len(), RECENT_KEPT);
        assert_eq!(recent[0], PathBuf::from("/d/11.SAM"));
        remember(&mut recent, &PathBuf::from("/d/5.SAM"));
        assert_eq!(recent[0], PathBuf::from("/d/5.SAM"));
        assert_eq!(recent.len(), RECENT_KEPT);
        assert_eq!(
            recent.iter().filter(|path| path.ends_with("5.SAM")).count(),
            1
        );
        assert_eq!(recent_label(0, &recent[0]), "1 5.SAM");
    }
}
