//! Ami Pro's glossary on the page surface (the TOC, index, and glossary
//! run; `docs/Design/UI_SPEC.md` §36, TX7, TX8): Edit › Mark Text ▸
//! Glossary…, Insert › Glossary Record… (Ctrl+K), the Glossary panel, and
//! Complete glossary names as you type. The data file is Ami Pro's; the app
//! writes it only to add, rename, or delete a record, through a temporary
//! file and a rename, and never when it only reads.

use std::path::{Path, PathBuf};

use amipro_document::Document;
use amipro_editing::{Command, InsertedDocument, TextPosition};
use amipro_formats::glossary::{
    self, Glossary, append_record, delete_record, fragment_text, new_glossary, read_glossary,
    record_document, rename_record,
};
use cosmic::dialog::file_chooser;

use crate::glossary_panel::{GlossaryAction, RecordRow, Shown, word_at};
use crate::{AmiProApp, Message};

/// Writes `bytes` to `path` through a temporary file beside it and a
/// rename (TX8).
fn write_replacing(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("glossary-tmp");
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        error.to_string()
    })
}

/// The commands that put `record` into the text at `at` as Ami Pro's
/// glossary does, inline (`13b0:2786`): its paragraphs inserted, then its
/// first joined to the text before the caret and its last to the text
/// after. `length` is the length of the caret's paragraph.
fn inline_insert(at: TextPosition, length: usize, record: Document, page: u32) -> Vec<Command> {
    let count = record.main_flow().paragraphs.len();
    let last_length = record
        .main_flow()
        .paragraphs
        .last()
        .map_or(0, |paragraph| paragraph.text.len());
    let mut commands = vec![Command::InsertDocument {
        at,
        source: InsertedDocument(std::sync::Arc::new(record)),
        first_page: page,
    }];
    let join = |paragraph: usize, byte: usize| Command::ReplaceText {
        start: TextPosition {
            paragraph,
            byte,
            ..at
        },
        end: TextPosition {
            paragraph: paragraph + 1,
            byte: 0,
            ..at
        },
        text: String::new(),
    };
    let p = at.paragraph;
    if count == 0 {
        return commands;
    }
    if at.byte == 0 {
        // The record before the paragraph: its last joins it.
        commands.push(join(p + count - 1, last_length));
    } else if at.byte >= length {
        // After it: the paragraph joins the record's first.
        commands.push(join(p, at.byte));
    } else {
        // Between its halves: the later join first.
        commands.push(join(p + count, last_length));
        commands.push(join(p, at.byte));
    }
    commands
}

impl AmiProApp {
    fn glossary_path(&self) -> Option<PathBuf> {
        let file = self.preferences.glossary_file.trim();
        (!file.is_empty()).then(|| PathBuf::from(file))
    }

    /// The data file's bytes and records, read now.
    fn read_glossary_file(&self) -> Result<(Vec<u8>, Glossary), String> {
        let path = self
            .glossary_path()
            .ok_or("Choose a glossary data file first")?;
        let bytes = std::fs::read(&path).map_err(|_| format!("Cannot find {}.", path.display()))?;
        let glossary = read_glossary(&bytes).map_err(|error| error.to_string())?;
        Ok((bytes, glossary))
    }

    /// The panel's records with their previews, from the file as it is.
    fn refresh_glossary(&mut self) {
        let read = self.read_glossary_file();
        let chosen = self.glossary_path().is_some();
        let Some(panel) = self.glossary.as_mut() else {
            return;
        };
        match read {
            Ok((bytes, glossary)) => {
                panel.records = glossary
                    .records
                    .iter()
                    .map(|record| RecordRow {
                        name: record.name.clone(),
                        preview: record_document(&bytes, record)
                            .map(|document| {
                                let text = document.plain_text();
                                let mut preview: String = text.chars().take(160).collect();
                                if text.chars().count() > 160 {
                                    preview.push('…');
                                }
                                preview
                            })
                            .unwrap_or_default(),
                    })
                    .collect();
                panel
                    .records
                    .sort_by_key(|record| record.name.to_lowercase());
            }
            Err(problem) => {
                panel.records.clear();
                if chosen {
                    panel.message = Some(problem);
                }
            }
        }
    }

