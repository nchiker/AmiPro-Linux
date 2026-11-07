//! Scenario tests: whole tasks as a user does them, through the window's
//! real message path, with one shared checker after every step (the user,
//! 2026-11-04: "a full test suite to test all parts of the application").
//!
//! A scenario is a [`Scenario`] over the test [`Runtime`]: each step is
//! either an [`Scenario::edit`] (it must change the document, Undo must
//! return exactly the document before it, and Redo the one after) or an
//! [`Scenario::act`] (no undo check: opening, saving, panels, views). After
//! every step [`Scenario::check`] asserts what holds for any document:
//!
//! - `Document::validate` passes, the pages lay out, and the window's view
//!   builds and takes a redraw;
//! - the hidden editor holds the main text;
//! - no power field shows an empty result (it would be invisible) unless
//!   the scenario allows it (a merge's empty value), and no field starts
//!   inside another unless the scenario allows it;
//! - the status bar says nothing that reads as a failure unless the
//!   scenario expects those words ([`Scenario::expect_status`]).
//!
//! [`Scenario::save_sam`] ends a scenario: Save As SAM must write without a
//! loss list (or with exactly the expected one), the file must read back
//! equal, and a fresh window must open it to the same text.
//!
//! Scratch files go under `app/cosmic/target/scenarios/`; the settings are
//! never the user's (`page_app` has none). Anything that opens a desktop
//! dialog (Open…, Save As…, Print…, Export PDF…, Insert › Image…) is never
//! clicked through a running click: send the dialog's finished message, or
//! use `Runtime::click_unrun`.

use std::path::{Path, PathBuf};

use amipro_document::{Document, FlowId, power_fields};
use amipro_formats::roundtrip::{Fact, describe};

use super::*;

mod commands;
mod contents;
mod crash_recovery;
mod documents;
mod drawing;
mod fields;
mod files;
mod find;
mod frames;
mod lists;
mod markdown;
mod master;
mod merge;
mod objects;
mod outline_mode;
mod output;
mod page_layout;
mod pictures;
mod proofing;
mod references;
mod styles;
mod table_data;
mod tables;
mod templates;
mod update_check;
mod views;

/// One scenario's window, its scratch folder, and what it allows.
struct Scenario {
    pub rt: Runtime,
    name: &'static str,
    step: usize,
    /// Status words a step may show without being a failure.
    expected_status: Vec<String>,
    /// Power fields whose result may be empty (a merge's empty values).
    pub allow_empty_fields: bool,
    /// Fields that may start inside another (a code that nests one).
    pub allow_nested_fields: bool,
    /// A log of the steps, printed when a check fails.
    log: Vec<String>,
}

/// Words in the status bar that read as something going wrong.
const FAILURE_WORDS: &[&str] = &[
    "fail",
    "could not",
    "cannot",
    "can't",
    "not a valid",
    "invalid",
    "error",
    "not saved",
    "refused",
    "panic",
    "unknown",
    "nothing to",
];

impl Scenario {
    /// A window on `document` (a new document's, or a fixture's), in a
    /// 1280 × 800 window, with the caret at the start of the main text.
    pub fn new(name: &'static str, document: Document) -> Self {
        let app = page_app(document);
        let mut scenario = Self {
            rt: Runtime::new(app),
            name,
            step: 0,
            expected_status: Vec::new(),
            allow_empty_fields: false,
            allow_nested_fields: false,
            log: Vec::new(),
        };
        scenario.check("the window opened");
        scenario
    }

    /// A new document as File › New makes it.
    pub fn new_document(name: &'static str) -> Self {
        Self::new(name, Document::default())
    }

    /// The scenario's own scratch folder, empty at the start.
    pub fn folder(&self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/scenarios")
            .join(self.name)
    }

