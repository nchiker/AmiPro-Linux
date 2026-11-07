//! Files: opening SAM files, importing DOCX, ODT, RTF, and text, Save As
//! first for imports, Save over a SAM (the .bak), Insert › File…, Save As
//! onto an existing file, closing a changed tab.

use std::path::{Path, PathBuf};

use crate::import::OptionsAction;
use amipro_exchange::import::{LineEnds, StyleOption};

use super::*;

/// Delivers `message` to the window but never runs the task it returns (a
/// message that would open the desktop's own dialog). Returns whether it
/// returned a task, which is how a dialog would have been asked for.
fn send_unrun(rt: &mut Runtime, message: Message) -> bool {
    let task = cosmic::Application::update(&mut rt.app, message);
    let asked = cosmic::iced::runtime::task::into_stream(task).is_some();
    rt.redraw();
    asked
}

fn sessions(s: &Scenario) -> usize {
    s.app().sessions.len()
}

/// A SAM file as Ami Pro writes it, with the text typed in it.
const LITERAL_SAM: &[u8] =
    b"[ver]\r\n\t4\r\n[edoc]\r\nHello from a literal file.\r\n\r\nA second paragraph of it.\r\n\r\n>\r\n";

const RTF: &[u8] = br"{\rtf1\ansi{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}{\s1 Heading 1;}}{\s1\b First heading\par}{\s0 Body with \i italic\i0  text\par}{\s0 Second body\par}}";

/// A document of three paragraphs made in the app and saved as SAM, as a
/// first window's work; returns the file.
fn made_document(name: &'static str, file: &str) -> (Scenario, PathBuf) {
    let mut s = Scenario::new_document(name);
    s.clean_folder();
    s.type_text("The first paragraph of the report");
    s.enter();
    s.type_text("The second paragraph");
    s.enter();
    s.type_text("The third and last paragraph");
    let path = s.save_sam(file);
    (s, path)
}

