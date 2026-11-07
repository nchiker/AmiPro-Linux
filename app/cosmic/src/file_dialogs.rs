//! Where the file choosers start (DF1). The pinned libcosmic open dialog
//! keeps a starting folder it never sends to the portal, so [`OpenDialog`]
//! builds the same request itself with the portal's current folder. It
//! takes the options of libcosmic's builder and answers with its response
//! and error types, so a call site changes one line. (The save dialog
//! already sends its `directory`.)

#![allow(clippy::result_large_err)] // libcosmic's `Error`, as its own dialogs return

use std::path::{Path, PathBuf};

use cosmic::dialog::ashpd::desktop::file_chooser::{OpenFileRequest, SelectedFiles};
use cosmic::dialog::ashpd::desktop::{Request, ResponseError};
use cosmic::dialog::file_chooser::open::{FileResponse, MultiFileResponse};
use cosmic::dialog::file_chooser::{Choice, Error, FileFilter};

/// The folder a chooser starts in: the folder of the document's own file
/// when it has one, else the Documents preference when that folder still
/// exists (a missing one counts as unset, without erasing the setting).
pub fn start_folder(documents: Option<&Path>, document: Option<&Path>) -> Option<PathBuf> {
    document
        .and_then(Path::parent)
        .filter(|folder| !folder.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| {
            documents
                .filter(|folder| folder.is_dir())
                .map(Path::to_path_buf)
        })
}

/// The same options as libcosmic's `open::Dialog`, plus the folder to
/// start in.
#[must_use]
pub struct OpenDialog {
    title: String,
    accept_label: Option<String>,
    directory: Option<PathBuf>,
    modal: bool,
    choices: Vec<Choice>,
    current_filter: Option<FileFilter>,
    filters: Vec<FileFilter>,
}

pub fn open_dialog() -> OpenDialog {
    OpenDialog {
        title: String::new(),
        accept_label: None,
        directory: None,
        modal: true,
        choices: Vec::new(),
        current_filter: None,
        filters: Vec::new(),
    }
}

impl OpenDialog {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    #[allow(dead_code)]
    pub fn accept_label(mut self, label: impl Into<String>) -> Self {
        self.accept_label = Some(label.into());
        self
    }

    /// The folder to start in; none leaves the desktop's own choice.
    pub fn directory(mut self, directory: impl Into<Option<PathBuf>>) -> Self {
        self.directory = directory.into();
        self
    }

    #[allow(dead_code)]
    pub fn choice(mut self, choice: impl Into<Choice>) -> Self {
        self.choices.push(choice.into());
        self
    }

    pub fn filter(mut self, filter: impl Into<FileFilter>) -> Self {
        self.filters.push(filter.into());
        self
    }

    #[allow(dead_code)]
    pub fn current_filter(mut self, filter: impl Into<FileFilter>) -> Self {
        self.current_filter = Some(filter.into());
        self
    }

    #[allow(dead_code)]
    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    async fn request(self, folders: bool, multiple: bool) -> Result<SelectedFiles, Error> {
        let error = |error| {
            if let cosmic::dialog::ashpd::Error::Response(ResponseError::Cancelled) = error {
                Error::Cancelled
            } else {
                Error::Open(error)
            }
        };
        let request: Request<SelectedFiles> = OpenFileRequest::default()
            .title(Some(self.title.as_str()))
            .accept_label(self.accept_label.as_deref())
            .directory(folders)
            .modal(self.modal)
            .multiple(multiple)
            .choices(self.choices)
            .filters(self.filters)
            .current_filter(self.current_filter)
            .current_folder::<&Path>(self.directory.as_deref())
            .map_err(Error::SetDirectory)?
            .send()
            .await
            .map_err(error)?;
        request.response().map_err(error)
    }

    pub async fn open_file(self) -> Result<FileResponse, Error> {
        self.request(false, false).await.map(FileResponse)
    }

    #[allow(dead_code)]
    pub async fn open_files(self) -> Result<MultiFileResponse, Error> {
        self.request(false, true).await.map(MultiFileResponse)
    }

    pub async fn open_folder(self) -> Result<FileResponse, Error> {
        self.request(true, false).await.map(FileResponse)
    }

    #[allow(dead_code)]
    pub async fn open_folders(self) -> Result<MultiFileResponse, Error> {
        self.request(true, true).await.map(MultiFileResponse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_folder_is_the_documents_own_else_the_preference_when_it_exists() {
        let scratch = std::env::temp_dir().join(format!("amipro-df1-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let missing = scratch.join("gone");
        // Unset, and nothing better.
        assert_eq!(start_folder(None, None), None);
        // Set and existing.
        assert_eq!(start_folder(Some(&scratch), None), Some(scratch.clone()));
        // Set but missing: unset when used.
        assert_eq!(start_folder(Some(&missing), None), None);
        // A document with a path keeps its own folder.
        let file = Path::new("/somewhere/else/a.SAM");
        assert_eq!(
            start_folder(Some(&scratch), Some(file)),
            Some(PathBuf::from("/somewhere/else"))
        );
        assert_eq!(
            start_folder(None, Some(file)),
            Some(PathBuf::from("/somewhere/else"))
        );
        std::fs::remove_dir_all(&scratch).unwrap();
    }
}
