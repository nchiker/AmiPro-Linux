//! File › Export ▸ Folder… (`UI_SPEC.md` §30, EX6): a whole folder of Ami
//! Pro documents exported at once, for the user's archive. A panel in the
//! rail chooses where from (with its subfolders), where to, and the format;
//! the export runs on a thread of its own, file by file, with Stop. The
//! originals are only read; each document goes to the same relative place
//! under To, never over an existing file, and an `Export report.txt` there
//! lists what each format could not hold and what could not be read.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use amipro_exchange::Format;
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::export::ExportJob;

/// The panel's state.
#[derive(Debug)]
pub struct ExportFolder {
    pub from: Option<PathBuf>,
    pub subfolders: bool,
    pub to: Option<PathBuf>,
    pub format: Format,
    pub running: Option<Running>,
    pub finished: Option<Finished>,
}

impl ExportFolder {
    pub fn new(from: Option<PathBuf>, to: Option<PathBuf>, format: Format) -> Self {
        Self {
            from,
            subfolders: true,
            to,
            format,
            running: None,
            finished: None,
        }
    }

    /// Whether Export can start: both folders, nothing running.
    pub fn ready(&self) -> bool {
        self.from.is_some() && self.to.is_some() && self.running.is_none()
    }
}

/// An export under way.
#[derive(Debug)]
pub struct Running {
    pub done: usize,
    pub total: usize,
    pub stop: Arc<AtomicBool>,
}

/// What a folder's export did.
#[derive(Clone, Debug, PartialEq)]
pub struct Finished {
    pub exported: usize,
    /// Exported, with something the format could not hold.
    pub with_losses: usize,
    pub not_read: usize,
    pub stopped: bool,
    pub report: Option<PathBuf>,
}

impl Finished {
    pub fn message(&self) -> String {
        let mut text = format!(
            "Exported {} {}",
            self.exported,
            if self.exported == 1 { "file" } else { "files" }
        );
        if self.with_losses > 0 {
            text.push_str(&format!(
                ", {} with something the format cannot hold",
                self.with_losses
            ));
        }
        if self.not_read > 0 {
            text.push_str(&format!("; {} could not be read", self.not_read));
        }
        if self.stopped {
            text.push_str("; stopped");
        }
        text
    }
}

#[derive(Clone, Debug)]
pub enum FolderAction {
    ChooseFrom,
    ChooseTo,
    /// A folder the desktop's dialog chose: To or From.
    Chosen {
        to: bool,
        folder: Option<PathBuf>,
    },
    Subfolders(bool),
    Format(Format),
    Export,
    Stop,
    OpenReport,
    Back,
    Close,
}

/// What the export's thread reports.
#[derive(Clone, Debug)]
pub enum Progress {
    Started { total: usize },
    Done { done: usize },
    Finished(Result<Finished, String>),
}

/// The rail's width while the panel shows.
pub const WIDTH: f32 = 300.0;

