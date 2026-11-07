//! The app's side of notes, bookmarks, and footnotes (the notes run;
//! `docs/Design/UI_SPEC.md` §34): Insert › Note, Footnote, and
//! Cross-Reference…, Edit › Notes… and Bookmarks…, Tools › Footnote
//! Options…, and the three rail panels' actions, each change one command
//! and one undo step.

use amipro_document::bookmarks::{self, Bookmark, BookmarkTarget};
use amipro_document::{FrameContent, InlineObject, NoteInfo, WallClock};
use amipro_editing::references::{NewBookmark, note_at, note_text, notes};
use amipro_editing::{Command, TextPosition};
use amipro_layout::DocumentLayout;

use crate::references_panel::{
    BookmarkRow, Kind, NoteRow, ReferenceAction, ReferencePanel, Shown, parse_inches,
};
use crate::{AmiProApp, PageSelection};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A note's time as its card shows it: "26 Oct 2026, 09:54" (Ami Pro's
/// title printed the year less 1900).
pub fn note_when(time: u32) -> String {
    let clock = WallClock::from_ami_pro(time);
    format!(
        "{} {} {}, {:02}:{:02}",
        clock.day,
        MONTHS[usize::from(clock.month.clamp(1, 12)) - 1],
        clock.year,
        clock.hour,
        clock.minute
    )
}

/// The page that shows `byte` of paragraph `paragraph` of `flow`.
pub fn page_showing(
    layout: &DocumentLayout,
    flow: amipro_document::FlowId,
    paragraph: usize,
    byte: usize,
) -> Option<u32> {
    layout
        .pages
        .iter()
        .find(|page| {
            page.lines.iter().any(|line| {
                line.flow == flow
                    && line.paragraph_index == paragraph
                    && line.bytes.start <= byte
                    && byte <= line.bytes.end
            })
        })
        .map(|page| page.number)
}

/// Where a bookmark is shown: its page.
fn bookmark_page(
    document: &amipro_document::Document,
    layout: &DocumentLayout,
    bookmark: &Bookmark,
) -> Option<u32> {
    match &bookmark.target {
        BookmarkTarget::Text { start, .. } => {
            page_showing(layout, start.flow, start.paragraph, start.byte)
        }
        BookmarkTarget::Cells { mark, .. } => page_showing(layout, mark.flow, 0, 0),
        BookmarkTarget::Frame(frame)
        | BookmarkTarget::Cell {
            table: Some(frame), ..
        } => {
            let _ = document;
            layout
                .pages
                .iter()
                .find(|page| {
                    page.frames
                        .iter()
                        .any(|frame_box| frame_box.frame == *frame)
                })
                .map(|page| page.number)
        }
        BookmarkTarget::Cell { .. } => None,
    }
}

/// Each bookmark's page, by name in small letters.
pub fn bookmark_pages(
    document: &amipro_document::Document,
    layout: &DocumentLayout,
) -> std::collections::BTreeMap<String, u32> {
    bookmarks::bookmarks(document)
        .into_iter()
        .filter_map(|bookmark| {
            let page = bookmark_page(document, layout, &bookmark)?;
            Some((bookmark.name.to_lowercase(), page))
        })
        .collect()
}

/// Edit › Go To's Next item kinds this run adds (Ami Pro's Bookmark, Note,
/// Footnote Mark, and Footnote Text; NR4).
pub const GO_TO_ITEMS: [&str; 4] = ["Bookmark", "Note", "Footnote mark", "Footnote text"];