    /// Opens the Glossary panel; Mark Text ▸ Glossary… with the name field
    /// focused for the selection.
    pub(crate) fn open_glossary_panel(&mut self) {
        let panel = self.glossary.take().unwrap_or_default();
        self.close_rail_panels();
        self.glossary = Some(panel);
        self.refresh_glossary();
        self.open_rail();
    }

    /// The record named `name` as a document to insert.
    fn glossary_record(&self, name: &str) -> Option<Document> {
        let (bytes, glossary) = self.read_glossary_file().ok()?;
        let record = glossary.find(name)?;
        record_document(&bytes, record)
    }

    /// Insert › Glossary Record… and Ctrl+K: the word at the caret
    /// replaced by its record when it names one, in one undo step; else
    /// the Glossary panel opens with the records (Ami Pro's list).
    pub(crate) fn insert_glossary_record(&mut self) {
        let caret = self.page_selection.focus;
        let word = self
            .page_selection
            .is_caret()
            .then(|| {
                let paragraph = self
                    .document
                    .flow(caret.flow)?
                    .paragraphs
                    .get(caret.paragraph)?;
                let range = word_at(&paragraph.text, caret.byte)?;
                Some((range.clone(), paragraph.text[range].to_string()))
            })
            .flatten();
        if let Some((range, name)) = word
            && let Some(record) = self.glossary_record(&name)
        {
            self.expand_glossary(caret, range, record, &name);
            return;
        }
        self.open_glossary_panel();
    }

    fn expand_glossary(
        &mut self,
        caret: TextPosition,
        range: std::ops::Range<usize>,
        record: Document,
        name: &str,
    ) {
        let start = TextPosition {
            byte: range.start,
            ..caret
        };
        let end = TextPosition {
            byte: range.end,
            ..caret
        };
        let page = self.page_selection.page;
        let length = self
            .document
            .flow(caret.flow)
            .and_then(|flow| flow.paragraphs.get(caret.paragraph))
            .map_or(0, |paragraph| paragraph.text.len())
            - (range.end - range.start);
        let mut commands = vec![Command::ReplaceText {
            start,
            end,
            text: String::new(),
        }];
        commands.extend(inline_insert(start, length, record, page));
        self.execute_page_commands(&[Command::Several(commands)]);
        self.status = format!("Glossary record “{name}” inserted");
    }

    /// The panel's Insert: the record at the caret (Ami Pro's dialog
    /// inserts at the caret, the selection left in place).
    fn insert_named_record(&mut self, name: &str) {
        let Some(record) = self.glossary_record(name) else {
            return;
        };
        let at = self.page_selection.ordered().0;
        let page = self.page_selection.page;
        let length = self
            .document
            .flow(at.flow)
            .and_then(|flow| flow.paragraphs.get(at.paragraph))
            .map_or(0, |paragraph| paragraph.text.len());
        self.execute_page_commands(&[Command::Several(inline_insert(at, length, record, page))]);
        self.status = format!("Glossary record “{name}” inserted");
    }

    /// The selection as a document of its own: the main text's with its
    /// formatting and styles, other text as plain text.
    fn selection_document(&self) -> Option<Document> {
        let (start, end) = self.page_selection.ordered();
        if start == end || start.flow != end.flow {
            return None;
        }
        let main = self.document.main_flow_id();
        if start.flow != main {
            return self
                .selected_text()
                .map(|text| Document::from_plain_text(&text));
        }
        let last = self.document.main_flow().paragraphs.len() - 1;
        let flow_end = TextPosition {
            flow: main,
            paragraph: last,
            byte: self.document.main_flow().paragraphs[last].text.len(),
        };
        let flow_start = TextPosition {
            flow: main,
            paragraph: 0,
            byte: 0,
        };
        let mut document = amipro_editing::apply(
            &self.document,
            &Command::ReplaceText {
                start: end,
                end: flow_end,
                text: String::new(),
            },
        )
        .map_or_else(|_| self.document.clone(), |applied| applied.document);
        document = amipro_editing::apply(
            &document,
            &Command::ReplaceText {
                start: flow_start,
                end: start,
                text: String::new(),
            },
        )
        .map_or(document, |applied| applied.document);
        Some(document)
    }

