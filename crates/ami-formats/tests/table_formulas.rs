//! Calculated table cells (run C, TD.3): a cell record with F3 bit `0x40`
//! has two bodies, the result and then the formula as typed, both with the
//! result's style tag and alignment. The model keeps the formula text in
//! `TableCell::formula`; what it cannot read stays as legacy bodies.

use std::path::PathBuf;

use amipro_document::formula::{Dims, Formula, Options};
use amipro_document::{CellContent, Document, FrameContent, NumberFormat, TableCell};
use amipro_formats::{SamImport, SamOptions, import_sam, sam_document, sam_losses};

fn fixture() -> String {
    // The synthetic fixture's calculated cell, with the consistent prefix
    // Ami Pro writes on both bodies.
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages-and-frames.sam"),
    )
    .unwrap()
    .replace("<+A>a1*2", "@Heading@<+A>@Sum(A1..A1)+a1*2")
}

fn open(sam: &str) -> Document {
    let SamImport { document, .. } = import_sam(sam.as_bytes(), None, "Fixture").unwrap();
    document
}

fn tables(document: &Document) -> Vec<&amipro_document::Table> {
    document
        .frames()
        .filter_map(|frame| match &frame.content {
            FrameContent::Table(table) => Some(table),
            _ => None,
        })
        .collect()
}

fn text(document: &Document, cell: &TableCell) -> String {
    let CellContent::Text(flow) = cell.content;
    document.flow(flow).unwrap().plain_text()
}

fn write(document: &Document) -> String {
    String::from_utf8_lossy(&sam_document(document, &SamOptions::at(1_791_958_088))).into_owned()
}

#[test]
fn a_calculated_cell_reads_its_formula_as_typed() {
    let document = open(&fixture());
    let table = tables(&document)[0];
    let cell = &table.cells[2];
    assert_eq!(text(&document, cell), "3250.00");
    assert_eq!(cell.formula.as_deref(), Some("@Sum(A1..A1)+a1*2"));
    assert!(cell.legacy.get("cell.body").is_none(), "nothing else kept");
    assert!(
        table
            .cells
            .iter()
            .filter(|cell| cell.formula.is_some())
            .count()
            == 1
    );
}

#[test]
fn the_second_body_is_written_back_as_read() {
    let sam = fixture();
    let document = open(&sam);
    let written = write(&document);
    assert!(written.contains(
        "\r\n@Heading@<+A>3250.00\r\n\r\n>\r\n@Heading@<+A>@Sum(A1..A1)+a1*2\r\n\r\n>\r\n"
    ));
    let again = open(&written);
    assert_eq!(
        tables(&again)[0].cells[2].formula,
        tables(&document)[0].cells[2].formula
    );
    // Stable on a second pass but for the editing-time counter.
    let (a, b) = (write(&again), written.clone());
    let body =
        |text: &str| text[text.find("[tbl]").unwrap()..text.find("[tble]").unwrap()].to_owned();
    assert_eq!(body(&a), body(&b));
}

#[test]
fn a_body_that_is_not_a_formula_is_kept_not_dropped() {
    // The synthetic fixture's second body has another prefix than the
    // result's: the model does not read it, the writer keeps it, and the
    // record keeps its formula bit.
    let sam = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages-and-frames.sam"),
    )
    .unwrap();
    let document = open(&sam);
    let cell = &tables(&document)[0].cells[2];
    assert_eq!(cell.formula, None);
    assert_eq!(cell.legacy.get("cell.body"), Some("<+A>a1*2"));
    let written = write(&document);
    assert!(written.contains("<+A>a1*2\r\n\r\n>\r\n"));
    assert!(written.contains(" 1 0 80 0 0 0 17 1 0 0 0 0\r\n"));
}

#[test]
fn a_formula_given_to_a_cell_is_written_with_the_results_prefix() {
    let mut document = open(&fixture());
    let frame = document
        .frames()
        .find(|frame| matches!(frame.content, FrameContent::Table(_)))
        .unwrap()
        .id;
    let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
        unreachable!()
    };
    // B2 had no formula, no alignment tag: only Table Text's untagged form.
    table.cells[3].formula = Some("=A2*2".into());
    let written = write(&document);
    assert!(written.contains(
        " 1 1 8256 0 0 0 0 1 0 0 0 0\r\nB2\r\n\r\nsecond paragraph\r\n\r\n>\r\n=A2*2\r\n\r\n>\r\n"
    ));
    let again = open(&written);
    assert_eq!(tables(&again)[0].cells[3].formula.as_deref(), Some("=A2*2"));
    // Taking the formula away clears the bit and the body.
    let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
        unreachable!()
    };
    table.cells[3].formula = None;
    table.cells[2].formula = None;
    let cleared = write(&document);
    assert!(!cleared.contains("=A2*2"));
    assert!(
        cleared.contains(" 1 0 16 0 0 0 17 1 0 0 0 0\r\n"),
        "0x40 cleared"
    );
}

#[test]
fn native_files_keep_the_formula() {
    let document = open(&fixture());
    let encoded = document.encode_native().unwrap();
    assert!(encoded.contains("cellformula\t@Sum(A1..A1)+a1*2\n"));
    let decoded = Document::decode_native(&encoded).unwrap();
    assert_eq!(decoded, document);
    assert_eq!(
        tables(&decoded)[0].cells[2].formula.as_deref(),
        Some("@Sum(A1..A1)+a1*2")
    );
    decoded.validate().unwrap();
}