/// The SAM files in `from`, and in its subfolders when `subfolders`, in
/// name order.
pub fn sam_files(from: &Path, subfolders: bool) -> Vec<PathBuf> {
    fn walk(folder: &Path, subfolders: bool, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(folder) else {
            return;
        };
        let mut entries: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                if subfolders {
                    walk(&path, subfolders, out);
                }
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(from, subfolders, &mut out);
    out
}

/// Where `file` (under `from`) goes under `to` in `format`: the same
/// relative place with the format's extension, never over a file there (a
/// number is added: `Letter (2).docx`).
pub fn target(from: &Path, to: &Path, file: &Path, format: Format) -> PathBuf {
    let relative = file.strip_prefix(from).unwrap_or(file);
    let mut path = to.join(relative);
    path.set_extension(format.extension());
    unused(path)
}

/// `path`, or the first of `NAME (2).EXT`, `NAME (3).EXT`, … not taken.
pub fn unused(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    (2..)
        .map(|number| folder.join(format!("{stem} ({number}){extension}")))
        .find(|candidate| !candidate.exists())
        .expect("a free name")
}

/// A folder's export, for its own thread.
pub struct FolderJob {
    pub from: PathBuf,
    pub subfolders: bool,
    pub to: PathBuf,
    pub format: Format,
    /// The style-sheet folders, where documents' sheets are also looked for.
    pub styles: Vec<PathBuf>,
    pub stop: Arc<AtomicBool>,
}

impl FolderJob {
    /// Exports every document, reporting as it goes, and writes the report.
    pub fn run(self, progress: &dyn Fn(Progress)) -> Result<Finished, String> {
        let files = sam_files(&self.from, self.subfolders);
        progress(Progress::Started { total: files.len() });
        std::fs::create_dir_all(&self.to).map_err(|error| error.to_string())?;
        let mut report = format!(
            "Export report: {} (.{})\nFrom: {}\nTo: {}\n\n",
            self.format.name(),
            self.format.extension(),
            self.from.display(),
            self.to.display()
        );
        let mut finished = Finished {
            exported: 0,
            with_losses: 0,
            not_read: 0,
            stopped: false,
            report: None,
        };
        for (index, file) in files.iter().enumerate() {
            if self.stop.load(Ordering::Relaxed) {
                finished.stopped = true;
                break;
            }
            let relative = file
                .strip_prefix(&self.from)
                .unwrap_or(file)
                .display()
                .to_string();
            match self.export_one(file) {
                Ok((written, losses)) => {
                    finished.exported += 1;
                    let name = written.strip_prefix(&self.to).unwrap_or(&written).display();
                    if losses.is_empty() {
                        report.push_str(&format!("{relative} → {name}: nothing lost\n"));
                    } else {
                        finished.with_losses += 1;
                        report.push_str(&format!(
                            "{relative} → {name}: {} cannot hold\n",
                            self.format.name()
                        ));
                        for loss in losses {
                            report.push_str(&format!("    • {loss}\n"));
                        }
                    }
                }
                Err(error) => {
                    finished.not_read += 1;
                    report.push_str(&format!("{relative}: not exported: {error}\n"));
                }
            }
            progress(Progress::Done { done: index + 1 });
        }
        report.push_str(&format!("\n{}.\n", finished.message()));
        let path = unused(self.to.join("Export report.txt"));
        std::fs::write(&path, report).map_err(|error| error.to_string())?;
        finished.report = Some(path);
        Ok(finished)
    }

    /// One document: read, laid out, exported, and written next to its
    /// place under To.
    fn export_one(&self, file: &Path) -> Result<(PathBuf, Vec<String>), String> {
        let import = amipro_formats::import_sam_path_with_style_directories(file, &self.styles)
            .map_err(|error| format!("{error:?}"))?;
        let document = import.document;
        let mut pages = crate::page_layout::SessionPages::default();
        pages.update(&document);
        let path = target(&self.from, &self.to, file, self.format);
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        }
        let title = file
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let prepared = ExportJob {
            document,
            layout: pages.layout().clone(),
            folder: file.parent().map(Path::to_path_buf),
            fields: amipro_document::FieldContext {
                file_name: file
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                directory: file
                    .parent()
                    .map(|folder| folder.display().to_string())
                    .unwrap_or_default(),
                now: amipro_document::WallClock {
                    second: 0,
                    ..crate::clock::local_now()
                },
                ..amipro_document::FieldContext::default()
            },
            title,
            format: self.format,
            path: path.clone(),
            set: Vec::new(),
        }
        .run()?;
        crate::export::write(&prepared)?;
        Ok((path, prepared.losses))
    }
}

fn send(action: FolderAction) -> Message {
    Message::ExportFolder(action)
}

/// A folder's row: its path (or "None chosen") and Choose….
fn folder_row<'a>(
    label: &'static str,
    folder: Option<&Path>,
    choose: FolderAction,
    running: bool,
) -> Element<'a, Message> {
    let shown = folder.map_or_else(
        || "None chosen".to_owned(),
        |folder| folder.display().to_string(),
    );
    widget::column::with_children(vec![
        widget::text::heading(label).into(),
        widget::text::caption(shown)
            .wrapping(cosmic::iced::widget::text::Wrapping::WordOrGlyph)
            .into(),
        widget::button::standard("Choose…")
            .on_press_maybe((!running).then_some(send(choose)))
            .into(),
    ])
    .spacing(4)
    .into()
}

