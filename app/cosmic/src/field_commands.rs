//! Fields on the page surface (`UI_SPEC.md` §28, decisions FL1–FL7):
//! Update (F9) and Update All (Ctrl+F9), Next and Previous Field, View ›
//! Show Power Fields (Alt+F9) and Field Shading, Lock Field, Auto run
//! fields when a document opens, the Query$ and Decide prompts, Include's
//! files, and the status bar's name for the field at the caret. Each update
//! is one undo step.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use amipro_document::{DocInfoEntry, InlineObject, power_fields, power_flag};
use amipro_editing::field_language::FieldHost;
use amipro_editing::fields::{
    CodesHidden, FieldAt, FieldInputs, UpdateScope, all_fields, codes_hidden, field_at, next_field,
    update_fields,
};

use crate::page_editing::PageSelection;
use crate::{AmiProApp, DocumentSession, page_surface};

/// A prompt a field asks when it is updated (Ami Pro's Query$ and Decide).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldQuestion {
    Query { message: String, default: String },
    Decide { message: String },
}

/// A prompt waiting for the user's answer, and the update to run again
/// with it.
#[derive(Clone, Debug)]
pub struct FieldPrompt {
    pub scope: UpdateScope,
    pub question: FieldQuestion,
    /// What is typed for Query$.
    pub text: String,
    pub queries: BTreeMap<String, String>,
    pub decisions: BTreeMap<String, bool>,
}

/// The prompt dialog's actions.
#[derive(Clone, Debug)]
pub enum PromptAction {
    Text(String),
    Answer(bool),
    Cancel,
}

/// What fields reach outside the document through: Include reads files
/// beside the document, prompts take the answers given so far.
pub struct AppFieldHost<'a> {
    pub folder: Option<&'a Path>,
    pub queries: &'a BTreeMap<String, String>,
    pub decisions: &'a BTreeMap<String, bool>,
    pub questions: Vec<FieldQuestion>,
    pub warnings: Vec<String>,
    /// Each bookmark's page as laid out, by name in small letters
    /// (`PageRef`).
    pub pages: BTreeMap<String, u32>,
}

impl FieldHost for AppFieldHost<'_> {
    fn include(
        &mut self,
        file: &str,
        bookmark: Option<&str>,
        _paragraph_ends: bool,
        _oem: bool,
    ) -> Option<Vec<String>> {
        let path = PathBuf::from(file.replace('\\', "/"));
        let path = if path.is_absolute() {
            path
        } else {
            self.folder?.join(path)
        };
        let path = existing_ignoring_case(&path)?;
        let is_sam = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"));
        if is_sam {
            let document = amipro_formats::import_sam_path(&path).ok()?.document;
            // `Include "file!bookmark"`: that bookmark's text.
            if let Some(name) = bookmark {
                let found = amipro_document::bookmarks::bookmark_named(&document, name)?;
                return Some(amipro_document::bookmarks::bookmark_text(&document, &found));
            }
            return Some(
                document
                    .paragraphs()
                    .iter()
                    .map(|paragraph| {
                        paragraph
                            .text
                            .replace(amipro_document::OBJECT_REPLACEMENT_CHARACTER, "")
                    })
                    .collect(),
            );
        }
        let bytes = std::fs::read(&path).ok()?;
        let text = String::from_utf8(bytes.clone())
            .unwrap_or_else(|_| amipro_document::decode_windows_1252(&bytes));
        Some(text.lines().map(str::to_owned).collect())
    }

    fn bookmark_page(&mut self, name: &str) -> Option<u32> {
        self.pages.get(&name.to_lowercase()).copied()
    }

    fn query(&mut self, message: &str, default: &str) -> Option<String> {
        if let Some(answer) = self.queries.get(message) {
            return Some(answer.clone());
        }
        let question = FieldQuestion::Query {
            message: message.into(),
            default: default.into(),
        };
        if !self.questions.contains(&question) {
            self.questions.push(question);
        }
        None
    }

    fn decide(&mut self, message: &str) -> Option<bool> {
        if let Some(answer) = self.decisions.get(message) {
            return Some(*answer);
        }
        let question = FieldQuestion::Decide {
            message: message.into(),
        };
        if !self.questions.contains(&question) {
            self.questions.push(question);
        }
        None
    }

    fn warn(&mut self, message: String) {
        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }
}

