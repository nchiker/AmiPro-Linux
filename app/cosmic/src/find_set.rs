//! The find bar's In All Files of the Master (MS11, `UI_SPEC.md` §37):
//! the matches of every file of the master's set, counted file by file;
//! Find Next and Previous going on into the next file (opened in its tab)
//! where this one has no more; Replace All in each file that has matches,
//! one undo step in each.
//!
//! Ami Pro's Find has no such option (`MASTER_NOTES.md` §1: only the TOC,
//! the index, and printing walk the files); this is the run's addition.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use amipro_document::Document;
use amipro_editing::find::{self, FindOptions};

use crate::find_bar::{FindBar, SetFileMatches};
use crate::{AmiProApp, Message, page_editing::PageSelection};

impl AmiProApp {
    /// The tab holding the master whose set the active tab is in: the
    /// active tab itself when it is a master with files, else an open
    /// master listing the active tab's file (not Treated as Single).
    pub(crate) fn find_set_master(&self) -> Option<usize> {
        let active = &self.sessions[self.active_session];
        if active
            .master_set
            .as_ref()
            .is_some_and(|set| !set.members.is_empty())
            && amipro_editing::master::is_master(&active.document)
        {
            return Some(self.active_session);
        }
        if active.treated_single {
            return None;
        }
        let path = active.file_path()?;
        self.sessions.iter().position(|session| {
            amipro_editing::master::is_master(&session.document)
                && session.master_set.as_ref().is_some_and(|set| {
                    set.members
                        .iter()
                        .any(|member| member.path.as_deref() == Some(path))
                })
        })
    }

    /// Each file of the set that can be searched, in order, with the
    /// document as it is now (an open tab's, else the file as read) and
    /// what identifies that state.
    fn set_search_documents(&self, master: usize) -> Vec<(PathBuf, u64, &Document)> {
        let Some(set) = self.sessions[master].master_set.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for member in &set.members {
            let Some(path) = member.path.clone() else {
                continue;
            };
            if let Some(session) = self.sessions.iter().find(|session| {
                session.file_path() == Some(path.as_path()) && !session.treated_single
            }) {
                // Tabs and read files apart: the generation's top bit.
                let key = session.rich_layout_generation & !(1 << 63);
                out.push((path, key, &session.document));
            } else if let Some(laid) = member.laid.as_ref() {
                let key = (Arc::as_ptr(laid) as u64) | (1 << 63);
                out.push((path, key, &laid.document));
            }
        }
        out
    }

    /// Counts the matches in every file of the set again when the text,
    /// the options, or a file changed.
    pub(crate) fn count_set_matches(&mut self) {
        let options = self.preferences.find_options.clone();
        let Some(bar) = self.sessions[self.active_session].find.as_ref() else {
            return;
        };
        if !bar.all_files {
            return;
        }
        let text = bar.find.clone();
        let Some(master) = self.find_set_master() else {
            if let Some(bar) = self.sessions[self.active_session].find.as_mut() {
                bar.files.clear();
                bar.files_key = None;
            }
            return;
        };
        let documents = self.set_search_documents(master);
        let key = (
            text.clone(),
            options.clone(),
            documents
                .iter()
                .map(|(path, key, _)| (path.clone(), *key))
                .collect::<Vec<_>>(),
        );
        if bar.files_key.as_ref() == Some(&key) {
            return;
        }
        let files: Vec<SetFileMatches> = documents
            .iter()
            .map(|(path, _, document)| SetFileMatches {
                path: path.clone(),
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                count: count_matches(document, &text, &options),
            })
            .collect();
        if let Some(bar) = self.sessions[self.active_session].find.as_mut() {
            bar.files = files;
            bar.files_key = Some(key);
        }
    }

    /// Find Next or Previous past this file's last (or first) match: the
    /// next file of the set with matches, opened in its tab, at its first
    /// (or last) match; round the set to this file again.
    pub(crate) fn find_in_next_file(&mut self, forward: bool) -> cosmic::app::Task<Message> {
        self.count_set_matches();
        let Some(bar) = self.sessions[self.active_session].find.clone() else {
            return cosmic::app::Task::none();
        };
        let here = self.sessions[self.active_session]
            .file_path()
            .map(Path::to_path_buf);
        let count = bar.files.len();
        let at = here
            .as_ref()
            .and_then(|here| bar.files.iter().position(|file| &file.path == here));
        // From the master itself the search starts with the first file (or
        // the last, going back).
        let order: Vec<usize> = (1..=count)
            .map(|step| match (at, forward) {
                (Some(at), true) => (at + step) % count,
                (Some(at), false) => (at + count - step % count) % count,
                (None, true) => step - 1,
                (None, false) => count - step,
            })
            .collect();
        let Some(target) = order.into_iter().find(|index| bar.files[*index].count > 0) else {
            self.status = format!("“{}” is not found in the master's files", bar.find);
            return cosmic::app::Task::none();
        };
        let path = bar.files[target].path.clone();
        if Some(&path) != here.as_ref() {
            self.carry_bar_to(&path, &bar);
        }
        self.sync_find_bar();
        let Some(bar) = self.sessions[self.active_session].find.as_ref() else {
            return cosmic::app::Task::none();
        };
        if bar.matches.is_empty() {
            return cosmic::app::Task::none();
        }
        let index = if forward { 0 } else { bar.matches.len() - 1 };
        self.select_match(index)
    }

