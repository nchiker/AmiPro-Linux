//! Draw mode in the application (`UI_SPEC.md` §33, decisions DW1–DW9):
//! entering it (Insert › Drawing…, a double-click or Enter on a drawing),
//! the page surface's events and the Draw menu's commands applied as one
//! step each (`Command::SetDrawing`), leaving it (Esc, Done, a click
//! outside), and the draw bar in the toolbar's place (DW3).

use std::sync::Arc;

use amipro_document::{FrameContent, FrameId, PictureFormat, Size};
use amipro_editing::drawing::{Align, Order, StyleChange};
use amipro_editing::{Command, drawing};
use amipro_pictures::sdw::{self, Drawing};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::command::CommandId;
use crate::draw_mode::{self, DrawMode, Event, Outcome, Tool};
use crate::frame_editing::FrameSelection;
use crate::page_surface::DrawView;
use crate::{AmiProApp, DocumentSession, Message};

/// The status bar while Draw mode is open.
pub(crate) const DRAW_NOTE: &str = "Drawing: Esc or Done returns to the document";

impl DocumentSession {
    /// The objects of the drawing Draw mode edits.
    pub(crate) fn draw_drawing(&self) -> Option<Drawing> {
        drawing::drawing_of(&self.document, self.draw.as_ref()?.frame)
    }

    /// Where Draw mode's drawing is on the pages: its frame's picture area
    /// and the picture's place in it (`1008:220a`: the drawing stretched
    /// onto the picture's rectangle), and its objects.
    pub(crate) fn draw_placement(&self) -> Option<(draw_mode::Placement, Drawing)> {
        let mode = self.draw.as_ref()?;
        let FrameContent::Picture(picture) = &self.document.frame(mode.frame)?.content else {
            return None;
        };
        let drawing = drawing::drawing_of(&self.document, mode.frame)?;
        let natural = drawing.natural_size();
        let natural = Size {
            width: natural.width.max(1),
            height: natural.height.max(1),
        };
        let layout = self.pages.layout();
        let (page_index, frame_box) =
            layout.pages.iter().enumerate().find_map(|(index, page)| {
                Some((
                    index,
                    page.frames.iter().find(|frame| frame.frame == mode.frame)?,
                ))
            })?;
        {
            let area = crate::page_surface::picture_area(frame_box);
            let shown = picture.scaling.drawn_size(
                natural,
                Size {
                    width: area.width.round() as i32,
                    height: area.height.round() as i32,
                },
            );
            Some((
                draw_mode::Placement {
                    page_index,
                    origin: (
                        area.x + picture.placed_pan().x as f32,
                        area.y + picture.placed_pan().y as f32,
                    ),
                    scale: (
                        shown.width as f32 / natural.width as f32,
                        shown.height as f32 / natural.height as f32,
                    ),
                    clip: frame_box.bounds,
                },
                drawing,
            ))
        }
    }

    /// The page (1-based) showing `frame`.
    pub(crate) fn draw_page(&self, frame: FrameId) -> Option<u32> {
        self.pages
            .layout()
            .pages
            .iter()
            .find(|page| page.frames.iter().any(|shown| shown.frame == frame))
            .map(|page| page.number)
    }

    /// What the page surface needs to show Draw mode at `scale` screen
    /// pixels a twip.
    pub(crate) fn draw_view(&self, scale: f32) -> Option<DrawView> {
        let mode = self.draw.as_ref()?;
        let (placement, drawing) = self.draw_placement()?;
        let pixel = 1.0 / f64::from((scale * placement.scale.0).max(1e-6));
        Some(DrawView {
            placement,
            overlay: mode.overlay(&drawing, pixel),
            tracking: mode.tracking(),
            tool: mode.tool,
            typing: mode.typing(),
        })
    }
}

/// A text object's box: its text's extent in its font.
fn measure(text: &sdw::Text) -> (f64, f64) {
    crate::metafile_render::text_extent(
        &text.face,
        f32::from(text.height.unsigned_abs()),
        text.height > 0,
        text.attributes & sdw::Text::BOLD != 0,
        text.attributes & sdw::Text::ITALIC != 0,
        &text.text,
    )
}

impl AmiProApp {
    /// Whether Draw mode is open in the active tab.
    pub(crate) fn drawing_open(&self) -> bool {
        self.page_surface && self.draw.is_some()
    }

