//! Tools › Preferences… (Ctrl+,; `UI_SPEC.md` §39, CM6): one dialog with
//! pages, in place of Ami Pro's User Setup and its Options and Paths
//! dialogs. Every value applies as it is set and is kept in the COSMIC
//! configuration (never the user's in tests).

use std::path::PathBuf;

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;

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
    Toolbar,
}

impl Page {
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::General, "General"),
        (Self::Folders, "Folders"),
        (Self::View, "View"),
        (Self::Toolbar, "Toolbar"),
    ];
}

/// The View page's settings: the View menu's own toggles, in one place.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewSetting {
    Toolbar,
    Ruler,
    FieldShading,
    Notes,
    Bookmarks,
    TabsReturns,
    ColumnGuides,
    MarginsInColour,
    Pictures,
    TableAddresses,
}

impl ViewSetting {
    pub const ALL: [(Self, &'static str); 10] = [
        (Self::Toolbar, "Toolbar"),
        (Self::Ruler, "Ruler"),
        (Self::FieldShading, "Field shading"),
        (Self::Notes, "Notes"),
        (Self::Bookmarks, "Bookmarks"),
        (Self::TabsReturns, "Tabs & Returns"),
        (Self::ColumnGuides, "Column guides"),
        (Self::MarginsInColour, "Margins in colour"),
        (Self::Pictures, "Pictures"),
        (Self::TableAddresses, "Table row/column headings"),
    ];
}

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
    /// Typed recent files; a number in [`RECENT_FILES`] applies at once.
    RecentFiles(String),
    KeepBackup(bool),
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
    /// Toolbar › Customize Toolbar…: the toolbar editor, in place of this
    /// dialog.
    CustomizeToolbar,
    Close,
}

/// What the pages show, read from the preferences.
pub struct Shown<'a> {
    pub keep_backup: bool,
    pub default_sheet: &'a str,
    /// The remembered style-sheet folders, which Remove can take.
    pub style_folders: &'a [PathBuf],
    /// The Documents folder the file choosers start in (DF1), when set.
    pub documents_folder: Option<&'a std::path::Path>,
    /// Folders from `AMIPRO_STYLES`, listed but not removable here.
    pub environment_folders: Vec<PathBuf>,
    pub view: [bool; 10],
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

fn general<'a>(state: &'a PreferencesState, shown: &Shown<'_>) -> Vec<Element<'a, Message>> {
    vec![
        number_field(
            "Undo levels",
            &state.undo_levels,
            &UNDO_LEVELS,
            PreferencesAction::UndoLevels,
        ),
        number_field(
            "Custom zoom (%)",
            &state.custom_zoom,
            &CUSTOM_ZOOM,
            PreferencesAction::CustomZoom,
        ),
        number_field(
            "Recent files in the File menu",
            &state.recent_files,
            &RECENT_FILES,
            PreferencesAction::RecentFiles,
        ),
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
        toggle_row(
            "Keep the original as NAME.SAM.bak when saving over an Ami Pro document",
            shown.keep_backup,
            |on| send(PreferencesAction::KeepBackup(on)),
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

fn folders<'a>(shown: &Shown<'_>) -> Vec<Element<'a, Message>> {
    let mut clear = widget::button::text("Clear");
    if shown.documents_folder.is_some() {
        clear = clear.on_press(send(PreferencesAction::ClearDocuments));
    }
    let mut children: Vec<Element<'a, Message>> = vec![
        widget::text::heading("Documents").into(),
        widget::text::caption(
            "Where Open, Save As, Insert File, Import, Export, and Insert Image start \
             when the document has no folder of its own.",
        )
        .into(),
        widget::row::with_children(vec![
            widget::text::body(documents_text(shown.documents_folder))
                .width(Length::Fill)
                .into(),
            widget::button::standard("Choose…")
                .on_press(send(PreferencesAction::ChooseDocuments))
                .into(),
            clear.into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
        widget::divider::horizontal::default().into(),
        widget::text::heading("Style-sheet folders").into(),
        widget::text::caption(
            "Searched for a document's style sheet and listed by Use Another Style Sheet.",
        )
        .into(),
    ];
    for folder in &shown.environment_folders {
        children.push(
            widget::row::with_children(vec![
                widget::text::body(folder.display().to_string())
                    .width(Length::Fill)
                    .into(),
                widget::text::caption("AMIPRO_STYLES").into(),
            ])
            .spacing(8)
            .into(),
        );
    }
    for (index, folder) in shown.style_folders.iter().enumerate() {
        children.push(
            widget::row::with_children(vec![
                widget::text::body(folder.display().to_string())
                    .width(Length::Fill)
                    .into(),
                widget::button::text("Remove")
                    .on_press(send(PreferencesAction::RemoveFolder(index)))
                    .into(),
            ])
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
    }
    if shown.style_folders.is_empty() && shown.environment_folders.is_empty() {
        children.push(widget::text::body("None yet.").into());
    }
    children.push(
        widget::button::standard("Add…")
            .on_press(send(PreferencesAction::AddFolder))
            .into(),
    );
    children
}

fn view_page<'a>(shown: &Shown<'_>) -> Vec<Element<'a, Message>> {
    let shown_settings = if shown.page_surface { 10 } else { 1 };
    ViewSetting::ALL
        .iter()
        .zip(shown.view)
        .take(shown_settings)
        .map(|((setting, label), on)| {
            let setting = *setting;
            widget::row::with_children(vec![
                widget::text::body(*label).width(Length::Fill).into(),
                widget::toggler(on)
                    .on_toggle(move |on| send(PreferencesAction::View(setting, on)))
                    .into(),
            ])
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .into()
        })
        .collect()
}

fn toolbar_page<'a>(shown: &Shown<'_>) -> Vec<Element<'a, Message>> {
    vec![
        toggle_row("Show the toolbar", shown.view[0], |on| {
            send(PreferencesAction::View(ViewSetting::Toolbar, on))
        }),
        widget::text::caption("Add, remove, and order the toolbar's buttons.").into(),
        widget::button::standard("Customize Toolbar…")
            .on_press(send(PreferencesAction::CustomizeToolbar))
            .into(),
    ]
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
        Page::View => view_page(shown),
        Page::Toolbar => toolbar_page(shown),
    };
    widget::dialog()
        .title("Preferences")
        .control(
            widget::column::with_children(vec![
                tabs.into(),
                widget::divider::horizontal::default().into(),
                widget::column::with_children(page)
                    .spacing(12)
                    .width(Length::Fill)
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
