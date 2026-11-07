//! Help › Check for Updates… (P6.2, `UI_SPEC.md` §43): the panel through the
//! window's message path with a scripted backend. Nothing reaches the
//! network, the installer, or a second window.

use std::sync::Arc;

use crate::updates::fake::{Fake, serve_release};
use crate::updates::{self, DOWNLOAD_PREFIX};
use crate::updates_panel::{Status, UpdatesAction};

use super::*;

fn scenario(name: &'static str, fake: Fake) -> (Scenario, Arc<Fake>) {
    let fake = Arc::new(fake);
    let mut s = Scenario::new_document(name);
    s.app_mut().update_backend = fake.clone();
    (s, fake)
}

fn open_panel(s: &mut Scenario) {
    s.act("Help › Check for Updates…", |rt| {
        rt.send(Message::Command(CommandId::CheckForUpdates));
    });
    assert_eq!(s.app().rail_panel(), RailPanel::Updates);
    assert!(s.app().rail_open());
}

fn act(s: &mut Scenario, what: &str, action: UpdatesAction) {
    s.act(what, move |rt| {
        rt.send(Message::Updates(action));
    });
}

fn status(s: &Scenario) -> Status {
    s.app().updates.status.clone()
}

fn failure(s: &Scenario) -> String {
    match status(s) {
        Status::Failed(error) => error,
        other => s.fail("status", &format!("expected a failure, got {other:?}")),
    }
}

#[test]
fn a_newer_version_is_downloaded_verified_installed_and_the_program_restarts() {
    let (mut s, fake) = scenario(
        "update-install",
        Fake {
            installer: true,
            ..Fake::default()
        },
    );
    serve_release(&fake, "v9.0.0", b"deb bytes", None);
    open_panel(&mut s);
    let Status::Newer(offer) = status(&s) else {
        s.fail(
            "check",
            &format!("expected a newer version: {:?}", status(&s)),
        )
    };
    assert_eq!(offer.version, "9.0.0");
    assert_eq!(offer.notes, "Line one\nLine two");
    assert!(s.app().preferences.last_update_check > 0);

    act(&mut s, "Download and Install", UpdatesAction::Download);
    let Status::Ready { path, .. } = status(&s) else {
        s.fail("download", &format!("expected Ready: {:?}", status(&s)))
    };
    assert_eq!(std::fs::read(&path).unwrap(), b"deb bytes");
    assert!(
        fake.installed.lock().unwrap().is_empty(),
        "nothing installs before the confirmation"
    );
    assert!(updates::install_command(&path).starts_with("pkexec apt-get install -y /"));

    act(&mut s, "Install", UpdatesAction::Install);
    assert!(matches!(status(&s), Status::Installed(_)));
    assert_eq!(*fake.installed.lock().unwrap(), vec![path.clone()]);
    assert!(!path.exists(), "the download is removed after the install");

    act(&mut s, "Restart", UpdatesAction::Restart);
    assert_eq!(*fake.restarts.lock().unwrap(), 1);
    assert!(s.app().quit_requested, "this window closes the usual way");
}

#[test]
fn up_to_date_says_so_and_offers_nothing_to_download() {
    let (mut s, fake) = scenario("update-current", Fake::default());
    serve_release(&fake, "v0.1.0", b"x", None);
    open_panel(&mut s);
    assert_eq!(status(&s), Status::UpToDate("0.1.0".into()));
    act(&mut s, "Download (not offered)", UpdatesAction::Download);
    act(&mut s, "Install (not offered)", UpdatesAction::Install);
    assert_eq!(status(&s), Status::UpToDate("0.1.0".into()));
    let before = fake.requests.lock().unwrap().len();
    act(&mut s, "Check again", UpdatesAction::CheckAgain);
    assert!(fake.requests.lock().unwrap().len() > before);
    // Esc closes the panel and the rail goes back to the Styles list.
    s.act("Esc", |rt| {
        rt.send(Message::Escape);
    });
    assert_eq!(s.app().rail_panel(), RailPanel::Styles);
}

#[test]
fn a_bad_checksum_is_refused_and_nothing_is_installed() {
    let (mut s, fake) = scenario(
        "update-badsum",
        Fake {
            installer: true,
            ..Fake::default()
        },
    );
    serve_release(&fake, "v9.0.0", b"tampered", Some(&"0".repeat(64)));
    open_panel(&mut s);
    act(&mut s, "Download and Install", UpdatesAction::Download);
    assert!(failure(&s).contains("checksum does not match"));
    assert!(s.app().updates.dir.is_none());
    act(&mut s, "Install (not offered)", UpdatesAction::Install);
    assert!(fake.installed.lock().unwrap().is_empty());
}

#[test]
fn a_failed_download_and_a_failed_check_are_reported_and_can_be_retried() {
    let (mut s, fake) = scenario("update-failures", Fake::default());
    open_panel(&mut s);
    assert_eq!(failure(&s), "not found");
    serve_release(&fake, "v9.0.0", b"deb", None);
    fake.fail(
        &format!("{DOWNLOAD_PREFIX}v9.0.0/amipro-linux_9.0.0_amd64.deb"),
        "The connection failed",
    );
    act(&mut s, "Check again", UpdatesAction::CheckAgain);
    assert!(matches!(status(&s), Status::Newer(_)));
    act(&mut s, "Download and Install", UpdatesAction::Download);
    assert_eq!(failure(&s), "The connection failed");
    serve_release(&fake, "v9.0.0", b"deb", None);
    act(&mut s, "Check again", UpdatesAction::CheckAgain);
    act(&mut s, "Download and Install", UpdatesAction::Download);
    assert!(matches!(
        status(&s),
        Status::CommandOnly { .. } | Status::Ready { .. }
    ));
    act(&mut s, "Not now", UpdatesAction::NotNow);
    assert!(matches!(status(&s), Status::Newer(_)));
}

