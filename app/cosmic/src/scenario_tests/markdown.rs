//! Markdown as a user meets it (MD1, `UI_SPEC.md` §30 and §31): a document
//! with a picture, a list, and a table exported as Markdown, the loss list
//! answered, the .md and its picture written, the .md opened in a new tab,
//! edited, saved as SAM, and reopened. Nothing here opens a desktop dialog:
//! the choosers' finished messages are sent.

use super::output::{export, made_document, open_exported, paragraph_texts};
use super::pictures::{bmp_file, insert_image};
use super::*;
use crate::file_workflow::SaveFormat;
use amipro_exchange::Format;

#[test]
fn a_document_is_exported_as_markdown_opened_edited_and_saved_as_sam() {
    let mut s = Scenario::new_document("markdown-round");
    s.clean_folder();
    s.type_text("Introduction line.");
    let photo = bmp_file(&s, "photo.bmp");
    insert_image(&mut s, &photo, false);
    s.caret_to_end();
    made_document(&mut s);

    // Export › Markdown always lists what it cannot hold.
    let path = export(&mut s, Format::Markdown, "Made.md", Some(true));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("Introduction line."), "{text}");
    assert!(text.contains("- first item"), "{text}");
    assert!(text.contains("| Name | Qty |"), "{text}");
    let picture = s.folder().join("Made-1.png");
    assert!(picture.is_file(), "the picture is written beside the .md");
    assert!(text.contains("](Made-1.png)"), "{text}");

    // Opened in a new tab: the same text, an import never written over.
    let tab = open_exported(&mut s, &path);
    let read = paragraph_texts(&s.app().sessions[tab].document);
    let document = &s.app().sessions[tab].document;
    let cells: Vec<String> = document
        .flows()
        .flat_map(|(_, flow)| flow.paragraphs.iter().map(|p| p.text.clone()))
        .collect();
    for want in ["Name", "Qty", "Pen", "4"] {
        assert!(cells.iter().any(|cell| cell == want), "{want}: {cells:?}");
    }
    for want in ["Introduction line.", "first item", "After the list."] {
        assert!(
            read.iter().any(|line| line.contains(want)),
            "{want}: {read:?}"
        );
    }
    assert!(
        s.app().status.starts_with("Opened Made.md (Markdown)"),
        "{}",
        s.app().status
    );
    assert!(matches!(
        s.app().location,
        crate::DocumentLocation::LegacySource(_)
    ));
    let before = std::fs::read(&path).unwrap();

    // Edited, then saved as SAM; the .md is untouched, and the SAM reopens.
    s.caret_to_end();
    s.enter();
    s.type_text("Typed after the import.");
    let sam = s.save_sam("MADE.SAM");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let s = s.reopen(&sam);
    assert!(s.main_text().contains("Typed after the import."));
    assert!(s.main_text().contains("Introduction line."));
}

#[test]
fn save_as_markdown_asks_before_writing_over_and_the_tab_keeps_its_file() {
    let mut s = Scenario::new_document("markdown-save-as");
    let folder = s.clean_folder();
    s.type_text("Some words.");
    let target = folder.join("Notes.md");
    std::fs::write(&target, b"old").unwrap();
    let chosen = target.clone();
    s.act("Save As Markdown onto a file", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::SaveDialogFinished {
            session_id,
            result: FileDialogResult::Selected(chosen),
            format: Some(SaveFormat::Markdown),
        });
    });
    let pending = s.app().export_pending.clone().expect("the list is shown");
    assert!(pending.replaces.contains(&"Notes.md".to_owned()));
    assert!(
        pending
            .losses
            .iter()
            .any(|line| line.starts_with("page layout"))
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
    s.act("Cancel", |rt| {
        rt.send(Message::CancelExport);
    });
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
    let chosen = target.clone();
    s.act("Save As Markdown again", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::SaveDialogFinished {
            session_id,
            result: FileDialogResult::Selected(chosen),
            format: Some(SaveFormat::Markdown),
        });
    });
    s.act("Export Anyway", |rt| {
        rt.send(Message::ExportAnyway);
    });
    assert!(
        std::fs::read_to_string(&target)
            .unwrap()
            .contains("Some words.")
    );
    assert_eq!(s.app().location, DocumentLocation::Untitled);
}
