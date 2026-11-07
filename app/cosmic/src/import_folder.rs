//! File › Import Folder… (`UI_SPEC.md` §31, IM11): a whole folder of other
//! programs' documents read at once into AmiPro-Linux documents. A panel in
//! the rail chooses where from (with its subfolders), where to, and the
//! style option; the import runs on a thread of its own, file by file, with
//! Stop. The originals are only read; each document goes to the same
//! relative place under To as an `.amipro` file, never over an existing
//! file, and an `Import report.txt` there lists what each could not keep.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use amipro_exchange::import::{ImportOptions, StyleOption};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::export_folder::unused;
use crate::import::{self, ImportJob, Purpose, Source};

/// The panel's state.
#[derive(Debug)]
pub struct ImportFolder {
    pub from: Option<PathBuf>,
    pub subfolders: bool,
    pub to: Option<PathBuf>,
    pub styles: StyleOption,
    pub running: Option<Running>,
    pub finished: Option<Finished>,
}

impl ImportFolder {
    pub fn new(from: Option<PathBuf>, to: Option<PathBuf>, styles: StyleOption) -> Self {
        Self {
            from,
            subfolders: true,
            to,
            styles,
            running: None,
            finished: None,
        }
    }

    /// Whether Import can start: both folders, nothing running.
    pub fn ready(&self) -> bool {
        self.from.is_some() && self.to.is_some() && self.running.is_none()
    }
}

/// An import under way.
#[derive(Debug)]
pub struct Running {
    pub done: usize,
    pub total: usize,
    pub stop: Arc<AtomicBool>,
}

/// What a folder's import did.
#[derive(Clone, Debug, PartialEq)]
pub struct Finished {
    pub imported: usize,
    /// Imported, with something the document could not keep as it was.
    pub with_findings: usize,
    pub not_read: usize,
    pub stopped: bool,
    pub report: Option<PathBuf>,
}

