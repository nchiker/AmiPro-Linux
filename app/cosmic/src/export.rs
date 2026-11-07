//! Export (`UI_SPEC.md` §30; decisions EX1–EX10): File › Export ▸ writes
//! the document as a Word Document, OpenDocument Text, Rich Text, Plain
//! Text, or Markdown (MD1; its pictures are written beside it) through
//! `amipro-exchange`'s filters, as Ami Pro's Save As writes
//! another type through its filters: a copy, the tab keeping its own file.
//! The exchange document is built off the interface's thread from the
//! session's pages (which place the frames), with the pictures converted
//! here; what the format cannot hold is listed before anything is written
//! (EX7).

use std::path::{Path, PathBuf};

use amipro_document::{Document, FieldContext, Picture};
use amipro_exchange::build::{BuildOptions, PictureSink, SunkPicture};
use amipro_exchange::{ExchangeDocument, Format};
use amipro_layout::DocumentLayout;

use crate::file_workflow::DocumentLocation;
use crate::pictures::Pictures;

/// The pictures of a document being exported, decoded on the export's own
/// thread.
pub(crate) struct ExportPictures {
    pictures: Pictures,
    folder: Option<PathBuf>,
}

impl ExportPictures {
    pub(crate) fn new(folder: Option<PathBuf>) -> Self {
        Self {
            pictures: Pictures::default(),
            folder,
        }
    }
}

impl PictureSink for ExportPictures {
    fn picture(&mut self, document: &Document, picture: &Picture) -> Option<SunkPicture> {
        self.pictures
            .exported(document, picture, self.folder.as_deref())
    }
}

/// A document to export, for another thread.
pub(crate) struct ExportJob {
    pub document: Document,
    pub layout: DocumentLayout,
    pub folder: Option<PathBuf>,
    pub fields: FieldContext,
    pub title: String,
    pub format: Format,
    pub path: PathBuf,
    /// On a master document, its set in order (MS10): each file is built
    /// with its numbers in the set and the files joined, each a section;
    /// empty for one document.
    pub set: Vec<crate::print::SetPart>,
}

/// An export made in memory: its bytes, and what the format cannot hold
/// (the loss list's lines), before anything is written.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Prepared {
    pub path: PathBuf,
    pub format: Format,
    pub bytes: Vec<u8>,
    pub losses: Vec<String>,
    pub pages: usize,
    /// Files written beside the export (Markdown's pictures): names
    /// relative to its folder, and bytes.
    pub extras: Extras,
    /// The names, of the export and its extras, that already exist and
    /// would be replaced; the loss dialog lists them (Markdown only, for
    /// the other formats' dialogs are unchanged).
    pub replaces: Vec<String>,
}

/// Files written beside an export: name relative to its folder, bytes.
pub(crate) type Extras = Vec<(String, std::sync::Arc<[u8]>)>;

/// The loss list's lines: what the document uses that `format` lacks,
/// then what it never holds.
fn loss_lines(features: &amipro_exchange::losses::Features, format: Format) -> Vec<String> {
    let mut lines: Vec<String> = amipro_exchange::losses::losses(features, format)
        .iter()
        .map(amipro_exchange::ExportLoss::message)
        .collect();
    lines.extend(
        amipro_exchange::losses::always_lost(format)
            .iter()
            .map(|line| (*line).to_owned()),
    );
    lines
}

/// Fills `prepared`'s extras and replaced names for `format`.
fn finish(
    document: &ExchangeDocument,
    format: Format,
    path: PathBuf,
    losses: Vec<String>,
    pages: usize,
) -> Prepared {
    let stem = path.file_stem().map_or_else(
        || "picture".to_owned(),
        |s| s.to_string_lossy().into_owned(),
    );
    let (bytes, extras) = bytes(document, format, &stem);
    let replaces = if format == Format::Markdown {
        let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
        std::iter::once(path.file_name().map(|n| n.to_string_lossy().into_owned()))
            .flatten()
            .chain(extras.iter().map(|(name, _)| name.clone()))
            .filter(|name| folder.join(name).exists())
            .collect()
    } else {
        Vec::new()
    };
    Prepared {
        bytes,
        losses,
        path,
        format,
        pages,
        extras,
        replaces,
    }
}

