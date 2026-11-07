//! Tools › Equations… and the Equation editor on the page surface
//! (`UI_SPEC.md` §42.5, run D, DO.9): which frame an equation goes in, what
//! each control of the panel and the source card does. Every applied change
//! sets the frame's stored text (`Command::SetEquation`: Ami Pro's two
//! lines, the natural size, the baseline) as one undo step; a run of typing
//! joins the step of the template that began it.

use std::path::PathBuf;
use std::sync::Arc;

use amipro_document::{FrameContent, FrameId, PictureFormat, PictureSource, Placement, Size};
use amipro_editing::{
    Command, NewFrameContent, NewPicture, NewPictureSource, PictureTarget, TextPosition,
};
use amipro_pictures::equation::{self, Stored};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::Task;
use cosmic::iced::widget::text_editor;

use crate::equation_panel::{self as ep, EquationAction, EquationPanel};
use crate::picture_insert::{self, Target};
use crate::{AmiProApp, Message, table_editing};

/// Where Tools › Equations… puts the equation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entry {
    /// An equation frame: edited in place.
    Equation(FrameId),
    /// A selected empty text frame.
    Frame(FrameId),
    /// The caret's empty table cell.
    Cell { table: FrameId, cell: usize },
    /// A new Flow with text frame at the caret.
    New(TextPosition),
    /// A new With para above frame after a table.
    AfterTable(FrameId),
}

impl AmiProApp {
    // ---------------------------------------------------------------- entry

    /// Where Tools › Equations… would put an equation now.
    pub(crate) fn equation_entry(&self) -> Result<Entry, String> {
        match picture_insert::target(
            &self.document,
            self.frame_selection.as_ref(),
            self.page_selection.focus,
        ) {
            Ok(Target::Replace(frame)) => {
                if self.is_equation_frame(frame) {
                    Ok(Entry::Equation(frame))
                } else {
                    Err("Equations needs an empty frame, an equation, or the main text; this frame holds a picture".into())
                }
            }
            Ok(Target::Frame(frame)) => Ok(Entry::Frame(frame)),
            Ok(Target::Cell { table, cell }) => Ok(Entry::Cell { table, cell }),
            Ok(Target::NewFrame(at)) => Ok(Entry::New(at)),
            Err(problem) => match table_editing::caret_cell(&self.document, self.page_selection) {
                Some((table, _, _)) if self.frame_selection.is_none() => {
                    Ok(Entry::AfterTable(table))
                }
                _ => Err(format!(
                    "Equations needs an empty frame, an equation, or the main text ({})",
                    problem.to_lowercase()
                )),
            },
        }
    }

    /// Whether `frame` holds an equation (a picture with the `.tex` driver).
    pub(crate) fn is_equation_frame(&self, frame: FrameId) -> bool {
        matches!(
            self.document.frame(frame).map(|frame| &frame.content),
            Some(FrameContent::Picture(picture)) if picture.format() == PictureFormat::Equation
        )
    }

    /// The stored text of an equation frame (`None`: not an equation, or
    /// its text is not two lines).
    pub(crate) fn equation_stored(&self, frame: FrameId) -> Option<Stored> {
        let FrameContent::Picture(picture) = &self.document.frame(frame)?.content else {
            return None;
        };
        if picture.format() != PictureFormat::Equation {
            return None;
        }
        let PictureSource::Embedded(object) = &picture.source else {
            return None;
        };
        Stored::read(&self.document.objects().get(object)?.bytes)
    }

    /// Tools › Equations…: an equation opens in place; anywhere else a new
    /// empty one starts.
    pub(crate) fn open_equations(&mut self) -> Task<cosmic::Action<Message>> {
        match self.equation_entry() {
            Err(problem) => {
                self.status = problem;
                Task::none()
            }
            Ok(Entry::Equation(frame)) => self.open_equation_frame(frame),
            Ok(entry) => match self.make_equation(entry, &Stored::new("")) {
                Some(frame) => self.open_equation_frame(frame),
                None => Task::none(),
            },
        }
    }