impl AmiProApp {
    /// Edit › Go To's Previous or Next of a Next item kind, from the caret.
    pub(crate) fn go_to_item(&mut self, item: usize, forward: bool) -> cosmic::app::Task<Message> {
        let main = self.document.main_flow_id();
        let mut places: Vec<TextPosition> = match item {
            0 => bookmarks::bookmarks(&self.document)
                .into_iter()
                .filter_map(|bookmark| match bookmark.target {
                    BookmarkTarget::Text { start, .. } => Some(TextPosition {
                        flow: start.flow,
                        paragraph: start.paragraph,
                        byte: start.byte,
                    }),
                    _ => None,
                })
                .collect(),
            1 => notes(&self.document)
                .into_iter()
                .map(|(at, _, _)| at)
                .collect(),
            _ => {
                let mut found = Vec::new();
                for (index, paragraph) in self.document.paragraphs().iter().enumerate() {
                    for (byte, object) in paragraph.object_offsets().zip(&paragraph.objects) {
                        if let InlineObject::Footnote { flow } = object {
                            found.push(if item == 2 {
                                TextPosition {
                                    flow: main,
                                    paragraph: index,
                                    byte,
                                }
                            } else {
                                TextPosition {
                                    flow: *flow,
                                    paragraph: 0,
                                    byte: 0,
                                }
                            });
                        }
                    }
                }
                found
            }
        };
        // Document order: the main text first, by place.
        places.sort_by_key(|place| (place.flow != main, place.flow, place.paragraph, place.byte));
        if places.is_empty() {
            self.status = format!(
                "No {} in the document",
                GO_TO_ITEMS[item.min(3)].to_lowercase()
            );
            return cosmic::app::Task::none();
        }
        let here = self.page_selection.focus;
        let key =
            |place: &TextPosition| (place.flow != main, place.flow, place.paragraph, place.byte);
        let now = key(&here);
        let target = if forward {
            places
                .iter()
                .find(|place| key(place) > now)
                .or(places.first())
        } else {
            places
                .iter()
                .rev()
                .find(|place| key(place) < now)
                .or(places.last())
        };
        let target = *target.expect("not empty");
        let page = page_showing(
            self.pages.layout(),
            target.flow,
            target.paragraph,
            target.byte,
        )
        .unwrap_or(self.page_selection.page);
        self.page_selection = PageSelection::caret(target, page);
        self.sync_editor_to_page();
        if item == 1 {
            // Ami Pro's Next item › Note opens the note.
            self.open_note_at(target);
        }
        self.status = format!("{} on page {page}", GO_TO_ITEMS[item.min(3)]);
        self.scroll_to_caret()
    }

    /// Opens a references panel in the rail; another rail panel closes.
    pub(crate) fn open_reference_panel(&mut self, kind: Kind) {
        let options = self.document.footnote_options().clone();
        let panel = match self.references.take() {
            Some(mut panel) => {
                panel.kind = kind;
                panel.sync_options(&options);
                panel
            }
            None => ReferencePanel::new(kind, &options),
        };
        self.close_rail_panels();
        self.references = Some(panel);
        self.open_rail();
    }

    /// The note initials and colour new notes take (Ami Pro's User Setup).
    fn new_note(&self) -> NoteInfo {
        NoteInfo {
            time: crate::clock::ami_pro_now(),
            initials: Some(self.preferences.note_initials.clone()),
            colour: Some(self.preferences.note_colour),
            number: None,
        }
    }

    /// Insert › Note: a note at the caret in any text, opened in the Notes
    /// panel, focused for typing.
    pub(crate) fn insert_note(&mut self) {
        let at = self.page_selection.ordered().0;
        let note = self.new_note();
        self.execute_page_commands(&[Command::InsertNote {
            at,
            note,
            text: String::new(),
        }]);
        if note_at(&self.document, at).is_some() {
            self.open_reference_panel(Kind::Notes);
            if let Some(panel) = self.references.as_mut() {
                panel.edit_note(at, "");
            }
            self.status = "Note inserted: type it in the Notes panel".into();
        }
    }

    /// Insert › Footnote: the reference at the caret and the caret in the
    /// footnote at the page's foot (NR8).
    pub(crate) fn insert_footnote(&mut self) {
        let at = self.page_selection.ordered().0;
        if at.flow != self.document.main_flow_id()
            && amipro_editing::tables::cell_of_flow(&self.document, at.flow).is_none()
        {
            self.status = "Footnotes go in the main text or a table's cell".into();
            return;
        }
        self.execute_page_commands(&[Command::InsertFootnote { at }]);
        if self.page_selection.flow() != self.document.main_flow_id() {
            self.status = "Footnote inserted: type its text; Esc returns".into();
        }
    }

    /// Insert › Cross-Reference…: the Bookmarks panel, where a bookmark's
    /// page or text is inserted.
    pub(crate) fn cross_reference(&mut self) {
        self.open_reference_panel(Kind::Bookmarks);
        self.status = "Choose a bookmark, then Its page or Its text".into();
    }

    /// Where a new bookmark would go: the selected frame, the selected
    /// cells, or the selection or caret in any text.
    fn new_bookmark_target(&self) -> NewBookmark {
        if let Some(frame) = self
            .frame_selection
            .as_ref()
            .and_then(|selection| selection.single(&self.document))
        {
            return NewBookmark::Frame(frame);
        }
        if let Some(cells) =
            crate::table_editing::cell_selection(&self.document, self.page_selection)
        {
            let range = cells.range();
            return NewBookmark::Cells {
                frame: cells.frame,
                row: range.rows.start,
                column: range.columns.start,
                rows: range.rows.len() as u16,
                columns: range.columns.len() as u16,
            };
        }
        let (start, end) = self.page_selection.ordered();
        NewBookmark::Text { start, end }
    }

