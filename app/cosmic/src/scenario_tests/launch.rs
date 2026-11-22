//! Launch: files named on the command line or handed over by another
//! program reach the window as `Message::OpenPaths` (what `main`'s startup
//! task and the D-Bus activation handler both send), and open as File › Open
//! does. The D-Bus transport itself is libcosmic's and is checked
//! physically (`docs/INSTALL.md`).

use std::path::PathBuf;

use super::*;

const SAM: &[u8] =
    b"[ver]\r\n\t4\r\n[edoc]\r\nHanded over by another program.\r\n\r\nSecond paragraph.\r\n\r\n>\r\n";

fn open_paths(s: &mut Scenario, paths: Vec<PathBuf>) {
    s.act("Open the named files", move |rt| {
        rt.send(Message::OpenPaths(paths));
    });
}

#[test]
fn a_named_file_opens_in_place_of_the_untouched_first_document() {
    let mut s = Scenario::new_document("launch-first");
    let folder = s.clean_folder();
    let path = folder.join("NAMED.SAM");
    std::fs::write(&path, SAM).unwrap();
    open_paths(&mut s, vec![path.clone()]);
    assert_eq!(s.app().sessions.len(), 1, "{}", s.app().status);
    assert_eq!(s.app().location, DocumentLocation::Sam(path));
    assert_eq!(
        s.main_text(),
        "Handed over by another program.\nSecond paragraph."
    );
    assert!(s.app().open_problem.is_none());
}

#[test]
fn a_file_handed_to_a_window_with_work_opens_in_a_new_tab_and_leaves_the_work() {
    let mut s = Scenario::new_document("launch-second");
    let folder = s.clean_folder();
    let path = folder.join("OTHER.SAM");
    std::fs::write(&path, SAM).unwrap();
    s.type_text("Unsaved work");
    open_paths(&mut s, vec![path.clone()]);
    assert_eq!(s.app().sessions.len(), 2);
    assert_eq!(s.app().location, DocumentLocation::Sam(path.clone()));
    assert!(
        s.app().sessions[0].dirty,
        "the unsaved document stays, still changed"
    );
    assert_eq!(s.app().sessions[0].document.plain_text(), "Unsaved work");
    // The same file again activates its tab rather than opening a copy.
    s.act("Activate the first tab", |rt| {
        rt.send(Message::ActivateTab(0));
    });
    open_paths(&mut s, vec![path.clone()]);
    assert_eq!(s.app().sessions.len(), 2);
    assert_eq!(s.app().location, DocumentLocation::Sam(path));
}

#[test]
fn a_missing_or_unreadable_file_shows_a_message_and_changes_nothing() {
    let mut s = Scenario::new_document("launch-missing");
    let folder = s.clean_folder();
    s.type_text("Keep me");
    s.expect_status("could not");
    s.expect_status("failed");
    let missing = folder.join("NOPE.SAM");
    open_paths(&mut s, vec![missing.clone()]);
    assert_eq!(s.app().sessions.len(), 1);
    let (title, body) = s.app().open_problem.clone().expect("a dialog says why");
    assert!(title.contains("NOPE.SAM"), "{title}");
    assert!(body.contains("No such file"), "{body}");
    s.act("Dismiss the message", |rt| {
        rt.send(Message::OpenProblemClose);
    });
    assert!(s.app().open_problem.is_none());
    // A folder is not a document.
    open_paths(&mut s, vec![folder.clone()]);
    assert!(s.app().open_problem.is_some());
    s.act("Dismiss the message", |rt| {
        rt.send(Message::OpenProblemClose);
    });
    // A damaged file that does not read as an Ami Pro document says so too.
    let bad = folder.join("BAD.SAM");
    std::fs::write(&bad, b"\0\0not a document").unwrap();
    open_paths(&mut s, vec![bad]);
    assert_eq!(s.app().sessions.len(), 1, "{}", s.app().status);
    assert!(s.app().open_problem.is_some(), "{}", s.app().status);
    assert_eq!(s.main_text(), "Keep me");
}

#[test]
fn several_files_open_one_tab_each() {
    let mut s = Scenario::new_document("launch-several");
    let folder = s.clean_folder();
    let a = folder.join("A.SAM");
    let b = folder.join("B.SAM");
    std::fs::write(&a, SAM).unwrap();
    std::fs::write(&b, SAM).unwrap();
    open_paths(&mut s, vec![a, b.clone()]);
    assert_eq!(s.app().sessions.len(), 2);
    assert_eq!(s.app().location, DocumentLocation::Sam(b));
}