    /// Puts a new equation where `entry` says, as one undo step.
    fn make_equation(&mut self, entry: Entry, stored: &Stored) -> Option<FrameId> {
        let layout = match stored.layout() {
            Ok(layout) => layout,
            Err(error) => {
                self.status = format!("Equation: {error}");
                return None;
            }
        };
        let natural = Size {
            width: layout.width,
            height: layout.height,
        };
        let size = natural;
        let bytes: Arc<[u8]> = stored.bytes().into();
        let cell_style = self.preferences.frame_defaults.cell_frame_style();
        // Ami Pro's equation frame has no line, no shadow, and no margins:
        // it is exactly the equation's size (EQV1, notes §12.2).
        let mut style = self
            .preferences
            .frame_defaults
            .frame_style(amipro_document::Rect::new(0, 0, size.width, size.height));
        style.border.sides = amipro_document::Sides::NONE;
        style.shadow = amipro_document::Shadow {
            right: 0,
            bottom: 0,
            ..style.shadow
        };
        style.interior.margins = amipro_document::Margins::uniform(0);
        style.opaque = false;
        let picture = || NewPicture {
            source: NewPictureSource::Embedded(bytes.clone()),
            extension: ".tex".into(),
            size: natural,
        };
        let before: std::collections::BTreeSet<_> =
            self.document.frames().map(|frame| frame.id).collect();
        let command = match entry {
            Entry::Equation(_) => return None,
            Entry::Frame(frame) => Command::InsertPicture {
                target: PictureTarget::Frame(frame),
                picture: picture(),
                cell_style,
            },
            Entry::Cell { table, cell } => Command::InsertPicture {
                target: PictureTarget::Cell { table, cell },
                picture: picture(),
                cell_style,
            },
            Entry::New(at) => Command::CreateFrame {
                placement: Placement::Inline { y_offset: 0, size },
                style,
                content: NewFrameContent::EmbeddedPicture {
                    bytes: bytes.clone(),
                    extension: ".tex".into(),
                },
                anchor: Some(at),
                layout: None,
            },
            Entry::AfterTable(table) => Command::CreateFrame {
                placement: Placement::BelowParagraph { x_offset: 0, size },
                style: amipro_document::FrameStyle {
                    wrap: amipro_document::Wrap::NoWrapAround,
                    ..style
                },
                content: NewFrameContent::EmbeddedPicture {
                    bytes: bytes.clone(),
                    extension: ".tex".into(),
                },
                anchor: Some(self.after_anchor(table)),
                layout: None,
            },
        };
        let selection = self.page_selection;
        if !self.execute_page_command(&command) {
            return None;
        }
        // A frame after a table or in a cell leaves the caret in the table.
        if matches!(entry, Entry::AfterTable(_) | Entry::Cell { .. }) {
            self.page_selection = crate::page_editing::valid_selection(&self.document, selection);
        }
        let made = match entry {
            Entry::Frame(frame) => Some(frame),
            Entry::Cell { table, cell } => table_editing::table_of(&self.document, table)
                .and_then(|table| table.cells.get(cell))
                .and_then(|cell| cell.frame),
            _ => None,
        }
        .or_else(|| {
            self.document
                .frames()
                .map(|frame| frame.id)
                .find(|id| !before.contains(id))
        })?;
        // The picture's size, baseline and kept words, in the same step.
        let fit = !matches!(entry, Entry::Cell { .. });
        self.amend_page_command(&Command::SetEquation {
            frame: made,
            bytes,
            natural,
            baseline: layout.baseline,
            fit_frame: fit,
        });
        self.status = "Equation made".into();
        Some(made)
    }

    // ---------------------------------------------------------------- panel

