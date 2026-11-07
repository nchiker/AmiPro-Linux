use std::fmt;
use std::path::{Path, PathBuf};

use amipro_document::{Document, DocumentError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentLocation {
    Untitled,
    /// A file read from another format (an import, a data file that is not
    /// SAM): never written over; Save goes to Save As.
    LegacySource(PathBuf),
    /// An Ami Pro document read or written as SAM: Save writes over it,
    /// keeping the original as a backup the first time (user, 2026-11-04).
    Sam(PathBuf),
    Native(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveRequest {
    ChooseDestination,
    SaveTo(PathBuf),
    /// Save writes the Ami Pro document over its file (after the loss
    /// check), with a backup the first time.
    SaveSam(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveSelection {
    Ready(PathBuf),
    /// An existing file, replaced after confirmation (an Ami Pro document
    /// with a backup the first time).
    ConfirmOverwrite(PathBuf),
    /// A new Ami Pro document (`.SAM`).
    ReadySam(PathBuf),
    /// A Markdown file (MD1): written as an export, which lists what it
    /// cannot hold and what it would replace before writing.
    ExportMarkdown(PathBuf),
}

/// The format File › Save As writes (Phase 4, decision A).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveFormat {
    Native,
    Sam,
    /// Markdown, an exchange format (MD1); the tab keeps its own file.
    Markdown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveSafetyError {
    MissingFileName,
    LegacyTarget,
    /// Style sheets are written by Save as a Style Sheet.
    StyleSheetTarget,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OverwriteConfirmation {
    pending: Option<PathBuf>,
}

impl fmt::Display for SaveSafetyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFileName => formatter.write_str("a native file name is required"),
            Self::LegacyTarget => {
                formatter.write_str("SAM and STY files are not written as native documents")
            }
            Self::StyleSheetTarget => formatter
                .write_str("style sheets are written with Format › Styles › Save as a Style Sheet"),
        }
    }
}

impl std::error::Error for SaveSafetyError {}

impl DocumentLocation {
    /// The file the document is at, if any.
    #[cfg_attr(not(feature = "page-surface"), allow(dead_code))]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Untitled => None,
            Self::LegacySource(path) | Self::Sam(path) | Self::Native(path) => Some(path),
        }
    }

    #[cfg_attr(not(feature = "page-surface"), allow(dead_code))]
    /// The folder of the file the document came from or goes to, where its
    /// linked pictures are looked for by name (decision P2).
    pub fn folder(&self) -> Option<&Path> {
        match self {
            Self::Untitled => None,
            Self::LegacySource(path) | Self::Sam(path) | Self::Native(path) => path.parent(),
        }
    }

    pub fn save_request(&self) -> SaveRequest {
        match self {
            Self::Native(path) => SaveRequest::SaveTo(path.clone()),
            Self::Sam(path) => SaveRequest::SaveSam(path.clone()),
            Self::Untitled | Self::LegacySource(_) => SaveRequest::ChooseDestination,
        }
    }

    pub fn suggested_file_name(&self, document: &Document) -> String {
        let stem = match self {
            Self::LegacySource(path) | Self::Sam(path) | Self::Native(path) => path
                .file_stem()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty()),
            Self::Untitled => None,
        }
        .unwrap_or(document.title.as_str());
        let stem = if stem.trim().is_empty() || stem == "Untitled" {
            "untitled"
        } else {
            stem
        };
        format!("{stem}.amipro")
    }
}

impl OverwriteConfirmation {
    pub fn begin(&mut self, path: PathBuf) {
        self.pending = Some(path);
    }

    pub fn pending(&self) -> Option<&Path> {
        self.pending.as_deref()
    }

    pub fn confirm(&mut self) -> Option<PathBuf> {
        self.pending.take()
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }
}

pub fn native_destination(selected: &Path) -> Result<PathBuf, SaveSafetyError> {
    if selected.file_name().is_none() {
        return Err(SaveSafetyError::MissingFileName);
    }
    let extension = selected.extension().and_then(|value| value.to_str());
    if extension
        .is_some_and(|value| value.eq_ignore_ascii_case("sam") || value.eq_ignore_ascii_case("sty"))
    {
        return Err(SaveSafetyError::LegacyTarget);
    }
    if extension.is_some_and(|value| value.eq_ignore_ascii_case("amipro")) {
        Ok(selected.to_path_buf())
    } else {
        Ok(selected.with_extension("amipro"))
    }
}

