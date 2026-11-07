//! Making and editing tables (`docs/Design/UI_SPEC.md` §32): each command
//! against what Ami Pro 3.1 wrote for the same change in the VM fixtures
//! TE1–TE20 (`docs/research/TABLE_EDIT_NOTES.md`), one step each, the
//! document before it untouched, and the result written as Ami Pro writes
//! it and read back equal.

use amipro_document::{
    CellContent, CellLeader, CellMargins, CellSpan, Document, FlowId, FrameContent, FrameId,
    Placement, Sides, StyleId, Table,
};
use amipro_editing::tables::{
    CellChange, CellRange, LinesChange, NewTable, TableDefaults, TableEdit, TableStyle,
    TableTarget, TrackSizes, fit_width, grid_size,
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

/// TE1's start: "Above the table." in a new document.
fn above() -> Document {
    Document::with_paragraphs([("Above the table.", StyleId(1))])
}

fn end_of_text(document: &Document) -> TextPosition {
    TextPosition {
        flow: document.main_flow_id(),
        paragraph: 0,
        byte: document.main_flow().paragraphs[0].text.len(),
    }
}

/// Tools › Tables 3 × 3 at the end of the line, typed A1…C3 (TE1).
fn te1() -> (Document, FrameId) {
    let document = above();
    let spec = NewTable::fitting(3, 3, 9_360, 284);
    let made = apply(
        &document,
        &Command::InsertTable {
            target: TableTarget::Text(end_of_text(&document)),
            table: spec,
        },
    )
    .unwrap();
    let mut document = made.document;
    let frame = *document.frame_stack().last().unwrap();
    for row in 0..3 {
        for column in 0..3 {
            let text = format!("{}{}", ['A', 'B', 'C'][column], row + 1);
            let flow = cell_flow(&document, frame, row, column);
            document = run(
                &document,
                Command::ReplaceText {
                    start: at(flow),
                    end: at(flow),
                    text,
                },
            );
        }
    }
    (document, frame)
}

fn at(flow: FlowId) -> TextPosition {
    TextPosition {
        flow,
        paragraph: 0,
        byte: 0,
    }
}

fn table(document: &Document, frame: FrameId) -> &Table {
    match &document.frame(frame).unwrap().content {
        FrameContent::Table(table) => table,
        other => panic!("not a table: {other:?}"),
    }
}

fn cell_flow(document: &Document, frame: FrameId, row: usize, column: usize) -> FlowId {
    let table = table(document, frame);
    let CellContent::Text(flow) = table.cells[row * table.columns.len() + column].content;
    flow
}

fn cell_text(document: &Document, frame: FrameId, row: usize, column: usize) -> String {
    document
        .flow(cell_flow(document, frame, row, column))
        .unwrap()
        .paragraphs[0]
        .text
        .clone()
}

fn edit(document: &Document, frame: FrameId, edit: TableEdit) -> Document {
    run(document, Command::EditTable { frame, edit })
}

/// The written `[tbl]` section and the table frame's record, and that the
/// copy reads back equal (Save As lists nothing).
fn written(document: &Document) -> (String, Vec<String>) {
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
    assert_eq!(sam_losses(document, &OPTIONS).unwrap(), []);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let start = text.find("\t[tbl]").unwrap();
    let end = text.find("\t[tble]").unwrap();
    let frame_start = text.find("[frm]").unwrap();
    let frame: Vec<String> = text[frame_start..]
        .lines()
        .skip(1)
        .take(6)
        .map(|line| line.trim().to_owned())
        .collect();
    (text[start..end].to_owned(), frame)
}

fn records(section: &str) -> Vec<String> {
    section
        .lines()
        .map(str::trim)
        .filter(|line| line.split(' ').all(|value| value.parse::<i64>().is_ok()))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_new_table_is_ami_pros_te1() {
    let document = above();
    let (made, frame) = te1();
    // The document before it is untouched; one step makes the table.
    assert!(document.frame_stack().is_empty());
    let table = table(&made, frame);
    assert_eq!(table.columns, [3_019; 3], "fit_width: (9360 - 43) / 3 - 86");
    assert_eq!(fit_width(9_360, 3, 86), 3_019);
    assert_eq!(table.rows, [284; 3]);
    assert_eq!(table.cell_margins, CellMargins::default());
    // The anchor at the caret, in the paragraph (TE1: "Above the
    // table.<:t0>").
    let paragraph = &made.main_flow().paragraphs[0];
    assert_eq!(paragraph.text, "Above the table.\u{FFFC}");
    let Placement::Inline { size, .. } = made.frame(frame).unwrap().placement else {
        panic!("a page table is a Flow with text frame");
    };
    assert_eq!(
        (size.width, size.height),
        (43 + 3 * (3_019 + 86), 43 + 3 * 370)
    );
    // Every cell in Table Text, made from Body Text.
    let style = made.flow(cell_flow(&made, frame, 0, 0)).unwrap().paragraphs[0].style;
    assert_eq!(made.styles()[&style].name, "Table Text");
    let (section, record) = written(&made);
    assert_eq!(records(&section)[0], "3 3 284 86 3019 86 1 43 43");
    assert_eq!(record[1], (0x18_00C4).to_string(), "{record:?}");
    assert_eq!(
        records(&section)[1..4],
        [
            "0 0 0 0 0 0 0 1 0 0 0 0",
            "0 1 0 0 0 0 0 1 0 0 0 0",
            "0 2 0 0 0 0 0 1 0 0 0 0"
        ]
    );
}

#[test]
fn a_table_goes_into_an_empty_frame_te19() {
    let document = above();
    let made = apply(
        &document,
        &Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: amipro_document::Rect::new(1_440, 4_320, 5_760, 5_760),
            },
            style: Default::default(),
            content: amipro_editing::NewFrameContent::Text,
            anchor: None,
            layout: None,
        },
    )
    .unwrap()
    .document;
    let frame = made.frame_stack()[0];
    let rect = made.frame(frame).unwrap().placement;
    let with = run(
        &made,
        Command::InsertTable {
            target: TableTarget::Frame(frame),
            table: NewTable::fitting(2, 3, 4_320 - 288, 284),
        },
    );
    assert_eq!(
        with.frame(frame).unwrap().placement,
        rect,
        "the frame stays"
    );
    assert_eq!(table(&with, frame).columns.len(), 2);
    let (section, record) = written(&with);
    assert_eq!(record[1], (0x1_00C4).to_string(), "TE19: {record:?}");
    assert!(records(&section)[0].starts_with("3 2 284 86 "));
    // A frame with text takes no table.
    let typed = run(
        &made,
        Command::ReplaceText {
            start: at(match made.frame(frame).unwrap().content {
                FrameContent::Text(flow) => flow,
                _ => unreachable!(),
            }),
            end: at(match made.frame(frame).unwrap().content {
                FrameContent::Text(flow) => flow,
                _ => unreachable!(),
            }),
            text: "x".into(),
        },
    );
    assert!(
        apply(
            &typed,
            &Command::InsertTable {
                target: TableTarget::Frame(frame),
                table: NewTable::fitting(2, 3, 4_032, 284),
            }
        )
        .is_err()
    );
}

