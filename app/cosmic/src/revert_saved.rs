//! File › Revert to Saved… (`UI_SPEC.md` §39), as Ami Pro's File ›
//! Revert (command 0x69, `COMMANDS_NOTES.md` §6): it asks first, then reads
//! the file again. Unlike Ami Pro's it is one undo step, so Undo brings the
//! unsaved changes back.

use cosmic::{Element, widget};

use crate::{AmiProApp, DocumentLocation, Message};

/// Ami Pro's question (string of command 0x69).
pub const QUESTION: &str =
    "This will undo all the changes you have made since you last saved. Are you sure?";

pub fn dialog(name: &str) -> Element<'static, Message> {
    widget::dialog()
        .title(format!("Revert {name} to the saved file?"))
        .body(QUESTION)
        .primary_action(
            widget::button::destructive("Revert").on_press(Message::RevertToSaved(true)),
        )
        .secondary_action(
            widget::button::standard("Cancel").on_press(Message::RevertToSaved(false)),
        )
        .into()
}

impl AmiProApp {
    /// The saved file the active document came from and would revert to.
    pub(crate) fn saved_file(&self) -> Option<&std::path::Path> {
        match &self.location {
            DocumentLocation::Sam(path) | DocumentLocation::Native(path) => Some(path),
            _ => None,
        }
    }

    /// Reads the saved file again in place of the document, as one undo
    /// step; the document is then unchanged since its save.
    pub(crate) fn revert_to_saved(&mut self) {
        let Some(path) = self.saved_file().map(std::path::Path::to_path_buf) else {
            return;
        };
        let read = match &self.location {
            DocumentLocation::Sam(_) => amipro_formats::import_sam_path_with_password(
                &path,
                &self.style_folders(),
                self.password.as_deref(),
            )
            .map(|import| import.document)
            .map_err(|error| error.to_string()),
            _ => amipro_document::Document::load_native(&path)
                .map(|mut document| {
                    amipro_formats::take_doc_info(&mut document);
                    document
                })
                .map_err(|error| error.to_string()),
        };
        let document = match read {
            Ok(document) => document,
            Err(error) => {
                self.status = format!("Revert to Saved failed: {error}");
                return;
            }
        };
        let previous = self.document.clone();
        self.push_undo();
        self.document = document;
        let caret = crate::HistoryCaret {
            cursor: self.editor.cursor(),
            #[cfg(feature = "page-surface")]
            page: crate::page_editing::PageSelection::start(&self.document),
            #[cfg(feature = "page-surface")]
            frames: None,
        };
        self.restore_history(&previous, caret);
        self.clean_document = self.document.clone();
        self.dirty = false;
        self.status = format!(
            "Reverted to the saved {}; Undo brings the changes back",
            crate::file_name(&path)
        );
    }
}