#[test]
fn a_literal_sam_file_opened_edited_and_saved_over_keeps_its_first_original() {
    let mut s = Scenario::new_document("files-literal-sam");
    let folder = s.clean_folder();
    let path = folder.join("OLD.SAM");
    std::fs::write(&path, LITERAL_SAM).unwrap();

    s.open(&path);
    assert_eq!(sessions(&s), 2, "{}", s.app().status);
    assert_eq!(s.app().location, DocumentLocation::Sam(path.clone()));
    assert_eq!(
        s.main_text(),
        "Hello from a literal file.\nA second paragraph of it."
    );
    assert!(!s.app().dirty, "an opened file is not marked changed");
    assert!(s.app().status.contains("Imported"), "{}", s.app().status);

    // Opening it again shows the tab that has it.
    s.no_change("Open the same file again", |rt| {
        rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            path.clone(),
        )));
    });
    assert_eq!(sessions(&s), 2);
    assert!(
        s.app().status.starts_with("Activated"),
        "{}",
        s.app().status
    );

    s.caret_to_end();
    s.type_text(" Edited.");
    assert!(s.app().dirty);
    s.enter();
    s.type_text("A third paragraph, typed in.");

    // Save: no chooser (no task is asked for), the status names the backup,
    // and the backup is the file exactly as it was.
    let asked = send_unrun(&mut s.rt, Message::Command(CommandId::Save));
    assert!(!asked, "Save on a SAM opens no chooser");
    assert!(!s.app().dirty, "{}", s.app().status);
    s.act("Save was made", |_| {});
    let backup = crate::file_workflow::sam_backup_path(&path);
    assert_eq!(std::fs::read(&backup).unwrap(), LITERAL_SAM);
    assert!(
        s.app().status.contains("OLD.SAM.bak"),
        "the status names the backup: {}",
        s.app().status
    );
    let first_save = std::fs::read(&path).unwrap();
    assert_ne!(first_save, LITERAL_SAM);

    // Undo past the save marks the tab changed again; Redo to the saved
    // document marks it saved.
    s.act("Undo past the save", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(s.app().dirty, "Undo after a save leaves the tab changed");
    s.act("Redo back to the saved document", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert!(
        !s.app().dirty,
        "the redone document is the saved one: the tab is not changed"
    );

    // Formatting and a style, then Save again: the backup stays the first.
    s.select_main((0, 0), (0, 5));
    s.command(CommandId::Bold);
    s.save();
    assert_eq!(
        std::fs::read(&backup).unwrap(),
        LITERAL_SAM,
        "the backup is never replaced"
    );
    assert!(
        !s.app().status.contains(".bak"),
        "later saves do not mention the backup: {}",
        s.app().status
    );
    assert_ne!(std::fs::read(&path).unwrap(), first_save);

    // The tab bar names the file; a fresh window opens the saved file to
    // the same text; the backup is a SAM that opens, too.
    let mut s = s.reopen(&path);
    let restored = path.with_file_name("RESTORED.SAM");
    std::fs::copy(&backup, &restored).unwrap();
    s.open(&restored);
    assert_eq!(
        s.main_text(),
        "Hello from a literal file.\nA second paragraph of it."
    );
}

#[test]
fn a_document_made_in_the_app_is_opened_edited_and_saved_over_in_a_fresh_window() {
    let (mut s, path) = made_document("files-made-sam", "REPORT.SAM");
    let first = std::fs::read(&path).unwrap();
    let mut s = s.reopen(&path);
    assert_eq!(s.app().location, DocumentLocation::Sam(path.clone()));
    s.caret_to_end();
    s.type_text(" And more.");
    s.select_main((0, 4), (0, 9));
    s.command(CommandId::Italic);
    s.save();
    let backup = crate::file_workflow::sam_backup_path(&path);
    assert_eq!(std::fs::read(&backup).unwrap(), first);
    s.caret_to_end();
    s.type_text(" Again.");
    s.save();
    assert_eq!(std::fs::read(&backup).unwrap(), first);
    let s = s.reopen(&path);
    assert!(
        s.main_text().ends_with("And more. Again."),
        "{}",
        s.main_text()
    );
}

#[test]
fn save_as_onto_an_existing_sam_asks_first_and_cancel_leaves_it() {
    let (mut s, existing) = made_document("files-save-as-existing", "FIRST.SAM");
    let first = std::fs::read(&existing).unwrap();

    // A second document, to be saved as FIRST.SAM.
    s.act("File › New", |rt| {
        rt.send(Message::Command(CommandId::NewDocument));
    });
    assert_eq!(sessions(&s), 2);
    s.type_text("A different document");
    let session_id = s.app().id;
    let choose = |path: PathBuf| Message::SaveDialogFinished {
        session_id,
        result: FileDialogResult::Selected(path),
        format: Some(SaveFormat::Sam),
    };

    s.act("Save As onto FIRST.SAM", |rt| {
        rt.send(choose(existing.clone()));
    });
    assert_eq!(
        s.app().overwrite_confirmation.pending(),
        Some(existing.as_path()),
        "{}",
        s.app().status
    );
    assert_eq!(
        std::fs::read(&existing).unwrap(),
        first,
        "nothing written yet"
    );
    s.expect_status("cancelled");
    s.act("Cancel", |rt| {
        rt.send(Message::CancelOverwrite);
    });
    assert_eq!(std::fs::read(&existing).unwrap(), first);
    assert!(s.app().dirty);
    assert_eq!(s.app().location, DocumentLocation::Untitled);
    assert!(s.app().overwrite_confirmation.pending().is_none());

    s.clear_expected_status();
    s.act("Save As onto FIRST.SAM again", |rt| {
        rt.send(choose(existing.clone()));
    });
    s.act("Replace", |rt| {
        rt.send(Message::ConfirmOverwrite);
    });
    assert!(!s.app().dirty, "{}", s.app().status);
    assert_eq!(s.app().location, DocumentLocation::Sam(existing.clone()));
    assert_ne!(std::fs::read(&existing).unwrap(), first);
    assert_eq!(
        std::fs::read(crate::file_workflow::sam_backup_path(&existing)).unwrap(),
        first,
        "the file replaced is kept once as .bak"
    );
    s.reopen(&existing);
}

#[test]
fn an_rtf_document_opens_through_import_options_and_is_never_written_over() {
    let mut s = Scenario::new_document("files-import-rtf");
    let folder = s.clean_folder();
    let source = folder.join("Report.rtf");
    std::fs::write(&source, RTF).unwrap();

    s.act("File › Open the RTF", |rt| {
        rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            source.clone(),
        )));
    });
    let dialog = s
        .app()
        .import_dialog
        .as_ref()
        .expect("Import Options asks first");
    assert_eq!(sessions(&s), 1, "nothing is opened before the choice");
    assert_eq!(dialog.options.styles, StyleOption::Convert);
    s.act("Import Options: Cancel", |rt| {
        rt.send(Message::ImportOptions(OptionsAction::Cancel));
    });
    assert!(s.app().import_dialog.is_none());
    assert_eq!(sessions(&s), 1);

    s.act("File › Open the RTF again", |rt| {
        rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            source.clone(),
        )));
    });
    s.act("Import Options: Convert styles, Open", |rt| {
        rt.send(Message::ImportOptions(OptionsAction::Styles(
            StyleOption::Convert,
        )));
        rt.send(Message::ImportOptions(OptionsAction::Open));
    });
    assert_eq!(sessions(&s), 2, "{}", s.app().status);
    assert_eq!(
        s.app().location,
        DocumentLocation::LegacySource(source.clone())
    );
    assert!(
        s.app().status.starts_with("Opened Report.rtf (Rich Text)"),
        "{}",
        s.app().status
    );
    assert_eq!(
        s.main_text(),
        "First heading\nBody with italic text\nSecond body"
    );
    let style_of = |s: &Scenario, paragraph: usize| {
        let document = s.document();
        document.styles()[&document.main_flow().paragraphs[paragraph].style]
            .name
            .clone()
    };
    assert_eq!(style_of(&s, 0), "Heading 1");
    assert_eq!(style_of(&s, 1), "Body Text");
    assert!(!s.app().dirty);

    // Save goes to Save As: a chooser is asked for, and nothing is written
    // over the source.
    let asked = send_unrun(&mut s.rt, Message::Command(CommandId::Save));
    assert!(asked, "Save on an import asks where to save");
    assert!(s.app().sam_save.is_none());
    assert_eq!(std::fs::read(&source).unwrap(), RTF);

    // Work in it, then Save As SAM (a new file); the source is untouched.
    s.caret_to_end();
    s.enter();
    s.type_text("Typed after the import");
    s.select_main((1, 0), (1, 4));
    s.command(CommandId::Bold);
    let sam = s.save_sam("REPORT.SAM");
    assert_eq!(std::fs::read(&source).unwrap(), RTF);
    assert_eq!(s.app().location, DocumentLocation::Sam(sam.clone()));
    let mut s = s.reopen(&sam);
    assert_eq!(
        s.main_text(),
        "First heading\nBody with italic text\nSecond body\nTyped after the import"
    );
    assert_eq!(style_of(&s, 0), "Heading 1");
    // And now Save writes over the SAM it became, the .bak the file Save
    // As made.
    let made = std::fs::read(&sam).unwrap();
    s.caret_to_end();
    s.type_text("!");
    s.save();
    assert_eq!(
        std::fs::read(crate::file_workflow::sam_backup_path(&sam)).unwrap(),
        made
    );
}