/// Where Save As writes and in which format: a typed `.sam` is an Ami Pro
/// document and a typed `.amipro` a native one; otherwise the format chosen
/// in the dialog, with its extension added (`.SAM`, or `.amipro` in place
/// of any other). `.sty` is refused: style sheets have their own command.
pub fn save_destination(
    selected: &Path,
    chosen: Option<SaveFormat>,
) -> Result<(PathBuf, SaveFormat), SaveSafetyError> {
    if selected.file_name().is_none() {
        return Err(SaveSafetyError::MissingFileName);
    }
    let extension = selected.extension().and_then(|value| value.to_str());
    match extension {
        Some(value) if value.eq_ignore_ascii_case("sam") => {
            Ok((selected.to_path_buf(), SaveFormat::Sam))
        }
        Some(value) if value.eq_ignore_ascii_case("sty") => Err(SaveSafetyError::StyleSheetTarget),
        Some(value) if value.eq_ignore_ascii_case("amipro") => {
            Ok((selected.to_path_buf(), SaveFormat::Native))
        }
        Some(value)
            if ["md", "markdown", "mdown"]
                .iter()
                .any(|name| value.eq_ignore_ascii_case(name)) =>
        {
            Ok((selected.to_path_buf(), SaveFormat::Markdown))
        }
        None if chosen == Some(SaveFormat::Markdown) => {
            Ok((selected.with_extension("md"), SaveFormat::Markdown))
        }
        None if chosen == Some(SaveFormat::Sam) => {
            Ok((selected.with_extension("SAM"), SaveFormat::Sam))
        }
        _ => Ok((selected.with_extension("amipro"), SaveFormat::Native)),
    }
}

/// What Save As does with the chosen file: an existing one is replaced
/// only after confirmation, an Ami Pro document with a backup the first
/// time (user, 2026-11-04; decision A kept new files only until then).
pub fn classify_save_selection(
    selected: &Path,
    chosen: Option<SaveFormat>,
    exists: impl Fn(&Path) -> bool,
) -> Result<SaveSelection, SaveSafetyError> {
    let (destination, format) = save_destination(selected, chosen)?;
    Ok(match format {
        SaveFormat::Sam if exists(&destination) => SaveSelection::ConfirmOverwrite(destination),
        SaveFormat::Sam => SaveSelection::ReadySam(destination),
        SaveFormat::Native if exists(&destination) => SaveSelection::ConfirmOverwrite(destination),
        SaveFormat::Native => SaveSelection::Ready(destination),
        SaveFormat::Markdown => SaveSelection::ExportMarkdown(destination),
    })
}

/// Writes `bytes` to a new file at `path` as native saves write: to a
/// temporary file beside it first, then into place, but never over an
/// existing file (a hard link fails when the name exists). Where the file
/// system has no hard links, the file is created new and written.
pub fn write_new_file_safely(path: &Path, bytes: &[u8]) -> Result<(), StyleSheetSaveError> {
    use std::io::Write;
    let Some(name) = path.file_name() else {
        return Err(StyleSheetSaveError::MissingFileName);
    };
    if path.exists() {
        return Err(StyleSheetSaveError::Exists(path.to_path_buf()));
    }
    let directory = path.parent().unwrap_or(Path::new("."));
    let temporary = directory.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()));
    if let Err(error) = written {
        let _ = std::fs::remove_file(&temporary);
        return Err(StyleSheetSaveError::Write(format!(
            "{}: {error}",
            temporary.display()
        )));
    }
    let linked = std::fs::hard_link(&temporary, path);
    let _ = std::fs::remove_file(&temporary);
    match linked {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(StyleSheetSaveError::Exists(path.to_path_buf()))
        }
        Err(_) => write_new_file(path, bytes),
    }
}

/// Writes `bytes` to `path`, replacing a file there (the desktop's dialog
/// asked first): to a temporary file beside it, then renamed into place,
/// so that a failed write leaves the old file as it was.
#[cfg_attr(not(feature = "page-surface"), allow(dead_code))]
pub fn write_file_replacing(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let Some(name) = path.file_name() else {
        return Err("the destination has no file name".into());
    };
    let directory = path.parent().unwrap_or(Path::new("."));
    let temporary = directory.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| std::fs::rename(&temporary, path));
    written.map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        format!("{}: {error}", path.display())
    })
}