#[test]
fn rows_and_columns_are_inserted_and_deleted_as_ami_pro_does_te2_te3_te4() {
    let (document, frame) = te1();
    let width = |document: &Document| grid_size(table(document, frame)).width;
    let height = |document: &Document| grid_size(table(document, frame)).height;
    // TE2: a row after row 1 at the defaults; no record for it.
    let te2 = edit(
        &document,
        frame,
        TableEdit::InsertRows {
            at: 2,
            count: 1,
            like: Some(1),
        },
    );
    assert_eq!(height(&te2), height(&document) + 370);
    assert_eq!(cell_text(&te2, frame, 3, 0), "A3");
    assert_eq!(cell_text(&te2, frame, 2, 0), "");
    let (section, _) = written(&te2);
    assert!(
        records(&section)
            .iter()
            .all(|record| !record.starts_with("2 "))
    );

    // TE3: column 1 deleted; the table narrows by its width and gutter.
    let te3 = edit(&document, frame, TableEdit::DeleteColumns(1..2));
    assert_eq!(width(&te3), width(&document) - (3_019 + 86));
    assert_eq!(cell_text(&te3, frame, 1, 1), "C2");
    written(&te3);

    // TE4: a column before column 1 at the default width; the table grows.
    let te4 = edit(
        &te3,
        frame,
        TableEdit::InsertColumns {
            at: 1,
            count: 1,
            like: Some(1),
            room: Some(9_360),
        },
    );
    assert_eq!(table(&te4, frame).columns, [3_019; 3]);
    assert_eq!(cell_text(&te4, frame, 0, 2), "C1");
    written(&te4);

    // TD4: a column with no room: the table keeps its width, the columns
    // narrow in proportion.
    let narrowed = edit(
        &document,
        frame,
        TableEdit::InsertColumns {
            at: 3,
            count: 1,
            like: Some(2),
            room: Some(9_360),
        },
    );
    let columns = &table(&narrowed, frame).columns;
    assert_eq!(columns.len(), 4);
    assert!(width(&narrowed) <= 9_360, "{}", width(&narrowed));
    assert!(
        columns.windows(2).all(|pair| pair[0] == pair[1]),
        "{columns:?}"
    );
    written(&narrowed);

    // Deleting every row deletes the table and its anchor.
    let gone = edit(&document, frame, TableEdit::DeleteRows(0..3));
    assert!(gone.frame(frame).is_none());
    assert_eq!(gone.main_flow().paragraphs[0].text, "Above the table.");
}

