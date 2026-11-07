//! Tools › Charting… and Chart from Table on the page surface
//! (`UI_SPEC.md` §42.4, run D, DO.8): which frame a chart goes in, the data
//! it starts with, and what each control of the Chart panel does. Every
//! change generates the chart's drawing again (`chart_gen`) and sets it on
//! the frame as one undo step (`Command::SetDrawing`; the frame keeps its
//! `.sct` driver); a slider drag amends one step. Nothing is modal: a
//! question or a refusal shows in the panel.

use std::sync::Arc;

use amipro_document::{
    FrameContent, FrameId, ObjectId, PictureFormat, PictureSource, Placement, Rect, Size,
};
use amipro_editing::tables::{CellBlock, CellRange};
use amipro_editing::{Command, NewFrameContent, NewPicture, NewPictureSource, PictureTarget};
use amipro_pictures::chart::{Chart, DataFormat, chart_of, parse_data};
use amipro_pictures::chart_gen::{Metrics, generate_drawing_with_axis};
use amipro_pictures::sdw::{self, Drawing};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::Task;
use cosmic::iced::widget::text_editor;

use crate::chart_panel::{self as cp, ChartAction, ChartPanel, Kind, MIN_FRAME, Notice};
use crate::picture_insert::{self, Target};
use crate::{AmiProApp, FileDialogResult, Message, table_editing};
use amipro_editing::TextPosition;

/// A chart frame Ami Pro makes is 3.2 by 2.7 inches (CHV2).
const DEFAULT_FRAME: Size = Size {
    width: 4_608,
    height: 3_888,
};

/// Where Tools › Charting… puts the chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entry {
    /// A chart frame: its panel opens.
    Chart(FrameId),
    /// A frame holding a drawing or a picture: replaced once the user agrees.
    Replace(FrameId),
    /// A selected empty text frame.
    Frame(FrameId),
    /// The caret's empty table cell.
    Cell { table: FrameId, cell: usize },
    /// A new Flow with text frame at the caret.
    New(TextPosition),
    /// A new With para above frame after a table.
    AfterTable(FrameId),
}

/// The metrics a chart in a frame of `size` is generated for (message 4012
/// refuses a frame no larger than 721 twips each way).
fn metrics_for(size: Size, style: &amipro_document::FrameStyle) -> Metrics {
    // The drawing rectangle is the frame's picture area: the frame less its
    // line and the picture inset (CHV1, notes 14.4).
    let insets = style.text_insets(size.width, size.height);
    Metrics::for_picture_area(
        size.width - insets.left - insets.right,
        size.height - insets.top - insets.bottom,
    )
}

fn drawing_size(drawing: &Drawing) -> Size {
    let natural = drawing.natural_size();
    Size {
        width: natural.width,
        height: natural.height,
    }
}