    /// Runs `step` on Draw mode with the drawing's objects, then applies
    /// what it asks.
    fn with_draw(&mut self, step: impl FnOnce(&mut DrawMode, &draw_mode::Context<'_>) -> Outcome) {
        let session = &mut self.sessions[self.active_session];
        let Some(drawing) = session.draw_drawing() else {
            session.draw = None;
            return;
        };
        let scale = session
            .draw_placement()
            .map_or(1.0, |(placement, _)| placement.scale.0.max(1e-3));
        // Twips a screen pixel is in the drawing, at the tab's zoom.
        let view = session.page_view();
        let screen = if view.scale > 0.0 {
            view.scale
        } else {
            1.0 / 15.0
        };
        let pixel = 1.0 / f64::from(screen * scale);
        let context = draw_mode::Context {
            drawing: &drawing,
            pixel,
            measure: &measure,
        };
        let Some(mut mode) = session.draw.take() else {
            return;
        };
        let outcome = step(&mut mode, &context);
        session.draw = Some(mode);
        self.apply_draw_outcome(outcome);
    }

    fn apply_draw_outcome(&mut self, outcome: Outcome) {
        let Some(frame) = self.draw.as_ref().map(|mode| mode.frame) else {
            return;
        };
        if let Some(drawing) = outcome.drawing {
            let command = Command::SetDrawing {
                frame,
                drawing: Arc::new(drawing),
            };
            let _ = self.execute_frame_commands(&[command], outcome.amend);
            self.status = crate::draw_commands::DRAW_NOTE.into();
        }
        if let Some((dx, dy)) = outcome.pan
            && let Some(FrameContent::Picture(picture)) =
                self.document.frame(frame).map(|frame| &frame.content)
        {
            let pan = amipro_document::Offset {
                x: picture.pan.x + dx,
                y: picture.pan.y + dy,
            };
            let _ =
                self.execute_frame_commands(&[Command::MovePictureInFrame { frame, pan }], false);
        }
        if let Some(status) = outcome.status {
            self.status = status.into();
        }
        if outcome.leave {
            self.leave_draw_mode();
        }
    }

    /// Opens Draw mode on `frame`'s drawing (`UI_SPEC.md` §33).
    pub(crate) fn enter_draw_mode(&mut self, frame: FrameId) {
        let remembered = self.draw_memory.clone();
        let session = &mut self.sessions[self.active_session];
        let mut mode = DrawMode::new(frame);
        if let Some(memory) = remembered {
            mode.pen = memory.pen;
            mode.fill = memory.fill;
            mode.text = memory.text;
            mode.grid = memory.grid;
            mode.rotate_step = memory.rotate_step;
        }
        session.draw = Some(mode);
        session.draw_properties = crate::drawing_properties::DrawingProperties::default();
        session.frame_selection = None;
        session.picture_mode = None;
        session.frame_keys = None;
        session.status = DRAW_NOTE.into();
        // Drawing Properties in the rail (DW2).
        self.open_rail();
    }

    /// Leaves Draw mode: what is being drawn is kept, the frame selected,
    /// and the mode's look remembered for the next drawing.
    pub(crate) fn leave_draw_mode(&mut self) {
        if self.draw.is_none() {
            return;
        }
        let finished = {
            let session = &mut self.sessions[self.active_session];
            match (session.draw_drawing(), session.draw.as_mut()) {
                (Some(drawing), Some(mode)) => {
                    let context = draw_mode::Context {
                        drawing: &drawing,
                        pixel: 15.0,
                        measure: &measure,
                    };
                    mode.finish(&context)
                }
                _ => Outcome::default(),
            }
        };
        if finished.drawing.is_some() {
            self.apply_draw_outcome(Outcome {
                leave: false,
                ..finished
            });
        }
        let session = &mut self.sessions[self.active_session];
        let Some(mode) = session.draw.take() else {
            return;
        };
        let page = session
            .draw_page(mode.frame)
            .unwrap_or(session.page_selection.page);
        if session.document.frame(mode.frame).is_some() {
            session.frame_selection = Some(FrameSelection::one(mode.frame, page));
        }
        session.status = "Frame selected".into();
        self.draw_memory = Some(mode);
    }

    /// A page surface event in Draw mode.
    pub(crate) fn update_draw(&mut self, event: Event) {
        if self.draw.is_none() {
            return;
        }
        self.with_draw(|mode, context| mode.handle(event, context));
    }

    /// A tool chosen on the draw bar.
    pub(crate) fn choose_draw_tool(&mut self, tool: Tool) {
        if self.draw.is_none() {
            return;
        }
        self.with_draw(|mode, context| mode.choose(tool, context));
    }

    /// A change of look from the draw bar or Drawing Properties: to the
    /// selection, and to new objects.
    pub(crate) fn restyle_drawing(&mut self, change: StyleChange) {
        if self.draw.is_none() {
            return;
        }
        self.with_draw(|mode, context| mode.command(draw_mode::Command::Style(change), context));
    }

    /// A colour from Drawing Properties' buttons.
    pub(crate) fn draw_colour(
        &mut self,
        target: crate::colour_control::Target,
        colour: amipro_document::Colour,
    ) {
        use crate::colour_control::Target;
        let colour = crate::drawing_properties::from_document(colour);
        let mut change = StyleChange::default();
        match target {
            Target::DrawLine => change.colour = Some(colour),
            Target::DrawFill => change.foreground = Some(colour),
            Target::DrawBackground => change.background = Some(colour),
            Target::DrawText => change.text_colour = Some(colour),
            _ => return,
        }
        self.restyle_drawing(change);
    }

    /// A change in Drawing Properties.
    pub(crate) fn update_draw_properties(
        &mut self,
        action: crate::drawing_properties::PropertyAction,
    ) {
        use crate::drawing_properties::{self as properties, PropertyAction, Submitted};
        if self.draw.is_none() {
            return;
        }
        match &action {
            PropertyAction::Text(field, text) => {
                self.draw_properties.texts.insert(*field, text.clone());
            }
            PropertyAction::Submit(field) => {
                let Some(drawing) = self.draw_drawing() else {
                    return;
                };
                let submitted = self.draw.as_ref().and_then(|mode| {
                    let shown = properties::shown(mode, &drawing);
                    self.draw_properties.submitted(*field, &shown)
                });
                self.draw_properties.texts.remove(field);
                match submitted {
                    Some(Submitted::Place(rect)) => self.with_draw(|mode, context| {
                        mode.command(draw_mode::Command::Place(rect), context)
                    }),
                    Some(Submitted::Step(tenths)) => {
                        if let Some(mode) = self.draw.as_mut() {
                            mode.rotate_step = if mode.rotate_step < 0 {
                                -tenths
                            } else {
                                tenths
                            };
                        }
                    }
                    None => {}
                }
            }
            PropertyAction::KeepRatio(on) => self.draw_properties.keep_ratio = *on,
            PropertyAction::Clockwise(on) => {
                if let Some(mode) = self.draw.as_mut() {
                    mode.rotate_step = if *on {
                        mode.rotate_step.abs()
                    } else {
                        -mode.rotate_step.abs()
                    };
                }
            }
            PropertyAction::Align(how) => {
                let how = *how;
                self.with_draw(|mode, context| {
                    mode.command(draw_mode::Command::Align(how), context)
                });
            }
            PropertyAction::Distribute(across) => {
                let across = *across;
                self.with_draw(|mode, context| {
                    mode.command(draw_mode::Command::Distribute { across }, context)
                });
            }
            _ => {
                if let Some(mode) = self.draw.as_mut()
                    && properties::grid(&mut mode.grid, &action)
                {
                    return;
                }
                if let Some(change) = properties::change(&action) {
                    self.restyle_drawing(change);
                }
            }
        }
    }

    /// Drawing Properties for the rail, while Draw mode is open.
    pub(crate) fn drawing_properties_view(&self) -> Option<Element<'_, Message>> {
        let mode = self.draw.as_ref()?;
        let drawing = self.draw_drawing()?;
        Some(crate::drawing_properties::panel(
            &self.draw_properties,
            mode,
            &drawing,
            self.colour_context(),
        ))
    }