#[test]
fn sizes_and_gutters_are_own_as_column_row_size_writes_them_te5_te6_te7() {
    let (document, frame) = te1();
    let te5 = edit(
        &document,
        frame,
        TableEdit::SetSizes {
            rows: 0..0,
            columns: 1..2,
            sizes: TrackSizes {
                width: Some(2_160),
                ..TrackSizes::default()
            },
        },
    );
    let (section, _) = written(&te5);
    assert!(
        records(&section).contains(&"1 2160 86 2 0".into()),
        "{section}"
    );
    let te6 = edit(
        &document,
        frame,
        TableEdit::SetSizes {
            rows: 0..0,
            columns: 1..2,
            sizes: TrackSizes {
                column_gutter: Some(288),
                ..TrackSizes::default()
            },
        },
    );
    let (section, _) = written(&te6);
    assert!(
        records(&section).contains(&"1 3019 288 4 0".into()),
        "{section}"
    );
    let te7 = edit(
        &document,
        frame,
        TableEdit::SetSizes {
            rows: 1..2,
            columns: 0..0,
            sizes: TrackSizes {
                row_gutter: Some(144),
                ..TrackSizes::default()
            },
        },
    );
    let (section, _) = written(&te7);
    assert!(
        records(&section).contains(&"1 284 144 6 0 3 0".into()),
        "{section}"
    );
    // Reset: the defaults again.
    let reset = edit(
        &te5,
        frame,
        TableEdit::ResetSizes {
            rows: 0..3,
            columns: 0..3,
        },
    );
    assert_eq!(table(&reset, frame).columns, [3_019; 3]);
    assert!(table(&reset, frame).layout.own_widths.is_empty());
}

