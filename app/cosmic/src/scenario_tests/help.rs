//! Help (F1, Contents, Search; `UI_SPEC.md` §45, P6.4): a user writes a
//! little, asks for help in several places, searches, follows links, goes
//! back, and returns to the document, which Help never changes.

use super::*;
use crate::help::Page;
use crate::help_panel::PanelAction;
use crate::table_commands::InsertTableAction;

fn help(s: &mut Scenario, what: &str, action: PanelAction) {
    s.no_change(what, move |rt| {
        rt.send(Message::HelpPanel(action));
    });
}

fn page(s: &Scenario) -> Page {
    s.app().help.nav.page.clone()
}

fn topic(s: &Scenario) -> String {
    match page(s) {
        Page::Topic(id) => id,
        other => s.fail("help", &format!("expected a topic, found {other:?}")),
    }
}

#[test]
fn a_writer_asks_for_help_while_working_and_goes_back_to_the_document() {
    let mut s = Scenario::new_document("help-while-working");
    s.type_text("Quarterly figures");
    let text = s.main_text();

    s.no_change("F1 with nothing special open", |rt| {
        rt.send(Message::Command(CommandId::Help));
    });
    assert_eq!(s.app().rail_panel(), RailPanel::Help);
    assert_eq!(page(&s), Page::Contents);

    // Search for a word, read a topic, follow a link out of it, and back.
    s.no_change("Help › Search Help…", |rt| {
        rt.send(Message::Command(CommandId::HelpSearch));
    });
    help(
        &mut s,
        "type: footnote",
        PanelAction::Query("footnote".into()),
    );
    assert_eq!(page(&s), Page::Search);
    help(
        &mut s,
        "open the first result",
        PanelAction::Open("notes-footnotes".into()),
    );
    assert_eq!(topic(&s), "notes-footnotes");
    help(
        &mut s,
        "follow See also",
        PanelAction::Open("fields".into()),
    );
    assert_eq!(topic(&s), "fields");
    help(&mut s, "Back", PanelAction::Back);
    assert_eq!(topic(&s), "notes-footnotes");
    help(&mut s, "Back to the results", PanelAction::Back);
    assert_eq!(page(&s), Page::Search);
    help(&mut s, "Forward", PanelAction::Forward);
    assert_eq!(topic(&s), "notes-footnotes");

    // A table: F1 in it opens the Tables topic.
    s.no_change("Esc closes Help", |rt| {
        rt.send(Message::Escape);
    });
    assert!(s.app().help_panel.is_none());
    s.act("Insert › Table…", |rt| {
        rt.send(Message::Command(CommandId::Table));
        rt.send(Message::InsertTable(InsertTableAction::Insert));
    });
    s.no_change("F1 in a table", |rt| {
        rt.send(Message::Command(CommandId::Help));
    });
    assert_eq!(topic(&s), "tables");

    // A panel's own help, then Check for Updates replaces Help (one panel).
    s.act("Tools › Sort…", |rt| {
        rt.send(Message::Command(CommandId::Sort));
    });
    s.no_change("F1 in the Sort panel", |rt| {
        rt.send(Message::Command(CommandId::Help));
    });
    assert_eq!(topic(&s), "tables");
    s.act("Help › Keyboard Shortcuts", |rt| {
        rt.send(Message::Command(CommandId::KeyboardShortcuts));
    });
    s.no_change("F1 over the shortcuts drawer", |rt| {
        rt.send(Message::Command(CommandId::Help));
    });
    assert_eq!(topic(&s), "keyboard-shortcuts");
    assert!(s.main_text().starts_with(&text), "the writing is untouched");
}
