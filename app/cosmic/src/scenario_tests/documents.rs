//! Documents as a user makes them: typed, formatted, styled, saved,
//! reopened, saved over.

use super::*;

/// The id of the style named `name` (any case).
fn style(scenario: &Scenario, name: &str) -> amipro_document::StyleId {
    *scenario
        .document()
        .styles()
        .iter()
        .find(|(_, style)| style.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| scenario.fail("style", &format!("no style named {name:?}")))
        .0
}

#[test]
fn a_new_document_typed_formatted_saved_reopened_and_saved_over() {
    let mut s = Scenario::new_document("new-document");
    s.clean_folder();

    s.type_text("A letter from AmiPro-Linux");
    s.command(CommandId::Center);
    s.enter();
    s.command(CommandId::AlignLeft);
    s.type_text("Typed text with a ");
    s.command_act(CommandId::Bold);
    s.type_text("bold");
    s.command_act(CommandId::Bold);
    s.type_text(" word and café, naïve, £5.");
    s.enter();
    s.type_text("A second paragraph to style.");
    let title = style(&s, "Heading 1");
    s.edit("apply Heading 1", |rt| {
        rt.send(Message::ApplyStyle(title));
    });

    // Select a word and make it italic, then delete a word.
    s.select_main((1, 2), (1, 7));
    s.command(CommandId::Italic);
    s.select_main((1, 0), (1, 6));
    s.edit("Delete the selection", |rt| {
        rt.send(Message::Command(CommandId::Delete));
    });

    let path = s.save_sam("LETTER.SAM");
    let mut s = s.reopen(&path);

    // Saved over: the original kept once as LETTER.SAM.bak.
    s.caret_to_end();
    s.type_text(" More.");
    let first = std::fs::read(&path).unwrap();
    s.save();
    let backup = crate::file_workflow::sam_backup_path(&path);
    assert_eq!(
        std::fs::read(&backup).unwrap(),
        first,
        "the backup is the original"
    );
    s.type_text(" Again.");
    s.save();
    assert_eq!(
        std::fs::read(&backup).unwrap(),
        first,
        "the backup is never replaced"
    );
    s.reopen(&path);
}

// ---- Typing undoes as one step per run (SU4) --------------------------------

/// Keys typed one by one, as the window delivers them (the page takes the
/// keyboard first, as a click on it does).
fn keys(rt: &mut Runtime, text: &str) {
    rt.app.page_keyboard += 1;
    for character in text.chars() {
        rt.events(&[key_press(
            cosmic::iced::keyboard::Key::Character(character.to_string().into()),
            cosmic::iced::keyboard::Modifiers::default(),
        )]);
    }
}

fn named_key(rt: &mut Runtime, key: cosmic::iced::keyboard::key::Named) {
    rt.app.page_keyboard += 1;
    rt.events(&[key_press(
        cosmic::iced::keyboard::Key::Named(key),
        cosmic::iced::keyboard::Modifiers::default(),
    )]);
}

fn undo(s: &mut Scenario) -> String {
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    s.main_text()
}

#[test]
fn a_run_of_typing_undoes_and_redoes_at_once() {
    let mut s = Scenario::new_document("typing-run");
    s.clean_folder();
    // `edit` checks that one Undo returns the document before all the keys
    // and one Redo the one after, the caret at the run's end.
    s.edit("type “Ann” key by key", |rt| keys(rt, "Ann"));
    assert_eq!(s.main_text(), "Ann");
    s.edit("type “ and Bob” key by key", |rt| keys(rt, " and Bob"));
    assert_eq!(s.main_text(), "Ann and Bob");
    s.edit("Backspace three times", |rt| {
        for _ in 0..3 {
            named_key(rt, cosmic::iced::keyboard::key::Named::Backspace);
        }
    });
    assert_eq!(s.main_text(), "Ann and ");
    s.save_sam("RUN.SAM");
}

#[test]
fn a_caret_move_a_command_enter_or_a_pause_ends_a_run_of_typing() {
    use cosmic::iced::keyboard::key::Named;
    let mut s = Scenario::new_document("typing-run-ends");
    s.act("type, Left, type", |rt| {
        keys(rt, "Ann");
        named_key(rt, Named::ArrowLeft);
        keys(rt, "x");
    });
    assert_eq!(s.main_text(), "Anxn");
    assert_eq!(undo(&mut s), "Ann", "the caret move ended the first run");
    assert_eq!(undo(&mut s), "");

    s.act("type, Bold at the caret, type", |rt| {
        keys(rt, "Ann");
        rt.send(Message::Command(CommandId::Bold));
        keys(rt, "b");
    });
    assert_eq!(s.main_text(), "Annb");
    assert_eq!(undo(&mut s), "Ann", "the command ended the run");
    assert_eq!(undo(&mut s), "");

    s.act("type, Enter, type", |rt| {
        keys(rt, "Ann");
        named_key(rt, Named::Enter);
        keys(rt, "Bob");
    });
    assert_eq!(undo(&mut s), "Ann\n", "Enter is a step of its own");
    assert_eq!(undo(&mut s), "Ann");
    assert_eq!(undo(&mut s), "");

    s.act("type, Backspace, type", |rt| {
        keys(rt, "Annie");
        named_key(rt, Named::Backspace);
        keys(rt, "a");
    });
    assert_eq!(s.main_text(), "Annia");
    assert_eq!(
        undo(&mut s),
        "Anni",
        "deleting and typing are runs of their own"
    );
    assert_eq!(undo(&mut s), "Annie");
    assert_eq!(undo(&mut s), "");

    s.act("type, a pause, type", |rt| {
        keys(rt, "Ann");
        rt.app.sessions[rt.app.active_session].age_typing_run();
        keys(rt, "ie");
    });
    assert_eq!(undo(&mut s), "Ann", "the pause ended the run");
    assert_eq!(undo(&mut s), "");

    s.act("type, Paste, type", |rt| {
        keys(rt, "Ann");
        rt.app.insert_clipboard_text(" pasted".into());
        keys(rt, "!");
    });
    assert_eq!(s.main_text(), "Ann pasted!");
    assert_eq!(
        undo(&mut s),
        "Ann pasted",
        "the key after Paste starts a run"
    );
    assert_eq!(undo(&mut s), "Ann", "Paste is a step of its own");
    assert_eq!(undo(&mut s), "");
}
