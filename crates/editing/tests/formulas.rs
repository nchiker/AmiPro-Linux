//! Table formulas as commands (run C, TD.6; `docs/research/TABLE_DATA_NOTES.md`
//! sections 10 to 12): Edit Formula's OK, Quick Add, recalculation inside the
//! commands that change a cell, the names following rows, columns, copies and
//! moves, the stored forms Ami Pro's VM showed, one undo step each, and the
//! SAM written after the edits read back equal.

use amipro_document::{CellContent, Document, FlowId, FrameContent, FrameId, StyleId, Table};
use amipro_editing::tables::{
    CellBlock, CellRange, FORMULA_IN_SORT, FORMULA_REFUSED, NO_FORMULA, NOTHING_RECALCULATED,
    NOTHING_TO_ADD, NewTable, TableEdit, TableTarget,
};
use amipro_editing::{Command, History, TextPosition, apply};
use amipro_formats::roundtrip::{differences, is_writer_addition};
use amipro_formats::{SamOptions, import_sam, sam_document, sam_losses};

const OPTIONS: SamOptions = SamOptions::at(1_791_958_088);

fn run(document: &Document, command: Command) -> Document {
    apply(document, &command)
        .unwrap_or_else(|error| panic!("{command:?}: {error:?}"))
        .document
}

fn refused(document: &Document, command: Command) -> &'static str {
    apply(document, &command).expect_err("the command is refused")
}

fn table(document: &Document, frame: FrameId) -> &Table {
    match &document.frame(frame).unwrap().content {
        FrameContent::Table(table) => table,
        other => panic!("not a table: {other:?}"),
    }
}

fn flow_of(document: &Document, frame: FrameId, row: usize, column: usize) -> FlowId {
    let table = table(document, frame);
    let CellContent::Text(flow) = table.cells[row * table.columns.len() + column].content;
    flow
}

fn text(document: &Document, frame: FrameId, row: usize, column: usize) -> String {
    document
        .flow(flow_of(document, frame, row, column))
        .unwrap()
        .paragraphs[0]
        .text
        .clone()
}

fn formula(document: &Document, frame: FrameId, row: usize, column: usize) -> Option<String> {
    let table = table(document, frame);
    table.cells[row * table.columns.len() + column]
        .formula
        .clone()
}

fn type_in(
    document: &Document,
    frame: FrameId,
    row: usize,
    column: usize,
    value: &str,
) -> Document {
    let flow = flow_of(document, frame, row, column);
    let old = document.flow(flow).unwrap().paragraphs[0].text.len();
    let at = |byte| TextPosition {
        flow,
        paragraph: 0,
        byte,
    };
    run(
        document,
        Command::ReplaceText {
            start: at(0),
            end: at(old),
            text: value.into(),
        },
    )
}

fn set(document: &Document, frame: FrameId, row: usize, column: usize, text: &str) -> Document {
    run(
        document,
        Command::SetCellFormula {
            frame,
            row,
            column,
            formula: Some(text.into()),
        },
    )
}

fn edit(document: &Document, frame: FrameId, edit: TableEdit) -> Document {
    run(document, Command::EditTable { frame, edit })
}

/// A table of `columns` by `rows` with the first column holding 1, 2, 3 ...
fn numbers(columns: usize, rows: usize) -> (Document, FrameId) {
    let document = Document::with_paragraphs([("Above the table.", StyleId(1))]);
    let end = TextPosition {
        flow: document.main_flow_id(),
        paragraph: 0,
        byte: document.main_flow().paragraphs[0].text.len(),
    };
    let made = run(
        &document,
        Command::InsertTable {
            target: TableTarget::Text(end),
            table: NewTable::fitting(columns, rows, 9_360, 284),
        },
    );
    let frame = *made.frame_stack().last().unwrap();
    let mut made = made;
    for row in 0..rows {
        made = type_in(&made, frame, row, 0, &format!("{}.00", row + 1));
    }
    (made, frame)
}

/// Writes the document, reads it back, and says what differs.
fn round_trip(document: &Document) -> Document {
    let bytes = sam_document(document, &OPTIONS);
    let copy = import_sam(&bytes, None, "Edited").unwrap();
    let found: Vec<String> = differences(document, &copy.document)
        .into_iter()
        .filter(|difference| {
            !is_writer_addition(difference)
                && !matches!(difference.aspect.as_str(), "face kinds" | "line heights")
        })
        .map(|difference| format!("{difference:?}"))
        .collect();
    assert_eq!(found, Vec::<String>::new(), "read back differs");
    copy.document
}

