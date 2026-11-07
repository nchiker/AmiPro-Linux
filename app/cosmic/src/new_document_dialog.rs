//! File › New… (`UI_SPEC.md` §39, CM1, CM2): a new document from a style
//! sheet in the style-sheet folders, listed by description or file name,
//! recently used first, With contents, and a preview of the first page.
//! Ctrl+N stays the blank `_DEFAULT.STY` document.

use std::path::PathBuf;

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::page_preview::{Miniature, page_preview};

/// Style sheets used from this dialog, kept for the next start.
pub const RECENT_SHEETS: usize = 5;

/// One style sheet the dialog lists.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetEntry {
    pub path: PathBuf,
    /// The file name as found (`_MEMO1.STY`).
    pub name: String,
    pub description: Option<String>,
    /// `[newmac]`: the macro Ami Pro runs on File › New (CM2).
    pub run_macro: Option<String>,
    pub has_contents: bool,
}

impl SheetEntry {
    fn read(path: PathBuf) -> Option<Self> {
        let bytes = std::fs::read(&path).ok()?;
        // A file that is not a style sheet is not listed.
        amipro_formats::read_style_sheet(&bytes, "x").ok()?;
        Some(Self {
            name: path.file_name()?.to_string_lossy().into_owned(),
            description: amipro_formats::style_sheet_description(&bytes),
            run_macro: amipro_formats::style_sheet_macro(&bytes),
            has_contents: amipro_formats::style_sheet_has_contents(&bytes),
            path,
        })
    }

