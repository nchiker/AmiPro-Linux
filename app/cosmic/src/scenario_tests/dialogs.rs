//! The modal dialogs worked from the keyboard, keys going through the
//! window (`UI_SPEC.md` §50): Go To by Ctrl+G and Enter, closing a changed
//! document with Save on Enter, Esc cancelling, and Enter doing nothing
//! where the main action destroys something.

use cosmic::iced::keyboard::{Key, Modifiers, key::Named};

use super::*;

fn open_dialog(s: &Scenario) -> bool {
    cosmic::Application::dialog(s.app()).is_some()
}

fn press(s: &mut Scenario, what: &str, key: Key, modifiers: Modifiers) {
    s.no_change(what, move |rt| window_key(rt, key, modifiers));
}

fn enter_key(s: &mut Scenario, what: &str) {
    press(s, what, Key::Named(Named::Enter), Modifiers::empty());
}

fn esc(s: &mut Scenario, what: &str) {
    press(s, what, Key::Named(Named::Escape), Modifiers::empty());
}

fn ctrl(s: &mut Scenario, letter: &'static str) {
    press(
        s,
        &format!("Ctrl+{letter}"),
        Key::Character(letter.into()),
        Modifiers::CTRL,
    );
}

fn typed(s: &mut Scenario, text: &'static str) {
    for character in text.chars() {
        press(
            s,
            &format!("type {character} in the dialog"),
            Key::Character(character.to_string().into()),
            Modifiers::empty(),
        );
    }
}

#[test]
fn go_to_is_worked_from_the_keyboard_alone() {
    let text: Vec<String> = (0..200)
        .map(|index| format!("Paragraph {index}."))
        .collect();
    let mut s = Scenario::new("dialogs-go-to", Document::from_plain_text(&text.join("\n")));
    s.clean_folder();
    assert!(s.app().pages.layout().pages.len() >= 3);
    // The page holds the keyboard, as after a click in the text.
    s.act("the page takes the keyboard", |rt| {
        rt.app.page_keyboard += 1;
        rt.redraw();
    });
    ctrl(&mut s, "g");
    assert!(s.app().go_to.is_some() && open_dialog(&s));
    // The field holds the current page, selected: typing replaces it.
    typed(&mut s, "3");
    enter_key(&mut s, "Enter is Go");
    assert!(s.app().go_to.is_none() && !open_dialog(&s));
    assert_eq!(s.app().page_selection.page, 3);
    assert!(s.rt.workspace_scroll() > 0.0);

    let selection = s.app().page_selection;
    ctrl(&mut s, "g");
    typed(&mut s, "1");
    esc(&mut s, "Esc cancels");
    assert!(!open_dialog(&s));
    assert_eq!(s.app().page_selection, selection);

    // A page that is not there: Enter keeps the dialog, Esc leaves.
    ctrl(&mut s, "g");
    typed(&mut s, "99");
    enter_key(&mut s, "Enter with no such page");
    let state = s.app().go_to.clone().expect("the refusal keeps the dialog");
    assert!(state.error.is_some(), "{state:?}");
    assert_eq!(s.app().page_selection, selection);
    esc(&mut s, "Esc after the refusal");
    assert!(!open_dialog(&s));
}

#[test]
fn closing_asks_with_save_on_enter_and_enter_never_destroys() {
    let mut s = Scenario::new_document("dialogs-close");
    let folder = s.clean_folder();
    s.type_text("The saved text.");
    let path = s.save_sam("CLOSE.SAM");
    let saved = std::fs::read(&path).unwrap();
    s.command_act(CommandId::NewDocument);
    s.type_text("Another tab that stays.");
    s.act("back to the saved document", |rt| {
        rt.send(Message::ActivateTab(0));
        rt.app.page_keyboard += 1;
        rt.redraw();
    });
    s.caret_to_end();
    s.type_text(" And a change.");

    // Revert to Saved: Enter does nothing, Esc keeps the change.
    s.no_change("Revert to Saved…", |rt| {
        rt.send(Message::Command(CommandId::RevertToSaved));
    });
    assert!(s.app().revert_pending && open_dialog(&s));
    enter_key(&mut s, "Enter does not revert");
    assert!(s.app().revert_pending, "the question is still asked");
    esc(&mut s, "Esc");
    assert!(!open_dialog(&s) && s.app().dirty);

    // Save over another file: Enter does not write over it.
    let other = folder.join("OTHER.SAM");
    std::fs::write(&other, b"someone else's file").unwrap();
    let target = other.clone();
    s.no_change("Save As onto an existing file", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::SaveDialogFinished {
            session_id,
            result: crate::FileDialogResult::Selected(target),
            format: Some(crate::SaveFormat::Sam),
        });
    });
    assert!(s.app().overwrite_confirmation.pending().is_some());
    enter_key(&mut s, "Enter does not replace");
    assert!(s.app().overwrite_confirmation.pending().is_some());
    assert_eq!(std::fs::read(&other).unwrap(), b"someone else's file");
    esc(&mut s, "Esc");
    assert!(!open_dialog(&s));
    assert_eq!(std::fs::read(&other).unwrap(), b"someone else's file");

    // A frame: made with Enter, and Enter does not confirm its deletion.
    s.command_act(CommandId::Frame);
    s.edit("Enter is Create", |rt| {
        window_key(rt, Key::Named(Named::Enter), Modifiers::empty());
    });
    assert!(!open_dialog(&s));
    assert!(s.app().frame_selection.is_some());
    let frames = s.document().frame_stack().len();
    s.no_change("Delete asks", |rt| {
        rt.send(Message::Command(CommandId::Delete));
    });
    assert!(s.app().pending_frame_delete);
    enter_key(&mut s, "Enter does not delete");
    assert!(s.app().pending_frame_delete);
    assert_eq!(s.document().frame_stack().len(), frames);
    esc(&mut s, "Esc");
    assert!(!open_dialog(&s));
    assert_eq!(s.document().frame_stack().len(), frames);
    s.act("Undo the frame", |rt| {
        rt.send(Message::Command(CommandId::Undo));
        rt.app.frame_selection = None;
        rt.app.page_keyboard += 1;
        rt.redraw();
    });
    assert_eq!(std::fs::read(&path).unwrap(), saved, "nothing saved yet");

    // Ctrl+W on the changed document: Esc keeps it open and unsaved.
    ctrl(&mut s, "w");
    assert_eq!(s.app().pending_close, Some(0));
    esc(&mut s, "Esc cancels the close");
    assert!(!open_dialog(&s));
    assert_eq!(s.app().sessions.len(), 2);
    assert!(s.app().dirty);
    assert_eq!(std::fs::read(&path).unwrap(), saved);

    // Enter is Save: the file is written and the tab closes.
    ctrl(&mut s, "w");
    assert!(open_dialog(&s));
    s.act("Enter saves and closes", |rt| {
        window_key(rt, Key::Named(Named::Enter), Modifiers::empty());
    });
    assert!(!open_dialog(&s));
    assert_eq!(s.app().sessions.len(), 1);
    assert_eq!(s.main_text(), "Another tab that stays.");
    assert_ne!(std::fs::read(&path).unwrap(), saved);
    s.open(&path);
    assert_eq!(s.main_text(), "The saved text. And a change.");
}