    /// Opens the editor on an equation frame (Tools › Equations…, a
    /// double-click, Enter).
    pub(crate) fn open_equation_frame(&mut self, frame: FrameId) -> Task<cosmic::Action<Message>> {
        let mut panel = EquationPanel::new(frame);
        match self.equation_stored(frame) {
            Some(stored) => load_panel(&mut panel, &stored),
            None => panel.unreadable = true,
        }
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.equation_panel = Some(panel);
        self.open_rail();
        self.status = "Equation: type TeX in the card, Esc to finish".into();
        focus_source()
    }

    /// After every message: the editor follows the document (an Undo, a
    /// Redo) and closes when its frame is gone.
    pub(crate) fn sync_equation_panel(&mut self) {
        let Some(frame) = self.equation_panel.as_ref().map(|panel| panel.frame) else {
            return;
        };
        if !self.is_equation_frame(frame) {
            self.equation_panel = None;
            return;
        }
        let Some(stored) = self.equation_stored(frame) else {
            if let Some(panel) = self.equation_panel.as_mut() {
                panel.unreadable = true;
            }
            return;
        };
        let Some(panel) = self.equation_panel.as_mut() else {
            return;
        };
        if panel.unreadable {
            // The text became readable (an Undo, a Redo).
            panel.unreadable = false;
            load_panel(panel, &stored);
            return;
        }
        if stored.tex != panel.last_applied || stored.settings != panel.last_settings {
            load_panel(panel, &stored);
        }
    }

    pub(crate) fn close_equation(&mut self) {
        if self.equation_panel.take().is_some() && self.rail_open() {
            self.toggle_rail();
        }
    }

    // -------------------------------------------------------------- changes

    /// Puts the source on the frame: parsed, laid out, and set as one undo
    /// step (`new_step`), or as part of the open one. A text that does not
    /// parse stays in the card with its error and the frame keeps the last
    /// good drawing.
    pub(crate) fn equation_apply(&mut self, new_step: bool) {
        let Some(panel) = self.equation_panel.as_mut() else {
            return;
        };
        if panel.unreadable {
            return;
        }
        let frame = panel.frame;
        let size = match panel.size.trim().parse::<f64>() {
            Ok(size) if (1.0..=400.0).contains(&size) => size,
            _ => {
                panel.size_error = true;
                return;
            }
        };
        panel.size_error = false;
        let tex = match equation::from_source(&panel.source_text()) {
            Ok(tex) => tex,
            Err(error) => {
                panel.error = Some(error);
                return;
            }
        };
        let mut settings = if panel.last_settings.is_empty() {
            equation::DEFAULT_SETTINGS.to_string()
        } else {
            panel.last_settings.clone()
        };
        let mut stored = Stored {
            settings: settings.clone(),
            tex,
        };
        stored.set_face(&panel.face, size);
        settings = stored.settings.clone();
        let layout = match stored.layout() {
            Ok(layout) => layout,
            Err(error) => {
                panel.error = Some(error);
                return;
            }
        };
        panel.error = None;
        if stored.tex == panel.last_applied && settings == panel.last_settings {
            return;
        }
        let join = panel.run_open && !new_step;
        let command = Command::SetEquation {
            frame,
            bytes: stored.bytes().into(),
            natural: Size {
                width: layout.width,
                height: layout.height,
            },
            baseline: layout.baseline,
            fit_frame: true,
        };
        let applied = if join {
            self.amend_page_command(&command);
            true
        } else {
            self.execute_page_command(&command)
        };
        if let Some(panel) = self.equation_panel.as_mut()
            && applied
        {
            panel.last_applied = stored.tex;
            panel.last_settings = settings;
            panel.run_open = true;
        }
    }

    /// Replaces the whole source (an import, a pasted LaTeX) as its own
    /// step.
    fn equation_set_source(&mut self, source: &str, own_step: bool) {
        let Some(panel) = self.equation_panel.as_mut() else {
            return;
        };
        panel.source = text_editor::Content::with_text(source);
        panel.run_open = !own_step;
        self.equation_apply(own_step);
    }

