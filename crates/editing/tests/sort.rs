//! Tools › Sort… as commands (run C, TD.8; `docs/research/TABLE_DATA_NOTES.md`
//! sections 10.7 and 11.7): paragraphs and table rows, the refusals and
//! questions, one undo step, and Ami Pro's tie order on its TDY1 table.

use amipro_document::{BreakKind, CellContent, Document, FrameContent, FrameId, StyleId, Table};
use amipro_editing::sort::{
    SORT_ACROSS_BREAK, SORT_CONNECTED, SortKey, SortKind, SortSettings, SortWord, is_question,
};
use amipro_editing::tables::{CellRange, FORMULA_IN_SORT, NewTable, TableEdit, TableTarget};
use amipro_editing::{Command, Editor, TextPosition, apply};

fn run(document: &Document, command: Command) -> Document {
    apply(document, &command)
        .unwrap_or_else(|error| panic!("{command:?}: {error:?}"))
        .document
}

fn key(field: u8, kind: SortKind, word: SortWord) -> SortKey {
    SortKey { field, kind, word }
}

fn by(keys: Vec<SortKey>, descending: bool) -> SortSettings {
    SortSettings {
        keys,
        descending,
        ..SortSettings::default()
    }
}

fn table(document: &Document, frame: FrameId) -> &Table {
    match &document.frame(frame).unwrap().content {
        FrameContent::Table(table) => table,
        other => panic!("not a table: {other:?}"),
    }
}

fn cell_text(document: &Document, frame: FrameId, row: usize, column: usize) -> String {
    let table = table(document, frame);
    let CellContent::Text(flow) = table.cells[row * table.columns.len() + column].content;
    document.flow(flow).unwrap().paragraphs[0].text.clone()
}

/// A table with the given rows (TDY1's shape: three columns).
fn table_of(rows: &[[&str; 3]]) -> (Document, FrameId) {
    let document = Document::with_paragraphs([("Above the table.", StyleId(1))]);
    let end = TextPosition {
        flow: document.main_flow_id(),
        paragraph: 0,
        byte: document.main_flow().paragraphs[0].text.len(),
    };
    let mut made = run(
        &document,
        Command::InsertTable {
            target: TableTarget::Text(end),
            table: NewTable::fitting(3, rows.len(), 9_360, 284),
        },
    );
    let frame = *made.frame_stack().last().unwrap();
    for (row, texts) in rows.iter().enumerate() {
        for (column, text) in texts.iter().enumerate() {
            let table = table(&made, frame);
            let CellContent::Text(flow) = table.cells[row * 3 + column].content;
            made = run(
                &made,
                Command::ReplaceText {
                    start: TextPosition {
                        flow,
                        paragraph: 0,
                        byte: 0,
                    },
                    end: TextPosition {
                        flow,
                        paragraph: 0,
                        byte: 0,
                    },
                    text: (*text).into(),
                },
            );
        }
    }
    (made, frame)
}

const TDY1: [[&str; 3]; 12] = [
    ["k2", "d09", "Z12"],
    ["k1", "d05", "Z11"],
    ["k2", "d03", "Z10"],
    ["k1", "d08", "Z9"],
    ["k3", "d01", "Z8"],
    ["k1", "d02", "Z7"],
    ["k2", "d07", "Z6"],
    ["k3", "d06", "Z5"],
    ["k1", "d04", "Z4"],
    ["k2", "d10", "Z3"],
    ["k3", "d11", "Z2"],
    ["k1", "d12", "Z1"],
];

fn sort_rows(frame: FrameId, rows: Option<std::ops::Range<usize>>) -> Command {
    Command::EditTable {
        frame,
        edit: TableEdit::SortRecords {
            rows,
            settings: by(vec![key(1, SortKind::Alphanumeric, SortWord::First)], false),
        },
    }
}

#[test]
fn a_table_sorts_in_ami_pros_order_on_its_tdy1_ties() {
    let (document, frame) = table_of(&TDY1);
    let sorted = run(&document, sort_rows(frame, None));
    let seconds: Vec<String> = (0..12)
        .map(|row| cell_text(&sorted, frame, row, 1))
        .collect();
    assert_eq!(
        seconds,
        [
            "d05", "d08", "d04", "d02", "d12", "d03", "d09", "d10", "d07", "d06", "d11", "d01"
        ]
    );
    // The rows travel whole: the third column followed its key.
    let third: Vec<String> = (0..12)
        .map(|row| cell_text(&sorted, frame, row, 2))
        .collect();
    assert_eq!(
        third,
        [
            "Z11", "Z9", "Z4", "Z7", "Z1", "Z10", "Z12", "Z3", "Z6", "Z5", "Z2", "Z8"
        ]
    );
}

#[test]
fn selected_rows_sort_and_the_others_stay() {
    let (document, frame) = table_of(&TDY1);
    let sorted = run(&document, sort_rows(frame, Some(2..6)));
    let firsts: Vec<String> = (0..12)
        .map(|row| cell_text(&sorted, frame, row, 0))
        .collect();
    assert_eq!(&firsts[..2], ["k2", "k1"]);
    assert_eq!(&firsts[2..6], ["k1", "k1", "k2", "k3"]);
    assert_eq!(&firsts[6..], ["k2", "k3", "k1", "k2", "k3", "k1"]);
}

