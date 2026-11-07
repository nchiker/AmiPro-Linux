//! The merge in the core (`docs/research/MERGE_NOTES.md`, section 4;
//! decisions MG6, MG9, MG10): merge documents worked with each record of a
//! data file, by Ami Pro's rules, from the same inputs as VM fixtures
//! MGM1–MGM7.

use amipro_document::merge::{Condition, Join, Operator};
use amipro_document::{Document, FieldContext};
use amipro_editing::TextPosition;
use amipro_editing::field_language::NoHost;
use amipro_editing::merge::conditions::selected;
use amipro_editing::merge::data::{DataTable, read_ami_data};
use amipro_editing::merge::{MergeInputs, MergedRecord, merge_all, merge_one, source_position};
use amipro_formats::import_sam;

fn sam(edoc: &str) -> Vec<u8> {
    format!("[ver]\r\n\t4\r\n[edoc]\r\n{edoc}>\r\n\r\n").into_bytes()
}

fn read(edoc: &str) -> Document {
    import_sam(&sam(edoc), None, "Merge").unwrap().document
}

fn field(name: &str) -> String {
    format!("<:X12,-32768;MergeField {name}><<{name}<;><:X~12,-32768;MergeField {name}>")
}

fn power(code: &str) -> String {
    format!("<:X3,-32768;{code}><:X~3,-32768;{code}>")
}

/// MGD1's records.
fn data() -> DataTable {
    let document = read(concat!(
        "~|\r\n\r\n",
        "TITLE~FIRST~MIDDLE~LAST~COMPANY~CITY~AMOUNT~DUE|\r\n\r\n",
        "Mr.~Terry~J.~Allen~Vega Corp~Boston~150~3/15/94|\r\n\r\n",
        "Ms.~Pam~~Beck~~Seattle~75.50~1/2/94|\r\n\r\n",
        "Mr.~Bill~~smith~Smith & Co~Boston~$1,200.00~12/31/93|\r\n\r\n",
        "Ms.~Ann~Lee~Smith~~Dallas~(20)~2/28/94|\r\n\r\n",
        "Dr.~Joe~~Black~Black Ltd~boston~100~3/15/94|\r\n\r\n",
    ));
    read_ami_data(&document).unwrap().0
}

/// MGL1.
fn letter() -> Document {
    read(&format!(
        "Dear {} {}:\r\n\r\n{} {} {}\r\n\r\n{}\r\n\r\n{}, {}, USA\r\n\r\nRecord {}.\r\n\r\nEnd.\r\n\r\n",
        field("TITLE"),
        field("LAST"),
        field("FIRST"),
        field("MIDDLE"),
        field("LAST"),
        field("COMPANY"),
        field("CITY"),
        field("COMPANY"),
        power("MergeRec"),
    ))
}

fn texts(record: &MergedRecord) -> Vec<String> {
    record
        .document
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect()
}

fn run(document: &Document, conditions: &[Condition], tidy: bool) -> Vec<MergedRecord> {
    let table = data();
    let chosen = selected(conditions, &table);
    let context = FieldContext::default();
    let mut host = NoHost::default();
    let mut inputs = MergeInputs {
        context: &context,
        host: &mut host,
        tidy,
        in_place: false,
    };
    merge_all(document, &table, &chosen, &mut inputs)
}

fn condition(field: &str, operator: &str, value: &str, join: Join) -> Condition {
    Condition {
        field: field.into(),
        operator: Operator::from_symbol(operator).unwrap(),
        value: value.into(),
        join,
    }
}

#[test]
fn every_record_makes_a_document_with_its_values_mgm1() {
    let merged = run(&letter(), &[], false);
    assert_eq!(merged.len(), 5);
    assert_eq!(
        texts(&merged[0]),
        [
            "Dear Mr. Allen:",
            "Terry J. Allen",
            "Vega Corp",
            "Boston, Vega Corp, USA",
            "Record 1.",
            "End."
        ]
    );
    // Pam Beck has no COMPANY and no MIDDLE: the paragraph holding only the
    // empty field takes no space (Ami Pro's rule).
    assert_eq!(
        texts(&merged[1]),
        [
            "Dear Ms. Beck:",
            "Pam  Beck",
            "Seattle, , USA",
            "Record 2.",
            "End."
        ]
    );
    assert_eq!(merged[4].number, 5);
    // The fields stay fields with the record's results.
    assert!(
        !merged[0].document.main_flow().paragraphs[0]
            .marks
            .is_empty()
    );
}

