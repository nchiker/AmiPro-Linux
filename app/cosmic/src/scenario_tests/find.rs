//! Find & Replace (`UI_SPEC.md` §27): the find bar, its options, its
//! matches in the main text, frames, headers, and footnotes; Replace All as
//! one undo step; typing while the bar is open.

use super::*;
use crate::find_bar::{AttributeChoice, Field, FindAction, FindMenu, Toggle};
use crate::frame_editing::FrameAction;

fn find(s: &mut Scenario, what: &str, action: FindAction) {
    s.act(what, move |rt| {
        rt.send(Message::Find(action));
    });
}

fn bar_matches(s: &Scenario) -> usize {
    s.app().find.as_ref().expect("the find bar").matches.len()
}

fn bar_label(s: &Scenario) -> String {
    s.app().find.as_ref().expect("the find bar").count_label()
}

/// The text of the current selection.
fn selected(s: &Scenario) -> String {
    let (start, end) = s.app().page_selection.ordered();
    if start.flow != end.flow || start.paragraph != end.paragraph {
        return String::new();
    }
    let flow = s.document().flow(start.flow).unwrap();
    flow.paragraphs[start.paragraph].text[start.byte..end.byte].to_owned()
}

/// A document: a main text with "needle" three times in different
/// capitals, a frame, a header, and a footnote each with one more.
fn needle_document(s: &mut Scenario) {
    s.type_text("Alpha needle one.");
    s.enter();
    s.type_text("Beta Needle two NEEDLE.");
    s.enter();
    s.type_text("Gamma and needles, needless.");
    // A footnote at the end of the first paragraph.
    let main = s.main_flow();
    s.caret(main, 0, "Alpha needle one.".len(), 1, false);
    s.command_act(CommandId::Footnote);
    if s.app().page_selection.focus.flow == main {
        s.fail("Footnote", "the caret is not in the footnote");
    }
    s.type_text("needle in a footnote");
    // A frame, typed in.
    s.caret(main, 2, 0, 1, false);
    s.command_act(CommandId::Frame);
    s.edit("Create the frame", |rt| {
        rt.send(Message::CreateFrame(
            crate::page_dialogs::CreateFrameAction::Create,
        ));
    });
    // (The checker's Undo and Redo leave the new frame unselected.)
    let frame = *s.document().frame_stack().last().unwrap();
    s.no_change("select the frame", move |rt| {
        rt.send(Message::Frame(FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        }));
    });
    s.act("Enter: type in the frame", |rt| {
        rt.send(Message::Frame(FrameAction::EnterText { at: None }));
    });
    if s.app().page_selection.focus.flow == main {
        s.fail("Frame", "the caret is not in the frame's text");
    }
    s.type_text("needle in a frame");
    // The header.
    s.command_act(CommandId::GoToHeader);
    if s.app().page_selection.focus.flow == main {
        s.fail("Go To Header", "the caret is not in the header");
    }
    s.type_text("needle in the header");
    s.caret(main, 0, 0, 1, false);
}