fn table_frame(document: &Document) -> FrameId {
    *document
        .frame_stack()
        .iter()
        .find(|id| {
            matches!(
                document.frame(**id).unwrap().content,
                FrameContent::Table(_)
            )
        })
        .expect("a table")
}

/// The cells of `a`'s table and of the table of `b` (the same table, read
/// back: frame numbers may differ) are equal in formula and text.
fn same_cells(a: &Document, b: &Document, frame: FrameId) {
    let other = table_frame(b);
    let (left, right) = (table(a, frame), table(b, other));
    assert_eq!(left.cells.len(), right.cells.len());
    let columns = left.columns.len();
    for index in 0..left.cells.len() {
        let (row, column) = (index / columns, index % columns);
        assert_eq!(
            formula(a, frame, row, column),
            formula(b, other, row, column),
            "formula {row},{column}"
        );
        assert_eq!(
            text(a, frame, row, column),
            text(b, other, row, column),
            "text {row},{column}"
        );
    }
}

fn section(document: &Document) -> String {
    let bytes = sam_document(document, &OPTIONS);
    let all = String::from_utf8_lossy(&bytes).into_owned();
    let start = all.find("\t[tbl]").unwrap();
    let end = all.find("\t[tble]").unwrap();
    all[start..end].to_owned()
}

#[test]
fn edit_formula_stores_the_text_as_typed_and_its_result() {
    let (document, frame) = numbers(3, 4);
    let document = type_in(&document, frame, 1, 0, "4.00");
    let one = set(&document, frame, 0, 1, "=A1");
    assert_eq!(formula(&one, frame, 0, 1).as_deref(), Some("=A1"));
    assert_eq!(text(&one, frame, 0, 1), "1.00");
    let spaced = set(&one, frame, 1, 1, "a1 + a2");
    assert_eq!(formula(&spaced, frame, 1, 1).as_deref(), Some("a1 + a2"));
    assert_eq!(text(&spaced, frame, 1, 1), "5.00");
    let bare = set(&spaced, frame, 2, 1, "sum(A1:A2)");
    assert_eq!(formula(&bare, frame, 2, 1).as_deref(), Some("sum(A1:A2)"));
    assert_eq!(text(&bare, frame, 2, 1), "5.00");
    let at = set(&bare, frame, 3, 1, "@sum(A1..A4)");
    assert_eq!(text(&at, frame, 3, 1), "12.00");
    // The result is right-aligned, as Ami Pro stores a calculated cell.
    let paragraph = &at.flow(flow_of(&at, frame, 3, 1)).unwrap().paragraphs[0];
    assert_eq!(
        paragraph.format.alignment,
        Some(amipro_document::Alignment::Right)
    );
    // Replacing a formula keeps one formula.
    let again = set(&at, frame, 3, 1, "A1*3");
    assert_eq!(formula(&again, frame, 3, 1).as_deref(), Some("A1*3"));
    assert_eq!(text(&again, frame, 3, 1), "3.00");
    same_cells(&again, &round_trip(&again), frame);
}

#[test]
fn a_formula_ami_pro_cannot_read_is_refused_and_nothing_is_stored() {
    let (document, frame) = numbers(3, 3);
    for bad in ["foo(A1)", "@sum(A1:B2)", "Z9", "2+", "", "1e5", "A1\nA2"] {
        let error = refused(
            &document,
            Command::SetCellFormula {
                frame,
                row: 0,
                column: 1,
                formula: Some(bad.into()),
            },
        );
        assert_eq!(error, FORMULA_REFUSED, "{bad:?}");
    }
    // A refusal leaves the history alone.
    let mut current = document.clone();
    let mut history: History<()> = History::new();
    let result = history.execute(
        &mut current,
        (),
        &Command::SetCellFormula {
            frame,
            row: 0,
            column: 1,
            formula: Some("foo(A1)".into()),
        },
    );
    assert!(result.is_err());
    assert_eq!(current, document);
    assert!(history.undo(&mut current, ()).is_none());
    // A cell is not its own undefined range: a table's own names only.
    assert_eq!(formula(&document, frame, 0, 1), None);
}

