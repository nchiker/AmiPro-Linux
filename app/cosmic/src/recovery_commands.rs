//! File › Recover Documents… and the autosave behind it (`UI_SPEC.md` §44,
//! P6.3). Recovery copies are written to the app's own data folder, never
//! over the user's file; the encoding and the write run on a thread, and
//! the result comes back as a message.

use cosmic::iced::Task;

use crate::file_workflow::DocumentLocation;
use crate::recovery::{self, Kind};
use crate::recovery_panel::{RecoveryAction, RecoveryPanel};
use crate::{AmiProApp, DocumentSession, Message, new_document};

fn kind_of(location: &DocumentLocation) -> Kind {
    match location {
        DocumentLocation::Untitled => Kind::Untitled,
        DocumentLocation::LegacySource(_) => Kind::Legacy,
        DocumentLocation::Sam(_) => Kind::Sam,
        DocumentLocation::Native(_) => Kind::Native,
    }
}

impl AmiProApp {
    /// File › Recover Documents…: the rail shows the copies found now.
    pub(crate) fn open_recovery(&mut self) {
        self.recovery.rescan();
        self.recovery.notice = None;
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.recovery_panel = Some(RecoveryPanel);
        self.open_rail();
    }

    /// At start: offer what a program that ended badly left.
    pub(crate) fn recovery_at_start(&mut self) {
        if let Some(store) = &self.recovery.store {
            store.cleanup();
        }
        self.recovery.rescan();
        if !self.recovery.found.is_empty() || !self.recovery.problems.is_empty() {
            self.open_recovery();
        }
    }

    /// A clean exit leaves no copies of this instance's.
    pub(crate) fn recovery_finish(&mut self) {
        if let Some(store) = &self.recovery.store {
            store.finish();
        }
    }

    fn close_recovery_panel(&mut self) {
        self.sessions[self.active_session].recovery_panel = None;
        if self.rail_open() {
            self.toggle_rail();
        }
    }

