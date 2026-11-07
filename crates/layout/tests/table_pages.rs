//! Tables across columns and pages, each rule from a VM fixture
//! (`docs/research/TABLE_PAGES_NOTES.md`), with the deterministic
//! [`FixedShaper`]: Body Text sets 240-twip lines, with no space around
//! paragraphs, so a one-line row and its gutter take 326 twips. The page is
//! Letter with 1-inch margins: the text area runs from 1,440 to 14,400.

use std::collections::HashMap;

use amipro_document::{
    CellContent, CellMargins, CellSpan, Document, FlowId, Frame, FrameContent, FrameId, FrameStyle,
    InlineObject, LayoutBox, LegacyData, Margins, Placement, Rect, Size, Spacing, StyleId, Table,
    TableCell, TableRows, TableSpacing, Wrap,
};
use amipro_editing::{Command, TextPosition, apply};
use amipro_layout::{
    Bounds, Container, DocumentLayout, FixedShaper, FrameBoxKind, IncrementalLayout, PageBox,
    ShapeCache, TablePart, layout_document,
};

const LINE: f32 = 240.0;
const GUTTER: f32 = 86.0;
const INSET: f32 = 43.0;
const ROW: f32 = LINE + GUTTER;
const TOP: f32 = 1_440.0;
const BOTTOM: f32 = 14_400.0;
const OBJECT: char = '\u{FFFC}';

fn layout(document: &Document) -> DocumentLayout {
    document.validate().unwrap();
    layout_document(document, &mut FixedShaper, &mut ShapeCache::default())
}

/// A table style as Ami Pro's: no margins; No wrap beside in the text
/// (a With para above frame cannot wrap around), Wrap around on the page.
fn table_style(wrap: Wrap) -> FrameStyle {
    let mut style = FrameStyle {
        wrap,
        ..FrameStyle::default()
    };
    style.interior = LayoutBox::single_column(Margins::uniform(0));
    style
}

struct Fixture {
    document: Document,
    table: FrameId,
    flows: Vec<FlowId>,
}

/// `before` one-line paragraphs, then a paragraph holding only the anchor of
/// a table of `rows` × `columns` one-line cells ("RrCc") 8,000 twips wide,
/// then "After the table.". `cells` replaces some cells' text (paragraphs
/// separated by newlines).
fn fixture(
    before: usize,
    rows: usize,
    columns: usize,
    cells: &[((usize, usize), String)],
    options: TableRows,
) -> Fixture {
    fixture_placed(before, rows, columns, cells, options, false)
}