    /// Goes to a bookmark: selects its text, cells, or frame.
    fn go_to_bookmark(&mut self, name: &str) -> cosmic::app::Task<Message> {
        let Some(bookmark) = bookmarks::bookmark_named(&self.document, name) else {
            self.status = format!("No bookmark named {name}");
            return cosmic::app::Task::none();
        };
        let layout = self.pages.layout();
        let page = bookmark_page(&self.document, layout, &bookmark).unwrap_or(1);
        match &bookmark.target {
            BookmarkTarget::Text { start, end } => {
                let start = TextPosition {
                    flow: start.flow,
                    paragraph: start.paragraph,
                    byte: start.byte,
                };
                let end = end.map_or(start, |end| TextPosition {
                    flow: end.flow,
                    paragraph: end.paragraph,
                    byte: end.byte,
                });
                self.page_selection = PageSelection {
                    anchor: start,
                    focus: end,
                    page,
                };
            }
            BookmarkTarget::Cells {
                mark,
                table: Some((frame, cell)),
                rows,
                columns,
            } => {
                let _ = mark;
                if let Some(FrameContent::Table(table)) =
                    self.document.frame(*frame).map(|frame| &frame.content)
                {
                    let width = table.columns.len();
                    let (row, column) = (cell / width, cell % width);
                    let last_row = (row + usize::from(*rows)).saturating_sub(1);
                    let last_column = (column + usize::from(*columns)).saturating_sub(1);
                    if let (Some(anchor), Some(focus)) = (
                        crate::table_editing::cell_start(&self.document, *frame, row, column),
                        crate::table_editing::cell_start(
                            &self.document,
                            *frame,
                            last_row,
                            last_column,
                        ),
                    ) {
                        self.page_selection = PageSelection {
                            anchor,
                            focus,
                            page,
                        };
                    }
                }
            }
            BookmarkTarget::Cell {
                table: Some(frame),
                row,
                column,
            } => {
                if let Some(start) = crate::table_editing::cell_start(
                    &self.document,
                    *frame,
                    *row as usize,
                    *column as usize,
                ) {
                    self.page_selection = PageSelection::caret(start, page);
                }
            }
            BookmarkTarget::Frame(frame) => {
                self.frame_selection =
                    Some(crate::frame_editing::FrameSelection::one(*frame, page));
            }
            _ => {
                self.status = format!("{name} names something no longer there");
                return cosmic::app::Task::none();
            }
        }
        self.sync_editor_to_page();
        self.status = format!("Bookmark {}", bookmark.name);
        self.scroll_to_caret()
    }

    /// The notes in order, and the index of the open one.
    fn note_rows(&self) -> Vec<NoteRow> {
        let layout = self.pages.layout();
        notes(&self.document)
            .into_iter()
            .map(|(at, note, flow)| NoteRow {
                page: page_showing(layout, at.flow, at.paragraph, at.byte),
                when: note_when(note.time),
                text: note_text(&self.document, flow),
                at,
                note,
            })
            .collect()
    }

    fn step_note(&mut self, forward: bool) -> cosmic::app::Task<Message> {
        let rows = self.note_rows();
        if rows.is_empty() {
            return cosmic::app::Task::none();
        }
        let current = self
            .references
            .as_ref()
            .and_then(|panel| panel.editing.as_ref())
            .and_then(|editing| rows.iter().position(|row| row.at == editing.at));
        let index = match (current, forward) {
            (Some(index), true) => (index + 1) % rows.len(),
            (Some(index), false) => (index + rows.len() - 1) % rows.len(),
            (None, true) => 0,
            (None, false) => rows.len() - 1,
        };
        let row = &rows[index];
        let (at, text, page) = (row.at, row.text.clone(), row.page);
        if let Some(panel) = self.references.as_mut() {
            panel.edit_note(at, &text);
        }
        self.page_selection = PageSelection::caret(at, page.unwrap_or(self.page_selection.page));
        self.sync_editor_to_page();
        self.scroll_to_caret()
    }

