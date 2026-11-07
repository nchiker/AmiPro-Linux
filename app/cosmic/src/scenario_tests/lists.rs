//! Bullets, numbering, and outline levels (`UI_SPEC.md` §21–22): Bullets
//! and Numbering as toggles, Enter ending a list, Insert › Bullet…, Promote
//! and Demote, a style's outline level, Outline Numbering…, typing in a
//! list, saving and reopening with the numbers shown.

use amipro_document::InlineObject;

use crate::page_dialogs::{BulletAction, NumberingScheme, OutlineNumberingAction};
use crate::style_dialogs::CreateAction;
use crate::style_properties::StyleAction;

use super::*;

fn style_name_of(s: &Scenario, index: usize) -> String {
    let document = s.document();
    document.styles()[&document.main_flow().paragraphs[index].style]
        .name
        .clone()
}

/// The numbers drawn before each paragraph.
fn numbers(s: &Scenario) -> Vec<Option<String>> {
    (0..s.document().paragraphs().len())
        .map(|index| drawn_number(s.app(), index))
        .collect()
}

fn n(text: &str) -> Option<String> {
    Some(text.to_owned())
}

#[test]
fn a_numbered_list_is_typed_nested_ended_with_enter_and_kept_through_a_save() {
    let mut s = Scenario::new_document("lists-numbered");
    s.clean_folder();
    s.type_text("Tasks for the day");
    s.enter();

    // Numbering on the empty paragraph makes the list style; typing and
    // Enter continue the list.
    s.command(CommandId::Numbering);
    assert_eq!(style_name_of(&s, 1), "Number List");
    assert_eq!(
        s.app().selected_list(),
        Some(amipro_editing::lists::ListKind::Numbering)
    );
    s.type_text("First task");
    s.enter();
    assert_eq!(
        style_name_of(&s, 2),
        "Number List",
        "Enter continues the list"
    );
    s.type_text("Second task");
    s.enter();
    s.type_text("Third task");
    assert_eq!(numbers(&s), [None, n("1."), n("2."), n("3.")]);

    // Demote the third: A. under 2; Promote brings it back.
    s.command(CommandId::Demote);
    assert_eq!(style_name_of(&s, 3), "List 2");
    assert_eq!(numbers(&s), [None, n("1."), n("2."), n("A.")]);
    s.command(CommandId::Promote);
    assert_eq!(numbers(&s), [None, n("1."), n("2."), n("3.")]);
    s.command(CommandId::Demote);
    s.command(CommandId::Demote);
    assert_eq!(numbers(&s)[3], n("i."));
    s.command(CommandId::Promote);
    s.command(CommandId::Promote);

    // The status bar and the toggles' states follow the caret.
    s.caret(s.main_flow(), 0, 2, 1, false);
    assert_eq!(s.app().selected_list(), None);
    s.caret(s.main_flow(), 2, 2, 1, false);
    assert_eq!(
        s.app().selected_list(),
        Some(amipro_editing::lists::ListKind::Numbering)
    );

    // Typing in the middle of an item keeps its number.
    s.type_text(" really");
    assert_eq!(numbers(&s), [None, n("1."), n("2."), n("3.")]);

    // Enter in an empty item ends the list, without a new paragraph.
    s.caret_to_end();
    s.enter();
    // (Promote after Demote leaves the item in List 1, which looks the same
    // as Number List; Enter gives the next item the same style.)
    assert_eq!(style_name_of(&s, 4), style_name_of(&s, 3));
    let count = s.document().paragraphs().len();
    s.act("Enter in the empty item", |rt| {
        rt.send(Message::Page(PageAction::ParagraphBreak));
    });
    assert_eq!(s.document().paragraphs().len(), count, "no new paragraph");
    assert_eq!(style_name_of(&s, 4), "Body Text");
    s.type_text("Back to plain text.");

    // Bullets: a second list; Bullets again turns it off.
    s.enter();
    s.command(CommandId::Bullets);
    assert_eq!(style_name_of(&s, 5), "Bullet 1");
    s.type_text("Apples");
    s.enter();
    s.type_text("Pears");
    s.enter();
    s.type_text("Plums");
    s.command(CommandId::Demote);
    assert_eq!(style_name_of(&s, 7), "Bullet 2", "bullets nest");
    s.command(CommandId::Promote);
    assert_eq!(style_name_of(&s, 7), "Bullet 1");
    s.command(CommandId::Bullets);
    assert_eq!(style_name_of(&s, 7), "Body Text");

    // Over a selection: Numbering numbers all the paragraphs it covers.
    s.select_main((5, 0), (7, 3));
    s.command(CommandId::Numbering);
    assert_eq!(numbers(&s)[5..], [n("1."), n("2."), n("3.")]);
    assert!(!s.app().page_selection.is_caret(), "the selection stays");

    // Undo is a step at a time, with the numbers following.
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(style_name_of(&s, 5), "Bullet 1");
    assert_eq!(style_name_of(&s, 6), "Bullet 1");
    s.act("Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });

    let shown = numbers(&s);
    let names: Vec<String> = (0..s.document().paragraphs().len())
        .map(|index| style_name_of(&s, index))
        .collect();
    let path = s.save_sam("TASKS.SAM");
    let s = s.reopen(&path);
    assert_eq!(numbers(&s), shown, "the numbers show again after reopening");
    let reopened: Vec<String> = (0..s.document().paragraphs().len())
        .map(|index| style_name_of(&s, index))
        .collect();
    assert_eq!(reopened, names);
}

#[test]
fn bullets_are_inserted_in_the_text_and_over_a_selection_through_the_dialog() {
    let mut s = Scenario::new_document("lists-insert-bullet");
    s.clean_folder();
    s.type_text("Price: 5 dollars");
    s.type_text(" per unit");
    let main = s.main_flow();

    // Insert › Bullet… at the caret: one step, the caret after it.
    s.caret(main, 0, 6, 1, false);
    s.act("Insert › Bullet…", |rt| {
        rt.send(Message::Command(CommandId::InsertBullet));
        assert!(cosmic::Application::dialog(&rt.app).is_some());
    });
    s.act("choose the second bullet", |rt| {
        rt.send(Message::InsertBullet(BulletAction::Choose(1)));
    });
    s.edit("Insert the bullet", |rt| {
        rt.send(Message::InsertBullet(BulletAction::Insert));
        assert!(rt.app.bullet_dialog.is_none());
    });
    let paragraph = &s.document().paragraphs()[0];
    assert_eq!(&paragraph.text[6..9], "\u{FFFC}");
    assert!(matches!(paragraph.objects[..], [InlineObject::Bullet(_)]));
    s.type_text("-");

    // The next time the dialog offers the last bullet; over a selection it
    // replaces the selection.
    s.select_main((0, 10), (0, 17));
    s.act("Insert › Bullet… over a selection", |rt| {
        rt.send(Message::Command(CommandId::InsertBullet));
    });
    s.act("choose another bullet", |rt| {
        rt.send(Message::InsertBullet(BulletAction::Choose(5)));
    });
    s.edit("Insert the bullet over the selection", |rt| {
        rt.send(Message::InsertBullet(BulletAction::Insert));
    });
    assert_eq!(
        s.document().paragraphs()[0]
            .objects
            .iter()
            .filter(|object| matches!(object, InlineObject::Bullet(_)))
            .count(),
        2
    );

    // Cancel changes nothing.
    s.no_change("Insert › Bullet… Cancel", |rt| {
        rt.send(Message::Command(CommandId::InsertBullet));
        rt.send(Message::InsertBullet(BulletAction::Cancel));
        assert!(rt.app.bullet_dialog.is_none());
    });

    // A bullet is deleted with Backspace in one step.
    s.caret(main, 0, 9, 1, false);
    s.edit("Backspace takes the bullet", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    assert_eq!(
        s.document().paragraphs()[0]
            .objects
            .iter()
            .filter(|object| matches!(object, InlineObject::Bullet(_)))
            .count(),
        1
    );

    let path = s.save_sam("INSBUL.SAM");
    let s = s.reopen(&path);
    assert_eq!(s.document().paragraphs()[0].objects.len(), 1);
}

#[test]
fn outline_levels_are_set_in_the_styles_then_numbered_by_a_scheme_and_kept() {
    let mut s = Scenario::new_document("lists-outline");
    s.clean_folder();
    s.type_text("Introduction");
    s.enter();
    s.type_text("Background");
    s.enter();
    s.type_text("Aims");
    s.enter();
    s.type_text("Method");
    s.enter();
    s.type_text("Sampling");
    s.enter();
    s.type_text("Some closing words.");

    // Two styles of the user's own with outline levels 1 and 2.
    let main = s.main_flow();
    for (name, level, paragraph) in [("Part", 1usize, 0usize), ("Sub", 2, 1)] {
        s.caret(main, paragraph, 0, 1, false);
        s.edit(&format!("Create Style {name:?}"), |rt| {
            rt.send(Message::Command(CommandId::CreateStyle));
            rt.send(Message::CreateStyle(CreateAction::Name(name.into())));
            rt.send(Message::CreateStyle(CreateAction::Create));
        });
        let id = *s
            .document()
            .styles()
            .iter()
            .find(|(_, style)| style.name == name)
            .unwrap()
            .0;
        s.edit(&format!("apply {name}"), move |rt| {
            rt.send(Message::ApplyStyle(id));
        });
        s.command_act(CommandId::BulletNumberStyle);
        assert_eq!(
            s.app().style_properties.as_ref().unwrap().style,
            id,
            "Bullet and Number Style… opens the caret style's panel"
        );
        s.edit(&format!("{name}: outline level {level}"), move |rt| {
            rt.send(Message::StyleProperties(StyleAction::OutlineLevel(level)));
        });
        assert_eq!(s.document().styles()[&id].bullets.level, level as u8);
    }
    // The other headings take the same styles.
    let part = *s
        .document()
        .styles()
        .iter()
        .find(|(_, st)| st.name == "Part")
        .unwrap()
        .0;
    let sub = *s
        .document()
        .styles()
        .iter()
        .find(|(_, st)| st.name == "Sub")
        .unwrap()
        .0;
    for (paragraph, style) in [(2usize, sub), (3, part), (4, sub)] {
        s.caret(main, paragraph, 0, 1, false);
        s.edit(
            &format!("apply a heading style to paragraph {paragraph}"),
            move |rt| {
                rt.send(Message::ApplyStyle(style));
            },
        );
    }

    // Outline Numbering… lists the levelled styles; Apply is one step each.
    s.act("Format › Styles ▸ Outline Numbering…", |rt| {
        rt.send(Message::Command(CommandId::OutlineNumbering));
        assert!(cosmic::Application::dialog(&rt.app).is_some());
    });
    let listed = s.app().outline_numbering.as_ref().unwrap().styles.clone();
    assert_eq!(listed, ["Part (1)", "Sub (2)"]);
    s.act("choose Ami Pro's 1, 1.1, 1.1.1", |rt| {
        rt.send(Message::OutlineNumbering(OutlineNumberingAction::Choose(
            NumberingScheme::Legal,
        )));
    });
    s.edit("Apply", |rt| {
        rt.send(Message::OutlineNumbering(OutlineNumberingAction::Apply));
        assert!(rt.app.outline_numbering.is_none());
    });
    assert_eq!(
        numbers(&s),
        [n("1"), n("1.1"), n("1.2"), n("2"), n("2.1"), None]
    );

    s.act("Outline Numbering… again", |rt| {
        rt.send(Message::Command(CommandId::OutlineNumbering));
        rt.send(Message::OutlineNumbering(OutlineNumberingAction::Choose(
            NumberingScheme::Outline,
        )));
    });
    s.edit("Apply Ami Pro's I, A, 1", |rt| {
        rt.send(Message::OutlineNumbering(OutlineNumberingAction::Apply));
    });
    assert_eq!(
        numbers(&s),
        [n("I."), n("A."), n("B."), n("II."), n("A."), None]
    );

    // Typing a new heading in the middle renumbers what follows.
    s.caret(main, 2, 4, 1, false);
    s.enter();
    s.type_text("Questions");
    assert_eq!(
        numbers(&s)[..4],
        [n("I."), n("A."), n("B."), n("C.")],
        "{:?}",
        numbers(&s)
    );
    assert_eq!(numbers(&s)[4], n("II."));

    // The lists scheme and Clear.
    s.act("Outline Numbering… Clear", |rt| {
        rt.send(Message::Command(CommandId::OutlineNumbering));
        rt.send(Message::OutlineNumbering(OutlineNumberingAction::Choose(
            NumberingScheme::Clear,
        )));
    });
    s.edit("Apply Clear", |rt| {
        rt.send(Message::OutlineNumbering(OutlineNumberingAction::Apply));
    });
    assert!(numbers(&s).iter().all(Option::is_none));
    s.act("Outline Numbering… Legal again", |rt| {
        rt.send(Message::Command(CommandId::OutlineNumbering));
        rt.send(Message::OutlineNumbering(OutlineNumberingAction::Choose(
            NumberingScheme::Legal,
        )));
    });
    s.edit("Apply Legal", |rt| {
        rt.send(Message::OutlineNumbering(OutlineNumberingAction::Apply));
    });
    let shown = numbers(&s);
    assert_eq!(shown[0], n("1"));

    let path = s.save_sam("OUTLINE.SAM");
    let s = s.reopen(&path);
    assert_eq!(numbers(&s), shown, "the numbers show again after reopening");
}