/// The name the status bar gives a field.
pub fn field_name(field: &FieldAt) -> String {
    match field {
        FieldAt::Object { object, .. } => match object {
            InlineObject::Date(date) => match date.source {
                amipro_document::DateSource::Today => "System date".into(),
                amipro_document::DateSource::Time => "System time".into(),
                amipro_document::DateSource::LastSaved => "Date of last revision".into(),
                amipro_document::DateSource::Created => "Date created".into(),
            },
            InlineObject::DocInfo(entry) => format!(
                "Doc Info: {}",
                DocInfoEntry::from_code(entry.code).map_or("(none)".into(), doc_info_label)
            ),
            InlineObject::Seq(seq) => format!("Power field: {}", seq.code),
            _ => String::new(),
        },
        FieldAt::Power { span, .. } => {
            let lock = if span.mark.locked() { " (locked)" } else { "" };
            format!("Power field: {}{lock}", span.mark.code)
        }
    }
}

/// Insert Doc Info Field's names for the entries (`1338:0131`).
pub fn doc_info_label(entry: DocInfoEntry) -> String {
    match entry {
        DocInfoEntry::FileName => "Filename".into(),
        DocInfoEntry::Path => "Path".into(),
        DocInfoEntry::StyleSheet => "Style Sheet".into(),
        DocInfoEntry::DateCreated => "Date Created".into(),
        DocInfoEntry::DateLastRevised => "Date Last Revised".into(),
        DocInfoEntry::TotalRevisions => "Total Revisions".into(),
        DocInfoEntry::Description => "Document Description".into(),
        DocInfoEntry::Field(index) => format!("Field{}", index + 1),
        DocInfoEntry::TimeCreated => "Time Created".into(),
        DocInfoEntry::TimeLastRevised => "Time Last Revised".into(),
        DocInfoEntry::EditingTime => "Total Editing Time".into(),
        DocInfoEntry::Pages => "Number of Pages".into(),
        DocInfoEntry::Words => "Number of Words".into(),
        DocInfoEntry::Characters => "Number of Chars".into(),
        DocInfoEntry::Size => "Size of Document".into(),
    }
}

impl DocumentSession {
    /// The fields and their shading, after every change.
    pub(crate) fn refresh_field_views(&mut self) {
        let mut shading = page_surface::Highlights::new();
        let fields = all_fields(&self.document);
        for field in &fields {
            let (start, end) = field.range();
            if start.paragraph == end.paragraph && start.byte < end.byte {
                shading
                    .entry((start.flow, start.paragraph))
                    .or_default()
                    .push(start.byte..end.byte);
            }
        }
        self.field_shading = shading;
        self.fields = fields;
    }

    /// Whether power fields show their codes (View › Show Power Fields).
    pub(crate) fn codes_shown(&self) -> bool {
        self.document.shown_codes().is_some()
    }

    /// Shows or hides the codes as one undo step (`fields::swap_codes`),
    /// the selection moving with the text the fields swap.
    pub(crate) fn swap_codes(&mut self, show: bool) -> bool {
        let swaps = amipro_editing::fields::code_swaps(&self.document, show);
        let command = if show {
            amipro_editing::Command::ShowPowerFields
        } else {
            amipro_editing::Command::HidePowerFields
        };
        let selection = self.page_selection;
        if !self.execute_field_step(&command) {
            return false;
        }
        self.page_selection = crate::page_editing::valid_selection(
            &self.document,
            PageSelection {
                anchor: swaps.map(selection.anchor),
                focus: swaps.map(selection.focus),
                page: selection.page,
            },
        );
        self.sync_editor_to_page();
        true
    }

    /// A field command that changes text, as one undo step, the editor's
    /// text kept with it.
    pub(crate) fn execute_field_step(&mut self, command: &amipro_editing::Command) -> bool {
        let previous = self.document.clone();
        if !self.execute_document_command(command) {
            return false;
        }
        self.page_selection =
            crate::page_editing::valid_selection(&self.document, self.page_selection);
        crate::synchronize_editor_text(&mut self.editor, &previous, &self.document);
        self.sync_editor_to_page();
        true
    }
}

/// An update's scope in the document with its codes hidden.
fn hidden_scope(scope: UpdateScope, hidden: &CodesHidden) -> UpdateScope {
    match scope {
        UpdateScope::At(at) => UpdateScope::At(hidden.map(at)),
        UpdateScope::Between(start, end) => {
            UpdateScope::Between(hidden.map(start), hidden.map(end))
        }
        other => other,
    }
}

