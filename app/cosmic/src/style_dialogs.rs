//! Ami Pro's Style menu dialogs under Format › Styles ▸ (`UI_SPEC.md` §14,
//! approved 2026-10-13): Create Style, Style Management, Use Another Style
//! Sheet, and Modify Style. Each follows Ami Pro 3.1's dialog (VM
//! screenshots outside Git: `createstyle.png`, `st1mgmt.png`,
//! `st1use2-crop.png`, `modstyle.png`) and applies its change through one
//! editing command, one undo step.

use std::collections::BTreeMap;

use amipro_document::{ParagraphStyle, StyleId, StyleOrigin};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;

/// A style's name as the dialogs list it: Ami Pro marks the styles stored
/// in the document with a bullet (Style Management's "Styles in document").
pub fn listed_name(style: &ParagraphStyle) -> String {
    match style.origin {
        StyleOrigin::Document => format!("• {}", style.name),
        StyleOrigin::Sheet | StyleOrigin::Missing => style.name.clone(),
    }
}

/// The styles a dialog lists: every style Ami Pro has (a name defined
/// nowhere is not one), by name.
pub fn listed_styles(styles: &BTreeMap<StyleId, ParagraphStyle>) -> Vec<(StyleId, String)> {
    let mut listed: Vec<_> = styles
        .values()
        .filter(|style| style.origin != StyleOrigin::Missing)
        .map(|style| (style.id, listed_name(style)))
        .collect();
    listed.sort_by_key(|(id, _)| styles[id].name.to_lowercase());
    listed
}

/// Format › Styles › Create Style…: a new style named `name`, based on a
/// listed style or on the caret paragraph (its style with its own
/// paragraph formatting: Ami Pro's "Selected text").
#[derive(Clone, Debug, PartialEq)]
pub struct CreateStyle {
    pub name: String,
    pub styles: Vec<(StyleId, String)>,
    /// The listed style chosen as the base.
    pub based_on: usize,
    pub selected_text: bool,
}

impl CreateStyle {
    /// The dialog over `styles`, offering `current` (the caret paragraph's
    /// style) as the base, as Ami Pro does.
    pub fn new(styles: &BTreeMap<StyleId, ParagraphStyle>, current: Option<StyleId>) -> Self {
        let listed = listed_styles(styles);
        let based_on = current
            .and_then(|current| listed.iter().position(|(id, _)| *id == current))
            .unwrap_or(0);
        Self {
            name: String::new(),
            styles: listed,
            based_on,
            selected_text: false,
        }
    }

    pub fn base(&self) -> Option<StyleId> {
        self.styles.get(self.based_on).map(|(id, _)| *id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CreateAction {
    Name(String),
    BasedOn(usize),
    SelectedText(bool),
    Create,
    /// Create, then open Modify Style on the new style (Ami Pro's Modify).
    Modify,
    Cancel,
}

/// The Create Style dialog. `problem` says why the name cannot be used;
/// Create waits for a name that can.
pub fn create_style_dialog(
    state: &CreateStyle,
    problem: Option<&'static str>,
    modify: bool,
) -> Element<'static, Message> {
    let names: Vec<String> = state.styles.iter().map(|(_, name)| name.clone()).collect();
    let ready = problem.is_none() && !state.name.is_empty();
    let choose = |selected_text| Message::CreateStyle(CreateAction::SelectedText(selected_text));
    let mut controls: Vec<Element<'static, Message>> = vec![
        widget::text::body("New style").into(),
        widget::text_input("Name", state.name.clone())
            .on_input(|name| Message::CreateStyle(CreateAction::Name(name)))
            .on_submit_maybe(ready.then_some(|_| Message::CreateStyle(CreateAction::Create)))
            .width(Length::Fill)
            .into(),
    ];
    if let Some(problem) = problem.filter(|_| !state.name.is_empty()) {
        controls.push(widget::text::caption(sentence(problem)).into());
    }
    controls.extend([
        widget::text::body("Based on").into(),
        widget::row::with_children(vec![
            widget::radio("Style:", false, Some(state.selected_text), choose).into(),
            widget::dropdown(names, Some(state.based_on), |index| {
                Message::CreateStyle(CreateAction::BasedOn(index))
            })
            .width(Length::Fill)
            .into(),
        ])
        .spacing(12)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
        widget::radio("Selected text", true, Some(state.selected_text), choose).into(),
    ]);
    let mut dialog = widget::dialog()
        .title("Create Style")
        .control(widget::column::with_children(controls).spacing(8))
        .primary_action(
            widget::button::suggested("Create")
                .on_press_maybe(ready.then_some(Message::CreateStyle(CreateAction::Create))),
        )
        .secondary_action(
            widget::button::standard("Cancel").on_press(Message::CreateStyle(CreateAction::Cancel)),
        );
    if modify {
        dialog = dialog.tertiary_action(
            widget::button::standard("Modify…")
                .on_press_maybe(ready.then_some(Message::CreateStyle(CreateAction::Modify))),
        );
    }
    dialog.into()
}

/// A list of names as selectable rows, the chosen one selected, at most
/// `height` tall; names defined nowhere show in the error colour, as Ami
/// Pro shows them red.
fn list<'a>(
    rows: Vec<(StyleId, String)>,
    selected: Option<StyleId>,
    on_select: impl Fn(StyleId) -> Message + 'a,
    empty: &'static str,
    height: f32,
    error: bool,
) -> Element<'a, Message> {
    if rows.is_empty() {
        return widget::container(widget::text::caption(empty))
            .padding(8)
            .height(Length::Fixed(height))
            .into();
    }
    let colour = error.then(|| {
        cosmic::iced::Color::from(cosmic::theme::active().cosmic().destructive_text_color())
    });
    let rows: Vec<Element<'a, Message>> = rows
        .into_iter()
        .map(|(id, name)| {
            let mut label = widget::text::body(name);
            if let Some(colour) = colour {
                label = label.class(cosmic::theme::Text::Color(colour));
            }
            widget::button::custom(label)
                .class(cosmic::theme::Button::ListItem([6.0; 4]))
                .selected(selected == Some(id))
                .padding([4, 8])
                .width(Length::Fill)
                .on_press(on_select(id))
                .into()
        })
        .collect();
    widget::scrollable(widget::column::with_children(rows).spacing(2))
        .height(Length::Fixed(height))
        .into()
}