#[test]
fn tidying_collapses_spaces_and_commas_an_empty_field_left_mg9() {
    let merged = run(&letter(), &[], true);
    assert_eq!(
        texts(&merged[1]),
        [
            "Dear Ms. Beck:",
            "Pam Beck",
            "Seattle, USA",
            "Record 2.",
            "End."
        ]
    );
    assert_eq!(texts(&merged[0])[1], "Terry J. Allen");
}

#[test]
fn conditions_choose_the_records_and_merge_rec_counts_the_chosen_mgm2_to_mgm5() {
    let merged = run(
        &letter(),
        &[condition("AMOUNT", ">", "100", Join::And)],
        false,
    );
    assert_eq!(merged.len(), 2);
    assert_eq!(texts(&merged[1])[0], "Dear Mr. smith:");
    assert_eq!(texts(&merged[1])[4], "Record 2.");
    let merged = run(
        &letter(),
        &[condition("DUE", ">=", "3/1/94", Join::And)],
        false,
    );
    assert_eq!(
        merged
            .iter()
            .map(|record| record.records[0])
            .collect::<Vec<_>>(),
        [0, 4]
    );
    let merged = run(
        &letter(),
        &[
            condition("CITY", "=", "Boston", Join::And),
            condition("AMOUNT", ">", "100", Join::Or),
            condition("LAST", "=", "Beck", Join::And),
        ],
        false,
    );
    assert_eq!(
        merged
            .iter()
            .map(|record| record.records[0])
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
}

#[test]
fn next_rec_puts_two_records_in_one_document_mgm6() {
    let document = read(&format!(
        "{} {}\r\n\r\n{}\r\n\r\n{} {}\r\n\r\nDone.\r\n\r\n",
        field("FIRST"),
        field("LAST"),
        power("NextRec"),
        field("FIRST"),
        field("LAST"),
    ));
    let merged = run(&document, &[], false);
    assert_eq!(merged.len(), 3);
    assert_eq!(texts(&merged[0])[0], "Terry Allen");
    assert_eq!(merged[0].records, [0, 1]);
    // A NextRec past the last record leaves the fields empty, and their
    // paragraph takes no space.
    // Ami Pro shows the NextRec paragraph without height too (MGM6).
    assert_eq!(texts(&merged[0]), ["Terry Allen", "Pam Beck", "Done."]);
    assert_eq!(texts(&merged[2]), ["Joe Black", "Done."]);
}

#[test]
fn skip_leaves_a_record_out_but_merge_rec_counts_it_mgs6_mgm7() {
    let skipping = |code: &str| {
        let document = read(&format!(
            "{}\r\n\r\n{} {}.\r\n\r\n",
            power(code),
            field("LAST"),
            power("MergeRec"),
        ));
        run(&document, &[], false)
            .iter()
            .map(|record| texts(record).last().unwrap().clone())
            .collect::<Vec<_>>()
    };
    // MGS6, MGS7: Beck left out; MergeRec counts it.
    let wanted = ["Allen 1.", "smith 3.", "Smith 4.", "Black 5."];
    assert_eq!(skipping("if LAST = \"Beck\" Skip endif"), wanted);
    assert_eq!(skipping("if MergeField LAST = \"Beck\" Skip endif"), wanted);
    // MGM7: a text ordered against a number holds for none (MC1).
    assert_eq!(skipping("if MergeField AMOUNT << 100 Skip endif").len(), 5);
}

#[test]
fn the_preview_works_one_record() {
    let table = data();
    let all: Vec<usize> = (0..table.records.len()).collect();
    let context = FieldContext::default();
    let mut host = NoHost::default();
    let mut inputs = MergeInputs {
        context: &context,
        host: &mut host,
        tidy: true,
        in_place: false,
    };
    let record = merge_one(&letter(), &table, &all, 3, &mut inputs).unwrap();
    assert_eq!(texts(&record)[0], "Dear Ms. Smith:");
    assert_eq!(texts(&record)[1], "Ann Lee Smith");
}

#[test]
fn merged_records_join_with_a_page_break_between() {
    let merged = run(
        &letter(),
        &[condition("AMOUNT", ">", "100", Join::And)],
        true,
    );
    let document = amipro_editing::merge::joined(&merged).unwrap();
    let paragraphs = &document.main_flow().paragraphs;
    let texts: Vec<&str> = paragraphs
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .collect();
    assert_eq!(texts.len(), 13, "{texts:?}");
    assert_eq!(texts[5], "End.");
    assert_eq!(
        paragraphs[6]
            .objects
            .iter()
            .filter(|object| matches!(object, amipro_document::InlineObject::Break { .. }))
            .count(),
        1
    );
    assert_eq!(texts[7], "Dear Mr. smith:");
    assert_eq!(texts[12], "End.");
}

/// MGM8: a data file without some of the letter's fields (MGT1's names).
#[test]
fn fields_the_data_file_lacks_come_out_empty_with_ami_pros_message_mgm8() {
    let letter = letter();
    assert_eq!(
        amipro_editing::merge::merge_field_names(&letter),
        ["TITLE", "LAST", "FIRST", "MIDDLE", "COMPANY", "CITY"]
    );
    let table = DataTable {
        names: vec!["First".into(), "Last".into(), "City".into()],
        records: vec![vec!["Terry".into(), "Allen".into(), "Boston".into()]],
    };
    assert_eq!(
        amipro_editing::merge::unknown_fields(&letter, &table),
        ["TITLE", "MIDDLE", "COMPANY"]
    );
    let context = FieldContext::default();
    let mut host = NoHost::default();
    let mut inputs = MergeInputs {
        context: &context,
        host: &mut host,
        tidy: false,
        in_place: false,
    };
    let merged = merge_all(&letter, &table, &[0], &mut inputs);
    assert_eq!(texts(&merged[0])[0], "Dear  Allen:");
    // The company line held only an empty field: left out (MC2).
    assert_eq!(texts(&merged[0])[2], "Boston, , USA");
    assert!(
        host.warnings
            .iter()
            .any(|warning| warning == "TITLE is not a valid field name."),
        "{:?}",
        host.warnings
    );
}

/// Merge, view & print works in place: nothing is left out, so an edit
/// made while viewing carries to the next record.
#[test]
fn in_place_keeps_every_paragraph_and_carries_edits() {
    let table = data();
    let all: Vec<usize> = (0..table.records.len()).collect();
    let context = FieldContext::default();
    let mut host = NoHost::default();
    let mut inputs = MergeInputs {
        context: &context,
        host: &mut host,
        tidy: true,
        in_place: true,
    };
    let letter = letter();
    let first = merge_one(&letter, &table, &all, 1, &mut inputs).unwrap();
    // Pam Beck has no company: the paragraph stays, empty.
    assert_eq!(texts(&first).len(), letter.main_flow().paragraphs.len());
    assert_eq!(texts(&first)[2], "");
    let mut edited = first.document.clone();
    let end = edited.main_flow().paragraphs.len() - 1;
    let main = edited.main_flow_id();
    edited = amipro_editing::apply(
        &edited,
        &amipro_editing::Command::ReplaceText {
            start: amipro_editing::TextPosition {
                flow: main,
                paragraph: end,
                byte: 0,
            },
            end: amipro_editing::TextPosition {
                flow: main,
                paragraph: end,
                byte: 3,
            },
            text: "Fin".into(),
        },
    )
    .unwrap()
    .document;
    let next = merge_one(&edited, &table, &all, 2, &mut inputs).unwrap();
    assert_eq!(texts(&next)[0], "Dear Mr. smith:");
    assert_eq!(texts(&next)[end], "Fin.");
}

/// Two fields put in one after the other, as the Merge panel's field
/// buttons are clicked: the second follows the first, not inside it.
#[test]
fn a_field_put_in_right_after_another_follows_it() {
    use amipro_editing::fields::{FieldInputs, insert_power_field};
    let mut document = Document::from_plain_text("Dear ");
    let context = FieldContext::default();
    let mut host = NoHost::default();
    let mut variables = std::collections::BTreeMap::new();
    let main = document.main_flow_id();
    let mut at = amipro_editing::TextPosition {
        flow: main,
        paragraph: 0,
        byte: 5,
    };
    for name in ["TITLE", "LAST", "MergeRec"] {
        let code = if name == "MergeRec" {
            name.to_string()
        } else {
            format!("MergeField {name}")
        };
        let mut inputs = FieldInputs {
            context: &context,
            variables: &mut variables,
            host: &mut host,
        };
        let command = insert_power_field(&document, at, &code, 0, &mut inputs).unwrap();
        let applied = amipro_editing::apply(&document, &command).unwrap();
        at = applied.caret.unwrap();
        document = applied.document;
    }
    let spans =
        amipro_document::power_fields(document.main_flow().paragraphs.iter().map(AsRef::as_ref));
    assert_eq!(spans.len(), 3);
    for pair in spans.windows(2) {
        let end = pair[0].end.expect("an end mark");
        assert!(
            (end.byte, end.mark) < (pair[1].start.byte, pair[1].start.mark),
            "{spans:#?}"
        );
    }
    let table = DataTable {
        names: vec!["Title".into(), "Last".into()],
        records: vec![vec!["Ms.".into(), "Beck".into()]],
    };
    let mut inputs = MergeInputs {
        context: &context,
        host: &mut host,
        tidy: false,
        in_place: false,
    };
    let merged = merge_all(&document, &table, &[0], &mut inputs);
    assert_eq!(texts(&merged[0])[0], "Dear Ms.Beck1");
}

/// A place in the preview (a record merged) maps back to the letter: text
/// between fields to the same text, a place in a value to after its field,
/// paragraphs by id (the user's second use, 2026-11-04).
#[test]
fn a_place_in_the_preview_maps_back_to_the_letter() {
    let letter = letter();
    let table = data();
    let context = FieldContext::default();
    let mut host = NoHost::default();
    let mut inputs = MergeInputs {
        context: &context,
        host: &mut host,
        tidy: true,
        in_place: false,
    };
    let chosen: Vec<usize> = (0..table.records.len()).collect();
    // Ms. Pam Beck: no middle name, no company (that paragraph drops).
    let shown = merge_one(&letter, &table, &chosen, 1, &mut inputs)
        .unwrap()
        .document;
    let flow = letter.main_flow_id();
    let at = |paragraph, byte| TextPosition {
        flow,
        paragraph,
        byte,
    };
    let texts = |document: &Document| -> Vec<String> {
        document
            .main_flow()
            .paragraphs
            .iter()
            .map(|p| p.text.clone())
            .collect()
    };
    assert_eq!(texts(&shown)[0], "Dear Ms. Beck:");
    let source = texts(&letter);
    assert_eq!(source[0], "Dear <TITLE> <LAST>:");
    let mapped = |paragraph, byte| source_position(&letter, &shown, at(paragraph, byte));
    // Before the first field, between fields, after the last.
    assert_eq!(mapped(0, 0), at(0, 0));
    assert_eq!(mapped(0, 5), at(0, 5));
    assert_eq!(mapped(0, 8), at(0, 12));
    assert_eq!(mapped(0, 9), at(0, 13));
    assert_eq!(mapped(0, 14), at(0, 20));
    // In a value: after its field.
    assert_eq!(mapped(0, 6), at(0, 12));
    assert_eq!(mapped(0, 11), at(0, 19));
    // The company's paragraph dropped from the preview: the next one maps
    // to its own place in the letter.
    let shown_texts = texts(&shown);
    let city = shown_texts
        .iter()
        .position(|text| text.starts_with("Seattle"))
        .unwrap();
    let letter_city = source
        .iter()
        .position(|text| text.starts_with("<CITY>"))
        .unwrap();
    assert_ne!(city, letter_city);
    let end = shown_texts[city].len();
    assert_eq!(
        mapped(city, end),
        at(letter_city, source[letter_city].len())
    );
}
