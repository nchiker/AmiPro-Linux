//! The Contents & Index panel, the generated TOC and index, and the
//! glossary (`UI_SPEC.md` §36; the "TOC, index, glossary" checklist of
//! `UI_ACCEPTANCE.md` §10).

use super::*;
use crate::contents_panel::{ContentsAction as C, IndexChoice, Page, TocChoice};
use crate::glossary_panel::GlossaryAction as G;
use amipro_document::FrameContent;

fn style(s: &Scenario, name: &str) -> amipro_document::StyleId {
    *s.document()
        .styles()
        .iter()
        .find(|(_, style)| style.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| s.fail("style", &format!("no style named {name:?}")))
        .0
}

/// The text of the cells of the TOC table: rows of (entry, page).
fn toc_rows(s: &Scenario) -> Vec<(String, String)> {
    let table = amipro_editing::contents::toc_table(s.document())
        .unwrap_or_else(|| s.fail("toc", "the document has no TOC"));
    let FrameContent::Table(toc) = &s.document().frame(table).unwrap().content else {
        s.fail("toc", "the TOC is not a table");
    };
    let cell = |index: usize| {
        let amipro_document::CellContent::Text(flow) = toc.cells[index].content;
        s.document().flow(flow).unwrap().paragraphs[0].text.clone()
    };
    (0..toc.rows.len())
        .map(|row| (cell(row * 2), cell(row * 2 + 1)))
        .collect()
}

fn toc_count(s: &Scenario) -> usize {
    s.document()
        .frames()
        .filter(|frame| amipro_document::contents::is_toc_table(frame))
        .count()
}

fn main_lines(s: &Scenario) -> Vec<String> {
    s.document()
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect()
}

fn marks(s: &Scenario) -> Vec<String> {
    s.document()
        .paragraphs()
        .iter()
        .flat_map(|paragraph| paragraph.marks.iter().map(|mark| mark.raw.clone()))
        .collect()
}

fn contents_action(s: &mut Scenario, what: &str, action: C) {
    s.act(what, move |rt| {
        rt.send(Message::Contents(action));
    });
}

fn contents_edit(s: &mut Scenario, what: &str, action: C) {
    s.edit(what, move |rt| {
        rt.send(Message::Contents(action));
    });
}