/// An editing message as a sentence: its first letter in upper case.
pub fn sentence(message: &str) -> String {
    let mut characters = message.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

/// Format › Styles › Style Management… (VM fixture ST1): the styles stored
/// in the document, the style sheet's, and the names defined nowhere.
/// `sheet` is the style sheet as read from its file, for Revert; `None`
/// when the file was not found.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleManagement {
    pub selected: Option<StyleId>,
    pub sheet: Option<amipro_document::StyleSheet>,
}

impl StyleManagement {
    /// The sheet's version of `style`, by name, for Revert.
    pub fn sheet_version(&self, style: &ParagraphStyle) -> Option<&ParagraphStyle> {
        self.sheet
            .as_ref()?
            .styles
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(&style.name))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ManagementAction {
    Select(StyleId),
    /// `<< Move <<`: a sheet style becomes stored in the document.
    MoveToDocument,
    /// The style sheet's version again, for a sheet style the document
    /// changed.
    Revert,
    /// A document style's paragraphs format as Body Text; a changed sheet
    /// style takes the sheet's version again (ST1).
    Remove,
    Close,
}

pub fn style_management_dialog(
    state: &StyleManagement,
    styles: &BTreeMap<StyleId, ParagraphStyle>,
    sheet_name: Option<&str>,
) -> Element<'static, Message> {
    let group = |origin| -> Vec<(StyleId, String)> {
        let mut names: Vec<_> = styles
            .values()
            .filter(|style| style.origin == origin)
            .map(|style| (style.id, style.name.clone()))
            .collect();
        names.sort_by_key(|(_, name)| name.to_lowercase());
        names
    };
    let select = |id| Message::StyleManagement(ManagementAction::Select(id));
    let column = |title: &'static str, rows, empty| -> Element<'static, Message> {
        widget::column::with_children(vec![
            widget::text::heading(title).into(),
            list(rows, state.selected, select, empty, 200.0, false),
        ])
        .spacing(4)
        .width(Length::Fill)
        .into()
    };
    let selected = state.selected.and_then(|id| styles.get(&id));
    let action = |label: &'static str, enabled: bool, action| -> Element<'static, Message> {
        widget::button::standard(label)
            .on_press_maybe(enabled.then_some(Message::StyleManagement(action)))
            .into()
    };
    let from_document = selected.is_some_and(|style| style.origin == StyleOrigin::Document);
    let revertible = selected
        .filter(|style| style.origin == StyleOrigin::Document)
        .is_some_and(|style| state.sheet_version(style).is_some());
    let mut controls: Vec<Element<'static, Message>> = vec![
        widget::text::body(match sheet_name {
            Some(name) => format!("Style sheet: {name}"),
            None => "No style sheet".into(),
        })
        .into(),
        widget::row::with_children(vec![
            column(
                "Styles in document",
                group(StyleOrigin::Document),
                "None stored in the document",
            ),
            column(
                "Styles in style sheet",
                group(StyleOrigin::Sheet),
                "No style sheet styles",
            ),
        ])
        .spacing(16)
        .into(),
    ];
    let missing = group(StyleOrigin::Missing);
    if !missing.is_empty() {
        controls.push(
            widget::column::with_children(vec![
                widget::text::heading("Defined nowhere (formatted as Body Text)").into(),
                list(
                    missing.clone(),
                    state.selected,
                    select,
                    "",
                    (missing.len() as f32 * 30.0).min(120.0),
                    true,
                ),
            ])
            .spacing(4)
            .into(),
        );
    }
    controls.push(
        widget::row::with_children(vec![
            action(
                "<< Move <<",
                selected.is_some_and(|style| style.origin == StyleOrigin::Sheet),
                ManagementAction::MoveToDocument,
            ),
            action("Revert", revertible, ManagementAction::Revert),
            action("Remove", from_document, ManagementAction::Remove),
        ])
        .spacing(8)
        .into(),
    );
    widget::dialog()
        .title("Style Management")
        .control(widget::column::with_children(controls).spacing(12))
        .primary_action(
            widget::button::suggested("Close")
                .on_press(Message::StyleManagement(ManagementAction::Close)),
        )
        .into()
}