#[test]
fn text_files_open_with_ascii_options_and_a_damaged_file_says_so() {
    let mut s = Scenario::new_document("files-import-text");
    let folder = s.clean_folder();
    let text = folder.join("notes.txt");
    std::fs::write(&text, b"one\r\ntwo\r\n\r\nthree\r\n").unwrap();
    let damaged = folder.join("Damaged.docx");
    std::fs::write(&damaged, b"PK\x03\x04 not a package").unwrap();

    s.act("File › Open the text file", |rt| {
        rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            text.clone(),
        )));
    });
    assert_eq!(
        s.app()
            .import_dialog
            .as_ref()
            .unwrap()
            .options
            .text
            .line_ends,
        LineEnds::Paragraphs
    );
    s.act("ASCII options: Join lines, Open", |rt| {
        rt.send(Message::ImportOptions(OptionsAction::LineEnds(
            LineEnds::Joined,
        )));
        rt.send(Message::ImportOptions(OptionsAction::Open));
    });
    assert_eq!(s.main_text(), "one two\nthree", "{}", s.app().status);
    assert_eq!(
        s.app().location,
        DocumentLocation::LegacySource(text.clone())
    );
    assert!(send_unrun(&mut s.rt, Message::Command(CommandId::Save)));
    s.caret_to_end();
    s.type_text(" and four");
    let sam = s.save_sam("NOTES.SAM");
    assert_eq!(
        std::fs::read(&text).unwrap(),
        b"one\r\ntwo\r\n\r\nthree\r\n"
    );

    // The choice is remembered for the next text file.
    s.act("File › Open the text file as a new copy", |rt| {
        let other = text.with_file_name("notes2.txt");
        std::fs::write(&other, b"a\r\nb\r\n").unwrap();
        rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            other,
        )));
    });
    assert_eq!(
        s.app()
            .import_dialog
            .as_ref()
            .unwrap()
            .options
            .text
            .line_ends,
        LineEnds::Joined,
        "the last choice is preselected"
    );
    s.act("Cancel", |rt| {
        rt.send(Message::ImportOptions(OptionsAction::Cancel));
    });
    assert_eq!(s.app().status, "Open cancelled");

    // A damaged package says so and opens nothing.
    let tabs = sessions(&s);
    s.expect_status("Import failed");
    s.no_change("File › Open a damaged DOCX", |rt| {
        rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            damaged.clone(),
        )));
        rt.send(Message::ImportOptions(OptionsAction::Open));
    });
    assert_eq!(sessions(&s), tabs);
    assert!(
        s.app().status.starts_with("Import failed: "),
        "{}",
        s.app().status
    );
    s.clear_expected_status();
    s.reopen(&sam);
}