impl ExportJob {
    /// Builds the exchange document and writes the format into memory.
    pub(crate) fn run(self) -> Result<Prepared, String> {
        if !self.set.is_empty() {
            return self.run_set();
        }
        let options = BuildOptions {
            layout: Some(&self.layout),
            fields: self.fields,
            title: self.title,
        };
        let mut pictures = ExportPictures::new(self.folder);
        let built = amipro_exchange::build(&self.document, &options, &mut pictures);
        let losses = loss_lines(&built.features, self.format);
        Ok(finish(
            &built.document,
            self.format,
            self.path,
            losses,
            self.layout.pages.len(),
        ))
    }
}

impl ExportJob {
    /// The master document's set as one document (MS10): the master's
    /// properties, each file built with its own pages, folder, and
    /// pictures, then joined.
    fn run_set(self) -> Result<Prepared, String> {
        let master = amipro_exchange::build(
            &self.document,
            &BuildOptions {
                layout: Some(&self.layout),
                fields: self.fields.clone(),
                title: self.title.clone(),
            },
            &mut amipro_exchange::build::NoPictures,
        );
        let mut files = Vec::with_capacity(self.set.len());
        let mut pages = 0;
        for part in &self.set {
            let mut fields = self.fields.clone();
            fields.file_name.clone_from(&part.title);
            if let Some(folder) = part.folder.as_ref() {
                fields.directory = folder.display().to_string();
            }
            let options = BuildOptions {
                layout: Some(&part.layout),
                fields,
                title: part.title.clone(),
            };
            let mut pictures = ExportPictures::new(part.folder.clone());
            let built = amipro_exchange::build(&part.document, &options, &mut pictures);
            pages += part.layout.pages.len();
            files.push(amipro_exchange::set::SetFile {
                document: built.document,
                features: built.features,
            });
        }
        let mut properties = master.document.properties;
        properties.pages = u32::try_from(pages).unwrap_or(u32::MAX);
        let (document, features) = amipro_exchange::set::join(files, properties);
        let losses = loss_lines(&features, self.format);
        Ok(finish(&document, self.format, self.path, losses, pages))
    }
}

/// The exchange document in `format`, its zips deflated, and the files to
/// write beside it (Markdown's pictures, named after `stem`).
pub(crate) fn bytes(document: &ExchangeDocument, format: Format, stem: &str) -> (Vec<u8>, Extras) {
    let deflate: &dyn Fn(&[u8]) -> Vec<u8> = &deflate;
    let plain = |bytes| (bytes, Vec::new());
    match format {
        Format::Docx => plain(amipro_exchange::docx::write(document, Some(deflate))),
        Format::Odt => plain(amipro_exchange::odt::write(document, Some(deflate))),
        Format::Rtf => plain(amipro_exchange::rtf::write(document)),
        Format::Text => plain(amipro_exchange::text::write(document)),
        Format::Markdown => {
            let written = amipro_exchange::markdown::write(document, stem);
            (written.text.into_bytes(), written.pictures)
        }
    }
}

/// Raw deflate (RFC 1951), as zip entries hold it.
pub(crate) fn deflate(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    if encoder.write_all(bytes).is_err() {
        return Vec::new();
    }
    encoder.finish().unwrap_or_default()
}

/// What an export wrote, for the status bar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Exported {
    pub path: PathBuf,
    pub format: Format,
}

impl Exported {
    /// The status bar's report.
    pub(crate) fn message(&self) -> String {
        let name = self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        format!("Exported {name} ({})", self.format.name())
    }
}