#[test]
fn find_steps_through_every_stream_and_replace_all_is_one_undo_step() {
    let mut s = Scenario::new_document("find-streams");
    s.clean_folder();
    needle_document(&mut s);
    let main = s.main_flow();

    // Edit › Find…, the text typed in the bar: whole words only (Ami
    // Pro's default), so "needles" and "needless" are not found.
    s.command_act(CommandId::Find);
    if s.app().find.is_none() {
        s.fail("Find", "the bar did not open");
    }
    find(
        &mut s,
        "type needle",
        FindAction::Text(Field::Find, "needle".into()),
    );
    // Main: 3. Footnote, frame, header: 1 each.
    assert_eq!(bar_matches(&s), 6, "{}", bar_label(&s));
    assert_eq!(bar_label(&s), "6 matches");

    // Find Next walks the matches; each is selected and reads "needle".
    let mut flows = Vec::new();
    for round in 0..6 {
        s.no_change(&format!("Find Next {round}"), |rt| {
            rt.send(Message::Command(CommandId::FindNext));
        });
        assert_eq!(selected(&s).to_lowercase(), "needle", "round {round}");
        flows.push(s.app().page_selection.focus.flow);
    }
    flows.sort();
    flows.dedup();
    assert_eq!(flows.len(), 4, "main, footnote, frame, header: {flows:?}");
    assert!(bar_label(&s).starts_with("6 of 6") || bar_label(&s).contains(" of 6"));
    // Find Previous goes back.
    s.no_change("Find Previous", |rt| {
        rt.send(Message::Command(CommandId::FindPrevious));
    });
    assert_eq!(selected(&s).to_lowercase(), "needle");

    // Replace: Edit › Replace… shows the Replace row. (The stream the
    // caret is in when the bar opens starts the order: go to the main text
    // first.)
    s.caret(main, 0, 0, 1, false);
    s.command_act(CommandId::Replace);
    if !s.app().find.as_ref().unwrap().replacing {
        s.fail("Replace", "the Replace row is not shown");
    }
    find(
        &mut s,
        "type the replacement",
        FindAction::Text(Field::Replace, "pin".into()),
    );
    s.caret(main, 0, 0, 1, false);
    s.no_change("Find Next", |rt| {
        rt.send(Message::Command(CommandId::FindNext));
    });
    s.edit("Replace one", |rt| {
        rt.send(Message::Find(FindAction::ReplaceOne));
    });
    assert!(
        s.main_text().contains("Alpha pin one."),
        "{}",
        s.main_text()
    );
    assert_eq!(bar_matches(&s), 5);
    // Replace All: the other five in all four streams, one undo step (the
    // checker's Undo returns every stream).
    s.edit("Replace All", |rt| {
        rt.send(Message::Find(FindAction::ReplaceAll));
    });
    assert_eq!(bar_matches(&s), 0);
    assert!(
        s.main_text().contains("Beta Pin two PIN."),
        "capitals kept: {}",
        s.main_text()
    );
    assert!(
        s.main_text().contains("needles, needless"),
        "whole words only"
    );

    // Typing in the page while the bar is open updates the matches.
    find(
        &mut s,
        "type pin",
        FindAction::Text(Field::Find, "pin".into()),
    );
    let before = bar_matches(&s);
    s.caret_to_end();
    s.type_text(" pin");
    assert_eq!(bar_matches(&s), before + 1, "typing adds a match");
    let last = s.document().paragraphs().len() - 1;
    let end = s.document().paragraphs()[last].text.len();
    s.select_main((last, end - 4), (last, end));
    s.edit("Backspace over the typed word", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    assert_eq!(bar_matches(&s), before, "the match went with the word");

    // Closing the bar leaves the document alone.
    s.no_change("Close the bar", |rt| {
        rt.send(Message::Find(FindAction::Close));
    });
    assert!(s.app().find.is_none());

    let path = s.save_sam("FINDALL.SAM");
    let s = s.reopen(&path);
    assert!(s.main_text().contains("Alpha pin one."));
}

#[test]
fn find_options_selection_limit_recent_and_attributes() {
    let mut s = Scenario::new_document("find-options");
    s.clean_folder();
    s.type_text("The cat sat. The Cat sat. THE CAT sat. Concatenate the thecat.");
    s.enter();
    s.type_text("A cat in the second paragraph. The end.");
    s.enter();
    s.type_text("Last cat.");
    // Italic on one word.
    s.select_main((0, 4), (0, 7));
    s.command(CommandId::Italic);
    let main = s.main_flow();
    s.caret(main, 0, 0, 1, false);

    s.command_act(CommandId::Find);
    find(
        &mut s,
        "type cat",
        FindAction::Text(Field::Find, "cat".into()),
    );
    // Whole word, any case: cat, Cat, CAT, cat, cat = 5 ("Concatenate" and
    // "thecat" are not whole words).
    assert_eq!(bar_matches(&s), 5, "{}", bar_label(&s));
    find(
        &mut s,
        "Whole word off",
        FindAction::Menu(FindMenu::Toggle(Toggle::WholeWord)),
    );
    assert_eq!(bar_matches(&s), 7);
    find(
        &mut s,
        "Match case",
        FindAction::Menu(FindMenu::Toggle(Toggle::MatchCase)),
    );
    assert_eq!(bar_matches(&s), 5, "lower-case cat only");
    find(
        &mut s,
        "Match case off",
        FindAction::Menu(FindMenu::Toggle(Toggle::MatchCase)),
    );
    // Find attributes: Italic finds the one italic "cat" only.
    find(
        &mut s,
        "Italic",
        FindAction::Menu(FindMenu::FindAttribute(AttributeChoice::Italic)),
    );
    assert_eq!(bar_matches(&s), 1);
    find(
        &mut s,
        "Any attribute",
        FindAction::Menu(FindMenu::FindAttribute(AttributeChoice::Any)),
    );
    assert_eq!(bar_matches(&s), 7);
    find(
        &mut s,
        "Whole word on",
        FindAction::Menu(FindMenu::Toggle(Toggle::WholeWord)),
    );

    // Replace All keeping capitals, then without.
    s.command_act(CommandId::Replace);
    find(
        &mut s,
        "replacement dog",
        FindAction::Text(Field::Replace, "dog".into()),
    );
    s.edit("Replace All, capitals kept", |rt| {
        rt.send(Message::Find(FindAction::ReplaceAll));
    });
    assert!(
        s.main_text()
            .contains("The dog sat. The Dog sat. THE DOG sat."),
        "{}",
        s.main_text()
    );
    assert!(
        s.main_text().contains("Concatenate"),
        "inside words left alone"
    );
    s.act("Undo the replacement", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(s.main_text().contains("The cat sat."));

    // A selection limits the search ("In the selection").
    s.select_main((0, 0), (1, 6));
    s.command_act(CommandId::Find);
    let within = s.app().find.as_ref().unwrap().within;
    if within.is_none() {
        s.fail(
            "Find",
            "a selection across paragraphs did not limit the search",
        );
    }
    find(
        &mut s,
        "type cat",
        FindAction::Text(Field::Find, "cat".into()),
    );
    let limited = bar_matches(&s);
    find(
        &mut s,
        "In the selection off",
        FindAction::Menu(FindMenu::Toggle(Toggle::InSelection)),
    );
    assert!(
        bar_matches(&s) > limited,
        "{} vs {limited}",
        bar_matches(&s)
    );
    s.edit("Replace All everywhere", |rt| {
        rt.send(Message::Find(FindAction::Text(
            Field::Replace,
            "cow".into(),
        )));
        rt.send(Message::Find(FindAction::ReplaceAll));
    });

    // Recent searches: the menu fills the field.
    find(&mut s, "close", FindAction::Close);
    s.command_act(CommandId::Find);
    find(
        &mut s,
        "type the",
        FindAction::Text(Field::Find, "the".into()),
    );
    s.no_change("Find Next remembers the search", |rt| {
        rt.send(Message::Command(CommandId::FindNext));
    });
    find(&mut s, "close", FindAction::Close);
    s.command_act(CommandId::Find);
    find(
        &mut s,
        "clear",
        FindAction::Text(Field::Find, String::new()),
    );
    find(&mut s, "Recent 0", FindAction::Menu(FindMenu::Recent(0)));
    assert_eq!(s.app().find.as_ref().unwrap().find, "the");
    // Insert Return puts Ami Pro's paragraph mark in the field.
    find(&mut s, "focus", FindAction::Focus(Field::Find, true));
    find(
        &mut s,
        "Insert Return",
        FindAction::Menu(FindMenu::InsertReturn),
    );
    assert!(s.app().find.as_ref().unwrap().find.ends_with('¶'));
    find(&mut s, "Esc in the field", FindAction::Escape);
    assert!(s.app().find.is_none());

    let path = s.save_sam("FINDOPT.SAM");
    s.reopen(&path);
}

#[test]
fn find_with_nothing_to_find_and_typing_in_the_bar() {
    let mut s = Scenario::new_document("find-nothing");
    s.clean_folder();
    s.type_text("Nothing special here.");
    s.command_act(CommandId::Find);
    find(
        &mut s,
        "type zebra",
        FindAction::Text(Field::Find, "zebra".into()),
    );
    assert_eq!(bar_label(&s), "No matches");
    s.expect_status("no match");
    s.no_change("Find Next with no match", |rt| {
        rt.send(Message::Command(CommandId::FindNext));
    });
    s.no_change("Replace one with no match", |rt| {
        rt.send(Message::Find(FindAction::ReplaceOne));
    });
    s.no_change("Replace All with no match", |rt| {
        rt.send(Message::Find(FindAction::ReplaceAll));
    });
    s.clear_expected_status();
    // Keys typed while the Find field has the keyboard go to the bar, not
    // the page.
    find(&mut s, "focus Find", FindAction::Focus(Field::Find, true));
    find(
        &mut s,
        "type special",
        FindAction::Text(Field::Find, "special".into()),
    );
    assert_eq!(bar_matches(&s), 1);
    s.no_change("Enter in the field", |rt| {
        rt.send(Message::Find(FindAction::Submit(Field::Find)));
    });
    assert_eq!(selected(&s), "special");
    // Typing with the match selected and the bar's field not focused
    // replaces the selection.
    find(&mut s, "unfocus", FindAction::Focus(Field::Find, false));
    s.edit("type over the match", |rt| {
        rt.send(Message::Page(PageAction::Insert("unusual".into())));
    });
    assert_eq!(bar_label(&s), "No matches");
    assert!(s.main_text().contains("unusual"));
    let path = s.save_sam("FINDNONE.SAM");
    s.reopen(&path);
}

/// Replace All on an italic word: the replacement is italic as the word was
/// (the user would see a formatting loss otherwise).
#[test]
fn replacing_an_italic_word_keeps_it_italic() {
    let mut s = Scenario::new_document("find-italic");
    s.type_text("The cat sat.");
    s.select_main((0, 4), (0, 7));
    s.command(CommandId::Italic);
    assert!(
        s.document().paragraphs()[0]
            .runs
            .iter()
            .any(|run| run.format.italic == Some(true)),
        "the word is italic before"
    );
    let main = s.main_flow();
    s.caret(main, 0, 0, 1, false);
    s.command_act(CommandId::Replace);
    find(
        &mut s,
        "type cat",
        FindAction::Text(Field::Find, "cat".into()),
    );
    find(
        &mut s,
        "type dog",
        FindAction::Text(Field::Replace, "dog".into()),
    );
    s.edit("Replace All", |rt| {
        rt.send(Message::Find(FindAction::ReplaceAll));
    });
    assert_eq!(s.main_text(), "The dog sat.");
    let paragraph = &s.document().paragraphs()[0];
    assert!(
        paragraph
            .runs
            .iter()
            .any(|run| run.format.italic == Some(true) && run.start == 4 && run.end == 7),
        "the italic ran over the replaced word: {:?}",
        paragraph.runs
    );
}