#[test]
fn without_pkexec_the_exact_command_is_shown_and_nothing_runs() {
    let (mut s, fake) = scenario("update-nopkexec", Fake::default());
    serve_release(&fake, "v9.0.0", b"deb bytes", None);
    open_panel(&mut s);
    act(&mut s, "Download and Install", UpdatesAction::Download);
    let Status::CommandOnly { path, .. } = status(&s) else {
        s.fail(
            "download",
            &format!("expected the command: {:?}", status(&s)),
        )
    };
    assert_eq!(std::fs::read(&path).unwrap(), b"deb bytes");
    act(&mut s, "Install (not offered)", UpdatesAction::Install);
    assert!(matches!(status(&s), Status::CommandOnly { .. }));
    assert!(fake.installed.lock().unwrap().is_empty());
    assert_eq!(
        updates::install_command(&path),
        format!("pkexec apt-get install -y {}", path.display())
    );
    act(&mut s, "Not now", UpdatesAction::NotNow);
    assert!(!path.exists());
}

#[test]
fn an_install_the_user_does_not_authorise_fails_visibly() {
    let (mut s, fake) = scenario(
        "update-declined",
        Fake {
            installer: true,
            ..Fake::default()
        },
    );
    *fake.install_error.lock().unwrap() = Some("The installation was not authorised".into());
    serve_release(&fake, "v9.0.0", b"deb bytes", None);
    open_panel(&mut s);
    act(&mut s, "Download and Install", UpdatesAction::Download);
    act(&mut s, "Install", UpdatesAction::Install);
    assert_eq!(failure(&s), "The installation was not authorised");
    assert!(s.app().updates.dir.is_none(), "the download is removed");
}

#[test]
fn the_start_up_check_is_off_by_default_quiet_and_once_a_day() {
    let (mut s, fake) = scenario("update-startup", Fake::default());
    assert!(!s.app().preferences.check_updates_on_start);
    drop(s.app_mut().startup_update_check());
    assert_eq!(status(&s), Status::Idle, "off by default");

    // The Preferences switch.
    s.command_act(CommandId::Preferences);
    s.act("turn the check on", |rt| {
        rt.send(Message::Preferences(
            crate::preferences_dialog::PreferencesAction::CheckUpdates(true),
        ));
    });
    assert!(s.app().preferences.check_updates_on_start);
    s.act("close Preferences", |rt| {
        rt.send(Message::Preferences(
            crate::preferences_dialog::PreferencesAction::Close,
        ));
    });

    // Not due: checked a moment ago.
    s.app_mut().preferences.last_update_check = updates::unix_now();
    drop(s.app_mut().startup_update_check());
    assert_eq!(status(&s), Status::Idle);
    // Due: a quiet check starts.
    s.app_mut().preferences.last_update_check = 1;
    drop(s.app_mut().startup_update_check());
    assert_eq!(status(&s), Status::Checking);
    assert!(s.app().updates.quiet);

    // Up to date: no panel, no word.
    act(
        &mut s,
        "answer: current",
        UpdatesAction::Checked(Ok(updates::Outcome::UpToDate {
            latest: "0.1.0".into(),
        })),
    );
    assert_eq!(status(&s), Status::Idle);
    assert_eq!(s.app().rail_panel(), RailPanel::Styles);

    // A failure is silent too; a newer version opens the panel.
    serve_release(&fake, "v9.0.0", b"deb", None);
    let outcome = updates::check(&*s.app().update_backend, &updates::installed_version());
    s.app_mut().updates.status = Status::Checking;
    s.app_mut().updates.quiet = true;
    act(
        &mut s,
        "answer: failed",
        UpdatesAction::Checked(Err("offline".into())),
    );
    assert_eq!(status(&s), Status::Idle);
    assert_eq!(s.app().rail_panel(), RailPanel::Styles);
    s.app_mut().updates.status = Status::Checking;
    s.app_mut().updates.quiet = true;
    act(&mut s, "answer: newer", UpdatesAction::Checked(outcome));
    assert!(matches!(status(&s), Status::Newer(_)));
    assert_eq!(s.app().rail_panel(), RailPanel::Updates);
}

#[test]
fn every_state_of_the_panel_builds_in_the_window() {
    let (mut s, fake) = scenario("update-views", Fake::default());
    serve_release(&fake, "v9.0.0", b"deb", None);
    open_panel(&mut s);
    let Status::Newer(offer) = status(&s) else {
        s.fail("check", "expected a newer version")
    };
    let path =
        std::path::PathBuf::from("/run/user/1000/amipro-update-1/amipro-linux_9.0.0_amd64.deb");
    for state in [
        Status::Idle,
        Status::Checking,
        Status::UpToDate("1.0.0".into()),
        Status::Newer(offer.clone()),
        Status::Downloading(offer.clone()),
        Status::Ready {
            offer: offer.clone(),
            path: path.clone(),
        },
        Status::Installing(offer.clone()),
        Status::Installed(offer.clone()),
        Status::CommandOnly { offer, path },
        Status::Failed("The connection failed".into()),
    ] {
        s.app_mut().updates.status = state.clone();
        s.act(&format!("{state:?}"), |_| {});
    }
}