/// Writes a prepared export: to a temporary file renamed into place, as
/// saves are.
pub(crate) fn write(prepared: &Prepared) -> Result<Exported, String> {
    crate::file_workflow::write_file_replacing(&prepared.path, &prepared.bytes)?;
    if let Some(folder) = prepared.path.parent() {
        for (name, bytes) in &prepared.extras {
            crate::file_workflow::write_file_replacing(&folder.join(name), bytes)?;
        }
    }
    Ok(Exported {
        path: prepared.path.clone(),
        format: prepared.format,
    })
}

/// The exported file's name: the document's file name with the format's
/// extension, else its title, else Untitled.
pub(crate) fn file_name(location: &DocumentLocation, title: &str, format: Format) -> String {
    let stem = location
        .path()
        .and_then(|path| path.file_stem())
        .map(|stem| stem.to_string_lossy().into_owned());
    let stem = stem.unwrap_or_else(|| {
        let title = title.trim();
        if title.is_empty() {
            "Untitled".to_owned()
        } else {
            title.to_owned()
        }
    });
    format!("{stem}.{}", format.extension())
}

/// The title the exported file's properties carry: the document's file
/// name without its extension, else its title.
pub(crate) fn title(location: &DocumentLocation, title: &str) -> String {
    let name = file_name(location, title, Format::Text);
    name.trim_end_matches(".txt").to_owned()
}

/// The save dialog's filter for `format`.
pub(crate) fn filter_name(format: Format) -> String {
    format!("{} (.{})", format.name(), format.extension())
}

