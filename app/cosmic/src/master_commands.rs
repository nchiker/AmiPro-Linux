//! The Master Document panel's behaviour and the set's live numbers
//! (`UI_SPEC.md` §37; MS1–MS4, MS9): File › Master Document…, each change
//! to the list one undo step on the master, the members read off the
//! interface's thread, and the numbers each open member continues from.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use amipro_document::contents::Sequence;
use amipro_editing::master::{is_master, is_member, set_files, set_outputs, stored_path};

use crate::master::{Carrier, Laid, MasterSet, MemberState, MemberView};
use crate::master_panel::{self, MasterAction, MasterPanel, Shown};
use crate::{AmiProApp, DocumentSession, Message};

/// What a tab made by the master panel holds (MS5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputKind {
    Toc,
    Index,
}

/// A member's numbers while its master is open (MS3): the set's, over the
/// ones it was opened with.
#[derive(Clone, Debug)]
pub struct LiveSequence {
    /// The member's own `[sequence]`, to return to when the master closes.
    pub stored: Option<Sequence>,
}

impl DocumentSession {
    /// The file the session's document came from or goes to.
    pub(crate) fn file_path(&self) -> Option<&Path> {
        self.location.path()
    }

    /// Gives the document the sequence `incoming` as its master's set
    /// numbers it (MS3), outside the history: the pages are laid out again
    /// when it changes. Its saved copy is changed the same way, so that the
    /// set's numbers alone do not make it modified.
    fn apply_live_sequence(&mut self, incoming: Option<Sequence>) {
        if self.live_sequence.is_none() {
            self.live_sequence = Some(LiveSequence {
                stored: self.document.contents().sequence.clone(),
            });
        }
        self.set_sequence(incoming);
    }

    /// Returns to the document's own numbers when its master is gone.
    fn release_live_sequence(&mut self) {
        if let Some(live) = self.live_sequence.take() {
            self.set_sequence(live.stored);
        }
    }

    fn set_sequence(&mut self, sequence: Option<Sequence>) {
        if self.document.contents().sequence == sequence {
            return;
        }
        for document in [&mut self.document, &mut self.clean_document] {
            let mut contents = document.contents().clone();
            contents.sequence = sequence.clone();
            document.set_contents(contents);
        }
        self.refresh_rich_layout();
    }
}

/// Reads a member off the interface's thread.
fn read_task(
    master: u64,
    stored: String,
    path: PathBuf,
    folders: Vec<PathBuf>,
) -> cosmic::app::Task<Message> {
    cosmic::task::future(async move {
        let result =
            crate::import::in_background(move || crate::master::read_member(&path, &folders))
                .await
                .map(Arc::new);
        Message::MasterRead {
            master,
            stored,
            result,
        }
    })
}

impl AmiProApp {
    /// File › Master Document…: the panel (MS1). On a member it says where
    /// its numbers start and looks for the master that lists it (MS4).
    pub(crate) fn open_master_panel(&mut self) {
        let master = self.document.contents().master.clone().unwrap_or_default();
        self.close_rail_panels();
        self.master_panel = Some(MasterPanel {
            toc_file: master.toc_file,
            index_file: master.index_file,
            message: None,
        });
        self.look_for_master();
        self.open_rail();
    }

    /// MS4: a member opened alone looks once for a master in its folder.
    fn look_for_master(&mut self) {
        if !is_member(&self.document) || self.live_sequence.is_some() || self.master_offer.is_some()
        {
            return;
        }
        let folders = self.style_folders();
        let found = self
            .file_path()
            .and_then(|path| crate::master::master_listing(path, &folders));
        if let Some(master) = &found {
            let name = master.file_name().unwrap_or_default().to_string_lossy();
            self.status = format!("Part of a master document: {name} lists it");
        }
        self.master_offer = Some(found);
    }