#[test]
fn errors_are_stored_as_ami_pro_stored_them() {
    let (document, frame) = numbers(3, 4);
    let document = type_in(&document, frame, 3, 0, "0.00");
    let document = type_in(&document, frame, 2, 0, "abc");
    // Divide by zero: stored, with an empty result; the loop likewise.
    let divided = set(&document, frame, 0, 1, "A1/A4");
    assert_eq!(formula(&divided, frame, 0, 1).as_deref(), Some("A1/A4"));
    assert_eq!(text(&divided, frame, 0, 1), "");
    // A reader of it counts it as 0 (an empty result is not an error).
    let reader = set(&divided, frame, 1, 1, "B1+1");
    assert_eq!(text(&reader, frame, 1, 1), "1.00");
    let loop_one = set(&reader, frame, 0, 2, "C2");
    let loop_two = set(&loop_one, frame, 1, 2, "C1");
    assert_eq!(formula(&loop_two, frame, 1, 2).as_deref(), Some("C1"));
    assert_eq!(text(&loop_two, frame, 1, 2), "");
    // A text cell, a range holding one, and the word REF: REF.
    let by_text = set(&loop_two, frame, 2, 1, "A3+1");
    assert_eq!(text(&by_text, frame, 2, 1), "");
    let by_range = set(&by_text, frame, 3, 1, "@sum(A1:A3)");
    assert_eq!(text(&by_range, frame, 3, 1), "");
    let by_word = set(&by_range, frame, 2, 2, "REF");
    assert_eq!(formula(&by_word, frame, 2, 2).as_deref(), Some("REF"));
    // F3 0x50 for the numbers and the errors that are not REF, 0x2050 for REF.
    let written = section(&by_word);
    let f3 = |row: usize, column: usize| -> i64 {
        written
            .lines()
            .map(str::trim)
            .filter_map(|line| {
                let values: Vec<i64> = line
                    .split(' ')
                    .map(str::parse)
                    .collect::<Result<_, _>>()
                    .ok()?;
                (values.len() >= 8 && values[0] == row as i64 && values[1] == column as i64)
                    .then(|| values[2])
            })
            .next()
            .unwrap_or_else(|| panic!("no record for {row},{column}"))
    };
    assert_eq!(f3(0, 1) & 0x2050, 0x50, "divide by zero");
    assert_eq!(f3(1, 1) & 0x2050, 0x50, "reader of an empty result");
    assert_eq!(f3(1, 2) & 0x2050, 0x50, "loop");
    assert_eq!(f3(2, 1) & 0x2050, 0x2050, "text cell");
    assert_eq!(f3(3, 1) & 0x2050, 0x2050, "range with text");
    assert_eq!(f3(2, 2) & 0x2050, 0x2050, "the word REF");
    same_cells(&by_word, &round_trip(&by_word), frame);
}

#[test]
fn dependants_follow_inside_the_same_step() {
    let (document, frame) = numbers(3, 3);
    let document = set(&document, frame, 0, 1, "A1*2");
    let document = set(&document, frame, 0, 2, "B1+1");
    let document = set(&document, frame, 2, 1, "@sum(B1..B2)");
    assert_eq!(text(&document, frame, 0, 2), "3.00");
    // Changing a formula changes what reads it, in one undo step.
    let mut current = document.clone();
    let mut history: History<()> = History::new();
    history
        .execute(
            &mut current,
            (),
            &Command::SetCellFormula {
                frame,
                row: 0,
                column: 1,
                formula: Some("A1*10".into()),
            },
        )
        .unwrap();
    assert_eq!(text(&current, frame, 0, 1), "10.00");
    assert_eq!(text(&current, frame, 0, 2), "11.00");
    assert_eq!(text(&current, frame, 2, 1), "10.00");
    assert!(history.undo(&mut current, ()).is_some());
    assert_eq!(current, document, "formula text and every result return");
    assert!(history.redo(&mut current, ()).is_some());
    assert_eq!(text(&current, frame, 0, 2), "11.00");
    // The user typed a number and left the cell: the trigger recalculates.
    let typed = type_in(&document, frame, 0, 0, "5.00");
    assert_eq!(text(&typed, frame, 0, 2), "3.00", "not per keystroke");
    let settled = run(
        &typed,
        Command::RecalculateTable {
            frame,
            changed: vec![(0, 0)],
        },
    );
    assert_eq!(text(&settled, frame, 0, 1), "10.00");
    assert_eq!(text(&settled, frame, 0, 2), "11.00");
    assert_eq!(text(&settled, frame, 2, 1), "10.00");
    // Nothing more to do: reported, not a step.
    let error = refused(
        &settled,
        Command::RecalculateTable {
            frame,
            changed: vec![(0, 0)],
        },
    );
    assert_eq!(error, NOTHING_RECALCULATED);
    // Recalculate Table, all: puts a stale result right, one step.
    let stale = type_in(&settled, frame, 0, 2, "99.00");
    let fixed = run(
        &stale,
        Command::RecalculateTable {
            frame,
            changed: Vec::new(),
        },
    );
    assert_eq!(text(&fixed, frame, 0, 2), "11.00");
}

