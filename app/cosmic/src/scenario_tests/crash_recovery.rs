//! Crash recovery (P6.3, `UI_SPEC.md` §44): autosave to hidden copies, a
//! crash simulated by dropping the window without a clean exit, a second
//! window that lists and recovers them. The clock and the processes are
//! fakes; the copies are real files under the scenario's scratch folder,
//! never the user's data folder.

use std::path::PathBuf;
use std::sync::Arc;

use crate::file_workflow::DocumentLocation;
use crate::recovery::fake::Fake;
use crate::recovery::{Store, read};
use crate::recovery_panel::{RecoveryAction, RecoveryState};

use super::*;

/// Delivers `message` but never runs the task it returns (one that would
/// open a desktop dialog); true when it returned a task.
fn send_unrun(rt: &mut Runtime, message: Message) -> bool {
    let task = cosmic::Application::update(&mut rt.app, message);
    let asked = cosmic::iced::runtime::task::into_stream(task).is_some();
    rt.redraw();
    asked
}

fn tick(s: &mut Scenario) {
    s.act("autosave tick", |rt| {
        rt.send(Message::Recovery(RecoveryAction::Tick));
    });
}

fn copy_of(s: &Scenario, id: u64) -> PathBuf {
    s.app()
        .recovery
        .store
        .as_ref()
        .unwrap()
        .own_dir()
        .join(format!("{id}.amipro-recovery"))
}

fn session_id(s: &Scenario) -> u64 {
    s.app().id
}

fn start(s: &mut Scenario) {
    s.act("start-up offers the copies", |rt| {
        rt.app.recovery_at_start();
    });
}

fn key_of(s: &Scenario, title: &str) -> String {
    s.app()
        .recovery
        .found
        .iter()
        .find(|entry| crate::recovery_panel::entry_title(entry) == title)
        .unwrap_or_else(|| s.fail("find", &format!("no copy called {title}")))
        .key()
}

fn act_recovery(s: &mut Scenario, what: &str, action: RecoveryAction) {
    s.act(what, move |rt| {
        rt.send(Message::Recovery(action));
    });
}