/// Edit Field's change of a field shown as its code, for the document with
/// the codes hidden: the field found where its start mark is.
fn hidden_field_change(
    command: amipro_editing::Command,
    hidden: &CodesHidden,
) -> Option<amipro_editing::Command> {
    let amipro_editing::Command::SetFieldMarks {
        flow, start, mark, ..
    } = command
    else {
        return Some(command);
    };
    let at = hidden.map(amipro_editing::TextPosition {
        flow,
        paragraph: start.paragraph,
        byte: start.byte,
    });
    match field_at(&hidden.document, at)? {
        FieldAt::Power { flow, span } => Some(amipro_editing::Command::SetFieldMarks {
            flow,
            start: span.start,
            end: span.end,
            mark: amipro_document::PowerMark { end: false, ..mark },
        }),
        FieldAt::Object {
            at,
            object: InlineObject::Seq(field),
        } => Some(amipro_editing::Command::SetFieldObject {
            at,
            object: InlineObject::Seq(amipro_document::SeqField {
                code: mark.code,
                value: field.value,
            }),
        }),
        FieldAt::Object { .. } => None,
    }
}

impl AmiProApp {
    /// The field the caret is in, on the page surface.
    pub(crate) fn field_at_caret(&self) -> Option<FieldAt> {
        if !self.page_surface || self.frame_selection.is_some() {
            return None;
        }
        // The view asks several times a frame: the fields found after the
        // last change answer it, and only a caret in one walks its flow.
        let caret = self.page_selection.focus;
        let place = (caret.paragraph, caret.byte);
        let near = self.fields.iter().any(|field| {
            let (start, end) = field.range();
            start.flow == caret.flow
                && (start.paragraph, start.byte) <= place
                && place <= (end.paragraph, end.byte)
        });
        near.then(|| field_at(&self.document, caret)).flatten()
    }

    /// Runs an update of `scope` with the answers given so far; a prompt
    /// not yet answered opens its dialog, and the update runs again when it
    /// is answered (`1648:02a0`).
    pub(crate) fn run_field_update(
        &mut self,
        scope: UpdateScope,
        queries: BTreeMap<String, String>,
        decisions: BTreeMap<String, bool>,
    ) {
        let context = self.field_context();
        let folder = self.location.folder().map(Path::to_path_buf);
        let mut host = AppFieldHost {
            folder: folder.as_deref(),
            queries: &queries,
            decisions: &decisions,
            questions: Vec::new(),
            warnings: Vec::new(),
            pages: self.bookmark_pages(),
        };
        let mut variables = std::mem::take(&mut self.field_variables);
        // While codes show, they are read back first; Update All leaves
        // them off, as Ami Pro's does.
        let hidden = codes_hidden(&self.document);
        let mut update = {
            let mut inputs = FieldInputs {
                context: &context,
                variables: &mut variables,
                host: &mut host,
            };
            match &hidden {
                Some(hidden) => {
                    update_fields(&hidden.document, hidden_scope(scope, hidden), &mut inputs)
                }
                None => update_fields(&self.document, scope, &mut inputs),
            }
        };
        self.field_variables = variables;
        if let Some(question) = host.questions.first().cloned() {
            let text = match &question {
                FieldQuestion::Query { default, .. } => default.clone(),
                FieldQuestion::Decide { .. } => String::new(),
            };
            self.field_prompt = Some(FieldPrompt {
                scope,
                question,
                text,
                queries,
                decisions,
            });
            return;
        }
        let warning = host.warnings.first().cloned();
        if let Some(hidden) = &hidden {
            update.command = hidden.step(
                &self.document,
                update.command.take().into_iter().collect(),
                scope != UpdateScope::All,
            );
        }
        let changed = update
            .command
            .as_ref()
            .is_some_and(|command| self.execute_field_step(command));
        self.status = match (warning, changed, update.updated) {
            (Some(warning), _, _) => warning,
            (None, true, count) => format!(
                "Updated {count} {}",
                if count == 1 { "field" } else { "fields" }
            ),
            (None, false, 0) if scope != UpdateScope::All => {
                "There is no field here to update".into()
            }
            (None, false, _) => "The fields are up to date".into(),
        };
    }