    /// The selected objects' count and what they allow, for enablement.
    fn draw_selection(&self) -> Option<(Vec<sdw::Object>, usize)> {
        let mode = self.draw.as_ref()?;
        let drawing = self.draw_drawing()?;
        let selected = mode
            .selection
            .iter()
            .filter_map(|&index| drawing.objects.get(index).cloned())
            .collect();
        Some((selected, drawing.objects.len()))
    }

    pub(crate) fn draw_command_enabled(&self, command: CommandId) -> bool {
        if !self.drawing_open() {
            return false;
        }
        let Some((selected, _)) = self.draw_selection() else {
            return false;
        };
        let count = selected.len();
        match command {
            CommandId::DrawExtract => {
                count == 1 && (selected[0].has_pen() || selected[0].has_fill())
            }
            CommandId::DrawApply
            | CommandId::DrawFront
            | CommandId::DrawBack
            | CommandId::DrawForward
            | CommandId::DrawBackward
            | CommandId::DrawRotateLeft
            | CommandId::DrawRotateRight
            | CommandId::DrawRotate
            | CommandId::DrawDuplicate => count >= 1,
            CommandId::DrawFlipHorizontal | CommandId::DrawFlipVertical => {
                selected.iter().any(sdw::Object::flippable)
            }
            CommandId::DrawGroup => count >= 2,
            CommandId::DrawUngroup => {
                count == 1 && matches!(selected[0].shape, sdw::Shape::Group { .. })
            }
            CommandId::DrawAlignLeft
            | CommandId::DrawAlignCentre
            | CommandId::DrawAlignRight
            | CommandId::DrawAlignTop
            | CommandId::DrawAlignMiddle
            | CommandId::DrawAlignBottom => count >= 2,
            CommandId::DrawDistributeAcross | CommandId::DrawDistributeDown => count >= 3,
            _ => true,
        }
    }