    /// Mark Glossary Record: the selection added to the data file as a
    /// record named as typed (Ami Pro's rules: 27 characters, no name in
    /// use).
    fn add_glossary_record(&mut self) {
        let name = self
            .glossary
            .as_ref()
            .map(|panel| panel.name.trim().to_string())
            .unwrap_or_default();
        let result = (|| -> Result<String, String> {
            let path = self
                .glossary_path()
                .ok_or("Choose a glossary data file first")?;
            let document = self
                .selection_document()
                .ok_or("Select the text the record holds first")?;
            let text = fragment_text(&document);
            let bytes =
                std::fs::read(&path).map_err(|_| format!("Cannot find {}.", path.display()))?;
            let written = append_record(&bytes, &name, &text).map_err(|error| error.to_string())?;
            write_replacing(&path, &written)?;
            Ok(format!("Record “{name}” added"))
        })();
        if let Some(panel) = self.glossary.as_mut() {
            match result {
                Ok(message) => {
                    panel.name.clear();
                    panel.message = Some(message);
                }
                Err(problem) => panel.message = Some(problem),
            }
        }
        self.refresh_glossary();
    }

    fn change_glossary_file(
        &mut self,
        change: impl FnOnce(&[u8]) -> Result<Vec<u8>, glossary::GlossaryError>,
        done: String,
    ) {
        let result = (|| -> Result<(), String> {
            let path = self
                .glossary_path()
                .ok_or("Choose a glossary data file first")?;
            let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
            let written = change(&bytes).map_err(|error| error.to_string())?;
            write_replacing(&path, &written)
        })();
        if let Some(panel) = self.glossary.as_mut() {
            panel.message = Some(match result {
                Ok(()) => done,
                Err(problem) => problem,
            });
        }
        self.refresh_glossary();
    }

    /// The desktop's file dialog for Data File… (open) or New… (save).
    fn glossary_file_task(new: bool) -> cosmic::app::Task<Message> {
        cosmic::task::future(async move {
            let path = if new {
                file_chooser::save::Dialog::new()
                    .title("New Glossary Data File".to_owned())
                    .file_name("GLOSSARY.SAM".to_owned())
                    .save_file()
                    .await
                    .ok()
                    .and_then(|response| response.url().and_then(|url| url.to_file_path().ok()))
            } else {
                file_chooser::open::Dialog::new()
                    .title("Glossary Data File".to_owned())
                    .open_file()
                    .await
                    .ok()
                    .and_then(|response| response.url().to_file_path().ok())
            };
            Message::GlossaryFile { path, new }
        })
    }