/// The app's own export of `s`'s document in `format` at `path`.
fn export(s: &Scenario, format: amipro_exchange::Format, path: &Path) {
    let prepared = prepared_export(s.app(), format, path);
    std::fs::write(path, prepared.bytes).unwrap();
}

#[test]
fn word_and_opendocument_files_the_app_exported_open_with_their_text_and_styles() {
    let mut s = Scenario::new_document("files-import-docx-odt");
    let folder = s.clean_folder();
    s.type_text("A heading of the exported document");
    let heading = *s
        .document()
        .styles()
        .iter()
        .find(|(_, style)| style.name == "Heading 1")
        .expect("Heading 1")
        .0;
    s.edit("apply Heading 1", |rt| {
        rt.send(Message::ApplyStyle(heading));
    });
    s.enter();
    let body = *s
        .document()
        .styles()
        .iter()
        .find(|(_, style)| style.name == "Body Text")
        .unwrap()
        .0;
    s.edit("apply Body Text", |rt| {
        rt.send(Message::ApplyStyle(body));
    });
    s.type_text("Body text with a ");
    s.command_act(CommandId::Bold);
    s.type_text("bold");
    s.command_act(CommandId::Bold);
    s.type_text(" word, café and £5.");
    let original = s.main_text();

    let docx = folder.join("Exported.docx");
    let odt = folder.join("Exported.odt");
    export(&s, amipro_exchange::Format::Docx, &docx);
    export(&s, amipro_exchange::Format::Odt, &odt);

    for (name, path) in [("Word", &docx), ("OpenDocument", &odt)] {
        s.act(&format!("File › Open the {name} file"), |rt| {
            rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
                path.clone(),
            )));
        });
        assert!(
            s.app().import_dialog.is_some(),
            "{name}: {}",
            s.app().status
        );
        s.act("Import Options: Open", |rt| {
            rt.send(Message::ImportOptions(OptionsAction::Open));
        });
        assert_eq!(
            s.app().location,
            DocumentLocation::LegacySource(path.clone()),
            "{name}: {}",
            s.app().status
        );
        assert_eq!(s.main_text().replace('\u{FFFC}', ""), original, "{name}");
        let document = s.document();
        let first = &document.main_flow().paragraphs[0];
        assert_eq!(document.styles()[&first.style].name, "Heading 1", "{name}");
        let second = &document.main_flow().paragraphs[1];
        assert_eq!(document.styles()[&second.style].name, "Body Text", "{name}");
        assert!(
            second
                .runs
                .iter()
                .any(|run| run.format.bold == Some(true) && (run.start, run.end) == (17, 21)),
            "{name}: the bold run came in: {:?}",
            second.runs
        );
        assert!(
            send_unrun(&mut s.rt, Message::Command(CommandId::Save)),
            "{name}"
        );
    }
    assert_eq!(sessions(&s), 3);

    // Work in the ODT tab and keep it as SAM.
    s.caret_to_end();
    s.type_text(" Added after the import.");
    let sam = s.save_sam("FROMODT.SAM");
    let s = s.reopen(&sam);
    assert!(s.main_text().ends_with("Added after the import."));
}