#[test]
fn modify_table_layout_sets_defaults_and_options_te12_to_te17() {
    let (document, frame) = te1();
    let te5 = edit(
        &document,
        frame,
        TableEdit::SetSizes {
            rows: 0..0,
            columns: 1..2,
            sizes: TrackSizes {
                width: Some(2_160),
                ..TrackSizes::default()
            },
        },
    );
    let defaults = TableDefaults {
        column_width: 2_160,
        column_gutter: 86,
        row_height: 284,
        row_gutter: 86,
        automatic: true,
        span_pages: false,
        center: true,
        honor_protection: true,
        fixed_sizes: true,
        line_around: Some(3),
        measured_rows: Vec::new(),
    };
    // TE16: the default width applies to the columns without their own.
    let te16 = edit(&te5, frame, TableEdit::SetDefaults(defaults.clone()));
    assert_eq!(table(&te16, frame).columns, [2_160; 3]);
    let (section, record) = written(&te16);
    // 0x485: Automatic, Center, Honor protection, Disable mouse sizing.
    assert_eq!(records(&section)[0], "3 3 284 86 2160 86 1157 43 43");
    // TE14: Line around, style 3: flags 0x1900C4, the frame 80 taller.
    assert_eq!(record[1], (0x19_00C4).to_string(), "{record:?}");
    let frame_of = te16.frame(frame).unwrap();
    assert_eq!(frame_of.style.border.sides, Sides::ALL);
    assert_eq!(frame_of.style.interior.margins.top, 40);
    // TE17: Automatic off keeps the rows as laid out; the default height
    // is for new rows.
    let off = edit(
        &document,
        frame,
        TableEdit::SetDefaults(TableDefaults {
            automatic: false,
            row_height: 720,
            line_around: None,
            center: false,
            honor_protection: false,
            fixed_sizes: false,
            column_width: 3_019,
            measured_rows: vec![284, 568, 284],
            ..defaults
        }),
    );
    let read = table(&off, frame);
    assert_eq!(read.rows, [284, 568, 284]);
    assert!(!read.row_options.automatic);
    let (section, _) = written(&off);
    assert_eq!(records(&section)[0], "3 3 720 86 3019 86 0 43 43");
}

#[test]
fn connected_cells_keep_the_hidden_text_te8_te9() {
    let (document, frame) = te1();
    let te8 = edit(
        &document,
        frame,
        TableEdit::Connect(CellRange::spanning((0, 1), (1, 2))),
    );
    let read = table(&te8, frame);
    assert_eq!(
        read.cells[1].span,
        CellSpan {
            rows: 2,
            columns: 2
        }
    );
    assert!(read.cells[5].span.is_covered());
    assert_eq!(cell_text(&te8, frame, 1, 2), "C2", "hidden, kept");
    let (section, _) = written(&te8);
    let records = records(&section);
    for record in [
        "0 1 384 2 2 0 0 1 0 0 0 0",
        "0 2 128 0 1 0 0 1 0 0 0 0",
        "1 1 128 1 0 0 0 1 0 0 0 0",
        "1 2 128 1 1 0 0 1 0 0 0 0",
    ] {
        assert!(records.contains(&record.into()), "{record} in {records:?}");
    }
    let te9 = edit(&te8, frame, TableEdit::Disconnect { row: 0, column: 1 });
    assert!(
        table(&te9, frame)
            .cells
            .iter()
            .all(|cell| cell.span == CellSpan::SINGLE)
    );
    assert_eq!(cell_text(&te9, frame, 1, 1), "B2");
    // A range crossing a block takes all of it (`14a0:1052`).
    let grown = edit(
        &te8,
        frame,
        TableEdit::Connect(CellRange::spanning((1, 0), (1, 1))),
    );
    assert_eq!(
        table(&grown, frame).cells[0].span,
        CellSpan {
            rows: 2,
            columns: 3
        }
    );
    written(&grown);
}