#[test]
fn a_crash_leaves_copies_that_the_next_start_recovers_and_the_original_is_never_touched() {
    let mut s = Scenario::new_document("recovery-crash");
    let folder = s.clean_folder();
    let root = folder.join("recovery");
    let fake_a = Fake::new(100, 1_000_000);
    let fake_b = fake_a.sibling(200);
    let clock = fake_a.now.clone();
    let running = fake_a.running.clone();
    s.app_mut().recovery = RecoveryState::with_store(Store::new(root.clone(), Arc::new(fake_a)));

    // A saved Ami Pro document, then changed and not saved.
    s.type_text("Saved text of the letter");
    let sam = s.save_sam("letter.sam");
    let original = std::fs::read(&sam).unwrap();
    tick(&mut s);
    assert!(
        !copy_of(&s, session_id(&s)).exists(),
        "a saved document has no recovery copy"
    );
    s.type_text(" and an unsaved tail");
    let first_id = session_id(&s);
    tick(&mut s);
    assert!(copy_of(&s, first_id).exists());
    // Nothing changed: the next tick writes nothing.
    let meta_path = copy_of(&s, first_id).with_extension("meta");
    let before = std::fs::read_to_string(&meta_path).unwrap();
    *clock.lock().unwrap() += 130;
    tick(&mut s);
    assert_eq!(std::fs::read_to_string(&meta_path).unwrap(), before);
    // A change is written at the next tick.
    s.type_text("!");
    *clock.lock().unwrap() += 130;
    tick(&mut s);
    assert_ne!(std::fs::read_to_string(&meta_path).unwrap(), before);

    // A second document that was never saved.
    s.command_act(CommandId::NewDocument);
    s.type_text("A note that has no file");
    let second_id = session_id(&s);
    tick(&mut s);
    assert!(copy_of(&s, second_id).exists());
    assert!(copy_of(&s, first_id).exists());
    assert_eq!(
        std::fs::read(&sam).unwrap(),
        original,
        "the user's file is untouched"
    );

    // kill -9: the window is dropped without a clean exit and its process
    // is gone.
    let crashed_dir = s.app().recovery.store.as_ref().unwrap().own_dir();
    drop(s);
    running.lock().unwrap().remove(&100);
    assert!(crashed_dir.exists(), "a crash leaves the session folder");

    // The next start.
    let mut b = Scenario::new_document("recovery-restart");
    b.app_mut().recovery = RecoveryState::with_store(Store::new(root.clone(), Arc::new(fake_b)));
    start(&mut b);
    assert_eq!(b.app().rail_panel(), RailPanel::Recovery);
    assert!(b.app().rail_open());
    assert_eq!(b.app().recovery.found.len(), 2);
    let entry = b
        .app()
        .recovery
        .found
        .iter()
        .find(|entry| crate::recovery_panel::entry_title(entry) == "letter.sam")
        .unwrap()
        .clone();
    assert_eq!(entry.meta.path.as_deref(), Some(sam.as_path()));
    assert!(entry.meta.bytes > 0);

    // Recover the letter.
    let key = key_of(&b, "letter.sam");
    act_recovery(&mut b, "Recover", RecoveryAction::Recover(key));
    assert_eq!(b.app().sessions.len(), 2);
    assert_eq!(b.app().active_session, 1);
    assert_eq!(b.app().display_name(), "letter.sam (recovered)");
    assert!(b.app().dirty);
    assert!(b.app().tab_label().starts_with("letter.sam (recovered)"));
    assert_eq!(
        b.main_text(),
        "Saved text of the letter and an unsaved tail!"
    );
    assert!(matches!(&b.app().location, DocumentLocation::LegacySource(p) if *p == sam));
    // The other copy is still on offer, in the panel.
    assert_eq!(b.app().recovery.found.len(), 1);
    assert_eq!(b.app().rail_panel(), RailPanel::Recovery);

    // Save does not write over the original: it asks where to save.
    let asked = send_unrun(&mut b.rt, Message::Command(CommandId::Save));
    assert!(asked, "Save asks for a place");
    assert!(b.app().sam_save.is_none());
    assert_eq!(std::fs::read(&sam).unwrap(), original);

    // The recovered tab gets a copy of its own at the next tick, and the
    // old one goes then.
    assert!(entry.data.exists());
    tick(&mut b);
    let recovered_id = session_id(&b);
    assert!(copy_of(&b, recovered_id).exists());
    assert!(
        !entry.data.exists(),
        "the source copy goes once it has a copy of its own"
    );

    // Discard the unsaved note.
    let key = key_of(&b, "Untitled");
    let note = b.app().recovery.found[0].clone();
    act_recovery(&mut b, "Discard", RecoveryAction::Discard(key));
    assert!(b.app().recovery.found.is_empty());
    assert!(!note.data.exists());

    // Save As a new native file: the copy goes and the suffix with it.
    let saved = folder.join("recovered-letter.amipro");
    b.act("Save As", {
        let saved = saved.clone();
        move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::SaveDialogFinished {
                session_id,
                result: crate::FileDialogResult::Selected(saved),
                format: Some(crate::SaveFormat::Native),
            });
        }
    });
    assert!(saved.exists());
    assert!(
        !copy_of(&b, recovered_id).exists(),
        "a saved document leaves no copy"
    );
    assert_eq!(b.app().display_name(), "recovered-letter.amipro");
    assert_eq!(
        std::fs::read(&sam).unwrap(),
        original,
        "the original is still untouched"
    );

    // A clean exit removes this instance's folder.
    let own = b.app().recovery.store.as_ref().unwrap().own_dir();
    b.act("Quit", |rt| {
        rt.send(Message::Command(CommandId::Quit));
    });
    assert!(b.app().quit_requested);
    assert!(!own.exists(), "a clean exit leaves nothing");
    // And a third start finds nothing.
    let mut c = Scenario::new_document("recovery-third");
    c.app_mut().recovery = RecoveryState::with_store(Store::new(
        root,
        Arc::new(Fake {
            pid: 300,
            now: clock,
            running,
        }),
    ));
    start(&mut c);
    assert!(c.app().recovery.found.is_empty());
    assert_eq!(c.app().rail_panel(), RailPanel::Styles);
}