/// The whole task: index entries marked, the index made in a new tab and
/// at the end, TOC entries and a TOC at the start updated after more text
/// is typed around it, a click on a TOC row, saved and reopened.
#[test]
fn a_document_gets_index_entries_a_toc_and_an_index_and_they_follow_the_typing() {
    let mut s = Scenario::new("contents-whole", contents_document());
    s.clean_folder();
    s.command_act(CommandId::ContentsIndex);
    assert_eq!(
        s.app().contents.as_ref().map(|panel| panel.page),
        Some(Page::Contents)
    );

    // Index: "printer" selected in the second paragraph, Mark Index Entry
    // (the panel opens on Entries with the selection), marked.
    s.select_main((1, 4), (1, 11));
    s.command_act(CommandId::MarkIndexEntry);
    let panel = s.app().contents.as_ref().unwrap();
    assert_eq!(
        (panel.page, panel.primary.as_str()),
        (Page::Entries, "printer")
    );
    contents_action(&mut s, "Primary Printer", C::Primary("Printer".into()));
    contents_edit(&mut s, "Mark", C::Mark);
    assert_eq!(marks(&s), ["<:X8,-32768;Index \"Printer\" #>"]);
    // Mark All, case folded: every other "printer".
    contents_action(&mut s, "Primary Printer", C::Primary("Printer".into()));
    contents_action(&mut s, "case folded", C::ExactCase(false));
    contents_edit(&mut s, "Mark All", C::MarkAll);
    assert_eq!(marks(&s).len(), 3);
    // Next Mark, then Remove the one shown.
    let main = s.main_flow();
    s.caret(main, 0, 0, 1, false);
    contents_action(&mut s, "Next Mark", C::NextMark);
    assert!(s.app().contents.as_ref().unwrap().current.is_some());
    contents_edit(&mut s, "Remove the mark", C::Remove);
    assert_eq!(marks(&s).len(), 2);
    // A TOC entry marked at the caret in "Installing"'s text.
    s.caret(main, 1, 3, 1, false);
    s.command_act(CommandId::MarkTocEntry);
    contents_action(&mut s, "TOC level 2", C::TocLevel("2".into()));
    contents_action(&mut s, "TOC text", C::TocText("Setting up".into()));
    contents_edit(&mut s, "Mark TOC entry", C::MarkToc);
    assert!(
        marks(&s)
            .iter()
            .any(|mark| mark == "<:X11,-32768;TOC 2 \"Setting up\">")
    );

    // The index in a new tab: the source stays as it was.
    let tabs = s.app().sessions.len();
    contents_action(
        &mut s,
        "index in a new document",
        C::IndexPlace(IndexChoice::NewDocument),
    );
    s.act("Generate Index", |rt| {
        rt.send(Message::Contents(C::GenerateIndex));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1);
    let index: Vec<String> = s
        .document()
        .paragraphs()
        .iter()
        .map(|p| p.text.clone())
        .collect();
    assert_eq!(index, ["Printer, 1, 2"], "the index tab");
    s.act("back to the document", |rt| {
        rt.send(Message::ActivateTab(0));
    });

    // The TOC at the start: Getting Started, Installing, Advanced.
    contents_action(&mut s, "TOC at the start", C::TocPlace(TocChoice::Start));
    contents_edit(&mut s, "Generate TOC", C::GenerateToc);
    let rows = toc_rows(&s);
    assert!(
        rows.iter()
            .any(|row| row.0 == "Getting Started" && row.1 == "1"),
        "{rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.0 == "Advanced" && row.1 == "2"),
        "{rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.0 == "Setting up"),
        "the TOC entry marked: {rows:?}"
    );
    let first_rows = rows.len();

    // Type around the TOC: a new heading at the end, in the Title style.
    s.caret_to_end();
    s.enter();
    let title = style(&s, "Title");
    s.edit("apply Title", move |rt| {
        rt.send(Message::ApplyStyle(title));
    });
    s.type_text("Appendix");
    // The TOC is stale until Update; Update replaces it in place.
    assert_eq!(toc_rows(&s).len(), first_rows);
    contents_edit(&mut s, "Update TOC", C::GenerateToc);
    let rows = toc_rows(&s);
    assert_eq!(rows.len(), first_rows + 1, "{rows:?}");
    assert!(rows.iter().any(|row| row.0 == "Appendix"), "{rows:?}");
    assert_eq!(toc_count(&s), 1, "replaced in place");
    // Typing in front of the TOC and Undo around it.
    s.caret(main, 0, 0, 1, false);
    s.type_text("Intro. ");
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    s.act("Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert_eq!(toc_count(&s), 1);

    // Ctrl+click on a TOC row goes to its heading; without Ctrl it stays.
    let table = amipro_editing::contents::toc_table(s.document()).unwrap();
    let FrameContent::Table(toc) = &s.document().frame(table).unwrap().content else {
        panic!("a table");
    };
    let row = toc_rows(&s)
        .iter()
        .position(|row| row.0 == "Advanced")
        .unwrap();
    let amipro_document::CellContent::Text(cell) = toc.cells[row * 2].content;
    s.act("hold Ctrl", |rt| {
        rt.send(Message::Modifiers(cosmic::iced::keyboard::Modifiers::CTRL));
    });
    s.act("click the TOC row", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(cell, 0, 2),
            page: 1,
            extend: false,
        }));
    });
    let caret = s.app().page_selection.focus;
    assert_eq!(caret.flow, s.main_flow());
    assert_eq!(s.document().paragraphs()[caret.paragraph].text, "Advanced");
    s.act("release Ctrl", |rt| {
        rt.send(Message::Modifiers(
            cosmic::iced::keyboard::Modifiers::empty(),
        ));
    });
    s.act("click the TOC row", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(cell, 0, 2),
            page: 1,
            extend: false,
        }));
    });
    assert_eq!(
        s.app().page_selection.focus.flow,
        cell,
        "without Ctrl the click stays in the cell"
    );

    // The index at the end of the document: one step, the index after the text.
    contents_action(&mut s, "index at the end", C::IndexPlace(IndexChoice::End));
    contents_edit(&mut s, "Generate Index", C::GenerateIndex);
    let lines = main_lines(&s);
    assert!(
        lines.iter().any(|line| line.starts_with("Printer")),
        "{lines:?}"
    );

    // Remove a level's page numbers: [toc] changes in one step.
    contents_edit(
        &mut s,
        "level 1 without page numbers",
        C::LevelPageNumber(0, false),
    );

    let path = s.save_sam("CONTENTS.SAM");
    let mut reopened = s.reopen(&path);
    reopened.command_act(CommandId::ContentsIndex);
    reopened.edit("Update TOC in the reopened document", |rt| {
        rt.send(Message::Contents(C::GenerateToc));
    });
    assert_eq!(toc_count(&reopened), 1);
}

