//! Help › Check for Updates… on the page surface (`UI_SPEC.md` §43, P6.2):
//! what each action of the Updates panel does. Transfers and the installer
//! run on a thread, never on the interface's; the result comes back as a
//! message. Nothing here runs anything but the backend's calls.

use std::path::PathBuf;
use std::sync::Arc;

use cosmic::iced::Task;

use crate::updates::{self, Backend};
use crate::updates_panel::{Status, UpdatesAction, UpdatesPanel};
use crate::{AmiProApp, Message};

/// Runs `work` on a thread and sends its result as an Updates action.
fn background<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    wrap: fn(T) -> UpdatesAction,
    interrupted: fn() -> T,
) -> Task<cosmic::Action<Message>> {
    Task::future(async move {
        let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
        std::thread::spawn(move || {
            let _ = done.send(work());
        });
        let result = wait.await.unwrap_or_else(|_| interrupted());
        cosmic::Action::from(Message::Updates(wrap(result)))
    })
}

impl AmiProApp {
    /// Help › Check for Updates…: the rail shows the panel and a check
    /// starts, unless a download or install is under way (the panel then
    /// shows it).
    pub(crate) fn open_updates(&mut self) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.updates_panel = Some(UpdatesPanel);
        self.open_rail();
        match self.updates.status {
            Status::Idle | Status::UpToDate(_) | Status::Newer(_) | Status::Failed(_) => {
                self.start_update_check(false)
            }
            _ => Task::none(),
        }
    }

    /// The quiet check at start-up: when Preferences asks for it and a day
    /// has passed since the last. The panel opens only if a newer version
    /// is found.
    pub(crate) fn startup_update_check(&mut self) -> Task<cosmic::Action<Message>> {
        if self.preferences.check_updates_on_start
            && updates::check_due(self.preferences.last_update_check, updates::unix_now())
        {
            self.start_update_check(true)
        } else {
            Task::none()
        }
    }

    fn start_update_check(&mut self, quiet: bool) -> Task<cosmic::Action<Message>> {
        self.updates.discard_download();
        self.updates.status = Status::Checking;
        self.updates.quiet = quiet;
        self.preferences.last_update_check = updates::unix_now();
        self.save_preferences();
        let backend: Arc<dyn Backend> = Arc::clone(&self.update_backend);
        background(
            move || updates::check(&*backend, &updates::installed_version()),
            UpdatesAction::Checked,
            || Err("The check was interrupted".into()),
        )
    }

    fn close_updates_panel(&mut self) {
        self.sessions[self.active_session].updates_panel = None;
        if self.rail_open() {
            self.toggle_rail();
        }
    }

    /// The Updates panel's actions.
    pub(crate) fn update_updates(
        &mut self,
        action: UpdatesAction,
    ) -> Task<cosmic::Action<Message>> {
        match action {
            UpdatesAction::Close => self.close_updates_panel(),
            UpdatesAction::CheckAgain => {
                if !self.updates.busy() {
                    return self.start_update_check(false);
                }
            }
            UpdatesAction::Checked(result) => {
                if self.updates.status != Status::Checking {
                    return Task::none();
                }
                let quiet = std::mem::take(&mut self.updates.quiet);
                self.updates.checked(result);
                if quiet {
                    if matches!(self.updates.status, Status::Newer(_)) {
                        let session = &mut self.sessions[self.active_session];
                        session.close_rail_panels();
                        session.updates_panel = Some(UpdatesPanel);
                        self.open_rail();
                    } else {
                        // A quiet check says nothing unless it found one.
                        self.updates.status = Status::Idle;
                    }
                }
            }
            UpdatesAction::Download => {
                let Status::Newer(offer) = self.updates.status.clone() else {
                    return Task::none();
                };
                self.updates.status = Status::Downloading(offer.clone());
                let backend: Arc<dyn Backend> = Arc::clone(&self.update_backend);
                return background(
                    move || {
                        let dir = updates::private_dir()
                            .map_err(|error| format!("No place for the download: {error}"))?;
                        updates::download_and_verify(&*backend, &offer, &dir).inspect_err(|_| {
                            let _ = std::fs::remove_dir_all(&dir);
                        })
                    },
                    UpdatesAction::Downloaded,
                    || Err("The download was interrupted".into()),
                );
            }
            UpdatesAction::Downloaded(result) => {
                let Status::Downloading(offer) = self.updates.status.clone() else {
                    return Task::none();
                };
                self.updates.status = match result {
                    Ok(path) => {
                        self.updates.dir = path.parent().map(PathBuf::from);
                        if self.update_backend.installer_available() {
                            Status::Ready { offer, path }
                        } else {
                            Status::CommandOnly { offer, path }
                        }
                    }
                    Err(error) => Status::Failed(error),
                };
            }
            UpdatesAction::Install => {
                let Status::Ready { offer, path } = self.updates.status.clone() else {
                    return Task::none();
                };
                self.updates.status = Status::Installing(offer);
                let backend: Arc<dyn Backend> = Arc::clone(&self.update_backend);
                return background(
                    move || backend.install(&path),
                    UpdatesAction::Installed,
                    || Err("The installation was interrupted".into()),
                );
            }
            UpdatesAction::Installed(result) => {
                let Status::Installing(offer) = self.updates.status.clone() else {
                    return Task::none();
                };
                self.updates.discard_download();
                self.updates.status = match result {
                    Ok(()) => Status::Installed(offer),
                    Err(error) => Status::Failed(error),
                };
            }
            UpdatesAction::NotNow => {
                if let Status::Ready { offer, .. } | Status::CommandOnly { offer, .. } =
                    self.updates.status.clone()
                {
                    self.updates.discard_download();
                    self.updates.status = Status::Newer(offer);
                }
            }
            UpdatesAction::Restart => {
                if !matches!(self.updates.status, Status::Installed(_)) {
                    return Task::none();
                }
                let backend: Arc<dyn Backend> = Arc::clone(&self.update_backend);
                return background(
                    move || backend.restart(),
                    UpdatesAction::Restarted,
                    || Err("The restart was interrupted".into()),
                );
            }
            UpdatesAction::Restarted(result) => match result {
                // The new version is up; this one closes the usual way,
                // asking about unsaved documents.
                Ok(()) => return self.begin_quit(),
                Err(error) => self.updates.status = Status::Failed(error),
            },
        }
        Task::none()
    }
}
