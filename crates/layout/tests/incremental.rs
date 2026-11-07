//! Incremental layout never goes stale (D5): after every kind of change, its
//! result equals a fresh layout of the same document, and a local edit
//! reuses pages.

use std::sync::Arc;

use amipro_document::{
    BreakKind, Breaks, CellContent, CellMargins, CellSpan, CharacterFormat, Document, FlowId,
    Frame, FrameContent, FrameId, FrameStyle, HeaderFooterKind, InlineObject, LegacyData,
    LineSpacing, Margins, PageSides, Placement, Rect, Size, Spacing, StyleId, Table, TableCell,
    Wrap,
};
use amipro_editing::{Command, History, TextPosition, apply};
use amipro_layout::{
    Container, DocumentLayout, FixedShaper, IncrementalLayout, LayoutKind, ShapeCache,
    layout_document,
};

const OBJECT: char = '\u{FFFC}';

fn fresh(document: &Document) -> DocumentLayout {
    layout_document(document, &mut FixedShaper, &mut ShapeCache::default())
}

/// Lays out incrementally and checks the result against a fresh layout.
fn check(incremental: &mut IncrementalLayout, document: &Document, what: &str) -> LayoutKind {
    document.validate().unwrap();
    let result = incremental.layout(document, &mut FixedShaper).clone();
    let expected = fresh(document);
    assert_eq!(
        result.pages.len(),
        expected.pages.len(),
        "{what}: page count"
    );
    for (index, (got, want)) in result.pages.iter().zip(&expected.pages).enumerate() {
        assert_eq!(got, want, "{what}: page {}", index + 1);
    }
    incremental.stats().kind
}

fn sentence(index: usize) -> String {
    let count = 1 + (index * 7) % 23;
    (0..count)
        .map(|word| ["ami", "pro", "frames", "pages", "layout", "x"][(index + word) % 6])
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_object(document: &mut Document, index: usize, object: InlineObject) {
    let paragraph = document.paragraph_mut(index);
    paragraph.text.push(OBJECT);
    paragraph.objects.push(object);
}

fn text_frame(document: &mut Document, placement: Placement, wrap: Wrap) -> FrameId {
    let frame = document.create_text_frame(
        placement,
        FrameStyle {
            wrap,
            ..FrameStyle::default()
        },
    );
    document.frame_stack_mut().push(frame);
    frame
}

fn set_flow_text(document: &mut Document, frame: FrameId, text: &str) {
    let FrameContent::Text(flow) = document.frame(frame).unwrap().content else {
        panic!("a text frame");
    };
    document
        .flow_mut(flow)
        .unwrap()
        .paragraph_mut(0)
        .unwrap()
        .text = text.into();
}

/// A document of many pages with every kind of placement the engine knows:
/// a header, a wrapping Where placed frame, With para above and Flow with
/// text frames, a floating header, page and column breaks, and a switch to a
/// balanced two-column layout and back.
fn rich_document() -> Document {
    let text: Vec<String> = (0..240).map(sentence).collect();
    let mut document = Document::from_plain_text(&text.join("\n"));
    let header = document.document_layout().header.right.unwrap();
    set_flow_text(&mut document, header, "Running head");

    let fixed = text_frame(
        &mut document,
        Placement::Fixed {
            page: 3,
            rect: Rect::new(4_000, 3_000, 8_000, 6_000),
        },
        Wrap::Around,
    );
    set_flow_text(&mut document, fixed, "Fixed frame text");

    let below = text_frame(
        &mut document,
        Placement::BelowParagraph {
            x_offset: 720,
            size: Size {
                width: 4_000,
                height: 2_000,
            },
        },
        Wrap::NoWrapBeside,
    );
    push_object(&mut document, 40, InlineObject::FrameAnchor(below));

    let inline = text_frame(
        &mut document,
        Placement::Inline {
            y_offset: 0,
            size: Size {
                width: 1_440,
                height: 720,
            },
        },
        Wrap::Around,
    );
    push_object(&mut document, 50, InlineObject::FrameAnchor(inline));

    let floating = document.create_flow(StyleId(1));
    document
        .flow_mut(floating)
        .unwrap()
        .paragraph_mut(0)
        .unwrap()
        .text = "Floating head".into();
    push_object(
        &mut document,
        60,
        InlineObject::FloatingHeaderFooter {
            kind: HeaderFooterKind::Header,
            pages: PageSides::All,
            flow: floating,
        },
    );

    let mut columns = document.new_standard_layout("Columns");
    columns.right.layout.columns = 2;
    columns.right.layout.gutter = 240;
    columns.right.layout.tabs = vec![Vec::new(), Vec::new()];
    columns.right.layout.balance = true;
    let columns_id = columns.id;
    document.layouts_mut().push(Arc::new(columns));
    let standard = document.document_layout().id;
    push_object(
        &mut document,
        80,
        InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: Some(columns_id),
            restart: false,
        },
    );
    push_object(
        &mut document,
        95,
        InlineObject::Break {
            kind: BreakKind::Column,
            centre: false,
            layout: None,
            restart: false,
        },
    );
    push_object(
        &mut document,
        110,
        InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: Some(standard),
            restart: false,
        },
    );
    document.validate().unwrap();
    document
}