/// Opens `path` in the desktop's program for its type (the status bar's
/// Open, EX10), through the OpenURI portal.
pub(crate) async fn open_file(path: PathBuf) -> Result<(), String> {
    use cosmic::dialog::ashpd::desktop::open_uri::OpenFileRequest;
    let file = std::fs::File::open(&path).map_err(|error| error.to_string())?;
    OpenFileRequest::default()
        .ask(false)
        .send_file(&file)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Whether a path's extension is `format`'s, ignoring case.
pub(crate) fn has_extension(path: &Path, format: Format) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(format.extension()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_named_after_the_documents() {
        let sam = DocumentLocation::LegacySource(PathBuf::from("/a/README31.SAM"));
        assert_eq!(file_name(&sam, "x", Format::Docx), "README31.docx");
        assert_eq!(
            file_name(&DocumentLocation::Untitled, "", Format::Odt),
            "Untitled.odt"
        );
        assert_eq!(title(&sam, "x"), "README31");
    }

    /// Exports a SAM, or every SAM below a folder, as the app does (its
    /// fonts, its picture conversion) into `AMIPRO_EXPORT_OUT` in all four
    /// formats; with `AMIPRO_EXPORT_PDF`, the app's PDF beside them, for
    /// comparing with LibreOffice's conversion. A folder's files are named
    /// by their index (the corpora's names stay private).
    #[test]
    #[ignore = "needs AMIPRO_EXPORT_SAM and AMIPRO_EXPORT_OUT"]
    fn export_files() {
        let (Some(input), Some(out)) = (
            std::env::var_os("AMIPRO_EXPORT_SAM"),
            std::env::var_os("AMIPRO_EXPORT_OUT"),
        ) else {
            eprintln!("AMIPRO_EXPORT_SAM or AMIPRO_EXPORT_OUT is not set; skipping");
            return;
        };
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|styles| std::env::split_paths(&styles).collect())
            .unwrap_or_default();
        let input = PathBuf::from(input);
        let mut files = Vec::new();
        collect(&input, &mut files);
        let many = input.is_dir();
        let pdf = std::env::var_os("AMIPRO_EXPORT_PDF").is_some();
        let (mut exported, mut lossless) = (0, [0usize; Format::ALL.len()]);
        let started = std::time::Instant::now();
        for (index, file) in files.iter().enumerate() {
            let Ok(import) = amipro_formats::import_sam_path_with_style_directories(file, &styles)
            else {
                continue;
            };
            let document = import.document;
            let mut pages = crate::page_layout::SessionPages::default();
            pages.update(&document);
            let layout = pages.layout().clone();
            let stem = if many {
                format!("{index:04}")
            } else {
                file.file_stem().unwrap().to_string_lossy().into_owned()
            };
            for (slot, format) in Format::ALL.into_iter().enumerate() {
                let prepared = ExportJob {
                    document: document.clone(),
                    layout: layout.clone(),
                    folder: file.parent().map(Path::to_path_buf),
                    fields: FieldContext::default(),
                    title: stem.clone(),
                    format,
                    path: out.join(format!("{stem}.{}", format.extension())),
                    set: Vec::new(),
                }
                .run()
                .unwrap();
                if prepared.losses.is_empty() {
                    lossless[slot] += 1;
                } else if !many {
                    for loss in &prepared.losses {
                        eprintln!("{}: {loss}", format.name());
                    }
                }
                write(&prepared).unwrap();
            }
            if pdf {
                let pictures = Pictures::default();
                let bytes = crate::print::document_pdf(
                    &document,
                    &layout,
                    Some((&pictures, file.parent())),
                    &crate::print::PdfOptions::default(),
                )
                .unwrap();
                std::fs::write(out.join(format!("{stem}.app.pdf")), bytes).unwrap();
            }
            exported += 1;
        }
        eprintln!(
            "SAM files {}; exported {exported}; nothing lost: DOCX {}, ODT {}, RTF {}, text {}, Markdown {}; {:.1} s",
            files.len(),
            lossless[0],
            lossless[1],
            lossless[2],
            lossless[3],
            lossless[4],
            started.elapsed().as_secs_f32()
        );
    }

    /// Release timings of an export (`AMIPRO_PERF_SAM`): the exchange
    /// document built and each format written, pictures converted (the
    /// export's background work), median and worst of five.
    #[test]
    #[ignore = "release timing; needs AMIPRO_PERF_SAM"]
    fn export_performance_baseline() {
        let Some(path) = std::env::var_os("AMIPRO_PERF_SAM") else {
            eprintln!("AMIPRO_PERF_SAM is not set; skipping");
            return;
        };
        let path = PathBuf::from(path);
        let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|styles| std::env::split_paths(&styles).collect())
            .unwrap_or_default();
        let document = amipro_formats::import_sam_path_with_style_directories(&path, &styles)
            .unwrap()
            .document;
        let mut pages = crate::page_layout::SessionPages::default();
        pages.update(&document);
        let layout = pages.layout().clone();
        for format in Format::ALL {
            let mut times = Vec::new();
            let mut size = 0;
            for _ in 0..5 {
                let started = std::time::Instant::now();
                let prepared = ExportJob {
                    document: document.clone(),
                    layout: layout.clone(),
                    folder: path.parent().map(Path::to_path_buf),
                    fields: FieldContext::default(),
                    title: "Timing".into(),
                    format,
                    path: PathBuf::from("unused"),
                    set: Vec::new(),
                }
                .run()
                .unwrap();
                times.push(started.elapsed().as_secs_f64() * 1_000.0);
                size = prepared.bytes.len();
            }
            times.sort_by(f64::total_cmp);
            eprintln!(
                "{}: {} pages, {size} bytes, median {:.1} ms, worst {:.1} ms",
                format.name(),
                layout.pages.len(),
                times[2],
                times[4]
            );
        }
    }

    fn collect(path: &Path, out: &mut Vec<PathBuf>) {
        if path.is_dir() {
            let mut entries: Vec<_> = std::fs::read_dir(path)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .collect();
            entries.sort();
            for entry in entries {
                collect(&entry, out);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
        {
            out.push(path.to_owned());
        }
    }

    #[test]
    fn deflate_reads_back() {
        use std::io::Read as _;
        let text = b"abcabcabcabcabcabcabcabc".repeat(20);
        let packed = deflate(&text);
        assert!(packed.len() < text.len());
        let mut unpacked = Vec::new();
        flate2::read::DeflateDecoder::new(&packed[..])
            .read_to_end(&mut unpacked)
            .unwrap();
        assert_eq!(unpacked, text);
    }
}