    /// Empties the scratch folder (call once at the start when the
    /// scenario writes files).
    pub fn clean_folder(&self) -> PathBuf {
        let folder = self.folder();
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    pub fn app(&self) -> &AmiProApp {
        &self.rt.app
    }

    pub fn app_mut(&mut self) -> &mut AmiProApp {
        &mut self.rt.app
    }

    pub fn document(&self) -> &Document {
        &self.rt.app.document
    }

    pub fn main_flow(&self) -> FlowId {
        self.document().main_flow_id()
    }

    /// Lets a status containing `words` (any case) pass the checker.
    pub fn expect_status(&mut self, words: &str) {
        self.expected_status.push(words.to_lowercase());
    }

    /// Withdraws every status allowance.
    pub fn clear_expected_status(&mut self) {
        self.expected_status.clear();
    }

    // ---- Steps ----------------------------------------------------------

    /// An editing step: it changes the document; Undo returns the document
    /// before it and Redo the one after, with the caret where the step
    /// left it.
    pub fn edit(&mut self, what: &str, step: impl FnOnce(&mut Runtime)) {
        self.begin(what);
        let before = describe(self.document());
        step(&mut self.rt);
        let after = describe(self.document());
        let selection = self.rt.app.page_selection;
        if before == after {
            self.fail(what, "the step changed nothing in the document");
        }
        self.check(what);
        self.rt.send(Message::Command(CommandId::Undo));
        let undone = describe(self.document());
        if undone != before {
            let diff = fact_diff(&before, &undone);
            self.fail(
                what,
                &format!("Undo did not return the document before the step:\n{diff}"),
            );
        }
        self.check(&format!("{what} (undone)"));
        self.rt.send(Message::Command(CommandId::Redo));
        let redone = describe(self.document());
        if redone != after {
            let diff = fact_diff(&after, &redone);
            self.fail(
                what,
                &format!("Redo did not return the document after the step:\n{diff}"),
            );
        }
        if self.rt.app.page_selection != selection {
            let now = self.rt.app.page_selection;
            self.fail(
                what,
                &format!("after Undo and Redo the caret is at {now:?}, not {selection:?}"),
            );
        }
        self.check(&format!("{what} (redone)"));
    }

    /// A step that is not one undoable edit (opening, saving, a panel, a
    /// view, a caret move). The checker still runs after it.
    pub fn act(&mut self, what: &str, step: impl FnOnce(&mut Runtime)) {
        self.begin(what);
        step(&mut self.rt);
        self.check(what);
    }

    /// A step that must leave the document as it was (a refused action, a
    /// view, a caret move); the checker still runs.
    pub fn no_change(&mut self, what: &str, step: impl FnOnce(&mut Runtime)) {
        self.begin(what);
        let before = describe(self.document());
        step(&mut self.rt);
        let after = describe(self.document());
        if before != after {
            let diff = fact_diff(&before, &after);
            self.fail(what, &format!("the step changed the document:\n{diff}"));
        }
        self.check(what);
    }

    fn begin(&mut self, what: &str) {
        self.step += 1;
        self.log.push(format!("{:>3}. {what}", self.step));
    }

    // ---- Common user actions ----------------------------------------------

    /// Types `text` at the caret (one Insert, as a run of keys arrives).
    pub fn type_text(&mut self, text: &str) {
        let text = text.to_owned();
        self.edit(&format!("type {text:?}"), move |rt| {
            rt.send(Message::Page(PageAction::Insert(text)));
        });
    }

    /// Enter.
    pub fn enter(&mut self) {
        self.edit("Enter", |rt| {
            rt.send(Message::Page(PageAction::ParagraphBreak));
        });
    }

    /// A menu or toolbar command that edits the document.
    pub fn command(&mut self, command: CommandId) {
        self.edit(&format!("{command:?}"), move |rt| {
            rt.send(Message::Command(command));
        });
    }

    /// A menu command that does not edit (or edits nothing yet: a caret
    /// format, a panel, a toggle of a view).
    pub fn command_act(&mut self, command: CommandId) {
        self.act(&format!("{command:?}"), move |rt| {
            rt.send(Message::Command(command));
        });
    }

    /// Puts the caret (or, with `extend`, the selection's end) at
    /// `paragraph` and `byte` of `flow`, on `page`.
    pub fn caret(&mut self, flow: FlowId, paragraph: usize, byte: usize, page: u32, extend: bool) {
        self.no_change(
            &format!(
                "caret to {paragraph}:{byte} of {flow:?}{}",
                if extend { " (extend)" } else { "" }
            ),
            move |rt| {
                rt.send(Message::Page(PageAction::MoveTo {
                    position: at(flow, paragraph, byte),
                    page,
                    extend,
                }));
            },
        );
    }

    /// Selects `from` to `to` in the main text (paragraph, byte).
    pub fn select_main(&mut self, from: (usize, usize), to: (usize, usize)) {
        let main = self.main_flow();
        self.caret(main, from.0, from.1, 1, false);
        self.caret(main, to.0, to.1, 1, true);
    }

    /// The caret at the end of the main text.
    pub fn caret_to_end(&mut self) {
        let main = self.main_flow();
        let flow = self.document().flow(main).unwrap();
        let last = flow.paragraphs.len() - 1;
        let byte = flow.paragraphs[last].text.len();
        let page = self.rt.app.pages.layout().pages.len() as u32;
        self.caret(main, last, byte, page, false);
    }

    /// The main text as plain text.
    pub fn main_text(&self) -> String {
        self.document().plain_text()
    }

    // ---- Files ------------------------------------------------------------

    /// File › Save As… to `name` in the scratch folder as SAM, as the
    /// chooser returns it, expecting no loss list. Returns the path.
    pub fn save_sam(&mut self, name: &str) -> PathBuf {
        self.save_sam_expecting(name, &[])
    }

    /// Save As SAM expecting a loss list with exactly entries containing
    /// `losses` (in any order); Save Anyway is then pressed.
    pub fn save_sam_expecting(&mut self, name: &str, losses: &[&str]) -> PathBuf {
        let path = self.folder().join(name);
        std::fs::create_dir_all(self.folder()).unwrap();
        let what = format!("Save As SAM {name}");
        let target = path.clone();
        self.act(&what, move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::SaveDialogFinished {
                session_id,
                result: crate::FileDialogResult::Selected(target),
                format: Some(crate::SaveFormat::Sam),
            });
        });
        self.finish_sam_save(&what, &path, losses);
        path
    }

    /// File › Save (Ctrl+S) on a document that already has a place,
    /// expecting no loss list.
    pub fn save(&mut self) {
        self.save_expecting(&[]);
    }

    pub fn save_expecting(&mut self, losses: &[&str]) {
        let path = self
            .rt
            .app
            .location
            .path()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.fail("Save", "the document has no place to save to"));
        self.act("Save", |rt| {
            rt.send(Message::Command(CommandId::Save));
        });
        self.finish_sam_save("Save", &path, losses);
    }

    fn finish_sam_save(&mut self, what: &str, path: &Path, losses: &[&str]) {
        match (&self.rt.app.sam_save, losses.is_empty()) {
            (None, true) => {}
            (Some(pending), true) => {
                let listed = pending.losses.clone();
                let facts = self.loss_facts();
                self.fail(
                    what,
                    &format!("a loss list where none was expected: {listed:?}\n{facts}"),
                );
            }
            (None, false) => self.fail(what, &format!("no loss list; expected {losses:?}")),
            (Some(pending), false) => {
                let listed = pending.losses.clone();
                let unexpected: Vec<_> = listed
                    .iter()
                    .filter(|loss| !losses.iter().any(|want| loss.contains(want)))
                    .collect();
                let missing: Vec<_> = losses
                    .iter()
                    .filter(|want| !listed.iter().any(|loss| loss.contains(**want)))
                    .collect();
                if !unexpected.is_empty() || !missing.is_empty() {
                    self.fail(
                        what,
                        &format!(
                            "loss list {listed:?}: unexpected {unexpected:?}, missing {missing:?}"
                        ),
                    );
                }
                self.act("Save Anyway", |rt| {
                    rt.send(Message::SaveSamAnyway);
                });
            }
        }
        if !path.exists() {
            let status = self.rt.app.status.clone();
            self.fail(
                what,
                &format!("{} was not written; status {status:?}", path.display()),
            );
        }
        if self.rt.app.dirty {
            self.fail(what, "the tab is still marked changed after the save");
        }
        if losses.is_empty() {
            self.read_back(what, path);
        }
    }

    /// The facts the loss check finds different, as it reads the document
    /// back (what the loss list counts), a line each.
    fn loss_facts(&self) -> String {
        use amipro_formats::roundtrip::{differences, is_writer_addition};
        let document = self.document();
        let Ok(copy) = amipro_formats::read_back(document, &amipro_formats::SamOptions::at(0))
        else {
            return "  (the document does not read back)".into();
        };
        let ours = describe(document);
        let theirs = describe(&copy);
        let full = |facts: &[Fact], place: &str, aspect: &str| {
            facts
                .iter()
                .find(|fact| fact.place == place && fact.aspect == aspect)
                .map(|fact| fact.value.clone())
        };
        // The values from just before where they first differ.
        let shown = |ours: Option<String>, theirs: Option<String>| {
            let (Some(a), Some(b)) = (&ours, &theirs) else {
                return (ours, theirs);
            };
            let start = a
                .char_indices()
                .zip(b.chars())
                .find(|((_, x), y)| x != y)
                .map_or(0, |((index, _), _)| index)
                .saturating_sub(60);
            let start = (0..=start)
                .rev()
                .find(|i| a.is_char_boundary(*i) && b.is_char_boundary(*i))
                .unwrap_or(0);
            let cut = |text: &str| text[start..].chars().take(240).collect::<String>();
            (Some(cut(a)), Some(cut(b)))
        };
        differences(document, &copy)
            .iter()
            .filter(|difference| {
                !is_writer_addition(difference)
                    && !matches!(difference.aspect.as_str(), "line heights" | "face kinds")
            })
            .map(|difference| {
                let (before, after) = shown(
                    full(&ours, &difference.place, &difference.aspect),
                    full(&theirs, &difference.place, &difference.aspect),
                );
                format!(
                    "  {} / {} ({:?}): {before:?} -> {after:?}",
                    difference.place, difference.aspect, difference.change,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The written file read by the importer, as File › Open reads it,
    /// against the session's document: the same facts but what Ami Pro's
    /// writer adds to every file and the save's own statistics.
    fn read_back(&mut self, what: &str, path: &Path) {
        let read = amipro_formats::import_sam_path_with_password(
            path,
            &[],
            self.rt.app.password.as_deref(),
        )
        .unwrap_or_else(|error| self.fail(what, &format!("the file does not read: {error}")));
        let ours = comparable(&describe(self.document()));
        let theirs = comparable(&describe(&read.document));
        if ours != theirs {
            let diff = fact_diff(&ours, &theirs);
            self.fail(what, &format!("the file does not read back equal:\n{diff}"));
        }
    }

    /// File › Open of `path` in a fresh window (as after quitting): the
    /// opened tab must show `path` with the same main text as now. Returns
    /// the new scenario, which carries this one's allowances.
    pub fn reopen(&mut self, path: &Path) -> Scenario {
        let text = self.main_text();
        let mut fresh = Scenario {
            rt: Runtime::new(page_app(Document::default())),
            name: self.name,
            step: self.step,
            expected_status: self.expected_status.clone(),
            allow_empty_fields: self.allow_empty_fields,
            allow_nested_fields: self.allow_nested_fields,
            log: self.log.clone(),
        };
        let target = path.to_path_buf();
        fresh.act(&format!("Open {}", path.display()), move |rt| {
            rt.send(Message::OpenDialogFinished(
                crate::FileDialogResult::Selected(target),
            ));
        });
        if fresh.rt.app.location.path() != Some(path) {
            let location = fresh.rt.app.location.clone();
            fresh.fail("reopen", &format!("the active tab is {location:?}"));
        }
        if fresh.main_text() != text {
            let now = fresh.main_text();
            fresh.fail(
                "reopen",
                &format!("the reopened text differs:\n  before {text:?}\n  after  {now:?}"),
            );
        }
        fresh
    }

    /// File › Open of `path` in this window (a new tab).
    pub fn open(&mut self, path: &Path) {
        let target = path.to_path_buf();
        self.act(&format!("Open {}", path.display()), move |rt| {
            rt.send(Message::OpenDialogFinished(
                crate::FileDialogResult::Selected(target),
            ));
        });
    }

    // ---- The checker ------------------------------------------------------

    /// What must hold after every step.
    pub fn check(&mut self, what: &str) {
        let document = self.rt.app.document.clone();
        if let Err(error) = document.validate() {
            self.fail(what, &format!("the document does not validate: {error:?}"));
        }
        if self.rt.app.pages.layout().pages.is_empty() {
            self.fail(what, "the document laid out no pages");
        }
        // The window's view builds and takes a redraw.
        self.rt.redraw();
        // The hidden editor follows the main text.
        let mut text = self.rt.app.editor.text();
        if text.ends_with('\n') && !document.plain_text().ends_with('\n') {
            text.pop();
        }
        if text != document.plain_text() {
            self.fail(what, "the hidden editor does not hold the main text");
        }
        for (flow, problem) in field_problems(&document) {
            let empty = problem.starts_with("empty");
            if (empty && self.allow_empty_fields) || (!empty && self.allow_nested_fields) {
                continue;
            }
            self.fail(what, &format!("{problem} in {flow:?}"));
        }
        let status = self.rt.app.status.to_lowercase();
        if let Some(word) = FAILURE_WORDS.iter().find(|word| status.contains(*word))
            && !self
                .expected_status
                .iter()
                .any(|allowed| status.contains(allowed))
        {
            let status = self.rt.app.status.clone();
            self.fail(what, &format!("the status bar reads {status:?} ({word:?})"));
        }
        if self.rt.app.save_problem.is_some() {
            let problem = self.rt.app.save_problem.clone();
            self.fail(what, &format!("a save problem is shown: {problem:?}"));
        }
    }

    /// Fails the scenario at `what` with the steps so far.
    pub fn fail(&self, what: &str, message: &str) -> ! {
        panic!(
            "scenario {:?}, step {} ({what}): {message}\nsteps:\n{}\nstatus: {:?}",
            self.name,
            self.step,
            self.log.join("\n"),
            self.rt.app.status
        )
    }
}

/// Facts that must survive a save: all but what the loss check also leaves
/// out (values derived from what is written: the fonts' printer line
/// heights and kinds; sections Ami Pro's writer adds to every file).
fn comparable(facts: &[Fact]) -> Vec<Fact> {
    use amipro_formats::roundtrip::{Change, Difference, is_writer_addition};
    facts
        .iter()
        .filter(|fact| {
            !(fact.place == "document"
                && (matches!(fact.aspect.as_str(), "line heights" | "face kinds")
                    || is_writer_addition(&Difference {
                        place: fact.place.clone(),
                        aspect: fact.aspect.clone(),
                        change: Change::Added,
                    })))
        })
        .map(|fact| {
            // A style sheet saved without contents has header and footer
            // frames without text, kept as such when the file reads its
            // layout from it; the header shows nothing either way.
            let mut fact = fact.clone();
            if fact.place.starts_with("layout ") && fact.aspect == "kept" {
                fact.value = fact
                    .value
                    .replace(", LegacyValue { key: \"txt\", value: \"none\" }", "")
                    .replace("LegacyValue { key: \"txt\", value: \"none\" }, ", "")
                    .replace("[LegacyValue { key: \"txt\", value: \"none\" }]", "[]");
            }
            fact
        })
        .collect()
}

/// The facts that differ between two descriptions, a line each (the first
/// 30).
fn fact_diff(before: &[Fact], after: &[Fact]) -> String {
    let mut lines = Vec::new();
    for fact in before {
        match after
            .iter()
            .find(|other| other.place == fact.place && other.aspect == fact.aspect)
        {
            Some(other) if other.value == fact.value => {}
            Some(other) => lines.push(format!(
                "  {} / {}: {:?} -> {:?}",
                fact.place, fact.aspect, fact.value, other.value
            )),
            None => lines.push(format!(
                "  {} / {}: {:?} gone",
                fact.place, fact.aspect, fact.value
            )),
        }
    }
    for fact in after {
        if !before
            .iter()
            .any(|other| other.place == fact.place && other.aspect == fact.aspect)
        {
            lines.push(format!(
                "  {} / {}: new {:?}",
                fact.place, fact.aspect, fact.value
            ));
        }
    }
    let more = lines.len().saturating_sub(30);
    lines.truncate(30);
    if more > 0 {
        lines.push(format!("  … {more} more"));
    }
    if lines.is_empty() {
        "  (the same facts in another order)".into()
    } else {
        lines.join("\n")
    }
}

/// Power fields that show an empty result, or start inside another field,
/// in every flow of `document`.
fn field_problems(document: &Document) -> Vec<(FlowId, String)> {
    let mut problems = Vec::new();
    for (id, flow) in document.flows() {
        let fields = power_fields(flow.paragraphs.iter().map(AsRef::as_ref));
        for (index, field) in fields.iter().enumerate() {
            if let Some(end) = field.end
                && end.paragraph == field.start.paragraph
                && end.byte == field.start.byte
            {
                problems.push((
                    id,
                    format!(
                        "empty result of field {:?} at paragraph {} byte {}",
                        field.mark.code, field.start.paragraph, field.start.byte
                    ),
                ));
            }
            for outer in &fields[..index] {
                let Some(end) = outer.end else { continue };
                if outer.start < field.start && field.start < end {
                    problems.push((
                        id,
                        format!(
                            "field {:?} inside field {:?} at paragraph {} byte {}",
                            field.mark.code,
                            outer.mark.code,
                            field.start.paragraph,
                            field.start.byte
                        ),
                    ));
                }
            }
        }
    }
    problems
}