/// The rich document with a tall With para above frame after every seventh
/// paragraph, so that some do not fit and take their paragraph to the next
/// column or page (V12), one that text runs through before every eleventh
/// (MERCURY.SAM), and a centring break.
fn crowded_document() -> Document {
    let mut document = rich_document();
    for index in (3..150).step_by(7) {
        let frame = text_frame(
            &mut document,
            Placement::BelowParagraph {
                x_offset: 0,
                size: Size {
                    width: 3_000,
                    height: 2_500 + (index as i32 % 5) * 900,
                },
            },
            Wrap::NoWrapBeside,
        );
        push_object(&mut document, index, InlineObject::FrameAnchor(frame));
    }
    for index in (5..150).step_by(11) {
        let frame = text_frame(
            &mut document,
            Placement::BelowParagraph {
                x_offset: 0,
                size: Size {
                    width: 2_000,
                    height: 1_800,
                },
            },
            Wrap::NoWrapAround,
        );
        let paragraph = document.paragraph_mut(index);
        paragraph.text.insert(0, OBJECT);
        paragraph
            .objects
            .insert(0, InlineObject::FrameAnchor(frame));
    }
    push_object(
        &mut document,
        130,
        InlineObject::Break {
            kind: BreakKind::Page,
            centre: true,
            layout: None,
            restart: false,
        },
    );
    document.validate().unwrap();
    document
}

fn at(document: &Document, paragraph: usize, byte: usize) -> TextPosition {
    TextPosition {
        flow: document.main_flow_id(),
        paragraph,
        byte,
    }
}

fn run(document: &Document, command: Command) -> Document {
    apply(document, &command).unwrap().document
}