#[test]
fn del_removes_the_formula_and_the_cell_record() {
    let (document, frame) = numbers(3, 3);
    let document = set(&document, frame, 0, 1, "A1*2");
    let document = set(&document, frame, 0, 2, "B1+1");
    let before_records = section(&document);
    let cleared = edit(
        &document,
        frame,
        TableEdit::ClearCells(CellRange::cell(0, 1)),
    );
    assert_eq!(formula(&cleared, frame, 0, 1), None);
    assert_eq!(text(&cleared, frame, 0, 1), "");
    // The cell is plain again, and what read it counts it as 0.
    assert_eq!(text(&cleared, frame, 0, 2), "1.00");
    let paragraph = &cleared
        .flow(flow_of(&cleared, frame, 0, 1))
        .unwrap()
        .paragraphs[0];
    assert_eq!(paragraph.format.alignment, None);
    let after = section(&cleared);
    let count = |section: &str| {
        section
            .lines()
            .filter(|line| line.trim().starts_with("0 1 "))
            .count()
    };
    assert_eq!(count(&before_records), 1);
    assert_eq!(count(&after), 0, "the record disappears from the file");
    same_cells(&cleared, &round_trip(&cleared), frame);
    // Edit Formula's clear does the same; a cell without one is refused.
    let removed = run(
        &document,
        Command::SetCellFormula {
            frame,
            row: 0,
            column: 1,
            formula: None,
        },
    );
    assert_eq!(removed, cleared);
    assert_eq!(
        refused(
            &cleared,
            Command::SetCellFormula {
                frame,
                row: 0,
                column: 1,
                formula: None,
            }
        ),
        NO_FORMULA
    );
}

#[test]
fn quick_add_sums_leftwards_and_upwards_from_after_the_last_label() {
    // Row 0: label, 10, empty, 20, caret. Row 1 is numbers under them.
    let (document, frame) = numbers(6, 3);
    let mut document = document;
    for (column, value) in [(1, "10.00"), (3, "20.00")] {
        document = type_in(&document, frame, 0, column, value);
    }
    document = type_in(&document, frame, 0, 0, "Total");
    let added = run(
        &document,
        Command::QuickAdd {
            frame,
            row: 0,
            column: 4,
            along_row: true,
        },
    );
    assert_eq!(
        formula(&added, frame, 0, 4).as_deref(),
        Some("@sum(B1..D1)")
    );
    assert_eq!(text(&added, frame, 0, 4), "30.00");
    // The caret cell's old content is replaced, a formula there too.
    let again = run(
        &added,
        Command::QuickAdd {
            frame,
            row: 0,
            column: 5,
            along_row: true,
        },
    );
    assert_eq!(
        formula(&again, frame, 0, 5).as_deref(),
        Some("@sum(B1..E1)")
    );
    assert_eq!(text(&again, frame, 0, 5), "60.00");
    // Column: upwards, the numeric column A from row 1 (A1 is a label here).
    let column = run(
        &again,
        Command::QuickAdd {
            frame,
            row: 2,
            column: 0,
            along_row: false,
        },
    );
    assert_eq!(
        formula(&column, frame, 2, 0).as_deref(),
        Some("@sum(A2..A2)")
    );
    // The neighbour itself a label: the range is that cell.
    assert_eq!(text(&column, frame, 1, 0), "2.00");
    let heading = type_in(&again, frame, 1, 0, "Heading");
    let by_label = run(
        &heading,
        Command::QuickAdd {
            frame,
            row: 2,
            column: 0,
            along_row: false,
        },
    );
    assert_eq!(
        formula(&by_label, frame, 2, 0).as_deref(),
        Some("@sum(A2..A2)")
    );
    assert_eq!(text(&by_label, frame, 2, 0), "");
    // A formula cell and an empty cell are walked through; no label: the edge.
    let none = run(
        &added,
        Command::QuickAdd {
            frame,
            row: 0,
            column: 2,
            along_row: true,
        },
    );
    assert_eq!(formula(&none, frame, 0, 2).as_deref(), Some("@sum(B1..B1)"));
    // Nothing to add on the first column or row.
    for (row, column, along_row) in [(0, 0, true), (0, 3, false)] {
        let error = refused(
            &document,
            Command::QuickAdd {
                frame,
                row,
                column,
                along_row,
            },
        );
        assert_eq!(error, NOTHING_TO_ADD);
    }
    same_cells(&again, &round_trip(&again), frame);
}