    /// The Draw menu's checks.
    pub(crate) fn draw_check(&self, command: CommandId) -> Option<bool> {
        let mode = self.draw.as_ref();
        match command {
            CommandId::DrawSnap => Some(mode.is_some_and(|mode| mode.grid.snap)),
            CommandId::DrawGrid => Some(mode.is_some_and(|mode| mode.grid.show)),
            CommandId::DrawSelectAll => Some(
                self.draw_selection()
                    .is_some_and(|(selected, all)| all > 0 && selected.len() == all),
            ),
            _ => None,
        }
    }

    pub(crate) fn run_draw_command(&mut self, command: CommandId) -> cosmic::app::Task<Message> {
        use draw_mode::Command as Draw;
        let step = |mode: &DrawMode| -> i32 { mode.rotate_step };
        let draw = match command {
            CommandId::DrawExtract => Draw::Extract,
            CommandId::DrawApply => Draw::Apply,
            CommandId::DrawRotateLeft => Draw::Rotate(-900),
            CommandId::DrawRotateRight => Draw::Rotate(900),
            CommandId::DrawRotate => Draw::Rotate(self.draw.as_ref().map_or(100, step)),
            CommandId::DrawFlipHorizontal => Draw::Flip { horizontal: true },
            CommandId::DrawFlipVertical => Draw::Flip { horizontal: false },
            CommandId::DrawGroup => Draw::Group,
            CommandId::DrawUngroup => Draw::Ungroup,
            CommandId::DrawFront => Draw::Order(Order::Front),
            CommandId::DrawBack => Draw::Order(Order::Back),
            CommandId::DrawForward => Draw::Order(Order::Forward),
            CommandId::DrawBackward => Draw::Order(Order::Backward),
            CommandId::DrawAlignLeft => Draw::Align(Align::Left),
            CommandId::DrawAlignCentre => Draw::Align(Align::Centre),
            CommandId::DrawAlignRight => Draw::Align(Align::Right),
            CommandId::DrawAlignTop => Draw::Align(Align::Top),
            CommandId::DrawAlignMiddle => Draw::Align(Align::Middle),
            CommandId::DrawAlignBottom => Draw::Align(Align::Bottom),
            CommandId::DrawDistributeAcross => Draw::Distribute { across: true },
            CommandId::DrawDistributeDown => Draw::Distribute { across: false },
            CommandId::DrawDuplicate => Draw::Duplicate,
            CommandId::DrawSelectAll => Draw::SelectAll,
            CommandId::DrawSnap => {
                if let Some(mode) = self.draw.as_mut() {
                    mode.grid.snap = !mode.grid.snap;
                }
                return cosmic::app::Task::none();
            }
            CommandId::DrawGrid => {
                if let Some(mode) = self.draw.as_mut() {
                    mode.grid.show = !mode.grid.show;
                }
                return cosmic::app::Task::none();
            }
            CommandId::DrawDone => {
                self.leave_draw_mode();
                return cosmic::app::Task::none();
            }
            CommandId::DrawProperties => {
                self.open_rail();
                return cosmic::app::Task::none();
            }
            CommandId::ImportDrawing => return self.import_drawing_task(),
            CommandId::SaveDrawingAs => return self.save_drawing_task(),
            _ => return cosmic::app::Task::none(),
        };
        self.with_draw(|mode, context| mode.command(draw, context));
        cosmic::app::Task::none()
    }

    /// Copy and Cut in Draw mode: the selected objects, as the app's own
    /// with SVG and PNG beside them (DW6); Cut then deletes them.
    pub(crate) fn copy_drawing(&mut self, cut: bool) -> cosmic::app::Task<Message> {
        let Some((selected, _)) = self.draw_selection() else {
            return cosmic::app::Task::none();
        };
        if selected.is_empty() {
            return cosmic::app::Task::none();
        }
        self.draw_clipboard = Some(selected.clone());
        let clip = crate::draw_files::DrawingClip::of(selected);
        if cut {
            self.with_draw(|mode, context| mode.command(draw_mode::Command::Delete, context));
        }
        self.status = if cut { "Objects cut" } else { "Objects copied" }.into();
        cosmic::iced::clipboard::write_data(clip)
    }