#[test]
fn insert_file_puts_a_document_at_the_caret_as_one_undo_step() {
    let (mut s, path) = made_document("files-insert-file", "BASE.SAM");
    let folder = s.folder();
    let rtf = folder.join("Insert.rtf");
    std::fs::write(&rtf, RTF).unwrap();
    let text = folder.join("Tail.txt");
    std::fs::write(&text, b"Tail one\r\n\r\nTail two\r\n").unwrap();
    let unknown = folder.join("Picture.bin");
    std::fs::write(&unknown, [0u8, 1, 2, 3, 4, 5]).unwrap();
    let before = s.main_text();

    // At the start of the second paragraph.
    let main = s.main_flow();
    s.caret(main, 1, 0, 1, false);
    let session_id = s.app().id;
    s.edit("Insert › File… Insert.rtf", |rt| {
        rt.send(Message::InsertFileDialogFinished {
            session_id,
            result: FileDialogResult::Selected(rtf.clone()),
        });
        assert!(rt.app.import_dialog.is_some(), "Import Options asks first");
        rt.send(Message::ImportOptions(OptionsAction::Open));
        assert!(
            rt.app.status.starts_with("Inserted Insert.rtf"),
            "{}",
            rt.app.status
        );
    });
    assert_eq!(sessions(&s), 1, "the file goes into this tab");
    assert!(s.app().dirty);
    let text_now = s.main_text();
    assert!(text_now.contains("First heading"), "{text_now}");
    assert!(text_now.contains("The second paragraph"), "{text_now}");
    assert!(
        text_now.find("First heading") < text_now.find("The second paragraph"),
        "inserted before the second paragraph: {text_now}"
    );
    assert_eq!(
        s.document().main_flow().paragraphs.len(),
        before.lines().count() + 3,
        "{text_now}"
    );

    // Typing goes on after the inserted text.
    s.type_text(">>");

    // A text file at the end.
    s.caret_to_end();
    s.edit("Insert › File… Tail.txt", |rt| {
        rt.send(Message::InsertFileDialogFinished {
            session_id,
            result: FileDialogResult::Selected(text.clone()),
        });
        rt.send(Message::ImportOptions(OptionsAction::Open));
    });
    assert!(s.main_text().contains("Tail two"), "{}", s.main_text());

    // A file of no format read, and a cancelled chooser, change nothing.
    s.expect_status("is not a");
    s.no_change("Insert › File… of an unknown file", |rt| {
        rt.send(Message::InsertFileDialogFinished {
            session_id,
            result: FileDialogResult::Selected(unknown.clone()),
        });
    });
    assert!(s.app().import_dialog.is_none());
    s.clear_expected_status();
    s.expect_status("cancelled");
    s.no_change("Insert › File… cancelled in the chooser", |rt| {
        rt.send(Message::InsertFileDialogFinished {
            session_id,
            result: FileDialogResult::Cancelled,
        });
    });
    s.clear_expected_status();

    s.save();
    assert!(std::fs::read(crate::file_workflow::sam_backup_path(&path)).is_ok());
    s.reopen(&path);
}