    pub(crate) fn update_equation(
        &mut self,
        action: EquationAction,
    ) -> Task<cosmic::Action<Message>> {
        match action {
            EquationAction::Edit(action) => {
                let edit = action.is_edit();
                if let Some(panel) = self.equation_panel.as_mut() {
                    panel.source.perform(action);
                }
                if edit {
                    self.equation_apply(false);
                }
            }
            EquationAction::Done | EquationAction::Close => self.close_equation(),
            EquationAction::Insert(template) => {
                if let Some(panel) = self.equation_panel.as_mut() {
                    if panel.unreadable {
                        return Task::none();
                    }
                    insert_template(panel, template);
                }
                self.equation_apply(true);
                return focus_source();
            }
            EquationAction::Face(index) => {
                if let Some(panel) = self.equation_panel.as_mut() {
                    panel.face = ep::FACES[index.min(ep::FACES.len() - 1)].to_string();
                }
                self.equation_apply(true);
            }
            EquationAction::Size(text) => {
                if let Some(panel) = self.equation_panel.as_mut() {
                    panel.size = text;
                    panel.size_error = false;
                }
            }
            EquationAction::SizeSubmit => self.equation_apply(true),
            EquationAction::Import => return self.equation_file_task(false),
            EquationAction::SaveAs => return self.equation_file_task(true),
            EquationAction::CopyLatex => {
                let Some(panel) = self.equation_panel.as_ref() else {
                    return Task::none();
                };
                let latex = equation::source_to_latex(&panel.source_text());
                self.status = "Copied the equation as LaTeX".into();
                return cosmic::iced::clipboard::write(latex);
            }
            EquationAction::PasteLatex => {
                let session_id = self.id;
                return cosmic::iced::clipboard::read().map(move |text| {
                    cosmic::Action::from(Message::EquationClipboard {
                        session_id,
                        text,
                        create: false,
                    })
                });
            }
        }
        Task::none()
    }

    // ------------------------------------------------------------- LaTeX

    /// Tools › Paste LaTeX Equation: the clipboard's LaTeX becomes an
    /// equation (a new one, or the one being edited). Offered, never
    /// automatic (OD11).
    pub(crate) fn paste_latex_equation(&mut self) -> Task<cosmic::Action<Message>> {
        let session_id = self.id;
        cosmic::iced::clipboard::read().map(move |text| {
            cosmic::Action::from(Message::EquationClipboard {
                session_id,
                text,
                create: true,
            })
        })
    }