/// TDF3's table: A1..A6 = 1..6 and formulas in column B.
fn tdf3() -> (Document, FrameId) {
    let (mut document, frame) = numbers(3, 7);
    document = type_in(&document, frame, 6, 0, "7.00");
    for (row, text) in [
        "@sum(A2..A5)",
        "A3*2",
        "A$1+$A2",
        "@sum(A1..A3)",
        "@sum(A3..A6)",
        "A2+A4",
    ]
    .iter()
    .enumerate()
    {
        document = set(&document, frame, row, 1, text);
    }
    (document, frame)
}

#[test]
fn deleting_a_row_rewrites_the_names_as_ami_pro_did_tdf3() {
    let (document, frame) = tdf3();
    assert_eq!(text(&document, frame, 0, 1), "14.00");
    let deleted = edit(&document, frame, TableEdit::DeleteRows(2..3));
    let names: Vec<_> = (0..6).map(|row| formula(&deleted, frame, row, 1)).collect();
    assert_eq!(
        names,
        [
            Some("@sum(A2..A4)".to_owned()),
            Some("REF*2".to_owned()),
            Some("@sum(A1..A2)".to_owned()),
            Some("@sum(A3..A5)".to_owned()),
            Some("A2+A3".to_owned()),
            None,
        ]
    );
    // The recalculation inside the command: the numbers follow (2+4+5).
    assert_eq!(text(&deleted, frame, 0, 1), "11.00");
    assert_eq!(text(&deleted, frame, 2, 1), "3.00");
    // A REF result keeps the text it had (VM TDF3 stored it stale).
    assert_eq!(text(&deleted, frame, 1, 1), "6.00");
    same_cells(&deleted, &round_trip(&deleted), frame);
    // A later recalculation shows it for what it is.
    let fixed = run(
        &deleted,
        Command::RecalculateTable {
            frame,
            changed: Vec::new(),
        },
    );
    assert_eq!(text(&fixed, frame, 1, 1), "");
}

#[test]
fn inserting_a_row_stretches_the_ranges_ami_pro_did_tdf3() {
    let (document, frame) = tdf3();
    let inserted = edit(
        &document,
        frame,
        TableEdit::InsertRows {
            at: 3,
            count: 1,
            like: None,
        },
    );
    let at = |row| formula(&inserted, frame, row, 1);
    assert_eq!(at(0).as_deref(), Some("@sum(A2..A6)"));
    assert_eq!(at(1).as_deref(), Some("A3*2"));
    assert_eq!(at(2).as_deref(), Some("A$1+$A2"));
    assert_eq!(at(3), None);
    assert_eq!(at(4).as_deref(), Some("@sum(A1..A3)"));
    assert_eq!(at(5).as_deref(), Some("@sum(A3..A7)"));
    assert_eq!(at(6).as_deref(), Some("A2+A5"));
    // The inserted row is empty: the sum over it is the same, and the
    // results are unchanged but for what now reads different cells.
    assert_eq!(text(&inserted, frame, 0, 1), "14.00");
    assert_eq!(text(&inserted, frame, 6, 1), "6.00");
    same_cells(&inserted, &round_trip(&inserted), frame);
}

#[test]
fn inserting_and_deleting_columns_rewrite_the_names() {
    let (document, frame) = tdf3();
    let inserted = edit(
        &document,
        frame,
        TableEdit::InsertColumns {
            at: 0,
            count: 1,
            like: None,
            room: None,
        },
    );
    assert_eq!(
        formula(&inserted, frame, 0, 2).as_deref(),
        Some("@sum(B2..B5)")
    );
    assert_eq!(formula(&inserted, frame, 2, 2).as_deref(), Some("B$1+$B2"));
    assert_eq!(text(&inserted, frame, 0, 2), "14.00");
    let deleted = edit(&inserted, frame, TableEdit::DeleteColumns(0..1));
    assert_eq!(
        formula(&deleted, frame, 0, 1).as_deref(),
        Some("@sum(A2..A5)")
    );
    // Deleting the data column makes every name REF.
    let gone = edit(&document, frame, TableEdit::DeleteColumns(0..1));
    assert_eq!(formula(&gone, frame, 1, 0).as_deref(), Some("REF*2"));
    same_cells(&gone, &round_trip(&gone), frame);
}