    /// A data file chosen: an existing one must read as a glossary; a new
    /// one is made from the default style sheet, as Ami Pro's Mark makes it.
    pub(crate) fn glossary_file_chosen(&mut self, path: Option<PathBuf>, new: bool) {
        let Some(path) = path else {
            return;
        };
        let problem = if new {
            if path.exists() {
                Some("That file exists: choose it with Data File…".to_string())
            } else {
                let sheet =
                    amipro_formats::find_style_sheet(None, "_DEFAULT.STY", &self.style_folders())
                        .and_then(|path| std::fs::read(path).ok())
                        .unwrap_or_else(|| b"[ver]\r\n\t4\r\n".to_vec());
                crate::file_workflow::write_new_file_safely(&path, &new_glossary(&sheet))
                    .err()
                    .map(|error| error.to_string())
            }
        } else {
            std::fs::read(&path)
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    read_glossary(&bytes)
                        .map(|_| ())
                        .map_err(|error| error.to_string())
                })
                .err()
        };
        match problem {
            Some(problem) => {
                if let Some(panel) = self.glossary.as_mut() {
                    panel.message = Some(problem);
                }
            }
            None => {
                self.preferences.glossary_file = path.display().to_string();
                self.save_preferences();
                if let Some(panel) = self.glossary.as_mut() {
                    panel.message = None;
                }
            }
        }
        self.refresh_glossary();
    }

    pub(crate) fn update_glossary(&mut self, action: GlossaryAction) -> cosmic::app::Task<Message> {
        use GlossaryAction as A;
        match action {
            A::Close => {
                self.glossary = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            A::Select(name) => {
                if let Some(panel) = self.glossary.as_mut() {
                    panel.rename = name.clone();
                    panel.selected = Some(name);
                }
            }
            A::Insert(name) => self.insert_named_record(&name),
            A::Name(text) => {
                if let Some(panel) = self.glossary.as_mut() {
                    panel.name = text.chars().take(glossary::NAME_LIMIT).collect();
                }
            }
            A::Add => self.add_glossary_record(),
            A::RenameText(text) => {
                if let Some(panel) = self.glossary.as_mut() {
                    panel.rename = text.chars().take(glossary::NAME_LIMIT).collect();
                }
            }
            A::Rename => {
                let Some((old, new)) = self
                    .glossary
                    .as_ref()
                    .and_then(|panel| Some((panel.selected.clone()?, panel.rename.clone())))
                else {
                    return cosmic::app::Task::none();
                };
                let done = format!("Renamed “{old}” to “{new}”");
                self.change_glossary_file(|bytes| rename_record(bytes, &old, &new), done);
                if let Some(panel) = self.glossary.as_mut() {
                    panel.selected = Some(new);
                }
            }
            A::Delete => {
                let Some(name) = self
                    .glossary
                    .as_ref()
                    .and_then(|panel| panel.selected.clone())
                else {
                    return cosmic::app::Task::none();
                };
                let done = format!("Deleted “{name}”");
                self.change_glossary_file(|bytes| delete_record(bytes, &name), done);
                if let Some(panel) = self.glossary.as_mut() {
                    panel.selected = None;
                }
            }
            A::ChooseFile => return Self::glossary_file_task(false),
            A::NewFile => return Self::glossary_file_task(true),
            A::Complete(on) => {
                self.preferences.complete_glossary = on;
                self.save_preferences();
            }
        }
        cosmic::app::Task::none()
    }

    /// Complete glossary names as you type (TX7): the record the word just
    /// typed names, if any, read from the data file.
    pub(crate) fn glossary_completion(&self) -> Option<(std::ops::Range<usize>, String)> {
        if !self.preferences.complete_glossary || !self.page_selection.is_caret() {
            return None;
        }
        let caret = self.page_selection.focus;
        let paragraph = self
            .document
            .flow(caret.flow)?
            .paragraphs
            .get(caret.paragraph)?;
        let range = word_at(&paragraph.text, caret.byte)?;
        if range.end != caret.byte {
            return None;
        }
        let (_, glossary) = self.read_glossary_file().ok()?;
        let name = &paragraph.text[range.clone()];
        glossary
            .find(name)
            .map(|record| (range, record.name.clone()))
    }

    /// Tab after a glossary name, with the option on: the record in its
    /// place. Whether it was taken.
    pub(crate) fn complete_glossary_on_tab(&mut self) -> bool {
        let Some((range, name)) = self.glossary_completion() else {
            return false;
        };
        let Some(record) = self.glossary_record(&name) else {
            return false;
        };
        self.expand_glossary(self.page_selection.focus, range, record, &name);
        true
    }

    pub(crate) fn glossary_view(&self) -> cosmic::Element<'_, Message> {
        let Some(state) = self.glossary.as_ref() else {
            return cosmic::widget::space::horizontal().into();
        };
        crate::glossary_panel::panel(
            state,
            Shown {
                file: &self.preferences.glossary_file,
                complete: self.preferences.complete_glossary,
                has_selection: !self.page_selection.is_caret(),
            },
        )
    }
}