#[test]
fn leaders_protection_lines_and_fills_te10_te11_tl1_tl4() {
    let (document, frame) = te1();
    let b2 = CellRange::cell(1, 1);
    let dots = edit(
        &document,
        frame,
        TableEdit::FormatCells {
            cells: b2.clone(),
            change: CellChange::Leader(CellLeader::Dots),
        },
    );
    let (section, _) = written(&dots);
    assert!(records(&section).contains(&"1 1 16384 0 0 0 0 1 0 0 0 0".into()));
    let protected = edit(
        &document,
        frame,
        TableEdit::FormatCells {
            cells: b2.clone(),
            change: CellChange::Protect(true),
        },
    );
    let (section, _) = written(&protected);
    assert!(records(&section).contains(&"1 1 0 0 0 0 0 1 1 0 0 0".into()));
    // Under Honor protection the others skip it; Protect does not.
    let mut honoured = protected.clone();
    if let FrameContent::Table(table) = &mut honoured.frame_mut(frame).unwrap().content {
        table.layout.honor_protection = true;
    }
    let skipped = edit(
        &honoured,
        frame,
        TableEdit::FormatCells {
            cells: CellRange::spanning((1, 0), (1, 2)),
            change: CellChange::Fill(Some(0x00_FFFF)),
        },
    );
    let fills: Vec<_> = table(&skipped, frame).cells[3..6]
        .iter()
        .map(|cell| cell.format.fill)
        .collect();
    assert_eq!(fills, [Some(0xFFFF), None, Some(0xFFFF)]);
    let (section, _) = written(&skipped);
    assert!(records(&section).contains(&"1 0 512 0 0 9 0 1 0 65535 0 0".into()));

    // TL1: All, style 1, on every cell.
    let all = edit(
        &document,
        frame,
        TableEdit::FormatCells {
            cells: CellRange::spanning((0, 0), (2, 2)),
            change: CellChange::Lines(LinesChange {
                all: Some(true),
                style: 1,
                ..LinesChange::default()
            }),
        },
    );
    assert!(
        table(&all, frame)
            .cells
            .iter()
            .all(|cell| cell.format.lines.word() == 0x1111)
    );
    written(&all);
    // A line wider than its gutter widens it (`14b0:09c8`): style 7 is
    // 240 twips.
    let wide = edit(
        &document,
        frame,
        TableEdit::FormatCells {
            cells: CellRange::cell(0, 0),
            change: CellChange::Lines(LinesChange {
                all: Some(true),
                style: 7,
                ..LinesChange::default()
            }),
        },
    );
    let read = table(&wide, frame);
    assert_eq!(read.column_gutter(0), 240);
    assert_eq!(read.row_gutter(0), 240);
    assert_eq!((read.spacing.top, read.spacing.left), (240, 240));
    written(&wide);
    // Outline sets only the range's edge; an unchecked side removes.
    let outline = edit(
        &all,
        frame,
        TableEdit::FormatCells {
            cells: CellRange::spanning((0, 0), (1, 1)),
            change: CellChange::Lines(LinesChange {
                outline: Some(true),
                left: Some(false),
                right: Some(false),
                top: Some(false),
                bottom: Some(false),
                style: 2,
                ..LinesChange::default()
            }),
        },
    );
    let lines =
        |row: usize, column: usize| table(&outline, frame).cells[row * 3 + column].format.lines;
    assert_eq!(
        (lines(0, 0).left, lines(0, 0).top, lines(0, 0).right),
        (2, 2, 0)
    );
    assert_eq!(
        (lines(1, 1).right, lines(1, 1).bottom, lines(1, 1).left),
        (2, 2, 0)
    );
    assert_eq!(lines(2, 2).word(), 0x1111, "outside the range");
}

#[test]
fn clearing_moving_distributing_and_styles() {
    let (document, frame) = te1();
    let cleared = edit(
        &document,
        frame,
        TableEdit::ClearCells(CellRange::spanning((0, 0), (1, 1))),
    );
    assert_eq!(cell_text(&cleared, frame, 0, 0), "");
    assert_eq!(cell_text(&cleared, frame, 1, 1), "");
    assert_eq!(cell_text(&cleared, frame, 2, 2), "C3");
    assert_eq!(table(&cleared, frame).rows.len(), 3);
    written(&cleared);

    // Alt+Down: row 0 after row 1.
    let moved = edit(
        &document,
        frame,
        TableEdit::MoveRows {
            rows: 0..1,
            to: 2,
            copy: false,
        },
    );
    assert_eq!(cell_text(&moved, frame, 0, 0), "A2");
    assert_eq!(cell_text(&moved, frame, 1, 0), "A1");
    // A copy of column 0 at the end.
    let copied = edit(
        &document,
        frame,
        TableEdit::MoveColumns {
            columns: 0..1,
            to: 3,
            copy: true,
        },
    );
    assert_eq!(table(&copied, frame).columns.len(), 4);
    assert_eq!(cell_text(&copied, frame, 2, 3), "A3");
    assert_eq!(cell_text(&copied, frame, 2, 0), "A3");
    assert_ne!(
        cell_flow(&copied, frame, 2, 3),
        cell_flow(&copied, frame, 2, 0)
    );
    written(&copied);

    let sized = edit(
        &document,
        frame,
        TableEdit::SetSizes {
            rows: 0..0,
            columns: 0..1,
            sizes: TrackSizes {
                width: Some(1_000),
                ..TrackSizes::default()
            },
        },
    );
    let even = edit(
        &sized,
        frame,
        TableEdit::Distribute {
            rows: None,
            columns: Some(0..3),
        },
    );
    let columns = &table(&even, frame).columns;
    assert_eq!(columns.iter().sum::<i32>(), 1_000 + 2 * 3_019);
    assert!(columns.iter().max().unwrap() - columns.iter().min().unwrap() <= 1);

    let banded = edit(
        &document,
        frame,
        TableEdit::ApplyStyle(TableStyle::BandedRows),
    );
    let read = table(&banded, frame);
    assert_eq!(read.cells[3].format.fill, Some(0xF7_F7F7));
    assert_eq!(read.cells[0].format.fill, None);
    let header = edit(
        &document,
        frame,
        TableEdit::ApplyStyle(TableStyle::HeaderRow),
    );
    assert!(table(&header, frame).row_options.headings.contains(&0));
    written(&header);

    let headings = edit(
        &document,
        frame,
        TableEdit::Headings {
            rows: 0..1,
            on: true,
        },
    );
    assert!(table(&headings, frame).row_options.headings.contains(&0));
}