    /// Paste in Draw mode: the clipboard's drawing objects, SVG, picture,
    /// or text (a text object per line), else what was copied here; in
    /// front, selected. The app's own objects come a little down and
    /// right, so the copy shows.
    pub(crate) fn paste_drawing(&mut self, data: Option<crate::draw_files::DrawingPaste>) {
        use crate::draw_files::{self, DRAWING_TYPE};
        if self.draw.is_none() {
            return;
        }
        let mut objects = data.and_then(|data| {
            if data.mime.starts_with("text/plain") {
                let look = self.draw.as_ref()?.text.clone();
                let text = String::from_utf8_lossy(&data.bytes).into_owned();
                let objects = draw_files::text_objects(&text, &look, &measure);
                return (!objects.is_empty()).then_some((objects, false));
            }
            let own = data.mime == DRAWING_TYPE;
            draw_files::objects_of(&data.bytes, &data.mime).map(|objects| (objects, own))
        });
        if objects.is_none() {
            objects = self.draw_clipboard.clone().map(|objects| (objects, true));
        }
        let Some((mut objects, own)) = objects else {
            self.status = "Nothing on the clipboard a drawing takes".into();
            return;
        };
        if own {
            for object in &mut objects {
                object.transform(&sdw::Affine::translate(144.0, 144.0));
            }
        }
        self.with_draw(|mode, context| mode.command(draw_mode::Command::Add(objects), context));
    }

    /// Draw › Import Drawing…: a drawing, an SVG, or a picture file's
    /// objects added to the drawing (DW6).
    fn import_drawing_task(&self) -> cosmic::app::Task<Message> {
        use cosmic::dialog::file_chooser::{self, FileFilter};
        let session_id = self.id;
        let start = self.start_folder();
        cosmic::task::future(async move {
            let mut filter = FileFilter::new("Drawings and pictures");
            for extension in ["sdw", "svg", "png", "jpg", "jpeg", "bmp", "gif"] {
                filter = filter
                    .glob(&format!("*.{extension}"))
                    .glob(&format!("*.{}", extension.to_ascii_uppercase()));
            }
            let dialog = crate::file_dialogs::open_dialog()
                .title("Import Drawing".to_owned())
                .directory(start)
                .filter(filter);
            let result = match dialog.open_file().await {
                Ok(response) => response.url().to_file_path().map_or_else(
                    |()| crate::FileDialogResult::Failed("not a local file".into()),
                    crate::FileDialogResult::Selected,
                ),
                Err(file_chooser::Error::Cancelled) => crate::FileDialogResult::Cancelled,
                Err(error) => crate::FileDialogResult::Failed(error.to_string()),
            };
            Message::DrawImportChosen { session_id, result }
        })
    }

    pub(crate) fn import_drawing_file(&mut self, path: &std::path::Path) {
        let kind = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let objects = std::fs::read(path)
            .ok()
            .and_then(|bytes| crate::draw_files::objects_of(&bytes, &kind));
        match objects {
            Some(objects) => {
                self.with_draw(|mode, context| {
                    mode.command(draw_mode::Command::Add(objects), context)
                });
                self.status = "Drawing imported".into();
            }
            None => self.status = "Import Drawing could not read this file".into(),
        }
    }

    /// Draw › Save Drawing As…: the drawing, or its selected objects, as an
    /// Ami Pro drawing, SVG, or PNG (DW6).
    fn save_drawing_task(&self) -> cosmic::app::Task<Message> {
        use cosmic::dialog::file_chooser::{self, FileFilter};
        let session_id = self.id;
        let folder = self.start_folder();
        cosmic::task::future(async move {
            let mut dialog = file_chooser::save::Dialog::new()
                .title("Save Drawing As".to_owned())
                .file_name("Drawing.sdw".to_owned())
                .filter(FileFilter::new("Ami Pro drawing (.sdw)").glob("*.sdw"))
                .filter(FileFilter::new("SVG (.svg)").glob("*.svg"))
                .filter(FileFilter::new("PNG (.png)").glob("*.png"));
            if let Some(folder) = folder {
                dialog = dialog.directory(folder);
            }
            let result = match dialog.save_file().await {
                Ok(response) => response
                    .url()
                    .and_then(|url| url.to_file_path().ok())
                    .map_or_else(
                        || crate::FileDialogResult::Failed("not a local file".into()),
                        crate::FileDialogResult::Selected,
                    ),
                Err(file_chooser::Error::Cancelled) => crate::FileDialogResult::Cancelled,
                Err(error) => crate::FileDialogResult::Failed(error.to_string()),
            };
            Message::DrawSaveChosen { session_id, result }
        })
    }

