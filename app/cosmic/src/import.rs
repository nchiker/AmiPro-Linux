//! Opening and inserting other programs' documents (`docs/Design/UI_SPEC.md`
//! §31, IM1–IM12): Word Documents, OpenDocument Text, Rich Text, and plain
//! text, recognised by their contents, read into the exchange document and
//! realised as an Ami Pro document off the interface's thread, after Ami
//! Pro's Import Options (the style choice, or for text its ASCII options).
//! What the document cannot hold comes back as findings for the Import
//! Report.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use amipro_document::Document;
use amipro_editing::TextPosition;
use amipro_exchange::import::{
    self, Characters, FindingKind, Findings, ImportOptions, LineEnds, SourceFormat, StyleOption,
};
use amipro_exchange::realise::{Place, RealiseOptions, realise};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;

/// The file name patterns File › Open and Insert › File… offer for the
/// formats read (besides SAM and `.amipro`).
pub(crate) const PATTERNS: [(&str, &[&str]); 5] = [
    ("Word Documents", &["docx", "docm", "dotx"]),
    ("OpenDocument Text", &["odt", "fodt", "ott"]),
    ("Rich Text", &["rtf"]),
    ("Plain Text", &["txt", "text", "asc"]),
    ("Markdown", &["md", "markdown", "mdown"]),
];

/// What a document read from a file is for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Purpose {
    /// File › Open: a new tab.
    Open,
    /// Insert › File… (IM3) or Paste (IM12): into the tab `session` at
    /// `at`, its Where placed frames from page `first_page` on.
    #[cfg_attr(not(feature = "page-surface"), allow(dead_code))]
    Insert {
        session: u64,
        at: TextPosition,
        first_page: u32,
        paste: bool,
    },
}

/// The format of the file at `path`, by its contents and then its name;
/// `None` when it is none of the formats read.
pub(crate) fn source_format(path: &Path) -> Option<SourceFormat> {
    let mut head = Vec::with_capacity(4_096);
    std::fs::File::open(path)
        .ok()?
        .take(4_096)
        .read_to_end(&mut head)
        .ok()?;
    import::detect(&head, extension(path).as_deref())
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

/// Whether `path` is named as one of the formats read (Import Folder…).
#[cfg(feature = "page-surface")]
pub(crate) fn is_import_name(path: &Path) -> bool {
    let extension = extension(path);
    PATTERNS
        .iter()
        .flat_map(|(_, patterns)| patterns.iter())
        .any(|pattern| extension.as_deref() == Some(*pattern))
}

/// Whether `path` names a document File › Open reads (dropped files).
#[cfg(feature = "page-surface")]
pub(crate) fn is_document(path: &Path) -> bool {
    matches!(extension(path).as_deref(), Some("sam" | "amipro"))
        || PATTERNS
            .iter()
            .flat_map(|(_, patterns)| patterns.iter())
            .any(|pattern| extension(path).as_deref() == Some(*pattern))
}

/// The clipboard's rich text, which Paste reads with Apply styles (IM12).
#[cfg(feature = "page-surface")]
#[derive(Clone, Debug)]
pub(crate) struct ClipboardRtf(pub Vec<u8>);

/// The types other programs offer rich text as.
#[cfg(feature = "page-surface")]
const RTF_TYPES: [&str; 3] = ["text/rtf", "application/rtf", "text/richtext"];

#[cfg(feature = "page-surface")]
impl TryFrom<(Vec<u8>, String)> for ClipboardRtf {
    type Error = ();

    fn try_from((bytes, mime): (Vec<u8>, String)) -> Result<Self, ()> {
        (RTF_TYPES.contains(&mime.as_str()) && bytes.starts_with(b"{\\rtf"))
            .then_some(Self(bytes))
            .ok_or(())
    }
}

#[cfg(feature = "page-surface")]
impl cosmic::iced::clipboard::mime::AllowedMimeTypes for ClipboardRtf {
    fn allowed() -> std::borrow::Cow<'static, [String]> {
        RTF_TYPES.iter().map(|&mime| mime.to_owned()).collect()
    }
}

