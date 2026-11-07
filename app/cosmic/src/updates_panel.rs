//! Help › Check for Updates… in the rail (`docs/Design/UI_SPEC.md` §43,
//! P6.2): the installed version, what the last check found, the release
//! notes' first lines, and the buttons that follow from the state. The
//! logic is in `updates.rs`; this file holds the state the app keeps, the
//! actions, and the view.

use std::path::PathBuf;

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::updates::{self, Offer, Outcome};

/// The panel's width in the rail.
pub const WIDTH: f32 = 300.0;

/// What the panel says about how updates are installed.
pub const NOTE: &str = "A newer version is downloaded, checked against the release's SHA-256 list, and only then installed with your system's installer; it asks for your password once.";

/// The rail panel's own state in a session: open or not.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UpdatesPanel;

/// Where an update stands. The app keeps one, whichever tab shows it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Status {
    #[default]
    Idle,
    Checking,
    UpToDate(String),
    Newer(Offer),
    Downloading(Offer),
    /// Downloaded and verified; the user confirms the install.
    Ready {
        offer: Offer,
        path: PathBuf,
    },
    Installing(Offer),
    Installed(Offer),
    /// Downloaded and verified, but `pkexec` or `apt-get` is missing: the
    /// panel shows the command to run.
    CommandOnly {
        offer: Offer,
        path: PathBuf,
    },
    Failed(String),
}

/// The app's update state: the status, whether the check in flight was the
/// quiet start-up one, and the directory holding a download.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UpdatesState {
    pub status: Status,
    pub quiet: bool,
    pub dir: Option<PathBuf>,
}

impl UpdatesState {
    /// Removes the downloaded package and its directory.
    pub fn discard_download(&mut self) {
        if let Some(dir) = self.dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// Whether a transfer or the installer is working.
    pub fn busy(&self) -> bool {
        matches!(
            self.status,
            Status::Checking | Status::Downloading(_) | Status::Installing(_)
        )
    }

    /// What a finished check changes.
    pub fn checked(&mut self, result: Result<Outcome, String>) {
        self.status = match result {
            Ok(Outcome::UpToDate { latest }) => Status::UpToDate(latest),
            Ok(Outcome::Newer(offer)) => Status::Newer(offer),
            Err(error) => Status::Failed(error),
        };
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdatesAction {
    Close,
    CheckAgain,
    /// "Download and Install": download, verify, then ask.
    Download,
    /// The confirmation: run the installer.
    Install,
    /// Decline the install; the download is deleted.
    NotNow,
    Restart,
    Checked(Result<Outcome, String>),
    /// The directory made for the download, and how the download went.
    Downloaded(Result<PathBuf, String>),
    Installed(Result<(), String>),
    Restarted(Result<(), String>),
}

fn send(action: UpdatesAction) -> Message {
    Message::Updates(action)
}

fn button<'a>(label: &'static str, action: UpdatesAction) -> Element<'a, Message> {
    widget::button::standard(label)
        .on_press(send(action))
        .into()
}

fn disabled_button<'a>(label: &'static str) -> Element<'a, Message> {
    widget::button::standard(label).into()
}

/// The state line and the buttons for a status.
fn body<'a>(status: &'a Status) -> Vec<Element<'a, Message>> {
    let mut out: Vec<Element<'a, Message>> = Vec::new();
    let notes = |offer: &Offer, out: &mut Vec<Element<'a, Message>>| {
        out.push(widget::text::heading(format!("What's new in {}", offer.version)).into());
        out.push(if offer.notes.is_empty() {
            widget::text::caption("The release has no notes.").into()
        } else {
            widget::text::caption(offer.notes.clone()).into()
        });
    };
    match status {
        Status::Idle => {
            out.push(widget::text::body("Not checked yet.").into());
            out.push(button("Check again", UpdatesAction::CheckAgain));
        }
        Status::Checking => {
            out.push(widget::text::body("Checking for updates…").into());
            out.push(disabled_button("Check again"));
        }
        Status::UpToDate(latest) => {
            out.push(widget::text::body(format!("You have the latest version ({latest}).")).into());
            out.push(button("Check again", UpdatesAction::CheckAgain));
        }
        Status::Newer(offer) => {
            out.push(widget::text::body(format!("Version {} is available.", offer.version)).into());
            notes(offer, &mut out);
            out.push(
                widget::row::with_children(vec![
                    widget::button::suggested("Download and Install")
                        .on_press(send(UpdatesAction::Download))
                        .into(),
                    button("Check again", UpdatesAction::CheckAgain),
                ])
                .spacing(8)
                .into(),
            );
        }
        Status::Downloading(offer) => {
            out.push(
                widget::text::body(format!(
                    "Downloading version {} ({})…",
                    offer.version,
                    size_text(offer.deb.size)
                ))
                .into(),
            );
            out.push(widget::progress_bar::indeterminate_linear().into());
            out.push(
                widget::text::caption("The download is checked before anything is installed.")
                    .into(),
            );
        }
        Status::Ready { offer, path } => {
            out.push(
                widget::text::body(format!(
                    "Version {} is downloaded and its checksum matches.",
                    offer.version
                ))
                .into(),
            );
            out.push(widget::text::caption("Installing runs:").into());
            out.push(widget::text::monotext(updates::install_command(path)).into());
            out.push(
                widget::row::with_children(vec![
                    widget::button::suggested("Install")
                        .on_press(send(UpdatesAction::Install))
                        .into(),
                    button("Not now", UpdatesAction::NotNow),
                ])
                .spacing(8)
                .into(),
            );
        }
        Status::Installing(offer) => {
            out.push(
                widget::text::body(format!(
                    "Installing version {}… (enter your password when asked)",
                    offer.version
                ))
                .into(),
            );
            out.push(widget::progress_bar::indeterminate_linear().into());
        }
        Status::Installed(offer) => {
            out.push(widget::text::body(format!("Version {} is installed.", offer.version)).into());
            out.push(
                widget::text::caption(
                    "Restart to use it: the new version opens, and this window then closes (asking about unsaved documents).",
                )
                .into(),
            );
            out.push(
                widget::button::suggested("Restart")
                    .on_press(send(UpdatesAction::Restart))
                    .into(),
            );
        }
        Status::CommandOnly { offer, path } => {
            out.push(
                widget::text::body(format!(
                    "Version {} is downloaded and its checksum matches, but this system has no pkexec or apt-get to install it for you.",
                    offer.version
                ))
                .into(),
            );
            out.push(widget::text::caption("Run this command in a terminal:").into());
            out.push(widget::text::monotext(updates::install_command(path)).into());
            out.push(button("Check again", UpdatesAction::CheckAgain));
        }
        Status::Failed(error) => {
            out.push(widget::text::body("The update check did not finish.").into());
            out.push(widget::text::caption(error.clone()).into());
            out.push(button("Check again", UpdatesAction::CheckAgain));
        }
    }
    out
}

/// "12.3 MB", or nothing when the size is not known.
pub fn size_text(bytes: u64) -> String {
    if bytes == 0 {
        "size unknown".into()
    } else {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    }
}

/// The panel.
pub fn panel<'a>(state: &'a UpdatesState, installed: &str) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Updates").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close the Updates panel")
            .on_press(send(UpdatesAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut children: Vec<Element<'a, Message>> = vec![
        header.into(),
        widget::text::body(format!("Installed version: {installed}")).into(),
    ];
    children.extend(body(&state.status));
    children.push(widget::text::caption(NOTE).into());
    widget::scrollable(
        widget::column::with_children(children)
            .spacing(8)
            .padding([8, 8])
            .width(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}