    /// Edit › Power Fields ▸ Update Fields (F9): the fields in the selection, or the one
    /// at the caret.
    pub(crate) fn update_fields_command(&mut self) {
        let (start, end) = self.page_selection.ordered();
        let scope = if start == end {
            UpdateScope::At(start)
        } else {
            UpdateScope::Between(start, end)
        };
        self.run_field_update(scope, BTreeMap::new(), BTreeMap::new());
    }

    /// The prompt dialog's answer: the update runs again with it.
    pub(crate) fn answer_field_prompt(&mut self, action: PromptAction) {
        let Some(mut prompt) = self.field_prompt.take() else {
            return;
        };
        match action {
            PromptAction::Text(text) => {
                prompt.text = text;
                self.field_prompt = Some(prompt);
            }
            PromptAction::Cancel => self.status = "The fields were not updated".into(),
            PromptAction::Answer(answer) => {
                match &prompt.question {
                    FieldQuestion::Query { message, .. } => {
                        prompt.queries.insert(message.clone(), prompt.text.clone());
                    }
                    FieldQuestion::Decide { message } => {
                        prompt.decisions.insert(message.clone(), answer);
                    }
                }
                self.run_field_update(prompt.scope, prompt.queries, prompt.decisions);
            }
        }
    }

    /// Fields with Auto run, as a document opens (`1648:0c3b`).
    pub(crate) fn auto_run_fields(&mut self) {
        let any = self.document.flows().any(|(_, flow)| {
            power_fields(flow.paragraphs.iter().map(AsRef::as_ref))
                .iter()
                .any(|field| field.mark.auto_run())
        });
        if any {
            self.run_field_update(UpdateScope::AutoRun, BTreeMap::new(), BTreeMap::new());
        }
    }

    /// Edit › Next Field and Previous Field: the whole field selected.
    pub(crate) fn go_to_field(&mut self, forward: bool) {
        let at = if forward {
            self.page_selection.ordered().1
        } else {
            self.page_selection.ordered().0
        };
        match next_field(&self.document, at, forward) {
            Some(field) => {
                let (start, end) = field.range();
                let page = self.page_selection.page;
                self.page_selection = PageSelection {
                    anchor: start,
                    focus: end,
                    page,
                };
                self.frame_selection = None;
                self.status = field_name(&field);
            }
            None => self.status = "The document has no fields".into(),
        }
    }

    /// View › Show Power Fields (Alt+F9): the codes in red in place of the
    /// results, to be edited in the text; off, the codes are read back and
    /// the results return (`fields::swap_codes`).
    pub(crate) fn toggle_show_codes(&mut self) {
        let show = !self.codes_shown();
        self.swap_codes(show);
        self.status = if show {
            "Power fields show their codes: edit them, then Update or Show Power Fields".into()
        } else {
            "Power fields show their results".into()
        };
    }

    /// Edit › Power Fields ▸ Lock Field: the caret's power field kept as it is by updates.
    pub(crate) fn toggle_field_lock(&mut self) {
        let Some(FieldAt::Power { flow, span }) = self
            .field_at_caret()
            .filter(|field| !matches!(field, FieldAt::Power { span, .. } if span.mark.kind == amipro_document::power_kind::SEQ))
        else {
            self.status = "Only power fields lock".into();
            return;
        };
        let mark = amipro_document::PowerMark {
            value: span.mark.value ^ power_flag::LOCK,
            ..span.mark.clone()
        };
        let locked = mark.locked();
        if self.execute_document_command(&amipro_editing::Command::SetFieldMarks {
            flow,
            start: span.start,
            end: span.end,
            mark,
        }) {
            self.status = if locked {
                "The field is locked: updates leave it".into()
            } else {
                "The field is unlocked".into()
            };
        }
    }