    pub(crate) fn save_drawing_file(&mut self, path: &std::path::Path) {
        use crate::draw_files::{Format, file_bytes};
        let Some(drawing) = self.draw_drawing() else {
            return;
        };
        let selected = self
            .draw
            .as_ref()
            .map(|mode| mode.selection.clone())
            .unwrap_or_default();
        let drawing = if selected.is_empty() {
            drawing
        } else {
            Drawing {
                objects: selected
                    .iter()
                    .filter_map(|&index| drawing.objects.get(index).cloned())
                    .collect(),
                ..drawing
            }
        };
        let (path, format) = match Format::of(path) {
            Some(format) => (path.to_path_buf(), format),
            None => (path.with_extension("sdw"), Format::Sdw),
        };
        self.status = match file_bytes(&drawing, format).map(|bytes| std::fs::write(&path, bytes)) {
            Some(Ok(())) if selected.is_empty() => "Drawing saved".into(),
            Some(Ok(())) => "Selected objects saved".into(),
            Some(Err(error)) => format!("Save Drawing As failed: {error}"),
            None => "Save Drawing As could not draw the drawing".into(),
        };
    }

    /// Delete and Select All in Draw mode act on the drawing's objects.
    pub(crate) fn draw_edit_command(&mut self, command: CommandId) -> bool {
        if !self.drawing_open() {
            return false;
        }
        let draw = match command {
            CommandId::Delete => draw_mode::Command::Delete,
            CommandId::SelectAll => draw_mode::Command::SelectAll,
            _ => return false,
        };
        self.with_draw(|mode, context| mode.command(draw, context));
        true
    }