/// The TOC levels edited in the panel: a level added and removed, moved,
/// leaders and right alignment; each one undoable step.
#[test]
fn the_toc_levels_are_edited_in_the_panel_and_the_toc_follows_them() {
    let mut s = Scenario::new("contents-levels", contents_document());
    s.command_act(CommandId::ContentsIndex);
    let levels = amipro_editing::contents::toc_levels(s.document());
    assert!(levels.len() >= 2, "{levels:?}");
    contents_edit(
        &mut s,
        "level 2 without right alignment",
        C::LevelRightAlign(1, false),
    );
    assert!(!amipro_editing::contents::toc_levels(s.document())[1].right_align);
    contents_edit(
        &mut s,
        "level 1 without page numbers",
        C::LevelPageNumber(0, false),
    );
    contents_edit(&mut s, "Generate TOC", C::GenerateToc);
    let rows = toc_rows(&s);
    assert!(
        rows.iter().any(|row| row.0 == "Getting Started"),
        "{rows:?}"
    );
    s.act("Undo the TOC", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(amipro_editing::contents::toc_table(s.document()).is_none());
}

/// A saved document is clean whatever was done before: here, TOC levels
/// changed, a TOC generated, undone, and the document saved.
#[test]
fn a_document_saved_after_its_toc_was_undone_is_marked_unchanged() {
    let mut s = Scenario::new("contents-save-after-undo", contents_document());
    s.clean_folder();
    s.command_act(CommandId::ContentsIndex);
    contents_edit(
        &mut s,
        "level 2 without right alignment",
        C::LevelRightAlign(1, false),
    );
    contents_edit(&mut s, "Generate TOC", C::GenerateToc);
    s.act("Undo the TOC", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    let path = s.save_sam("LEVELS.SAM");
    s.reopen(&path);
}

/// A document with a generated TOC saves as Ami Pro's SAM with no loss
/// list (the TOC is Ami Pro's own table).
#[test]
fn a_document_with_a_generated_toc_saves_as_sam_without_losses() {
    let mut s = Scenario::new("contents-toc-saves", contents_document());
    s.clean_folder();
    s.command_act(CommandId::ContentsIndex);
    contents_edit(&mut s, "Generate TOC", C::GenerateToc);
    let path = s.save_sam("TOC.SAM");
    s.reopen(&path);
}

/// The glossary: a record made from a selection, expanded with Ctrl+K,
/// completed as you type with Tab, renamed and deleted; the data file is
/// the scratch one, never the user's.
#[test]
fn a_glossary_record_is_made_expanded_completed_and_removed() {
    let mut s = Scenario::new(
        "contents-glossary",
        Document::from_plain_text("Sincerely yours, Ami.\nSay sy"),
    );
    s.clean_folder();
    let path = scratch_glossary("scenario-glossary");
    s.app_mut().preferences.glossary_file = path.display().to_string();
    // Mark Text ▸ Glossary…: the selection becomes the record "sy".
    s.select_main((0, 0), (0, 21));
    s.command_act(CommandId::MarkGlossary);
    s.act("name it sy", |rt| {
        rt.send(Message::Glossary(G::Name("sy".into())));
    });
    s.act("Add", |rt| {
        rt.send(Message::Glossary(G::Add));
    });
    let glossary = amipro_formats::glossary::read_glossary(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(glossary.names(), ["sy"]);
    s.act("name it SY again", |rt| {
        rt.send(Message::Glossary(G::Name("SY".into())));
    });
    s.act("Add again", |rt| {
        rt.send(Message::Glossary(G::Add));
    });
    assert_eq!(
        s.app().glossary.as_ref().unwrap().message.as_deref(),
        Some("Glossary name already in use.")
    );

    // Ctrl+K after "sy": the word becomes the record, one undo step.
    let main = s.main_flow();
    s.caret(main, 1, 6, 1, false);
    s.command(CommandId::InsertGlossary);
    assert_eq!(main_lines(&s)[1], "Say Sincerely yours, Ami.");
    // The expansion is typed after, and Undo/Redo keep it whole (checked
    // by the step).
    s.type_text(" Thanks.");
    s.caret_to_end();
    s.enter();

    // Complete as you type: a second record, Tab completes it.
    let bytes = amipro_formats::glossary::append_record(
        &std::fs::read(&path).unwrap(),
        "addr",
        b"1 Main Street",
    )
    .unwrap();
    std::fs::write(&path, bytes).unwrap();
    s.act("Complete as you type", |rt| {
        rt.send(Message::Glossary(G::Complete(true)));
    });
    s.type_text("addr");
    s.act("Tab completes the record", |rt| {
        rt.send(Message::Page(PageAction::Insert("\t".into())));
    });
    assert_eq!(
        main_lines(&s).last().map(String::as_str),
        Some("1 Main Street")
    );
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });

    // The panel: rename and delete change the data file.
    s.act("Select sy", |rt| {
        rt.send(Message::Glossary(G::Select("sy".into())));
    });
    s.act("rename text", |rt| {
        rt.send(Message::Glossary(G::RenameText("yours".into())));
    });
    s.act("Rename", |rt| {
        rt.send(Message::Glossary(G::Rename));
    });
    let names = |path: &std::path::Path| {
        let mut names = amipro_formats::glossary::read_glossary(&std::fs::read(path).unwrap())
            .unwrap()
            .names();
        names.sort();
        names
    };
    assert_eq!(names(&path), ["addr", "yours"]);
    s.act("Delete", |rt| {
        rt.send(Message::Glossary(G::Delete));
    });
    assert_eq!(names(&path), ["addr"]);
    let saved = s.save_sam("GLOSSARY-DOC.SAM");
    s.reopen(&saved);
}
