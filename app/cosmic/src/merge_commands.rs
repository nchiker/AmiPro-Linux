//! The Merge panel's behaviour (`docs/Design/UI_SPEC.md` §38): the data
//! file read off the interface's thread, merge fields inserted at the
//! caret, the preview (display only, MG5), conditions (one undo step
//! each, MG6), and the outputs (MG4).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use amipro_document::Document;
use amipro_document::merge::{Condition, MergeSettings};
use amipro_editing::merge::conditions::selected;
use amipro_editing::merge::{MergeInputs, MergedRecord, merge_all, merge_one, set_merge_command};
use cosmic::dialog::file_chooser::{self, FileFilter};

use amipro_editing::merge::labels::{LabelSheet, SHEETS, label_run, labels_off, set_up_labels};
use amipro_editing::merge::{MergedRecord as Merged, unknown_fields};

use crate::merge::{MergeData, read_data, resolve};
use crate::merge_panel::{self, MergeAction, MergePanel, Shown, StartFrom};
use crate::{AmiProApp, Message};

type Task = cosmic::app::Task<Message>;

/// The document's own text with its fields, while the preview shows a
/// record's values in its place (MG5).
#[derive(Clone, Debug)]
pub struct MergePreview {
    pub source: Document,
}

/// Merge, view & print (MG4): the records the merge takes, the one shown,
/// and the documents chosen to print, sent together at the end (MC7).
#[derive(Clone, Debug)]
pub struct MergeViewing {
    pub records: Vec<usize>,
    pub position: usize,
    pub printed: Vec<Document>,
}

impl crate::DocumentSession {
    /// The preview goes off: the document shows its fields again. True when
    /// it was on.
    pub(crate) fn leave_merge_preview(&mut self) -> bool {
        let Some(preview) = self.merge_preview.take() else {
            return false;
        };
        let shown = std::mem::replace(&mut self.document, preview.source);
        crate::synchronize_editor_text(&mut self.editor, &shown, &self.document);
        // The caret was placed in the record's values: it goes to the same
        // place in the letter (after a field when it was in its value), so
        // what follows never lands inside a field (the user's second use).
        let map = |at| amipro_editing::merge::source_position(&self.document, &shown, at);
        let selection = crate::page_editing::PageSelection {
            anchor: map(self.page_selection.anchor),
            focus: map(self.page_selection.focus),
            page: self.page_selection.page,
        };
        self.page_selection = crate::page_editing::valid_selection(&self.document, selection);
        self.refresh_rich_layout();
        true
    }

    /// An edit while the preview shows a record turns the preview off and
    /// is not made: the record's values are display only (MG5; MC3).
    pub(crate) fn preview_refuses_edit(&mut self) -> bool {
        if !self.leave_merge_preview() {
            return false;
        }
        self.status = "Preview off: the document shows its merge fields again".into();
        true
    }
}

impl AmiProApp {
    /// File › Merge… and Insert › Merge Field…: the panel, reading the
    /// data file `[recfile]` names.
    pub(crate) fn open_merge_panel(&mut self) -> Task {
        self.close_rail_panels();
        self.merge_panel
            .get_or_insert_with(MergePanel::default)
            .shown = true;
        self.open_rail();
        self.read_stored_merge_data()
    }

    /// Reads the data file `[recfile]` names when the panel has none yet
    /// (the panel kept, hidden, when only the Envelope panel needs it).
    pub(crate) fn read_stored_merge_data(&mut self) -> Task {
        let Some(file) = self.merge_document().merge().record_file.clone() else {
            return Task::none();
        };
        let panel = self.merge_panel.get_or_insert_with(MergePanel::default);
        if panel.data.is_some() || panel.reading {
            return Task::none();
        }
        let folder = self.location.folder().map(Path::to_path_buf);
        match resolve(&file.data_file, folder.as_deref()) {
            Some(path) => {
                let description = resolve(&file.description, folder.as_deref());
                self.read_merge_data(path, description)
            }
            None => Task::none(),
        }
    }

    /// The merge document: the document itself, not the preview's copy.
    pub(crate) fn merge_document(&self) -> &Document {
        self.merge_preview
            .as_ref()
            .map_or(&self.document, |preview| &preview.source)
    }

    fn read_merge_data(&mut self, path: PathBuf, description: Option<PathBuf>) -> Task {
        if let Some(panel) = self.merge_panel.as_mut() {
            panel.reading = true;
            panel.message = None;
        }
        let folders = self.style_folders();
        let session_id = self.id;
        cosmic::task::future(async move {
            let result = crate::import::in_background(move || {
                read_data(&path, description.as_deref(), &folders)
            })
            .await
            .map(Arc::new);
            Message::MergeRead { session_id, result }
        })
    }

    pub(crate) fn merge_read(&mut self, result: Result<Arc<MergeData>, String>) {
        let Some(panel) = self.merge_panel.as_mut() else {
            return;
        };
        panel.reading = false;
        match result {
            Ok(data) => {
                panel.keys = data.table.field_keys();
                panel.data = Some(data);
                panel.position = 0;
            }
            Err(error) => panel.message = Some(error),
        }
        self.refresh_merge_preview();
    }

    /// The records the merge takes: those the conditions select when With
    /// conditions is on, else all.
    pub(crate) fn merge_records(&self) -> Vec<usize> {
        let Some(data) = self
            .merge_panel
            .as_ref()
            .and_then(|panel| panel.data.as_ref())
        else {
            return Vec::new();
        };
        let settings = self.merge_document().merge();
        if settings.with_conditions {
            selected(&settings.conditions, &data.table)
        } else {
            (0..data.table.records.len()).collect()
        }
    }

