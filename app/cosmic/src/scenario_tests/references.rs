//! Notes, footnotes, endnotes, bookmarks, and cross-references
//! (`UI_SPEC.md` §34).

use super::*;
use crate::references_panel::{Kind, ReferenceAction};
use amipro_document::{FootnoteOptions, InlineObject, bookmarks};
use amipro_editing::references::{note_text, notes};
use cosmic::iced::widget::text_editor::{Action, Edit};

fn reference(s: &mut Scenario, what: &str, action: ReferenceAction) {
    s.act(what, move |rt| {
        rt.send(Message::References(action));
    });
}

fn reference_edit(s: &mut Scenario, what: &str, action: ReferenceAction) {
    s.edit(what, move |rt| {
        rt.send(Message::References(action));
    });
}

/// Characters typed in the open note's editor, one undo step.
fn note_typed(s: &mut Scenario, text: &str) {
    let text = text.to_owned();
    s.edit(&format!("type {text:?} in the note"), move |rt| {
        for character in text.chars() {
            rt.send(Message::References(ReferenceAction::NoteEdit(
                Action::Edit(Edit::Insert(character)),
            )));
        }
    });
}

fn caret_at(s: &mut Scenario, paragraph: usize, byte: usize) {
    let main = s.main_flow();
    s.caret(main, paragraph, byte, 1, false);
}

fn footnote_flows(s: &Scenario) -> Vec<FlowId> {
    s.document()
        .paragraphs()
        .iter()
        .flat_map(|paragraph| paragraph.objects.iter())
        .filter_map(|object| match object {
            InlineObject::Footnote { flow } => Some(*flow),
            _ => None,
        })
        .collect()
}

#[test]
fn notes_inserted_typed_navigated_coloured_and_removed() {
    let mut s = Scenario::new_document("refs-notes");
    s.clean_folder();
    s.app_mut().preferences.note_initials = "LS".into();
    s.type_text("One two three. Four five six.");
    caret_at(&mut s, 0, 3);

    // Insert › Note opens the Notes panel with an editor for the new note.
    s.command(CommandId::InsertNote);
    if s.app().references.as_ref().map(|panel| panel.kind) != Some(Kind::Notes) {
        s.fail("Insert Note", "the Notes panel did not open");
    }
    let found = notes(s.document());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.initials.as_deref(), Some("LS"));
    note_typed(&mut s, "Check this");
    assert_eq!(
        note_text(s.document(), notes(s.document())[0].2),
        "Check this"
    );

    // A second note later in the text, numbered after the first.
    let second_at = s.main_text().find("Four").unwrap();
    caret_at(&mut s, 0, second_at);
    s.command(CommandId::InsertNote);
    note_typed(&mut s, "And this");
    let found = notes(s.document());
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].1.number, Some(1));
    assert_eq!(found[1].1.number, Some(2));

    // Initials and colour change for new notes.
    reference(&mut s, "Initials", ReferenceAction::Initials("XY".into()));
    // Next and Previous Note, Go to Note.
    caret_at(&mut s, 0, 0);
    s.act("Next Note", |rt| {
        rt.send(Message::References(ReferenceAction::NextNote));
    });
    s.act("Next Note again", |rt| {
        rt.send(Message::References(ReferenceAction::NextNote));
    });
    s.act("Previous Note", |rt| {
        rt.send(Message::References(ReferenceAction::PreviousNote));
    });
    let first = notes(s.document())[0].0;
    reference(
        &mut s,
        "Go to the first note",
        ReferenceAction::GoToNote(first),
    );

    // Type right before and after a note's mark in the text; the notes stay.
    let (first, _, _) = notes(s.document())[0].clone();
    caret_at(&mut s, first.paragraph, first.byte);
    s.type_text("[");
    let (first, _, _) = notes(s.document())[0].clone();
    caret_at(&mut s, first.paragraph, first.byte + '\u{FFFC}'.len_utf8());
    s.type_text("]");
    assert_eq!(notes(s.document()).len(), 2);

    // View › Notes is a check, on by default.
    assert!(s.app().preferences.show_notes);
    s.command_act(CommandId::ShowNotes);
    assert!(!s.app().preferences.show_notes);
    s.command_act(CommandId::ShowNotes);

    // Remove one note (one undo step; the text stays); then all of them.
    let (first, _, _) = notes(s.document())[0].clone();
    reference_edit(
        &mut s,
        "Remove the first note",
        ReferenceAction::RemoveNote(first),
    );
    assert_eq!(notes(s.document()).len(), 1);
    assert!(s.main_text().contains("[]"), "{}", s.main_text());
    reference_edit(&mut s, "Remove all notes", ReferenceAction::RemoveAllNotes);
    assert!(notes(s.document()).is_empty());

    // A note again, then Delete and Backspace over its mark in the text.
    caret_at(&mut s, 0, 5);
    s.command(CommandId::InsertNote);
    note_typed(&mut s, "temporary");
    let (mark, _, _) = notes(s.document())[0].clone();
    caret_at(&mut s, mark.paragraph, mark.byte);
    s.edit("Delete takes the mark", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert!(
        notes(s.document()).is_empty(),
        "the note went with its mark"
    );
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(notes(s.document()).len(), 1);
    let (mark, _, _) = notes(s.document())[0].clone();
    caret_at(&mut s, mark.paragraph, mark.byte + '\u{FFFC}'.len_utf8());
    s.edit("Backspace takes the mark", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    assert!(notes(s.document()).is_empty());
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    // A selection holding the mark, deleted.
    let (mark, _, _) = notes(s.document())[0].clone();
    s.select_main(
        (mark.paragraph, mark.byte.saturating_sub(2)),
        (mark.paragraph, mark.byte + 5),
    );
    s.edit("Delete a selection holding the mark", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert!(notes(s.document()).is_empty());
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });

    let path = s.save_sam("NOTES.SAM");
    let s = s.reopen(&path);
    assert_eq!(notes(s.document()).len(), 1);
    assert_eq!(
        note_text(s.document(), notes(s.document())[0].2),
        "temporary"
    );
}

