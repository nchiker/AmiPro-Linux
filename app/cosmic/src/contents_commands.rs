//! Tools › TOC, Index… and Edit › Mark Text ▸ on the page surface (the TOC,
//! index, and glossary run; `docs/Design/UI_SPEC.md` §36): the Contents &
//! Index panel's actions, the layout as the contents' host, and generating
//! a TOC or an index in place or into a new tab (TX1–TX6). Each change is
//! one command and one undo step.

use amipro_document::contents::{
    ContentsSettings, IndexEntry, Leader, MasterSettings, TOC_LEVELS, TocLevel, TocMark,
    TocSettings,
};
use amipro_document::{Document, FlowId, InlineObject, PowerMark, power_kind};
use amipro_editing::contents::{
    ContentsHost, IndexMark, IndexOptions, IndexPlace, IndexRequest, MarkChanges, TocPlace,
    TocRequest, build_index, collect_index, collect_toc, index_mark, toc_levels, toc_mark,
};
use amipro_editing::{Command, TextPosition};
use amipro_layout::DocumentLayout;

use crate::contents_panel::{
    self, ContentsAction, ContentsPanel, EntryRow, IndexChoice, MarkAt, Page, Shown, TocChoice,
};
use crate::reference_commands::page_showing;
use crate::{AmiProApp, Message, PageSelection};

/// The layout as the contents' host: each place's page and its number as
/// the page's page-number field shows it.
pub struct LayoutHost<'a> {
    pub document: &'a Document,
    pub layout: &'a DocumentLayout,
}

impl LayoutHost<'_> {
    /// The text the page's page-number field shows (its leading text and
    /// style), or the number the page counts.
    fn shown(&self, index: usize) -> Option<String> {
        let page = self.layout.pages.get(index)?;
        for line in &page.lines {
            let Some(paragraph) = self
                .document
                .flow(line.flow)
                .and_then(|flow| flow.paragraphs.get(line.paragraph_index))
            else {
                continue;
            };
            for word in &line.words {
                if let Some(object) = word.object
                    && matches!(
                        paragraph.objects.get(object),
                        Some(InlineObject::PageNumber(_))
                    )
                    && let Some(text) = word.text.as_ref().filter(|text| !text.is_empty())
                {
                    return Some(text.clone());
                }
            }
        }
        Some(page.shown.to_string())
    }
}