#[test]
fn every_kind_of_change_matches_a_fresh_layout() {
    let mut document = rich_document();
    let mut incremental = IncrementalLayout::new();
    assert_eq!(
        check(&mut incremental, &document, "first"),
        LayoutKind::Full
    );
    let pages = incremental.current().pages.len();
    assert!(pages >= 7, "the document spans {pages} pages");
    assert_eq!(
        check(&mut incremental, &document, "same document"),
        LayoutKind::Unchanged
    );

    // Typing in the middle: only the pages around the edit are set again.
    document = run(
        &document,
        Command::insert(at(&document, 30, 3), " typed words"),
    );
    assert_eq!(
        check(&mut incremental, &document, "typing"),
        LayoutKind::Incremental
    );
    let stats = incremental.stats();
    assert!(
        stats.pages_reused > 0 && stats.pages_laid_out <= 3,
        "typing reuses pages: {stats:?}"
    );

    // A new paragraph and a deleted one shift the indexes after them.
    document = run(&document, Command::split_paragraph(at(&document, 20, 2)));
    assert_eq!(
        check(&mut incremental, &document, "split"),
        LayoutKind::Incremental
    );
    document = run(
        &document,
        Command::join_with_previous(&document, document.main_flow_id(), 21).unwrap(),
    );
    assert_eq!(
        check(&mut incremental, &document, "join"),
        LayoutKind::Incremental
    );
    document = run(
        &document,
        Command::delete(at(&document, 12, 0), at(&document, 15, 0)),
    );
    check(&mut incremental, &document, "delete paragraphs");

    // Many lines at once: pages are added, and later removed.
    let long = vec!["more text"; 300].join(" ");
    document = run(&document, Command::insert(at(&document, 45, 0), long));
    check(&mut incremental, &document, "a long insertion");
    let len = document.paragraphs()[45].text.len();
    document = run(
        &document,
        Command::delete(at(&document, 45, 0), at(&document, 45, len - 10)),
    );
    check(&mut incremental, &document, "removing it");

    // Direct formatting and a paragraph style.
    document = run(
        &document,
        Command::FormatCharacters {
            start: at(&document, 70, 0),
            end: at(&document, 70, 3),
            format: CharacterFormat {
                font_size_points: Some(36.0),
                ..CharacterFormat::default()
            },
        },
    );
    assert_eq!(
        check(&mut incremental, &document, "format"),
        LayoutKind::Incremental
    );
    let other_style = *document
        .styles()
        .keys()
        .find(|id| **id != StyleId(1))
        .unwrap();
    document = run(
        &document,
        Command::SetParagraphStyle {
            flow: document.main_flow_id(),
            first: 90,
            last: 92,
            style: other_style,
        },
    );
    check(&mut incremental, &document, "paragraph style");

    // Edits near the start, in the columns, and at the very end.
    document = run(&document, Command::insert(at(&document, 0, 0), "Start "));
    check(&mut incremental, &document, "first paragraph");
    document = run(&document, Command::insert(at(&document, 97, 0), "Column "));
    check(&mut incremental, &document, "balanced columns");
    let last = document.paragraphs().len() - 1;
    document = run(&document, Command::insert(at(&document, last, 0), "End "));
    check(&mut incremental, &document, "last paragraph");
    document = run(&document, Command::split_paragraph(at(&document, last, 0)));
    check(&mut incremental, &document, "new last paragraph");

    // A style definition: everything in that style changes.
    document
        .styles_mut()
        .get_mut(&StyleId(1))
        .unwrap()
        .font_size_points = 14.0;
    assert_eq!(
        check(&mut incremental, &document, "style definition"),
        LayoutKind::Full
    );

    // Resizing: margins and page size.
    document.document_layout_mut().right.layout.margins = Margins::uniform(2_160);
    assert_eq!(
        check(&mut incremental, &document, "margins"),
        LayoutKind::Full
    );
    document.document_layout_mut().right.width = 11_906;
    check(&mut incremental, &document, "page size");

    // Header text only: headers are set again over the same main text.
    let header = document.document_layout().header.right.unwrap();
    set_flow_text(&mut document, header, "A longer running head");
    assert_eq!(
        check(&mut incremental, &document, "header"),
        LayoutKind::Repaint
    );
}