    /// The prompt's dialog (Query$, Decide), while one waits.
    pub(crate) fn field_prompt_dialog(&self) -> Option<cosmic::Element<'_, crate::Message>> {
        use cosmic::widget;
        let prompt = self.field_prompt.as_ref()?;
        let send = crate::Message::FieldPrompt;
        Some(match &prompt.question {
            FieldQuestion::Query { message, .. } => widget::dialog()
                .title("Power field")
                .body(message.clone())
                .control(
                    widget::text_input("", prompt.text.clone())
                        .on_input(move |text| send(PromptAction::Text(text)))
                        .on_submit(move |_| send(PromptAction::Answer(true))),
                )
                .primary_action(
                    widget::button::suggested("OK").on_press(send(PromptAction::Answer(true))),
                )
                .secondary_action(
                    widget::button::standard("Cancel").on_press(send(PromptAction::Cancel)),
                )
                .into(),
            FieldQuestion::Decide { message } => widget::dialog()
                .title("Power field")
                .body(message.clone())
                .primary_action(
                    widget::button::suggested("Yes").on_press(send(PromptAction::Answer(true))),
                )
                .secondary_action(
                    widget::button::standard("No").on_press(send(PromptAction::Answer(false))),
                )
                .tertiary_action(
                    widget::button::text("Cancel").on_press(send(PromptAction::Cancel)),
                )
                .into(),
        })
    }
}

/// `path`, or the file in its folder with its name in other capitals, as
/// DOS names in documents may have them.
fn existing_ignoring_case(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    let name = path.file_name()?.to_str()?;
    std::fs::read_dir(path.parent()?)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|candidate| {
            candidate
                .file_name()
                .and_then(|other| other.to_str())
                .is_some_and(|other| other.eq_ignore_ascii_case(name))
        })
}

impl AmiProApp {
    /// Opens the Fields panel in the rail at `page` (Insert › Field…,
    /// Insert › Date/Time…, Edit Field…).
    pub(crate) fn open_fields_panel(&mut self, page: crate::fields_panel::Page) {
        let at_caret = self.field_at_caret();
        self.close_rail_panels();
        self.fields_panel = Some(crate::fields_panel::FieldsPanel::new(page, at_caret));
        self.open_rail();
    }

    /// Opens Document Properties in the rail at `section` (File ›
    /// Document Properties…, Tools › Word Count).
    pub(crate) fn open_document_properties(
        &mut self,
        section: crate::document_properties::Section,
    ) {
        let properties =
            crate::document_properties::DocumentProperties::new(&self.document, section);
        self.close_rail_panels();
        self.document_properties = Some(properties);
        self.open_rail();
    }

    /// The two panels follow the caret and the document after every
    /// message, and go with the rail.
    pub(crate) fn sync_field_panels(&mut self) {
        if !self.rail_open() {
            self.fields_panel = None;
            self.document_properties = None;
            self.print_panel = None;
            self.import_report_shown = false;
            return;
        }
        let at_caret = self.field_at_caret();
        let session = &mut self.sessions[self.active_session];
        if let Some(panel) = session.fields_panel.as_mut() {
            panel.sync(at_caret);
        }
        if let Some(panel) = session.document_properties.as_mut() {
            panel.sync(&session.document);
        }
    }

    fn close_field_panels(&mut self, close_rail: bool) {
        self.fields_panel = None;
        self.document_properties = None;
        if close_rail && self.rail_open() {
            self.toggle_rail();
        }
    }

    /// The Fields panel's actions: each insertion and change one undo step.
    pub(crate) fn update_fields_panel(&mut self, action: crate::fields_panel::FieldsAction) {
        use crate::fields_panel::Outcome;
        let saved = self.preferences.saved_fields.clone();
        let context = self.sessions[self.active_session].field_context();
        let Some(panel) = self.sessions[self.active_session].fields_panel.as_mut() else {
            return;
        };
        let today = panel.today_text(&context);
        let outcome = panel.update(action, &saved);
        let (start, end) = self.page_selection.ordered();
        match outcome {
            Outcome::Stay => {}
            Outcome::Back => self.close_field_panels(false),
            Outcome::Close => self.close_field_panels(true),
            Outcome::InsertText => {
                self.execute_page_commands(&[amipro_editing::Command::ReplaceText {
                    start,
                    end,
                    text: today,
                }]);
                self.status = "Today's date inserted as text".into();
            }
            Outcome::InsertObject(object) => {
                let mut commands = Vec::new();
                if start != end {
                    commands.push(amipro_editing::Command::ReplaceText {
                        start,
                        end,
                        text: String::new(),
                    });
                }
                commands.push(amipro_editing::Command::InsertFieldObject { at: start, object });
                self.execute_page_commands(&commands);
                self.status = "Field inserted".into();
            }
            Outcome::InsertPower { code, flags } => {
                self.insert_power_field_at(start, end, &code, flags)
            }
            Outcome::SetObject(at, object) => {
                self.execute_document_command(&amipro_editing::Command::SetFieldObject {
                    at,
                    object,
                });
            }
            Outcome::SetMarks(command) => {
                // While codes show, the change goes to the field with them
                // hidden, and its new code shows.
                let command = match codes_hidden(&self.document) {
                    Some(hidden) => hidden_field_change(*command, &hidden)
                        .and_then(|command| hidden.step(&self.document, vec![command], true)),
                    None => Some(*command),
                };
                if command.is_some_and(|command| self.execute_field_step(&command)) {
                    self.status = "Field changed: Update to show its new result".into();
                }
            }
            Outcome::UpdateThis => self.update_fields_command(),
            Outcome::Save(name, code) => {
                self.preferences
                    .saved_fields
                    .retain(|(saved, _)| *saved != name);
                self.preferences.saved_fields.push((name.clone(), code));
                self.save_preferences();
                self.status = format!("Saved the field as {name}");
            }
        }
    }