/// As [`fixture`], the table placed as Ami Pro places page tables, Flow
/// with text on a line of its own, when `inline`.
fn fixture_placed(
    before: usize,
    rows: usize,
    columns: usize,
    cells: &[((usize, usize), String)],
    options: TableRows,
    inline: bool,
) -> Fixture {
    let mut text: Vec<String> = (1..=before)
        .map(|line| format!("Line {line} of the text"))
        .collect();
    text.push(String::new());
    text.push("After the table.".into());
    let mut document = Document::from_plain_text(&text.join("\n"));
    let body = document.body_text_style();
    document.styles_mut().get_mut(&body).unwrap().spacing = Spacing::default();
    let given: HashMap<(usize, usize), String> = cells.iter().cloned().collect();
    let mut table_cells = Vec::new();
    let mut flows = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            let flow = document.create_flow(StyleId(1));
            let cell_text = given
                .get(&(row, column))
                .cloned()
                .unwrap_or_else(|| format!("R{row}C{column}"));
            let paragraphs: Vec<&str> = cell_text.split('\n').collect();
            {
                let content = document.flow_mut(flow).unwrap();
                content.paragraph_mut(0).unwrap().text = paragraphs[0].into();
            }
            for extra in &paragraphs[1..] {
                let mut paragraph = document.flow(flow).unwrap().paragraphs[0].as_ref().clone();
                paragraph.id = document.allocate_paragraph_id();
                paragraph.text = (*extra).into();
                document
                    .flow_mut(flow)
                    .unwrap()
                    .paragraphs
                    .push(std::sync::Arc::new(paragraph));
            }
            flows.push(flow);
            table_cells.push(TableCell {
                content: CellContent::Text(flow),
                span: CellSpan::SINGLE,
                frame: None,
                format: Default::default(),
                formula: None,
                legacy: LegacyData::default(),
            });
        }
    }
    let width = 8_000;
    let table = document.allocate_frame_id();
    let size = Size {
        width: width + 43,
        height: 700,
    };
    document.insert_frame(Frame {
        id: table,
        placement: if inline {
            Placement::Inline { y_offset: 0, size }
        } else {
            Placement::BelowParagraph { x_offset: 0, size }
        },
        style: table_style(if inline {
            Wrap::Around
        } else {
            Wrap::NoWrapBeside
        }),
        group: None,
        content: FrameContent::Table(Table {
            columns: vec![(width - 86 * columns as i32) / columns as i32; columns],
            rows: vec![LINE as i32; rows],
            cell_margins: CellMargins::default(),
            spacing: TableSpacing::default(),
            row_options: options,
            layout: Default::default(),
            read_rows: Default::default(),
            cells: table_cells,
            legacy: LegacyData::default(),
        }),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().push(table);
    let anchor = document.paragraph_mut(before);
    anchor.text = OBJECT.to_string();
    anchor.objects = vec![InlineObject::FrameAnchor(table)];
    Fixture {
        document,
        table,
        flows,
    }
}

fn table_mut(document: &mut Document, table: FrameId) -> &mut Table {
    let FrameContent::Table(content) = &mut document.frame_mut(table).unwrap().content else {
        panic!("a table");
    };
    content
}

/// Each part of `table`: its page number, bounds, and rows.
fn parts(result: &DocumentLayout, table: FrameId) -> Vec<(u32, Bounds, TablePart)> {
    result
        .pages
        .iter()
        .flat_map(|page| {
            page.frames
                .iter()
                .filter(|frame_box| frame_box.frame == table)
                .map(|frame_box| {
                    let FrameBoxKind::Table { part, .. } = &frame_box.kind else {
                        panic!("a table box");
                    };
                    (page.number, frame_box.bounds, part.clone())
                })
        })
        .collect()
}

fn cell_lines(page: &PageBox, table: FrameId, cell: usize) -> Vec<&amipro_layout::LineBox> {
    page.lines
        .iter()
        .filter(|line| line.container == Container::Cell { frame: table, cell })
        .collect()
}

/// The page whose main text holds paragraph `index`'s first line.
fn page_of_paragraph(result: &DocumentLayout, index: usize) -> u32 {
    result
        .pages
        .iter()
        .find(|page| {
            page.lines
                .iter()
                .any(|line| line.container == Container::Main && line.paragraph_index == index)
        })
        .map(|page| page.number)
        .unwrap()
}

#[test]
fn a_table_continues_with_whole_rows_whose_text_and_gutter_fit_tp1() {
    // 30 lines (7,200 twips), then a 40-row table with a 250-twip top inset:
    // page 1 has 5,760 twips left, 16 rows and their gutters take 5,466 of
    // them after the inset, and row 16's text (240) would fit in the 294
    // left but not with its gutter, so it starts page 2.
    let mut fixture = fixture(30, 40, 2, &[], TableRows::default());
    table_mut(&mut fixture.document, fixture.table).spacing.top = 250;
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found.len(), 2, "{found:?}");
    let (page, first, part) = &found[0];
    assert_eq!((*page, part.rows.clone()), (1, 0..16));
    assert_eq!(
        (first.y, first.height),
        (TOP + 30.0 * LINE, 250.0 + 16.0 * ROW)
    );
    // Page 2 from the column's top, with the inset again.
    let (page, second, part) = &found[1];
    assert_eq!((*page, part.rows.clone()), (2, 16..40));
    assert_eq!((second.y, second.height), (TOP, 250.0 + 24.0 * ROW));
    assert_eq!(part.heading, None);
    // Row 16's text is the first on page 2, below the inset.
    let lines = cell_lines(&result.pages[1], fixture.table, 32);
    assert_eq!(lines[0].top, TOP + 250.0);
    // The text after the table follows its last part.
    let after = result.pages[1]
        .lines
        .iter()
        .find(|line| line.container == Container::Main && line.paragraph_index == 31)
        .unwrap();
    assert_eq!(after.top, second.bottom());
    // The anchor's empty line stays where the table began, as Ami Pro's
    // page table ends page 1 on it.
    assert_eq!(page_of_paragraph(&result, 30), 1);
    assert_eq!(page_of_paragraph(&result, 31), 2);
}