    pub(crate) fn master_view(&self) -> cosmic::Element<'_, Message> {
        let Some(state) = self.master_panel.as_ref() else {
            return cosmic::widget::space::horizontal().into();
        };
        let member = is_member(&self.document).then(|| {
            let sequence = self
                .document
                .contents()
                .sequence
                .clone()
                .unwrap_or_default();
            (sequence.page_base + 1, sequence.footnotes + 1)
        });
        let separators = self
            .document
            .contents()
            .master
            .as_ref()
            .is_some_and(|master| master.separators());
        master_panel::panel(Shown {
            set: self.master_set.as_ref(),
            toc_file: &state.toc_file,
            index_file: &state.index_file,
            separators,
            member,
            offer: self
                .master_offer
                .as_ref()
                .and_then(|offer| offer.as_deref()),
            live: self.live_sequence.is_some(),
            message: state.message.as_deref(),
        })
    }

    fn set_master_message(&mut self, message: impl Into<String>) {
        if let Some(panel) = self.master_panel.as_mut() {
            panel.message = Some(message.into());
        }
    }

    /// The master's files as stored.
    fn master_files(&self) -> Vec<String> {
        self.document
            .contents()
            .master
            .as_ref()
            .map(|master| master.files.clone())
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn master_files_for_tests(&self) -> Vec<String> {
        self.master_files()
    }

    /// Gives the master the list `files`, one undo step.
    fn change_master_files(&mut self, files: Vec<String>, selected: Option<usize>) {
        let command = set_files(&self.document, files);
        self.execute_document_command(&command);
        let set = self.master_set.get_or_insert_with(MasterSet::default);
        set.selected = selected;
    }

    pub(crate) fn update_master(&mut self, action: MasterAction) -> cosmic::app::Task<Message> {
        use MasterAction as A;
        let selected = self.master_set.as_ref().and_then(|set| set.selected);
        match action {
            A::Close => {
                self.master_panel = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            A::Add => {
                let start = self.start_folder();
                return cosmic::task::future(async move {
                    let paths = crate::file_dialogs::open_dialog()
                        .title("Add to Master Document".to_owned())
                        .directory(start)
                        .open_files()
                        .await
                        .map(|response| {
                            response
                                .urls()
                                .iter()
                                .filter_map(|url| url.to_file_path().ok())
                                .collect()
                        })
                        .unwrap_or_default();
                    Message::Master(MasterAction::Added(paths))
                });
            }
            A::Added(paths) => {
                let own = self.file_path().map(Path::to_path_buf);
                let mut refused = 0;
                let mut files = self.master_files();
                let mut at = selected.unwrap_or(files.len()).min(files.len());
                for path in paths {
                    let extension = path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .map(str::to_ascii_lowercase);
                    // MS8: Ami Pro's documents and the app's own.
                    if !matches!(extension.as_deref(), Some("sam" | "amipro"))
                        || own.as_deref() == Some(path.as_path())
                    {
                        refused += 1;
                        continue;
                    }
                    files.insert(at, stored_path(&path.to_string_lossy()));
                    at += 1;
                }
                if refused > 0 {
                    self.set_master_message(
                        "Only Ami Pro and AmiPro-Linux documents other than this one can be files of a master document.",
                    );
                }
                if files != self.master_files() {
                    self.change_master_files(files, None);
                }
            }
            A::Select(index) => {
                let set = self.master_set.get_or_insert_with(MasterSet::default);
                set.selected = (set.selected != Some(index)).then_some(index);
            }
            A::Remove => {
                if let Some(index) = selected {
                    let mut files = self.master_files();
                    if index < files.len() {
                        files.remove(index);
                        self.change_master_files(files, None);
                    }
                }
            }
            A::Up | A::Down => {
                if let Some(index) = selected {
                    let mut files = self.master_files();
                    let other = if matches!(action, A::Up) {
                        index.checked_sub(1)
                    } else {
                        (index + 1 < files.len()).then_some(index + 1)
                    };
                    if let Some(other) = other {
                        files.swap(index, other);
                        self.change_master_files(files, Some(other));
                    }
                }
            }
            A::Open(index) | A::CheckGo(index) => {
                let path = self
                    .master_set
                    .as_ref()
                    .and_then(|set| set.members.get(index))
                    .and_then(|member| member.path.clone());
                match path {
                    Some(path) => self.open_document_path(path),
                    None => self.set_master_message("That file is missing: Locate… it first."),
                }
            }
            A::OpenAll => {
                let master = self.active_session;
                let paths: Vec<PathBuf> = self
                    .master_set
                    .as_ref()
                    .map(|set| {
                        set.members
                            .iter()
                            .filter_map(|member| member.path.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                for path in paths {
                    self.open_document_path(path);
                }
                self.active_session = master;
            }
            A::Locate(index) => {
                let start = self.start_folder();
                return cosmic::task::future(async move {
                    let path = crate::file_dialogs::open_dialog()
                        .title("Locate File".to_owned())
                        .directory(start)
                        .open_file()
                        .await
                        .ok()
                        .and_then(|response| response.url().to_file_path().ok());
                    match path {
                        Some(path) => Message::Master(MasterAction::Located(index, path)),
                        None => Message::Repaint,
                    }
                });
            }
            A::Located(index, path) => {
                let mut files = self.master_files();
                if let Some(file) = files.get_mut(index) {
                    *file = stored_path(&path.to_string_lossy());
                    self.change_master_files(files, Some(index));
                }
            }
            A::TocFile(text) => {
                if let Some(panel) = self.master_panel.as_mut() {
                    panel.toc_file = text;
                }
            }
            A::IndexFile(text) => {
                if let Some(panel) = self.master_panel.as_mut() {
                    panel.index_file = text;
                }
            }
            A::CommitOutputs | A::Separators(_) => {
                let Some(panel) = self.master_panel.clone() else {
                    return cosmic::app::Task::none();
                };
                let separators = match action {
                    A::Separators(on) => on,
                    _ => self
                        .document
                        .contents()
                        .master
                        .as_ref()
                        .is_some_and(|master| master.separators()),
                };
                let command = set_outputs(
                    &self.document,
                    panel.toc_file.trim(),
                    panel.index_file.trim(),
                    separators,
                );
                if let amipro_editing::Command::SetContents(contents) = &command
                    && *contents != *self.document.contents()
                {
                    self.execute_document_command(&command);
                }
            }
            A::GenerateContents => return self.generate_set_contents(),
            A::GenerateIndex => return self.generate_set_index(),
            A::TreatAsSingle => {
                self.release_live_sequence();
                self.treated_single = true;
                let command = amipro_editing::master::treat_as_single(&self.document);
                self.execute_document_command(&command);
                self.master_offer = None;
                self.set_master_message("Treated as a single document: its numbers start from 1.");
            }
            A::OpenMaster => {
                if let Some(Some(path)) = self.master_offer.clone() {
                    self.open_document_path(path);
                }
            }
        }
        cosmic::app::Task::none()
    }

    /// A member read off the interface's thread.
    pub(crate) fn master_member_read(
        &mut self,
        master: u64,
        stored: String,
        result: Result<Arc<Laid>, String>,
    ) {
        let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == master)
        else {
            return;
        };
        let Some(set) = session.master_set.as_mut() else {
            return;
        };
        let paths: Vec<PathBuf> = set
            .members
            .iter()
            .filter(|member| member.stored == stored)
            .filter_map(|member| member.path.clone())
            .collect();
        set.reading.retain(|path| !paths.contains(path));
        for member in set
            .members
            .iter_mut()
            .filter(|member| member.stored == stored)
        {
            match &result {
                Ok(laid) => {
                    member.laid = Some(laid.clone());
                    member.state = MemberState::Ready;
                }
                Err(error) => member.state = MemberState::Unreadable(error.clone()),
            }
        }
    }

    /// Each file of the set as it is numbered now: an open tab's document
    /// and pages, or the file read and laid out again with the numbers it
    /// continues from. Files after one not read are left out, as Ami Pro's
    /// run stops there.
    fn set_documents(
        &self,
    ) -> Vec<(
        PathBuf,
        amipro_document::Document,
        amipro_layout::DocumentLayout,
    )> {
        let Some(set) = self.master_set.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for member in &set.members {
            let Some(path) = member.path.clone() else {
                break;
            };
            if let Some(session) = self.sessions.iter().find(|session| {
                session.file_path() == Some(path.as_path()) && !session.treated_single
            }) {
                out.push((
                    path,
                    session.document.clone(),
                    session.pages.layout().clone(),
                ));
                continue;
            }
            let Some(laid) = member.laid.as_ref() else {
                break;
            };
            let mut document = laid.document.clone();
            let mut contents = document.contents().clone();
            contents.sequence = member.incoming.clone();
            document.set_contents(contents);
            let mut pages = crate::page_layout::SessionPages::default();
            pages.update(&document);
            out.push((path, document, pages.layout().clone()));
        }
        out
    }

    /// What Print and Export PDF take on a master (MS6): the TOC file when
    /// named and found, the files in order, then the index file, each laid
    /// out with its numbers in the set; nothing for a document that is not
    /// a master or when This document only is chosen.
    pub(crate) fn print_set(&self) -> Vec<crate::print::SetPart> {
        if !is_master(&self.document) || self.print_this_only {
            return Vec::new();
        }
        let master = self.document.contents().master.clone().unwrap_or_default();
        let folder = self
            .file_path()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        let folders = self.style_folders();
        let output = |name: &str| -> Option<crate::print::SetPart> {
            if name.trim().is_empty() {
                return None;
            }
            let path = crate::master::resolve(name, folder.as_deref())?;
            let document = crate::master::read_document(&path, &folders).ok()?;
            let mut pages = crate::page_layout::SessionPages::default();
            pages.update(&document);
            Some(crate::print::SetPart {
                title: path.file_name()?.to_string_lossy().into_owned(),
                pages: None,
                folder: path.parent().map(Path::to_path_buf),
                layout: pages.layout().clone(),
                document,
            })
        };
        let mut parts: Vec<crate::print::SetPart> = output(&master.toc_file).into_iter().collect();
        for (path, document, layout) in self.set_documents() {
            parts.push(crate::print::SetPart {
                title: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                folder: path.parent().map(Path::to_path_buf),
                document,
                layout,
                pages: None,
            });
        }
        parts.extend(output(&master.index_file));
        parts
    }

    /// Generate Contents over the set (MS5): every file in order, with
    /// its pages in the set, by the master's TOC levels (Ami Pro applies the
    /// master's `[toc]` to every file), into a new tab on `_TOC.STY`; each
    /// row remembers its file and heading for Ctrl+click.
    fn generate_set_contents(&mut self) -> cosmic::app::Task<Message> {
        use amipro_editing::contents::{TocPlace, TocRequest, collect_toc};
        let levels = self.levels();
        let mut entries = Vec::new();
        let mut links = Vec::new();
        for (path, document, layout) in self.set_documents() {
            let host = crate::contents_commands::LayoutHost {
                document: &document,
                layout: &layout,
            };
            for entry in collect_toc(&document, &levels, &host) {
                links.push((path.clone(), entry.source));
                entries.push(entry);
            }
        }
        if entries.is_empty() {
            self.set_master_message(
                "No TOC entries in the files: give a style a level in TOC, Index… (the master's levels apply to every file).",
            );
            return cosmic::app::Task::none();
        }
        let count = entries.len();
        let request = amipro_editing::Command::PutToc(Box::new(TocRequest {
            place: TocPlace::Output,
            levels,
            entries,
            width: self.toc_width(),
        }));
        let title = format!("{} TOC", self.document.title);
        let output = self.output_document("_TOC.STY", &title);
        match amipro_editing::apply(&output, &request) {
            Ok(applied) => {
                let master = self.id;
                self.open_in_new_tab(applied.document, "Table of contents of the master document");
                self.set_links = links;
                self.output_of = Some((master, OutputKind::Toc));
                self.status = format!("Table of contents of the set: {count} entries in a new tab");
            }
            Err(error) => self.set_master_message(error.to_string()),
        }
        cosmic::app::Task::none()
    }

    /// Generate Index over the set (MS5): every file's marks with their
    /// pages in the set, sorted and merged as one index, into a new tab on
    /// `_INDEX.STY`.
    fn generate_set_index(&mut self) -> cosmic::app::Task<Message> {
        use amipro_editing::contents::{
            IndexOptions, IndexPlace, IndexRequest, build_index, collect_index,
        };
        let mut marks = Vec::new();
        let mut offset = 0;
        for (_, document, layout) in self.set_documents() {
            let host = crate::contents_commands::LayoutHost {
                document: &document,
                layout: &layout,
            };
            for mut mark in collect_index(&document, &host) {
                // Pages count through the set (the run's offset, +0xe2).
                if let Some((index, _)) = mark.page.as_mut() {
                    *index += offset;
                }
                marks.push(mark);
            }
            offset += layout.pages.len();
        }
        let options = IndexOptions {
            separators: self
                .document
                .contents()
                .master
                .as_ref()
                .is_some_and(|master| master.separators()),
            ranges: self.preferences.index_ranges,
        };
        let lines = build_index(&marks, options);
        if lines.is_empty() {
            self.set_master_message("No index entries in the files.");
            return cosmic::app::Task::none();
        }
        let count = lines.len();
        let title = format!("{} Index", self.document.title);
        let output = self.output_document("_INDEX.STY", &title);
        let request = amipro_editing::Command::PutIndex(Box::new(IndexRequest {
            place: IndexPlace::Output,
            lines,
            styles: Vec::new(),
        }));
        match amipro_editing::apply(&output, &request) {
            Ok(applied) => {
                let master = self.id;
                self.open_in_new_tab(applied.document, "Index of the master document");
                self.output_of = Some((master, OutputKind::Index));
                self.status = format!("Index of the set: {count} lines in a new tab");
            }
            Err(error) => self.set_master_message(error.to_string()),
        }
        cosmic::app::Task::none()
    }

    /// Ctrl+click on a row of a set's TOC (MS5): its file opens at the
    /// heading.
    pub(crate) fn follow_set_link(&mut self) -> Option<cosmic::app::Task<Message>> {
        if self.set_links.is_empty() {
            return None;
        }
        let caret = self.page_selection.focus;
        let (_, row, _) = amipro_editing::tables::cell_of_flow(&self.document, caret.flow)?;
        let (path, target) = self.set_links.get(row)?.clone();
        self.open_document_path(path);
        let layout = self.pages.layout();
        let page = crate::reference_commands::page_showing(
            layout,
            target.flow,
            target.paragraph,
            target.byte,
        )
        .unwrap_or(1);
        self.document.flow(target.flow)?;
        self.page_selection = crate::PageSelection::caret(target, page);
        self.sync_editor_to_page();
        self.status = "Went to the heading".into();
        Some(self.scroll_to_caret())
    }

    /// A saved output tab becomes the master's TOC or index file (MS5).
    fn record_master_output(&mut self, master: u64, kind: OutputKind, path: String) {
        let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.id == master)
        else {
            return;
        };
        let master_session = &mut self.sessions[index];
        let contents = master_session.document.contents().clone();
        let current = contents.master.clone().unwrap_or_default();
        let (toc, index_file) = match kind {
            OutputKind::Toc => (path, current.index_file.clone()),
            OutputKind::Index => (current.toc_file.clone(), path),
        };
        if (toc.as_str(), index_file.as_str())
            == (current.toc_file.as_str(), current.index_file.as_str())
        {
            return;
        }
        let command = set_outputs(
            &master_session.document,
            &toc,
            &index_file,
            current.separators(),
        );
        master_session.execute_document_command(&command);
        if let Some(panel) = master_session.master_panel.as_mut() {
            panel.toc_file = toc;
            panel.index_file = index_file;
        }
    }

    /// After every message (MS3): each open master's files found and read,
    /// and the numbers each continues from worked out in order; members
    /// open in tabs take them live and lay out again; a member whose
    /// master is gone returns to its own numbers.
    pub(crate) fn sync_master_sets(&mut self) -> Option<cosmic::app::Task<Message>> {
        let saved_outputs: Vec<(u64, OutputKind, String)> = self
            .sessions
            .iter()
            .filter_map(|session| {
                let (master, kind) = session.output_of?;
                Some((
                    master,
                    kind,
                    stored_path(&session.file_path()?.to_string_lossy()),
                ))
            })
            .collect();
        for (master, kind, path) in saved_outputs {
            self.record_master_output(master, kind, path);
        }
        let folders = self.style_folders();
        let mut tasks = Vec::new();
        let mut claimed: Vec<u64> = Vec::new();
        for index in 0..self.sessions.len() {
            if !is_master(&self.sessions[index].document) {
                self.sessions[index].master_set = None;
                continue;
            }
            let master_id = self.sessions[index].id;
            let files = self.sessions[index]
                .document
                .contents()
                .master
                .as_ref()
                .map(|master| master.files.clone())
                .unwrap_or_default();
            let folder = self.sessions[index]
                .file_path()
                .and_then(Path::parent)
                .map(Path::to_path_buf);
            let mut set = self.sessions[index].master_set.take().unwrap_or_default();
            set.refresh(&files, folder.as_deref());
            let mut carrier = Carrier::default();
            for position in 0..set.members.len() {
                let path = set.members[position].path.clone();
                let open = path.as_deref().and_then(|path| {
                    self.sessions.iter().position(|session| {
                        session.id != master_id
                            && !session.treated_single
                            && session.file_path() == Some(path)
                    })
                });
                if let Some(open) = open {
                    claimed.push(self.sessions[open].id);
                    let incoming = carrier.incoming().cloned();
                    let base = incoming.as_ref().map_or(0, |sequence| sequence.page_base);
                    self.sessions[open].apply_live_sequence(incoming);
                    let session = &self.sessions[open];
                    if set.members[position].state == MemberState::Unread {
                        set.members[position].state = MemberState::Ready;
                    }
                    carrier.step(
                        &mut set.members[position],
                        Some(MemberView {
                            document: &session.document,
                            layout: session.pages.layout(),
                            base,
                        }),
                    );
                    continue;
                }
                // A file changed on disk since it was read is read again.
                if let (Some(laid), Some(path)) = (&set.members[position].laid, &path) {
                    let modified = std::fs::metadata(path)
                        .and_then(|meta| meta.modified())
                        .ok();
                    if modified != laid.modified {
                        set.members[position].laid = None;
                        set.members[position].state = MemberState::Unread;
                    }
                }
                match set.members[position].laid.clone() {
                    Some(laid) => carrier.step(
                        &mut set.members[position],
                        Some(MemberView {
                            document: &laid.document,
                            layout: &laid.layout,
                            base: laid.base,
                        }),
                    ),
                    None => {
                        if let Some(path) = &path
                            && set.members[position].state == MemberState::Unread
                            && !set.reading.contains(path)
                        {
                            set.reading.push(path.clone());
                            tasks.push(read_task(
                                master_id,
                                set.members[position].stored.clone(),
                                path.clone(),
                                folders.clone(),
                            ));
                        }
                        carrier.step(&mut set.members[position], None);
                    }
                }
            }
            self.sessions[index].master_set = Some(set);
        }
        for session in &mut self.sessions {
            if session.live_sequence.is_some() && !claimed.contains(&session.id) {
                session.release_live_sequence();
            }
        }
        (!tasks.is_empty()).then(|| cosmic::app::Task::batch(tasks))
    }
}