/// Format › Styles › Use Another Style Sheet… (ST1B): the style sheets in
/// the style-sheet folders, listed by description as Ami Pro's "List by
/// description" does, or by file name.
#[derive(Clone, Debug, PartialEq)]
pub struct UseStyleSheet {
    /// Each sheet's path and description (its file name when it has none).
    pub sheets: Vec<(std::path::PathBuf, String)>,
    pub selected: Option<usize>,
    pub by_description: bool,
    pub folders: Vec<std::path::PathBuf>,
}

impl UseStyleSheet {
    /// The `.sty` files in `folders`, each read for its description.
    pub fn new(folders: Vec<std::path::PathBuf>) -> Self {
        let mut sheets = Vec::new();
        for folder in &folders {
            let Ok(entries) = std::fs::read_dir(folder) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let is_sheet = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("sty"));
                if !is_sheet || sheets.iter().any(|(known, _)| *known == path) {
                    continue;
                }
                let description = std::fs::read(&path)
                    .ok()
                    .and_then(|bytes| amipro_formats::style_sheet_description(&bytes))
                    .unwrap_or_default();
                sheets.push((path, description));
            }
        }
        sheets.sort_by_key(|(path, _)| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
        });
        Self {
            sheets,
            selected: None,
            by_description: true,
            folders,
        }
    }

    fn label(&self, index: usize) -> String {
        let (path, description) = &self.sheets[index];
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if self.by_description && !description.is_empty() {
            description.clone()
        } else {
            name
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UseSheetAction {
    Select(usize),
    ByDescription(bool),
    /// Choose a sheet anywhere with the file chooser; its folder joins the
    /// style-sheet folders.
    Browse,
    Apply,
    Cancel,
}

pub fn use_style_sheet_dialog(
    state: &UseStyleSheet,
    current: Option<&str>,
) -> Element<'static, Message> {
    let mut order: Vec<usize> = (0..state.sheets.len()).collect();
    order.sort_by_key(|index| state.label(*index).to_lowercase());
    let rows: Vec<Element<'static, Message>> = order
        .into_iter()
        .map(|index| {
            widget::button::custom(widget::text::body(state.label(index)))
                .class(cosmic::theme::Button::ListItem([6.0; 4]))
                .selected(state.selected == Some(index))
                .padding([4, 8])
                .width(Length::Fill)
                .on_press(Message::UseStyleSheet(UseSheetAction::Select(index)))
                .into()
        })
        .collect();
    let list: Element<'static, Message> = if rows.is_empty() {
        widget::text::caption(if state.folders.is_empty() {
            "No style-sheet folder yet: choose a sheet with Browse…, and its folder is remembered."
        } else {
            "No style sheets in the style-sheet folders."
        })
        .into()
    } else {
        widget::scrollable(widget::column::with_children(rows).spacing(2))
            .height(Length::Fixed(240.0))
            .into()
    };
    let mut controls: Vec<Element<'static, Message>> = vec![
        widget::text::body(format!(
            "Current style sheet: {}",
            current.unwrap_or("none")
        ))
        .into(),
        widget::text::heading("Change to").into(),
        list,
        widget::checkbox(state.by_description)
            .label("List by description")
            .on_toggle(|on| Message::UseStyleSheet(UseSheetAction::ByDescription(on)))
            .into(),
    ];
    if let Some((path, description)) = state.selected.and_then(|index| state.sheets.get(index)) {
        controls.push(
            widget::text::caption(format!(
                "{}{}",
                path.display(),
                if description.is_empty() {
                    String::new()
                } else {
                    format!(": {description}")
                }
            ))
            .into(),
        );
    }
    widget::dialog()
        .title("Use Another Style Sheet")
        .control(widget::column::with_children(controls).spacing(8))
        .primary_action(
            widget::button::suggested("OK").on_press_maybe(
                state
                    .selected
                    .map(|_| Message::UseStyleSheet(UseSheetAction::Apply)),
            ),
        )
        .secondary_action(
            widget::button::standard("Cancel")
                .on_press(Message::UseStyleSheet(UseSheetAction::Cancel)),
        )
        .tertiary_action(
            widget::button::standard("Browse…")
                .on_press(Message::UseStyleSheet(UseSheetAction::Browse)),
        )
        .into()
}