    fn step_footnote(&mut self, forward: bool) -> cosmic::app::Task<Message> {
        let main = self.document.main_flow_id();
        let mut places = Vec::new();
        for (index, paragraph) in self.document.paragraphs().iter().enumerate() {
            for (byte, object) in paragraph.object_offsets().zip(&paragraph.objects) {
                if matches!(object, InlineObject::Footnote { .. }) {
                    places.push(TextPosition {
                        flow: main,
                        paragraph: index,
                        byte,
                    });
                }
            }
        }
        if places.is_empty() {
            return cosmic::app::Task::none();
        }
        let here = self.page_selection.focus;
        let here = (here.flow == main).then_some((here.paragraph, here.byte));
        let target = if forward {
            places
                .iter()
                .find(|place| here.is_none_or(|here| (place.paragraph, place.byte) > here))
                .or(places.first())
        } else {
            places
                .iter()
                .rev()
                .find(|place| here.is_none_or(|here| (place.paragraph, place.byte) < here))
                .or(places.last())
        };
        let target = *target.expect("not empty");
        let page = self
            .pages
            .layout()
            .page_of(target.paragraph, target.byte)
            .unwrap_or(1);
        self.page_selection = PageSelection::caret(target, page);
        self.sync_editor_to_page();
        self.scroll_to_caret()
    }