    /// Opens `path` in its tab (or goes to it) with the bar's texts and
    /// options; the bar in the tab left closes.
    fn carry_bar_to(&mut self, path: &Path, bar: &FindBar) {
        if let Some(left) = self.sessions[self.active_session].find.take() {
            self.last_find = Some(left);
        }
        self.open_document_path(path.to_path_buf());
        let session = &mut self.sessions[self.active_session];
        let caret = PageSelection::caret(session.page_selection.ordered().0, 1);
        let mut carried = FindBar::open(&session.document, caret, bar.replacing, Some(bar));
        carried.find.clone_from(&bar.find);
        carried.within = None;
        carried.all_files = true;
        session.find = Some(carried);
    }

    /// Options ▾ › Files of the Master: goes to the file and its first
    /// match.
    pub(crate) fn find_go_to_file(&mut self, index: usize) -> cosmic::app::Task<Message> {
        let Some(bar) = self.sessions[self.active_session].find.clone() else {
            return cosmic::app::Task::none();
        };
        let Some(file) = bar.files.get(index).cloned() else {
            return cosmic::app::Task::none();
        };
        let here = self.sessions[self.active_session]
            .file_path()
            .map(Path::to_path_buf);
        if here.as_ref() != Some(&file.path) {
            self.carry_bar_to(&file.path, &bar);
        }
        self.sync_find_bar();
        let has = self.sessions[self.active_session]
            .find
            .as_ref()
            .is_some_and(|bar| !bar.matches.is_empty());
        if has {
            self.select_match(0)
        } else {
            cosmic::app::Task::none()
        }
    }

    /// Replace All in every file of the set that has matches (MS11): each
    /// file opened in its tab and changed in one undo step there; the tab
    /// the search was made in stays active.
    pub(crate) fn find_replace_all_files(&mut self) {
        self.count_set_matches();
        let Some(bar) = self.sessions[self.active_session].find.clone() else {
            return;
        };
        let origin = self.sessions[self.active_session].id;
        let paths: Vec<PathBuf> = bar
            .files
            .iter()
            .filter(|file| file.count > 0)
            .map(|file| file.path.clone())
            .collect();
        if paths.is_empty() {
            self.status = format!("“{}” is not found in the master's files", bar.find);
            return;
        }
        let options = self.preferences.find_options.clone();
        let kept = self.sessions[self.active_session].find.take();
        let (mut replaced, mut files) = (0, 0);
        for path in paths {
            self.open_document_path(path.clone());
            let session = &mut self.sessions[self.active_session];
            if session.file_path() != Some(path.as_path()) {
                continue;
            }
            let caret = PageSelection::caret(session.page_selection.ordered().0, 1);
            let mut here = FindBar::open(&session.document, caret, bar.replacing, Some(&bar));
            here.find.clone_from(&bar.find);
            here.within = None;
            let generation = session.rich_layout_generation;
            here.refresh(&session.document, generation, &options);
            if let Some(command) =
                find::replace_all(&session.document, &here.matches, &bar.replace, &options)
                && session.execute_page_command(&command)
            {
                replaced += here.matches.len();
                files += 1;
            }
        }
        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == origin)
        {
            session.find = kept;
        }
        if let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.id == origin)
        {
            self.active_session = index;
        }
        if let Some(bar) = self.sessions[self.active_session].find.as_mut() {
            bar.current = None;
            bar.within = None;
        }
        find_bar_remember(self, &bar.find);
        self.sync_find_bar();
        self.status = match (replaced, files) {
            (1, _) => "Replaced 1 match in 1 file".into(),
            (count, 1) => format!("Replaced {count} matches in 1 file"),
            (count, files) => format!("Replaced {count} matches in {files} files"),
        };
    }
}

fn find_bar_remember(app: &mut AmiProApp, text: &str) {
    crate::find_bar::remember(&mut app.preferences.recent_searches, text);
}

/// How many matches `text` has in `document`, in every stream the options
/// search, from its main text.
fn count_matches(document: &Document, text: &str, options: &FindOptions) -> usize {
    if text.is_empty() {
        return 0;
    }
    let streams = find::search_streams(document, document.main_flow_id(), options);
    find::find_all(document, text, &streams, options, None).len()
}