#[test]
fn a_document_saved_or_closed_without_saving_leaves_no_copy_and_a_clean_exit_leaves_none() {
    let mut s = Scenario::new_document("recovery-saved-closed");
    let folder = s.clean_folder();
    let root = folder.join("recovery");
    s.app_mut().recovery = RecoveryState::with_store(Store::new(
        root.clone(),
        Arc::new(Fake::new(100, 2_000_000)),
    ));
    s.type_text("First document");
    let first = session_id(&s);
    tick(&mut s);
    assert!(copy_of(&s, first).exists());
    // Saving removes the copy.
    s.save_sam("first.sam");
    assert!(!copy_of(&s, first).exists());
    // Typing again and undoing back to the saved text removes it too.
    s.type_text(" more");
    tick(&mut s);
    assert!(copy_of(&s, first).exists());
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(!s.app().dirty);
    assert!(
        !copy_of(&s, first).exists(),
        "changed back to the saved state"
    );

    // A second document, closed with Discard.
    s.command_act(CommandId::NewDocument);
    s.type_text("Second, to be discarded");
    let second = session_id(&s);
    tick(&mut s);
    assert!(copy_of(&s, second).exists());
    s.act("File › Close", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    assert!(copy_of(&s, second).exists(), "asking is not discarding");
    s.act("Discard", |rt| {
        rt.send(Message::ConfirmCloseDiscard);
    });
    assert!(!copy_of(&s, second).exists());

    // A third, left open: the exit with Discard removes the whole folder.
    s.command_act(CommandId::NewDocument);
    s.type_text("Third");
    tick(&mut s);
    let own = s.app().recovery.store.as_ref().unwrap().own_dir();
    assert!(own.exists());
    s.act("Quit", |rt| {
        rt.send(Message::Command(CommandId::Quit));
    });
    assert!(s.app().pending_close.is_some());
    assert!(own.exists(), "the question comes first");
    s.act("Discard", |rt| {
        rt.send(Message::ConfirmCloseDiscard);
    });
    assert!(!own.exists());
}

#[test]
fn two_windows_do_not_offer_each_other_their_copies() {
    let mut a = Scenario::new_document("recovery-two-a");
    let root = a.clean_folder().join("recovery");
    let fake_a = Fake::new(100, 3_000_000);
    let fake_b = fake_a.sibling(200);
    let running = fake_a.running.clone();
    a.app_mut().recovery = RecoveryState::with_store(Store::new(root.clone(), Arc::new(fake_a)));
    a.type_text("Typed in the first window");
    tick(&mut a);
    let id = session_id(&a);
    assert!(copy_of(&a, id).exists());

    let mut b = Scenario::new_document("recovery-two-b");
    b.app_mut().recovery = RecoveryState::with_store(Store::new(root, Arc::new(fake_b)));
    start(&mut b);
    assert!(
        b.app().recovery.found.is_empty(),
        "the first window still runs"
    );
    assert_eq!(b.app().rail_panel(), RailPanel::Styles);
    // The menu command shows an empty panel.
    b.command_act(CommandId::RecoverDocuments);
    assert_eq!(b.app().rail_panel(), RailPanel::Recovery);
    assert!(b.app().recovery.found.is_empty());

    // The first window dies; File › Recover Documents… now lists it.
    running.lock().unwrap().remove(&100);
    b.command_act(CommandId::RecoverDocuments);
    assert_eq!(b.app().recovery.found.len(), 1);
    // Esc closes the panel and keeps the copy.
    b.act("Esc", |rt| {
        rt.send(Message::Escape);
    });
    assert_eq!(b.app().rail_panel(), RailPanel::Styles);
    assert!(copy_of(&a, id).exists());
    // Discard all removes it.
    b.command_act(CommandId::RecoverDocuments);
    act_recovery(&mut b, "Discard all", RecoveryAction::DiscardAll);
    assert!(!copy_of(&a, id).exists());
}

#[test]
fn recovery_off_a_password_and_a_damaged_copy() {
    let mut s = Scenario::new_document("recovery-edges");
    let root = s.clean_folder().join("recovery");
    let fake = Fake::new(100, 4_000_000);
    let other = fake.sibling(200);
    let running = fake.running.clone();
    s.app_mut().recovery = RecoveryState::with_store(Store::new(root.clone(), Arc::new(fake)));

    // Preferences: off writes nothing.
    s.app_mut().preferences.keep_recovery = false;
    s.type_text("Not copied");
    let id = session_id(&s);
    tick(&mut s);
    assert!(!copy_of(&s, id).exists());
    assert!(s.app().recovery_interval().is_none());
    // Preferences › General sets both through the window's messages.
    s.command_act(CommandId::Preferences);
    s.act("Keep recovery copies", |rt| {
        rt.send(Message::Preferences(
            crate::preferences_dialog::PreferencesAction::KeepRecovery(true),
        ));
        rt.send(Message::Preferences(
            crate::preferences_dialog::PreferencesAction::RecoveryMinutes(5),
        ));
    });
    assert!(s.app().preferences.keep_recovery);
    assert_eq!(s.app().preferences.recovery_minutes, 5);
    assert_eq!(
        s.app().recovery_interval(),
        Some(std::time::Duration::from_secs(300))
    );
    // A document with a password is never copied.
    s.app_mut().password = Some("secret".into());
    s.type_text(" and protected");
    tick(&mut s);
    assert!(!copy_of(&s, id).exists());
    s.app_mut().password = None;
    tick(&mut s);
    assert!(copy_of(&s, id).exists());

    // The copy is damaged by a disk fault.
    std::fs::write(copy_of(&s, id), "not a document").unwrap();
    running.lock().unwrap().remove(&100);
    let mut b = Scenario::new_document("recovery-edges-b");
    b.app_mut().recovery = RecoveryState::with_store(Store::new(root, Arc::new(other)));
    start(&mut b);
    assert_eq!(b.app().recovery.found.len(), 1);
    let key = b.app().recovery.found[0].key();
    let entry = b.app().recovery.found[0].clone();
    assert!(read(&entry).is_err());
    act_recovery(&mut b, "Recover", RecoveryAction::Recover(key));
    assert_eq!(b.app().sessions.len(), 1, "nothing opened");
    assert!(
        b.app()
            .recovery
            .notice
            .as_deref()
            .is_some_and(|notice| notice.contains("could not be read") && notice.contains("kept"))
    );
    assert!(entry.data.exists(), "the damaged copy is kept");
    assert_eq!(b.app().recovery.found.len(), 1);
}