    pub(crate) fn equation_clipboard(
        &mut self,
        session_id: u64,
        text: Option<String>,
        create: bool,
    ) -> Task<cosmic::Action<Message>> {
        let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.id == session_id)
        else {
            return Task::none();
        };
        self.active_session = index;
        let Some(source) = text.as_deref().and_then(equation::latex_to_source) else {
            self.status = "The clipboard holds no LaTeX equation (it begins with $, \\[ or \\begin{equation})".into();
            return Task::none();
        };
        if let Err(error) = equation::from_source(&source) {
            self.status = format!("The LaTeX equation does not read: {error}");
            return Task::none();
        }
        // A new equation takes the LaTeX in the step that made it.
        let mut own_step = true;
        if self.equation_panel.is_none() && create {
            match self.equation_entry() {
                Ok(Entry::Equation(frame)) => {
                    let _ = self.open_equation_frame(frame);
                }
                Ok(entry) => match self.make_equation(entry, &Stored::new("")) {
                    Some(frame) => {
                        let _ = self.open_equation_frame(frame);
                        own_step = false;
                    }
                    None => return Task::none(),
                },
                Err(problem) => {
                    self.status = problem;
                    return Task::none();
                }
            }
        }
        self.equation_set_source(&source, own_step);
        self.status = "Equation made from LaTeX".into();
        focus_source()
    }

    // -------------------------------------------------------------- files

    fn equation_file_task(&mut self, save: bool) -> Task<cosmic::Action<Message>> {
        let session_id = self.id;
        let start = self.start_folder();
        cosmic::task::future(async move {
            let filter = FileFilter::new("TeX files (.tex)").glob("*.tex");
            let path = if save {
                let mut dialog = file_chooser::save::Dialog::new()
                    .title("Save As Equation".to_owned())
                    .file_name("EQUATION.TEX".to_owned())
                    .filter(filter);
                if let Some(start) = start {
                    dialog = dialog.directory(start);
                }
                dialog
                    .save_file()
                    .await
                    .ok()
                    .and_then(|response| response.url().and_then(|url| url.to_file_path().ok()))
            } else {
                let mut dialog = file_chooser::open::Dialog::new()
                    .title("Import Equation".to_owned())
                    .filter(filter);
                if let Some(start) = start {
                    dialog = dialog.directory(start);
                }
                dialog
                    .open_file()
                    .await
                    .ok()
                    .and_then(|response| response.url().to_file_path().ok())
            };
            Message::EquationFile {
                session_id,
                path,
                save,
            }
        })
    }

    /// A file chosen for Import Equation… (read, normalised as Ami Pro's
    /// import does) or Save As Equation… (Ami Pro's text).
    pub(crate) fn equation_file_chosen(
        &mut self,
        session_id: u64,
        path: Option<PathBuf>,
        save: bool,
    ) {
        let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.id == session_id)
        else {
            return;
        };
        self.active_session = index;
        let Some(path) = path else {
            return;
        };
        if save {
            let Some(panel) = self.equation_panel.as_ref() else {
                return;
            };
            let tex = match equation::from_source(&panel.source_text()) {
                Ok(tex) => tex,
                Err(error) => {
                    self.status = format!("Save As Equation: {error}");
                    return;
                }
            };
            let text = equation::save_as_text(&tex);
            self.status = match std::fs::write(&path, text.as_bytes()) {
                Ok(()) => format!("Equation saved as {}", path.display()),
                Err(error) => format!("Save As Equation failed: {error}"),
            };
            return;
        }
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("Import Equation failed: {error}");
                return;
            }
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        match equation::import_tex(&text) {
            Ok(tex) => {
                self.equation_set_source(&equation::to_source(&tex), true);
                self.status = format!("Equation imported from {}", path.display());
            }
            Err(error) => {
                self.status = format!("Import Equation: {error}");
            }
        }
    }
}

/// The panel's state from the frame's stored text.
fn load_panel(panel: &mut EquationPanel, stored: &Stored) {
    let (face, size) = stored.face();
    panel.source = text_editor::Content::with_text(&equation::to_source(&stored.tex));
    panel.face = face;
    panel.size = format!("{}", size.round() as i64);
    panel.size_error = false;
    panel.error = None;
    panel.last_applied = stored.tex.clone();
    panel.last_settings = stored.settings.clone();
    panel.run_open = false;
    panel.unreadable = false;
    // The caret at the end, where the user goes on typing.
    panel
        .source
        .perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}

/// Puts a template at the caret and selects its first hole.
pub(crate) fn insert_template(panel: &mut EquationPanel, template: &str) {
    panel
        .source
        .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
            Arc::new(template.to_string()),
        )));
    let after = template.chars().skip_while(|c| *c != ep::HOLE).count();
    if after > 0 {
        for _ in 0..after {
            panel
                .source
                .perform(text_editor::Action::Move(text_editor::Motion::Left));
        }
        panel
            .source
            .perform(text_editor::Action::Select(text_editor::Motion::Right));
    }
}

/// A task that puts the keyboard in the source field.
fn focus_source() -> Task<cosmic::Action<Message>> {
    cosmic::iced::runtime::widget::operation::focus::<Message>(cosmic::iced::widget::Id::new(
        ep::SOURCE_ID,
    ))
    .chain(Task::done(Message::Repaint))
    .map(cosmic::Action::from)
}