#[test]
fn every_table_command_is_one_undo_step() {
    let (document, frame) = te1();
    let mut current = document.clone();
    let mut history: History<()> = History::new();
    for command in [
        TableEdit::InsertRows {
            at: 0,
            count: 2,
            like: None,
        },
        TableEdit::Connect(CellRange::spanning((0, 0), (1, 1))),
        TableEdit::FormatCells {
            cells: CellRange::cell(2, 2),
            change: CellChange::Fill(Some(0)),
        },
        TableEdit::DeleteColumns(2..3),
    ] {
        let before = current.clone();
        history
            .execute(
                &mut current,
                (),
                &Command::EditTable {
                    frame,
                    edit: command,
                },
            )
            .unwrap();
        assert_ne!(current, before);
        history.undo(&mut current, ());
        assert_eq!(current, before);
        history.redo(&mut current, ());
    }
}

/// TD7: tabbed paragraphs become a table where they stood, and back.
#[test]
fn text_becomes_a_table_and_a_table_text() {
    let document = Document::with_paragraphs([
        ("Before", StyleId(1)),
        ("Name\tPlace\tYear", StyleId(1)),
        ("Ami\tPro", StyleId(1)),
        ("After", StyleId(1)),
    ]);
    let made = run(
        &document,
        Command::TextToTable {
            first: 1,
            last: 2,
            width: 9_360,
            row_height: 284,
        },
    );
    let paragraphs: Vec<_> = made
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect();
    assert_eq!(paragraphs, ["Before", "\u{FFFC}", "After"]);
    let frame = *made.frame_stack().last().unwrap();
    let read = table(&made, frame);
    assert_eq!((read.rows.len(), read.columns.len()), (2, 3));
    assert_eq!(cell_text(&made, frame, 0, 2), "Year");
    assert_eq!(cell_text(&made, frame, 1, 1), "Pro");
    assert_eq!(cell_text(&made, frame, 1, 2), "");
    written(&made);
    let back = run(&made, Command::TableToText { frame });
    assert!(back.frame(frame).is_none());
    let paragraphs: Vec<_> = back
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect();
    assert_eq!(
        paragraphs,
        ["Before", "Name\tPlace\tYear", "Ami\tPro\t", "After"]
    );
}