/// Writes `document` as a new Ami Pro document at `path` and returns the
/// number of bytes written (the app writes its own bytes, `write_sam`).
#[cfg(test)]
pub fn save_sam_document(
    document: &Document,
    path: &Path,
    options: &amipro_formats::SamOptions,
) -> Result<usize, StyleSheetSaveError> {
    let bytes = amipro_formats::sam_document(document, options);
    write_new_file_safely(path, &bytes).map(|()| bytes.len())
}

/// Where the original of `path` is kept when an Ami Pro document is first
/// written over: the same name with `.bak` added (`LETTER.SAM.bak`).
pub fn sam_backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    path.with_file_name(name)
}

/// An Ami Pro document written over its file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SamReplaced {
    pub bytes: usize,
    /// The backup this save made: the file's original, kept the first time
    /// it is written over and never replaced afterwards.
    pub backup: Option<PathBuf>,
}

/// Writes `document` as an Ami Pro document over the file at `path` (user,
/// 2026-11-04). The first time (no backup beside it yet) the original is
/// copied to [`sam_backup_path`] before anything is written, and a failed
/// copy writes nothing; the new file goes to a temporary file beside it,
/// keeps the original's permissions, and is renamed into place, so a failed
/// write leaves the file as it was. A link is followed to the file it
/// names. With no file at `path` the document is written as a new one.
/// Without `keep_backup` (Preferences, CM6) no backup is made.
#[cfg_attr(
    not(any(test, feature = "page-surface")),
    expect(
        dead_code,
        reason = "the merge writes documents over files (page surface)"
    )
)]
pub fn replace_sam_document(
    document: &Document,
    path: &Path,
    options: &amipro_formats::SamOptions,
    keep_backup: bool,
) -> Result<SamReplaced, StyleSheetSaveError> {
    let bytes = amipro_formats::sam_document(document, options);
    replace_sam_bytes(path, &bytes, keep_backup)
}

/// [`replace_sam_document`] for bytes already written (a protected file).
pub fn replace_sam_bytes(
    path: &Path,
    bytes: &[u8],
    keep_backup: bool,
) -> Result<SamReplaced, StyleSheetSaveError> {
    use std::io::Write;
    let failed = |at: &Path, error: std::io::Error| {
        StyleSheetSaveError::Write(format!("{}: {error}", at.display()))
    };
    let target = match std::fs::canonicalize(path) {
        Ok(target) => target,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return write_new_file_safely(path, bytes).map(|()| SamReplaced {
                bytes: bytes.len(),
                backup: None,
            });
        }
        Err(error) => return Err(failed(path, error)),
    };
    let metadata = std::fs::metadata(&target).map_err(|error| failed(&target, error))?;
    if !metadata.is_file() {
        return Err(StyleSheetSaveError::Write(format!(
            "{} is not a file",
            target.display()
        )));
    }
    let backup_path = sam_backup_path(&target);
    let backup = if !keep_backup || backup_path.symlink_metadata().is_ok() {
        None
    } else {
        let original = std::fs::read(&target).map_err(|error| failed(&target, error))?;
        write_new_file_safely(&backup_path, &original)?;
        Some(backup_path)
    };
    let Some(name) = target.file_name() else {
        return Err(StyleSheetSaveError::MissingFileName);
    };
    let directory = target.parent().unwrap_or(Path::new("."));
    let temporary = directory.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.set_permissions(metadata.permissions())?;
            file.sync_all()
        })
        .and_then(|()| std::fs::rename(&temporary, &target));
    if let Err(error) = written {
        let _ = std::fs::remove_file(&temporary);
        return Err(failed(&target, error));
    }
    Ok(SamReplaced {
        bytes: bytes.len(),
        backup,
    })
}

/// Why a style sheet cannot be written where asked (decision D3: new
/// files only, never overwriting).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleSheetSaveError {
    MissingFileName,
    NotAStyleSheetName,
    Exists(PathBuf),
    Write(String),
}

impl fmt::Display for StyleSheetSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFileName => formatter.write_str("a file name is required"),
            Self::NotAStyleSheetName => formatter.write_str("a style sheet's name ends in .sty"),
            Self::Exists(path) => write!(
                formatter,
                "{} already exists; it is not replaced",
                path.display()
            ),
            Self::Write(error) => formatter.write_str(error),
        }
    }
}