/// The chart's lines of data from a table's cells as Ami Pro's data text:
/// an empty cell among numbers is a zero.
fn table_text(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            let numeric = cells
                .iter()
                .any(|cell| !cell.trim().is_empty() && cp::parse_value(cell).is_ok());
            cells
                .iter()
                .enumerate()
                .map(|(index, cell)| {
                    if numeric && index > 0 && cell.trim().is_empty() {
                        "0"
                    } else {
                        cell.trim()
                    }
                })
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl AmiProApp {
    // ---------------------------------------------------------------- entry

    /// What a frame holds, for the panel: a chart, an unreadable chart, or
    /// something Charting would replace.
    fn chart_kind(&self, frame: FrameId) -> Option<Kind> {
        let FrameContent::Picture(picture) = &self.document.frame(frame)?.content else {
            return None;
        };
        Some(match picture.format() {
            PictureFormat::Chart => {
                if self
                    .read_chart(frame)
                    .is_some_and(|(_, chart)| chart.is_some())
                {
                    Kind::Chart
                } else {
                    Kind::Unreadable
                }
            }
            PictureFormat::Drawing => {
                let empty = amipro_editing::drawing::drawing_of(&self.document, frame)
                    .is_some_and(|drawing| drawing.objects.is_empty());
                if empty {
                    Kind::Replace("an empty drawing")
                } else {
                    Kind::Replace("a drawing")
                }
            }
            PictureFormat::Ole => Kind::Replace("an OLE object"),
            _ => Kind::Replace("a picture"),
        })
    }

    /// The drawing object of a chart frame and the chart its record reads
    /// as (`None` when the record is not in the known layout).
    pub(crate) fn read_chart(&self, frame: FrameId) -> Option<(ObjectId, Option<Chart>)> {
        let FrameContent::Picture(picture) = &self.document.frame(frame)?.content else {
            return None;
        };
        let PictureSource::Embedded(object) = &picture.source else {
            return None;
        };
        let chart = amipro_editing::drawing::drawing_of(&self.document, frame)
            .and_then(|drawing| chart_of(&drawing))
            .map(|(_, chart)| chart);
        Some((*object, chart))
    }

    /// Where Tools › Charting… would put a chart now.
    pub(crate) fn chart_entry(&self) -> Result<Entry, String> {
        match picture_insert::target(
            &self.document,
            self.frame_selection.as_ref(),
            self.page_selection.focus,
        ) {
            Ok(Target::Replace(frame)) => Ok(match self.chart_kind(frame) {
                Some(Kind::Chart | Kind::Unreadable) => Entry::Chart(frame),
                _ => Entry::Replace(frame),
            }),
            Ok(Target::Frame(frame)) => Ok(Entry::Frame(frame)),
            Ok(Target::Cell { table, cell }) => Ok(Entry::Cell { table, cell }),
            Ok(Target::NewFrame(at)) => Ok(Entry::New(at)),
            Err(problem) => match table_editing::caret_cell(&self.document, self.page_selection) {
                Some((table, _, _)) if self.frame_selection.is_none() => {
                    Ok(Entry::AfterTable(table))
                }
                _ => Err(format!(
                    "Charting needs an empty frame, a chart, an empty table cell, or the main text ({})",
                    problem.to_lowercase()
                )),
            },
        }
    }

    /// Tools › Charting…: a chart opens its panel; a drawing asks first;
    /// anywhere else a new chart starts, from the clipboard's tabbed text
    /// when it holds some, else from sample data.
    pub(crate) fn open_charting(&mut self) -> Task<cosmic::Action<Message>> {
        // Selected table cells are the data (Chart from Table).
        if let Some(at) = self.table_at()
            && at.selected
            && at.raw.rows.len() * at.raw.columns.len() > 1
        {
            return self.chart_from_table();
        }
        match self.chart_entry() {
            Err(problem) => {
                self.status = problem;
                Task::none()
            }
            Ok(Entry::Chart(frame)) => {
                self.open_chart_panel(frame, None);
                Task::none()
            }
            Ok(Entry::Replace(frame)) => {
                let kind = self.chart_kind(frame).unwrap_or(Kind::Replace("a picture"));
                self.show_chart_panel(ChartPanel::new(Some(frame), kind));
                Task::none()
            }
            Ok(_) => {
                let session_id = self.id;
                self.show_chart_panel(ChartPanel::new(None, Kind::Starting));
                cosmic::iced::clipboard::read().map(move |text| {
                    cosmic::Action::from(Message::ChartClipboard { session_id, text })
                })
            }
        }
    }

    /// The clipboard's answer: a new chart from its tabbed text, or from the
    /// sample.
    pub(crate) fn charting_clipboard(&mut self, session_id: u64, text: Option<String>) {
        let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.id == session_id)
        else {
            return;
        };
        self.active_session = index;
        if self.chart_replace.is_some() {
            self.replace_with_chart(text);
            return;
        }
        let starting = self
            .chart_panel
            .as_ref()
            .is_some_and(|panel| panel.kind == Kind::Starting);
        let entry = match self.chart_entry() {
            Ok(entry) if !matches!(entry, Entry::Chart(_) | Entry::Replace(_)) => entry,
            other => {
                if starting {
                    self.close_chart();
                }
                if let Err(problem) = other {
                    self.status = problem;
                }
                return;
            }
        };
        let data = text.as_deref().and_then(cp::clipboard_data);
        let sample = data.is_none();
        let chart = cp::new_chart(data);
        let notice = sample.then(|| Notice::Info(cp::SAMPLE_NOTE.into()));
        if let Some(frame) = self.make_chart(entry, &chart) {
            self.open_chart_panel(frame, notice);
        }
    }

    /// Tools › Chart from Table: the selected cells (the whole table when
    /// only the caret is in it) become the data of a new chart in a frame
    /// after the table, copied in (OD4: no live link).
    pub(crate) fn chart_from_table(&mut self) -> Task<cosmic::Action<Message>> {
        let Some(at) = self.table_at() else {
            self.status = "Put the caret in a table, or select its cells, to chart them".into();
            return Task::none();
        };
        let range = if at.selected {
            at.raw.clone()
        } else {
            CellRange {
                rows: 0..at.table.rows.len(),
                columns: 0..at.table.columns.len(),
            }
        };
        let text = CellBlock::copy(&self.document, &at.table, &range).text();
        let data = match parse_data(&table_text(&text), &DataFormat::default()) {
            Ok(data) => data,
            Err(error) => {
                self.status = format!(
                    "Chart from Table: {}",
                    cp::data_problem(error)
                        .replace("enter at least one number", "the cells hold none")
                );
                return Task::none();
            }
        };
        let chart = cp::new_chart(Some(data));
        if let Some(frame) = self.make_chart(Entry::AfterTable(at.frame), &chart) {
            self.open_chart_panel(
                frame,
                Some(Notice::Info(
                    "A chart of the table's cells; the data is a copy.".into(),
                )),
            );
        }
        Task::none()
    }

    /// The paragraph and byte just after the anchor of a table's frame in
    /// the main text, or the end of the main text.
    pub(crate) fn after_anchor(&self, frame: FrameId) -> TextPosition {
        let main = self.document.main_flow_id();
        for (index, paragraph) in self.document.paragraphs().iter().enumerate() {
            let found = paragraph
                .object_offsets()
                .zip(&paragraph.objects)
                .find(|(_, object)| {
                    matches!(object, amipro_document::InlineObject::FrameAnchor(id) if *id == frame)
                });
            if let Some((byte, _)) = found {
                return TextPosition {
                    flow: main,
                    paragraph: index,
                    byte: byte + '\u{FFFC}'.len_utf8(),
                };
            }
        }
        let paragraphs = self.document.paragraphs();
        let last = paragraphs.len().saturating_sub(1);
        TextPosition {
            flow: main,
            paragraph: last,
            byte: paragraphs
                .get(last)
                .map_or(0, |paragraph| paragraph.text.len()),
        }
    }

    /// Puts a new chart where `entry` says, as one undo step; the frame.
    fn make_chart(&mut self, entry: Entry, chart: &Chart) -> Option<FrameId> {
        let column = self.caret_column();
        let fit = |size: Size| Size {
            width: size.width.min(column.width.max(MIN_FRAME + 1)),
            height: size.height.min(column.height.max(MIN_FRAME + 1)),
        };
        let frame_size = |app: &Self, frame: FrameId| {
            app.document
                .frame(frame)
                .and_then(|frame| frame.placement.size())
                .unwrap_or(DEFAULT_FRAME)
        };
        let size = match entry {
            Entry::Chart(frame) | Entry::Replace(frame) | Entry::Frame(frame) => {
                frame_size(self, frame)
            }
            Entry::Cell { .. } => DEFAULT_FRAME,
            Entry::New(_) | Entry::AfterTable(_) => fit(DEFAULT_FRAME),
        };
        if size.width <= MIN_FRAME || size.height <= MIN_FRAME {
            self.status = "The frame is too small to hold a chart".into();
            return None;
        }
        let cell_style = self.preferences.frame_defaults.cell_frame_style();
        let style =
            self.preferences
                .frame_defaults
                .frame_style(Rect::new(0, 0, size.width, size.height));
        // A frame that exists is generated for its own line and insets.
        let metrics_style = match entry {
            Entry::Chart(frame) | Entry::Replace(frame) | Entry::Frame(frame) => self
                .document
                .frame(frame)
                .map_or_else(|| style.clone(), |frame| frame.style.clone()),
            _ => style.clone(),
        };
        let drawing = generate_drawing_with_axis(chart, &metrics_for(size, &metrics_style), None);
        let bytes: Arc<[u8]> = sdw::write(&drawing).into();
        let natural = drawing_size(&drawing);
        let picture = || NewPicture {
            source: NewPictureSource::Embedded(bytes.clone()),
            extension: ".sct".into(),
            size: natural,
        };
        let before: std::collections::BTreeSet<_> =
            self.document.frames().map(|frame| frame.id).collect();
        let command = match entry {
            Entry::Chart(frame) | Entry::Replace(frame) => Command::ReplacePicture {
                frame,
                picture: picture(),
            },
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
                    extension: ".sct".into(),
                },
                anchor: Some(at),
                layout: None,
            },
            Entry::AfterTable(table) => Command::CreateFrame {
                placement: Placement::BelowParagraph { x_offset: 0, size },
                // With para above frames do not wrap text around.
                style: amipro_document::FrameStyle {
                    wrap: amipro_document::Wrap::NoWrapAround,
                    ..style
                },
                content: NewFrameContent::EmbeddedPicture {
                    bytes: bytes.clone(),
                    extension: ".sct".into(),
                },
                anchor: Some(self.after_anchor(table)),
                layout: None,
            },
        };
        if !self.execute_page_command(&command) {
            return None;
        }
        let made = match entry {
            Entry::Chart(frame) | Entry::Replace(frame) | Entry::Frame(frame) => Some(frame),
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
        });
        self.status = "Chart made".into();
        let made = made?;
        self.chart_drawing.remove(&made);
        Some(made)
    }

    // ---------------------------------------------------------------- panel

    /// Shows `panel` in the rail; every other panel gives way.
    fn show_chart_panel(&mut self, panel: ChartPanel) {
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.chart_panel = Some(panel);
        self.open_rail();
    }

    /// Opens the panel on a chart frame (a double-click, Tools › Charting…).
    pub(crate) fn open_chart_panel(&mut self, frame: FrameId, notice: Option<Notice>) {
        let kind = self.chart_kind(frame).unwrap_or(Kind::Unreadable);
        let mut panel = ChartPanel::new(Some(frame), kind);
        panel.notice = notice;
        if self.chart_drawing.contains(&frame) && panel.notice.is_none() {
            panel.notice = Some(Notice::Info(
                "This chart was edited with Drawing. A change here generates it again and replaces those edits.".into(),
            ));
        }
        if kind == Kind::Chart {
            self.load_chart_panel(&mut panel);
        }
        self.show_chart_panel(panel);
    }

    /// Reads the chart from the document into `panel`.
    fn load_chart_panel(&self, panel: &mut ChartPanel) {
        let Some((object, chart)) = panel.frame.and_then(|frame| self.read_chart(frame)) else {
            panel.chart = None;
            return;
        };
        let automatic = chart
            .as_ref()
            .zip(panel.frame)
            .is_some_and(|(chart, frame)| self.chart_is_automatic(frame, chart));
        let kind = if chart.is_some() {
            Kind::Chart
        } else {
            Kind::Unreadable
        };
        panel.kind = kind;
        panel.load(Some(object), chart, automatic);
    }

    /// The frame's size (a cell's frame: the default) and style.
    fn chart_frame_geometry(&self, frame: FrameId) -> (Size, amipro_document::FrameStyle) {
        match self.document.frame(frame) {
            Some(frame) => (
                frame.placement.size().unwrap_or(DEFAULT_FRAME),
                frame.style.clone(),
            ),
            None => (DEFAULT_FRAME, amipro_document::FrameStyle::default()),
        }
    }

    /// Whether the chart's stored axis is the one the generator computes.
    fn chart_is_automatic(&self, frame: FrameId, chart: &Chart) -> bool {
        let (size, style) = self.chart_frame_geometry(frame);
        let generated = generate_drawing_with_axis(chart, &metrics_for(size, &style), None);
        chart_of(&generated).is_some_and(|(_, made)| made.axis == chart.axis)
            || !cp::has_axis(cp::icon_of(chart))
    }

    /// After every message: the panel follows the document (an Undo, a
    /// Redo) and closes when its frame is gone.
    pub(crate) fn sync_chart_panel(&mut self) {
        let Some(frame) = self.chart_panel.as_ref().and_then(|panel| panel.frame) else {
            return;
        };
        if self.document.frame(frame).is_none() {
            self.chart_panel = None;
            return;
        }
        let Some(panel) = self.chart_panel.as_ref() else {
            return;
        };
        if matches!(panel.kind, Kind::Replace(_) | Kind::Starting) {
            return;
        }
        let Some((object, _)) = self.read_chart(frame) else {
            // The frame no longer holds a picture.
            self.chart_panel = None;
            return;
        };
        if panel.object != Some(object) {
            let mut panel = self.chart_panel.take().expect("checked");
            self.load_chart_panel(&mut panel);
            self.chart_panel = Some(panel);
        }
    }

    pub(crate) fn close_chart(&mut self) {
        if self.chart_panel.take().is_some() && self.rail_open() {
            self.toggle_rail();
        }
    }

    // -------------------------------------------------------------- changes

    /// Applies the panel's pending text (the Data text, grid cells, the
    /// axis and depth boxes), then `edit`, then generates the chart again as
    /// one undo step. When the user is applying that text itself (`explicit`:
    /// Enter, leaving the box, Apply), a refused text stays in its box with
    /// its reason and nothing changes; when the user turned to something
    /// else, the refused text is dropped with its reason in the notice and
    /// the change goes on.
    fn chart_edit(&mut self, edit: impl FnOnce(&mut Chart)) -> bool {
        self.chart_edit_as(false, edit)
    }

    fn chart_edit_as(&mut self, explicit: bool, edit: impl FnOnce(&mut Chart)) -> bool {
        let Some(panel) = self.chart_panel.as_ref() else {
            return false;
        };
        let Some(original) = panel.chart.clone() else {
            return false;
        };
        let Some(frame) = panel.frame else {
            return false;
        };
        let mut chart = original.clone();
        let mut dropped: Option<String> = None;
        // The Data text first: it can change the shape the cells address.
        if panel.data_edited {
            match parse_data(&panel.data_text(), &DataFormat::default()) {
                Ok(data) => chart.set_data(data),
                Err(error) => {
                    let text = cp::data_problem(error);
                    if explicit {
                        if let Some(panel) = self.chart_panel.as_mut() {
                            panel.data_error = Some(text);
                        }
                        return false;
                    }
                    dropped = Some(text.into());
                }
            }
        }
        if !panel.cells.is_empty() {
            match cp::with_cells(&chart, &panel.cells) {
                Ok(edited) => chart = edited,
                Err(reason) => {
                    if explicit {
                        if let Some(panel) = self.chart_panel.as_mut() {
                            panel.cell_error = Some(reason);
                        }
                        return false;
                    }
                    dropped = Some(reason);
                }
            }
        }
        if let Some(text) = panel.depth.clone() {
            match text
                .trim()
                .parse::<u16>()
                .ok()
                .filter(|depth| (1..=100).contains(depth))
            {
                Some(depth) => set_depth(&mut chart, depth),
                None => {
                    let reason = "Depth must be from 1 to 100.";
                    if explicit {
                        if let Some(panel) = self.chart_panel.as_mut() {
                            panel.notice = Some(Notice::Refused(reason.into()));
                        }
                        return false;
                    }
                    dropped = Some(reason.into());
                }
            }
        }
        let mut typed_axis = false;
        if panel.axis.edited {
            match panel.axis.parse() {
                Ok(axis) => {
                    chart.axis = axis;
                    typed_axis = true;
                }
                Err(reason) => {
                    if explicit {
                        if let Some(panel) = self.chart_panel.as_mut() {
                            panel.axis.error = Some(reason);
                        }
                        return false;
                    }
                    dropped = Some(reason);
                }
            }
        }
        edit(&mut chart);
        let automatic = self
            .chart_panel
            .as_ref()
            .is_some_and(|panel| panel.axis.automatic)
            && !typed_axis;
        let changed = chart != original;
        let committed = changed && self.commit_chart(frame, chart, automatic);
        if !committed && let Some(panel) = self.chart_panel.as_mut() {
            // Nothing new: what was typed is the chart's own, or refused.
            panel.cells.clear();
            panel.cell_error = None;
            panel.depth = None;
            panel.data_edited = false;
            panel.data_error = None;
            panel.axis.edited = false;
            panel.axis.error = None;
            if !changed {
                panel.notice = None;
            }
            panel.data =
                text_editor::Content::with_text(&original.data_text(&DataFormat::default()));
        }
        if let (Some(reason), Some(panel)) = (dropped, self.chart_panel.as_mut()) {
            panel.notice = Some(Notice::Refused(format!("{reason} It was not applied.")));
        }
        committed
    }

    /// Generates `chart` for the frame (with its stored axis unless
    /// `automatic`) and sets the drawing: one undo step, or part of the
    /// slider drag in progress.
    fn commit_chart(&mut self, frame: FrameId, chart: Chart, automatic: bool) -> bool {
        let (size, style) = self.chart_frame_geometry(frame);
        if size.width <= MIN_FRAME || size.height <= MIN_FRAME {
            if let Some(panel) = self.chart_panel.as_mut() {
                panel.notice = Some(Notice::Refused(
                    "The frame is too small to hold a chart; make it larger first.".into(),
                ));
            }
            return false;
        }
        let axis = (!automatic && cp::has_axis(cp::icon_of(&chart))).then_some(chart.axis);
        let drawing = generate_drawing_with_axis(&chart, &metrics_for(size, &style), axis);
        let command = Command::SetDrawing {
            frame,
            drawing: Arc::new(drawing),
        };
        let session = &mut self.sessions[self.active_session];
        let before = session.history.undo_len();
        if let Err(reason) = session.execute_frame_commands(&[command], false) {
            if let Some(panel) = session.chart_panel.as_mut() {
                panel.notice = Some(Notice::Refused(format!(
                    "The chart was not changed: {reason}"
                )));
            }
            return false;
        }
        session.status = "Chart changed".into();
        session.chart_drawing.remove(&frame);
        let Some(mut panel) = self.chart_panel.take() else {
            return true;
        };
        panel.notice = None;
        self.load_chart_panel(&mut panel);
        // The axis mode just used is the one the boxes show (an Automatic
        // that was only unticked stays unticked).
        panel.axis.automatic = automatic || !cp::has_axis(cp::icon_of(&chart));
        self.chart_panel = Some(panel);
        let _ = before;
        true
    }

    /// A colour chosen for the selected colour-set entry.
    pub(crate) fn chart_colour(&mut self, colour: amipro_document::Colour) {
        let Some(index) = self.chart_panel.as_ref().map(|panel| panel.colour) else {
            return;
        };
        self.chart_edit(|chart| {
            let pattern = chart.palette[index][3];
            chart.palette[index] = [
                (colour & 0xFF) as u8,
                ((colour >> 8) & 0xFF) as u8,
                ((colour >> 16) & 0xFF) as u8,
                pattern,
            ];
        });
    }

    /// One action of the panel.
    pub(crate) fn update_chart(&mut self, action: ChartAction) -> Task<cosmic::Action<Message>> {
        if self.chart_panel.is_none() {
            return Task::none();
        }
        // A box that is only being typed in keeps its text; anything else
        // the user does applies it first (as part of the same step).
        match action {
            ChartAction::Close => {
                self.close_chart();
            }
            ChartAction::Cell { row, column, text } => {
                if let Some(panel) = self.chart_panel.as_mut() {
                    panel.cells.insert((row, column), text);
                    panel.cell_error = None;
                }
            }
            ChartAction::CommitCells | ChartAction::AxisSubmit | ChartAction::DepthSubmit => {
                self.chart_edit_as(true, |_| {});
            }
            ChartAction::AxisText(field, text) => {
                if let Some(panel) = self.chart_panel.as_mut() {
                    *panel.axis.field_mut(field) = text;
                    panel.axis.edited = true;
                    panel.axis.error = None;
                }
            }
            ChartAction::DepthText(text) => {
                if let Some(panel) = self.chart_panel.as_mut() {
                    panel.depth = Some(text);
                }
            }
            ChartAction::DataEdit(edit) => {
                if let Some(panel) = self.chart_panel.as_mut() {
                    let changes = edit.is_edit();
                    panel.data.perform(edit);
                    if changes {
                        panel.data_edited = true;
                        panel.data_error = parse_data(&panel.data_text(), &DataFormat::default())
                            .err()
                            .map(cp::data_problem);
                    }
                }
            }
            ChartAction::ApplyData => {
                self.chart_edit_as(true, |_| {});
            }
            ChartAction::SelectColour(index) => {
                if let Some(panel) = self.chart_panel.as_mut() {
                    panel.colour = index.min(15);
                }
            }
            ChartAction::Icon(icon) => {
                self.chart_edit(|chart| {
                    chart.set_icon(icon);
                    if !cp::supports_3d(icon) {
                        chart.oblique_depth = 0;
                        chart.perspective_depth = 0;
                    }
                });
            }
            ChartAction::Legend(on) => {
                self.chart_edit(|chart| chart.legend = u16::from(on));
            }
            ChartAction::Grid(on) => {
                self.chart_edit(|chart| chart.grid = u16::from(on));
            }
            ChartAction::ThreeD(on) => {
                self.chart_edit(|chart| {
                    let pie = cp::is_pie(cp::icon_of(chart));
                    let depth = match chart.depth() {
                        0 => 50,
                        depth => depth,
                    };
                    chart.oblique_depth = 0;
                    chart.perspective_depth = 0;
                    if on {
                        // A 3D pie is stored with perspective depth, as
                        // Ami Pro's own are.
                        if pie {
                            chart.perspective_depth = depth;
                        } else {
                            chart.oblique_depth = depth;
                        }
                    }
                });
            }
            ChartAction::Perspective(on) => {
                self.chart_edit(|chart| {
                    let depth = chart.depth().max(1);
                    chart.oblique_depth = 0;
                    chart.perspective_depth = 0;
                    if on {
                        chart.perspective_depth = depth;
                    } else {
                        chart.oblique_depth = depth;
                    }
                });
            }
            ChartAction::Depth(depth) => {
                // The first move starts a step; the rest amend it until the
                // release (the same rule as the image sliders).
                let before = self.history.undo_len();
                self.chart_edit(|chart| set_depth(chart, depth));
                if self.colour_drag.is_none() && self.history.undo_len() > before {
                    self.colour_drag = Some(self.history.undo_len());
                }
            }
            ChartAction::DepthEnd => self.colour_drag = None,
            ChartAction::AxisAutomatic(on) => {
                if on {
                    if let Some(panel) = self.chart_panel.as_mut() {
                        panel.axis.edited = false;
                        panel.axis.error = None;
                        panel.axis.automatic = true;
                    }
                    // Generated again with the computed axis; a change
                    // only when the stored one differs.
                    let frame = self.chart_panel.as_ref().and_then(|panel| panel.frame);
                    let chart = self
                        .chart_panel
                        .as_ref()
                        .and_then(|panel| panel.chart.clone());
                    if let (Some(frame), Some(chart)) = (frame, chart) {
                        self.commit_chart(frame, chart, true);
                    }
                } else if let Some(panel) = self.chart_panel.as_mut() {
                    panel.axis.automatic = false;
                }
            }
            ChartAction::Flip => {
                self.chart_edit(Chart::flip);
            }
            ChartAction::AddSeries => {
                let mut added = false;
                self.chart_edit(|chart| added = cp::add_series(chart));
                if !added {
                    self.too_much();
                }
            }
            ChartAction::AddCategory => {
                let mut added = false;
                self.chart_edit(|chart| added = cp::add_category(chart));
                if !added {
                    self.too_much();
                }
            }
            ChartAction::RemoveSeries => {
                self.chart_edit(|chart| {
                    cp::remove_series(chart);
                });
            }
            ChartAction::RemoveCategory => {
                self.chart_edit(|chart| {
                    cp::remove_category(chart);
                });
            }
            ChartAction::CopyCsv => {
                if let Some(chart) = self
                    .chart_panel
                    .as_ref()
                    .and_then(|panel| panel.chart.as_ref())
                {
                    let text = cp::csv(chart);
                    self.status = "Chart data copied as comma-separated values".into();
                    return cosmic::iced::clipboard::write(text);
                }
            }
            ChartAction::SaveImage => return self.save_chart_image_task(),
            ChartAction::FromTable => return self.chart_from_table(),
            ChartAction::Drawing => {
                self.chart_edit(|_| {});
                if let Some(panel) = self.chart_panel.take() {
                    match panel.frame {
                        Some(frame) if matches!(panel.kind, Kind::Chart | Kind::Unreadable) => {
                            self.chart_drawing.insert(frame);
                            self.enter_draw_mode(frame);
                        }
                        _ => self.chart_panel = Some(panel),
                    }
                }
            }
            ChartAction::ConfirmReplace => {
                let Some(frame) = self.chart_panel.as_ref().and_then(|panel| panel.frame) else {
                    return Task::none();
                };
                // An unreadable chart or a drawing is replaced by a chart of
                // the clipboard's data or the sample.
                let session_id = self.id;
                self.chart_replace = Some(frame);
                return cosmic::iced::clipboard::read().map(move |text| {
                    cosmic::Action::from(Message::ChartClipboard { session_id, text })
                });
            }
            ChartAction::CancelReplace => self.close_chart(),
        }
        Task::none()
    }

    fn too_much(&mut self) {
        if let Some(panel) = self.chart_panel.as_mut() {
            panel.notice = Some(Notice::Refused(
                cp::data_problem(amipro_pictures::chart::DataError::TooMuch).into(),
            ));
        }
    }

    /// The clipboard's answer to a Replace question: the chart replaces
    /// the frame's picture.
    pub(crate) fn replace_with_chart(&mut self, text: Option<String>) {
        let Some(frame) = self.chart_replace.take() else {
            return;
        };
        let data = text.as_deref().and_then(cp::clipboard_data);
        let sample = data.is_none();
        let chart = cp::new_chart(data);
        if self.make_chart(Entry::Replace(frame), &chart).is_some() {
            let notice = sample.then(|| Notice::Info(cp::SAMPLE_NOTE.into()));
            self.open_chart_panel(frame, notice);
        }
    }

    // ---------------------------------------------------------------- image

    /// Save Chart as Image…: the portal's save dialog, PNG or SVG.
    fn save_chart_image_task(&mut self) -> Task<cosmic::Action<Message>> {
        let Some(frame) = self.chart_panel.as_ref().and_then(|panel| panel.frame) else {
            return Task::none();
        };
        if amipro_editing::drawing::drawing_of(&self.document, frame).is_none() {
            self.status = "This chart has no drawing to save".into();
            return Task::none();
        }
        let session_id = self.id;
        let start = self.start_folder();
        cosmic::task::future(async move {
            let png = FileFilter::new("PNG image (.png)").glob("*.png");
            let svg = FileFilter::new("SVG image (.svg)").glob("*.svg");
            let mut dialog = file_chooser::save::Dialog::new()
                .title("Save Chart as Image".to_owned())
                .file_name("Chart.png".to_owned())
                .filter(png)
                .filter(svg);
            if let Some(start) = start {
                dialog = dialog.directory(start);
            }
            let result = match dialog.save_file().await {
                Ok(response) => response
                    .url()
                    .and_then(|url| url.to_file_path().ok())
                    .map_or_else(
                        || FileDialogResult::Failed("the destination is not a local file".into()),
                        FileDialogResult::Selected,
                    ),
                Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                Err(error) => FileDialogResult::Failed(error.to_string()),
            };
            Message::ChartImageChosen { session_id, result }
        })
    }

    /// Writes the chart's picture to `path` (SVG for `.svg`, else PNG at
    /// 300 dpi).
    pub(crate) fn save_chart_image(&mut self, path: &std::path::Path) {
        let Some(frame) = self.chart_panel.as_ref().and_then(|panel| panel.frame) else {
            return;
        };
        let Some(drawing) = amipro_editing::drawing::drawing_of(&self.document, frame) else {
            self.status = "This chart has no drawing to save".into();
            return;
        };
        let format = crate::draw_files::Format::of(path).unwrap_or(crate::draw_files::Format::Png);
        let format = if format == crate::draw_files::Format::Sdw {
            crate::draw_files::Format::Png
        } else {
            format
        };
        let path = if crate::draw_files::Format::of(path) == Some(format) {
            path.to_path_buf()
        } else {
            path.with_extension(if format == crate::draw_files::Format::Svg {
                "svg"
            } else {
                "png"
            })
        };
        self.status = match crate::draw_files::file_bytes(&drawing, format)
            .map(|bytes| std::fs::write(&path, bytes))
        {
            Some(Ok(())) => format!("Chart saved as {}", path.display()),
            Some(Err(error)) => format!("Save Chart as Image failed: {error}"),
            None => "Save Chart as Image could not draw the chart".into(),
        };
    }

    // ------------------------------------------------------------ the page

    /// Double-click or Enter on a chart frame: the panel, or Draw mode when
    /// the chart was last edited with Drawing (as Ami Pro opens the last
    /// function used).
    pub(crate) fn open_chart_frame(&mut self, frame: FrameId) {
        if self.chart_drawing.contains(&frame) {
            self.enter_draw_mode(frame);
        } else {
            self.open_chart_panel(frame, None);
        }
    }

    /// Whether `frame` holds a chart (a picture with the chart driver).
    pub(crate) fn is_chart_frame(&self, frame: FrameId) -> bool {
        matches!(
            self.document.frame(frame).map(|frame| &frame.content),
            Some(FrameContent::Picture(picture)) if picture.format() == PictureFormat::Chart
        )
    }
}

/// Sets the depth of whichever 3D mode is on.
fn set_depth(chart: &mut Chart, depth: u16) {
    if chart.perspective_depth != 0 {
        chart.perspective_depth = depth;
    } else if chart.oblique_depth != 0 {
        chart.oblique_depth = depth;
    }
}