#[test]
fn copying_and_pasting_a_formula_cell_shifts_the_relative_names_tdf3() {
    let (document, frame) = tdf3();
    let paste = |source: (usize, usize), target: (usize, usize)| {
        let block = CellBlock::copy(
            &document,
            table(&document, frame),
            &CellRange::cell(source.0, source.1),
        );
        edit(
            &document,
            frame,
            TableEdit::PasteCells {
                at: target,
                block,
                area: None,
                add_rows: false,
            },
        )
    };
    // Offset (0, +2): `A$1+$A2` becomes `A$1+$A4`; the result follows at once.
    let down = paste((2, 1), (4, 1));
    assert_eq!(formula(&down, frame, 4, 1).as_deref(), Some("A$1+$A4"));
    assert_eq!(text(&down, frame, 4, 1), "5.00");
    // Offset (+1, +1): `@sum(A1..A2)` becomes `@sum(B2..B3)`.
    let (small, frame2) = numbers(3, 4);
    let small = set(&small, frame2, 0, 1, "@sum(A1..A2)");
    let block = CellBlock::copy(&small, table(&small, frame2), &CellRange::cell(0, 1));
    let pasted = edit(
        &small,
        frame2,
        TableEdit::PasteCells {
            at: (1, 2),
            block,
            area: None,
            add_rows: false,
        },
    );
    assert_eq!(
        formula(&pasted, frame2, 1, 2).as_deref(),
        Some("@sum(B2..B3)")
    );
    // Over a formula cell, plain text takes the formula away.
    let block = CellBlock::copy(&document, table(&document, frame), &CellRange::cell(0, 0));
    let over = edit(
        &document,
        frame,
        TableEdit::PasteCells {
            at: (1, 1),
            block,
            area: None,
            add_rows: false,
        },
    );
    assert_eq!(formula(&over, frame, 1, 1), None);
    assert_eq!(text(&over, frame, 1, 1), "1.00");
    // A name pushed above the table becomes REF.
    let block = CellBlock::copy(&document, table(&document, frame), &CellRange::cell(4, 1));
    let off = edit(
        &document,
        frame,
        TableEdit::PasteCells {
            at: (0, 1),
            block,
            area: None,
            add_rows: false,
        },
    );
    assert_eq!(formula(&off, frame, 0, 1).as_deref(), Some("@sum(REF)"));
    same_cells(&down, &round_trip(&down), frame);
    same_cells(&off, &round_trip(&off), frame);
}

#[test]
fn moving_rows_moves_the_formulas_relative_names_with_them() {
    let (document, frame) = tdf3();
    let moved = edit(
        &document,
        frame,
        TableEdit::MoveRows {
            rows: 5..6,
            to: 0,
            copy: false,
        },
    );
    // `A2+A4` in row 6 moves to row 1: five rows up.
    assert_eq!(formula(&moved, frame, 0, 1).as_deref(), Some("REF+REF"));
    let moved = edit(
        &document,
        frame,
        TableEdit::MoveRows {
            rows: 0..1,
            to: 3,
            copy: true,
        },
    );
    assert_eq!(
        formula(&moved, frame, 3, 1).as_deref(),
        Some("@sum(A5..A8)")
    );
}

#[test]
fn a_formula_in_the_rows_stops_a_sort() {
    let (document, frame) = tdf3();
    let error = refused(
        &document,
        Command::EditTable {
            frame,
            edit: TableEdit::SortRows {
                column: 0,
                descending: true,
            },
        },
    );
    assert_eq!(error, FORMULA_IN_SORT);
    let (plain, frame) = numbers(2, 4);
    let sorted = edit(
        &plain,
        frame,
        TableEdit::SortRows {
            column: 0,
            descending: true,
        },
    );
    assert_eq!(text(&sorted, frame, 0, 0), "4.00");
}

