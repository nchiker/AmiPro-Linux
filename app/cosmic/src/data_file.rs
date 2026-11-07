//! Merge data files entered as Ami Pro tables (`docs/Design/UI_SPEC.md`
//! §38, "Rework after first use"; the user's MR1): a data file's tab shows
//! its pages with one table, the field names in its heading row and a
//! record in each later row, edited with the table editing of §32 (a click
//! and typing in a cell, Tab, a new record from the last cell, the Table
//! menu, Sort). Ami Pro merges from such a table document (kind 4, VM
//! MGT1). Ami Pro's delimited data document (`~|`) is shown as the same
//! table and written back in its own form by Save. Open merge documents
//! whose data file this is follow its records live.

use std::path::Path;
use std::sync::Arc;

use amipro_document::Document;
use amipro_editing::merge::data::{
    DataTable, Delimiters, data_document, delimiter_clash, read_ami_data, read_table_data,
    table_data_document,
};

use crate::merge::{DataKind, MergeData};
use crate::page_editing::PageSelection;
use crate::{AmiProApp, DocumentSession};

/// What a data file's tab keeps about its records.
#[derive(Clone, Debug)]
pub struct DataForm {
    /// Ami Pro's delimited form Save writes the table back in; `None` for
    /// a table data file, saved as it is.
    pub delimiters: Option<Delimiters>,
    /// The document the records were last read from (an edit, Undo, and
    /// Redo make another).
    pub read_from: Document,
    pub table: DataTable,
}

/// One line of Table Text in a new document: a new row's least height.
fn row_height() -> amipro_document::Twips {
    crate::table_commands::one_line(&Document::from_plain_text(""))
}