/// Where Save as a Style Sheet writes: the chosen file, with `.STY` added
/// when it has no extension. Any other extension is refused.
pub fn style_sheet_destination(selected: &Path) -> Result<PathBuf, StyleSheetSaveError> {
    if selected.file_name().is_none() {
        return Err(StyleSheetSaveError::MissingFileName);
    }
    match selected.extension().and_then(|value| value.to_str()) {
        None => Ok(selected.with_extension("STY")),
        Some(extension) if extension.eq_ignore_ascii_case("sty") => Ok(selected.to_path_buf()),
        Some(_) => Err(StyleSheetSaveError::NotAStyleSheetName),
    }
}

/// Writes `bytes` to a new file at `path`, refusing an existing one; the
/// check and the creation are one step, so no file is ever replaced.
pub fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), StyleSheetSaveError> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::AlreadyExists => StyleSheetSaveError::Exists(path.to_path_buf()),
            _ => StyleSheetSaveError::Write(format!("{}: {error}", path.display())),
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        // Only the file just created is removed.
        let _ = std::fs::remove_file(path);
        return Err(StyleSheetSaveError::Write(format!(
            "{}: {error}",
            path.display()
        )));
    }
    Ok(())
}

/// A style sheet name Ami Pro 3.1 can list: eight letters or digits of the
/// document's name, upper case, and `.STY` (it reads 8.3 names only).
pub fn suggested_style_sheet_name(location: &DocumentLocation, document: &Document) -> String {
    let source = match location {
        DocumentLocation::LegacySource(path)
        | DocumentLocation::Sam(path)
        | DocumentLocation::Native(path) => path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default(),
        DocumentLocation::Untitled => document.title.as_str(),
    };
    let stem: String = source
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .take(8)
        .collect::<String>()
        .to_ascii_uppercase();
    format!("{}.STY", if stem.is_empty() { "STYLE" } else { &stem })
}

pub fn save_native_document(document: &Document, path: &Path) -> Result<(), DocumentError> {
    let destination =
        native_destination(path).map_err(|error| DocumentError::Invalid(error.to_string()))?;
    if destination != path {
        return Err(DocumentError::Invalid(
            "native save target must use the .amipro extension".into(),
        ));
    }
    document.save_native(path)
}

#[cfg(test)]
mod tests {
    use amipro_document::{CharacterFormat, StyleId};
    use amipro_editing::{Cursor, Editor};

    use super::*;