/// Cells pasted as Ami Pro pastes them (`13b0:4065`, `14c8:044f`): over
/// the cells from the caret's, repeated over a larger selection that is a
/// multiple of the block, refused over one that is not, rows added to a
/// table in the text, cut at the last row otherwise.
#[test]
fn copied_cells_paste_over_cells_and_repeat_over_a_selection() {
    use amipro_editing::tables::CellBlock;
    let (document, frame) = te1();
    let block = CellBlock::copy(
        &document,
        table(&document, frame),
        &CellRange::spanning((0, 0), (0, 1)),
    );
    assert_eq!(block.text(), "A1\tB1");
    // At (2,1): B3 and C3 become A1 and B1.
    let pasted = edit(
        &document,
        frame,
        TableEdit::PasteCells {
            at: (2, 1),
            block: block.clone(),
            area: None,
            add_rows: true,
        },
    );
    assert_eq!(cell_text(&pasted, frame, 2, 1), "A1");
    assert_eq!(cell_text(&pasted, frame, 2, 2), "B1");
    // A two-row block from the last row adds a row to a table in the text.
    let two = CellBlock::copy(
        &document,
        table(&document, frame),
        &CellRange::spanning((0, 0), (1, 0)),
    );
    let grown = edit(
        &document,
        frame,
        TableEdit::PasteCells {
            at: (2, 0),
            block: two.clone(),
            area: None,
            add_rows: true,
        },
    );
    assert_eq!(table(&grown, frame).rows.len(), 4);
    assert_eq!(cell_text(&grown, frame, 3, 0), "A2");
    let cut = edit(
        &document,
        frame,
        TableEdit::PasteCells {
            at: (2, 0),
            block: two,
            area: None,
            add_rows: false,
        },
    );
    assert_eq!(table(&cut, frame).rows.len(), 3);
    // A 1 × 2 block over a 2 × 2 selection repeats; over 2 × 3 it is
    // refused (Ami Pro: "Shade range does not match copied data range.").
    let repeated = edit(
        &document,
        frame,
        TableEdit::PasteCells {
            at: (1, 0),
            block: block.clone(),
            area: Some(CellRange::spanning((1, 0), (2, 1))),
            add_rows: true,
        },
    );
    assert_eq!(cell_text(&repeated, frame, 2, 1), "B1");
    assert!(
        apply(
            &document,
            &Command::EditTable {
                frame,
                edit: TableEdit::PasteCells {
                    at: (0, 0),
                    block,
                    area: Some(CellRange::spanning((0, 0), (1, 2))),
                    add_rows: true,
                },
            }
        )
        .is_err()
    );
    written(&repeated);
}

