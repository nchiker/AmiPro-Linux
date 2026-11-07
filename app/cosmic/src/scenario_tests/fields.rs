//! Fields (`UI_SPEC.md` §28): dates, times, Doc Info, and power fields
//! inserted, updated, shown as codes, edited, locked, typed around, taken
//! away with Backspace and Delete, saved and reopened.

use super::*;
use crate::document_properties::{PropertiesAction, Text};
use crate::fields_panel::{FieldsAction, Page};
use amipro_document::InlineObject;

/// A power field of the main text: its paragraph, the byte range of its
/// result (or of its code while codes show), and its code.
#[derive(Clone, Debug)]
struct Power {
    paragraph: usize,
    from: usize,
    to: usize,
    code: String,
}

fn power_in_main(s: &Scenario) -> Vec<Power> {
    let flow = s.document().flow(s.main_flow()).unwrap();
    power_fields(flow.paragraphs.iter().map(AsRef::as_ref))
        .into_iter()
        .map(|field| {
            let end = field.end.unwrap_or(field.start);
            Power {
                paragraph: field.start.paragraph,
                from: field.start.byte,
                to: end.byte,
                code: field.mark.code.clone(),
            }
        })
        .collect()
}

/// The text of the field `index` (its result).
fn result_of(s: &Scenario, index: usize) -> String {
    let fields = power_in_main(s);
    let field = &fields[index];
    s.document().paragraphs()[field.paragraph].text[field.from..field.to].to_owned()
}

fn objects_in_main(s: &Scenario) -> Vec<InlineObject> {
    s.document()
        .paragraphs()
        .iter()
        .flat_map(|paragraph| paragraph.objects.iter().cloned())
        .collect()
}

fn fields_action(s: &mut Scenario, what: &str, action: FieldsAction) {
    s.act(what, move |rt| {
        rt.send(Message::FieldsPanel(action));
    });
}

/// Insert › Field… with `code` typed in the panel, as an editing step.
fn insert_power(s: &mut Scenario, code: &str) {
    s.command_act(CommandId::Field);
    fields_action(s, "Page: Power field", FieldsAction::Page(Page::Power));
    fields_action(s, &format!("code {code}"), FieldsAction::Code(code.into()));
    s.edit(&format!("Insert the field {code}"), |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::Insert));
    });
}

fn caret_at(s: &mut Scenario, paragraph: usize, byte: usize) {
    let main = s.main_flow();
    s.caret(main, paragraph, byte, 1, false);
}

fn text(s: &Scenario) -> String {
    s.main_text()
}