pub fn panel<'a>(state: &ExportFolder) -> Element<'a, Message> {
    let running = state.running.is_some();
    let header = widget::row::with_children(vec![
        widget::text::heading("Export Folder").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(FolderAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Export Folder")
            .on_press(send(FolderAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut body: Vec<Element<'a, Message>> = vec![
        folder_row(
            "From",
            state.from.as_deref(),
            FolderAction::ChooseFrom,
            running,
        ),
        widget::checkbox(state.subfolders)
            .label("Include subfolders")
            .on_toggle_maybe((!running).then_some(|on| send(FolderAction::Subfolders(on))))
            .into(),
        folder_row("To", state.to.as_deref(), FolderAction::ChooseTo, running),
        widget::text::heading("As").into(),
    ];
    for format in [Format::Docx, Format::Odt, Format::Rtf, Format::Text] {
        body.push(
            widget::radio(
                widget::text::body(format!("{} (.{})", format.name(), format.extension())),
                format,
                Some(state.format),
                move |format| send(FolderAction::Format(format)),
            )
            .into(),
        );
    }
    body.push(
        widget::text::caption(
            "Each document's style sheet is looked for beside it and in the style-sheet folders. \
             The documents are only read; nothing is written over a file already there, and an \
             Export report.txt lists what each could not keep.",
        )
        .into(),
    );
    if let Some(running) = &state.running {
        body.push(
            widget::text::body(format!("Exporting {} of {}…", running.done, running.total)).into(),
        );
    } else if let Some(finished) = &state.finished {
        body.push(widget::text::body(finished.message()).into());
        if finished.report.is_some() {
            body.push(
                widget::button::text("Open the report")
                    .on_press(send(FolderAction::OpenReport))
                    .into(),
            );
        }
    }
    let button: Element<'a, Message> = if running {
        widget::button::custom(widget::text::body("Stop").width(Length::Fill).center())
            .class(cosmic::theme::Button::Destructive)
            .width(Length::Fill)
            .height(32)
            .on_press(send(FolderAction::Stop))
            .into()
    } else {
        widget::button::custom(widget::text::body("Export").width(Length::Fill).center())
            .class(cosmic::theme::Button::Suggested)
            .width(Length::Fill)
            .height(32)
            .on_press_maybe(state.ready().then_some(send(FolderAction::Export)))
            .into()
    };
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
        widget::divider::horizontal::default().into(),
        button,
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_keep_their_place_and_never_replace_a_file() {
        let root = std::env::temp_dir().join(format!("amipro-folder-{}", std::process::id()));
        let from = root.join("from");
        let to = root.join("to");
        std::fs::create_dir_all(from.join("Client")).unwrap();
        std::fs::create_dir_all(to.join("Client")).unwrap();
        std::fs::write(from.join("A.SAM"), b"x").unwrap();
        std::fs::write(from.join("Client/B.sam"), b"x").unwrap();
        std::fs::write(from.join("notes.txt"), b"x").unwrap();
        assert_eq!(sam_files(&from, true).len(), 2);
        assert_eq!(sam_files(&from, false).len(), 1);
        let b = from.join("Client/B.sam");
        assert_eq!(
            target(&from, &to, &b, Format::Docx),
            to.join("Client/B.docx")
        );
        std::fs::write(to.join("Client/B.docx"), b"there").unwrap();
        assert_eq!(
            target(&from, &to, &b, Format::Docx),
            to.join("Client/B (2).docx")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_message_counts_what_happened() {
        let finished = Finished {
            exported: 3,
            with_losses: 1,
            not_read: 2,
            stopped: false,
            report: None,
        };
        assert_eq!(
            finished.message(),
            "Exported 3 files, 1 with something the format cannot hold; 2 could not be read"
        );
    }
}