#[test]
fn validation_refuses_a_formula_that_cannot_be_stored() {
    let mut document = open(&fixture());
    let frame = document
        .frames()
        .find(|frame| matches!(frame.content, FrameContent::Table(_)))
        .unwrap()
        .id;
    for bad in ["", "a\nb", "a\u{7}b"] {
        let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
            unreachable!()
        };
        table.cells[2].formula = Some(bad.into());
        assert!(document.validate().is_err(), "{bad:?}");
    }
    let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
        unreachable!()
    };
    // A covered cell shows nothing of its own: no formula there.
    table.cells[2].formula = Some("1+1".into());
    document.validate().unwrap();
    let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
        unreachable!()
    };
    table.cells[0].span = amipro_document::CellSpan {
        rows: 1,
        columns: 2,
    };
    table.cells[1].span = amipro_document::CellSpan::COVERED;
    table.cells[1].formula = Some("1+1".into());
    assert!(document.validate().is_err());
}

#[test]
fn save_as_lists_the_native_only_functions() {
    let mut document = open(&fixture());
    assert!(
        sam_losses(&document, &SamOptions::at(1))
            .unwrap()
            .is_empty()
    );
    let frame = document
        .frames()
        .find(|frame| matches!(frame.content, FrameContent::Table(_)))
        .unwrap()
        .id;
    let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
        unreachable!()
    };
    table.cells[2].formula = Some("@avg(A1..A1)".into());
    let dims = Dims {
        columns: 2,
        rows: 2,
    };
    let extras = Options {
        extras: true,
        ..Options::default()
    };
    assert!(
        Formula::compile("@avg(A1..A1)", dims, extras)
            .unwrap()
            .uses_extras
    );
    let losses = sam_losses(&document, &SamOptions::at(1)).unwrap();
    assert_eq!(losses.len(), 1, "{losses:?}");
    assert!(losses[0].message().contains("function Ami Pro lacks"));
    assert_eq!(losses[0].count, 1);
}

#[test]
fn a_styles_number_format_is_read_and_written_as_it_was() {
    let document = open(&fixture());
    for style in document.styles().values() {
        let values = style.kept.number_format.to_values();
        assert_eq!(
            NumberFormat::from_values(&values).unwrap(),
            style.kept.number_format
        );
    }
}

#[test]
fn an_untagged_formula_starting_with_at_is_written_twice() {
    // Ami Pro writes `@` twice at the start of an untagged paragraph, so
    // that the text is not taken for a style tag; the formula is `@sum(…)`.
    let sam = fixture()
        .replace("@Heading@<+A>3250.00", "<+A>3250.00")
        .replace("@Heading@<+A>@Sum(A1..A1)+a1*2", "<+A>@@sum(A1..A1)");
    let document = open(&sam);
    let cell = &tables(&document)[0].cells[2];
    assert_eq!(cell.formula.as_deref(), Some("@sum(A1..A1)"));
    let written = write(&document);
    assert!(written.contains("\r\n<+A>3250.00\r\n\r\n>\r\n<+A>@@sum(A1..A1)\r\n\r\n>\r\n"));
    // A single `@` there is not Ami Pro's form: the body stays as it is.
    let single = sam.replace("<+A>@@sum", "<+A>@sum");
    let document = open(&single);
    let cell = &tables(&document)[0].cells[2];
    assert_eq!(cell.formula, None);
    assert_eq!(cell.legacy.get("cell.body"), Some("<+A>@sum(A1..A1)"));
}

fn set_formula(document: &mut Document, cell: usize, formula: &str) {
    let frame = document
        .frames()
        .find(|frame| matches!(frame.content, FrameContent::Table(_)))
        .unwrap()
        .id;
    let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
        unreachable!()
    };
    table.cells[cell].formula = Some(formula.into());
}

#[test]
fn ref_is_stored_with_bit_0x2000_and_a_divide_by_zero_is_not() {
    let mut document = open(&fixture());
    // Cell 2 reads A1, a text cell: REF. Cell 3 divides by zero.
    set_formula(&mut document, 2, "A1+1");
    set_formula(&mut document, 3, "1/0");
    let written = write(&document);
    assert!(
        written.contains(" 1 0 8272 0 0 0 17 1 0 0 0 0\r\n"),
        "0x2000 + 0x50"
    );
    assert!(
        written.contains(" 1 1 64 0 0 0 0 1 0 0 0 0\r\n"),
        "no 0x2000 for 1/0"
    );
    // The word REF (a deleted referenced cell) is REF as well.
    set_formula(&mut document, 3, "REF*2");
    assert!(write(&document).contains(" 1 1 8256 0 0 0 0 1 0 0 0 0\r\n"));
    // Reading does not recalculate: the stored result stays (VM TDF6).
    let document = open(&write(&document));
    let table = tables(&document)[0];
    assert_eq!(text(&document, &table.cells[2]), "3250.00");
}

#[test]
fn an_empty_result_is_written_as_the_alignment_tag_alone() {
    let mut document = open(&fixture());
    // Cell 1 never held text; with a formula that divides by zero its result
    // is empty (VM TDF1: `<+A>` alone, F3 0x50).
    set_formula(&mut document, 1, "1/0");
    let written = write(&document);
    assert!(
        written.contains("<+A>\r\n\r\n>\r\n<+A>1/0\r\n\r\n>\r\n"),
        "{written}"
    );
    let again = open(&written);
    let cell = &tables(&again)[0].cells[1];
    assert_eq!(cell.formula.as_deref(), Some("1/0"));
    assert_eq!(text(&again, cell), "");
}