#[test]
fn a_row_that_does_not_fit_moves_whole_tp3() {
    // Row 6 holds 16 lines: rows 0–5 on page 1, row 6 starts page 2.
    let tall = (1..=16)
        .map(|line| format!("Tall cell line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let fixture = fixture(30, 12, 2, &[((6, 0), tall)], TableRows::default());
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found[0].2.rows, 0..6);
    assert_eq!(found[1].2.rows, 6..12);
    assert_eq!(found[1].2.heights[0], 16.0 * LINE);
    assert_eq!(cell_lines(&result.pages[1], fixture.table, 12).len(), 16);
}

#[test]
fn a_row_taller_than_a_page_is_cut_to_it_tp4() {
    // 60 lines in row 2: on an empty page the inset, the row, and its
    // gutter fit 53 lines (43 + 12,720 + 86 ≤ 12,960); the rest is hidden,
    // and the table goes on with row 3 on page 3.
    let huge = (1..=60)
        .map(|line| format!("Huge cell line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let fixture = fixture(5, 6, 2, &[((2, 0), huge)], TableRows::default());
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    let rows: Vec<_> = found
        .iter()
        .map(|(page, _, part)| (*page, part.rows.clone()))
        .collect();
    assert_eq!(rows, [(1, 0..2), (2, 2..3), (3, 3..6)]);
    assert_eq!(found[1].2.heights, [53.0 * LINE]);
    assert!(!found[1].2.cut);
    assert_eq!(cell_lines(&result.pages[1], fixture.table, 4).len(), 53);
}

#[test]
fn rows_span_pages_split_a_row_between_lines_tp5_tp4b() {
    let tall = (1..=16)
        .map(|line| format!("Tall cell line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let options = TableRows {
        span_pages: true,
        ..TableRows::default()
    };
    let fixture = fixture(30, 12, 2, &[((6, 0), tall)], options.clone());
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    // Page 1: from 8,640, the inset and rows 0–5 end at 10,639; the row's
    // text may run to 14,314 (the bottom less its gutter): 15 lines.
    let (_, first, part) = &found[0];
    assert_eq!(part.rows, 0..7);
    assert!(part.cut);
    let shown = ((BOTTOM - GUTTER - (first.y + INSET + 6.0 * ROW)) / LINE).floor();
    assert_eq!(shown, 15.0);
    assert_eq!(part.heights[6], shown * LINE);
    assert_eq!(cell_lines(&result.pages[0], fixture.table, 12).len(), 15);
    // Page 2: the row's last line, then the rest of the table.
    let (page, second, part) = &found[1];
    assert_eq!((*page, part.rows.clone()), (2, 6..12));
    assert!(part.continued() && !part.cut);
    assert_eq!(part.shown_before[0], 15.0 * LINE);
    assert_eq!(part.heights[0], LINE);
    let lines = cell_lines(&result.pages[1], fixture.table, 12);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].top, second.y + INSET);
    // The other cell of the row showed its one line on page 1, so nothing of
    // it is on page 2.
    assert!(cell_lines(&result.pages[1], fixture.table, 13).is_empty());

    // TP4B: a row of 60 lines splits over the pages it needs.
    let huge = (1..=60)
        .map(|line| format!("Huge cell line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let fixture = fixture_with(5, 6, &[((2, 0), huge)], options);
    let result = layout(&fixture.document);
    let total: usize = result
        .pages
        .iter()
        .map(|page| cell_lines(page, fixture.table, 4).len())
        .sum();
    assert_eq!(total, 60, "every line shown once");
}

fn fixture_with(
    before: usize,
    rows: usize,
    cells: &[((usize, usize), String)],
    options: TableRows,
) -> Fixture {
    fixture(before, rows, 2, cells, options)
}

#[test]
fn headings_repeat_on_later_parts_with_one_more_inset_tp2_tp2b_tp2d() {
    let mut options = TableRows::default();
    options.headings.extend([0, 1, 40]);
    let fixture = fixture(30, 80, 2, &[], options);
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found[0].2.heading, None, "the first part has its own rows");
    let first_rows = found[0].2.rows.end;
    for (page, bounds, part) in &found[1..] {
        let heading = part.heading.clone().unwrap();
        if part.rows.start <= 40 {
            assert_eq!(heading, 0..2, "page {page}");
        } else {
            assert_eq!(heading, 40..41, "page {page}");
        }
        // The inset, the heading rows, one more inset, and the rows.
        let rows: f32 = part.heights.iter().map(|height| height + GUTTER).sum();
        assert_eq!(
            bounds.height,
            INSET + heading.len() as f32 * ROW + INSET + rows,
            "page {page}"
        );
    }
    // The heading's text is drawn again on page 2, at the part's top, and
    // the first continuing row right below it.
    let (_, second, _) = &found[1];
    let heading = cell_lines(&result.pages[1], fixture.table, 0);
    assert_eq!(heading[0].top, second.y + INSET);
    let next = cell_lines(&result.pages[1], fixture.table, first_rows * 2);
    assert_eq!(next[0].top, second.y + INSET + 2.0 * ROW);
    // A page holding a heading holds one row less than room for the rows
    // alone: (12,960 − 2 × 43) / 326 = 39 rows with the 2-row heading.
    let (_, _, part) = &found[1];
    assert_eq!(part.rows.len() + 2, 39);
}

#[test]
fn a_part_starting_with_a_heading_row_repeats_the_one_before_tp2f() {
    let mut options = TableRows::default();
    options.headings.extend([0, 17]);
    let fixture = fixture(30, 40, 2, &[], options);
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found[1].2.rows.start, 17);
    assert_eq!(found[1].2.heading, Some(0..1));
}

#[test]
fn a_page_break_on_a_row_starts_the_next_page_with_it_tp6() {
    let mut options = TableRows::default();
    options.page_breaks.insert(2);
    let fixture = fixture(30, 40, 2, &[], options);
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    let rows: Vec<_> = found
        .iter()
        .map(|(page, _, part)| (*page, part.rows.clone()))
        .collect();
    assert_eq!(rows, [(1, 0..2), (2, 2..40)]);
}

#[test]
fn a_table_continues_in_the_next_column_tp9() {
    let mut fixture = fixture(10, 40, 1, &[], TableRows::default());
    let layout_id = fixture.document.document_layout().id;
    let page_layout = fixture.document.layout_mut(layout_id).unwrap();
    page_layout.right.layout.columns = 2;
    page_layout.right.layout.gutter = 360;
    page_layout.right.layout.tabs = vec![Vec::new(); 2];
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found.len(), 2, "{found:?}");
    let ((page_a, first, _), (page_b, second, _)) = (&found[0], &found[1]);
    assert_eq!((*page_a, *page_b), (1, 1));
    assert_eq!(second.y, TOP);
    assert_eq!(second.x, first.x + (9_360.0 - 360.0) / 2.0 + 360.0);
}

#[test]
fn a_table_whose_first_row_does_not_fit_takes_its_paragraph_along_tp10_tp11() {
    // 53 lines leave 240 twips: not enough for the inset, a row, and its
    // gutter. The table and its anchor's line go to page 2.
    let fixture = fixture(53, 6, 2, &[], TableRows::default());
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].0, found[0].1.y), (2, TOP));
    assert_eq!(page_of_paragraph(&result, 53), 2);

    // TP11: the anchor ends a line of text, which goes along.
    let mut fixture = fixture_with(52, 6, &[], TableRows::default());
    let anchor = fixture.document.paragraph_mut(52);
    anchor.text = format!("A heading that holds the table{OBJECT}");
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].0, found[0].1.y), (2, TOP + LINE));
    assert_eq!(page_of_paragraph(&result, 52), 2);
}