#[test]
fn every_formula_command_is_one_undo_step_and_undo_restores_text_and_results() {
    let (document, frame) = tdf3();
    let mut current = document.clone();
    let mut history: History<()> = History::new();
    let commands = [
        Command::SetCellFormula {
            frame,
            row: 5,
            column: 2,
            formula: Some("B1+B5".into()),
        },
        Command::QuickAdd {
            frame,
            row: 6,
            column: 1,
            along_row: false,
        },
        Command::EditTable {
            frame,
            edit: TableEdit::DeleteRows(2..3),
        },
        Command::EditTable {
            frame,
            edit: TableEdit::InsertColumns {
                at: 0,
                count: 1,
                like: None,
                room: None,
            },
        },
        Command::EditTable {
            frame,
            edit: TableEdit::ClearCells(CellRange::cell(0, 2)),
        },
        Command::SetCellFormula {
            frame,
            row: 1,
            column: 2,
            formula: None,
        },
    ];
    for command in commands {
        let before = current.clone();
        history.execute(&mut current, (), &command).unwrap();
        assert_ne!(current, before, "{command:?}");
        assert!(history.undo(&mut current, ()).is_some());
        assert_eq!(current, before, "{command:?}");
        history.redo(&mut current, ());
    }
}

#[test]
fn a_stored_result_is_kept_on_open_and_the_extras_are_listed() {
    let (document, frame) = numbers(3, 3);
    let document = set(&document, frame, 0, 1, "A1+A2");
    // A result edited by hand stays as it is across a write and a read.
    let stale = type_in(&document, frame, 0, 1, "99.00");
    let copy = round_trip(&stale);
    let other = table_frame(&copy);
    assert_eq!(text(&copy, other, 0, 1), "99.00");
    assert_eq!(formula(&copy, other, 0, 1).as_deref(), Some("A1+A2"));
    // The native-only functions are accepted, and Save As lists them.
    let extras = set(&document, frame, 1, 1, "@avg(A1..A3)");
    assert_eq!(text(&extras, frame, 1, 1), "2.00");
    let losses = sam_losses(&extras, &OPTIONS).unwrap();
    assert!(
        losses
            .iter()
            .any(|loss| format!("{loss:?}").contains("Formula")),
        "{losses:?}"
    );
    for (formula_text, expected) in [
        ("@min(A1..A3)", "1.00"),
        ("@max(A1..A3)", "3.00"),
        ("@count(A1..A3)", "3.00"),
    ] {
        let one = set(&document, frame, 1, 1, formula_text);
        assert_eq!(text(&one, frame, 1, 1), expected, "{formula_text}");
    }
}

/// A small deterministic generator for the property test.
struct Random(u64);

impl Random {
    fn below(&mut self, limit: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % limit
    }
}

#[test]
fn edits_then_a_sam_write_and_read_give_the_same_formulas_and_results() {
    let formulas = [
        "A1+A2",
        "@sum(A1..A4)",
        "B1*2",
        "=C1-A1",
        "A1/A2",
        "(A1+A2)*3",
        "A3%",
        "sum(B1:B4)",
        "REF",
        "foo(1)",
    ];
    for seed in 0..6u64 {
        let mut random = Random(seed + 1);
        let (mut document, frame) = numbers(4, 6);
        for _ in 0..60 {
            let rows = table(&document, frame).rows.len();
            let columns = table(&document, frame).columns.len();
            let (row, column) = (random.below(rows), random.below(columns));
            let command = match random.below(8) {
                0..=2 => Command::SetCellFormula {
                    frame,
                    row,
                    column,
                    formula: Some(formulas[random.below(formulas.len())].to_owned()),
                },
                3 => Command::QuickAdd {
                    frame,
                    row,
                    column,
                    along_row: random.below(2) == 0,
                },
                4 => Command::EditTable {
                    frame,
                    edit: TableEdit::ClearCells(CellRange::cell(row, column)),
                },
                5 if rows > 3 => Command::EditTable {
                    frame,
                    edit: TableEdit::DeleteRows(row..row + 1),
                },
                5 | 6 if rows < 12 => Command::EditTable {
                    frame,
                    edit: TableEdit::InsertRows {
                        at: row,
                        count: 1,
                        like: None,
                    },
                },
                _ => Command::RecalculateTable {
                    frame,
                    changed: vec![(row, column)],
                },
            };
            if let Ok(applied) = apply(&document, &command) {
                document = applied.document;
                document.validate().unwrap();
            }
            let copy = round_trip(&document);
            same_cells(&document, &copy, frame);
            assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
        }
    }
}

// ---- TD.11: the native-only functions (TN4) --------------------------------