#[test]
fn dates_times_and_doc_info_inserted_typed_around_and_removed() {
    let mut s = Scenario::new_document("fields-objects");
    s.clean_folder();
    s.type_text("Dated: , by , done.");

    // Insert › Date/Time…: the system date, format b, after "Dated: ".
    caret_at(&mut s, 0, "Dated: ".len());
    s.command_act(CommandId::DateTime);
    if s.app().fields_panel.is_none() {
        s.fail("Date/Time", "the panel did not open");
    }
    fields_action(&mut s, "System date", FieldsAction::DateKind(1));
    fields_action(&mut s, "format b", FieldsAction::DateFormat(1));
    s.edit("Insert the date", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::Insert));
    });
    assert!(matches!(objects_in_main(&s)[0], InlineObject::Date(_)));

    // Today's date as text is plain text.
    caret_at(&mut s, 0, 0);
    fields_action(&mut s, "Today's date", FieldsAction::DateKind(0));
    let before = text(&s);
    s.edit("Insert today's date as text", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::Insert));
    });
    assert!(text(&s).len() > before.len());
    assert_eq!(objects_in_main(&s).len(), 1, "still one field object");

    // The time (System time) at the end of the paragraph.
    s.caret_to_end();
    fields_action(&mut s, "System time", FieldsAction::DateKind(2));
    s.edit("Insert the time", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::Insert));
    });
    assert_eq!(objects_in_main(&s).len(), 2);

    // Doc Info entries.
    let by = text(&s).find("by ").unwrap() + 3;
    caret_at(&mut s, 0, by);
    fields_action(&mut s, "Page: Doc Info", FieldsAction::Page(Page::DocInfo));
    fields_action(&mut s, "entry 0", FieldsAction::DocEntry(0));
    s.edit("Insert a Doc Info field", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::Insert));
    });
    assert_eq!(objects_in_main(&s).len(), 3);

    // Type right before and right after each object.
    let objects: Vec<usize> = s.document().paragraphs()[0]
        .text
        .match_indices('\u{FFFC}')
        .map(|(at, _)| at)
        .collect();
    assert_eq!(objects.len(), 3);
    for (round, at) in objects.iter().rev().enumerate() {
        // From the last to the first so earlier offsets stay valid.
        caret_at(&mut s, 0, at + '\u{FFFC}'.len_utf8());
        s.type_text(&format!("<after{round}>"));
        caret_at(&mut s, 0, *at);
        s.type_text(&format!("<before{round}>"));
    }
    assert!(text(&s).contains("<before0>"), "{}", text(&s));

    // Backspace just after an object and Delete just before one.
    let at = s.document().paragraphs()[0].text.find('\u{FFFC}').unwrap();
    caret_at(&mut s, 0, at + '\u{FFFC}'.len_utf8());
    let count = objects_in_main(&s).len();
    s.edit("Backspace just after a date", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    assert_eq!(objects_in_main(&s).len(), count - 1, "the field went whole");
    let at = s.document().paragraphs()[0].text.find('\u{FFFC}').unwrap();
    caret_at(&mut s, 0, at);
    s.edit("Delete just before a field", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert_eq!(objects_in_main(&s).len(), count - 2);

    // A selection holding the remaining field is deleted with it.
    let at = s.document().paragraphs()[0].text.find('\u{FFFC}').unwrap();
    s.select_main((0, at.saturating_sub(2)), (0, at + 3 + 2));
    s.edit("Delete a selection holding a field", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert!(objects_in_main(&s).is_empty());

    // Doc Info shows what Document Properties holds.
    s.command_act(CommandId::DocumentProperties);
    s.act("Set the description", |rt| {
        rt.send(Message::DocumentProperties(PropertiesAction::Text(
            Text::Description,
            "Quarterly report".into(),
        )));
        rt.send(Message::DocumentProperties(PropertiesAction::Commit));
    });
    s.caret_to_end();
    s.command_act(CommandId::Field);
    fields_action(&mut s, "Page: Doc Info", FieldsAction::Page(Page::DocInfo));
    fields_action(&mut s, "Document Description", FieldsAction::DocEntry(3));
    s.edit("Insert the description", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::Insert));
    });
    assert_eq!(power_in_main(&s).len(), 1);
    assert_eq!(result_of(&s, 0), "Quarterly report");

    // The new description reaches the field on Update All.
    s.command_act(CommandId::DocumentProperties);
    s.act("change the description", |rt| {
        rt.send(Message::DocumentProperties(PropertiesAction::Text(
            Text::Description,
            "Annual report".into(),
        )));
        rt.send(Message::DocumentProperties(PropertiesAction::Commit));
    });
    s.edit("Update All Fields", |rt| {
        rt.send(Message::Command(CommandId::UpdateAllFields));
    });
    assert_eq!(result_of(&s, 0), "Annual report");

    s.type_text(" end");
    let path = s.save_sam("OBJECTS.SAM");
    let s = s.reopen(&path);
    assert_eq!(power_in_main(&s).len(), 1);
}