#[test]
fn rows_keep_their_size_without_automatic_tp2e() {
    let options = TableRows {
        automatic: false,
        ..TableRows::default()
    };
    let three = "One\nTwo\nThree".to_owned();
    let fixture = fixture(2, 3, 2, &[((1, 0), three)], options);
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found[0].2.heights, [LINE, LINE, LINE]);
    // The cell's text is cut to its row.
    assert_eq!(cell_lines(&result.pages[0], fixture.table, 2).len(), 1);
}

#[test]
fn a_table_in_a_frame_shows_the_rows_its_frame_holds_tp8() {
    let mut document = Document::from_plain_text("Text");
    let body = document.body_text_style();
    document.styles_mut().get_mut(&body).unwrap().spacing = Spacing::default();
    let mut cells = Vec::new();
    for _ in 0..20 {
        let flow = document.create_flow(StyleId(1));
        document
            .flow_mut(flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "Cell".into();
        cells.push(TableCell {
            content: CellContent::Text(flow),
            span: CellSpan::SINGLE,
            frame: None,
            format: Default::default(),
            formula: None,
            legacy: LegacyData::default(),
        });
    }
    let frame = document.allocate_frame_id();
    document.insert_frame(Frame {
        id: frame,
        placement: Placement::Fixed {
            page: 1,
            rect: Rect::new(1_440, 4_320, 5_760, 5_760),
        },
        style: table_style(Wrap::Around),
        group: None,
        content: FrameContent::Table(Table {
            columns: vec![2_000, 2_000],
            rows: vec![LINE as i32; 10],
            cell_margins: CellMargins::default(),
            spacing: TableSpacing::default(),
            row_options: TableRows::default(),
            layout: Default::default(),
            read_rows: Default::default(),
            cells,
            legacy: LegacyData::default(),
        }),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().push(frame);
    let result = layout(&document);
    let page = &result.pages[0];
    let frame_box = page
        .frames
        .iter()
        .find(|frame_box| frame_box.frame == frame)
        .unwrap();
    let FrameBoxKind::Table { cells, .. } = &frame_box.kind else {
        panic!("a table");
    };
    // 1,440 tall: the inset and four rows (1,347) fit, the fifth row's text
    // starts at 1,347 and is cut, the rest are not shown.
    let shown: Vec<_> = (0..20)
        .filter(|cell| !cell_lines(page, frame, *cell).is_empty())
        .collect();
    assert_eq!(shown, (0..8).collect::<Vec<_>>());
    assert!(
        cells
            .iter()
            .all(|cell| cell.bottom() <= frame_box.interior.bottom() + 0.5)
    );
    assert_eq!(cells[19], Bounds::default());
}

#[test]
fn pictures_in_cells_go_with_their_rows_part_tp7() {
    let mut fixture = fixture(30, 40, 2, &[((20, 1), String::new())], TableRows::default());
    let picture = fixture.document.allocate_frame_id();
    fixture.document.insert_frame(Frame {
        id: picture,
        placement: Placement::InCell,
        style: FrameStyle::default(),
        group: None,
        content: FrameContent::Legacy(LegacyData::default()),
        legacy: LegacyData::default(),
    });
    fixture.document.frame_stack_mut().insert(0, picture);
    table_mut(&mut fixture.document, fixture.table).cells[41].frame = Some(picture);
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    let (page, second, part) = &found[1];
    assert!(part.rows.contains(&20));
    let page_box = &result.pages[*page as usize - 1];
    let picture_box = page_box
        .frames
        .iter()
        .find(|frame_box| frame_box.frame == picture)
        .unwrap();
    let row = (20 - part.rows.start) as f32;
    assert_eq!(picture_box.bounds.y, second.y + INSET + row * ROW);
    // Nowhere else.
    let placed = result
        .pages
        .iter()
        .flat_map(|page| &page.frames)
        .filter(|frame_box| frame_box.frame == picture)
        .count();
    assert_eq!(placed, 1);
}

#[test]
fn edits_in_a_continued_table_lay_out_as_a_fresh_layout() {
    let options = TableRows {
        span_pages: true,
        ..TableRows::default()
    };
    let tall = (1..=16)
        .map(|line| format!("Tall {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let fixture = fixture(30, 40, 2, &[((6, 0), tall)], options);
    let mut document = fixture.document.clone();
    let mut incremental = IncrementalLayout::new();
    let check = |incremental: &mut IncrementalLayout, document: &Document, what: &str| {
        let got = incremental.layout(document, &mut FixedShaper).clone();
        let want = layout(document);
        assert_eq!(got.pages.len(), want.pages.len(), "{what}");
        for (index, (got, want)) in got.pages.iter().zip(&want.pages).enumerate() {
            assert_eq!(got, want, "{what}: page {}", index + 1);
        }
    };
    check(&mut incremental, &document, "first");
    // A cell on page 2 grows, then shrinks.
    let flow = fixture.flows[60];
    for text in ["one\ntwo\nthree", "x"] {
        let content = document.flow(flow).unwrap();
        let last = content.paragraphs.len() - 1;
        let end = TextPosition {
            flow,
            paragraph: last,
            byte: content.paragraphs[last].text.len(),
        };
        let start = TextPosition {
            flow,
            paragraph: 0,
            byte: 0,
        };
        let command = Command::ReplaceText {
            start,
            end,
            text: text.into(),
        };
        document = apply(&document, &command).unwrap().document;
        check(&mut incremental, &document, text);
    }
    // Text before the table moves it.
    let main = document.main_flow_id();
    let at = TextPosition {
        flow: main,
        paragraph: 3,
        byte: 0,
    };
    for _ in 0..3 {
        document = apply(&document, &Command::split_paragraph(at))
            .unwrap()
            .document;
        check(&mut incremental, &document, "a line above");
    }
    // A heading and a page break.
    let table = table_mut(&mut document, fixture.table);
    table.row_options.headings.insert(0);
    table.row_options.page_breaks.insert(25);
    check(&mut incremental, &document, "heading and break");
}

#[test]
fn page_tables_on_a_line_of_their_own_continue_too() {
    // TP1 with the table as Ami Pro stores page tables: Flow with text.
    let mut fixture = fixture_placed(30, 40, 2, &[], TableRows::default(), true);
    table_mut(&mut fixture.document, fixture.table).spacing.top = 250;
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    let rows: Vec<_> = found
        .iter()
        .map(|(page, _, part)| (*page, part.rows.clone()))
        .collect();
    assert_eq!(rows, [(1, 0..16), (2, 16..40)]);
    assert_eq!(found[0].1.y, TOP + 30.0 * LINE);
    assert_eq!(found[1].1.y, TOP);
    // The table's line stays on page 1, the text after follows page 2's part.
    assert_eq!(page_of_paragraph(&result, 30), 1);
    let after = result.pages[1]
        .lines
        .iter()
        .find(|line| line.container == Container::Main && line.paragraph_index == 31)
        .unwrap();
    assert_eq!(after.top, found[1].1.bottom());

    // TP11: a heading line and the table in one paragraph go to page 2
    // together when the first row does not fit below the heading.
    let mut fixture = fixture_placed(52, 6, 2, &[], TableRows::default(), true);
    let anchor = fixture.document.paragraph_mut(52);
    anchor.text = format!("A heading that holds the table {OBJECT}");
    let result = layout(&fixture.document);
    let found = parts(&result, fixture.table);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].0, found[0].1.y), (2, TOP + LINE));
    assert_eq!(page_of_paragraph(&result, 52), 2);

    // Incremental layout of edits in the table's cells and above it.
    let fixture = fixture_placed(30, 40, 2, &[], TableRows::default(), true);
    let mut document = fixture.document.clone();
    let mut incremental = IncrementalLayout::new();
    let mut check = |document: &Document, what: &str| {
        let got = incremental.layout(document, &mut FixedShaper).clone();
        let want = layout(document);
        assert_eq!(got.pages, want.pages, "{what}");
    };
    check(&document, "first");
    let flow = fixture.flows[50];
    let start = TextPosition {
        flow,
        paragraph: 0,
        byte: 0,
    };
    document = apply(&document, &Command::insert(start, "grown\nrow\n"))
        .unwrap()
        .document;
    check(&document, "a cell on page 2");
    let main = document.main_flow_id();
    let at = TextPosition {
        flow: main,
        paragraph: 2,
        byte: 0,
    };
    document = apply(&document, &Command::split_paragraph(at))
        .unwrap()
        .document;
    check(&document, "a line above");
}

/// The row cache (VM RC1–RC3, user decision 2026-10-19): Ami Pro stores its
/// own measure of each Automatic row, so a row as read keeps that height
/// while its text measures here at most half its gutter taller, the text
/// running into the gutter; an edit or a stale height measures the row.
#[test]
fn a_row_as_read_keeps_ami_pros_stored_height() {
    // Four 240-twip lines measure 960 here; Ami Pro's printer stored 940.
    let cached = |stored: i32, read: bool| {
        let mut fixture = fixture(
            0,
            3,
            2,
            &[((0, 0), "a\nb\nc\nd".into())],
            TableRows::default(),
        );
        let document = fixture.document.clone();
        let table = table_mut(&mut fixture.document, fixture.table);
        table.rows[0] = stored;
        if read {
            let print = {
                let FrameContent::Table(content) = &document.frame(fixture.table).unwrap().content
                else {
                    panic!("a table");
                };
                let mut content = content.clone();
                content.rows[0] = stored;
                document.table_row_fingerprint(&content, 0)
            };
            table_mut(&mut fixture.document, fixture.table)
                .read_rows
                .insert(0, print);
        }
        fixture
    };
    let row_of = |fixture: &Fixture, document: &Document| {
        let result = layout(document);
        let part = parts(&result, fixture.table).remove(0).2;
        let lines = cell_lines(&result.pages[0], fixture.table, 0);
        let next = cell_lines(&result.pages[0], fixture.table, 2)[0].top;
        (part.heights[0], lines.len(), next - lines[0].top)
    };

    // As read: Ami Pro's height, every line shown, the next row after it.
    let fixture = cached(940, true);
    assert_eq!(
        row_of(&fixture, &fixture.document),
        (940.0, 4, 940.0 + GUTTER)
    );
    // Not read from a file: measured.
    let fresh = cached(940, false);
    assert_eq!(row_of(&fresh, &fresh.document), (960.0, 4, 960.0 + GUTTER));
    // A stale height (Ami Pro would measure the row again, RC1): measured.
    let stale = cached(480, true);
    assert_eq!(row_of(&stale, &stale.document), (960.0, 4, 960.0 + GUTTER));

    // An edit in the row measures it, in a fresh layout and incrementally.
    let flow = fixture.flows[0];
    let at = TextPosition {
        flow,
        paragraph: 0,
        byte: 1,
    };
    let edited = apply(
        &fixture.document,
        &Command::ReplaceText {
            start: at,
            end: at,
            text: "z".into(),
        },
    )
    .unwrap()
    .document;
    assert_eq!(row_of(&fixture, &edited), (960.0, 4, 960.0 + GUTTER));
    let mut incremental = IncrementalLayout::new();
    incremental.layout(&fixture.document, &mut FixedShaper);
    let got = incremental.layout(&edited, &mut FixedShaper).clone();
    assert_eq!(got.pages, layout(&edited).pages);
    // An edit in another row keeps it.
    let other = TextPosition {
        flow: fixture.flows[2],
        paragraph: 0,
        byte: 0,
    };
    let elsewhere = apply(
        &fixture.document,
        &Command::ReplaceText {
            start: other,
            end: other,
            text: "z".into(),
        },
    )
    .unwrap()
    .document;
    assert_eq!(row_of(&fixture, &elsewhere).0, 940.0);

    // A row Ami Pro measured taller than its text shrinks to it once edited
    // (VM RC2), and a row whose cells are emptied keeps a line (RC3).
    let tall = cached(2_000, true);
    assert_eq!(row_of(&tall, &tall.document).0, 2_000.0, "as read");
    let edited = apply(
        &tall.document,
        &Command::ReplaceText {
            start: TextPosition {
                flow: tall.flows[0],
                paragraph: 0,
                byte: 1,
            },
            end: TextPosition {
                flow: tall.flows[0],
                paragraph: 0,
                byte: 1,
            },
            text: "z".into(),
        },
    )
    .unwrap()
    .document;
    assert_eq!(row_of(&tall, &edited).0, 960.0, "fits its text (RC2)");
    let emptied = apply(
        &tall.document,
        &Command::ReplaceText {
            start: TextPosition {
                flow: tall.flows[0],
                paragraph: 0,
                byte: 0,
            },
            end: TextPosition {
                flow: tall.flows[0],
                paragraph: 3,
                byte: 1,
            },
            text: String::new(),
        },
    )
    .unwrap()
    .document;
    let result = layout(&emptied);
    assert_eq!(
        parts(&result, tall.table).remove(0).2.heights[0],
        240.0,
        "an empty row keeps a line (RC3)"
    );
}

/// Center table on page (`[tbl]` value 7 `0x4`, VM TE13): a page table in
/// the middle of its column, Flow with text or With para above.
#[test]
fn a_centred_table_stands_in_the_middle_of_its_column_te13() {
    for inline in [true, false] {
        let mut fixture = fixture_placed(1, 2, 2, &[], TableRows::default(), inline);
        let left = parts(&layout(&fixture.document), fixture.table)[0].1.x;
        table_mut(&mut fixture.document, fixture.table)
            .layout
            .center = true;
        let centred = parts(&layout(&fixture.document), fixture.table)[0].1;
        // A 6.5 in column from 1,440; the table 8,043 wide.
        assert_eq!(left, 1_440.0, "inline {inline}");
        assert_eq!(
            centred.x,
            1_440.0 + ((9_360 - 8_043) / 2) as f32,
            "inline {inline}"
        );
    }
}

#[test]
fn a_numeric_cell_is_one_word_shown_through_its_styles_number_format_td5() {
    use amipro_document::{NumberFormat, NumberKind};
    let mut fixture = fixture(
        0,
        1,
        3,
        &[
            ((0, 0), "1234.5".into()),
            ((0, 1), "-7.25".into()),
            ((0, 2), "$1,200.00".into()),
        ],
        TableRows::default(),
    );
    let style = fixture.document.flow(fixture.flows[0]).unwrap().paragraphs[0].style;
    fixture
        .document
        .styles_mut()
        .get_mut(&style)
        .unwrap()
        .kept
        .number_format = NumberFormat {
        flags: 0x198,
        kind: NumberKind::Currency,
        ..NumberFormat::default()
    };
    let result = layout(&fixture.document);
    let page = &result.pages[0];
    let word = |cell: usize| {
        let lines = cell_lines(page, fixture.table, cell);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].words.len(), 1, "one word over the stored text");
        lines[0].words[0].clone()
    };
    let first = word(0);
    assert_eq!(first.text.as_deref(), Some("$1,234.50"));
    assert_eq!(first.bytes, 0..6, "the stored bytes");
    assert!(!first.red);
    let negative = word(1);
    assert_eq!(negative.text.as_deref(), Some("-$7.25"));
    assert!(negative.red);
    assert_eq!(negative.bytes, 0..5);
    let text = word(2);
    assert_eq!(text.text, None, "text cells are drawn as typed");
    // The stored text is untouched.
    let FrameContent::Table(table) = &fixture.document.frame(fixture.table).unwrap().content else {
        panic!("a table");
    };
    let CellContent::Text(flow) = table.cells[0].content;
    assert_eq!(
        fixture.document.flow(flow).unwrap().paragraphs[0].text,
        "1234.5"
    );
    assert!(first.width > 0.0);
}