/// TNK-TD11a: `@avg @min @max @count` over a column with a blank, a text
/// cell, and numbers. The blank is skipped (`@sum` counts it as 0); a cell
/// that is not a number gives REF in `@avg @min @max`, and `@count` counts
/// the numbers and never gives REF; no numbers at all give 0.
#[test]
fn the_extras_skip_blanks_refuse_text_and_count_numbers_td11() {
    let (document, frame) = numbers(3, 5);
    // A1 = 1, A2 = 2, A3 = 3 from `numbers`; A4 blank; A5 text.
    let document = type_in(&document, frame, 3, 0, "");
    let document = type_in(&document, frame, 4, 0, "abc");
    for (formula_text, expected) in [
        ("@avg(A1..A4)", "2.00"),
        ("@min(A1..A4)", "1.00"),
        ("@max(A1..A4)", "3.00"),
        ("@count(A1..A4)", "3.00"),
        ("@sum(A1..A4)", "6.00"),
        ("@avg(B1..B4)", "0.00"),
        ("@min(B1..B4)", "0.00"),
        ("@max(B1..B4)", "0.00"),
        ("@count(B1..B4)", "0.00"),
        ("@COUNT(A1:A5)", "3.00"),
        ("=@Avg(A1.A3)", "2.00"),
    ] {
        let one = set(&document, frame, 0, 2, formula_text);
        assert_eq!(text(&one, frame, 0, 2), expected, "{formula_text}");
        assert_eq!(
            formula(&one, frame, 0, 2).as_deref(),
            Some(formula_text),
            "kept as typed"
        );
    }
    for formula_text in [
        "@avg(A1..A5)",
        "@min(A1..A5)",
        "@max(A1..A5)",
        "@sum(A1..A5)",
    ] {
        let one = set(&document, frame, 0, 2, formula_text);
        // REF is stored as the empty result (TNK-TD6a), as `@sum` does.
        assert_eq!(text(&one, frame, 0, 2), "", "{formula_text}");
    }
    // A change in the range recalculates the extra like any formula.
    let one = set(&document, frame, 0, 2, "@max(A1..A3)");
    let changed = run(
        &type_in(&one, frame, 1, 0, "9.00"),
        Command::RecalculateTable {
            frame,
            changed: vec![(1, 0)],
        },
    );
    assert_eq!(text(&changed, frame, 0, 2), "9.00");
}

/// (b) to (d): Save As lists the extras (once per cell, counted), the SAM
/// keeps the text and the result and reads them back, and so does the native
/// format; a formula with only Ami Pro's functions lists nothing.
#[test]
fn the_extras_are_listed_by_save_as_and_kept_by_sam_and_native_td11() {
    let (document, frame) = numbers(4, 3);
    let mut made = document.clone();
    for (column, text) in ["@avg(A1..A3)", "@min(A1..A3)", "@max(A1..A3)"]
        .into_iter()
        .enumerate()
    {
        made = set(&made, frame, 0, column + 1, text);
    }
    made = set(&made, frame, 1, 1, "@count(A1..A3)");
    made = set(&made, frame, 1, 2, "@sum(A1..A3)");
    let losses = sam_losses(&made, &OPTIONS).unwrap();
    let messages: Vec<String> = losses
        .iter()
        .map(amipro_formats::roundtrip::Loss::message)
        .collect();
    assert_eq!(
        messages,
        [
            "4 formulas with a function Ami Pro lacks (@avg, @min, @max, @count), which it cannot understand"
        ]
    );
    let plain = set(&document, frame, 0, 1, "@sum(A1..A3)");
    assert!(sam_losses(&plain, &OPTIONS).unwrap().is_empty());
    // SAM: text and result kept, read back equal.
    let copy = round_trip(&made);
    same_cells(&made, &copy, frame);
    let other = table_frame(&copy);
    assert_eq!(formula(&copy, other, 0, 1).as_deref(), Some("@avg(A1..A3)"));
    assert_eq!(text(&copy, other, 0, 1), "2.00");
    assert_eq!(text(&copy, other, 1, 1), "3.00");
    assert!(section(&made).contains("@avg(A1..A3)"));
    // Native v3: the same cells, still calculating after the load.
    let encoded = made.encode_native().unwrap();
    let loaded = Document::decode_native(&encoded).unwrap();
    same_cells(&made, &loaded, frame);
    let loaded_frame = table_frame(&loaded);
    assert_eq!(
        formula(&loaded, loaded_frame, 1, 1).as_deref(),
        Some("@count(A1..A3)")
    );
    let again = run(
        &type_in(&loaded, loaded_frame, 2, 0, "6.00"),
        Command::RecalculateTable {
            frame: loaded_frame,
            changed: vec![(2, 0)],
        },
    );
    assert_eq!(text(&again, loaded_frame, 0, 3), "6.00", "@max follows");
}