    pub(crate) fn merge_view(&self) -> cosmic::Element<'_, Message> {
        let Some(panel) = self.merge_panel.as_ref() else {
            return cosmic::widget::space::horizontal().into();
        };
        let settings = self.merge_document().merge();
        merge_panel::panel(Shown {
            panel,
            stored: settings
                .record_file
                .as_ref()
                .map(|file| file.data_file.as_str()),
            conditions: &settings.conditions,
            with_conditions: settings.with_conditions,
            selected: self.merge_records().len(),
            preview: self.merge_preview.is_some(),
            unknown: panel
                .data
                .as_ref()
                .map(|data| unknown_fields(self.merge_document(), &data.table))
                .unwrap_or_default(),
            labels: LabelSheet::of(self.merge_document()),
            viewing: self.merge_viewing.as_ref().map(|viewing| {
                (
                    viewing.position,
                    viewing.records.len(),
                    viewing.printed.len(),
                )
            }),
            fields: panel.fields,
        })
    }

    fn set_merge_message(&mut self, message: impl Into<String>) {
        if let Some(panel) = self.merge_panel.as_mut() {
            panel.message = Some(message.into());
        }
    }

    /// Gives the merge document `settings`, one undo step (the preview
    /// goes off first: the step is on the document itself).
    fn change_merge(&mut self, settings: MergeSettings) {
        self.leave_merge_preview();
        // The same settings again (the data file the letter already names,
        // chosen or made anew) change nothing and are no failure.
        if *self.document.merge() == settings {
            return;
        }
        let command = set_merge_command(settings);
        self.execute_document_command(&command);
    }

    /// The preview's document: the merge document with the chosen record.
    fn merged_preview(&self, source: &Document) -> Option<Document> {
        let panel = self.merge_panel.as_ref()?;
        let data = panel.data.as_ref()?;
        let records = self.merge_records();
        if records.is_empty() {
            return None;
        }
        let position = panel.position.min(records.len() - 1);
        let context = self.field_context();
        let mut host = amipro_editing::field_language::NoHost::default();
        let mut inputs = MergeInputs {
            context: &context,
            host: &mut host,
            tidy: true,
            in_place: false,
        };
        merge_one(source, &data.for_merge(), &records, position, &mut inputs)
            .map(|record| record.document)
    }

    /// Shows the chosen record again (after a step, a change of data or
    /// conditions).
    fn refresh_merge_preview(&mut self) {
        let Some(source) = self
            .merge_preview
            .as_ref()
            .map(|preview| preview.source.clone())
        else {
            return;
        };
        if let Some(merged) = self.merged_preview(&source) {
            let session: &mut crate::DocumentSession = self;
            let shown = std::mem::replace(&mut session.document, merged);
            crate::synchronize_editor_text(&mut session.editor, &shown, &session.document);
            self.refresh_rich_layout();
        }
    }

    fn set_preview(&mut self, on: bool) {
        if !on {
            self.leave_merge_preview();
            return;
        }
        if self.merge_preview.is_some() {
            return;
        }
        let source = self.document.clone();
        match self.merged_preview(&source) {
            Some(merged) => {
                let session: &mut crate::DocumentSession = self;
                session.document = merged;
                crate::synchronize_editor_text(&mut session.editor, &source, &session.document);
                self.merge_preview = Some(MergePreview { source });
                self.refresh_rich_layout();
            }
            None => self.set_merge_message("No record to preview."),
        }
    }

    /// Every chosen record merged (MG4, MG9, MG10).
    fn merged_records(&self) -> Vec<MergedRecord> {
        let Some(data) = self
            .merge_panel
            .as_ref()
            .and_then(|panel| panel.data.clone())
        else {
            return Vec::new();
        };
        let records = self.merge_records();
        self.merge_with(&data, &records)
    }

    /// The merge of `records` through the app's field host: Include reads
    /// files beside the document; prompts keep their results.
    fn merge_with(&self, data: &MergeData, records: &[usize]) -> Vec<MergedRecord> {
        let context = self.field_context();
        let folder = self.location.folder().map(Path::to_path_buf);
        let (queries, decisions) = (Default::default(), Default::default());
        let mut host = crate::field_commands::AppFieldHost {
            folder: folder.as_deref(),
            queries: &queries,
            decisions: &decisions,
            questions: Vec::new(),
            warnings: Vec::new(),
            pages: Default::default(),
        };
        let mut inputs = MergeInputs {
            context: &context,
            host: &mut host,
            tidy: true,
            in_place: false,
        };
        merge_all(
            self.merge_document(),
            &data.for_merge(),
            records,
            &mut inputs,
        )
    }

    fn insert_merge_code(&mut self, code: &str) {
        self.leave_merge_preview();
        let (mut start, mut end) = self.page_selection.ordered();
        // Never inside another field: after the outermost one holding the
        // caret.
        if start == end {
            let after = outside_fields(&self.document, start);
            (start, end) = (after, after);
        }
        self.insert_power_field_at(start, end, code, 0);
        // A field the data file has is not "possibly undefined" (Ami Pro's
        // 553 is for a name it lacks).
        let name = code.strip_prefix("MergeField ").map(str::to_uppercase);
        let known = self.merge_panel.as_ref().is_some_and(|panel| {
            name.as_ref()
                .is_some_and(|name| panel.keys.iter().any(|key| key.eq_ignore_ascii_case(name)))
        });
        if known && self.status.ends_with("is not a valid field name.") {
            self.status = format!("Merge field {} inserted", name.unwrap_or_default());
        }
    }

    pub(crate) fn update_merge(&mut self, action: MergeAction) -> Task {
        use MergeAction as A;
        match action {
            A::Step(step) => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.step = Some(step);
                }
            }
            A::Options(open) => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.options = open;
                }
            }
            A::Close => {
                self.leave_merge_preview();
                self.end_viewing();
                self.merge_panel = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            A::Open => {
                let start = self.start_folder();
                return cosmic::task::future(async move {
                    let filter = FileFilter::new("Data files")
                        .glob("*.sam")
                        .glob("*.SAM")
                        .glob("*.amipro")
                        .glob("*.csv")
                        .glob("*.CSV")
                        .glob("*.tsv")
                        .glob("*.txt")
                        .glob("*.dbf")
                        .glob("*.DBF")
                        .glob("*.dif")
                        .glob("*.xlsx")
                        .glob("*.ods");
                    let path = crate::file_dialogs::open_dialog()
                        .title("Open Data File".to_owned())
                        .directory(start)
                        .filter(filter)
                        .open_file()
                        .await
                        .ok()
                        .and_then(|response| response.url().to_file_path().ok());
                    match path {
                        Some(path) => Message::Merge(MergeAction::Chosen(path)),
                        None => Message::Repaint,
                    }
                });
            }
            A::Chosen(path) => {
                // The data file named as a path, one undo step; Ami Pro's
                // defaults for the rest when the document had none.
                let mut settings = self.merge_document().merge().clone();
                let mut file = settings.record_file.clone().unwrap_or_default();
                file.data_file = path.to_string_lossy().into_owned();
                file.description = String::new();
                settings.record_file = Some(file);
                self.change_merge(settings);
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.data = None;
                    // The fields next (MR2).
                    panel.step = Some(crate::merge_panel::Step::MergeDocument);
                }
                return self.read_merge_data(path, None);
            }
            A::Insert(name) => self.insert_merge_code(&format!("MergeField {name}")),
            A::InsertMergeRec => self.insert_merge_code("MergeRec"),
            A::InsertNextRec => self.insert_merge_code("NextRec"),
            A::InsertSkip => {
                let first = self
                    .merge_panel
                    .as_ref()
                    .and_then(|panel| panel.keys.first().cloned())
                    .unwrap_or_else(|| "FIELD".into());
                self.insert_merge_code(&format!("if MergeField {first} = \"\" Skip endif"));
            }
            A::Preview(on) => self.set_preview(on),
            A::Previous | A::Next => {
                let count = self.merge_records().len();
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.position = match action {
                        A::Previous => panel.position.saturating_sub(1),
                        _ => (panel.position + 1).min(count.saturating_sub(1)),
                    };
                }
                self.refresh_merge_preview();
            }
            A::AddCondition => {
                let mut settings = self.merge_document().merge().clone();
                let field = self
                    .merge_panel
                    .as_ref()
                    .and_then(|panel| panel.keys.first().cloned())
                    .unwrap_or_default();
                settings.conditions.push(Condition {
                    field,
                    ..Condition::default()
                });
                settings.with_conditions = true;
                self.change_merge(settings);
            }
            A::RemoveCondition(index) => {
                let mut settings = self.merge_document().merge().clone();
                if index < settings.conditions.len() {
                    settings.conditions.remove(index);
                    if let Some(panel) = self.merge_panel.as_mut()
                        && index < panel.values.len()
                    {
                        panel.values.remove(index);
                    }
                    self.change_merge(settings);
                }
            }
            A::ConditionField(index, field) => {
                let name = self
                    .merge_panel
                    .as_ref()
                    .and_then(|panel| panel.keys.get(field).cloned());
                if let Some(name) = name {
                    self.change_condition(index, |condition| condition.field = name);
                }
            }
            A::ConditionOperator(index, operator) => {
                self.change_condition(index, |condition| condition.operator = operator)
            }
            A::ConditionJoin(index, join) => {
                self.change_condition(index, |condition| condition.join = join)
            }
            A::ConditionValue(index, text) => {
                // A typed value is kept (one undo step) on Enter.
                if let Some(panel) = self.merge_panel.as_mut() {
                    if panel.values.len() <= index {
                        panel.values.resize(index + 1, String::new());
                    }
                    panel.values[index] = text;
                }
            }
            A::CommitValue(index) => {
                let value = self
                    .merge_panel
                    .as_ref()
                    .and_then(|panel| panel.values.get(index).cloned());
                if let Some(value) = value {
                    self.change_condition(index, |condition| condition.value = value);
                }
            }
            A::WithConditions(on) => {
                let mut settings = self.merge_document().merge().clone();
                settings.with_conditions = on;
                self.change_merge(settings);
            }
            A::ToNewDocument => {
                let merged = self.merged_records();
                // Labels: the whole sheets, each label at its place (MC8).
                if let Some((sheet, run)) = self.label_plan(merged.len()) {
                    let labels: Vec<Option<&Document>> = run
                        .iter()
                        .map(|record| {
                            record
                                .and_then(|record| merged.get(record))
                                .map(|record| &record.document)
                        })
                        .collect();
                    let document = amipro_editing::merge::labels::label_sheets(&sheet, &labels);
                    let sheets = run.len().div_ceil(sheet.per_sheet());
                    self.open_in_new_tab(
                        document,
                        &format!(
                            "Merged {} labels on {sheets} sheet{}",
                            run.iter().flatten().count(),
                            if sheets == 1 { "" } else { "s" }
                        ),
                    );
                    return Task::none();
                }
                match amipro_editing::merge::joined(&merged) {
                    Some(document) => {
                        let count = merged.len();
                        self.open_in_new_tab(
                            document,
                            &format!("Merged {count} record{}", if count == 1 { "" } else { "s" }),
                        );
                    }
                    None => self.set_merge_message("No record to merge."),
                }
            }
            A::ToPdf => {
                if self
                    .merge_panel
                    .as_ref()
                    .is_some_and(|panel| panel.pdf_each)
                {
                    if self
                        .merge_panel
                        .as_ref()
                        .is_some_and(|panel| panel.pdf_name_field.is_none())
                    {
                        self.set_merge_message("Choose the field that names each PDF.");
                        return Task::none();
                    }
                    return crate::chosen_folder("Folder for the PDFs", |folder| match folder {
                        Some(folder) => Message::Merge(MergeAction::ToPdfFolderChosen(folder)),
                        None => Message::Repaint,
                    });
                }
                return self.merge_save_task(SaveKind::Pdf);
            }
            A::ToPdfChosen(path) => return self.merge_to_pdf(path),
            A::ToPdfFolderChosen(folder) => return self.merge_to_pdfs(folder),
            A::PdfEach(on) => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.pdf_each = on;
                }
            }
            A::PdfNameField(field) => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.pdf_name_field = Some(field);
                }
            }
            A::ToAmiProFile => return self.merge_save_task(SaveKind::AmiPro),
            A::ToAmiProFileChosen(path) => self.merge_to_ami_pro_file(&path),
            A::ToPrinter => return self.merge_to_printer(),
            A::ViewAndPrint => self.start_viewing(),
            A::PrintAndViewNext => return self.view_next(true),
            A::SkipAndViewNext => return self.view_next(false),
            A::PrintAll => return self.view_print_all(),
            A::CancelViewing => return self.finish_viewing(),
            A::NewDataFile => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.new_names = Some(String::new());
                }
            }
            A::NewDataNames(text) => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.new_names = Some(text);
                }
            }
            A::CancelNewData => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.new_names = None;
                }
            }
            A::CreateDataFile => return self.choose_new_data_file(),
            A::NewDataFileChosen(path) => return self.write_new_data_file(&path),
            A::EditRecords => {
                let path = self
                    .merge_panel
                    .as_ref()
                    .and_then(|panel| panel.data.as_ref())
                    .filter(|data| data.kind.editable())
                    .map(|data| data.path.clone());
                if let Some(path) = path {
                    self.edit_records(&path);
                }
                return Task::none();
            }
            A::Labels(on) => self.set_labels(on),
            A::LabelSheet(index) => {
                if let Some(sheet) = SHEETS.get(index).copied() {
                    if let Some(panel) = self.merge_panel.as_mut() {
                        panel.sheet = Some(index);
                    }
                    self.apply_label_sheet(&sheet);
                }
            }
            A::LabelStart(label) => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.start = label.max(1);
                }
            }
            A::LabelCopies(text) => {
                if let Some(panel) = self.merge_panel.as_mut() {
                    panel.copies = text;
                }
            }
            A::StartFrom(start) => return self.start_from(start),
            A::PdfsWritten { session_id, result } => {
                if let Some(panel) = self
                    .sessions
                    .iter_mut()
                    .find(|session| session.id == session_id)
                    .and_then(|session| session.merge_panel.as_mut())
                {
                    panel.message = Some(match result {
                        Ok(count) => {
                            format!("Wrote {count} PDF{}", if count == 1 { "" } else { "s" })
                        }
                        Err(error) => format!("The PDFs stopped: {error}"),
                    });
                }
                return Task::none();
            }
        }
        // The preview follows the data and conditions as they change.
        if self.merge_preview.is_some() {
            self.refresh_merge_preview();
        }
        Task::none()
    }

    /// The merged records as the set's parts, each laid out.
    fn merged_parts(&self, merged: &[Merged]) -> Vec<crate::print::SetPart> {
        let folder = self.location.folder().map(Path::to_path_buf);
        merged
            .iter()
            .map(|record| crate::print::SetPart {
                layout: crate::page_layout::with_private_shaper(|shaper| {
                    amipro_layout::layout_document(
                        &record.document,
                        shaper,
                        &mut amipro_layout::ShapeCache::default(),
                    )
                }),
                document: record.document.clone(),
                folder: folder.clone(),
                title: format!("Record {}", record.number),
                pages: None,
            })
            .collect()
    }

    /// The labels a merge as labels prints: the sheet and the run (MG7).
    fn label_plan(&self, records: usize) -> Option<(LabelSheet, Vec<Option<usize>>)> {
        let sheet = LabelSheet::of(self.merge_document())?;
        let panel = self.merge_panel.as_ref()?;
        Some((sheet, label_run(records, panel.copies(), panel.start())))
    }

    /// Every chosen record's pages as one print job (labels on their
    /// sheets), with the print options' additions.
    fn merged_job(&mut self, paper: Option<(f32, f32)>) -> Option<crate::print::PdfJob> {
        let merged = self.merged_records();
        if merged.is_empty() {
            self.set_merge_message("No record to merge.");
            return None;
        }
        let options = self.prepared_print(paper)?;
        let set = self.merged_parts(&merged);
        let labels = self.label_plan(set.len());
        Some(crate::print::PdfJob {
            document: self.merge_document().clone(),
            layout: self.pages.layout().clone(),
            folder: self.location.folder().map(Path::to_path_buf),
            set,
            options,
            labels,
        })
    }

    #[cfg(test)]
    pub(crate) fn merged_job_for_tests(&mut self) -> Option<crate::print::PdfJob> {
        self.merged_job(None)
    }

    /// Merge to Printer…: the desktop's print dialog once, for the merged
    /// output's pages (labels: its sheets).
    fn merge_to_printer(&mut self) -> Task {
        let merged = self.merged_records();
        if merged.is_empty() {
            self.set_merge_message("No record to merge.");
            return Task::none();
        }
        let (first, count) = match self.label_plan(merged.len()) {
            Some((sheet, run)) => {
                let (width, height) = sheet.paper.size();
                (
                    (width as f32, height as f32),
                    run.len().div_ceil(sheet.per_sheet()),
                )
            }
            None => {
                let parts = self.merged_parts(&merged);
                let first = parts
                    .first()
                    .and_then(|part| part.layout.pages.first())
                    .map_or((12_240.0, 15_840.0), |page| (page.width, page.height));
                (
                    first,
                    parts.iter().map(|part| part.layout.pages.len()).sum(),
                )
            }
        };
        if let Some(panel) = self.merge_panel.as_mut() {
            panel.printing = true;
        }
        let title = format!("{} merged", self.print_title());
        let session_id = self.id;
        cosmic::task::future(async move {
            let result = crate::print_portal::prepare(title, first, count, 0, false).await;
            Message::PrintPrepared { session_id, result }
        })
    }

    /// The print dialog answered while a merge waits: the chosen pages of
    /// the merged output (counted across it, MC9), or View & Print's
    /// records.
    pub(crate) fn print_merged(
        &mut self,
        prepared: &crate::print_portal::Prepared,
    ) -> Option<Task> {
        let panel = self.merge_panel.as_mut()?;
        if !panel.printing {
            return None;
        }
        panel.printing = false;
        let mut job = match self.viewing_job.take() {
            Some(documents) => {
                let parts = documents
                    .iter()
                    .enumerate()
                    .map(|(index, document)| crate::print::SetPart {
                        layout: crate::page_layout::with_private_shaper(|shaper| {
                            amipro_layout::layout_document(
                                document,
                                shaper,
                                &mut amipro_layout::ShapeCache::default(),
                            )
                        }),
                        document: document.clone(),
                        folder: self.location.folder().map(Path::to_path_buf),
                        title: format!("Record {}", index + 1),
                        pages: None,
                    })
                    .collect();
                let options = self.prepared_print(prepared.paper)?;
                crate::print::PdfJob {
                    document: self.merge_document().clone(),
                    layout: self.pages.layout().clone(),
                    folder: None,
                    set: parts,
                    options,
                    labels: None,
                }
            }
            None => self.merged_job(prepared.paper)?,
        };
        // The dialog's pages count across the whole output.
        if job.labels.is_none() {
            let mut at = 0;
            for part in &mut job.set {
                let count = part.layout.pages.len();
                part.pages = Some(
                    prepared
                        .pages
                        .iter()
                        .filter(|page| (at..at + count).contains(*page))
                        .map(|page| page - at)
                        .collect(),
                );
                at += count;
            }
        }
        let pages = job.pages();
        if pages == 0 {
            self.status = "No pages to print".into();
            return Some(Task::none());
        }
        let title = format!("{} merged", self.print_title());
        let token = prepared.token;
        self.status = format!(
            "Printing {pages} merged page{}…",
            if pages == 1 { "" } else { "s" }
        );
        let session_id = self.id;
        Some(cosmic::task::future(async move {
            let result = match crate::print::in_background(move || job.render()).await {
                Ok(pdf) => crate::print_portal::send(title, pdf, token)
                    .await
                    .map(|()| pages),
                Err(error) => Err(error),
            };
            Message::Printed { session_id, result }
        }))
    }

    /// One PDF per record in `folder`, each named from the chosen field's
    /// value (never over a file: a number is added), off the interface's
    /// thread.
    fn merge_to_pdfs(&mut self, folder: PathBuf) -> Task {
        let merged = self.merged_records();
        let Some((data, field)) = self
            .merge_panel
            .as_ref()
            .and_then(|panel| Some((panel.data.clone()?, panel.pdf_name_field?)))
        else {
            return Task::none();
        };
        if merged.is_empty() {
            self.set_merge_message("No record to merge.");
            return Task::none();
        }
        let Some(options) = self.prepared_print(None) else {
            return Task::none();
        };
        let parts = self.merged_parts(&merged);
        let names: Vec<String> = merged
            .iter()
            .map(|record| {
                let value = record
                    .records
                    .first()
                    .and_then(|first| data.table.records.get(*first))
                    .and_then(|values| values.get(field))
                    .cloned()
                    .unwrap_or_default();
                pdf_file_name(&value, record.number)
            })
            .collect();
        self.set_merge_message(format!("Writing {} PDFs…", parts.len()));
        let session_id = self.id;
        cosmic::task::future(async move {
            let result = crate::print::in_background(move || {
                let mut written = 0;
                let mut taken: Vec<String> = Vec::new();
                for (part, name) in parts.into_iter().zip(names) {
                    let path = unused_path(&folder, &name, &mut taken);
                    let job = crate::print::PdfJob {
                        document: part.document.clone(),
                        layout: part.layout.clone(),
                        folder: part.folder.clone(),
                        set: Vec::new(),
                        options: options.clone(),
                        labels: None,
                    };
                    let pdf = job.render()?;
                    crate::file_workflow::write_new_file_safely(&path, &pdf)
                        .map_err(|error| error.to_string())?;
                    written += 1;
                }
                Ok(written)
            })
            .await;
            Message::Merge(MergeAction::PdfsWritten { session_id, result })
        })
    }

    /// Merge, view & print: the document shows the first record, in place
    /// (MG4).
    fn start_viewing(&mut self) {
        self.leave_merge_preview();
        let records = self.merge_records();
        if records.is_empty() {
            self.set_merge_message("No record to merge.");
            return;
        }
        self.merge_viewing = Some(MergeViewing {
            records,
            position: 0,
            printed: Vec::new(),
        });
        self.show_viewed_record();
    }

    /// The record the viewing is at, worked into the document as it is now
    /// (edits made while viewing carry, as in Ami Pro's one document).
    fn show_viewed_record(&mut self) {
        let Some((records, position)) = self
            .merge_viewing
            .as_ref()
            .map(|viewing| (viewing.records.clone(), viewing.position))
        else {
            return;
        };
        let Some(data) = self
            .merge_panel
            .as_ref()
            .and_then(|panel| panel.data.clone())
        else {
            return;
        };
        let context = self.field_context();
        let mut host = amipro_editing::field_language::NoHost::default();
        let mut inputs = MergeInputs {
            context: &context,
            host: &mut host,
            tidy: false,
            in_place: true,
        };
        if let Some(record) = merge_one(
            &self.document,
            &data.for_merge(),
            &records,
            position,
            &mut inputs,
        ) {
            let previous = std::mem::replace(&mut self.document, record.document);
            let session = &mut self.sessions[self.active_session];
            crate::synchronize_editor_text(&mut session.editor, &previous, &session.document);
            self.refresh_rich_layout();
        }
    }

    /// Print and View Next, or Skip and View Next: the record shown kept to
    /// print (as printed: empty fields left out, MG9) or not, then the next.
    fn view_next(&mut self, print: bool) -> Task {
        if print {
            let mut shown = self.document.clone();
            amipro_editing::merge::drop_empty_field_paragraphs(&mut shown);
            amipro_editing::merge::tidy_empty_fields(&mut shown);
            if let Some(viewing) = self.merge_viewing.as_mut() {
                viewing.printed.push(shown);
            }
        }
        let more = self.merge_viewing.as_mut().is_some_and(|viewing| {
            viewing.position += 1;
            viewing.position < viewing.records.len()
        });
        if more {
            self.show_viewed_record();
            Task::none()
        } else {
            self.finish_viewing()
        }
    }

    /// Print All: the record shown and every later one kept to print.
    fn view_print_all(&mut self) -> Task {
        while self.merge_viewing.is_some() {
            let more = self
                .merge_viewing
                .as_ref()
                .is_some_and(|viewing| viewing.position + 1 < viewing.records.len());
            if !more {
                return self.view_next(true);
            }
            let _ = self.view_next(true);
        }
        Task::none()
    }

    /// The viewing ends: the document shows its fields again (MergeRec 0,
    /// as Ami Pro's after Cancel), and the records kept go to the printer
    /// together (MC7).
    fn finish_viewing(&mut self) -> Task {
        let Some(viewing) = self.merge_viewing.take() else {
            return Task::none();
        };
        self.reset_merge_fields();
        if viewing.printed.is_empty() {
            self.set_merge_message("Viewing ended; nothing was printed.");
            return Task::none();
        }
        let first = viewing
            .printed
            .first()
            .and_then(|document| {
                document
                    .layouts()
                    .first()
                    .map(|layout| (layout.right.width as f32, layout.right.height as f32))
            })
            .unwrap_or((12_240.0, 15_840.0));
        let count = viewing.printed.len();
        self.viewing_job = Some(viewing.printed);
        if let Some(panel) = self.merge_panel.as_mut() {
            panel.printing = true;
        }
        let title = format!("{} merged", self.print_title());
        let session_id = self.id;
        self.set_merge_message(format!(
            "{count} record{} to print",
            if count == 1 { "" } else { "s" }
        ));
        cosmic::task::future(async move {
            let result = crate::print_portal::prepare(title, first, count, 0, false).await;
            Message::PrintPrepared { session_id, result }
        })
    }

    /// Viewing left without printing (the panel closed).
    fn end_viewing(&mut self) {
        if self.merge_viewing.take().is_some() {
            self.reset_merge_fields();
        }
    }

    /// The merge fields show `<NAME>` again, MergeRec 0: the fields worked
    /// outside a merge, not as an undo step.
    fn reset_merge_fields(&mut self) {
        let context = self.field_context();
        let mut host = amipro_editing::field_language::NoHost::default();
        let mut variables = std::collections::BTreeMap::new();
        let mut inputs = amipro_editing::fields::FieldInputs {
            context: &context,
            variables: &mut variables,
            host: &mut host,
        };
        let update = amipro_editing::fields::update_fields(
            &self.document,
            amipro_editing::fields::UpdateScope::All,
            &mut inputs,
        );
        if let Some(command) = update.command
            && let Ok(applied) = amipro_editing::apply(&self.document, &command)
        {
            let previous = std::mem::replace(&mut self.document, applied.document);
            let session = &mut self.sessions[self.active_session];
            crate::synchronize_editor_text(&mut session.editor, &previous, &session.document);
            self.refresh_rich_layout();
        }
    }

    /// New Data File…: the names typed, then where to write it (a new
    /// file only).
    fn choose_new_data_file(&mut self) -> Task {
        let names = self.new_data_names();
        if names.is_empty() {
            self.set_merge_message("Type the field names first, separated by commas.");
            return Task::none();
        }
        let folder = self.start_folder();
        cosmic::task::future(async move {
            let mut dialog = file_chooser::save::Dialog::new()
                .title("New Data File".to_owned())
                .file_name("DATA.SAM".to_owned())
                .filter(
                    FileFilter::new("Ami Pro documents (.sam)")
                        .glob("*.sam")
                        .glob("*.SAM"),
                );
            if let Some(folder) = folder {
                dialog = dialog.directory(folder);
            }
            let path = dialog
                .save_file()
                .await
                .ok()
                .and_then(|response| response.url().and_then(|url| url.to_file_path().ok()));
            match path {
                Some(path) => Message::Merge(MergeAction::NewDataFileChosen(path)),
                None => Message::Repaint,
            }
        })
    }

    fn new_data_names(&self) -> Vec<String> {
        self.merge_panel
            .as_ref()
            .and_then(|panel| panel.new_names.as_ref())
            .map(|names| {
                names
                    .split(',')
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Writes the new data file (Ami Pro's table data file), makes it the
    /// document's data file (one undo step), and opens it as a table. An
    /// existing file the chooser confirmed is written over, its original
    /// kept as `NAME.SAM.bak` the first time (MC6); a file open in a tab
    /// with unsaved changes, or the merge document itself, is not.
    fn write_new_data_file(&mut self, path: &Path) -> Task {
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let open = crate::session_index_for_path(&self.sessions, path);
        if open == Some(self.active_session) {
            return self.new_data_problem(format!(
                "{file_name} is the merge document; choose another name for the data file."
            ));
        }
        if open.is_some_and(|index| self.sessions[index].dirty) {
            return self.new_data_problem(format!(
                "{file_name} is open with unsaved changes; save or close it, then make the data file again."
            ));
        }
        let names = self.new_data_names();
        // A table data file (MR1): the names in its heading row, one empty
        // record to type in.
        let document = amipro_editing::merge::data::table_data_document(
            &amipro_editing::merge::data::DataTable {
                names,
                records: Vec::new(),
            },
            crate::table_commands::one_line(&amipro_document::Document::from_plain_text("")),
        );
        let options = self.sam_options(crate::clock::ami_pro_now());
        let replaced = match crate::file_workflow::replace_sam_document(
            &document,
            path,
            &options,
            self.preferences.keep_backup,
        ) {
            Ok(replaced) => replaced,
            Err(error) => {
                return self.new_data_problem(format!("{error}. The data file was not made."));
            }
        };
        if let Some(panel) = self.merge_panel.as_mut() {
            panel.new_names = None;
            panel.message = None;
        }
        // A tab already showing the file shows the new one.
        if let Some(index) = open {
            let old = &self.sessions[index];
            self.sessions[index] = crate::DocumentSession::new(
                old.id,
                document,
                crate::DocumentLocation::Sam(path.to_path_buf()),
                String::new(),
            );
        }
        let task = self.update_merge(MergeAction::Chosen(path.to_path_buf()));
        self.edit_records(path);
        if let Some(backup) = replaced.backup {
            let backup = backup
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.status = format!(
                "{file_name} was replaced by the new data file; the original is kept as {backup}. Type in the cells; Tab moves on, and Tab in the last cell adds a record"
            );
        }
        task
    }

    /// Why New Data File… made nothing: in the panel and the status bar.
    fn new_data_problem(&mut self, message: String) -> Task {
        self.status = message.clone();
        self.set_merge_message(message);
        Task::none()
    }

    /// Merge as labels on (Avery 5160 unless the page is already a label)
    /// or off, one undo step.
    fn set_labels(&mut self, on: bool) {
        if !on {
            if let Some(command) = labels_off(self.merge_document()) {
                self.leave_merge_preview();
                self.execute_document_command(&command);
            }
            return;
        }
        let index = self
            .merge_panel
            .as_ref()
            .and_then(|panel| panel.sheet)
            .unwrap_or(0);
        if let Some(panel) = self.merge_panel.as_mut() {
            panel.sheet = Some(index);
        }
        self.apply_label_sheet(&SHEETS[index]);
    }

    fn apply_label_sheet(&mut self, sheet: &LabelSheet) {
        self.leave_merge_preview();
        match set_up_labels(self.merge_document(), sheet) {
            Ok(command) => {
                self.execute_document_command(&command);
            }
            Err(error) => self.set_merge_message(error.to_string()),
        }
    }

    /// Start from (MG11): a new document from Ami Pro's merge sheet, with
    /// its contents when it has some, or the Envelope panel.
    fn start_from(&mut self, start: StartFrom) -> Task {
        let (sheet, title) = match start {
            StartFrom::Envelope => return self.open_envelope_panel(),
            StartFrom::Letter => ("_MERGLET.STY", "Merge letter"),
            StartFrom::Labels => ("_LABEL.STY", "Labels"),
        };
        let (document, found) = self.sheet_document(sheet, title);
        self.open_in_new_tab(
            document,
            &if found {
                format!("A new document from {sheet}")
            } else {
                format!("{sheet} was not found: a new document")
            },
        );
        let task = self.open_merge_panel();
        if start == StartFrom::Labels {
            self.set_labels(true);
        }
        task
    }

    /// A new document from Ami Pro's style sheet `name` with its contents
    /// (`_MERGLET.STY`'s letter and fields), else a new document; true when
    /// the sheet was found.
    pub(crate) fn sheet_document(&self, name: &str, title: &str) -> (Document, bool) {
        if let Some(path) = amipro_formats::find_style_sheet(None, name, &self.style_folders())
            && let Ok(bytes) = std::fs::read(&path)
            && let Ok(import) = amipro_formats::import_sam(&bytes, None, title)
        {
            let mut document = import.document;
            document.title = title.to_string();
            return (document, true);
        }
        let mut document = crate::new_document();
        document.title = title.to_string();
        (document, false)
    }

    fn change_condition(&mut self, index: usize, change: impl FnOnce(&mut Condition)) {
        let mut settings = self.merge_document().merge().clone();
        let Some(condition) = settings.conditions.get_mut(index) else {
            return;
        };
        change(condition);
        if let Some(panel) = self.merge_panel.as_mut()
            && let Some(value) = panel.values.get_mut(index)
        {
            *value = settings.conditions[index].value.clone();
        }
        self.change_merge(settings);
    }

    fn merge_save_task(&mut self, kind: SaveKind) -> Task {
        let folder = self.start_folder();
        let title = self.document.title.clone();
        cosmic::task::future(async move {
            let (name, filter) = match kind {
                SaveKind::Pdf => (
                    format!("{title} merged.pdf"),
                    FileFilter::new("PDF documents (.pdf)").glob("*.pdf"),
                ),
                SaveKind::AmiPro => (
                    "MERGED.SAM".to_string(),
                    FileFilter::new("Ami Pro documents (.sam)")
                        .glob("*.sam")
                        .glob("*.SAM"),
                ),
            };
            let mut dialog = file_chooser::save::Dialog::new()
                .title("Merge to".to_owned())
                .file_name(name)
                .filter(filter);
            if let Some(folder) = folder {
                dialog = dialog.directory(folder);
            }
            let path = dialog
                .save_file()
                .await
                .ok()
                .and_then(|response| response.url().and_then(|url| url.to_file_path().ok()));
            match (path, kind) {
                (Some(path), SaveKind::Pdf) => Message::Merge(MergeAction::ToPdfChosen(path)),
                (Some(path), SaveKind::AmiPro) => {
                    Message::Merge(MergeAction::ToAmiProFileChosen(path))
                }
                (None, _) => Message::Repaint,
            }
        })
    }

    /// Merge to PDF: every record's pages in one PDF, written off the
    /// interface's thread (each record a part, its bookmark its number);
    /// labels on their sheets.
    fn merge_to_pdf(&mut self, path: PathBuf) -> Task {
        let Some(job) = self.merged_job(None) else {
            return Task::none();
        };
        let job = crate::print::ExportJob { job, path };
        self.status = "Merging to PDF…".into();
        let session_id = self.id;
        cosmic::task::future(async move {
            let result = crate::print::in_background(move || job.run()).await;
            Message::PdfExported { session_id, result }
        })
    }

    /// Ami Pro's Merge & save as: one SAM, a page break between records,
    /// to a new file only (D3).
    fn merge_to_ami_pro_file(&mut self, path: &Path) {
        let merged = self.merged_records();
        let Some(document) = amipro_editing::merge::joined(&merged) else {
            self.set_merge_message("No record to merge.");
            return;
        };
        let options = self.sam_options(crate::clock::ami_pro_now());
        let bytes = amipro_formats::sam_document(&document, &options);
        match crate::file_workflow::write_new_file_safely(path, &bytes) {
            Ok(()) => self.set_merge_message(format!(
                "Merged {} records into {}",
                merged.len(),
                path.file_name().unwrap_or_default().to_string_lossy()
            )),
            Err(error) => self.set_merge_message(error.to_string()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum SaveKind {
    Pdf,
    AmiPro,
}

/// A PDF's file name from a field's value: characters a file name cannot
/// hold become `_`; an empty value takes the record's number.
pub(crate) fn pdf_file_name(value: &str, number: u32) -> String {
    let cleaned: String = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_control() || "/\\:*?\"<>|".contains(character) {
                '_'
            } else {
                character
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches(['.', ' ']).to_string();
    if cleaned.is_empty() {
        format!("Record {number}")
    } else {
        cleaned
    }
}

/// `folder/name.pdf`, or with " (2)", " (3)", … when that file exists or
/// an earlier record took the name.
pub(crate) fn unused_path(folder: &Path, name: &str, taken: &mut Vec<String>) -> PathBuf {
    let mut candidate = format!("{name}.pdf");
    let mut count = 1;
    while folder.join(&candidate).exists() || taken.contains(&candidate.to_lowercase()) {
        count += 1;
        candidate = format!("{name} ({count}).pdf");
    }
    taken.push(candidate.to_lowercase());
    folder.join(candidate)
}

/// `at`, or after the outermost power field whose value holds it.
fn outside_fields(
    document: &Document,
    at: amipro_editing::TextPosition,
) -> amipro_editing::TextPosition {
    let Some(flow) = document.flow(at.flow) else {
        return at;
    };
    amipro_document::power_fields(flow.paragraphs.iter().map(AsRef::as_ref))
        .into_iter()
        .filter_map(|span| span.end.map(|end| (span.start, end)))
        .filter(|(start, end)| {
            start.paragraph == at.paragraph
                && end.paragraph == at.paragraph
                && start.byte < at.byte
                && at.byte < end.byte
        })
        .map(|(_, end)| end.byte)
        .max()
        .map_or(at, |byte| amipro_editing::TextPosition { byte, ..at })
}