impl ContentsHost for LayoutHost<'_> {
    fn page(&self, flow: FlowId, paragraph: usize, byte: usize) -> Option<(usize, String)> {
        let number = page_showing(self.layout, flow, paragraph, byte)?;
        let index = number as usize - 1;
        Some((index, self.shown(index)?))
    }

    fn number_text(&self, flow: FlowId, paragraph: usize) -> Option<String> {
        self.layout.pages.iter().find_map(|page| {
            page.lines
                .iter()
                .find(|line| line.flow == flow && line.paragraph_index == paragraph)
                .and_then(|line| line.bullet.as_ref())
                .map(|bullet| {
                    bullet
                        .pieces
                        .iter()
                        .filter_map(|piece| match piece {
                            amipro_layout::BulletPiece::Text { text, .. } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<String>()
                })
                .filter(|text| !text.trim().is_empty())
        })
    }
}

/// An index entry's label in the Entries list.
fn label(entry: &IndexEntry) -> String {
    let mut label = entry.primary.clone();
    if !entry.secondary.is_empty() {
        label.push_str(" · ");
        label.push_str(&entry.secondary);
    }
    if !entry.other.is_empty() {
        label.push_str(&entry.other);
    }
    label
}

/// The index mark at a place, if it is still there.
fn mark_entry(document: &Document, at: MarkAt) -> Option<IndexEntry> {
    let raw = &document
        .flow(at.at.flow)?
        .paragraphs
        .get(at.at.paragraph)?
        .marks
        .get(at.mark)?
        .raw;
    let power = PowerMark::parse(raw)?;
    (!power.end && power.kind == power_kind::INDEX)
        .then(|| IndexEntry::parse(&power.code))
        .flatten()
}

/// Every index mark's place in text order: the main text first, then the
/// text frames.
fn mark_places(document: &Document) -> Vec<(MarkAt, IndexEntry)> {
    let mut flows = vec![document.main_flow_id()];
    flows.extend(document.frames().filter_map(|frame| match frame.content {
        amipro_document::FrameContent::Text(flow) => Some(flow),
        _ => None,
    }));
    let mut places = Vec::new();
    for flow in flows {
        let Some(text) = document.flow(flow) else {
            continue;
        };
        for (paragraph_index, paragraph) in text.paragraphs.iter().enumerate() {
            for (index, mark) in paragraph.marks.iter().enumerate() {
                let at = MarkAt {
                    at: TextPosition {
                        flow,
                        paragraph: paragraph_index,
                        byte: mark.byte,
                    },
                    mark: index,
                };
                if let Some(entry) = mark_entry(document, at) {
                    places.push((at, entry));
                }
            }
        }
    }
    places
}

/// Whether `text` has `word` as a whole word at `at`.
fn whole_word(text: &str, at: usize, length: usize) -> bool {
    let before = text[..at].chars().next_back();
    let after = text[at + length..].chars().next();
    !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
}

impl AmiProApp {
    /// Opens the Contents & Index panel at `page`; Mark Index Entry takes
    /// the selection as the Primary, or the mark at the caret.
    pub(crate) fn open_contents_panel(&mut self, page: Page) {
        let mut panel = self
            .contents
            .take()
            .unwrap_or_else(|| ContentsPanel::new(page));
        panel.page = page;
        panel.message = None;
        if page == Page::Entries {
            let selected = self.selected_text().unwrap_or_default();
            let caret = self.page_selection.ordered().0;
            let at_caret = mark_places(&self.document)
                .into_iter()
                .find(|(at, _)| at.at == caret);
            match at_caret {
                Some((at, entry)) if selected.is_empty() => self.load_mark(&mut panel, at, &entry),
                _ => {
                    let first_line = selected.lines().next().unwrap_or("").to_string();
                    panel.new_entry(&first_line);
                    panel.toc_text = first_line.chars().take(240).collect();
                }
            }
            panel.toc_level = self.caret_toc_level().to_string();
        }
        self.close_rail_panels();
        self.contents = Some(panel);
        self.open_rail();
    }

    /// TOC Entry's first level: the caret paragraph style's TOC level, or 1
    /// (`1588:1f96`).
    fn caret_toc_level(&self) -> usize {
        let caret = self.page_selection.ordered().0;
        let style = self
            .document
            .flow(caret.flow)
            .and_then(|flow| flow.paragraphs.get(caret.paragraph))
            .and_then(|paragraph| self.document.styles().get(&paragraph.style));
        style
            .and_then(|style| {
                toc_levels(&self.document)
                    .iter()
                    .position(|level| level.style.eq_ignore_ascii_case(&style.name))
            })
            .map_or(1, |index| index + 1)
    }

    fn load_mark(&self, panel: &mut ContentsPanel, at: MarkAt, entry: &IndexEntry) {
        panel.primary = entry.primary.clone();
        panel.secondary = entry.secondary.clone();
        panel.page_number = entry.page_number;
        panel.other_on = !entry.other.is_empty();
        panel.other = entry.other.clone();
        panel.current = Some(at);
    }

    fn set_levels(&mut self, levels: Vec<TocLevel>) {
        let mut settings = self.document.contents().clone();
        settings.toc = Some(TocSettings::from_levels(&levels));
        self.set_contents(settings);
    }

    fn set_contents(&mut self, settings: ContentsSettings) {
        if &settings != self.document.contents() {
            self.execute_page_commands(&[Command::SetContents(settings)]);
        }
    }

    pub(crate) fn levels(&self) -> Vec<TocLevel> {
        toc_levels(&self.document)
    }

    fn master(&self) -> MasterSettings {
        self.document.contents().master.clone().unwrap_or_default()
    }

    /// The width a TOC fills: the main text's column on the caret's page.
    pub(crate) fn toc_width(&self) -> i32 {
        self.pages
            .layout()
            .pages
            .first()
            .and_then(|page| page.columns.first())
            .map_or(9_360, |column| column.width.round() as i32)
    }

    fn index_marks(&self) -> Vec<IndexMark> {
        let host = LayoutHost {
            document: &self.document,
            layout: self.pages.layout(),
        };
        collect_index(&self.document, &host)
    }

    /// A document made from Ami Pro's `name` style sheet (`_TOC.STY`,
    /// `_INDEX.STY`) when one is found, else a new document.
    pub(crate) fn output_document(&self, sheet: &str, title: &str) -> Document {
        let mut document = crate::new_document();
        document.title = title.to_string();
        if let Some(path) = amipro_formats::find_style_sheet(None, sheet, &self.style_folders())
            && let Ok(bytes) = std::fs::read(path)
            && let Ok(sheet) = amipro_formats::read_style_sheet(&bytes, sheet.to_string())
            && let Ok(applied) = amipro_editing::apply(&document, &Command::UseStyleSheet { sheet })
        {
            document = applied.document;
        }
        document
    }

    /// Opens `document` in a new tab.
    pub(crate) fn open_in_new_tab(&mut self, document: Document, status: &str) {
        let session = crate::DocumentSession::new(
            self.next_session_id,
            document,
            crate::DocumentLocation::Untitled,
            status.to_string(),
        );
        self.next_session_id += 1;
        self.sessions.push(session);
        self.active_session = self.sessions.len() - 1;
    }

    fn generate_toc(&mut self, choice: TocChoice) {
        let levels = self.levels();
        let host = LayoutHost {
            document: &self.document,
            layout: self.pages.layout(),
        };
        let entries = collect_toc(&self.document, &levels, &host);
        if entries.is_empty() {
            self.set_contents_message(
                "No TOC entries: give a style a level, or mark entries on the Entries page.",
            );
            return;
        }
        let count = entries.len();
        let width = self.toc_width();
        let place = match choice {
            TocChoice::Start => TocPlace::Start,
            TocChoice::Caret => TocPlace::At(self.page_selection.ordered().0),
            TocChoice::NewDocument => TocPlace::Output,
        };
        let request = Command::PutToc(Box::new(TocRequest {
            place,
            levels,
            entries,
            width,
        }));
        if choice == TocChoice::NewDocument {
            let title = format!("{} TOC", self.document.title);
            let output = self.output_document("_TOC.STY", &title);
            match amipro_editing::apply(&output, &request) {
                Ok(applied) => {
                    self.open_in_new_tab(applied.document, "Table of contents");
                    self.status = format!("Table of contents with {count} entries in a new tab");
                }
                Err(error) => self.set_contents_message(error),
            }
            return;
        }
        let replaced = amipro_editing::contents::toc_table(&self.document).is_some();
        // The levels used are stored as the document's `[toc]`, as TOC
        // Options' OK stores them in Ami Pro, so that Ami Pro makes the same
        // TOC again (TQ5; twin ATI1).
        let mut commands = Vec::new();
        if self.document.contents().toc.is_none() && !self.levels().is_empty() {
            let mut settings = self.document.contents().clone();
            settings.toc = Some(TocSettings::from_levels(&self.levels()));
            commands.push(Command::SetContents(settings));
        }
        commands.push(request);
        self.execute_page_commands(&[Command::Several(commands)]);
        let message = if replaced && choice == TocChoice::Start {
            format!("Table of contents updated: {count} entries")
        } else {
            format!("Table of contents made: {count} entries")
        };
        self.set_contents_message(&message);
        self.status = message;
    }

    fn generate_index(&mut self, choice: IndexChoice) {
        let marks = self.index_marks();
        let options = IndexOptions {
            separators: self.master().separators(),
            ranges: self.preferences.index_ranges,
        };
        let lines = build_index(&marks, options);
        if lines.is_empty() {
            self.set_contents_message("No index entries: mark them on the Entries page.");
            return;
        }
        let count = lines.len();
        match choice {
            IndexChoice::NewDocument => {
                let title = format!("{} Index", self.document.title);
                let output = self.output_document("_INDEX.STY", &title);
                let request = Command::PutIndex(Box::new(IndexRequest {
                    place: IndexPlace::Output,
                    lines,
                    styles: Vec::new(),
                }));
                match amipro_editing::apply(&output, &request) {
                    Ok(applied) => {
                        self.open_in_new_tab(applied.document, "Index");
                        self.status = format!("Index of {count} lines in a new tab");
                    }
                    Err(error) => self.set_contents_message(error),
                }
            }
            IndexChoice::End => {
                let styles = self
                    .output_document("_INDEX.STY", "")
                    .styles()
                    .values()
                    .cloned()
                    .collect();
                self.execute_page_commands(&[Command::PutIndex(Box::new(IndexRequest {
                    place: IndexPlace::End,
                    lines,
                    styles,
                }))]);
                self.set_contents_message(&format!("Index of {count} lines at the end"));
            }
        }
    }

    fn set_contents_message(&mut self, message: &str) {
        if let Some(panel) = self.contents.as_mut() {
            panel.message = Some(message.to_string());
        }
    }

    /// Mark: a new entry at the caret, or the shown mark changed.
    fn mark_entry(&mut self) {
        let Some(panel) = self.contents.as_ref() else {
            return;
        };
        let entry = panel.entry();
        if entry.primary.trim().is_empty() {
            self.set_contents_message("Must specify a primary.");
            return;
        }
        let current = panel
            .current
            .filter(|at| mark_entry(&self.document, *at).is_some());
        let changes = match current {
            Some(at) => MarkChanges {
                remove: vec![(at.at.flow, at.at.paragraph, at.mark)],
                add: vec![(at.at, index_mark(&entry))],
            },
            None => {
                let at = self.page_selection.ordered().0;
                MarkChanges {
                    remove: Vec::new(),
                    add: vec![(at, index_mark(&entry))],
                }
            }
        };
        self.execute_page_commands(&[Command::PutMarks(changes)]);
        if let Some(panel) = self.contents.as_mut() {
            let primary = panel.primary.clone();
            panel.new_entry("");
            panel.message = Some(format!("Marked {primary}"));
        }
    }

    /// Mark All (TX4): the entry at every occurrence of its text in the
    /// main text and text frames, as whole words, skipping places that
    /// already hold the same entry; one undo step.
    fn mark_all(&mut self) {
        let Some(panel) = self.contents.as_ref() else {
            return;
        };
        let entry = panel.entry();
        let exact = panel.exact_case;
        let word = if entry.secondary.is_empty() {
            entry.primary.clone()
        } else {
            entry.secondary.clone()
        };
        let needle = word.trim();
        if needle.is_empty() {
            return;
        }
        let raw = index_mark(&entry);
        let mut flows = vec![self.document.main_flow_id()];
        flows.extend(
            self.document
                .frames()
                .filter_map(|frame| match frame.content {
                    amipro_document::FrameContent::Text(flow) => Some(flow),
                    _ => None,
                }),
        );
        let mut add = Vec::new();
        for flow in flows {
            let Some(text) = self.document.flow(flow) else {
                continue;
            };
            for (paragraph_index, paragraph) in text.paragraphs.iter().enumerate() {
                let (haystack, needle) = if exact {
                    (paragraph.text.clone(), needle.to_string())
                } else {
                    (paragraph.text.to_lowercase(), needle.to_lowercase())
                };
                if haystack.len() != paragraph.text.len() {
                    continue;
                }
                let mut from = 0;
                while let Some(found) = haystack[from..].find(&needle) {
                    let at = from + found;
                    from = at + needle.len();
                    if !whole_word(&paragraph.text, at, needle.len()) {
                        continue;
                    }
                    let end = at + needle.len();
                    let already = paragraph
                        .marks
                        .iter()
                        .any(|mark| (at..=end).contains(&mark.byte) && mark.raw == raw);
                    if !already {
                        add.push((
                            TextPosition {
                                flow,
                                paragraph: paragraph_index,
                                byte: end,
                            },
                            raw.clone(),
                        ));
                    }
                }
            }
        }
        if add.is_empty() {
            self.set_contents_message(&format!("No unmarked “{needle}” found"));
            return;
        }
        let count = add.len();
        self.execute_page_commands(&[Command::PutMarks(MarkChanges {
            remove: Vec::new(),
            add,
        })]);
        self.set_contents_message(&format!(
            "Marked {count} {}",
            if count == 1 { "place" } else { "places" }
        ));
    }

    fn remove_mark(&mut self) {
        let Some(at) = self.contents.as_ref().and_then(|panel| panel.current) else {
            return;
        };
        if mark_entry(&self.document, at).is_none() {
            return;
        }
        self.execute_page_commands(&[Command::PutMarks(MarkChanges {
            remove: vec![(at.at.flow, at.at.paragraph, at.mark)],
            add: Vec::new(),
        })]);
        if let Some(panel) = self.contents.as_mut() {
            panel.new_entry("");
            panel.message = Some("Entry removed".into());
        }
    }

    /// Next and Prev. Mark, Next and Prev. Same: from the shown mark or the
    /// caret, without wrapping, as Ami Pro's dialog.
    fn step_mark(&mut self, forward: bool, same: bool) -> cosmic::app::Task<Message> {
        let places = mark_places(&self.document);
        let primary = self
            .contents
            .as_ref()
            .map(|panel| panel.primary.clone())
            .unwrap_or_default();
        let from = self.contents.as_ref().and_then(|panel| panel.current);
        let caret = self.page_selection.ordered().0;
        let position = |at: &MarkAt| {
            let main = self.document.main_flow_id();
            (
                at.at.flow != main,
                at.at.flow,
                at.at.paragraph,
                at.at.byte,
                at.mark,
            )
        };
        let here = from.map(|at| position(&at)).unwrap_or((
            caret.flow != self.document.main_flow_id(),
            caret.flow,
            caret.paragraph,
            caret.byte,
            if forward { usize::MAX } else { 0 },
        ));
        let candidates = places
            .into_iter()
            .filter(|(_, entry)| !same || entry.primary == primary);
        let found = if forward {
            candidates
                .filter(|(at, _)| position(at) > here)
                .min_by_key(|(at, _)| position(at))
        } else {
            candidates
                .filter(|(at, _)| position(at) < here)
                .max_by_key(|(at, _)| position(at))
        };
        match found {
            Some((at, entry)) => {
                if let Some(mut panel) = self.contents.take() {
                    self.load_mark(&mut panel, at, &entry);
                    panel.message = None;
                    self.contents = Some(panel);
                }
                self.go_to_mark(at)
            }
            None => {
                self.set_contents_message(if forward {
                    "No more index entries after this one"
                } else {
                    "No index entries before this one"
                });
                cosmic::app::Task::none()
            }
        }
    }

    fn go_to_mark(&mut self, at: MarkAt) -> cosmic::app::Task<Message> {
        let page = page_showing(self.pages.layout(), at.at.flow, at.at.paragraph, at.at.byte)
            .unwrap_or(self.page_selection.page);
        self.page_selection = PageSelection::caret(at.at, page);
        self.sync_editor_to_page();
        self.scroll_to_caret()
    }

    fn mark_toc_entry(&mut self) {
        let Some(panel) = self.contents.as_ref() else {
            return;
        };
        let Ok(level) = panel.toc_level.trim().parse::<u8>() else {
            return;
        };
        if !(1..=TOC_LEVELS as u8).contains(&level) || panel.toc_text.is_empty() {
            return;
        }
        let mark = TocMark {
            level,
            text: panel.toc_text.chars().take(240).collect(),
        };
        let at = self.page_selection.ordered().0;
        self.execute_page_commands(&[Command::PutMarks(MarkChanges {
            remove: Vec::new(),
            add: vec![(at, toc_mark(&mark))],
        })]);
        self.set_contents_message(&format!("TOC entry marked at level {level}"));
    }

    pub(crate) fn update_contents(&mut self, action: ContentsAction) -> cosmic::app::Task<Message> {
        use ContentsAction as A;
        match action {
            A::Close => {
                self.contents = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            A::Page(page) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.page = page;
                    panel.message = None;
                }
            }
            A::Promote(index) if index > 0 => {
                let mut levels = self.levels();
                levels.swap(index, index - 1);
                self.set_levels(levels);
            }
            A::Demote(index) => {
                let mut levels = self.levels();
                if index + 1 < levels.len() {
                    levels.swap(index, index + 1);
                    self.set_levels(levels);
                }
            }
            A::RemoveLevel(index) => {
                let mut levels = self.levels();
                if index < levels.len() {
                    levels.remove(index);
                    self.set_levels(levels);
                }
            }
            A::AddLevel(index) => {
                let mut levels = self.levels();
                if let Some(style) = self.other_styles().get(index).cloned()
                    && levels.len() < TOC_LEVELS
                {
                    levels.push(TocLevel {
                        style,
                        page_number: true,
                        right_align: true,
                        leader: Leader::Dots,
                    });
                    self.set_levels(levels);
                }
            }
            A::LevelPageNumber(index, on) => {
                self.change_level(index, |level| level.page_number = on)
            }
            A::LevelRightAlign(index, on) => self.change_level(index, |level| {
                level.right_align = on;
                level.leader = if on { Leader::Dots } else { Leader::None };
            }),
            A::LevelLeader(index, choice) => self.change_level(index, |level| {
                if let Some((leader, _)) = contents_panel::leaders(level.right_align).get(choice) {
                    level.leader = *leader;
                }
            }),
            A::TocPlace(choice) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.toc_place = choice;
                }
            }
            A::GenerateToc => {
                let choice = self
                    .contents
                    .as_ref()
                    .map(|panel| panel.toc_place)
                    .unwrap_or_default();
                self.generate_toc(choice);
            }
            A::Separators(on) => {
                let mut settings = self.document.contents().clone();
                let mut master = settings.master.clone().unwrap_or_default();
                master.flags = if on {
                    master.flags | MasterSettings::SEPARATORS
                } else {
                    master.flags & !MasterSettings::SEPARATORS
                };
                settings.master = Some(master);
                self.set_contents(settings);
            }
            A::Ranges(on) => {
                self.preferences.index_ranges = on;
                self.save_preferences();
            }
            A::IndexPlace(choice) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.index_place = choice;
                }
            }
            A::GenerateIndex => {
                let choice = self
                    .contents
                    .as_ref()
                    .map(|panel| panel.index_place)
                    .unwrap_or_default();
                self.generate_index(choice);
            }
            A::Primary(text) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.primary = text;
                }
            }
            A::Secondary(text) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.secondary = text;
                }
            }
            A::PageNumber(on) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.page_number = on;
                }
            }
            A::OtherOn(on) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.other_on = on;
                }
            }
            A::Other(text) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.other = text;
                    panel.other_on = true;
                }
            }
            A::ChoosePrimary(index) => {
                let primaries = self.primaries();
                if let (Some(panel), Some(primary)) = (self.contents.as_mut(), primaries.get(index))
                {
                    panel.primary = primary.clone();
                }
            }
            A::ChooseSecondary(index) => {
                let secondaries = self.secondaries();
                if let (Some(panel), Some(secondary)) =
                    (self.contents.as_mut(), secondaries.get(index))
                {
                    panel.secondary = secondary.clone();
                }
            }
            A::Mark => self.mark_entry(),
            A::MarkAll => self.mark_all(),
            A::ExactCase(on) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.exact_case = on;
                }
            }
            A::Remove => self.remove_mark(),
            A::NextMark => return self.step_mark(true, false),
            A::PreviousMark => return self.step_mark(false, false),
            A::NextSame => return self.step_mark(true, true),
            A::PreviousSame => return self.step_mark(false, true),
            A::GoTo(at) => {
                if let Some(entry) = mark_entry(&self.document, at)
                    && let Some(mut panel) = self.contents.take()
                {
                    self.load_mark(&mut panel, at, &entry);
                    self.contents = Some(panel);
                    return self.go_to_mark(at);
                }
            }
            A::TocLevel(text) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.toc_level = text.chars().filter(char::is_ascii_digit).take(1).collect();
                }
            }
            A::TocText(text) => {
                if let Some(panel) = self.contents.as_mut() {
                    panel.toc_text = text.chars().take(240).collect();
                }
            }
            A::MarkToc => self.mark_toc_entry(),
            A::Promote(_) => {}
        }
        cosmic::app::Task::none()
    }

    /// The panel's lists, from the document as it is now (after every
    /// message while the panel shows).
    pub(crate) fn refresh_contents_lists(&mut self) {
        if self.contents.is_none() {
            return;
        }
        let (others, primaries, secondaries) =
            (self.other_styles(), self.primaries(), self.secondaries());
        if let Some(panel) = self.contents.as_mut() {
            panel.other_styles = others;
            panel.primaries = primaries;
            panel.secondaries = secondaries;
        }
    }

    fn change_level(&mut self, index: usize, change: impl FnOnce(&mut TocLevel)) {
        let mut levels = self.levels();
        if let Some(level) = levels.get_mut(index) {
            change(level);
            self.set_levels(levels);
        }
    }

    /// The paragraph styles that are no level, sorted by name.
    fn other_styles(&self) -> Vec<String> {
        let levels = self.levels();
        let mut names: Vec<String> = self
            .document
            .styles()
            .values()
            .map(|style| style.name.clone())
            .filter(|name| {
                !levels
                    .iter()
                    .any(|level| level.style.eq_ignore_ascii_case(name))
                    && !name.starts_with("TOC")
            })
            .collect();
        names.sort_by_key(|name| name.to_lowercase());
        names
    }

    /// The document's distinct primaries, sorted as the index sorts them.
    fn primaries(&self) -> Vec<String> {
        let mut primaries: Vec<String> = mark_places(&self.document)
            .into_iter()
            .map(|(_, entry)| entry.primary)
            .collect();
        primaries.sort_by(|a, b| amipro_editing::contents::index_order(a, b));
        primaries.dedup();
        primaries
    }

    /// The secondaries under the Primary typed (case-sensitive, as Ami
    /// Pro's dialog).
    fn secondaries(&self) -> Vec<String> {
        let primary = self
            .contents
            .as_ref()
            .map(|panel| panel.primary.clone())
            .unwrap_or_default();
        let mut secondaries: Vec<String> = mark_places(&self.document)
            .into_iter()
            .filter(|(_, entry)| entry.primary == primary && !entry.secondary.is_empty())
            .map(|(_, entry)| entry.secondary)
            .collect();
        secondaries.sort_by(|a, b| amipro_editing::contents::index_order(a, b));
        secondaries.dedup();
        secondaries
    }

    pub(crate) fn contents_view(&self) -> cosmic::Element<'_, Message> {
        let Some(state) = self.contents.as_ref() else {
            return cosmic::widget::space::horizontal().into();
        };
        let marks = self.index_marks();
        let entries = marks
            .iter()
            .filter_map(|mark| {
                let at = mark_places(&self.document)
                    .into_iter()
                    .find(|(place, _)| place.at == mark.at)?
                    .0;
                Some(EntryRow {
                    at,
                    label: label(&mark.entry),
                    page: mark.page.as_ref().map(|(_, text)| text.clone()),
                })
            })
            .collect();
        contents_panel::panel(
            state,
            Shown {
                levels: self.levels(),
                other_styles: &state.other_styles,
                has_toc: amipro_editing::contents::toc_table(&self.document).is_some(),
                separators: self.master().separators(),
                ranges: self.preferences.index_ranges,
                entries,
                primaries: &state.primaries,
                secondaries: &state.secondaries,
                example: contents_panel::example(&state.entry()),
            },
        )
    }
}