impl AmiProApp {
    /// Edit Records: the data file's tab (opened when it is not) with its
    /// records as a table.
    pub(crate) fn edit_records(&mut self, path: &Path) {
        let index = match crate::session_index_for_path(&self.sessions, path) {
            Some(index) => index,
            None => {
                let folders = self.style_folders();
                let document = match crate::master::read_document(path, &folders) {
                    Ok(document) => document,
                    Err(error) => {
                        self.status = format!("The data file could not be opened: {error}");
                        return;
                    }
                };
                let extension = path.extension().and_then(|extension| extension.to_str());
                let location = if extension
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("amipro"))
                {
                    crate::DocumentLocation::Native(path.to_path_buf())
                } else if extension.is_some_and(|extension| extension.eq_ignore_ascii_case("sam")) {
                    // Save writes over it, with a backup the first time
                    // (MC6, the user's answer of 2026-11-04).
                    crate::DocumentLocation::Sam(path.to_path_buf())
                } else {
                    crate::DocumentLocation::LegacySource(path.to_path_buf())
                };
                let session =
                    DocumentSession::new(self.next_session_id, document, location, String::new());
                self.next_session_id += 1;
                self.sessions.push(session);
                self.sessions.len() - 1
            }
        };
        self.active_session = index;
        self.show_records_as_table();
        // Typing goes into the first cell at once, as the status bar says
        // ("Type in the cells"), without a click on the page first.
        self.page_keyboard += 1;
    }

    /// The active tab's data file as a table, the caret in its first
    /// record. A delimited data document becomes the table in a fresh tab
    /// state (its edits so far are in the table; Undo starts there), and
    /// Save writes it back in its own form.
    pub(crate) fn show_records_as_table(&mut self) {
        if self.data_form.is_none() {
            if let Ok(table) = read_table_data(&self.document) {
                self.data_form = Some(DataForm {
                    delimiters: None,
                    read_from: self.document.clone(),
                    table,
                });
            } else {
                match read_ami_data(&self.document) {
                    Ok((table, delimiters)) => {
                        let converted = table_data_document(&table, row_height());
                        let old = &self.sessions[self.active_session];
                        let mut fresh = DocumentSession::new(
                            old.id,
                            converted.clone(),
                            old.location.clone(),
                            String::new(),
                        );
                        fresh.dirty = old.dirty;
                        fresh.data_form = Some(DataForm {
                            delimiters: Some(delimiters),
                            read_from: converted,
                            table,
                        });
                        self.sessions[self.active_session] = fresh;
                    }
                    Err(error) => {
                        self.status = format!("Not a merge data file: {}", error.message());
                        return;
                    }
                }
            }
        }
        // The caret in the first record's first cell, ready to type.
        let frame = self.document.frame_stack().first().copied();
        if let Some(frame) = frame
            && let Some(amipro_document::FrameContent::Table(table)) =
                self.document.frame(frame).map(|frame| &frame.content)
            && let Some(caret) =
                amipro_editing::tables::cell_caret(table, usize::from(table.rows.len() > 1), 0)
        {
            self.page_selection = PageSelection::caret(caret, 0);
            self.frame_selection = None;
            self.sync_editor_to_page();
        }
        self.status =
            "Type in the cells; Tab moves on, and Tab in the last cell adds a record".into();
    }

    /// What Save writes for the active tab as SAM: a delimited data
    /// file's table back in its own form (`None` for any other document).
    /// A value holding a delimiter cannot be written that way.
    pub(crate) fn data_for_sam(&self) -> Result<Option<Document>, String> {
        let Some(delimiters) = self.data_form.as_ref().and_then(|form| form.delimiters) else {
            return Ok(None);
        };
        let table = read_table_data(&self.document).map_err(|error| error.message())?;
        if let Some((record, field)) = delimiter_clash(&table, delimiters) {
            let name = table.names.get(field).cloned().unwrap_or_default();
            let marks = match delimiters.record {
                Some(record) => format!("{} or {record}", delimiters.field),
                None => delimiters.field.to_string(),
            };
            return Err(match record {
                Some(record) => format!(
                    "Record {}'s {name} holds {marks}, which this data file uses between values",
                    record + 1
                ),
                None => format!(
                    "The field name {name} holds {marks}, which this data file uses between values"
                ),
            });
        }
        let mut document = data_document(&table, delimiters);
        document.title = self.document.title.clone();
        Ok(Some(document))
    }

    /// After any message: each data tab's records follow its table (an
    /// edit, Undo, Redo), and merge panels whose data file is an open data
    /// tab take its records as they are now.
    pub(crate) fn sync_data_forms(&mut self) {
        // The Merge panel's count of the merge document's fields (MR2).
        for session in &mut self.sessions {
            let document = session
                .merge_preview
                .as_ref()
                .map_or(&session.document, |preview| &preview.source)
                .clone();
            if let Some(panel) = session.merge_panel.as_mut()
                && panel.fields_of.as_ref() != Some(&document)
            {
                panel.fields = amipro_editing::merge::merge_field_names(&document).len();
                panel.fields_of = Some(document);
            }
        }
        let mut live: Vec<(std::path::PathBuf, DataTable)> = Vec::new();
        for session in &mut self.sessions {
            let Some(form) = session.data_form.as_mut() else {
                continue;
            };
            if form.read_from != session.document {
                if let Ok(table) = read_table_data(&session.document) {
                    form.table = table;
                }
                form.read_from = session.document.clone();
            }
            if let Some(path) = session.location.path() {
                live.push((path.to_path_buf(), form.table.clone()));
            }
        }
        if live.is_empty() {
            return;
        }
        for session in &mut self.sessions {
            let Some(panel) = session.merge_panel.as_mut() else {
                continue;
            };
            let Some(data) = panel.data.clone() else {
                continue;
            };
            if let Some((_, table)) = live.iter().find(|(path, _)| *path == data.path)
                && data.table != *table
            {
                panel.keys = table.field_keys();
                panel.data = Some(Arc::new(MergeData {
                    table: table.clone(),
                    kind: if data.kind == DataKind::AmiPro {
                        DataKind::AmiPro
                    } else {
                        DataKind::Table
                    },
                    ..(*data).clone()
                }));
            }
        }
    }
}