/// Paste's options: Ami Pro pastes rich text with Apply styles.
#[cfg(feature = "page-surface")]
pub(crate) fn paste_options() -> ImportOptions {
    ImportOptions {
        styles: StyleOption::Apply,
        ..ImportOptions::default()
    }
}

/// The Import Options dialog (Ami Pro's Import Options and ASCII Options),
/// before the file is read.
#[derive(Clone, Debug)]
pub(crate) struct OptionsDialog {
    pub source: Source,
    pub format: SourceFormat,
    pub purpose: Purpose,
    pub options: ImportOptions,
}

/// Where the document's bytes are.
#[derive(Clone, Debug)]
pub(crate) enum Source {
    File(PathBuf),
    /// The clipboard's rich text (Paste).
    #[cfg_attr(not(feature = "page-surface"), allow(dead_code))]
    Clipboard(Vec<u8>),
}

impl Source {
    pub(crate) fn name(&self) -> String {
        match self {
            Self::File(path) => path.file_name().map_or_else(
                || path.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            ),
            Self::Clipboard(_) => "the clipboard".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OptionsAction {
    Styles(StyleOption),
    LineEnds(LineEnds),
    KeepStyleNames(bool),
    Characters(usize),
    Open,
    Cancel,
}

/// The character sets the dialog offers, detected first.
const CHARACTERS: [(Characters, &str); 5] = [
    (Characters::Detect, "Detected"),
    (Characters::Utf8, "Unicode (UTF-8)"),
    (Characters::WindowsAnsi, "Windows ANSI"),
    (Characters::PcAscii, "PC-ASCII"),
    (Characters::Ascii7, "7-bit ASCII"),
];

impl OptionsDialog {
    pub(crate) fn new(
        source: Source,
        format: SourceFormat,
        purpose: Purpose,
        options: ImportOptions,
    ) -> Self {
        Self {
            source,
            format,
            purpose,
            options,
        }
    }

    /// Applies a choice (Open and Cancel are the app's).
    pub(crate) fn act(&mut self, action: OptionsAction) {
        match action {
            OptionsAction::Styles(styles) => self.options.styles = styles,
            OptionsAction::LineEnds(line_ends) => self.options.text.line_ends = line_ends,
            OptionsAction::KeepStyleNames(keep) => self.options.text.keep_style_names = keep,
            OptionsAction::Characters(index) => {
                if let Some((characters, _)) = CHARACTERS.get(index) {
                    self.options.text.characters = *characters;
                }
            }
            OptionsAction::Open | OptionsAction::Cancel => {}
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let action = |action| Message::ImportOptions(action);
        let row =
            |control: Element<'static, Message>, note: &'static str| -> Element<'static, Message> {
                widget::row::with_children(vec![
                    widget::container(control)
                        .width(Length::Fixed(210.0))
                        .into(),
                    widget::text::caption(note).into(),
                ])
                .spacing(12)
                .align_y(cosmic::iced::Alignment::Center)
                .into()
            };
        let mut controls: Vec<Element<'_, Message>> = Vec::new();
        if self.format.has_styles() {
            controls.push(widget::text::heading("Paragraph styles").into());
            let chosen = Some(self.options.styles);
            for (option, label, note) in [
                (
                    StyleOption::Convert,
                    "Convert styles",
                    "the file's styles, as the file defines them",
                ),
                (
                    StyleOption::Apply,
                    "Apply styles",
                    "Body Text, each style's look kept as formatting",
                ),
                (
                    StyleOption::Keep,
                    "Keep style names",
                    "the names, defined by this document's styles",
                ),
                (
                    StyleOption::Ignore,
                    "Ignore styles",
                    "Body Text with the file's own formatting only",
                ),
            ] {
                controls.push(row(
                    widget::radio(label, option, chosen, move |option| {
                        action(OptionsAction::Styles(option))
                    })
                    .into(),
                    note,
                ));
            }
        } else {
            controls.push(widget::text::heading("Line ends").into());
            let chosen = Some(self.options.text.line_ends);
            for (option, label, note) in [
                (
                    LineEnds::Paragraphs,
                    "Each line is a paragraph",
                    "CR/LF at paragraph ends only",
                ),
                (
                    LineEnds::Joined,
                    "Join lines into paragraphs",
                    "CR/LF at lines; a blank line ends one",
                ),
            ] {
                controls.push(row(
                    widget::radio(label, option, chosen, move |option| {
                        action(OptionsAction::LineEnds(option))
                    })
                    .into(),
                    note,
                ));
            }
            controls.push(row(
                widget::checkbox(self.options.text.keep_style_names)
                    .label("Keep style names")
                    .on_toggle(move |keep| action(OptionsAction::KeepStyleNames(keep)))
                    .into(),
                "@Style@ at a line's start",
            ));
            let selected = CHARACTERS
                .iter()
                .position(|(characters, _)| *characters == self.options.text.characters)
                .unwrap_or(0);
            controls.push(
                widget::row::with_children(vec![
                    widget::container(widget::text::body("Characters"))
                        .width(Length::Fixed(210.0))
                        .into(),
                    widget::dropdown(&CHARACTER_LABELS, Some(selected), move |index| {
                        action(OptionsAction::Characters(index))
                    })
                    .width(Length::Fixed(200.0))
                    .into(),
                ])
                .spacing(12)
                .align_y(cosmic::iced::Alignment::Center)
                .into(),
            );
        }
        let verb = match self.purpose {
            Purpose::Open => "Open",
            Purpose::Insert { paste: true, .. } => "Paste",
            Purpose::Insert { .. } => "Insert",
        };
        widget::dialog()
            .title(format!("Import Options — {}", self.source.name()))
            .body(format!("{} as an Ami Pro document.", self.format.name()))
            .control(
                widget::column::with_children(controls)
                    .spacing(8)
                    .width(Length::Fill),
            )
            .primary_action(widget::button::suggested(verb).on_press(action(OptionsAction::Open)))
            .secondary_action(
                widget::button::standard("Cancel").on_press(action(OptionsAction::Cancel)),
            )
            .into()
    }
}

const CHARACTER_LABELS: [&str; 5] = [
    CHARACTERS[0].1,
    CHARACTERS[1].1,
    CHARACTERS[2].1,
    CHARACTERS[3].1,
    CHARACTERS[4].1,
];

/// A document to read and realise, for another thread.
pub(crate) struct ImportJob {
    pub source: Source,
    pub options: ImportOptions,
    pub purpose: Purpose,
    /// The new document's styles (Open), or the document inserted into.
    pub base: Document,
    /// Inserting into a frame's or a cell's text.
    pub into_text: bool,
    pub now: u32,
}

/// A document read from a file, realised.
#[derive(Clone, Debug)]
pub(crate) struct Imported {
    pub source: Source,
    pub purpose: Purpose,
    pub format: SourceFormat,
    pub document: Document,
    pub report: Report,
}

/// What the Import Report lists (IM8): the file, its format, the findings,
/// and where their markers ended up.
#[derive(Clone, Debug, Default)]
#[cfg_attr(not(feature = "page-surface"), allow(dead_code))]
pub(crate) struct Report {
    pub file: String,
    pub format: String,
    pub findings: Findings,
    pub markers: BTreeMap<u32, Place>,
    /// What was done: opened, inserted, or pasted.
    pub heading: String,
    /// For each finding, the place Go to shows next.
    pub next: Vec<usize>,
}

impl ImportJob {
    pub(crate) fn run(self) -> Result<Imported, String> {
        let (bytes, extension) = match &self.source {
            Source::File(path) => {
                let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
                if metadata.len() > import::MAX_FILE_SIZE as u64 {
                    return Err(import::ImportError::TooLarge.to_string());
                }
                (
                    std::fs::read(path).map_err(|error| error.to_string())?,
                    extension(path),
                )
            }
            Source::Clipboard(bytes) => (bytes.clone(), Some("rtf".to_owned())),
        };
        let read = import::read(&bytes, extension.as_deref(), &self.options, &inflate)
            .map_err(|error| error.to_string())?;
        let mut findings = read.findings.clone();
        let inserting = matches!(self.purpose, Purpose::Insert { .. });
        if inserting
            && read
                .document
                .sections
                .iter()
                .any(|section| !section.headers.is_empty() || !section.footers.is_empty())
        {
            findings.count(FindingKind::InsertedHeaders);
        }
        let title = match &self.source {
            Source::File(path) => path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default(),
            Source::Clipboard(_) => String::new(),
        };
        let options = RealiseOptions {
            styles: import::style_option(read.format, &self.options),
            base: &self.base,
            inserting,
            into_text: self.into_text,
            title: if read.document.properties.title.trim().is_empty() {
                title
            } else {
                read.document.properties.title.clone()
            },
            now: self.now,
        };
        let realised = crate::page_layout::with_private_shaper(|shaper| {
            realise(&read.document, &options, shaper, &mut findings)
        });
        realised
            .document
            .validate()
            .map_err(|error| format!("the document read is not valid: {error}"))?;
        Ok(Imported {
            report: Report {
                file: self.source.name(),
                format: read.format.name().to_owned(),
                findings,
                markers: realised.markers,
                heading: match self.purpose {
                    Purpose::Open => "Opened as an Ami Pro document.",
                    Purpose::Insert { paste: true, .. } => "Pasted at the caret.",
                    Purpose::Insert { .. } => "Inserted at the caret.",
                }
                .to_owned(),
                next: Vec::new(),
            },
            source: self.source,
            purpose: self.purpose,
            format: read.format,
            document: realised.document,
        })
    }
}

/// A package's part inflated (`flate2`), exactly `size` bytes or refused.
pub(crate) fn inflate(data: &[u8], size: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(size.min(64 << 20));
    flate2::read::DeflateDecoder::new(data)
        .take(size as u64 + 1)
        .read_to_end(&mut out)
        .ok()?;
    (out.len() == size).then_some(out)
}

/// Runs `work` on a thread of its own and waits for it.
pub(crate) async fn in_background<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = cosmic::iced::futures::channel::oneshot::channel();
    std::thread::Builder::new()
        .name("import".into())
        .spawn(move || {
            let _ = sender.send(work());
        })
        .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|_| "the import stopped unexpectedly".to_owned())?
}

/// The status bar's message after a file is read.
pub(crate) fn status(imported: &Imported) -> String {
    let what = match imported.purpose {
        Purpose::Open => format!(
            "Opened {} ({})",
            imported.report.file,
            imported.format.name()
        ),
        Purpose::Insert { paste: true, .. } => "Pasted".to_owned(),
        Purpose::Insert { .. } => format!("Inserted {}", imported.report.file),
    };
    match imported.report.findings.len() {
        0 => what,
        1 => format!("{what}; 1 thing is different (Import Report)"),
        count => format!("{what}; {count} things are different (Import Report)"),
    }
}

/// The options to preselect: the last choice (IM2), kept in the settings
/// as words.
pub(crate) fn options_from(styles: &str, line_ends: &str, keep_names: bool) -> ImportOptions {
    ImportOptions {
        styles: match styles {
            "apply" => StyleOption::Apply,
            "keep" => StyleOption::Keep,
            "ignore" => StyleOption::Ignore,
            _ => StyleOption::Convert,
        },
        text: import::TextOptions {
            line_ends: if line_ends == "joined" {
                LineEnds::Joined
            } else {
                LineEnds::Paragraphs
            },
            keep_style_names: keep_names,
            characters: Characters::Detect,
        },
    }
}

/// The words the settings keep for `options`.
pub(crate) fn words_of(options: &ImportOptions) -> (&'static str, &'static str, bool) {
    (
        match options.styles {
            StyleOption::Convert => "convert",
            StyleOption::Apply => "apply",
            StyleOption::Keep => "keep",
            StyleOption::Ignore => "ignore",
        },
        match options.text.line_ends {
            LineEnds::Paragraphs => "paragraphs",
            LineEnds::Joined => "joined",
        },
        options.text.keep_style_names,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary(name: &str, bytes: &[u8]) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("amipro-import-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn files_are_known_by_their_contents() {
        let rtf = temporary("letter.txt", br"{\rtf1\ansi Hello\par}");
        assert_eq!(source_format(&rtf), Some(SourceFormat::Rtf));
        let text = temporary("notes.txt", b"Just text\r\n");
        assert_eq!(source_format(&text), Some(SourceFormat::Text));
        let sam = temporary("old.sam", b"[ver]\r\n\t4\r\n");
        assert_eq!(source_format(&sam), None);
        for name in ["a.md", "b.markdown", "c.mdown", "D.MD"] {
            let markdown = temporary(name, b"# Title\n\nBody\n");
            assert_eq!(
                source_format(&markdown),
                Some(SourceFormat::Markdown),
                "{name}"
            );
        }
        assert!(PATTERNS.iter().any(|(name, _)| *name == "Markdown"));
        #[cfg(feature = "page-surface")]
        {
            assert!(is_document(Path::new("A.DOCX")));
            assert!(is_document(Path::new("b.sam")));
            assert!(is_document(Path::new("notes.md")));
            assert!(!is_document(Path::new("c.png")));
        }
    }

    #[test]
    fn a_rich_text_file_is_read_and_realised_off_the_thread() {
        let path = temporary(
            "styled.rtf",
            br"{\rtf1\ansi{\stylesheet{\s0 Normal;}{\s1 Heading 1;}}{\s1 Title\par}{\s0 Body \b bold\b0\par}}",
        );
        let job = ImportJob {
            source: Source::File(path),
            options: ImportOptions::default(),
            purpose: Purpose::Open,
            base: crate::new_document(),
            into_text: false,
            now: 0,
        };
        let imported = job.run().unwrap();
        assert_eq!(imported.format, SourceFormat::Rtf);
        let paragraphs = &imported.document.main_flow().paragraphs;
        assert_eq!(paragraphs[0].text, "Title");
        let heading = &imported.document.styles()[&paragraphs[0].style];
        assert_eq!(heading.name, "Heading 1");
        assert!(paragraphs[1].text.starts_with("Body bold"));
        assert_eq!(imported.report.file, "styled.rtf");
        assert!(status(&imported).starts_with("Opened styled.rtf (Rich Text)"));
    }

    #[test]
    fn a_markdown_file_is_read_realised_and_reported() {
        let path = temporary(
            "notes.md",
            b"# Title\n\nBody with **bold** and a [link](http://example.org).\n\n- one\n- two\n",
        );
        let job = ImportJob {
            source: Source::File(path),
            options: ImportOptions::default(),
            purpose: Purpose::Open,
            base: crate::new_document(),
            into_text: false,
            now: 0,
        };
        let imported = job.run().unwrap();
        assert_eq!(imported.format, SourceFormat::Markdown);
        let paragraphs = &imported.document.main_flow().paragraphs;
        assert_eq!(paragraphs[0].text, "Title");
        assert_eq!(
            imported.document.styles()[&paragraphs[0].style].name,
            "Heading 1"
        );
        assert!(paragraphs.iter().any(|p| p.text.contains("one")));
        assert!(status(&imported).starts_with("Opened notes.md (Markdown)"));
    }

    fn documents_under(path: &Path, out: &mut Vec<PathBuf>) {
        if path.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .collect();
            entries.sort();
            for entry in entries {
                documents_under(&entry, out);
            }
        } else if source_format(path).is_some()
            && !matches!(extension(path).as_deref(), Some("sam" | "amipro"))
        {
            out.push(path.to_owned());
        }
    }

    /// `AMIPRO_IMPORT_IN=FILE_OR_DIR AMIPRO_IMPORT_OUT=DIR`: each document
    /// imported as File › Open imports it (Convert styles) and its pages
    /// written as the app's PDF (`NNNN.app.pdf` for a folder, the corpora's
    /// names staying private), with aggregate counts and times.
    #[test]
    #[ignore = "needs AMIPRO_IMPORT_IN and AMIPRO_IMPORT_OUT"]
    fn import_files() {
        let (Some(input), Some(out)) = (
            std::env::var_os("AMIPRO_IMPORT_IN"),
            std::env::var_os("AMIPRO_IMPORT_OUT"),
        ) else {
            eprintln!("AMIPRO_IMPORT_IN or AMIPRO_IMPORT_OUT is not set; skipping");
            return;
        };
        let (input, out) = (PathBuf::from(input), PathBuf::from(out));
        std::fs::create_dir_all(&out).unwrap();
        let mut files = Vec::new();
        documents_under(&input, &mut files);
        let many = input.is_dir();
        let (mut imported, mut failed, mut pages) = (0, 0, 0);
        let mut times = Vec::new();
        for (index, file) in files.iter().enumerate() {
            let started = std::time::Instant::now();
            let result = ImportJob {
                source: Source::File(file.clone()),
                options: ImportOptions::default(),
                purpose: Purpose::Open,
                base: crate::new_document(),
                into_text: false,
                now: 0,
            }
            .run();
            let Ok(result) = result else {
                failed += 1;
                continue;
            };
            let mut laid_out = crate::page_layout::SessionPages::default();
            laid_out.update(&result.document);
            times.push(started.elapsed());
            imported += 1;
            pages += laid_out.layout().pages.len();
            #[cfg(feature = "page-surface")]
            {
                // A folder's documents by number, unless already numbered.
                let own = file.file_stem().unwrap().to_string_lossy().into_owned();
                let stem = if many && !own.bytes().all(|byte| byte.is_ascii_digit()) {
                    format!("{index:04}")
                } else {
                    own
                };
                let pictures = crate::pictures::Pictures::default();
                let bytes = crate::print::document_pdf(
                    &result.document,
                    laid_out.layout(),
                    Some((&pictures, file.parent())),
                    &crate::print::PdfOptions::default(),
                )
                .unwrap();
                std::fs::write(out.join(format!("{stem}.app.pdf")), bytes).unwrap();
            }
            #[cfg(not(feature = "page-surface"))]
            let _ = (index, many);
        }
        times.sort();
        eprintln!(
            "documents {}; imported {imported}; not read {failed}; pages {pages}; \
             read and laid out: median {:?}, slowest {:?}",
            files.len(),
            times.get(times.len() / 2).copied().unwrap_or_default(),
            times.last().copied().unwrap_or_default()
        );
    }

    /// Release timings of opening a document (`AMIPRO_PERF_IMPORT`): read
    /// and realised (the import's thread), then laid out, median and worst
    /// of five.
    #[test]
    #[ignore = "release timing; needs AMIPRO_PERF_IMPORT"]
    fn import_performance_baseline() {
        let Some(path) = std::env::var_os("AMIPRO_PERF_IMPORT") else {
            eprintln!("AMIPRO_PERF_IMPORT is not set; skipping");
            return;
        };
        let path = PathBuf::from(path);
        let (mut read, mut laid) = (Vec::new(), Vec::new());
        for _ in 0..5 {
            let started = std::time::Instant::now();
            let imported = ImportJob {
                source: Source::File(path.clone()),
                options: ImportOptions::default(),
                purpose: Purpose::Open,
                base: crate::new_document(),
                into_text: false,
                now: 0,
            }
            .run()
            .unwrap();
            read.push(started.elapsed());
            let started = std::time::Instant::now();
            let mut pages = crate::page_layout::SessionPages::default();
            pages.update(&imported.document);
            laid.push(started.elapsed());
        }
        read.sort();
        laid.sort();
        eprintln!(
            "read and realised: median {:?}, worst {:?}; laid out: median {:?}, worst {:?}",
            read[2], read[4], laid[2], laid[4]
        );
    }

    #[test]
    fn the_last_choice_is_kept_as_words() {
        let options = options_from("keep", "joined", true);
        assert_eq!(options.styles, StyleOption::Keep);
        assert_eq!(options.text.line_ends, LineEnds::Joined);
        assert_eq!(words_of(&options), ("keep", "joined", true));
        assert_eq!(options_from("", "", false), ImportOptions::default());
    }
}