    pub fn label(&self, by_description: bool) -> String {
        match (&self.description, by_description) {
            (Some(description), true) => format!("{description} ({})", self.name),
            _ => self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewDocumentDialog {
    pub sheets: Vec<SheetEntry>,
    /// The recently used sheets present, newest first (indices into
    /// `sheets`).
    pub recent: Vec<usize>,
    pub selected: Option<usize>,
    pub by_description: bool,
    pub with_contents: bool,
    /// The selected sheet's first page, laid out.
    pub preview: Option<Miniature>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NewAction {
    Select(usize),
    ByDescription(bool),
    WithContents(bool),
    Create,
    Cancel,
}

impl NewDocumentDialog {
    /// The `.sty` files of `folders` (each name once, the first folder's),
    /// the recent ones first, and `default` (or the first) selected.
    pub fn new(folders: &[PathBuf], recent: &[String], default: &str) -> Self {
        let mut sheets: Vec<SheetEntry> = Vec::new();
        for folder in folders {
            let Ok(entries) = std::fs::read_dir(folder) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("sty"))
                })
                .collect();
            paths.sort();
            for path in paths {
                if let Some(entry) = SheetEntry::read(path)
                    && !sheets
                        .iter()
                        .any(|known| known.name.eq_ignore_ascii_case(&entry.name))
                {
                    sheets.push(entry);
                }
            }
        }
        let by_description = true;
        sheets.sort_by_key(|sheet| sheet.label(by_description).to_lowercase());
        let find = |name: &str| {
            sheets
                .iter()
                .position(|sheet| sheet.name.eq_ignore_ascii_case(name))
        };
        let recent: Vec<usize> = recent.iter().filter_map(|name| find(name)).collect();
        let selected = recent
            .first()
            .copied()
            .or_else(|| find(default))
            .or((!sheets.is_empty()).then_some(0));
        let with_contents = selected.is_some_and(|index| sheets[index].has_contents);
        Self {
            sheets,
            recent,
            selected,
            by_description,
            with_contents,
            preview: None,
        }
    }

    pub fn selected_sheet(&self) -> Option<&SheetEntry> {
        self.selected.and_then(|index| self.sheets.get(index))
    }
}

/// `name` goes to the top of the recent sheets, once.
pub fn remember(recent: &mut Vec<String>, name: &str) {
    recent.retain(|known| !known.eq_ignore_ascii_case(name));
    recent.insert(0, name.to_owned());
    recent.truncate(RECENT_SHEETS);
}

/// The new document: the sheet's contents (With contents) or a blank
/// document, on the sheet's styles and page layout and naming it, as
/// Use Another Style Sheet makes them (`[sty]` is the sheet's name).
pub fn document_from_sheet(
    sheet: &SheetEntry,
    with_contents: bool,
) -> Result<amipro_document::Document, String> {
    let bytes = std::fs::read(&sheet.path).map_err(|error| error.to_string())?;
    let style_sheet = amipro_formats::read_style_sheet(&bytes, sheet.name.clone())
        .map_err(|error| error.to_string())?;
    let title = sheet
        .description
        .clone()
        .unwrap_or_else(|| sheet.name.clone());
    let base = if with_contents && sheet.has_contents {
        let mut document = amipro_formats::import_sam(&bytes, None, title)
            .map_err(|error| error.to_string())?
            .document;
        document.set_style_sheet(None);
        document
    } else {
        crate::new_document()
    };
    let mut document = match amipro_editing::apply(
        &base,
        &amipro_editing::Command::UseStyleSheet { sheet: style_sheet },
    ) {
        Ok(applied) => applied.document,
        Err(error) => return Err(error.to_string()),
    };
    *document.info_mut() = amipro_document::DocumentInfo::new(crate::clock::ami_pro_now());
    Ok(document)
}

fn send(action: NewAction) -> Message {
    Message::NewDocumentDialog(action)
}

pub fn dialog(state: &NewDocumentDialog) -> Element<'_, Message> {
    const LIST_HEIGHT: f32 = 300.0;
    let by_description = state.by_description;
    let row = |index: usize| -> Element<'_, Message> {
        let sheet = &state.sheets[index];
        widget::button::custom(
            widget::text::body(sheet.label(by_description))
                .wrapping(cosmic::iced::widget::text::Wrapping::None),
        )
        .class(cosmic::theme::Button::ListItem([6.0; 4]))
        .selected(state.selected == Some(index))
        .width(Length::Fill)
        .on_press(if state.selected == Some(index) {
            send(NewAction::Create)
        } else {
            send(NewAction::Select(index))
        })
        .into()
    };
    let mut list: Vec<Element<'_, Message>> = Vec::new();
    if !state.recent.is_empty() {
        list.push(widget::text::caption_heading("Recent").into());
        list.extend(state.recent.iter().map(|index| row(*index)));
        list.push(widget::text::caption_heading("All style sheets").into());
    }
    let mut all: Vec<usize> = (0..state.sheets.len()).collect();
    all.sort_by_key(|index| state.sheets[*index].label(by_description).to_lowercase());
    list.extend(all.into_iter().map(row));
    if state.sheets.is_empty() {
        list.push(
            widget::text::body(
                "No style sheets were found. Add a style-sheet folder in Preferences › Folders.",
            )
            .into(),
        );
    }
    let list = widget::scrollable(widget::column::with_children(list).spacing(2))
        .width(Length::Fixed(300.0))
        .height(Length::Fixed(LIST_HEIGHT));
    let preview: Element<'_, Message> = match &state.preview {
        Some(page) => page_preview(page, 200.0, LIST_HEIGHT).into(),
        None => widget::container(widget::text::caption("No preview"))
            .center(Length::Fixed(200.0))
            .height(Length::Fixed(LIST_HEIGHT))
            .into(),
    };
    let selected = state.selected_sheet();
    let mut options: Vec<Element<'_, Message>> = vec![
        widget::checkbox(by_description)
            .label("List by description")
            .on_toggle(|on| send(NewAction::ByDescription(on)))
            .into(),
        widget::checkbox(state.with_contents)
            .label("With contents")
            .on_toggle_maybe(
                selected
                    .is_some_and(|sheet| sheet.has_contents)
                    .then_some(|on| send(NewAction::WithContents(on))),
            )
            .into(),
    ];
    if let Some(name) = selected.and_then(|sheet| sheet.run_macro.as_deref()) {
        options.push(
            widget::text::caption(format!(
                "Ami Pro runs the macro {name} for this style sheet; AmiPro-Linux does not run macros."
            ))
            .into(),
        );
    }
    widget::dialog()
        .title("New Document")
        .control(
            widget::column::with_children(vec![
                widget::row::with_children(vec![list.into(), preview])
                    .spacing(16)
                    .into(),
                widget::column::with_children(options).spacing(8).into(),
            ])
            .spacing(12),
        )
        .primary_action(
            widget::button::suggested("Create")
                .on_press_maybe(selected.is_some().then(|| send(NewAction::Create))),
        )
        .secondary_action(widget::button::standard("Cancel").on_press(send(NewAction::Cancel)))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document as lines to compare: page layouts, each flow's paragraphs
    /// (style and text, objects as marks), and the frames (placement, look,
    /// content) in a stable order.
    fn outline(document: &amipro_document::Document) -> Vec<String> {
        use amipro_document::{FrameContent, InlineObject, Placement};
        let style = |id| {
            document
                .styles()
                .get(&id)
                .map_or_else(|| "?".to_owned(), |style| style.name.clone())
        };
        let flow_lines = |label: &str, flow: amipro_document::FlowId| -> Vec<String> {
            let Some(flow) = document.flow(flow) else {
                return Vec::new();
            };
            flow.paragraphs
                .iter()
                .enumerate()
                .map(|(index, paragraph)| {
                    let mut objects = paragraph.objects.iter();
                    let text: String = paragraph
                        .text
                        .chars()
                        .map(|character| match character {
                            '\u{FFFC}' => match objects.next() {
                                Some(InlineObject::FrameAnchor(_)) => "[frame]".to_owned(),
                                Some(other) => format!("[{}]", object_kind(other)),
                                None => "[?]".to_owned(),
                            },
                            other => other.to_string(),
                        })
                        .collect();
                    format!("{label} {index} {}: {text}", style(paragraph.style))
                })
                .collect()
        };
        let mut lines = Vec::new();
        for layout in document.layouts() {
            lines.push(format!("layout {}: {:?}", layout.name, layout.right));
        }
        lines.extend(flow_lines("main", document.main_flow_id()));
        let mut frames: Vec<String> = document
            .frames()
            .map(|frame| {
                let table = matches!(frame.content, FrameContent::Table(_));
                let placement = match frame.placement {
                    // A table frame's height is a cache of its rows
                    // (`SAM_PAGE_FRAME_SPEC.md` §4.2, field 16).
                    Placement::Inline { size, .. } if table => format!("Inline {}", size.width),
                    // The hang below the baseline is set by dragging; a
                    // new document's frames have their file's.
                    Placement::Inline { size, .. } => format!("Inline {size:?}"),
                    other => format!("{other:?}"),
                };
                let content = match &frame.content {
                    FrameContent::Text(flow) => flow_lines("  text", *flow).join(" | "),
                    FrameContent::Table(table) => format!("table {} cells", table.cells.len()),
                    FrameContent::Picture(_) => "picture".to_owned(),
                    FrameContent::Legacy(_) => "legacy".to_owned(),
                };
                format!(
                    "frame {placement} {:?} {:?} {:?}: {content}",
                    frame.style.wrap, frame.style.border, frame.style.shadow
                )
            })
            .collect();
        frames.sort();
        lines.extend(frames);
        lines
    }

    fn object_kind(object: &amipro_document::InlineObject) -> String {
        let debug = format!("{object:?}");
        debug
            .split(|character: char| !character.is_alphanumeric())
            .next()
            .unwrap_or_default()
            .to_owned()
    }

    /// File › New… with contents against Ami Pro's: each
    /// `AMIPRO_NEW_TWINS` entry `SHEET=SAVED` names a style sheet and the
    /// document Ami Pro's New made from it with Run macro off, saved as SAM
    /// (VM fixtures; `COMMANDS_PROGRESS.md`, "Fixes found by the run").
    /// Prints what differs and fails if anything does.
    #[test]
    #[ignore = "reads Ami Pro's style sheets and documents from the VM lab"]
    fn new_documents_match_ami_pros() {
        let Ok(twins) = std::env::var("AMIPRO_NEW_TWINS") else {
            eprintln!("AMIPRO_NEW_TWINS is not set; skipping");
            return;
        };
        let mut differing = 0;
        for twin in twins.split(';').filter(|twin| !twin.is_empty()) {
            let (sheet, saved) = twin.split_once('=').expect("SHEET=SAVED");
            let sheet = SheetEntry::read(PathBuf::from(sheet)).expect("a style sheet");
            let ours = outline(&document_from_sheet(&sheet, true).expect("a new document"));
            let theirs = outline(
                &amipro_formats::import_sam_path(saved)
                    .expect("Ami Pro's document")
                    .document,
            );
            let only_ours: Vec<_> = ours.iter().filter(|line| !theirs.contains(line)).collect();
            let only_theirs: Vec<_> = theirs.iter().filter(|line| !ours.contains(line)).collect();
            eprintln!(
                "{}: {} lines, {} only ours, {} only Ami Pro's",
                sheet.name,
                ours.len(),
                only_ours.len(),
                only_theirs.len()
            );
            for line in &only_ours {
                eprintln!("  ours:    {line}");
            }
            for line in &only_theirs {
                eprintln!("  Ami Pro: {line}");
            }
            differing += usize::from(!only_ours.is_empty() || !only_theirs.is_empty());
        }
        assert_eq!(differing, 0, "new documents unlike Ami Pro's");
    }

    /// How long File › New… takes to list the style sheets of
    /// `AMIPRO_STYLES` (aggregate numbers only).
    #[test]
    #[ignore = "reads the private style-sheet folder"]
    fn listing_the_style_sheet_folder_is_quick() {
        let Some(folder) = std::env::var_os("AMIPRO_STYLES") else {
            return;
        };
        let started = std::time::Instant::now();
        let dialog = NewDocumentDialog::new(&[PathBuf::from(folder)], &[], "_DEFAULT.STY");
        let elapsed = started.elapsed();
        let contents = dialog
            .sheets
            .iter()
            .filter(|sheet| sheet.has_contents)
            .count();
        let macros = dialog
            .sheets
            .iter()
            .filter(|sheet| sheet.run_macro.is_some())
            .count();
        let described = dialog
            .sheets
            .iter()
            .filter(|sheet| sheet.description.is_some())
            .count();
        eprintln!(
            "{} sheets ({contents} with contents, {macros} with a macro, {described} described) in {elapsed:?}",
            dialog.sheets.len()
        );
    }

    #[test]
    fn recent_sheets_go_to_the_top_once() {
        let mut recent = Vec::new();
        for name in [
            "A.STY", "B.STY", "a.sty", "C.STY", "D.STY", "E.STY", "F.STY",
        ] {
            remember(&mut recent, name);
        }
        assert_eq!(recent, ["F.STY", "E.STY", "D.STY", "C.STY", "a.sty"]);
    }
}