#[test]
fn footnotes_typed_in_place_numbered_navigated_and_turned_into_endnotes() {
    let mut s = Scenario::new_document("refs-footnotes");
    s.clean_folder();
    s.type_text("First claim. Second claim.");
    s.enter();
    s.type_text("Third paragraph.");
    let main = s.main_flow();
    caret_at(&mut s, 0, "First claim.".len());

    // Insert › Footnote puts the caret in the footnote; typed in place.
    s.command(CommandId::Footnote);
    let first = s.app().page_selection.focus.flow;
    assert_ne!(first, main, "the caret is in the footnote");
    s.type_text("Source one.");
    assert_eq!(
        s.document().flow(first).unwrap().plain_text(),
        "Source one."
    );
    // Enter in a footnote makes a second paragraph.
    s.enter();
    s.type_text("More on one.");
    // Esc goes back to the text after the reference.
    s.no_change("Esc", |rt| {
        rt.send(Message::Page(PageAction::Escape));
    });
    assert_eq!(s.app().page_selection.focus.flow, main);
    s.type_text(" Aside.");

    // A second footnote at the end of the second sentence.
    let at = s.main_text().find("Second claim.").unwrap() + "Second claim.".len();
    caret_at(&mut s, 0, at);
    s.command(CommandId::Footnote);
    let second = s.app().page_selection.focus.flow;
    s.type_text("Source two.");
    s.no_change("Esc", |rt| {
        rt.send(Message::Page(PageAction::Escape));
    });
    assert_eq!(footnote_flows(&s), [first, second]);
    // Both are laid out on the page in the footnote area.
    let on_page = |s: &Scenario, flow: FlowId| {
        s.app().pages.layout().pages[0]
            .lines
            .iter()
            .any(|line| line.container == amipro_layout::Container::Footnote(flow))
    };
    assert!(on_page(&s, first) && on_page(&s, second));

    // The footnote's text typed at its start and end.
    s.caret(second, 0, 0, 1, false);
    s.type_text("(");
    let end = s.document().flow(second).unwrap().paragraphs[0].text.len();
    s.caret(second, 0, end, 1, false);
    s.type_text(")");
    assert_eq!(
        s.document().flow(second).unwrap().plain_text(),
        "(Source two.)"
    );

    // Go To › Next item steps through the footnotes.
    s.caret(main, 1, 0, 1, false);
    s.act("Go To", |rt| {
        rt.send(Message::Command(CommandId::GoToPage));
    });
    s.act("Item: footnotes", |rt| {
        rt.send(Message::GoTo(crate::GoToAction::Item(3)));
    });
    s.act("Step", |rt| {
        rt.send(Message::GoTo(crate::GoToAction::Step(true)));
    });
    assert_ne!(s.app().page_selection.focus.flow, main);

    // Footnote Options: the panel; endnotes put the notes on a page of
    // their own; starting number and style.
    s.command_act(CommandId::FootnoteOptions);
    if s.app().references.as_ref().map(|panel| panel.kind) != Some(Kind::Footnotes) {
        s.fail("Footnote Options", "the Footnotes panel did not open");
    }
    let pages = s.app().pages.layout().pages.len();
    // (Footnote options are not among the facts the checker compares, so
    // these are plain steps; Undo is checked here.)
    reference(
        &mut s,
        "Endnotes",
        ReferenceAction::Flag(FootnoteOptions::ENDNOTES, true),
    );
    assert!(s.document().footnote_options().endnotes());
    assert_eq!(
        s.app().pages.layout().pages.len(),
        pages + 1,
        "the endnotes' page"
    );
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(!s.document().footnote_options().endnotes());
    assert_eq!(s.app().pages.layout().pages.len(), pages);
    s.act("Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    reference(&mut s, "Start at 3", ReferenceAction::Start("3".into()));
    assert_eq!(s.document().footnote_options().start, 3);
    reference(&mut s, "Number style", ReferenceAction::NumberStyle);
    reference(
        &mut s,
        "Endnotes off",
        ReferenceAction::Flag(FootnoteOptions::ENDNOTES, false),
    );

    // Previous and Next footnote in the panel.
    s.act("Next footnote", |rt| {
        rt.send(Message::References(ReferenceAction::NextFootnote));
    });
    s.act("Previous footnote", |rt| {
        rt.send(Message::References(ReferenceAction::PreviousFootnote));
    });

    // Delete the first reference: its footnote goes with it; Undo.
    caret_at(&mut s, 0, 0);
    let mark = s.main_text().find('\u{FFFC}').unwrap();
    caret_at(&mut s, 0, mark);
    s.edit("Delete before the reference", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert_eq!(
        footnote_flows(&s).len(),
        1,
        "the footnote went with its reference"
    );
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(footnote_flows(&s).len(), 2);

    let path = s.save_sam("FOOTNOTE.SAM");
    let s = s.reopen(&path);
    assert_eq!(footnote_flows(&s).len(), 2);
}

#[test]
fn bookmarks_added_gone_to_renamed_typed_in_and_removed() {
    let mut s = Scenario::new_document("refs-bookmarks");
    s.clean_folder();
    s.type_text("Alpha beta gamma delta.");
    s.enter();
    s.type_text("Second paragraph.");
    // Select "beta gamma" and open the Bookmarks panel.
    s.select_main((0, 6), (0, 16));
    s.command_act(CommandId::Bookmarks);
    if s.app().references.as_ref().map(|panel| panel.kind) != Some(Kind::Bookmarks) {
        s.fail("Bookmarks", "the panel did not open");
    }
    // A name with a space is refused in place.
    reference(
        &mut s,
        "name with a space",
        ReferenceAction::Name("two words".into()),
    );
    s.no_change("Add with a bad name", |rt| {
        rt.send(Message::References(ReferenceAction::Add));
    });
    if s.app().references.as_ref().unwrap().problem.is_none() {
        s.fail("Add", "a name with a space was not refused");
    }
    reference(&mut s, "name", ReferenceAction::Name("middle".into()));
    reference_edit(&mut s, "Add", ReferenceAction::Add);
    assert_eq!(bookmarks::bookmarks(s.document()).len(), 1);
    // The same name again is refused (names are not case-sensitive).
    s.select_main((1, 0), (1, 6));
    reference(&mut s, "same name", ReferenceAction::Name("MIDDLE".into()));
    s.no_change("Add a duplicate", |rt| {
        rt.send(Message::References(ReferenceAction::Add));
    });
    assert_eq!(bookmarks::bookmarks(s.document()).len(), 1);

    // A second bookmark.
    reference(&mut s, "name", ReferenceAction::Name("second".into()));
    reference_edit(&mut s, "Add the second", ReferenceAction::Add);
    assert_eq!(bookmarks::bookmarks(s.document()).len(), 2);

    // Go To selects the bookmarked text from anywhere.
    caret_at(&mut s, 1, 3);
    s.act("Go To middle", |rt| {
        rt.send(Message::References(ReferenceAction::GoTo("MIDDLE".into())));
    });
    let selection = s.app().page_selection.ordered();
    assert_eq!((selection.0.byte, selection.1.byte), (6, 16));

    // Typing inside the bookmark keeps it around the text; at its edges the
    // text goes outside (or in; either way the bookmark stays).
    caret_at(&mut s, 0, 11);
    s.type_text("++");
    assert_eq!(bookmarks::bookmarks(s.document()).len(), 2);
    s.act("Go To middle again", |rt| {
        rt.send(Message::References(ReferenceAction::GoTo("middle".into())));
    });
    let selection = s.app().page_selection.ordered();
    assert!(
        selection.1.byte - selection.0.byte >= 12,
        "the bookmark grew: {selection:?}"
    );
    caret_at(&mut s, 0, 0);
    s.type_text(">> ");
    let end = s.document().paragraphs()[0].text.len();
    caret_at(&mut s, 0, end);
    s.type_text(" <<");
    assert_eq!(bookmarks::bookmarks(s.document()).len(), 2);

    // Rename, then Remove.
    reference(
        &mut s,
        "Select middle",
        ReferenceAction::Select("middle".into()),
    );
    reference(&mut s, "new name", ReferenceAction::Name("centre".into()));
    reference_edit(&mut s, "Rename", ReferenceAction::Rename);
    assert!(
        bookmarks::bookmarks(s.document())
            .iter()
            .any(|b| b.name == "centre")
    );
    reference(
        &mut s,
        "Select second",
        ReferenceAction::Select("second".into()),
    );
    reference_edit(&mut s, "Remove", ReferenceAction::Remove);
    assert_eq!(bookmarks::bookmarks(s.document()).len(), 1);

    // Delete the bookmarked text altogether: the bookmark goes or collapses
    // without leaving a field or mark behind; Undo restores it.
    reference(
        &mut s,
        "Go to centre",
        ReferenceAction::GoTo("centre".into()),
    );
    s.edit("Delete the bookmarked text", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(bookmarks::bookmarks(s.document()).len(), 1);

    // View › Bookmarks toggles their brackets.
    s.command_act(CommandId::ShowBookmarks);
    s.command_act(CommandId::ShowBookmarks);

    let path = s.save_sam("BOOKMARK.SAM");
    let s = s.reopen(&path);
    assert_eq!(bookmarks::bookmarks(s.document()).len(), 1);
    assert_eq!(bookmarks::bookmarks(s.document())[0].name, "centre");
}

#[test]
fn cross_references_show_a_bookmarks_page_and_text_and_follow_updates() {
    let mut s = Scenario::new_document("refs-crossrefs");
    s.clean_folder();
    // Enough lines for a second page, the target near the end.
    let lines: Vec<String> = (0..80).map(|index| format!("Line {index}.")).collect();
    let main = s.main_flow();
    s.edit("paste 80 lines", {
        let text = lines.join("\n");
        move |rt| {
            rt.send(Message::Page(PageAction::Insert(text)));
        }
    });
    let pages = s.app().pages.layout().pages.len();
    assert!(pages >= 2, "{pages} pages");
    s.select_main((75, 0), (75, "Line 75.".len()));
    s.command_act(CommandId::Bookmarks);
    reference(&mut s, "name", ReferenceAction::Name("target".into()));
    reference_edit(&mut s, "Add", ReferenceAction::Add);

    // Insert › Cross-Reference… with the page, then with the text.
    s.caret(main, 0, 0, 1, false);
    s.command_act(CommandId::CrossReference);
    reference(
        &mut s,
        "Select target",
        ReferenceAction::Select("target".into()),
    );
    reference_edit(
        &mut s,
        "Page",
        ReferenceAction::CrossReference { page: true },
    );
    assert!(
        s.document().paragraphs()[0]
            .text
            .starts_with(&pages.to_string())
            || s.document().paragraphs()[0]
                .text
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit()),
        "{:?}",
        s.document().paragraphs()[0].text
    );
    s.caret(main, 1, 0, 1, false);
    reference_edit(
        &mut s,
        "Text",
        ReferenceAction::CrossReference { page: false },
    );
    assert!(s.document().paragraphs()[1].text.starts_with("Line 75."));
    // Type right before and after each cross-reference field; none goes
    // empty or nested (the checker also fails those).
    s.caret(main, 0, 0, 1, false);
    s.type_text("p.");
    let first_end = s.document().paragraphs()[0].text.find("Line 0.").unwrap();
    s.caret(main, 0, first_end, 1, false);
    s.type_text(" ");
    // Update All Fields works the references out again.
    s.act("Update All Fields", |rt| {
        rt.send(Message::Command(CommandId::UpdateAllFields));
    });
    // Add text before the target so it may move; the page field follows.
    s.caret(main, 2, 0, 1, false);
    s.edit("add a page of lines before the target", {
        let text: String = (0..60).map(|index| format!("Extra {index}\n")).collect();
        move |rt| {
            rt.send(Message::Page(PageAction::Insert(text)));
        }
    });
    s.act("Update All Fields again", |rt| {
        rt.send(Message::Command(CommandId::UpdateAllFields));
    });
    // Remove the bookmark: the cross-reference fields say so (or empty
    // visibly), never a silent empty field.
    reference(
        &mut s,
        "Select target",
        ReferenceAction::Select("target".into()),
    );
    reference_edit(&mut s, "Remove", ReferenceAction::Remove);
    s.expect_status("not a valid field name");
    s.act("Update All Fields without the bookmark", |rt| {
        rt.send(Message::Command(CommandId::UpdateAllFields));
    });

    let path = s.save_sam("CROSSREF.SAM");
    s.reopen(&path);
}