#[test]
fn power_fields_inserted_one_by_one_and_typed_around() {
    let mut s = Scenario::new_document("fields-power");
    s.clean_folder();
    s.type_text("Fields: ");
    // Set a description so the Description field has a result.
    s.command_act(CommandId::DocumentProperties);
    s.act("Set the description", |rt| {
        rt.send(Message::DocumentProperties(PropertiesAction::Text(
            Text::Description,
            "Desc".into(),
        )));
        rt.send(Message::DocumentProperties(PropertiesAction::Commit));
    });
    let codes = [
        "NumPages",
        "NumWords",
        "NumChars",
        "2+5",
        "UCase$(\"abc\")",
        "LCase$(\"ABC\")",
        "Len(\"abcd\")",
        "StrCat$(\"a\",\"b\")",
        "Left$(\"hello\",2)",
        "Mid$(\"hello\",2,3)",
        "Right$(\"hello\",2)",
        "Seq Fig",
        "Seq Fig",
        "Description",
        "CreateDate",
        "EditDate",
        "Now()",
        "FileSize",
        "NumEdits",
        "TotalEditingTime",
    ];
    for code in codes {
        s.caret_to_end();
        insert_power(&mut s, code);
        let index = power_in_main(&s).len() - 1;
        let result = result_of(&s, index);
        if result.is_empty() {
            s.fail(code, &format!("{code} shows an empty result"));
        }
        // Text right after the field, as typed by the user.
        s.caret_to_end();
        s.type_text(" | ");
    }
    assert_eq!(
        power_in_main(&s).len(),
        codes.len() - 2,
        "the two Seq fields are objects"
    );
    assert_eq!(objects_in_main(&s).len(), 2);
    // A few results worked out.
    let find = |s: &Scenario, code: &str| {
        power_in_main(s)
            .iter()
            .position(|field| field.code.starts_with(code))
            .unwrap_or_else(|| s.fail(code, "field not found"))
    };
    assert_eq!(result_of(&s, find(&s, "2+5")), "7");
    assert_eq!(result_of(&s, find(&s, "UCase$")), "ABC");
    assert_eq!(result_of(&s, find(&s, "Len(")), "4");
    assert_eq!(result_of(&s, find(&s, "StrCat$")), "ab");
    assert_eq!(result_of(&s, find(&s, "Left$")), "he");
    assert_eq!(result_of(&s, find(&s, "Description")), "Desc");

    // Typing right before and right after a field, and inside its result.
    let first = power_in_main(&s)[0].clone();
    caret_at(&mut s, 0, first.from);
    s.type_text("[");
    let first = power_in_main(&s)[0].clone();
    caret_at(&mut s, 0, first.to);
    s.type_text("]");
    let text_now = text(&s);
    assert!(text_now.contains('['), "{text_now}");
    let fields_now = power_in_main(&s);
    assert_eq!(
        fields_now.len(),
        codes.len() - 2,
        "typing at the edges keeps every field"
    );
    // None of them has gone empty or nested (the checker also asserts it).

    // Update, Update All, Next and Previous Field.
    s.act("Update Fields", |rt| {
        rt.send(Message::Command(CommandId::UpdateFields));
    });
    s.act("Update All Fields", |rt| {
        rt.send(Message::Command(CommandId::UpdateAllFields));
    });
    caret_at(&mut s, 0, 0);
    s.no_change("Next Field", |rt| {
        rt.send(Message::Command(CommandId::NextField));
    });
    let selected = s.app().page_selection.ordered();
    let first = &power_in_main(&s)[0];
    assert_eq!(
        (selected.0.byte, selected.1.byte),
        (first.from, first.to),
        "Next Field selects the whole field"
    );
    s.no_change("Next Field again", |rt| {
        rt.send(Message::Command(CommandId::NextField));
    });
    let second = &power_in_main(&s)[1];
    assert_eq!(s.app().page_selection.ordered().0.byte, second.from);
    s.no_change("Previous Field", |rt| {
        rt.send(Message::Command(CommandId::PreviousField));
    });
    assert_eq!(s.app().page_selection.ordered().0.byte, first.from);

    let path = s.save_sam("POWER.SAM");
    let s = s.reopen(&path);
    assert_eq!(
        power_in_main(&s).len(),
        codes.len() - 2,
        "the two Seq fields are objects"
    );
    assert_eq!(objects_in_main(&s).len(), 2);
}