/// The twins (TE.12): Ami Pro's TE1 changed by each command family here and
/// written as Ami Pro documents (`ATE*.SAM` in `AMIPRO_TWIN_OUT`) for Ami Pro
/// 3.1 to open; with the VM fixtures (`AMIPRO_VM_LAB`), each table's
/// records are compared with the fixture Ami Pro made for the same change.
#[test]
#[ignore = "needs the private VM fixtures (AMIPRO_VM_LAB) and AMIPRO_TWIN_OUT"]
fn table_twins() {
    let (Some(lab), Some(out)) = (
        std::env::var_os("AMIPRO_VM_LAB"),
        std::env::var_os("AMIPRO_TWIN_OUT"),
    ) else {
        eprintln!("AMIPRO_VM_LAB and AMIPRO_TWIN_OUT are not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let out = std::path::PathBuf::from(out);
    let te1 = amipro_formats::import_sam_path(lab.join("TE1.SAM"))
        .unwrap()
        .document;
    let frame = *te1.frame_stack().last().unwrap();
    let section = |document: &Document| {
        let text = String::from_utf8_lossy(&sam_document(document, &OPTIONS)).into_owned();
        let start = text.find("\t[tbl]").unwrap();
        let end = text.find("\t[tble]").unwrap();
        records(&text[start..end])
    };
    let fixture = |name: &str| {
        let text =
            String::from_utf8_lossy(&std::fs::read(lab.join(format!("{name}.SAM"))).unwrap())
                .into_owned();
        let start = text.find("\t[tbl]").unwrap();
        let end = text.find("\t[tble]").unwrap();
        records(&text[start..end])
    };
    let b2 = CellRange::cell(1, 1);
    let twins: Vec<(&str, Option<&str>, Document)> = vec![
        (
            "ATE2",
            Some("TE2"),
            edit(
                &te1,
                frame,
                TableEdit::InsertRows {
                    at: 2,
                    count: 1,
                    like: Some(1),
                },
            ),
        ),
        (
            "ATE3",
            Some("TE3"),
            edit(&te1, frame, TableEdit::DeleteColumns(1..2)),
        ),
        (
            "ATE5",
            Some("TE5"),
            edit(
                &te1,
                frame,
                TableEdit::SetSizes {
                    rows: 0..0,
                    columns: 1..2,
                    sizes: TrackSizes {
                        width: Some(2_160),
                        ..TrackSizes::default()
                    },
                },
            ),
        ),
        (
            "ATE8",
            Some("TE8"),
            edit(
                &te1,
                frame,
                TableEdit::Connect(CellRange::spanning((0, 1), (1, 2))),
            ),
        ),
        (
            "ATE10",
            Some("TE10"),
            edit(
                &te1,
                frame,
                TableEdit::FormatCells {
                    cells: b2.clone(),
                    change: CellChange::Leader(CellLeader::Dots),
                },
            ),
        ),
        (
            "ATE11",
            Some("TE11"),
            edit(
                &te1,
                frame,
                TableEdit::FormatCells {
                    cells: b2.clone(),
                    change: CellChange::Protect(true),
                },
            ),
        ),
        ("ATEL", None, {
            let lines = edit(
                &te1,
                frame,
                TableEdit::FormatCells {
                    cells: CellRange::spanning((0, 0), (2, 2)),
                    change: CellChange::Lines(LinesChange {
                        all: Some(true),
                        style: 3,
                        ..LinesChange::default()
                    }),
                },
            );
            edit(
                &lines,
                frame,
                TableEdit::FormatCells {
                    cells: CellRange::spanning((0, 0), (0, 2)),
                    change: CellChange::Fill(Some(0x00_FFFF)),
                },
            )
        }),
        (
            "ATES",
            None,
            edit(&te1, frame, TableEdit::ApplyStyle(TableStyle::HeaderRow)),
        ),
        ("ATEM", None, {
            let rows = edit(
                &te1,
                frame,
                TableEdit::MoveRows {
                    rows: 0..1,
                    to: 3,
                    copy: false,
                },
            );
            edit(
                &rows,
                frame,
                TableEdit::InsertColumns {
                    at: 3,
                    count: 1,
                    like: Some(2),
                    room: Some(9_360),
                },
            )
        }),
        ("ATEC", None, {
            let mut defaults = TableDefaults {
                column_width: 3_017,
                column_gutter: 86,
                row_height: 284,
                row_gutter: 86,
                automatic: true,
                span_pages: false,
                center: true,
                honor_protection: false,
                fixed_sizes: false,
                line_around: Some(3),
                measured_rows: Vec::new(),
            };
            defaults.column_width = 2_160;
            edit(&te1, frame, TableEdit::SetDefaults(defaults))
        }),
    ];
    for (name, made_by_ami_pro, document) in twins {
        let path = out.join(format!("{name}.SAM"));
        std::fs::write(&path, sam_document(&document, &OPTIONS)).unwrap();
        eprintln!("wrote {}", path.display());
        if let Some(fixture_name) = made_by_ami_pro {
            let ours = section(&document);
            let theirs = fixture(fixture_name);
            // Ami Pro's measured-row records aside (flag 2 rows it adds when
            // it lays a table out), the cells and the header agree.
            let cells = |records: &[String]| -> Vec<String> {
                records
                    .iter()
                    .filter(|record| record.split(' ').count() >= 12)
                    .cloned()
                    .collect()
            };
            assert_eq!(
                cells(&ours),
                cells(&theirs),
                "{name} against {fixture_name}"
            );
            assert_eq!(ours[0], theirs[0], "{name}'s header");
        }
    }
    // A table made here in a new document, as TE1 was made in Ami Pro.
    let document = above();
    let made = run(
        &document,
        Command::InsertTable {
            target: TableTarget::Text(end_of_text(&document)),
            table: NewTable::fitting(3, 3, 9_360, 284),
        },
    );
    std::fs::write(out.join("ATE1.SAM"), sam_document(&made, &OPTIONS)).unwrap();
    // Paragraphs made into a table.
    let text = Document::with_paragraphs([
        ("Name\tPlace", StyleId(1)),
        ("Ami\tPro", StyleId(1)),
        ("After", StyleId(1)),
    ]);
    let converted = run(
        &text,
        Command::TextToTable {
            first: 0,
            last: 1,
            width: 9_360,
            row_height: 284,
        },
    );
    std::fs::write(out.join("ATET.SAM"), sam_document(&converted, &OPTIONS)).unwrap();
}