    /// Whether Insert › Drawing… can put a drawing where the caret or the
    /// frame selection is: where Insert › Image… can, or a drawing
    /// selected (which it opens).
    pub(crate) fn drawing_target_enabled(&self) -> bool {
        use crate::picture_insert::{Target, target};
        if !self.page_surface || self.drawing_open() {
            return false;
        }
        match target(
            &self.document,
            self.frame_selection.as_ref(),
            self.page_selection.focus,
        ) {
            Ok(Target::Replace(frame)) => self.frame_is_drawing(frame),
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub(crate) fn frame_is_drawing(&self, frame: FrameId) -> bool {
        matches!(
            self.document.frame(frame).map(|frame| &frame.content),
            Some(FrameContent::Picture(picture)) if picture.format() == PictureFormat::Drawing
        )
    }

    /// Insert › Drawing… (Ami Pro's Tools › Drawing, `1168:1a87`): a new
    /// drawing in the selected empty frame, the caret's empty cell, or a
    /// frame Ami Pro makes at the caret (`1260:0509`: the remembered
    /// Create Frame size within the column, Flow with text, the default
    /// frame), then Draw mode on it; a drawing selected opens.
    pub(crate) fn insert_drawing(&mut self) {
        use crate::picture_insert::{Target, target};
        use amipro_editing::{NewFrameContent, NewPicture, NewPictureSource, PictureTarget};
        let found = match target(
            &self.document,
            self.frame_selection.as_ref(),
            self.page_selection.focus,
        ) {
            Ok(found) => found,
            Err(problem) => {
                self.status = problem.replace("picture", "drawing");
                return;
            }
        };
        let page = self.page_selection.page;
        let picture = |size| NewPicture {
            source: NewPictureSource::Embedded(drawing::empty_drawing()),
            extension: ".sdw".into(),
            size,
        };
        let empty = Size {
            width: drawing::empty_drawing_size(),
            height: drawing::empty_drawing_size(),
        };
        let command = match found {
            Target::Replace(frame) => {
                if self.frame_is_drawing(frame) {
                    self.enter_draw_mode(frame);
                } else {
                    self.status = "A drawing goes into an empty frame".into();
                }
                return;
            }
            Target::Frame(frame) => Command::InsertPicture {
                target: PictureTarget::Frame(frame),
                picture: picture(empty),
                cell_style: self.preferences.frame_defaults.cell_frame_style(),
            },
            Target::Cell { table, cell } => Command::InsertPicture {
                target: PictureTarget::Cell { table, cell },
                picture: picture(empty),
                cell_style: self.preferences.frame_defaults.cell_frame_style(),
            },
            Target::NewFrame(at) => {
                let remembered = self
                    .preferences
                    .last_frame
                    .unwrap_or(amipro_editing::frame_creation::FrameGeometry::FIRST);
                let column = self.caret_column();
                let size = Size {
                    width: remembered.width.min(column.width).max(360),
                    height: remembered.height.min(column.height).max(360),
                };
                let style = self
                    .preferences
                    .frame_defaults
                    .frame_style(amipro_document::Rect::new(0, 0, size.width, size.height));
                Command::CreateFrame {
                    placement: amipro_document::Placement::Inline { y_offset: 0, size },
                    style,
                    content: NewFrameContent::EmbeddedPicture {
                        bytes: drawing::empty_drawing(),
                        extension: ".sdw".into(),
                    },
                    anchor: Some(at),
                    layout: None,
                }
            }
        };
        let before: std::collections::BTreeSet<_> =
            self.document.frames().map(|frame| frame.id).collect();
        if !self.execute_page_command(&command) {
            return;
        }
        let made = match found {
            Target::Frame(frame) => Some(frame),
            _ => self
                .document
                .frames()
                .map(|frame| frame.id)
                .find(|id| !before.contains(id)),
        };
        if let Some(frame) = made {
            self.sessions[self.active_session].page_selection.page = page;
            self.enter_draw_mode(frame);
        }
    }

    /// The draw bar (DW3): in the toolbar's place while Draw mode is open.
    pub(crate) fn draw_bar_view(&self) -> Element<'_, Message> {
        let Some(mode) = self.draw.as_ref() else {
            return widget::space().into();
        };
        let icon = |name: &'static str| {
            widget::icon::from_svg_bytes(
                crate::chrome::app_icon_svg(name).expect("the app's own draw icons"),
            )
            .symbolic(true)
        };
        let command = |id: CommandId, name: &'static str| -> Element<'_, Message> {
            let checked = self.draw_check(id) == Some(true);
            widget::button::icon(icon(name))
                .name(crate::command::metadata(id).label)
                .tooltip(crate::command::metadata(id).label)
                .selected(checked)
                .class(crate::toggle_button_class(checked))
                .width(32)
                .height(32)
                .on_press_maybe(
                    self.draw_command_enabled(id)
                        .then_some(Message::Command(id)),
                )
                .into()
        };
        let history = |id: CommandId, name: &'static str| -> Element<'_, Message> {
            widget::button::icon(widget::icon::from_name(name))
                .name(crate::command::metadata(id).label)
                .tooltip(crate::command::metadata(id).label)
                .width(32)
                .height(32)
                .on_press_maybe(self.command_enabled(id).then_some(Message::Command(id)))
                .into()
        };
        let divider = || -> Element<'_, Message> {
            widget::divider::vertical::default()
                .height(24)
                .width(crate::chrome::DIVIDER)
                .into()
        };
        let mut items: Vec<Element<'_, Message>> = vec![
            history(CommandId::Undo, "edit-undo-symbolic"),
            history(CommandId::Redo, "edit-redo-symbolic"),
            divider(),
        ];
        for tool in Tool::ALL {
            let on = mode.tool == tool;
            items.push(
                widget::button::icon(icon(tool_icon(tool)))
                    .name(tool.label())
                    .tooltip(tool.label())
                    .selected(on)
                    .class(crate::toggle_button_class(on))
                    .width(32)
                    .height(32)
                    .on_press(Message::DrawTool(tool))
                    .into(),
            );
        }
        items.push(divider());
        for (id, name) in [
            (
                CommandId::DrawSelectAll,
                crate::chrome::DRAW_ICON_SELECT_ALL,
            ),
            (CommandId::DrawGroup, crate::chrome::DRAW_ICON_GROUP),
            (CommandId::DrawFront, crate::chrome::DRAW_ICON_FRONT),
            (CommandId::DrawBack, crate::chrome::DRAW_ICON_BACK),
            (CommandId::DrawRotate, crate::chrome::DRAW_ICON_ROTATE),
            (
                CommandId::DrawFlipHorizontal,
                crate::chrome::DRAW_ICON_FLIP_HORIZONTAL,
            ),
            (
                CommandId::DrawFlipVertical,
                crate::chrome::DRAW_ICON_FLIP_VERTICAL,
            ),
        ] {
            items.push(command(id, name));
        }
        items.push(divider());
        items.push(command(CommandId::DrawGrid, crate::chrome::DRAW_ICON_GRID));
        items.push(command(CommandId::DrawSnap, crate::chrome::DRAW_ICON_SNAP));
        items.push(divider());
        items.push(command(
            CommandId::DrawExtract,
            crate::chrome::DRAW_ICON_EXTRACT,
        ));
        items.push(command(
            CommandId::DrawApply,
            crate::chrome::DRAW_ICON_APPLY,
        ));
        items.push(divider());
        // The text objects' font: face, size, bold, italic, underline.
        let faces = draw_faces(&mode.text.face);
        let chosen = faces.iter().position(|face| *face == mode.text.face);
        let face_list = faces.clone();
        items.push(
            widget::dropdown(faces, chosen, move |index| {
                Message::DrawStyle(StyleChange {
                    face: Some(face_list[index].clone()),
                    ..StyleChange::default()
                })
            })
            .width(150)
            .into(),
        );
        let sizes: Vec<String> = DRAW_SIZES.iter().map(|size| size.to_string()).collect();
        let points = i32::from(mode.text.height.unsigned_abs()) / 20;
        items.push(
            widget::dropdown(
                sizes,
                DRAW_SIZES.iter().position(|&size| size == points),
                |index| {
                    Message::DrawStyle(StyleChange {
                        height: Some(-(DRAW_SIZES[index] * 20) as i16),
                        ..StyleChange::default()
                    })
                },
            )
            .width(64)
            .into(),
        );
        for (bit, name, label) in [
            (sdw::Text::BOLD, "format-text-bold-symbolic", "Bold"),
            (sdw::Text::ITALIC, "format-text-italic-symbolic", "Italic"),
            (
                sdw::Text::UNDERLINE,
                "format-text-underline-symbolic",
                "Underline",
            ),
        ] {
            let on = mode.text.attributes & bit != 0;
            let mut change = StyleChange::default();
            match bit {
                sdw::Text::BOLD => change.bold = Some(!on),
                sdw::Text::ITALIC => change.italic = Some(!on),
                _ => change.underline = Some(!on),
            }
            items.push(
                widget::button::icon(widget::icon::from_name(name))
                    .name(label)
                    .tooltip(label)
                    .selected(on)
                    .class(crate::toggle_button_class(on))
                    .width(32)
                    .height(32)
                    .on_press(Message::DrawStyle(change))
                    .into(),
            );
        }
        items.push(widget::space().width(Length::Fill).into());
        items.push(
            widget::button::suggested("Done")
                .on_press(Message::Command(CommandId::DrawDone))
                .into(),
        );
        widget::container(
            widget::scrollable(
                widget::row::with_children(items)
                    .spacing(crate::chrome::ICON_GAP)
                    .align_y(cosmic::iced::Alignment::Center),
            )
            .direction(cosmic::iced::widget::scrollable::Direction::Horizontal(
                cosmic::iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .scroller_width(4),
            ))
            .width(Length::Fill),
        )
        .padding([4.0, crate::chrome::BAR_PADDING])
        .height(crate::ACTION_BAR_HEIGHT)
        .width(Length::Fill)
        .into()
    }
}