    /// A power field put in at the caret, worked out at once; its prompts
    /// asked as it is inserted, as Ami Pro asks them.
    pub(crate) fn insert_power_field_at(
        &mut self,
        start: amipro_editing::TextPosition,
        end: amipro_editing::TextPosition,
        code: &str,
        flags: u16,
    ) {
        let context = self.field_context();
        let folder = self.location.folder().map(Path::to_path_buf);
        let (queries, decisions) = (BTreeMap::new(), BTreeMap::new());
        let mut host = AppFieldHost {
            folder: folder.as_deref(),
            queries: &queries,
            decisions: &decisions,
            questions: Vec::new(),
            warnings: Vec::new(),
            pages: self.bookmark_pages(),
        };
        let mut variables = std::mem::take(&mut self.field_variables);
        // While codes show, the field goes in with them hidden, and its code
        // shows.
        let hidden = codes_hidden(&self.document);
        let (start, end) = match &hidden {
            Some(hidden) => (hidden.map(start), hidden.map(end)),
            None => (start, end),
        };
        let command = {
            let mut inputs = FieldInputs {
                context: &context,
                variables: &mut variables,
                host: &mut host,
            };
            amipro_editing::fields::insert_power_field(
                hidden
                    .as_ref()
                    .map_or(&self.document, |hidden| &hidden.document),
                start,
                code,
                flags,
                &mut inputs,
            )
        };
        self.field_variables = variables;
        let Ok(command) = command else {
            self.status = "The field cannot go there".into();
            return;
        };
        // A field whose result is empty would show nothing on the page
        // (an invisible field): it is not put in, and the status says so.
        if let amipro_editing::Command::InsertPowerField { result, mark, .. } = &command
            && result.is_empty()
            && amipro_document::has_result(mark.kind)
        {
            self.status =
                format!("Not inserted: {code} gives no text now, so the field would show nothing");
            return;
        }
        let mut commands = Vec::new();
        if start != end {
            commands.push(amipro_editing::Command::ReplaceText {
                start,
                end,
                text: String::new(),
            });
        }
        commands.push(command);
        match &hidden {
            Some(hidden) => {
                if let Some(step) = hidden.step(&self.document, commands, true) {
                    self.execute_page_commands(&[step]);
                }
            }
            None => self.execute_page_commands(&commands),
        }
        self.status = host
            .warnings
            .first()
            .cloned()
            .unwrap_or_else(|| "Power field inserted".into());
        if !host.questions.is_empty() {
            let caret = self.page_selection.focus;
            self.run_field_update(UpdateScope::At(caret), BTreeMap::new(), BTreeMap::new());
        }
    }

    /// Document Properties' actions: each change one undo step.
    pub(crate) fn update_document_properties(
        &mut self,
        action: crate::document_properties::PropertiesAction,
    ) {
        use crate::document_properties::Outcome;
        let session = &mut self.sessions[self.active_session];
        let Some(panel) = session.document_properties.as_mut() else {
            return;
        };
        match panel.update(action, &session.document) {
            Outcome::Stay => {}
            Outcome::Back => self.close_field_panels(false),
            Outcome::Close => self.close_field_panels(true),
            Outcome::Apply(info) => {
                if self.execute_document_command(&amipro_editing::Command::SetDocumentInfo(info)) {
                    self.status = "Document properties changed".into();
                }
            }
        }
    }