#[test]
fn a_formula_in_the_rows_refuses_and_one_without_sorts() {
    let (document, frame) = table_of(&TDY1);
    let with = run(
        &document,
        Command::SetCellFormula {
            frame,
            row: 3,
            column: 2,
            formula: Some("A1".into()),
        },
    );
    let error = apply(&with, &sort_rows(frame, None)).expect_err("refused");
    assert_eq!(error, FORMULA_IN_SORT);
    // Rows outside the formula's row sort.
    apply(&with, &sort_rows(frame, Some(4..12))).expect("sorts");
}

#[test]
fn connected_cells_ask_and_continue_disconnects_them() {
    let (document, frame) = table_of(&TDY1);
    let joined = run(
        &document,
        Command::EditTable {
            frame,
            edit: TableEdit::Connect(CellRange {
                rows: 10..12,
                columns: 0..2,
            }),
        },
    );
    let error = apply(&joined, &sort_rows(frame, None)).expect_err("asks");
    assert_eq!(error, SORT_CONNECTED);
    assert!(is_question(error));
    let command = Command::EditTable {
        frame,
        edit: TableEdit::SortRecords {
            rows: None,
            settings: SortSettings {
                confirmed: true,
                ..by(vec![key(1, SortKind::Alphanumeric, SortWord::First)], false)
            },
        },
    };
    let sorted = run(&joined, command);
    assert!(
        table(&sorted, frame)
            .cells
            .iter()
            .all(|cell| cell.span == amipro_document::CellSpan::SINGLE)
    );
}

#[test]
fn records_of_two_rows_move_together() {
    let (document, frame) = table_of(&[
        ["b", "1", ""],
        ["b", "2", ""],
        ["a", "3", ""],
        ["a", "4", ""],
    ]);
    let command = Command::EditTable {
        frame,
        edit: TableEdit::SortRecords {
            rows: None,
            settings: SortSettings {
                record: 2,
                ..by(vec![key(1, SortKind::Alphanumeric, SortWord::All)], false)
            },
        },
    };
    let sorted = run(&document, command);
    let seconds: Vec<String> = (0..4)
        .map(|row| cell_text(&sorted, frame, row, 1))
        .collect();
    assert_eq!(seconds, ["3", "4", "1", "2"]);
}

fn paragraphs(editor: &Editor) -> Vec<String> {
    editor
        .document()
        .main_flow()
        .paragraphs
        .iter()
        .map(|p| p.text.clone())
        .collect()
}

fn lines(texts: &[&str]) -> Editor {
    Editor::new(Document::with_paragraphs(
        texts.iter().map(|t| (*t, StyleId(1))),
    ))
}

#[test]
fn paragraphs_sort_as_one_undo_step_and_undo_restores_them() {
    let mut editor = lines(&["head", "pear 2", "fig 1", "apple 3", "tail"]);
    let flow = editor.document().main_flow_id();
    let before = paragraphs(&editor);
    editor
        .execute(&Command::SortParagraphs {
            flow,
            first: 1,
            last: 3,
            settings: by(vec![key(1, SortKind::Alphanumeric, SortWord::First)], false),
        })
        .unwrap();
    assert_eq!(
        paragraphs(&editor),
        ["head", "apple 3", "fig 1", "pear 2", "tail"]
    );
    assert!(editor.undo().unwrap());
    assert_eq!(paragraphs(&editor), before);
    assert!(editor.redo().unwrap());
    assert_eq!(paragraphs(&editor)[1], "apple 3");
}

#[test]
fn a_break_in_the_text_asks_and_continue_sorts_across_it() {
    let mut editor = lines(&["b", "a", "c"]);
    let flow = editor.document().main_flow_id();
    editor
        .execute(&Command::InsertBreak {
            at: TextPosition {
                flow,
                paragraph: 1,
                byte: 1,
            },
            kind: BreakKind::Page,
            centre: false,
            layout: None,
        })
        .unwrap();
    let settings = by(vec![key(1, SortKind::Alphanumeric, SortWord::All)], false);
    let ask = Command::SortParagraphs {
        flow,
        first: 0,
        last: 2,
        settings: settings.clone(),
    };
    assert_eq!(editor.execute(&ask).unwrap_err(), SORT_ACROSS_BREAK);
    editor
        .execute(&Command::SortParagraphs {
            flow,
            first: 0,
            last: 2,
            settings: SortSettings {
                confirmed: true,
                ..settings
            },
        })
        .unwrap();
    assert_eq!(paragraphs(&editor).len(), 3);
}

#[test]
fn a_sorted_text_is_refused_as_nothing_to_sort() {
    let mut editor = lines(&["a", "b", "c"]);
    let flow = editor.document().main_flow_id();
    let error = editor
        .execute(&Command::SortParagraphs {
            flow,
            first: 0,
            last: 2,
            settings: by(vec![key(1, SortKind::Alphanumeric, SortWord::All)], false),
        })
        .unwrap_err();
    assert_eq!(error, amipro_editing::sort::SORT_NOTHING);
}