    fn temporary_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "amipro-file-workflow-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn legacy_sources_always_require_save_as() {
        for name in ["source.docx", "styles.STY", "records.txt"] {
            let location = DocumentLocation::LegacySource(PathBuf::from(name));
            assert_eq!(location.save_request(), SaveRequest::ChooseDestination);
        }
        let sam = DocumentLocation::Sam(PathBuf::from("LETTER.SAM"));
        assert_eq!(
            sam.save_request(),
            SaveRequest::SaveSam(PathBuf::from("LETTER.SAM"))
        );
    }

    #[test]
    fn native_documents_save_only_to_the_chosen_native_path() {
        let path = PathBuf::from("documents/report.amipro");
        let location = DocumentLocation::Native(path.clone());
        assert_eq!(location.save_request(), SaveRequest::SaveTo(path));
    }

    #[test]
    fn save_as_rejects_legacy_extensions_and_adds_native_extension() {
        assert_eq!(
            native_destination(Path::new("report")),
            Ok(PathBuf::from("report.amipro"))
        );
        assert_eq!(
            native_destination(Path::new("report.txt")),
            Ok(PathBuf::from("report.amipro"))
        );
        assert_eq!(
            native_destination(Path::new("report.SAM")),
            Err(SaveSafetyError::LegacyTarget)
        );
        assert_eq!(
            native_destination(Path::new("report.sty")),
            Err(SaveSafetyError::LegacyTarget)
        );
    }

    #[test]
    fn existing_native_destination_requires_confirmation() {
        assert_eq!(
            classify_save_selection(Path::new("report.amipro"), None, |_| true),
            Ok(SaveSelection::ConfirmOverwrite(PathBuf::from(
                "report.amipro"
            )))
        );
        assert_eq!(
            classify_save_selection(Path::new("report.amipro"), None, |_| false),
            Ok(SaveSelection::Ready(PathBuf::from("report.amipro")))
        );
    }

    #[test]
    fn ami_pro_documents_are_written_over_with_a_backup_the_first_time() {
        let directory = temporary_directory("sam-replace");
        let target = directory.join("LETTER.SAM");
        let options = amipro_formats::SamOptions::at(1_791_958_088);
        // No file yet: written as a new one, with no backup.
        let first =
            replace_sam_document(&Document::from_plain_text("One"), &target, &options, true)
                .unwrap();
        assert_eq!(first.backup, None);
        let original = std::fs::read(&target).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
        }
        // The first write over it keeps the original beside it.
        let second =
            replace_sam_document(&Document::from_plain_text("Two"), &target, &options, true)
                .unwrap();
        let backup = directory.join("LETTER.SAM.bak");
        assert_eq!(second.backup.as_deref(), Some(backup.as_path()));
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        let read = |path: &Path| {
            amipro_formats::import_sam(&std::fs::read(path).unwrap(), None, "LETTER")
                .unwrap()
                .document
                .plain_text()
        };
        assert_eq!(read(&target), "Two");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }
        // Later writes leave the backup as the first original.
        let third =
            replace_sam_document(&Document::from_plain_text("Three"), &target, &options, true)
                .unwrap();
        assert_eq!(third.backup, None);
        assert_eq!(read(&target), "Three");
        assert_eq!(read(&backup), "One");
        // A link is followed: the file it names is written, the link stays.
        #[cfg(unix)]
        {
            let link = directory.join("LINK.SAM");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            replace_sam_document(&Document::from_plain_text("Four"), &link, &options, true)
                .unwrap();
            assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
            assert_eq!(read(&target), "Four");
            assert!(!directory.join("LINK.SAM.bak").exists());
        }
        // Nothing else is left in the folder (no temporary files).
        let mut names: Vec<String> = std::fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        #[cfg(unix)]
        assert_eq!(names, ["LETTER.SAM", "LETTER.SAM.bak", "LINK.SAM"]);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn save_as_markdown_is_an_export_whatever_exists() {
        for (selected, chosen, want) in [
            ("notes.md", None, "notes.md"),
            ("notes.MARKDOWN", Some(SaveFormat::Sam), "notes.MARKDOWN"),
            ("notes", Some(SaveFormat::Markdown), "notes.md"),
        ] {
            for exists in [false, true] {
                assert_eq!(
                    classify_save_selection(Path::new(selected), chosen, |_| exists),
                    Ok(SaveSelection::ExportMarkdown(PathBuf::from(want)))
                );
            }
        }
    }

    #[test]
    fn save_as_writes_ami_pro_documents_to_new_files_or_over_them_after_confirmation() {
        // The typed extension decides, then the dialog's choice.
        assert_eq!(
            classify_save_selection(Path::new("letter.sam"), None, |_| false),
            Ok(SaveSelection::ReadySam(PathBuf::from("letter.sam")))
        );
        assert_eq!(
            classify_save_selection(Path::new("letter"), Some(SaveFormat::Sam), |_| false),
            Ok(SaveSelection::ReadySam(PathBuf::from("letter.SAM")))
        );
        assert_eq!(
            classify_save_selection(Path::new("letter.amipro"), Some(SaveFormat::Sam), |_| false),
            Ok(SaveSelection::Ready(PathBuf::from("letter.amipro")))
        );
        assert_eq!(
            classify_save_selection(Path::new("letter.SAM"), None, |_| true),
            Ok(SaveSelection::ConfirmOverwrite(PathBuf::from("letter.SAM")))
        );
        assert_eq!(
            classify_save_selection(Path::new("sheet.sty"), None, |_| false),
            Err(SaveSafetyError::StyleSheetTarget)
        );
        // A new file is written new: never over one that appeared since.
        let directory = temporary_directory("sam-new-files");
        let target = directory.join("LETTER.SAM");
        let document = Document::from_plain_text("Hello");
        let written = save_sam_document(
            &document,
            &target,
            &amipro_formats::SamOptions::at(1_791_958_088),
        )
        .unwrap();
        assert_eq!(std::fs::read(&target).unwrap().len(), written);
        assert_eq!(
            save_sam_document(
                &document,
                &target,
                &amipro_formats::SamOptions::at(1_791_958_088),
            ),
            Err(StyleSheetSaveError::Exists(target.clone()))
        );
        let reread = amipro_formats::import_sam(&std::fs::read(&target).unwrap(), None, "LETTER")
            .unwrap()
            .document;
        assert_eq!(reread.plain_text(), "Hello");
        // Nothing but the document is left in the folder.
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn cancelling_overwrite_discards_the_pending_destination() {
        let directory = temporary_directory("cancel-overwrite");
        let target = directory.join("existing.amipro");
        std::fs::write(&target, b"existing document").unwrap();
        let mut confirmation = OverwriteConfirmation::default();

        confirmation.begin(target.clone());
        assert_eq!(confirmation.pending(), Some(target.as_path()));
        confirmation.cancel();

        assert_eq!(confirmation.confirm(), None);
        assert_eq!(std::fs::read(&target).unwrap(), b"existing document");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn style_sheets_are_written_to_new_sty_files_only() {
        assert_eq!(
            style_sheet_destination(Path::new("sheets/MINE")),
            Ok(PathBuf::from("sheets/MINE.STY"))
        );
        assert_eq!(
            style_sheet_destination(Path::new("sheets/mine.sty")),
            Ok(PathBuf::from("sheets/mine.sty"))
        );
        assert_eq!(
            style_sheet_destination(Path::new("sheets/mine.sam")),
            Err(StyleSheetSaveError::NotAStyleSheetName)
        );
        let directory = temporary_directory("sheet");
        let existing = directory.join("_DEFAULT.STY");
        std::fs::write(&existing, b"Ami Pro's own sheet").unwrap();
        assert_eq!(
            write_new_file(&existing, b"ours"),
            Err(StyleSheetSaveError::Exists(existing.clone()))
        );
        assert_eq!(std::fs::read(&existing).unwrap(), b"Ami Pro's own sheet");
        let new = directory.join("MINE.STY");
        write_new_file(&new, b"ours").unwrap();
        assert_eq!(std::fs::read(&new).unwrap(), b"ours");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn suggested_style_sheet_names_are_ami_pro_names() {
        let document = Document::default();
        assert_eq!(
            suggested_style_sheet_name(
                &DocumentLocation::LegacySource(PathBuf::from("docs/Quarterly report 2.SAM")),
                &document
            ),
            "QUARTERL.STY"
        );
        assert_eq!(
            suggested_style_sheet_name(&DocumentLocation::Untitled, &Document::default()),
            "UNTITLED.STY"
        );
    }

    #[test]
    fn native_save_never_modifies_legacy_source() {
        let directory = temporary_directory("legacy-protection");
        let source = directory.join("original.SAM");
        let target = directory.join("converted.amipro");
        std::fs::write(&source, b"original legacy bytes").unwrap();

        save_native_document(&Document::default(), &target).unwrap();

        assert_eq!(std::fs::read(&source).unwrap(), b"original legacy bytes");
        assert!(Document::load_native(&target).is_ok());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn save_as_preserves_paragraph_and_character_formatting() {
        let directory = temporary_directory("formatting-round-trip");
        let target = directory.join("formatted.amipro");
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Formatted text".into();
        document.paragraph_mut(0).style = StyleId(2);
        document
            .paragraph_mut(0)
            .apply_character_format(
                10,
                14,
                &CharacterFormat {
                    italic: Some(true),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();

        save_native_document(&document, &target).unwrap();

        assert_eq!(Document::load_native(&target).unwrap(), document);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn safe_save_api_refuses_non_native_target() {
        let directory = temporary_directory("reject-legacy");
        let source = directory.join("original.SAM");
        std::fs::write(&source, b"original legacy bytes").unwrap();

        assert!(save_native_document(&Document::default(), &source).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), b"original legacy bytes");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn multiparagraph_rich_edit_saves_reopens_and_leaves_legacy_source_unchanged() {
        let directory = temporary_directory("rich-edit-round-trip");
        let source = directory.join("original.SAM");
        let target = directory.join("edited.amipro");
        let legacy_bytes = b"independent legacy source";
        std::fs::write(&source, legacy_bytes).unwrap();

        let mut document = Document::from_plain_text("Heading\nformatted tail");
        document.paragraph_mut(0).style = StyleId(2);
        document
            .paragraph_mut(1)
            .apply_character_format(
                0,
                9,
                &CharacterFormat {
                    underline: Some(true),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();
        let mut editor = Editor::new(document);
        editor
            .replace_document_range(
                Cursor {
                    paragraph: 0,
                    byte: 7,
                },
                Cursor {
                    paragraph: 1,
                    byte: 0,
                },
                "\nInserted\n",
            )
            .unwrap();

        save_native_document(editor.document(), &target).unwrap();

        assert_eq!(&Document::load_native(&target).unwrap(), editor.document());
        assert_eq!(std::fs::read(&source).unwrap(), legacy_bytes);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