    pub(crate) fn fields_panel_view(&self) -> cosmic::Element<'_, crate::Message> {
        use crate::fields_panel::{Page, Shown, doc_info_labels, panel};
        let panel_state = self.fields_panel.as_ref().expect("the Fields panel shows");
        let context = self.field_context();
        let document = &self.document;
        let doc_info = doc_info_labels(
            |index| document.info().field_name(index),
            |object| match object {
                InlineObject::DocInfo(entry) => context.doc_info_text(entry, document),
                _ => String::new(),
            },
        );
        let preview = match panel_state.page {
            Page::DateTime | Page::DocInfo | Page::Power | Page::ThisField => {
                self.panel_preview(panel_state, &context)
            }
        };
        let this_shows = self
            .field_at_caret()
            .map(|field| self.field_shows(&field, &context));
        panel(
            panel_state,
            Shown {
                context,
                preview,
                this_shows,
                doc_info,
                saved: &self.preferences.saved_fields,
            },
        )
    }

    /// What the Insert section's choice would show now.
    fn panel_preview(
        &self,
        panel_state: &crate::fields_panel::FieldsPanel,
        context: &amipro_document::FieldContext,
    ) -> String {
        use crate::fields_panel::{FieldsAction, Outcome};
        // The choice as it would be inserted, worked out without changing
        // anything.
        let mut probe = crate::fields_panel::FieldsPanel::new(panel_state.page, None);
        probe.clone_choice_from(panel_state);
        match probe.update(FieldsAction::Insert, &[]) {
            Outcome::InsertText => panel_state.today_text(context),
            Outcome::InsertObject(object) => self.object_shows(&object, context),
            Outcome::InsertPower { code, .. } => self.code_shows(&code, context),
            _ => String::new(),
        }
    }

    fn object_shows(
        &self,
        object: &InlineObject,
        context: &amipro_document::FieldContext,
    ) -> String {
        match object {
            InlineObject::Date(date) => context.date_text(date, self.document.info()),
            InlineObject::DocInfo(entry) => context.doc_info_text(entry, &self.document),
            InlineObject::Seq(seq) => seq.text(),
            _ => String::new(),
        }
    }

    /// A code worked out as a preview: the variables copied, prompts not
    /// asked, nothing changed.
    fn code_shows(&self, code: &str, context: &amipro_document::FieldContext) -> String {
        use amipro_editing::field_language::{FieldEnvironment, NoHost, Outcome, evaluate};
        let mut variables = self.field_variables.clone();
        let bookmarks = amipro_editing::fields::bookmark_names(&self.document);
        let mut host = NoHost::default();
        let mut environment = FieldEnvironment {
            document: &self.document,
            context,
            variables: &mut variables,
            bookmarks: &bookmarks,
            host: &mut host,
        };
        match evaluate(code, &mut environment) {
            Outcome::Result(paragraphs) => paragraphs.join(" / "),
            Outcome::Nothing => "(nothing: the field shows no result)".into(),
            Outcome::Kept => "(not worked out here: the result is kept)".into(),
            Outcome::Waiting => "(asks when it is updated)".into(),
        }
    }

    fn field_shows(&self, field: &FieldAt, context: &amipro_document::FieldContext) -> String {
        match field {
            FieldAt::Object { object, .. } => self.object_shows(object, context),
            FieldAt::Power { flow, span } => match span.end {
                Some(end) if end.paragraph == span.start.paragraph => self
                    .document
                    .flow(*flow)
                    .and_then(|flow| flow.paragraph(end.paragraph))
                    .map(|paragraph| paragraph.text[span.start.byte..end.byte].to_owned())
                    .unwrap_or_default(),
                _ => "(no result)".into(),
            },
        }
    }

    pub(crate) fn document_properties_view(&self) -> cosmic::Element<'_, crate::Message> {
        let panel_state = self
            .document_properties
            .as_ref()
            .expect("Document Properties shows");
        let context = self.field_context();
        crate::document_properties::panel(
            panel_state,
            &self.document,
            crate::document_properties::Facts {
                file_name: context.file_name.clone(),
                folder: context.directory.clone(),
                pages: context.pages,
                saved: context.saved,
                session_minutes: context.session_minutes,
            },
        )
    }
}

impl AmiProApp {
    /// Each bookmark's page as the pages are laid out now, for PageRef.
    pub(crate) fn bookmark_pages(&self) -> BTreeMap<String, u32> {
        crate::reference_commands::bookmark_pages(&self.document, self.pages.layout())
    }
}