#[test]
fn closing_a_changed_tab_asks_and_each_answer_does_what_it_says() {
    let (mut s, path) = made_document("files-close-changed", "CLOSE.SAM");
    let saved = std::fs::read(&path).unwrap();

    // A clean tab closes at once.
    s.act("File › New", |rt| {
        rt.send(Message::Command(CommandId::NewDocument));
    });
    assert_eq!(sessions(&s), 2);
    s.act("File › Close", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    assert_eq!(sessions(&s), 1);
    assert_eq!(s.app().location, DocumentLocation::Sam(path.clone()));

    // A changed tab asks; Cancel keeps it; Discard closes it and the file
    // on disk is as it was.
    s.caret_to_end();
    s.type_text(" -- changed");
    s.act("File › Close on a changed tab", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    assert_eq!(sessions(&s), 1);
    assert!(s.app().pending_close.is_some(), "the close prompt shows");
    assert!(cosmic::Application::dialog(s.app()).is_some());
    s.act("Cancel", |rt| {
        rt.send(Message::CancelClose);
    });
    assert!(s.app().pending_close.is_none());
    assert!(s.main_text().ends_with("-- changed"));
    s.act("File › Close again", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    s.act("Save", |rt| {
        rt.send(Message::SaveAndClose);
    });
    // The only tab: a new untitled document takes its place.
    assert_eq!(sessions(&s), 1);
    assert_eq!(s.app().location, DocumentLocation::Untitled);
    assert_ne!(std::fs::read(&path).unwrap(), saved, "Save wrote the file");
    assert_eq!(
        std::fs::read(crate::file_workflow::sam_backup_path(&path)).unwrap(),
        saved
    );

    // Discard on an untitled changed tab; Save on an untitled one asks for
    // a name, and the tab closes once the file is written.
    s.type_text("Never saved");
    s.act("File › Close", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    s.act("Discard", |rt| {
        rt.send(Message::ConfirmCloseDiscard);
    });
    assert_eq!(s.main_text(), "");
    assert_eq!(sessions(&s), 1);

    s.type_text("Saved when closing");
    s.act("File › Close", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    let asked = send_unrun(&mut s.rt, Message::SaveAndClose);
    assert!(asked, "an untitled tab asks where to save");
    assert!(s.app().close_after_save.is_some());
    let target = s.folder().join("WHENCLOSED.SAM");
    let session_id = s.app().id;
    s.act("the chooser answers", |rt| {
        rt.send(Message::SaveDialogFinished {
            session_id,
            result: FileDialogResult::Selected(target.clone()),
            format: Some(SaveFormat::Sam),
        });
    });
    assert!(target.exists(), "{}", s.app().status);
    assert_eq!(s.main_text(), "", "the tab closed; a new document shows");
    let s = {
        let mut s = s;
        s.open(&target);
        assert_eq!(s.main_text(), "Saved when closing");
        s
    };
    drop(s);
}

#[test]
fn a_cancelled_save_as_changes_nothing_and_a_refused_name_says_why() {
    let mut s = Scenario::new_document("files-save-as-problem");
    let folder = s.clean_folder();
    s.type_text("A document not yet saved");
    let session_id = s.app().id;

    s.expect_status("cancelled");
    s.act("Save As cancelled in the chooser", |rt| {
        rt.send(Message::SaveDialogFinished {
            session_id,
            result: FileDialogResult::Cancelled,
            format: Some(SaveFormat::Sam),
        });
    });
    assert!(s.app().dirty);
    assert_eq!(s.app().location, DocumentLocation::Untitled);
    s.clear_expected_status();

    // A style sheet's name is not a document's: a dialog says why and the
    // document is still unsaved.
    // (The checker flags a shown save problem, so the step reads the
    // dialog and closes it, as the user does, before the checker looks.)
    s.expect_status("failed");
    s.no_change(
        "Save As to a .STY name, then close the problem dialog",
        |rt| {
            rt.send(Message::SaveDialogFinished {
                session_id,
                result: FileDialogResult::Selected(folder.join("SHEET.STY")),
                format: None,
            });
            let problem = rt.app.save_problem.clone().expect("a dialog says why");
            assert_eq!(problem.title, "Not a document name");
            assert!(cosmic::Application::dialog(&rt.app).is_some());
            rt.send(Message::SaveProblemClose);
        },
    );
    assert!(!folder.join("SHEET.STY").exists());
    s.clear_expected_status();
    assert!(s.app().dirty);

    // A native save works and Save then writes over it.
    s.act("Save As .amipro", |rt| {
        rt.send(Message::SaveDialogFinished {
            session_id,
            result: FileDialogResult::Selected(folder.join("native.amipro")),
            format: Some(SaveFormat::Native),
        });
    });
    assert!(folder.join("native.amipro").exists(), "{}", s.app().status);
    assert!(!s.app().dirty);
    s.type_text(" and then changed");
    s.save_native();
}

impl Scenario {
    /// Save (Ctrl+S) on a native document.
    fn save_native(&mut self) {
        let path = self.app().location.path().map(Path::to_path_buf).unwrap();
        self.act("Save", |rt| {
            rt.send(Message::Command(CommandId::Save));
        });
        assert!(!self.app().dirty, "{}", self.app().status);
        let loaded = Document::load_native(&path).unwrap();
        assert_eq!(loaded.plain_text(), self.main_text());
    }
}

#[test]
fn a_password_set_protects_every_save_and_the_file_opens_only_with_it() {
    use crate::password_dialog::PasswordAction;
    use amipro_formats::protect::{ProtectError, is_protected, protect, unprotect};

    const INVALID_OPEN: &str =
        "Invalid Password. Encrypted file will not open with incorrect password.";
    let mut s = Scenario::new_document("files-password");
    s.clean_folder();
    s.type_text("A confidential paragraph");

    // File › Password… opens the box; nothing about the session changes.
    s.no_change("File › Password…", |rt| {
        rt.send(Message::Command(CommandId::Password));
    });
    assert!(s.app().password_dialog.is_some(), "the box opens");
    assert_eq!(s.app().password, None);
    assert!(s.app().dirty, "typing had changed the document already");
    assert_eq!(sessions(&s), 1);

    // A verify that differs is refused.
    s.no_change("Enter Lotus1, verify lotus1, OK", |rt| {
        rt.send(Message::Password(PasswordAction::Password("Lotus1".into())));
        rt.send(Message::Password(PasswordAction::Verify("lotus1".into())));
        rt.send(Message::Password(PasswordAction::Submit));
    });
    let dialog = s.app().password_dialog.clone().expect("the box stays open");
    assert!(
        dialog
            .error
            .as_deref()
            .is_some_and(|e| e.starts_with("Invalid Password Verification")),
        "{:?}",
        dialog.error
    );
    assert_eq!(s.app().password, None, "a refused verify sets nothing");

    // The right verify sets it: not an undo step, and the document changed.
    let undo_before = s.app().history.undo_len();
    s.no_change("Verify Lotus1, OK", |rt| {
        rt.send(Message::Password(PasswordAction::Verify("Lotus1".into())));
        rt.send(Message::Password(PasswordAction::Submit));
    });
    assert!(s.app().password_dialog.is_none(), "the box closes");
    assert_eq!(s.app().password.as_deref(), Some("Lotus1"));
    assert!(s.app().dirty);
    assert_eq!(
        s.app().history.undo_len(),
        undo_before,
        "a password is not an undo step"
    );

    // Save As SAM writes it protected; only the right password reads it.
    let path = s.save_sam("SECRET.SAM");
    let bytes = std::fs::read(&path).unwrap();
    assert!(
        bytes.starts_with(b"[encrypt]\r\n"),
        "{:?}",
        &bytes[..bytes.len().min(16)]
    );
    assert!(is_protected(&bytes));
    let plain = unprotect(&bytes, "Lotus1").expect("the password opens it");
    let read = amipro_formats::import_sam(&plain, None, "SECRET").unwrap();
    assert_eq!(read.document.plain_text(), "A confidential paragraph");
    assert_eq!(
        unprotect(&bytes, "lotus1"),
        Err(ProtectError::WrongPassword)
    );
    assert!(matches!(
        amipro_formats::import_sam_path(&path),
        Err(amipro_formats::ImportError::PasswordProtected)
    ));
    // The bytes the app wrote are the ones the library would make.
    assert_eq!(protect(&plain, "Lotus1"), bytes);

    // Close it and open the file again: the box asks, no tab opens.
    s.act("File › Close", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    assert_eq!(s.main_text(), "");
    assert_eq!(s.app().password, None, "the new document has no password");
    assert_eq!(sessions(&s), 1);
    s.no_change("Open SECRET.SAM", |rt| {
        rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            path.clone(),
        )));
    });
    assert_eq!(sessions(&s), 1, "no tab opens before the password");
    let dialog = s.app().password_dialog.clone().expect("the box asks");
    assert!(matches!(
        dialog.purpose,
        crate::password_dialog::Purpose::Open(_)
    ));
    assert_eq!(dialog.error, None);

    // A wrong password: Ami Pro's message, the box stays, no tab.
    s.no_change("Open with the password wrong", |rt| {
        rt.send(Message::Password(PasswordAction::Password("wrong".into())));
        rt.send(Message::Password(PasswordAction::Submit));
    });
    assert_eq!(sessions(&s), 1);
    let dialog = s.app().password_dialog.clone().expect("the box stays open");
    assert_eq!(dialog.error.as_deref(), Some(INVALID_OPEN));

    // The right one opens it with the password kept for its saves.
    s.act("Open with Lotus1", |rt| {
        rt.send(Message::Password(PasswordAction::Password("Lotus1".into())));
        rt.send(Message::Password(PasswordAction::Submit));
    });
    assert!(s.app().password_dialog.is_none());
    assert_eq!(s.app().location, DocumentLocation::Sam(path.clone()));
    assert_eq!(s.main_text(), "A confidential paragraph");
    assert_eq!(s.app().password.as_deref(), Some("Lotus1"));
    assert!(!s.app().dirty, "an opened file is not marked changed");

    // Editing and File › Save keep it protected.
    s.caret_to_end();
    s.type_text(" and then more");
    s.save();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"[encrypt]\r\n"));
    let plain = unprotect(&bytes, "Lotus1").unwrap();
    let read = amipro_formats::import_sam(&plain, None, "SECRET").unwrap();
    assert_eq!(
        read.document.plain_text(),
        "A confidential paragraph and then more"
    );

    // Password… › Remove: saves are plain again.
    s.no_change("File › Password…", |rt| {
        rt.send(Message::Command(CommandId::Password));
    });
    assert!(s.app().password_dialog.is_some());
    s.no_change("Remove", |rt| {
        rt.send(Message::Password(PasswordAction::Remove));
    });
    assert!(s.app().password_dialog.is_none());
    assert_eq!(s.app().password, None);
    assert!(s.app().dirty, "removing the password changes the document");
    s.save();
    let bytes = std::fs::read(&path).unwrap();
    assert!(
        bytes.starts_with(b"[ver]"),
        "{:?}",
        &bytes[..bytes.len().min(16)]
    );
    assert!(!is_protected(&bytes));
    let read = amipro_formats::import_sam_path(&path).unwrap();
    assert_eq!(
        read.document.plain_text(),
        "A confidential paragraph and then more"
    );
}

#[test]
fn a_protected_file_reverts_opens_from_recent_files_and_is_refused_by_insert_file() {
    use crate::password_dialog::{PasswordAction, Purpose};
    let mut s = Scenario::new_document("files-password-more");
    s.clean_folder();
    s.type_text("Saved words");
    s.no_change("File › Password…, Pw9, Pw9", |rt| {
        rt.send(Message::Command(CommandId::Password));
        rt.send(Message::Password(PasswordAction::Password("Pw9".into())));
        rt.send(Message::Password(PasswordAction::Verify("Pw9".into())));
        rt.send(Message::Password(PasswordAction::Submit));
    });
    assert_eq!(s.app().password.as_deref(), Some("Pw9"));
    let path = s.save_sam("REVERT.SAM");
    assert!(std::fs::read(&path).unwrap().starts_with(b"[encrypt]"));

    // Revert to Saved reads the protected file with the document's password.
    s.caret_to_end();
    s.type_text(" and unsaved ones");
    s.command_act(CommandId::RevertToSaved);
    assert!(s.app().revert_pending);
    s.edit("Revert", |rt| {
        rt.send(Message::RevertToSaved(true));
    });
    assert_eq!(s.main_text(), "Saved words");
    assert!(!s.app().dirty);
    assert_eq!(s.app().password.as_deref(), Some("Pw9"), "kept");

    // From File's recent files: the box asks; Cancel opens nothing.
    s.act("File › Close", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    let recent = s
        .app()
        .preferences
        .recent_files
        .iter()
        .position(|file| *file == path)
        .expect("the saved file is a recent file");
    s.no_change("File › 1 REVERT.SAM", |rt| {
        rt.send(Message::OpenRecent(recent));
    });
    let dialog = s.app().password_dialog.clone().expect("the box asks");
    assert!(matches!(dialog.purpose, Purpose::Open(_)));
    s.no_change("Cancel", |rt| {
        rt.send(Message::Password(PasswordAction::Cancel));
    });
    assert!(s.app().password_dialog.is_none());
    assert_eq!(s.main_text(), "", "still the empty document");
    s.act("File › 1 REVERT.SAM, Pw9", |rt| {
        rt.send(Message::OpenRecent(recent));
        rt.send(Message::Password(PasswordAction::Password("Pw9".into())));
        rt.send(Message::Password(PasswordAction::Submit));
    });
    assert_eq!(s.main_text(), "Saved words");
    assert_eq!(s.app().password.as_deref(), Some("Pw9"));

    // Insert › File refuses a protected file, as Ami Pro does (3148).
    s.act("File › Close", |rt| {
        rt.send(Message::Command(CommandId::CloseDocument));
    });
    s.type_text("Host text");
    let session_id = s.app().id;
    s.expect_status("encrypted file");
    s.act("Insert › File… REVERT.SAM", |rt| {
        rt.send(Message::InsertFileDialogFinished {
            session_id,
            result: FileDialogResult::Selected(path.clone()),
        });
    });
    assert_eq!(s.main_text(), "Host text");
    assert!(
        s.app().status.contains("encrypted file"),
        "{}",
        s.app().status
    );
    assert!(
        s.app().password_dialog.is_none(),
        "Insert File does not ask"
    );
}