    /// The autosave: every changed document with unsaved changes is written
    /// (a document with a password is not: the copy would hold it in the
    /// clear).
    fn recovery_tick(&mut self) -> Task<cosmic::Action<Message>> {
        if !self.preferences.keep_recovery {
            return Task::none();
        }
        let Some(store) = self.recovery.store.clone() else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        for session in &self.sessions {
            let id = session.id;
            if !session.dirty
                || session.password.is_some()
                || self.recovery.pending.contains(&id)
                || self.recovery.last.get(&id) == Some(&session.document)
            {
                continue;
            }
            self.recovery.pending.insert(id);
            let snapshot = Box::new(session.document.clone());
            let title = session.display_name();
            let path = session.location.path().map(std::path::Path::to_path_buf);
            let kind = kind_of(&session.location);
            let store = store.clone();
            tasks.push(Task::future(async move {
                let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let result = snapshot
                        .encode_native()
                        .map_err(|error| error.to_string())
                        .and_then(|text| store.write(id, &title, path.as_deref(), kind, &text));
                    let _ = done.send((snapshot, result));
                });
                let (snapshot, result) = wait
                    .await
                    .unwrap_or_else(|_| (Box::default(), Err("interrupted".into())));
                cosmic::Action::from(Message::Recovery(RecoveryAction::Written {
                    id,
                    snapshot,
                    result,
                }))
            }));
        }
        Task::batch(tasks)
    }

    /// After every message: copies of documents that were saved, closed, or
    /// changed back are removed.
    pub(crate) fn sync_recovery(&mut self) {
        for session in &mut self.sessions {
            if !session.dirty {
                session.recovered = false;
            }
        }
        let Some(store) = self.recovery.store.clone() else {
            return;
        };
        let sessions = &self.sessions;
        let live = |id: u64| {
            sessions
                .iter()
                .any(|session| session.id == id && session.dirty)
        };
        self.recovery.written.retain(|id| {
            let keep = live(*id);
            if !keep {
                store.remove(*id);
            }
            keep
        });
        let written = &self.recovery.written;
        self.recovery.last.retain(|id, _| written.contains(id));
        self.recovery.inherited.retain(|id, entry| {
            let keep = live(*id);
            if !keep {
                store.remove_entry(entry);
            }
            keep
        });
    }

    /// The Recover documents panel's actions, and the autosave's.
    pub(crate) fn update_recovery(
        &mut self,
        action: RecoveryAction,
    ) -> Task<cosmic::Action<Message>> {
        match action {
            RecoveryAction::Close => self.close_recovery_panel(),
            RecoveryAction::Tick => return self.recovery_tick(),
            RecoveryAction::Written {
                id,
                snapshot,
                result,
            } => {
                self.recovery.pending.remove(&id);
                let live = self
                    .sessions
                    .iter()
                    .any(|session| session.id == id && session.dirty);
                match result {
                    Ok(_) if live => {
                        self.recovery.last.insert(id, *snapshot);
                        self.recovery.written.insert(id);
                        if let (Some(entry), Some(store)) =
                            (self.recovery.inherited.remove(&id), &self.recovery.store)
                        {
                            store.remove_entry(&entry);
                        }
                    }
                    // Saved or closed while the copy was written.
                    Ok(_) => {
                        if let Some(store) = &self.recovery.store {
                            store.remove(id);
                        }
                    }
                    Err(error) => {
                        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == id) {
                            session.status =
                                format!("The recovery copy could not be written: {error}");
                        }
                    }
                }
            }
            RecoveryAction::Discard(key) => {
                if let Some(position) = self.recovery.found.iter().position(|e| e.key() == key) {
                    let entry = self.recovery.found.remove(position);
                    if let Some(store) = &self.recovery.store {
                        store.remove_entry(&entry);
                    }
                    self.recovery.notice = Some(format!(
                        "Discarded {}.",
                        crate::recovery_panel::entry_title(&entry)
                    ));
                }
            }
            RecoveryAction::DiscardAll => {
                let entries = std::mem::take(&mut self.recovery.found);
                if let Some(store) = &self.recovery.store {
                    for entry in &entries {
                        store.remove_entry(entry);
                    }
                }
                self.recovery.notice = Some("Discarded all recovery copies.".into());
            }
            RecoveryAction::Recover(key) => self.recover_entry(&key),
        }
        Task::none()
    }

    /// Opens a copy as an unsaved document that remembers where it came
    /// from. Save never writes over that file: it goes to Save As.
    fn recover_entry(&mut self, key: &str) {
        let Some(position) = self.recovery.found.iter().position(|e| e.key() == key) else {
            return;
        };
        let entry = self.recovery.found[position].clone();
        let title = crate::recovery_panel::entry_title(&entry);
        let document = match recovery::read(&entry) {
            Ok(document) => document,
            Err(error) => {
                self.recovery.notice = Some(format!(
                    "{title} could not be read ({error}). The copy is kept."
                ));
                return;
            }
        };
        let location = entry
            .meta
            .path
            .clone()
            .map_or(DocumentLocation::Untitled, DocumentLocation::LegacySource);
        let mut session = DocumentSession::new(
            self.next_session_id,
            document,
            location,
            format!("Recovered {title}; Save As keeps it"),
        );
        // Unsaved by definition: no undo returns to a "clean" state.
        session.clean_document = new_document();
        session.dirty = true;
        session.recovered = true;
        let id = session.id;
        self.next_session_id += 1;
        self.sessions.push(session);
        self.active_session = self.sessions.len() - 1;
        self.recovery.found.remove(position);
        self.recovery.inherited.insert(id, entry);
        self.recovery.notice = Some(format!("Recovered {title}."));
        let session = &mut self.sessions[self.active_session];
        if !self.recovery.found.is_empty() || !self.recovery.problems.is_empty() {
            session.close_rail_panels();
            session.recovery_panel = Some(RecoveryPanel);
        } else {
            session.recovery_panel = None;
        }
    }

    /// The autosave's timer, while recovery is on.
    pub(crate) fn recovery_interval(&self) -> Option<std::time::Duration> {
        (self.recovery.store.is_some() && self.preferences.keep_recovery).then(|| {
            std::time::Duration::from_secs(60 * u64::from(self.preferences.recovery_minutes.max(1)))
        })
    }
}
