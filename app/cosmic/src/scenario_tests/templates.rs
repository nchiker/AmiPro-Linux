//! File › New… (`UI_SPEC.md` §39, CM1, CM2): a document from a style
//! sheet, with or without its contents, saved as SAM naming its sheet.

use amipro_document::Document;
use amipro_formats::SheetOptions;

use crate::new_document_dialog::NewAction;
use crate::preferences_dialog::PreferencesAction;

use super::*;

/// A style sheet written by the app's own writer: a document's styles and
/// layout, its text when `contents`.
fn write_sheet(folder: &Path, name: &str, text: &str, description: &str, contents: bool) {
    let bytes = amipro_formats::style_sheet(
        &Document::from_plain_text(text),
        &SheetOptions {
            description: description.into(),
            saved_at: 0,
            contents,
        },
    );
    std::fs::write(folder.join(name), bytes).unwrap();
}

fn dialog(s: &Scenario) -> &crate::new_document_dialog::NewDocumentDialog {
    s.app()
        .new_document_dialog
        .as_ref()
        .unwrap_or_else(|| s.fail("dialog", "File › New… is not open"))
}

fn sheet_index(s: &Scenario, name: &str) -> usize {
    dialog(s)
        .sheets
        .iter()
        .position(|sheet| sheet.name == name)
        .unwrap_or_else(|| s.fail("dialog", &format!("{name} is not listed")))
}

#[test]
fn a_memo_from_a_style_sheet_with_contents_is_saved_as_sam_and_reopened() {
    let mut s = Scenario::new_document("new-from-sheet");
    let folder = s.clean_folder();
    let sheets = folder.join("styles");
    std::fs::create_dir_all(&sheets).unwrap();
    write_sheet(
        &sheets,
        "MEMO.STY",
        "MEMO\nTo:\nFrom:",
        "Memo with lines",
        true,
    );
    write_sheet(&sheets, "PLAIN.STY", "", "Plain pages", false);
    // A sheet that runs a macro in Ami Pro (`[newmac]`, CM2).
    let mut letter = Vec::new();
    write_sheet(&sheets, "LETTER.STY", "Dear", "Letter", true);
    let written = std::fs::read(sheets.join("LETTER.STY")).unwrap();
    let at = written.windows(5).position(|w| w == b"[tag]").unwrap();
    letter.extend_from_slice(&written[..at]);
    letter.extend_from_slice(b"[newmac]\r\n\tLETTER.SMM\r\n");
    letter.extend_from_slice(&written[at..]);
    std::fs::write(sheets.join("LETTER.STY"), letter).unwrap();

    // The folder is added in Preferences › Folders.
    s.command_act(CommandId::Preferences);
    let chosen = sheets.clone();
    s.act("add the style-sheet folder", move |rt| {
        rt.send(Message::Preferences(PreferencesAction::FolderChosen(Some(
            chosen,
        ))));
        rt.send(Message::Preferences(PreferencesAction::Close));
    });

    // File › New… lists the three by description; the memo has contents.
    s.command_act(CommandId::NewFromStyleSheet);
    assert_eq!(dialog(&s).sheets.len(), 3);
    assert!(dialog(&s).by_description);
    let memo = sheet_index(&s, "MEMO.STY");
    let plain = sheet_index(&s, "PLAIN.STY");
    assert_eq!(
        dialog(&s).sheets[memo].label(true),
        "Memo with lines (MEMO.STY)"
    );
    assert!(dialog(&s).sheets[memo].has_contents);
    assert!(!dialog(&s).sheets[plain].has_contents);
    s.act("select the memo", move |rt| {
        rt.send(Message::NewDocumentDialog(NewAction::Select(memo)));
    });
    assert!(dialog(&s).with_contents);
    let preview = dialog(&s).preview.clone().expect("a preview of the memo");
    assert!(preview.lines.len() >= 3, "{preview:?}");
    s.act("Create", |rt| {
        rt.send(Message::NewDocumentDialog(NewAction::Create));
    });
    assert!(s.app().new_document_dialog.is_none());
    assert_eq!(s.app().sessions.len(), 2);
    assert!(
        s.main_text().starts_with("MEMO\nTo:\nFrom:"),
        "{:?}",
        s.main_text()
    );
    assert_eq!(
        s.document().style_sheet().map(|link| link.name.as_str()),
        Some("MEMO.STY")
    );

    // Typed into, saved as SAM naming its sheet, and reopened.
    s.caret_to_end();
    s.type_text(" Ann");
    let path = s.save_sam("MEMO.SAM");
    let bytes = std::fs::read(&path).unwrap();
    assert!(
        bytes.windows(8).any(|w| w == b"MEMO.STY"),
        "the SAM names its sheet"
    );

    // Recent first next time; a blank document from the plain sheet.
    s.command_act(CommandId::NewFromStyleSheet);
    assert_eq!(dialog(&s).recent, vec![sheet_index(&s, "MEMO.STY")]);
    assert_eq!(dialog(&s).selected, Some(sheet_index(&s, "MEMO.STY")));
    s.act("select the plain sheet and Create", move |rt| {
        rt.send(Message::NewDocumentDialog(NewAction::Select(plain)));
        rt.send(Message::NewDocumentDialog(NewAction::Create));
    });
    assert_eq!(s.main_text(), "");
    assert_eq!(
        s.document().style_sheet().map(|link| link.name.as_str()),
        Some("PLAIN.STY")
    );

    // The letter's macro is named, not run; without contents it is blank.
    s.command_act(CommandId::NewFromStyleSheet);
    let letter = sheet_index(&s, "LETTER.STY");
    assert_eq!(
        dialog(&s).sheets[letter].run_macro.as_deref(),
        Some("LETTER.SMM")
    );
    s.act("the letter without contents", move |rt| {
        rt.send(Message::NewDocumentDialog(NewAction::Select(letter)));
        rt.send(Message::NewDocumentDialog(NewAction::WithContents(false)));
        rt.send(Message::NewDocumentDialog(NewAction::Create));
    });
    assert_eq!(s.main_text(), "");
    assert!(
        s.app().status.contains("LETTER.SMM is not run"),
        "{}",
        s.app().status
    );

    // Cancel leaves everything as it was.
    let tabs = s.app().sessions.len();
    s.command_act(CommandId::NewFromStyleSheet);
    s.act("Cancel", |rt| {
        rt.send(Message::NewDocumentDialog(NewAction::Cancel));
    });
    assert_eq!(s.app().sessions.len(), tabs);
}