/// A table anchored in the main text is as tall as its cells' text, so a
/// cell edit lays the main text out again from the table's anchor; other
/// frame text only repaints.
#[test]
fn table_cell_edits_move_the_text_after_the_table() {
    let mut document = rich_document();
    let cells: Vec<TableCell> = (0..2)
        .map(|_| TableCell {
            content: CellContent::Text(document.create_flow(StyleId(1))),
            span: CellSpan::SINGLE,
            frame: None,
            format: Default::default(),
            formula: None,
            legacy: LegacyData::default(),
        })
        .collect();
    let flows: Vec<FlowId> = cells
        .iter()
        .map(|cell| {
            let CellContent::Text(flow) = cell.content;
            flow
        })
        .collect();
    let table = document.allocate_frame_id();
    document.insert_frame(Frame {
        id: table,
        placement: Placement::BelowParagraph {
            x_offset: 0,
            size: Size {
                width: 4_000,
                height: 700,
            },
        },
        style: FrameStyle {
            wrap: Wrap::NoWrapAround,
            ..FrameStyle::default()
        },
        group: None,
        content: FrameContent::Table(Table {
            columns: vec![1_800, 1_800],
            rows: vec![240],
            cell_margins: CellMargins::default(),
            spacing: Default::default(),
            row_options: Default::default(),
            layout: Default::default(),
            read_rows: Default::default(),
            cells,
            legacy: LegacyData::default(),
        }),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().push(table);
    let paragraph = document.paragraph_mut(60);
    paragraph.text.insert(0, OBJECT);
    paragraph
        .objects
        .insert(0, InlineObject::FrameAnchor(table));
    // A picture in the second cell (§24), first in the stack as Ami Pro
    // lists frames in cells.
    let picture = document.allocate_frame_id();
    document.insert_frame(Frame {
        id: picture,
        placement: Placement::InCell,
        style: FrameStyle::default(),
        group: None,
        content: FrameContent::Legacy(LegacyData::default()),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().insert(0, picture);
    let FrameContent::Table(content) = &mut document.frame_mut(table).unwrap().content else {
        unreachable!()
    };
    content.cells[1].frame = Some(picture);
    let mut incremental = IncrementalLayout::new();
    check(&mut incremental, &document, "first");
    // The picture's box follows its table's, over the cell's text area.
    let boxes = |layout: &DocumentLayout| {
        layout
            .pages
            .iter()
            .flat_map(|page| &page.frames)
            .filter(|frame| frame.frame == table || frame.frame == picture)
            .map(|frame| frame.bounds)
            .collect::<Vec<_>>()
    };
    let placed = boxes(incremental.layout(&document, &mut FixedShaper));
    assert_eq!(placed.len(), 2, "the table, then its picture");
    // Inside the table frame's margins (the default frame's 144), the
    // inset, and the first column with its gutter.
    assert_eq!(placed[1].x, placed[0].x + 144.0 + 43.0 + 1_800.0 + 86.0);
    assert_eq!(
        (placed[1].y, placed[1].height),
        (placed[0].y + 144.0 + 43.0, 240.0)
    );

    let set_cell = |document: &mut Document, flow: FlowId, text: &str| {
        document
            .flow_mut(flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = text.into();
    };
    set_cell(&mut document, flows[0], &sentence(22).repeat(4));
    assert_eq!(
        check(&mut incremental, &document, "a growing cell"),
        LayoutKind::Incremental
    );
    // The picture grows with its row (TF3).
    let laid_out = incremental.layout(&document, &mut FixedShaper).clone();
    let grown = boxes(&laid_out);
    let row: f32 = laid_out
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| {
            line.container
                == Container::Cell {
                    frame: table,
                    cell: 0,
                }
        })
        .map(|line| line.height)
        .sum();
    assert!(grown[1].height > 240.0, "{grown:?}");
    assert_eq!(grown[1].height, row, "as tall as the row's text");
    set_cell(&mut document, flows[1], "x");
    assert_eq!(
        check(&mut incremental, &document, "a cell of the same height"),
        LayoutKind::Incremental
    );
    assert!(
        incremental.stats().pages_laid_out <= 2,
        "{:?}",
        incremental.stats()
    );
    set_cell(&mut document, flows[0], "");
    check(&mut incremental, &document, "a shrinking cell");
    // Delete in the picture's cell.
    document.remove_frame(picture).unwrap();
    check(&mut incremental, &document, "the picture deleted");
    assert_eq!(
        boxes(incremental.layout(&document, &mut FixedShaper)).len(),
        1
    );
    let header = document.document_layout().header.right.unwrap();
    set_flow_text(&mut document, header, "Running head again");
    assert_eq!(
        check(&mut incremental, &document, "header"),
        LayoutKind::Repaint
    );
}

#[test]
fn undo_and_redo_match_a_fresh_layout() {
    let mut document = rich_document();
    let mut history: History<()> = History::new();
    let mut incremental = IncrementalLayout::new();
    check(&mut incremental, &document, "first");
    let flow = document.main_flow_id();
    let commands = [
        Command::insert(at(&document, 10, 0), "inserted "),
        Command::split_paragraph(at(&document, 33, 4)),
        // Paragraphs without objects: deleting a floating header's mark would
        // delete its flow, a structural change.
        Command::delete(at(&document, 65, 0), at(&document, 67, 0)),
        Command::SetParagraphStyle {
            flow,
            first: 5,
            last: 5,
            style: StyleId(2),
        },
    ];
    for command in &commands {
        history.execute(&mut document, (), command).unwrap();
        check(&mut incremental, &document, &format!("{command:?}"));
    }
    while history.undo(&mut document, ()).is_some() {
        let kind = check(&mut incremental, &document, "undo");
        assert_ne!(kind, LayoutKind::Full, "undo shares the earlier roots");
    }
    while history.redo(&mut document, ()).is_some() {
        check(&mut incremental, &document, "redo");
    }
}

/// A small deterministic generator, so failures reproduce.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % below.max(1)
    }
}

fn boundary(text: &str, byte: usize) -> usize {
    (0..=byte.min(text.len()))
        .rev()
        .find(|index| text.is_char_boundary(*index))
        .unwrap_or(0)
}

#[test]
fn random_edits_never_leave_the_layout_stale() {
    for seed in 1..=16 {
        let mut random = Lcg(seed);
        let mut document = if seed % 2 == 0 {
            crowded_document()
        } else {
            rich_document()
        };
        // Seeds 13–16: break rules (B1–B8): unbreakable paragraphs kept with
        // the next one, and page breaks before and after, which roll back
        // and move paragraphs.
        if seed > 12 {
            let body = document.paragraphs()[0].style;
            let mut rule = document.styles()[&body].clone();
            rule.id = StyleId(500);
            rule.name = "Rule".into();
            rule.breaks = Breaks(if seed < 15 {
                Breaks::KEEP_WITH_NEXT
            } else {
                Breaks::WITHIN | Breaks::PAGE_BEFORE
            });
            let mut after = rule.clone();
            after.id = StyleId(501);
            after.name = "After".into();
            after.breaks = Breaks(Breaks::WITHIN | Breaks::PAGE_AFTER);
            document.styles_mut().insert(rule.id, rule);
            document.styles_mut().insert(after.id, after);
            for index in (2..document.paragraphs().len()).step_by(if seed < 15 { 9 } else { 23 }) {
                document.paragraph_mut(index).style = StyleId(500);
            }
            document.paragraph_mut(40).style = StyleId(501);
        }
        // Seeds 9–12: space above and below, added at page tops or not, and
        // fixed line heights (S1–S5), which page starts must replay alike.
        if (9..=12).contains(&seed) {
            let style = document.paragraphs()[0].style;
            document.styles_mut().get_mut(&style).unwrap().spacing = Spacing {
                line: if seed > 10 {
                    LineSpacing::Double(560)
                } else {
                    LineSpacing::Single
                },
                above: 200,
                below: 100,
                add_at_break: seed % 2 == 0,
                tightness: 100,
            };
        }
        let mut history: History<()> = History::new();
        let mut incremental = IncrementalLayout::new();
        check(&mut incremental, &document, "first");
        let (mut incremental_steps, mut reused) = (0, 0);
        for step in 0..120 {
            let paragraphs = document.paragraphs().len();
            let paragraph = random.next(paragraphs);
            let text = &document.paragraphs()[paragraph].text;
            let byte = boundary(text, random.next(text.len() + 1));
            let command = match random.next(9) {
                0..=2 => Command::insert(
                    at(&document, paragraph, byte),
                    ["a", "word ", "several more words here ", "\n"][random.next(4)],
                ),
                3 => Command::split_paragraph(at(&document, paragraph, byte)),
                4 if paragraph > 0 => {
                    Command::join_with_previous(&document, document.main_flow_id(), paragraph)
                        .unwrap()
                }
                5 => {
                    let end = (paragraph + random.next(3)).min(paragraphs - 1);
                    let end_text = &document.paragraphs()[end].text;
                    let end_byte = boundary(end_text, random.next(end_text.len() + 1));
                    if end == paragraph && end_byte < byte {
                        continue;
                    }
                    Command::delete(at(&document, paragraph, byte), at(&document, end, end_byte))
                }
                6 => Command::FormatCharacters {
                    start: at(&document, paragraph, 0),
                    end: at(&document, paragraph, byte),
                    format: CharacterFormat {
                        font_size_points: Some([8.0, 12.0, 30.0][random.next(3)]),
                        ..CharacterFormat::default()
                    },
                },
                7 => {
                    history.undo(&mut document, ());
                    check(
                        &mut incremental,
                        &document,
                        &format!("seed {seed} step {step} undo"),
                    );
                    continue;
                }
                _ => {
                    history.redo(&mut document, ());
                    check(
                        &mut incremental,
                        &document,
                        &format!("seed {seed} step {step} redo"),
                    );
                    continue;
                }
            };
            if history.execute(&mut document, (), &command).is_err() {
                continue;
            }
            check(
                &mut incremental,
                &document,
                &format!("seed {seed} step {step} {command:?}"),
            );
            if incremental.stats().kind == LayoutKind::Incremental {
                incremental_steps += 1;
                reused += incremental.stats().pages_reused;
            }
        }
        assert!(
            incremental_steps > 50 && reused > incremental_steps,
            "seed {seed}: {incremental_steps} incremental layouts reused {reused} pages"
        );
    }
}

#[test]
fn a_style_change_renumbers_the_paragraphs_after_it() {
    // Numbers count runs of a style (VM fixture BN2): a change to one
    // paragraph renumbers every later one in its run, which a reused tail
    // of pages would leave stale.
    let text: Vec<String> = (0..160).map(sentence).collect();
    let mut document = Document::from_plain_text(&text.join("\n"));
    let body = document.paragraphs()[0].style;
    let mut numbered = document.styles()[&body].clone();
    numbered.id = StyleId(90);
    numbered.name = "Num".into();
    numbered.bullets = amipro_document::Bullets {
        text: "\u{1A}.".into(),
        ..amipro_document::Bullets::default()
    };
    document.styles_mut().insert(numbered.id, numbered);
    document = run(
        &document,
        Command::SetParagraphStyle {
            flow: document.main_flow_id(),
            first: 0,
            last: 159,
            style: StyleId(90),
        },
    );
    let mut incremental = IncrementalLayout::new();
    check(&mut incremental, &document, "numbered");
    assert!(incremental.current().pages.len() > 3);
    for (style, what) in [(body, "run broken"), (StyleId(90), "run joined")] {
        document = run(
            &document,
            Command::SetParagraphStyle {
                flow: document.main_flow_id(),
                first: 5,
                last: 5,
                style,
            },
        );
        check(&mut incremental, &document, what);
    }
}

#[test]
fn page_numbers_after_a_toc_break_survive_later_edits() {
    // A TOC break (Ami Pro's flag 0x02) ending the first paragraph, then
    // pages of text; edits on later pages lay those pages out again, and the
    // pages keep counting from 1 after the break, as a fresh layout does.
    let text: Vec<String> = (0..120).map(sentence).collect();
    let mut document = Document::from_plain_text(&text.join("\n"));
    push_object(
        &mut document,
        0,
        InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: None,
            restart: true,
        },
    );
    let mut incremental = IncrementalLayout::new();
    check(&mut incremental, &document, "first");
    let pages = incremental.current().pages.clone();
    assert!(pages.len() >= 3);
    assert_eq!((pages[1].number, pages[1].shown), (2, 1));
    for (index, paragraph) in [(40, 0), (90, 1), (5, 2)] {
        document = run(
            &document,
            Command::insert(at(&document, index, 0), format!("edit {paragraph} ")),
        );
        check(&mut incremental, &document, &format!("edit {paragraph}"));
        assert_eq!(incremental.current().pages[1].shown, 1);
    }
}