impl AmiProApp {
    /// The heading the caret's TOC row stands for, if the caret is in the
    /// generated TOC's table (TX6): the row's entry among the TOC's
    /// entries as they are now, by row while their number is the table's,
    /// else by its text.
    pub(crate) fn toc_link_target(&self) -> Option<TextPosition> {
        let caret = self.page_selection.focus;
        let (frame, row, _) = amipro_editing::tables::cell_of_flow(&self.document, caret.flow)?;
        let holder = self.document.frame(frame)?;
        if !amipro_document::contents::is_toc_table(holder) {
            return None;
        }
        let amipro_document::FrameContent::Table(table) = &holder.content else {
            return None;
        };
        let host = LayoutHost {
            document: &self.document,
            layout: self.pages.layout(),
        };
        let entries = collect_toc(&self.document, &self.levels(), &host);
        let columns = table.columns.len().max(1);
        if entries.len() * columns == table.cells.len() {
            return entries.get(row).map(|entry| entry.source);
        }
        let amipro_document::CellContent::Text(flow) = table.cells.get(row * columns)?.content;
        let text = self.document.flow(flow)?.paragraphs.first()?.text.clone();
        entries
            .iter()
            .find(|entry| text.starts_with(&entry.text))
            .map(|entry| entry.source)
    }

    /// Ctrl+click's jump: the caret to the TOC row's heading.
    pub(crate) fn follow_toc_link(&mut self) -> Option<cosmic::app::Task<Message>> {
        let target = self.toc_link_target()?;
        let page = page_showing(
            self.pages.layout(),
            target.flow,
            target.paragraph,
            target.byte,
        )
        .unwrap_or(self.page_selection.page);
        self.page_selection = PageSelection::caret(target, page);
        self.sync_editor_to_page();
        self.status = "Went to the heading".into();
        Some(self.scroll_to_caret())
    }
}