/// Point sizes the draw bar offers for text objects.
const DRAW_SIZES: [i32; 16] = [6, 8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72];

/// The faces the draw bar offers: Ami Pro's usual ones and the current.
pub(crate) fn draw_faces(current: &str) -> Vec<String> {
    let mut faces: Vec<String> = [
        "Arial",
        "Courier New",
        "Helvetica",
        "Times New Roman",
        "Tms Rmn",
        "Helv",
        "Book Antiqua",
        "AvantGarde",
    ]
    .iter()
    .map(|face| (*face).to_owned())
    .collect();
    if !faces.iter().any(|face| face == current) {
        faces.push(current.to_owned());
    }
    faces
}

fn tool_icon(tool: Tool) -> &'static str {
    use crate::chrome as c;
    match tool {
        Tool::Select => c::DRAW_ICON_SELECT,
        Tool::Hand => c::DRAW_ICON_HAND,
        Tool::Line => c::DRAW_ICON_LINE,
        Tool::Polyline => c::DRAW_ICON_POLYLINE,
        Tool::Polygon => c::DRAW_ICON_POLYGON,
        Tool::Rectangle => c::DRAW_ICON_RECTANGLE,
        Tool::RoundedRectangle => c::DRAW_ICON_ROUNDED,
        Tool::Ellipse => c::DRAW_ICON_ELLIPSE,
        Tool::Arc => c::DRAW_ICON_ARC,
        Tool::Text => c::DRAW_ICON_TEXT,
    }
}