    /// The references panels' actions.
    pub(crate) fn update_references(
        &mut self,
        action: ReferenceAction,
    ) -> cosmic::app::Task<Message> {
        use ReferenceAction as A;
        match action {
            A::Close => {
                self.references = None;
                self.contents = None;
                self.glossary = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            A::EditNote(at) => {
                let text = note_at(&self.document, at)
                    .map(|(_, flow)| note_text(&self.document, flow))
                    .unwrap_or_default();
                if let Some(panel) = self.references.as_mut() {
                    panel.edit_note(at, &text);
                }
            }
            A::NoteEdit(edit) => {
                let Some(panel) = self.references.as_mut() else {
                    return cosmic::app::Task::none();
                };
                let Some(editing) = panel.editing.as_mut() else {
                    return cosmic::app::Task::none();
                };
                let changes = edit.is_edit();
                editing.content.perform(edit);
                if changes {
                    let at = editing.at;
                    let text = editing.content.text();
                    let text = text.strip_suffix('\n').unwrap_or(&text).to_owned();
                    if let Some((mut note, _)) = note_at(&self.document, at) {
                        // The note's time is when it was last changed.
                        note.time = crate::clock::ami_pro_now();
                        let session = &mut self.sessions[self.active_session];
                        if let Err(error) = session.execute_frame_commands(
                            &[Command::SetNote {
                                at,
                                note,
                                text: Some(text),
                            }],
                            true,
                        ) {
                            self.status = error;
                        }
                    }
                }
            }
            A::GoToNote(at) => {
                let page = page_showing(self.pages.layout(), at.flow, at.paragraph, at.byte)
                    .unwrap_or(self.page_selection.page);
                self.page_selection = PageSelection::caret(at, page);
                self.sync_editor_to_page();
                return self.scroll_to_caret();
            }
            A::RemoveNote(at) => {
                if let Some(panel) = self.references.as_mut()
                    && panel
                        .editing
                        .as_ref()
                        .is_some_and(|editing| editing.at == at)
                {
                    panel.editing = None;
                }
                self.execute_page_commands(&[Command::RemoveNotes { at: Some(at) }]);
                self.status = "Note removed".into();
            }
            A::RemoveAllNotes => {
                if let Some(panel) = self.references.as_mut() {
                    panel.editing = None;
                }
                self.execute_page_commands(&[Command::RemoveNotes { at: None }]);
                self.status = "Every note removed (Undo brings them back)".into();
            }
            A::PreviousNote => return self.step_note(false),
            A::NextNote => return self.step_note(true),
            A::Initials(text) => {
                self.preferences.note_initials = text
                    .chars()
                    .filter(|character| *character != ',' && !character.is_control())
                    .take(6)
                    .collect();
                self.save_preferences();
            }
            A::NoteColour(colour) => {
                self.preferences.note_colour = colour;
                self.save_preferences();
                // The open note takes it too.
                let editing = self
                    .references
                    .as_ref()
                    .and_then(|panel| panel.editing.as_ref())
                    .map(|editing| editing.at);
                if let Some(at) = editing
                    && let Some((mut note, _)) = note_at(&self.document, at)
                {
                    note.colour = Some(colour);
                    self.execute_page_commands(&[Command::SetNote {
                        at,
                        note,
                        text: None,
                    }]);
                }
            }
            A::Name(text) => {
                if let Some(panel) = self.references.as_mut() {
                    panel.name = text;
                    panel.problem = None;
                }
            }
            A::Add => {
                let name = self
                    .references
                    .as_ref()
                    .map(|panel| panel.name.clone())
                    .unwrap_or_default();
                match bookmarks::check_name(&self.document, &name) {
                    Err(problem) => {
                        if let Some(panel) = self.references.as_mut() {
                            panel.problem = Some(problem.message().into());
                        }
                    }
                    Ok(name) => {
                        let target = self.new_bookmark_target();
                        let before = self.history.undo_len();
                        let selection = self.page_selection;
                        let session = &mut self.sessions[self.active_session];
                        let result = session.execute_frame_commands(
                            &[Command::AddBookmark {
                                name: name.clone(),
                                target,
                            }],
                            false,
                        );
                        self.page_selection = selection;
                        match result {
                            Ok(()) if self.history.undo_len() > before => {
                                if let Some(panel) = self.references.as_mut() {
                                    panel.selected = Some(name.clone());
                                    panel.name.clear();
                                }
                                self.status = format!("Bookmark {name} added");
                            }
                            Ok(()) => {}
                            Err(error) => {
                                if let Some(panel) = self.references.as_mut() {
                                    panel.problem = Some(
                                        error
                                            .strip_prefix("Frame change refused: ")
                                            .unwrap_or(&error)
                                            .to_owned(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            A::Select(name) => {
                if let Some(panel) = self.references.as_mut() {
                    panel.selected = Some(name.clone());
                    panel.name = name;
                    panel.problem = None;
                }
            }
            A::GoTo(name) => {
                let name = if name.is_empty() {
                    self.references
                        .as_ref()
                        .map(|panel| panel.name.trim().to_owned())
                        .unwrap_or_default()
                } else {
                    name
                };
                return self.go_to_bookmark(&name);
            }
            A::Rename => {
                let Some((from, to)) = self
                    .references
                    .as_ref()
                    .and_then(|panel| Some((panel.selected.clone()?, panel.name.clone())))
                else {
                    return cosmic::app::Task::none();
                };
                if let Err(problem) = bookmarks::check_name(&self.document, &to) {
                    if let Some(panel) = self.references.as_mut() {
                        panel.problem = Some(problem.message().into());
                    }
                    return cosmic::app::Task::none();
                }
                let selection = self.page_selection;
                let session = &mut self.sessions[self.active_session];
                if session
                    .execute_frame_commands(
                        &[Command::ChangeBookmark {
                            name: from,
                            to: Some(to.clone()),
                        }],
                        false,
                    )
                    .is_ok()
                    && let Some(panel) = self.references.as_mut()
                {
                    panel.selected = Some(to.trim().to_owned());
                }
                self.page_selection = selection;
            }
            A::Remove => {
                let Some(name) = self
                    .references
                    .as_ref()
                    .and_then(|panel| panel.selected.clone())
                else {
                    return cosmic::app::Task::none();
                };
                let selection = self.page_selection;
                let session = &mut self.sessions[self.active_session];
                let _ = session.execute_frame_commands(
                    &[Command::ChangeBookmark {
                        name: name.clone(),
                        to: None,
                    }],
                    false,
                );
                self.page_selection = selection;
                if let Some(panel) = self.references.as_mut() {
                    panel.selected = None;
                }
                self.status = format!("Bookmark {name} removed");
            }
            A::ShowBookmarks(on) => {
                self.preferences.show_bookmarks = on;
                self.save_preferences();
            }
            A::CrossReference { page } => {
                let Some(name) = self
                    .references
                    .as_ref()
                    .and_then(|panel| panel.selected.clone())
                else {
                    return cosmic::app::Task::none();
                };
                let code = if page {
                    format!("PageRef {name}")
                } else {
                    format!("Bookmark {name}")
                };
                let (start, end) = self.page_selection.ordered();
                self.insert_power_field_at(start, end, &code, 0);
            }
            A::Flag(bit, on) => {
                let mut options = self.document.footnote_options().clone();
                options.set(bit, on);
                self.set_footnote_options(options);
            }
            A::Start(text) => {
                if let Some(panel) = self.references.as_mut() {
                    panel.start.clone_from(&text);
                }
                if let Ok(start) = text.trim().parse::<u16>()
                    && (1..=9_999).contains(&start)
                {
                    let mut options = self.document.footnote_options().clone();
                    options.start = start;
                    self.set_footnote_options(options);
                }
            }
            A::Length(text) => {
                if let Some(panel) = self.references.as_mut() {
                    panel.length.clone_from(&text);
                }
                if let Some(length) = parse_inches(&text) {
                    let mut options = self.document.footnote_options().clone();
                    options.length = length;
                    options.set(amipro_document::FootnoteOptions::CUSTOM_LENGTH, true);
                    self.set_footnote_options(options);
                }
            }
            A::Indent(text) => {
                if let Some(panel) = self.references.as_mut() {
                    panel.indent.clone_from(&text);
                }
                if let Some(indent) = parse_inches(&text) {
                    let mut options = self.document.footnote_options().clone();
                    options.indent = indent;
                    self.set_footnote_options(options);
                }
            }
            A::ContinuedOn(text) => {
                let mut options = self.document.footnote_options().clone();
                options.continued_on = text;
                self.set_footnote_options(options);
            }
            A::ContinuedFrom(text) => {
                let mut options = self.document.footnote_options().clone();
                options.continued_from = text;
                self.set_footnote_options(options);
            }
            A::NumberStyle => self.open_footnote_number_style(),
            A::PreviousFootnote => return self.step_footnote(false),
            A::NextFootnote => return self.step_footnote(true),
        }
        cosmic::app::Task::none()
    }

    /// Changes Footnote Options as one step; typing in a message joins its
    /// keys.
    fn set_footnote_options(&mut self, options: amipro_document::FootnoteOptions) {
        if options == *self.document.footnote_options() {
            return;
        }
        let selection = self.page_selection;
        let session = &mut self.sessions[self.active_session];
        if let Err(error) =
            session.execute_frame_commands(&[Command::SetFootnoteOptions(options)], true)
        {
            self.status = error;
        }
        self.page_selection = selection;
    }

    /// Style Properties at the Footnote style's Bullets and numbers.
    fn open_footnote_number_style(&mut self) {
        let Some(style) = self
            .document
            .styles()
            .values()
            .find(|style| style.name.eq_ignore_ascii_case("Footnote"))
            .map(|style| style.id)
        else {
            self.status = "The Footnote style comes with the first footnote".into();
            return;
        };
        self.references = None;
        self.contents = None;
        self.glossary = None;
        self.open_style_properties(Some(style));
        if let Some(state) = self.style_properties.as_mut() {
            state.open =
                std::collections::BTreeSet::from([crate::style_properties::Section::Bullets]);
        }
    }

    pub(crate) fn references_view(&self) -> cosmic::Element<'_, Message> {
        let state = self.references.as_ref().expect("a references panel shows");
        let layout = self.pages.layout();
        let footnotes = self
            .document
            .paragraphs()
            .iter()
            .flat_map(|paragraph| &paragraph.objects)
            .filter(|object| matches!(object, InlineObject::Footnote { .. }))
            .count();
        crate::references_panel::panel(
            state,
            Shown {
                notes: self.note_rows(),
                initials: &self.preferences.note_initials,
                colour: self.preferences.note_colour,
                bookmarks: bookmarks::bookmarks(&self.document)
                    .into_iter()
                    .map(|bookmark| BookmarkRow {
                        page: bookmark_page(&self.document, layout, &bookmark),
                        bookmark,
                    })
                    .collect(),
                show_bookmarks: self.preferences.show_bookmarks,
                options: self.document.footnote_options(),
                footnotes,
            },
        )
    }

    /// A double-click on a note's mark (the caret beside it): the Notes
    /// panel at that note, as Ami Pro opens its window.
    pub(crate) fn open_note_at(&mut self, at: TextPosition) {
        let text = note_at(&self.document, at)
            .map(|(_, flow)| note_text(&self.document, flow))
            .unwrap_or_default();
        self.open_reference_panel(Kind::Notes);
        if let Some(panel) = self.references.as_mut() {
            panel.edit_note(at, &text);
        }
    }
}

/// The note whose mark stands at or just before `at`.
pub fn note_beside(document: &amipro_document::Document, at: TextPosition) -> Option<TextPosition> {
    let paragraph = document.flow(at.flow)?.paragraph(at.paragraph)?;
    let length = amipro_document::OBJECT_REPLACEMENT_CHARACTER.len_utf8();
    paragraph
        .object_offsets()
        .zip(&paragraph.objects)
        .find(|(byte, object)| {
            matches!(object, InlineObject::Note { .. })
                && (*byte == at.byte || byte + length == at.byte)
        })
        .map(|(byte, _)| TextPosition { byte, ..at })
}

use crate::Message;