impl Finished {
    pub fn message(&self) -> String {
        let mut text = format!(
            "Imported {} {}",
            self.imported,
            if self.imported == 1 { "file" } else { "files" }
        );
        if self.with_findings > 0 {
            text.push_str(&format!(
                ", {} with something different",
                self.with_findings
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
    Styles(StyleOption),
    Import,
    Stop,
    OpenReport,
    Back,
    Close,
}

/// What the import's thread reports.
#[derive(Clone, Debug)]
pub enum Progress {
    Started { total: usize },
    Done { done: usize },
    Finished(Result<Finished, String>),
}

/// The rail's width while the panel shows.
pub const WIDTH: f32 = 300.0;

/// The documents File › Open would import in `from`, and in its subfolders
/// when `subfolders`, in name order: named as one of the formats read and
/// recognised by their contents.
pub fn document_files(from: &Path, subfolders: bool) -> Vec<PathBuf> {
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
            } else if import::is_import_name(&path) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(from, subfolders, &mut out);
    out
}

/// Where `file` (under `from`) goes under `to`: the same relative place as
/// an `.amipro` document, never over a file there.
pub fn target(from: &Path, to: &Path, file: &Path) -> PathBuf {
    let relative = file.strip_prefix(from).unwrap_or(file);
    let mut path = to.join(relative);
    path.set_extension("amipro");
    unused(path)
}

/// A folder's import, for its own thread.
pub struct FolderJob {
    pub from: PathBuf,
    pub subfolders: bool,
    pub to: PathBuf,
    pub options: ImportOptions,
    pub stop: Arc<AtomicBool>,
}

impl FolderJob {
    /// Imports every document, reporting as it goes, and writes the report.
    pub fn run(self, progress: &dyn Fn(Progress)) -> Result<Finished, String> {
        let files = document_files(&self.from, self.subfolders);
        progress(Progress::Started { total: files.len() });
        std::fs::create_dir_all(&self.to).map_err(|error| error.to_string())?;
        let mut report = format!(
            "Import report: AmiPro-Linux documents (.amipro)\nFrom: {}\nTo: {}\n\n",
            self.from.display(),
            self.to.display()
        );
        let mut finished = Finished {
            imported: 0,
            with_findings: 0,
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
            match self.import_one(file) {
                Ok((written, findings)) => {
                    finished.imported += 1;
                    let name = written.strip_prefix(&self.to).unwrap_or(&written).display();
                    if findings.is_empty() {
                        report.push_str(&format!("{relative} → {name}: nothing different\n"));
                    } else {
                        finished.with_findings += 1;
                        report.push_str(&format!("{relative} → {name}:\n"));
                        for finding in findings.iter() {
                            report.push_str(&format!(
                                "    • {}\n",
                                finding.kind.describe(finding.count)
                            ));
                        }
                    }
                }
                Err(error) => {
                    finished.not_read += 1;
                    report.push_str(&format!("{relative}: not imported: {error}\n"));
                }
            }
            progress(Progress::Done { done: index + 1 });
        }
        report.push_str(&format!("\n{}.\n", finished.message()));
        let path = unused(self.to.join("Import report.txt"));
        std::fs::write(&path, report).map_err(|error| error.to_string())?;
        finished.report = Some(path);
        Ok(finished)
    }

    /// One document: read, realised, and written as an `.amipro` document
    /// at its place under To.
    fn import_one(
        &self,
        file: &Path,
    ) -> Result<(PathBuf, amipro_exchange::import::Findings), String> {
        if import::source_format(file).is_none() {
            return Err("not a Word, OpenDocument, Rich Text, Markdown, or text file".into());
        }
        let imported = ImportJob {
            source: Source::File(file.to_path_buf()),
            options: self.options,
            purpose: Purpose::Open,
            base: crate::new_document(),
            into_text: false,
            now: crate::clock::ami_pro_now(),
        }
        .run()?;
        let path = target(&self.from, &self.to, file);
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        }
        let native = imported
            .document
            .encode_native()
            .map_err(|error| error.to_string())?;
        crate::file_workflow::write_new_file_safely(&path, native.as_bytes())
            .map_err(|error| error.to_string())?;
        Ok((path, imported.report.findings))
    }
}

fn send(action: FolderAction) -> Message {
    Message::ImportFolder(action)
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

pub fn panel<'a>(state: &ImportFolder) -> Element<'a, Message> {
    let running = state.running.is_some();
    let header = widget::row::with_children(vec![
        widget::text::heading("Import Folder").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(FolderAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Import Folder")
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
        widget::text::body("AmiPro-Linux documents (.amipro)").into(),
        widget::text::heading("Paragraph styles").into(),
    ];
    for (option, label) in [
        (StyleOption::Convert, "Convert styles"),
        (StyleOption::Apply, "Apply styles"),
        (StyleOption::Keep, "Keep style names"),
        (StyleOption::Ignore, "Ignore styles"),
    ] {
        body.push(
            widget::radio(
                widget::text::body(label),
                option,
                Some(state.styles),
                move |option| send(FolderAction::Styles(option)),
            )
            .into(),
        );
    }
    body.push(
        widget::text::caption(
            "Word Documents, OpenDocument Text, Rich Text, and text files are read; text files \
             as each line a paragraph. The documents are only read; nothing is written over a \
             file already there, and an Import report.txt lists what each could not keep.",
        )
        .into(),
    );
    if let Some(running) = &state.running {
        body.push(
            widget::text::body(format!("Importing {} of {}…", running.done, running.total)).into(),
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
        widget::button::custom(widget::text::body("Import").width(Length::Fill).center())
            .class(cosmic::theme::Button::Suggested)
            .width(Length::Fill)
            .height(32)
            .on_press_maybe(state.ready().then_some(send(FolderAction::Import)))
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
    fn a_folder_imports_into_amipro_documents_with_a_report() {
        let root =
            std::env::temp_dir().join(format!("amipro-import-folder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let from = root.join("from");
        let to = root.join("to");
        std::fs::create_dir_all(from.join("Client")).unwrap();
        std::fs::write(
            from.join("Letter.rtf"),
            br"{\rtf1\ansi{\stylesheet{\s0 Normal;}}{\s0 Dear reader\par}}",
        )
        .unwrap();
        std::fs::write(from.join("Client").join("notes.txt"), b"one\r\ntwo\r\n").unwrap();
        std::fs::write(from.join("Broken.docx"), b"PK\x03\x04 not a package").unwrap();
        std::fs::write(from.join("picture.png"), b"\x89PNG").unwrap();
        // Never over a file already there.
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(to.join("Letter.amipro"), b"kept").unwrap();
        let originals: Vec<Vec<u8>> = document_files(&from, true)
            .iter()
            .map(|file| std::fs::read(file).unwrap())
            .collect();
        assert_eq!(document_files(&from, true).len(), 3);
        assert_eq!(document_files(&from, false).len(), 2);
        let counted = std::sync::Mutex::new(Vec::new());
        let finished = FolderJob {
            from: from.clone(),
            subfolders: true,
            to: to.clone(),
            options: ImportOptions::default(),
            stop: Arc::new(AtomicBool::new(false)),
        }
        .run(&|progress| counted.lock().unwrap().push(format!("{progress:?}")))
        .unwrap();
        assert_eq!(finished.imported, 2);
        assert_eq!(finished.not_read, 1);
        assert_eq!(std::fs::read(to.join("Letter.amipro")).unwrap(), b"kept");
        let letter = amipro_document::Document::load_native(to.join("Letter (2).amipro")).unwrap();
        assert_eq!(letter.plain_text(), "Dear reader");
        let notes =
            amipro_document::Document::load_native(to.join("Client").join("notes.amipro")).unwrap();
        assert_eq!(notes.plain_text(), "one\ntwo");
        let report = std::fs::read_to_string(finished.report.unwrap()).unwrap();
        assert!(
            report.contains("Letter.rtf → Letter (2).amipro: nothing different"),
            "{report}"
        );
        assert!(report.contains("Broken.docx: not imported: "), "{report}");
        assert!(
            report.ends_with("Imported 2 files; 1 could not be read.\n"),
            "{report}"
        );
        let after: Vec<Vec<u8>> = document_files(&from, true)
            .iter()
            .map(|file| std::fs::read(file).unwrap())
            .collect();
        assert_eq!(after, originals);
        let counted = counted.into_inner().unwrap();
        assert_eq!(counted.first().unwrap(), "Started { total: 3 }");
        assert_eq!(counted.len(), 4);
        let _ = std::fs::remove_dir_all(root);
    }
}