#[test]
fn power_fields_shown_as_codes_typed_over_updated_edited_and_locked() {
    let mut s = Scenario::new_document("fields-codes");
    s.clean_folder();
    s.type_text("Sum: , words: .");
    caret_at(&mut s, 0, "Sum: ".len());
    insert_power(&mut s, "2+5");
    assert_eq!(result_of(&s, 0), "7");
    let after = text(&s).find(", words: ").unwrap() + ", words: ".len();
    caret_at(&mut s, 0, after);
    insert_power(&mut s, "NumWords");

    // Lock Field with the caret in the first field: checked.
    let field = power_in_main(&s)[0].clone();
    caret_at(&mut s, 0, field.from + 1);
    s.edit("Lock Field", |rt| {
        rt.send(Message::Command(CommandId::LockField));
    });
    if menu_check(s.app(), CommandId::LockField) != Some(true) {
        s.fail("Lock Field", "the check mark is not on");
    }
    // Show Power Fields: the codes in the text.
    s.edit("Show Power Fields", |rt| {
        rt.send(Message::Command(CommandId::ShowPowerFields));
    });
    assert!(s.app().codes_shown());
    assert!(text(&s).contains("<2+5>"), "{}", text(&s));
    assert!(text(&s).contains("<NumWords>"), "{}", text(&s));
    // Type over the code of the first field: a new sum.
    let code_at = text(&s).find("<2+5>").unwrap();
    s.select_main((0, code_at + 1), (0, code_at + 4));
    s.edit("type over the code", |rt| {
        rt.send(Message::Page(PageAction::Insert("3*4".into())));
    });
    assert!(text(&s).contains("<3*4>"), "{}", text(&s));
    // Update with the field locked leaves it; unlocked, it works out 12.
    s.act("Update All with codes showing", |rt| {
        rt.send(Message::Command(CommandId::UpdateAllFields));
    });
    assert!(!s.app().codes_shown(), "Update All switches the codes off");
    s.act("Show Power Fields", |rt| {
        rt.send(Message::Command(CommandId::ShowPowerFields));
    });
    s.act("Show Power Fields off", |rt| {
        rt.send(Message::Command(CommandId::ShowPowerFields));
    });
    let results = (result_of(&s, 0), result_of(&s, 1));
    eprintln!("results after the code was typed over: {results:?}");

    // Edit Field… in the Fields panel: the code of the field at the caret.
    let field = power_in_main(&s)[0].clone();
    caret_at(&mut s, 0, field.from);
    s.command_act(CommandId::EditField);
    if s.app().fields_panel.is_none() {
        s.fail("Edit Field", "the panel did not open");
    }
    s.act("type a new code", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::ThisCode("6*7".into())));
    });
    s.edit("submit the new code", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::ThisSubmit));
    });
    s.act("Update this field", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::ThisUpdate));
    });
    // The lock keeps the old result; take it off and update.
    s.edit("Lock off in the panel", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::ThisLock(false)));
    });
    s.act("Update this field again", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::ThisUpdate));
    });
    assert_eq!(result_of(&s, 0), "42", "{}", text(&s));
    s.edit("Auto run on", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::ThisAutoRun(true)));
    });

    // Backspace after the first field takes it whole.
    let field = power_in_main(&s)[0].clone();
    caret_at(&mut s, 0, field.to);
    let count = power_in_main(&s).len();
    s.edit("Backspace after a field", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    assert_eq!(power_in_main(&s).len(), count - 1);
    let field = power_in_main(&s)[0].clone();
    caret_at(&mut s, 0, field.from);
    s.edit("Delete before a field", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert!(power_in_main(&s).is_empty());
    s.type_text("!");

    let path = s.save_sam("CODES.SAM");
    s.reopen(&path);
}

#[test]
fn fields_in_a_paragraph_are_removed_by_a_selection_and_typed_around_with_codes_shown() {
    let mut s = Scenario::new_document("fields-selection");
    s.clean_folder();
    s.type_text("A ");
    insert_power(&mut s, "NumWords");
    s.caret_to_end();
    s.type_text(" B ");
    insert_power(&mut s, "NumPages");
    s.caret_to_end();
    s.type_text(" C");
    s.enter();
    s.type_text("Second ");
    insert_power(&mut s, "NumChars");
    s.caret_to_end();
    s.type_text(" end");
    assert_eq!(power_in_main(&s).len(), 3);

    // Two fields selected across a line break and cut away.
    let first = power_in_main(&s)[0].clone();
    let main = s.main_flow();
    s.caret(main, 0, first.from, 1, false);
    let last = power_in_main(&s)[2].clone();
    s.caret(main, 1, last.to, 1, true);
    s.edit("Delete a selection holding three fields", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert!(power_in_main(&s).is_empty(), "{:?}", text(&s));

    // Type over a selected field's result; the field goes with it.
    s.type_text(" X ");
    insert_power(&mut s, "NumWords");
    let field = power_in_main(&s)[0].clone();
    s.select_main((field.paragraph, field.from), (field.paragraph, field.to));
    s.edit("type over a selected field", |rt| {
        rt.send(Message::Page(PageAction::Insert("typed".into())));
    });
    assert!(power_in_main(&s).is_empty());
    assert!(text(&s).contains("typed"));

    // With codes shown, typing at the code's edges stays outside the code.
    insert_power(&mut s, "NumWords");
    s.edit("Show Power Fields", |rt| {
        rt.send(Message::Command(CommandId::ShowPowerFields));
    });
    let code_at = text(&s).find("<NumWords>").unwrap();
    caret_at(&mut s, 0, code_at);
    s.type_text("(");
    let after_code = text(&s).find("<NumWords>").unwrap() + "<NumWords>".len();
    caret_at(&mut s, 0, after_code);
    s.type_text(")");
    s.edit("Show Power Fields off", |rt| {
        rt.send(Message::Command(CommandId::ShowPowerFields));
    });
    assert!(
        text(&s).contains('(') && text(&s).contains(')'),
        "{}",
        text(&s)
    );
    assert_eq!(power_in_main(&s).len(), 1);

    let path = s.save_sam("SELECT.SAM");
    s.reopen(&path);
}

/// A field whose result is empty shows nothing; the user can neither see
/// nor select it. A Description field in a document with no description:
/// Ami Pro shows nothing, and so does the app, so the editing keys have to
/// take it along with its neighbours (the lesson of the merge fields).
#[test]
fn an_empty_description_field_is_not_left_invisible() {
    let mut s = Scenario::new_document("fields-empty");
    s.clean_folder();
    s.type_text("Description: ");
    s.caret_to_end();
    s.command_act(CommandId::Field);
    fields_action(&mut s, "Page: Doc Info", FieldsAction::Page(Page::DocInfo));
    fields_action(&mut s, "Document Description", FieldsAction::DocEntry(3));
    // A field whose result is empty would be invisible: it is not put in,
    // and the status bar says why.
    s.no_change("Insert the description field", |rt| {
        rt.send(Message::FieldsPanel(FieldsAction::Insert));
    });
    assert!(
        s.app().status.starts_with("Not inserted"),
        "{}",
        s.app().status
    );
    s.type_text("!");
}

/// Codes whose result is empty (a function of nothing): the app drops the
/// field (an empty one would be invisible) but its status bar still says
/// "Power field inserted". The status must tell the truth: a field in the
/// text, or a message that none was inserted.
#[test]
fn a_field_with_an_empty_result_is_either_inserted_or_reported() {
    for code in ["Left$(\"abc\",0)", "UCase$(\"\")", "StrCat$(\"\",\"\")"] {
        let mut s = Scenario::new_document("fields-empty-codes");
        s.type_text("Before ");
        s.caret_to_end();
        s.command_act(CommandId::Field);
        fields_action(&mut s, "Page: Power field", FieldsAction::Page(Page::Power));
        fields_action(&mut s, "code", FieldsAction::Code(code.into()));
        s.act(&format!("Insert {code}"), |rt| {
            rt.send(Message::FieldsPanel(FieldsAction::Insert));
        });
        let inserted = !power_in_main(&s).is_empty();
        let status = s.app().status.clone();
        if !inserted
            && status.to_lowercase().contains("inserted")
            && !status.to_lowercase().contains("not")
        {
            s.fail(
                code,
                &format!("no field is in the text but the status says {status:?}"),
            );
        }
    }
}
