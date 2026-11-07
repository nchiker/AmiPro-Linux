//! The layout engine's rules, each from a VM fixture
//! (`docs/research/SAM_FORMAT_NOTES.md`), with the deterministic
//! [`FixedShaper`]: 12 pt Body Text sets 120-twip characters on 240-twip
//! lines (ascent 192, descent 48) with 120 twips after each paragraph. The
//! default page is Letter (12,240 × 15,840) with 1-inch margins, so the text
//! area is 9,360 × 12,960 from (1,440, 1,440).

use amipro_document::{
    Alignment, BreakKind, Breaks, CellContent, CellMargins, CellSpan, ColumnLine, Document,
    FaceKey, FlowId, Frame, FrameContent, FrameId, FrameStyle, HeaderFooterKind, Indents,
    InlineObject, LayoutBox, LegacyData, LineHeights, LineSpacing, Margins, PageNumberField,
    PageSide, PageSides, Placement, Rect, RepeatPages, Shadow, Size, Spacing, StyleId, Table,
    TableCell, TableSpacing, Wrap,
};
use amipro_layout::{
    Bounds, Container, DocumentLayout, FixedShaper, FrameBoxKind, Layer, LineBox, PageBox,
    ShapeCache, layout_document,
};

const LINE: f32 = 240.0;
const AFTER: f32 = 120.0;
const OBJECT: char = '\u{FFFC}';

/// A document of Body Text paragraphs with 120 twips after each, as new
/// documents had until 2026-10-18 (they now have none, as `_DEFAULT.STY`):
/// several rules place frames and tables against the space after.
fn plain_text(text: &str) -> Document {
    let mut document = Document::from_plain_text(text);
    let body = document.body_text_style();
    document.styles_mut().get_mut(&body).unwrap().spacing = Spacing::below(120);
    document
}

fn layout(document: &Document) -> DocumentLayout {
    document.validate().unwrap();
    layout_document(document, &mut FixedShaper, &mut ShapeCache::default())
}

fn main_lines(page: &PageBox) -> Vec<&LineBox> {
    page.lines
        .iter()
        .filter(|line| line.container == Container::Main)
        .collect()
}

fn text_of<'a>(document: &'a Document, line: &LineBox) -> &'a str {
    let flow = document.flow(line.flow).unwrap();
    &flow.paragraphs[line.paragraph_index].text[line.bytes.clone()]
}

/// "word word word …" of `count` words of five characters.
fn words(count: usize) -> String {
    vec!["wordy"; count].join(" ")
}

fn page_break(layout: Option<amipro_document::LayoutId>, centre: bool) -> InlineObject {
    InlineObject::Break {
        kind: BreakKind::Page,
        centre,
        layout,
        restart: false,
    }
}

/// Appends `object` to paragraph `index` of the main flow.
fn append_object(document: &mut Document, index: usize, object: InlineObject) {
    let paragraph = document.paragraph_mut(index);
    paragraph.text.push(OBJECT);
    paragraph.objects.push(object);
}

fn fixed_frame(document: &mut Document, page: u32, rect: Rect, wrap: Wrap) -> FrameId {
    let frame = document.create_text_frame(
        Placement::Fixed { page, rect },
        FrameStyle {
            wrap,
            ..FrameStyle::default()
        },
    );
    document.frame_stack_mut().push(frame);
    frame
}

fn frame_text(document: &mut Document, frame: FrameId, text: &str) {
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

#[test]
fn page_breaks_start_the_next_page_g20_v01() {
    let mut document = plain_text("First\nSecond");
    append_object(&mut document, 0, page_break(None, false));
    let result = layout(&document);
    assert_eq!(result.pages.len(), 2);
    assert_eq!(
        text_of(&document, main_lines(&result.pages[0])[0]),
        "First\u{FFFC}"
    );
    let second = main_lines(&result.pages[1]);
    assert_eq!(text_of(&document, second[0]), "Second");
    assert_eq!(
        second[0].top, 1_440.0,
        "the text after the break starts the page"
    );

    // V01: a break before all the text leaves page 1 empty of text.
    let mut document = plain_text("Text");
    let paragraph = document.paragraph_mut(0);
    paragraph.text.insert(0, OBJECT);
    paragraph.objects.push(page_break(None, false));
    let result = layout(&document);
    assert_eq!(result.pages.len(), 2);
    assert_eq!(text_of(&document, main_lines(&result.pages[1])[0]), "Text");
}

#[test]
fn inserted_layouts_apply_from_the_next_page_g21() {
    let mut document = plain_text("Page one\nPage two\nPage three");
    let mut inserted = document.new_standard_layout("1");
    inserted.right.layout.margins = Margins::uniform(2_880);
    let inserted_id = inserted.id;
    document.layouts_mut().push(std::sync::Arc::new(inserted));
    append_object(&mut document, 0, page_break(None, false));
    append_object(&mut document, 1, page_break(Some(inserted_id), false));
    let result = layout(&document);
    assert_eq!(result.pages.len(), 3);
    assert_eq!(
        result.pages[1].columns[0].x, 1_440.0,
        "page 2 keeps the old layout"
    );
    assert_eq!(result.pages[2].layout, inserted_id);
    assert_eq!(result.pages[2].columns[0].x, 2_880.0);
    assert_eq!(main_lines(&result.pages[2])[0].x, 2_880.0);
}

#[test]
fn a_document_can_start_in_an_inserted_layout() {
    // Ami Pro's `[l1]`: every page uses the inserted layout until a break
    // names another (the corpus's page tables name it on every page).
    let mut document = plain_text("Page one\nPage two");
    let mut inserted = document.new_standard_layout("1");
    inserted.right.layout.margins = Margins::uniform(2_880);
    let inserted_id = inserted.id;
    document.layouts_mut().push(std::sync::Arc::new(inserted));
    document.set_start_layout(Some(inserted_id));
    append_object(&mut document, 0, page_break(None, false));
    let result = layout(&document);
    assert_eq!(result.pages.len(), 2);
    for page in &result.pages {
        assert_eq!(page.layout, inserted_id);
        assert_eq!(page.columns[0].x, 2_880.0);
    }
}

#[test]
fn centring_breaks_centre_the_text_above_them_v23() {
    let mut document = plain_text("Centred\nNext");
    append_object(&mut document, 0, page_break(None, true));
    let result = layout(&document);
    let line = main_lines(&result.pages[0])[0];
    assert_eq!(line.top, 1_440.0 + (12_960.0 - LINE) / 2.0);
    assert_eq!(main_lines(&result.pages[1])[0].top, 1_440.0);
}

#[test]
fn the_page_count_includes_the_highest_where_placed_frame_v11_x06() {
    let mut document = plain_text("One page of text");
    let frame = fixed_frame(
        &mut document,
        3,
        Rect::new(2_000, 2_000, 4_000, 4_000),
        Wrap::Around,
    );
    let result = layout(&document);
    assert_eq!(result.pages.len(), 3);
    assert!(
        result.pages[1]
            .frames
            .iter()
            .all(|frame_box| frame_box.frame != frame)
    );
    let placed = result.pages[2]
        .frames
        .iter()
        .find(|frame_box| frame_box.frame == frame)
        .unwrap();
    assert_eq!(
        placed.bounds,
        Bounds {
            x: 2_000.0,
            y: 2_000.0,
            width: 2_000.0,
            height: 2_000.0
        }
    );
    assert_eq!(result.pages[2].layout, document.document_layout().id);
}

fn two_columns(document: &mut Document, balance: bool) {
    let layout = &mut document.document_layout_mut().right.layout;
    layout.columns = 2;
    layout.gutter = 240;
    layout.tabs = vec![Vec::new(), Vec::new()];
    layout.balance = balance;
}

#[test]
fn text_flows_through_columns_and_column_breaks_g05() {
    // Columns are (9,360 − 240) / 2 = 4,560 wide: 38 characters.
    let mut document = plain_text(&words(500));
    two_columns(&mut document, false);
    let result = layout(&document);
    let columns = &result.pages[0].columns;
    assert_eq!(columns[1].x, 1_440.0 + 4_560.0 + 240.0);
    let lines = main_lines(&result.pages[0]);
    // Six words per 4,560-twip line: 84 lines, 54 filling the first column.
    assert_eq!(
        lines.iter().filter(|line| line.x == columns[0].x).count(),
        54
    );
    assert_eq!(
        lines.iter().filter(|line| line.x == columns[1].x).count(),
        30
    );
    assert!(lines.iter().all(|line| line.width <= 4_560.0));

    let mut document = plain_text("Left\nRight");
    two_columns(&mut document, false);
    append_object(
        &mut document,
        0,
        InlineObject::Break {
            kind: BreakKind::Column,
            centre: false,
            layout: None,
            restart: false,
        },
    );
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    assert_eq!(
        (lines[1].x, lines[1].top),
        (result.pages[0].columns[1].x, 1_440.0)
    );

    // A column break anywhere in a paragraph ends the column after the
    // paragraph (decompile; V22 wrote `<:c>` at the paragraph's start).
    let mut document = plain_text(&format!(
        "{}
Next",
        words(30)
    ));
    two_columns(&mut document, false);
    let paragraph = document.paragraph_mut(0);
    paragraph.text.insert(0, OBJECT);
    paragraph.objects.insert(
        0,
        InlineObject::Break {
            kind: BreakKind::Column,
            centre: false,
            layout: None,
            restart: false,
        },
    );
    let result = layout(&document);
    let columns = &result.pages[0].columns;
    let first = document.paragraphs()[0].id;
    let lines = main_lines(&result.pages[0]);
    let (broken, next): (Vec<&LineBox>, Vec<&LineBox>) = lines
        .iter()
        .copied()
        .partition(|line| line.paragraph == first);
    assert!(broken.len() > 1);
    assert!(broken.iter().all(|line| line.x == columns[0].x));
    assert_eq!((next[0].x, next[0].top), (columns[1].x, 1_440.0));
}

#[test]
fn column_balance_evens_the_last_page_g06() {
    let mut document = plain_text(&["line"; 10].join("\n"));
    document
        .styles_mut()
        .get_mut(&StyleId(1))
        .unwrap()
        .spacing
        .below = 0;
    two_columns(&mut document, true);
    let result = layout(&document);
    let columns = &result.pages[0].columns;
    let lines = main_lines(&result.pages[0]);
    let left = lines.iter().filter(|line| line.x == columns[0].x).count();
    let right = lines.iter().filter(|line| line.x == columns[1].x).count();
    assert_eq!((left, right), (5, 5));
}

#[test]
fn left_pages_use_their_own_geometry_g15_g16() {
    let mut document = plain_text(&vec![words(10); 200].join("\n"));
    let mut left = document.document_layout().right.clone();
    left.layout.margins.left = 2_160;
    document.document_layout_mut().left = Some(left);
    document.document_layout_mut().mirror = true;
    let result = layout(&document);
    assert!(result.pages.len() >= 3);
    assert_eq!(result.pages[0].side, PageSide::Right);
    assert_eq!(result.pages[0].columns[0].x, 1_440.0);
    assert_eq!(result.pages[1].side, PageSide::Left);
    assert_eq!(result.pages[1].columns[0].x, 2_160.0);
    assert_eq!(result.pages[2].columns[0].x, 1_440.0);
}

fn header_lines(page: &PageBox) -> Vec<&LineBox> {
    page.lines
        .iter()
        .filter(|line| line.container != Container::Main && line.top < 1_440.0)
        .collect()
}

fn set_header(document: &mut Document, left: bool, text: &str) {
    let layout = document.document_layout();
    let frame = if left {
        layout.header.left
    } else {
        layout.header.right
    }
    .unwrap();
    frame_text(document, frame, text);
}

#[test]
fn headers_follow_sides_the_second_page_rule_and_floating_overrides_g15_g19_g22_v17() {
    let mut document = plain_text(&vec![words(10); 200].join("\n"));
    let right = document.document_layout().right.clone();
    document.document_layout_mut().left = Some(right);
    let left_header = document.create_text_frame(
        Placement::Header {
            side: PageSide::Left,
        },
        document
            .frame(document.document_layout().header.right.unwrap())
            .unwrap()
            .style
            .clone(),
    );
    document.document_layout_mut().header.left = Some(left_header);
    set_header(&mut document, false, "Right head");
    set_header(&mut document, true, "Left head");
    document.document_layout_mut().header_from_second_page = true;
    let result = layout(&document);
    assert!(
        header_lines(&result.pages[0]).is_empty(),
        "G19: no header on page 1"
    );
    assert_eq!(
        text_of(&document, header_lines(&result.pages[1])[0]),
        "Left head"
    );
    assert_eq!(
        text_of(&document, header_lines(&result.pages[2])[0]),
        "Right head"
    );
    // The header sits in the top margin band, inside its frame's margins.
    assert_eq!(header_lines(&result.pages[2])[0].top, 792.0);

    // G22, V17: a floating header from the page after its mark; an empty one
    // blanks the header.
    let floating = document.create_flow(StyleId(1));
    document
        .flow_mut(floating)
        .unwrap()
        .paragraph_mut(0)
        .unwrap()
        .text = "Floating".into();
    append_object(
        &mut document,
        0,
        InlineObject::FloatingHeaderFooter {
            kind: HeaderFooterKind::Header,
            pages: PageSides::Right,
            flow: floating,
        },
    );
    let result = layout(&document);
    assert_eq!(
        text_of(&document, header_lines(&result.pages[1])[0]),
        "Left head"
    );
    assert_eq!(
        text_of(&document, header_lines(&result.pages[2])[0]),
        "Floating"
    );
    assert!(matches!(
        header_lines(&result.pages[2])[0].container,
        Container::Floating(flow) if flow == floating
    ));
}

#[test]
fn page_number_fields_show_their_page_g18() {
    let mut document = plain_text(&vec![words(10); 200].join("\n"));
    let header = document.document_layout().header.right.unwrap();
    let FrameContent::Text(flow) = document.frame(header).unwrap().content else {
        panic!("text header");
    };
    let paragraph = document.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
    paragraph.text = format!("Header text.\t{OBJECT}");
    paragraph
        .objects
        .push(InlineObject::PageNumber(PageNumberField {
            leading_text: "Page ".into(),
            ..PageNumberField::plain()
        }));
    let result = layout(&document);
    for page in &result.pages[..2] {
        let field = header_lines(page)[0]
            .words
            .iter()
            .find(|word| word.object == Some(0))
            .unwrap();
        assert_eq!(
            field.text.as_deref(),
            Some(format!("Page {}", page.number).as_str())
        );
        assert_eq!(
            field.width,
            6.0 * 120.0,
            "measured in the paragraph's style"
        );
    }
}

#[test]
fn pages_after_a_toc_break_are_numbered_from_one_again_ti3t() {
    // TI3T: Ami Pro's TOC page, its break (flag 0x02), then the text, whose
    // first page shows "Page 1" again; later pages count on from there.
    let mut document = plain_text(&["TOC", "One", "Two"].join("\n"));
    append_object(
        &mut document,
        0,
        InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: None,
            restart: true,
        },
    );
    append_object(&mut document, 1, page_break(None, false));
    let header = document.document_layout().header.right.unwrap();
    let FrameContent::Text(flow) = document.frame(header).unwrap().content else {
        panic!("text header");
    };
    let paragraph = document.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
    paragraph.text = format!("Page {OBJECT}");
    paragraph
        .objects
        .push(InlineObject::PageNumber(PageNumberField::plain()));
    let result = layout(&document);
    assert_eq!(result.pages.len(), 3);
    let shown: Vec<(u32, u32, String)> = result
        .pages
        .iter()
        .map(|page| {
            let field = header_lines(page)[0]
                .words
                .iter()
                .find(|word| word.object == Some(0))
                .and_then(|word| word.text.clone())
                .unwrap_or_default();
            (page.number, page.shown, field)
        })
        .collect();
    assert_eq!(
        shown,
        [
            (1, 1, "1".to_string()),
            (2, 1, "1".to_string()),
            (3, 2, "2".to_string())
        ]
    );
}

#[test]
fn page_number_fields_start_on_their_page_in_their_style() {
    // `1308:0026`: nothing before the start page, then the start number, in
    // roman numerals for style `i` (`PAGE_LAYOUT_NOTES.md`).
    let mut document = plain_text(&vec![words(10); 300].join("\n"));
    let header = document.document_layout().header.right.unwrap();
    let FrameContent::Text(flow) = document.frame(header).unwrap().content else {
        panic!("text header");
    };
    let paragraph = document.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
    paragraph.text = OBJECT.to_string();
    paragraph
        .objects
        .push(InlineObject::PageNumber(PageNumberField {
            style: 'i',
            start_page: 2,
            start_number: 1,
            leading_text: "- ".into(),
        }));
    let result = layout(&document);
    let shown: Vec<_> = result.pages[..4]
        .iter()
        .map(|page| {
            header_lines(page)[0]
                .words
                .iter()
                .find(|word| word.object == Some(0))
                .and_then(|word| word.text.clone())
                .unwrap()
        })
        .collect();
    assert_eq!(shown, ["", "- i", "- ii", "- iii"]);
}

#[test]
fn page_numbers_take_the_text_formatting_where_they_stand_pf1() {
    use amipro_document::{CharacterFormat, CharacterRun};
    // VM fixture PF1: Text › Font on G18's field, 24 pt red, stored
    // `<:f480,2Times New Roman,255,0,0><:P10,0,Page ><:f>`; Ami Pro sets
    // the number in 24 pt and its line grows.
    let mut document = plain_text(&vec![words(10); 40].join("\n"));
    let header = document.document_layout().header.right.unwrap();
    let FrameContent::Text(flow) = document.frame(header).unwrap().content else {
        panic!("text header");
    };
    let paragraph = document.flow_mut(flow).unwrap().paragraph_mut(0).unwrap();
    paragraph.text = format!("Header{OBJECT}");
    paragraph
        .objects
        .push(InlineObject::PageNumber(PageNumberField {
            leading_text: "Page ".into(),
            ..PageNumberField::plain()
        }));
    paragraph.runs.push(CharacterRun {
        start: 6,
        end: 9,
        format: CharacterFormat {
            font_size_points: Some(24.0),
            colour: Some(255),
            ..CharacterFormat::default()
        },
    });
    let result = layout(&document);
    let line = header_lines(&result.pages[0])[0];
    let field = line
        .words
        .iter()
        .find(|word| word.object == Some(0))
        .unwrap();
    assert_eq!(field.text.as_deref(), Some("Page 1"));
    // Six characters at 24 pt in the fixed shaper: 6 × 240.
    assert_eq!(field.width, 6.0 * 240.0);
    // The line is as tall as the 24 pt field (480), not the 12 pt text.
    assert_eq!(line.height, 480.0);
}

#[test]
fn dates_and_doc_info_entries_show_their_text_fd1_fd2() {
    use amipro_document::{
        DateField, DateSource, DocInfoField, DocumentInfo, FieldContext, WallClock,
    };
    use amipro_layout::layout_document_with;
    // FD1's System date and FD2's Filename, each one piece in the line,
    // measured in the paragraph's style; with no clock (the default
    // context) the date is 1 January 1970's.
    let mut document = plain_text("S: x\nF: y");
    *document.info_mut() = DocumentInfo::new(1_792_520_631);
    let paragraph = document.paragraph_mut(0);
    paragraph.text = format!("S: {OBJECT}");
    paragraph.objects.push(InlineObject::Date(DateField {
        source: DateSource::Today,
        format: b'b',
    }));
    let paragraph = document.paragraph_mut(1);
    paragraph.text = format!("F: {OBJECT}");
    paragraph.objects.push(InlineObject::DocInfo(DocInfoField {
        code: 1,
        format: b'a',
    }));
    document.validate().unwrap();
    let context = FieldContext {
        now: WallClock {
            year: 2026,
            month: 10,
            day: 20,
            hour: 11,
            minute: 25,
            second: 0,
        },
        file_name: "FD1.SAM".into(),
        ..FieldContext::default()
    };
    let shown = |result: &DocumentLayout| -> Vec<(Option<String>, f32)> {
        main_lines(&result.pages[0])
            .iter()
            .map(|line| {
                let word = line
                    .words
                    .iter()
                    .find(|word| word.object == Some(0))
                    .unwrap();
                (word.text.clone(), word.width)
            })
            .collect()
    };
    let result = layout_document_with(
        &document,
        &mut FixedShaper,
        &mut ShapeCache::default(),
        &context,
    );
    assert_eq!(
        shown(&result),
        [
            (Some("October 20, 2026".into()), 16.0 * 120.0),
            (Some("FD1.SAM".into()), 7.0 * 120.0),
        ]
    );
    assert_eq!(
        shown(&layout(&document))[1].0.as_deref(),
        Some("(Untitled)")
    );
    // A new context lays out a document with fields again.
    let mut incremental = amipro_layout::IncrementalLayout::new();
    incremental.layout(&document, &mut FixedShaper);
    incremental.set_field_context(context);
    let again = incremental.layout(&document, &mut FixedShaper);
    assert_eq!(shown(again)[0].0.as_deref(), Some("October 20, 2026"));
}

#[test]
fn inserted_bullets_take_the_size_where_they_stand_bl1() {
    use amipro_document::{CharacterFormat, CharacterRun};
    // VM fixture BL1: a dot (half the ascent wide) in 12 pt text, and one
    // after a change to 24 pt, which Ami Pro draws twice as large in a
    // taller line.
    let mut document = plain_text("A dot\nBig dot");
    let paragraph = document.paragraph_mut(0);
    paragraph.text = format!("A{OBJECT}dot");
    paragraph.objects.push(InlineObject::Bullet('\u{11}'));
    let paragraph = document.paragraph_mut(1);
    paragraph.text = format!("Big{OBJECT}dot");
    paragraph.objects.push(InlineObject::Bullet('\u{11}'));
    paragraph.runs.push(CharacterRun {
        start: 3,
        end: 3 + OBJECT.len_utf8() + 3,
        format: CharacterFormat {
            font_size_points: Some(24.0),
            ..CharacterFormat::default()
        },
    });
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let bullet = |line: &LineBox| {
        line.words
            .iter()
            .find(|word| word.object == Some(0))
            .unwrap()
            .clone()
    };
    assert_eq!(bullet(lines[0]).width, 240.0 * 0.9 / 2.0);
    assert_eq!(bullet(lines[1]).width, 480.0 * 0.9 / 2.0);
    assert_eq!(lines[1].height, 480.0);
    // The text after it starts where the bullet ends.
    let after = lines[0]
        .words
        .iter()
        .find(|word| word.bytes.start == 1 + OBJECT.len_utf8())
        .unwrap();
    assert_eq!(after.x, bullet(lines[0]).x + bullet(lines[0]).width);
}

fn repeating(document: &mut Document, pages: RepeatPages, rect: Rect) -> FrameId {
    let frame =
        document.create_text_frame(Placement::Repeating { pages, rect }, FrameStyle::default());
    document.document_layout_mut().repeating.push(frame);
    frame
}

#[test]
fn repeating_frames_sit_below_page_frames_in_stack_order_f08_f09_z01_v14() {
    let mut document = plain_text(&vec![words(10); 120].join("\n"));
    let all = repeating(
        &mut document,
        RepeatPages::All,
        Rect::new(100, 100, 400, 400),
    );
    let mirrored = repeating(
        &mut document,
        RepeatPages::RightLeft,
        Rect::new(100, 500, 400, 800),
    );
    let back = fixed_frame(
        &mut document,
        1,
        Rect::new(2_000, 2_000, 3_000, 3_000),
        Wrap::NoWrapAround,
    );
    let front = fixed_frame(
        &mut document,
        1,
        Rect::new(2_500, 2_500, 3_500, 3_500),
        Wrap::NoWrapAround,
    );
    let result = layout(&document);
    let order: Vec<_> = result.pages[0]
        .frames
        .iter()
        .filter(|frame_box| {
            frame_box.frame != document.document_layout().header.right.unwrap()
                && frame_box.frame != document.document_layout().footer.right.unwrap()
        })
        .map(|frame_box| (frame_box.frame, frame_box.layer))
        .collect();
    assert_eq!(
        order,
        [
            (all, Layer::Layout),
            (mirrored, Layer::Layout),
            (back, Layer::Page),
            (front, Layer::Page)
        ]
    );
    let on_page_two = |frame| {
        result.pages[1]
            .frames
            .iter()
            .find(|frame_box| frame_box.frame == frame)
            .map(|frame_box| frame_box.bounds.x)
    };
    assert_eq!(on_page_two(all), Some(100.0));
    assert_eq!(
        on_page_two(mirrored),
        Some(12_240.0 - 400.0),
        "mirrored on left pages"
    );
    assert_eq!(on_page_two(back), None);
}

/// FC7G and FC8: Ami Pro draws Where placed frames above anchored ones,
/// whichever was made first; within a kind the stack decides.
#[test]
fn anchored_frames_are_drawn_below_where_placed_ones_fc8() {
    let order = |document: &Document| -> Vec<FrameId> {
        layout(document).pages[0]
            .frames
            .iter()
            .filter(|frame_box| frame_box.layer == Layer::Page)
            .map(|frame_box| frame_box.frame)
            .collect()
    };
    let below = Placement::BelowParagraph {
        x_offset: 0,
        size: Size {
            width: 2_000,
            height: 1_000,
        },
    };
    // FC8: the Where placed frame first, the anchored one after.
    let mut document = plain_text("One\nTwo");
    let fixed = fixed_frame(
        &mut document,
        1,
        Rect::new(2_160, 3_600, 5_040, 5_760),
        Wrap::NoWrapAround,
    );
    let first = anchored(&mut document, 1, false, below, Wrap::NoWrapBeside);
    let second = anchored(&mut document, 0, false, below, Wrap::NoWrapBeside);
    assert_eq!(document.frame_stack(), [fixed, first, second]);
    assert_eq!(order(&document), [first, second, fixed]);
    // FC7G: the anchored one first.
    let mut document = plain_text("One\nTwo");
    let anchored_frame = anchored(&mut document, 1, false, below, Wrap::NoWrapBeside);
    let fixed = fixed_frame(
        &mut document,
        1,
        Rect::new(3_600, 3_600, 6_480, 5_760),
        Wrap::NoWrapAround,
    );
    assert_eq!(order(&document), [anchored_frame, fixed]);
}

/// C04: a frame with two columns and a line between them draws the line
/// centred in the gutter, as tall as the text area, with only one column
/// holding text; none with one column.
#[test]
fn a_frames_line_between_columns_runs_down_its_gutter_c04() {
    let mut document = plain_text("Main");
    let frame = fixed_frame(
        &mut document,
        1,
        Rect::new(2_160, 2_880, 5_040, 5_040),
        Wrap::Around,
    );
    {
        let style = &mut document.frame_mut(frame).unwrap().style;
        style.interior.columns = 2;
        style.interior.tabs = vec![Vec::new(), Vec::new()];
        style.interior.line_between = Some(ColumnLine {
            style: 2,
            colour: 0,
        });
    }
    frame_text(&mut document, frame, "Frame text here.");
    let result = layout(&document);
    let frame_box = result.pages[0]
        .frames
        .iter()
        .find(|frame_box| frame_box.frame == frame)
        .unwrap();
    assert_eq!(frame_box.strokes.len(), 1, "{:?}", frame_box.strokes);
    let stroke = frame_box.strokes[0].bounds;
    let interior = frame_box.interior;
    let gutter_centre = interior.x + interior.width / 2.0;
    assert!(
        (stroke.x + stroke.width / 2.0 - gutter_centre).abs() < 1.0,
        "{stroke:?}"
    );
    assert_eq!((stroke.y, stroke.height), (interior.y, interior.height));
    // One column: no line.
    document.frame_mut(frame).unwrap().style.interior.columns = 1;
    document.frame_mut(frame).unwrap().style.interior.tabs = vec![Vec::new()];
    let result = layout(&document);
    assert!(
        result.pages[0]
            .frames
            .iter()
            .all(|frame_box| frame_box.strokes.is_empty())
    );
}

fn anchored(
    document: &mut Document,
    paragraph: usize,
    at_start: bool,
    placement: Placement,
    wrap: Wrap,
) -> FrameId {
    let frame = document.create_text_frame(
        placement,
        FrameStyle {
            wrap,
            ..FrameStyle::default()
        },
    );
    document.frame_stack_mut().push(frame);
    let target = document.paragraph_mut(paragraph);
    if at_start {
        target.text.insert(0, OBJECT);
        target.objects.insert(0, InlineObject::FrameAnchor(frame));
    } else {
        target.text.push(OBJECT);
        target.objects.push(InlineObject::FrameAnchor(frame));
    }
    frame
}

fn bounds_of(result: &DocumentLayout, frame: FrameId) -> (u32, Bounds) {
    result
        .pages
        .iter()
        .find_map(|page| {
            page.frames
                .iter()
                .find(|frame_box| frame_box.frame == frame)
                .map(|frame_box| (page.number, frame_box.bounds))
        })
        .unwrap()
}

#[test]
fn with_para_above_frames_follow_their_paragraph_clamped_to_the_column_f06_v12() {
    let mut document = plain_text("Anchor paragraph\nAfter");
    let frame = anchored(
        &mut document,
        0,
        false,
        Placement::BelowParagraph {
            x_offset: 720,
            size: Size {
                width: 20_000,
                height: 1_000,
            },
        },
        Wrap::NoWrapBeside,
    );
    let result = layout(&document);
    let (page, bounds) = bounds_of(&result, frame);
    assert_eq!(page, 1);
    // Clamped to the column: as wide as the column, so the offset is spent.
    // It stands below the text, before the space after (MERCURY.SAM).
    assert_eq!(
        bounds,
        Bounds {
            x: 1_440.0,
            y: 1_440.0 + LINE,
            width: 9_360.0,
            height: 1_000.0
        }
    );
    let lines = main_lines(&result.pages[0]);
    assert_eq!(
        lines[1].top,
        bounds.bottom() + AFTER,
        "the next paragraph starts below the frame"
    );

    // D7 refined: an anchor at the start of its paragraph puts the frame
    // before it (`<:t n>` tables, TB1).
    let mut document = plain_text("Before\nTable follows");
    let frame = anchored(
        &mut document,
        1,
        true,
        Placement::BelowParagraph {
            x_offset: 0,
            size: Size {
                width: 2_000,
                height: 600,
            },
        },
        Wrap::NoWrapBeside,
    );
    let result = layout(&document);
    let (_, bounds) = bounds_of(&result, frame);
    assert_eq!(bounds.y, 1_440.0 + LINE + AFTER);
    assert_eq!(main_lines(&result.pages[0])[1].top, bounds.bottom());

    // V12: a frame that does not fit takes its paragraph to the next page.
    // Paragraphs take 360 twips, so paragraph 35 is the last on page 1.
    let mut document = plain_text(&["x"; 40].join("\n"));
    let frame = anchored(
        &mut document,
        35,
        false,
        Placement::BelowParagraph {
            x_offset: 0,
            size: Size {
                width: 2_000,
                height: 3_000,
            },
        },
        Wrap::NoWrapBeside,
    );
    let result = layout(&document);
    let (page, bounds) = bounds_of(&result, frame);
    assert_eq!(page, 2);
    let paragraph_page = result.page_of(35, 0).unwrap();
    assert_eq!(paragraph_page, 2, "the paragraph moved with its frame");
    assert_eq!(bounds.y, 1_440.0 + LINE);
}

/// MERCURY.SAM, whose stored rectangles for seven icons and a picture follow
/// from its paragraph caches: text runs through a With para above frame set
/// to No wrap around, so the text after the frame starts beside it; a
/// paragraph that holds only anchors takes no height; and a table always
/// takes its height.
#[test]
fn text_runs_beside_with_para_above_frames_without_wrap_mercury() {
    let below = Placement::BelowParagraph {
        x_offset: 0,
        size: Size {
            width: 576,
            height: 576,
        },
    };

    // An anchor that ends a paragraph: the frame stands below the text, and
    // the next paragraph starts after the space after, beside the frame.
    let mut document = plain_text("Subhead\nText beside the icon");
    let frame = anchored(&mut document, 0, false, below, Wrap::NoWrapAround);
    let result = layout(&document);
    let (_, bounds) = bounds_of(&result, frame);
    assert_eq!(bounds.y, 1_440.0 + LINE);
    assert_eq!(main_lines(&result.pages[0])[1].top, 1_440.0 + LINE + AFTER);

    // An anchor that starts a paragraph: the frame and the first line share
    // a top, and the paragraph keeps its height.
    let mut document = plain_text("Subhead\nText beside the icon\nNext");
    let frame = anchored(&mut document, 1, true, below, Wrap::NoWrapAround);
    let result = layout(&document);
    let (_, bounds) = bounds_of(&result, frame);
    let lines = main_lines(&result.pages[0]);
    assert_eq!(lines[1].top, bounds.y);
    assert_eq!(lines[2].top, 1_440.0 + 2.0 * (LINE + AFTER));

    // A paragraph that holds only the anchor takes no height; its line keeps
    // a baseline for the caret. No wrap beside still keeps text below.
    for (wrap, after_frame) in [(Wrap::NoWrapAround, false), (Wrap::NoWrapBeside, true)] {
        let mut document = plain_text("Subhead\n\nText beside the icon");
        let frame = anchored(&mut document, 1, true, below, wrap);
        let result = layout(&document);
        let (_, bounds) = bounds_of(&result, frame);
        let lines = main_lines(&result.pages[0]);
        assert_eq!(bounds.y, 1_440.0 + LINE + AFTER);
        let top = if after_frame {
            bounds.bottom()
        } else {
            bounds.y
        };
        assert_eq!((lines[1].top, lines[1].height), (top, 0.0));
        assert!(lines[1].baseline > lines[1].top);
        assert_eq!(lines[2].top, top + AFTER, "{wrap:?}");
    }

    // A table takes its height although its record has no wrap bit, like
    // MERCURY.SAM's sales table, whose anchor has a paragraph to itself.
    let mut document = plain_text("Before\n\nAfter");
    let flow = document.create_flow(StyleId(1));
    let frame = document.allocate_frame_id();
    document.insert_frame(Frame {
        id: frame,
        placement: Placement::BelowParagraph {
            x_offset: 1_936,
            size: Size {
                width: 5_488,
                height: 2_484,
            },
        },
        style: FrameStyle {
            wrap: Wrap::NoWrapAround,
            ..FrameStyle::default()
        },
        group: None,
        content: FrameContent::Table(Table {
            columns: vec![5_488],
            rows: vec![2_484],
            cell_margins: CellMargins::default(),
            spacing: Default::default(),
            row_options: Default::default(),
            layout: Default::default(),
            read_rows: Default::default(),
            cells: vec![TableCell {
                content: CellContent::Text(flow),
                span: CellSpan::SINGLE,
                frame: None,
                format: Default::default(),
                formula: None,
                legacy: LegacyData::default(),
            }],
            legacy: LegacyData::default(),
        }),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().push(frame);
    let target = document.paragraph_mut(1);
    target.text.insert(0, OBJECT);
    target.objects.insert(0, InlineObject::FrameAnchor(frame));
    let result = layout(&document);
    let (_, bounds) = bounds_of(&result, frame);
    let lines = main_lines(&result.pages[0]);
    assert_eq!(
        (bounds.x, bounds.y),
        (1_440.0 + 1_936.0, 1_440.0 + LINE + AFTER)
    );
    assert_eq!(lines[2].top, bounds.bottom() + AFTER);
}

#[test]
fn flow_with_text_frames_stand_on_the_baseline_and_move_with_their_line_x02_v13() {
    let mut document = plain_text("Before and after");
    let frame = document.create_text_frame(
        Placement::Inline {
            y_offset: 0,
            size: Size {
                width: 1_200,
                height: 1_000,
            },
        },
        FrameStyle::default(),
    );
    document.frame_stack_mut().push(frame);
    let paragraph = document.paragraph_mut(0);
    paragraph.text = format!("Before {OBJECT} after");
    paragraph.objects.push(InlineObject::FrameAnchor(frame));
    let result = layout(&document);
    let line = main_lines(&result.pages[0])[0];
    assert_eq!(line.height, 1_000.0 + 48.0, "the frame raises the line");
    let (_, bounds) = bounds_of(&result, frame);
    assert_eq!(
        bounds,
        Bounds {
            x: 1_440.0 + 7.0 * 120.0,
            y: line.baseline - 1_000.0,
            width: 1_200.0,
            height: 1_000.0
        }
    );

    // Z05: the right shadow takes space after the frame, so two frames with
    // the Normal shadow (100 twips) stand 100 twips apart.
    let mut document = plain_text("Two frames");
    let style = FrameStyle {
        shadow: Shadow {
            right: 100,
            bottom: 100,
            ..Shadow::default()
        },
        ..FrameStyle::default()
    };
    let mut frames = Vec::new();
    for _ in 0..2 {
        let frame = document.create_text_frame(
            Placement::Inline {
                y_offset: 0,
                size: Size {
                    width: 1_440,
                    height: 720,
                },
            },
            style.clone(),
        );
        document.frame_stack_mut().push(frame);
        let paragraph = document.paragraph_mut(0);
        paragraph.text.push(OBJECT);
        paragraph.objects.push(InlineObject::FrameAnchor(frame));
        frames.push(frame);
    }
    let result = layout(&document);
    let (_, first) = bounds_of(&result, frames[0]);
    let (_, second) = bounds_of(&result, frames[1]);
    assert_eq!(first.width, 1_440.0, "the box is the frame's own size");
    assert_eq!(second.x - first.x, 1_440.0 + 100.0);

    // V13: a line that does not fit moves to the next page with its frame.
    let mut document = plain_text(&format!("{}\nTall {OBJECT}", vec!["x"; 35].join("\n")));
    let frame = document.create_text_frame(
        Placement::Inline {
            y_offset: 0,
            size: Size {
                width: 1_200,
                height: 4_000,
            },
        },
        FrameStyle::default(),
    );
    document.frame_stack_mut().push(frame);
    document
        .paragraph_mut(35)
        .objects
        .push(InlineObject::FrameAnchor(frame));
    let result = layout(&document);
    let (page, bounds) = bounds_of(&result, frame);
    assert_eq!(page, 2);
    assert_eq!(bounds.y, 1_440.0);
}

/// Body Text with a 720-twip first-line indent and no space after, one
/// paragraph per line of `text`, as `_REPORT2.STY`'s Body Text.
fn indented_text(text: &str) -> Document {
    let mut document = Document::from_plain_text(text);
    let body = document.body_text_style();
    document.styles_mut().get_mut(&body).unwrap().indents = Indents {
        first: 720,
        ..Indents::default()
    };
    document
}

/// Anchors a Flow with text frame of `width` × 408 with `shadow` (right and
/// bottom) at the end of paragraph `index`.
fn inline_frame(document: &mut Document, index: usize, width: i32, shadow: i32) -> FrameId {
    let frame = document.create_text_frame(
        Placement::Inline {
            y_offset: 0,
            size: Size { width, height: 408 },
        },
        FrameStyle {
            shadow: Shadow {
                right: shadow,
                bottom: shadow,
                ..Shadow::default()
            },
            ..FrameStyle::default()
        },
    );
    document.frame_stack_mut().push(frame);
    append_object(document, index, InlineObject::FrameAnchor(frame));
    frame
}

#[test]
fn flow_with_text_frames_hold_their_shadows_and_move_left_when_too_wide_fa1_fa3() {
    // FA1: a frame in a paragraph of its own stands at its line's top, the
    // line ending below its bottom shadow; one that fits keeps the indent.
    let mut document = indented_text("\n\nAfter");
    let narrow = inline_frame(&mut document, 0, 1_440, 172);
    // Wider than the column: as wide as the column leaves beside its right
    // shadow, at the column's left.
    let wide = inline_frame(&mut document, 1, 20_000, 172);
    let result = layout(&document);
    let (_, narrow) = bounds_of(&result, narrow);
    assert_eq!((narrow.x, narrow.y), (1_440.0 + 720.0, 1_440.0));
    let (_, wide) = bounds_of(&result, wide);
    assert_eq!(
        wide,
        Bounds {
            x: 1_440.0,
            y: 1_440.0 + 408.0 + 172.0,
            width: 9_360.0 - 172.0,
            height: 408.0,
        }
    );
    let lines = main_lines(&result.pages[0]);
    assert_eq!(lines[2].top, wide.bottom() + 172.0, "below the shadow");

    // FA3: a little too wide for the indented line, the frame ends with its
    // shadow at the line's right edge; FA2: without a shadow its line is
    // its own height.
    let mut document = indented_text("\nAfter");
    let frame = inline_frame(&mut document, 0, 9_000, 172);
    let result = layout(&document);
    let (_, bounds) = bounds_of(&result, frame);
    assert_eq!(bounds.x, 1_440.0 + 9_360.0 - 9_172.0);
    let mut document = indented_text("\nAfter");
    let frame = inline_frame(&mut document, 0, 1_440, 0);
    let result = layout(&document);
    let (_, bounds) = bounds_of(&result, frame);
    assert_eq!(main_lines(&result.pages[0])[1].top, bounds.bottom());
}

#[test]
fn a_gap_beside_a_wrapping_frame_takes_a_line_whose_first_word_fits_fg1_fg3() {
    // Gaps of 20 and 200 twips (120-twip characters): an empty line fits
    // either, "a" only the wider; the frame's shadow narrows the gap.
    for (gap, shadow, a_beside) in [(20, 0, false), (200, 0, true), (200, 172, false)] {
        let mut document = Document::from_plain_text("\na\nAfter");
        let rect = Rect {
            left: 1_440,
            top: 1_440,
            right: 1_440 + 9_360 - gap,
            bottom: 2_880,
        };
        let frame = fixed_frame(&mut document, 1, rect, Wrap::Around);
        document.frame_mut(frame).unwrap().style.shadow = Shadow {
            right: shadow,
            bottom: shadow,
            ..Shadow::default()
        };
        let result = layout(&document);
        let lines = main_lines(&result.pages[0]);
        let beside = (1_440 + 9_360 - gap + shadow) as f32;
        assert_eq!((lines[0].x, lines[0].top), (beside, 1_440.0), "gap {gap}");
        if a_beside {
            assert_eq!((lines[1].x, lines[1].top), (beside, 1_440.0 + LINE));
        } else {
            assert_eq!(lines[1].x, 1_440.0, "gap {gap} shadow {shadow}");
            assert_eq!(lines[1].top, (2_880 + shadow) as f32);
        }
    }
}

/// A document of three-line paragraphs, its first page ending inside one,
/// and a second style ("Rule") to give break rules to.
fn break_document() -> Document {
    let text: Vec<String> = (0..60)
        .map(|index| format!("{index} {}", words(30)))
        .collect();
    let mut document = plain_text(&text.join("\n"));
    let body = document.paragraphs()[0].style;
    let mut rule = document.styles()[&body].clone();
    rule.id = StyleId(99);
    rule.name = "Rule".into();
    document.styles_mut().insert(rule.id, rule);
    document
}

fn with_rule(document: &mut Document, paragraphs: &[usize], breaks: u16) {
    document.styles_mut().get_mut(&StyleId(99)).unwrap().breaks = Breaks(breaks);
    for index in paragraphs {
        document.paragraph_mut(*index).style = StyleId(99);
    }
}

/// The page holding each line of paragraph `index`.
fn pages_of(result: &DocumentLayout, index: usize) -> Vec<u32> {
    result
        .pages
        .iter()
        .flat_map(|page| {
            main_lines(page)
                .into_iter()
                .filter(|line| line.paragraph_index == index)
                .map(|_| page.number)
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn break_rules_follow_the_style_b1_b8() {
    let plain = break_document();
    let result = layout(&plain);
    // The paragraph that straddles the end of page 1.
    let straddling = (0..60)
        .find(|index| {
            let pages = pages_of(&result, *index);
            pages.first() == Some(&1) && pages.last() == Some(&2)
        })
        .expect("a paragraph across the page end");

    // B1: page break before.
    let mut document = plain.clone();
    with_rule(&mut document, &[3], Breaks::WITHIN | Breaks::PAGE_BEFORE);
    let result = layout(&document);
    assert_eq!(pages_of(&result, 2), [1, 1, 1]);
    assert_eq!(pages_of(&result, 3)[0], 2);
    // B2: page break after.
    let mut document = plain.clone();
    with_rule(&mut document, &[3], Breaks::WITHIN | Breaks::PAGE_AFTER);
    let result = layout(&document);
    assert_eq!(pages_of(&result, 3), [1, 1, 1]);
    assert_eq!(pages_of(&result, 4)[0], 2);
    // B5: no break within: the straddling paragraph goes to page 2 whole.
    let mut document = plain.clone();
    with_rule(&mut document, &[straddling], 0);
    let result = layout(&document);
    assert!(pages_of(&result, straddling).iter().all(|page| *page == 2));
    assert_eq!(*pages_of(&result, straddling - 1).last().unwrap(), 1);
    // B6: the paragraph before it, kept with the next (and so whole), goes
    // to page 2 with it once the straddling one may not break.
    let mut document = plain.clone();
    with_rule(&mut document, &[straddling - 1], Breaks::KEEP_WITH_NEXT);
    let breaks = document.styles()[&document.paragraphs()[straddling].style].breaks;
    let mut kept = document.clone();
    kept.styles_mut()
        .get_mut(&document.paragraphs()[straddling].style)
        .unwrap()
        .breaks = Breaks(breaks.0 & !Breaks::WITHIN);
    let result = layout(&kept);
    assert!(
        pages_of(&result, straddling - 1)
            .iter()
            .all(|page| *page == 2)
    );
    assert!(pages_of(&result, straddling).iter().all(|page| *page == 2));
    // B7: keep with previous on the second does the same.
    let mut document = plain.clone();
    with_rule(&mut document, &[straddling], Breaks::KEEP_WITH_PREVIOUS);
    let result = layout(&document);
    assert!(
        pages_of(&result, straddling - 1)
            .iter()
            .all(|page| *page == 2)
    );
}

#[test]
fn paragraph_and_line_spacing_follow_the_style_s1_s5() {
    let spacing = |document: &mut Document, value: Spacing| {
        let style = document.paragraphs()[0].style;
        document.styles_mut().get_mut(&style).unwrap().spacing = value;
    };
    let base = Spacing {
        line: LineSpacing::Single,
        above: 144,
        below: 288,
        add_at_break: false,
        tightness: 100,
    };
    let mut document = plain_text("First\nSecond");
    spacing(&mut document, base);
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let column = result.pages[0].columns[0];
    // S4: no space above at the top of the page ("When not at break"); below
    // and above add between paragraphs.
    assert_eq!(lines[0].top, column.y);
    assert_eq!(lines[1].top, lines[0].top + lines[0].height + 288.0 + 144.0);
    let single = (lines[0].height, lines[0].baseline - lines[0].top);

    // S5: "Always" adds it at the top of the page too.
    spacing(
        &mut document,
        Spacing {
            add_at_break: true,
            ..base
        },
    );
    let result = layout(&document);
    assert_eq!(main_lines(&result.pages[0])[0].top, column.y + 144.0);

    // S2 and the decompile: Double is twice the line's single height
    // whatever height the style stores (a dialog cache), the baseline half
    // the extra lower; 1 1/2 is h + ⌊h/2⌋.
    for (line_spacing, height) in [
        (LineSpacing::Double(561), single.0 * 2.0),
        (
            LineSpacing::OneAndAHalf(432),
            single.0 + (single.0 / 2.0).floor(),
        ),
        (LineSpacing::Custom(561), 561.0),
    ] {
        spacing(
            &mut document,
            Spacing {
                line: line_spacing,
                ..base
            },
        );
        let result = layout(&document);
        let line = main_lines(&result.pages[0])[0];
        assert_eq!(line.height, height, "{line_spacing:?}");
        assert_eq!(
            line.baseline - line.top,
            single.1 + ((height - single.0) / 2.0).floor(),
            "{line_spacing:?}"
        );
    }
    // A custom height below the single one moves the baseline up by half
    // the difference, but never above the top of the line.
    spacing(
        &mut document,
        Spacing {
            line: LineSpacing::Custom(20),
            ..base
        },
    );
    let result = layout(&document);
    let line = main_lines(&result.pages[0])[0];
    assert_eq!(line.height, 20.0);
    assert_eq!(
        line.baseline - line.top,
        (single.1 - ((single.0 - 20.0) / 2.0).floor()).max(0.0)
    );
}

#[test]
fn printer_line_heights_replace_the_fonts_own() {
    // DECISIONS.md, 2026-10-12, "Line heights follow the printer": a font
    // with a printer line height takes it for its lines, and a shape cache
    // kept between layouts notices when the heights change.
    let mut document = plain_text(&words(60));
    let style = document.styles()[&document.paragraphs()[0].style].clone();
    let mut cache = ShapeCache::default();
    let natural = layout_document(&document, &mut FixedShaper, &mut cache);
    let natural_height = main_lines(&natural.pages[0])[0].height;
    assert_ne!(natural_height, 300.0);
    let mut heights = LineHeights::default();
    heights.insert(
        FaceKey::new(
            &style.font_family.to_uppercase(),
            style.font_size_points,
            style.bold,
            style.italic,
        ),
        300,
    );
    document.set_line_heights(heights);
    document.validate().unwrap();
    let printer = layout_document(&document, &mut FixedShaper, &mut cache);
    let lines = main_lines(&printer.pages[0]);
    assert!(lines.len() > 1);
    assert!(lines.iter().all(|line| line.height == 300.0));
    assert_eq!(printer, layout(&document), "as a fresh cache gives");
    // Another weight is another font: bold text keeps its own height.
    document.styles_mut().get_mut(&style.id).unwrap().bold = true;
    let bold = layout_document(&document, &mut FixedShaper, &mut cache);
    assert_eq!(main_lines(&bold.pages[0])[0].height, natural_height);
}

#[test]
fn paragraph_rules_take_their_space_and_span_their_length_nx1_nx2() {
    use amipro_document::{ParagraphRules, RuleLength, RuleSide};
    // VM fixtures NX1, NX2: a rule takes its space at the top of the first
    // line or the bottom of the last, outside the line spacing; Margins spans
    // the column, Text the line's text, Other its length by the alignment.
    let mut document = plain_text(&format!("{}\nab\ncd", words(40)));
    let style_id = document.paragraphs()[0].style;
    let side = |space| RuleSide {
        on: true,
        line: 2,
        space,
    };
    let mut ruled = document.styles()[&style_id].clone();
    ruled.id = StyleId(90);
    ruled.name = "Ruled".into();
    ruled.indents = Indents {
        all: 720,
        ..Indents::default()
    };
    ruled.spacing = Spacing::below(0);
    ruled.rules = ParagraphRules {
        above: side(100),
        below: side(300),
        length: RuleLength::Text,
        colour: 255,
        ..ParagraphRules::default()
    };
    let mut other = ruled.clone();
    other.id = StyleId(91);
    other.name = "Other".into();
    other.indents = Indents::default();
    other.alignment = Alignment::Right;
    other.rules.length = RuleLength::Other;
    other.rules.other_length = 1440;
    other.rules.above.on = false;
    let mut margins = ruled.clone();
    margins.id = StyleId(92);
    margins.name = "Margins".into();
    margins.rules.length = RuleLength::Margins;
    margins.spacing.line = LineSpacing::Double(0);
    for style in [ruled, other, margins] {
        document.styles_mut().insert(style.id, style);
    }
    document.paragraph_mut(0).style = StyleId(90);
    document.paragraph_mut(1).style = StyleId(91);
    document.paragraph_mut(2).style = StyleId(92);
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    // 40 five-letter words at 120 twips a letter break into lines of the
    // 8,640-twip indented measure.
    let first = lines[0];
    let last = lines
        .iter()
        .rfind(|line| line.paragraph_index == 0)
        .unwrap();
    assert!(
        lines
            .iter()
            .filter(|line| line.paragraph_index == 0)
            .count()
            >= 3
    );
    assert_eq!(first.height, LINE + 100.0);
    assert_eq!(
        first.baseline,
        first.top + 100.0 + 192.0,
        "below the rule's space"
    );
    assert_eq!(last.height, LINE + 300.0);
    let middle = lines[1];
    assert_eq!((middle.height, middle.rules.len()), (LINE, 0));
    // Text: from the indented text's start to its end, in the style's colour.
    let [above] = &first.rules[..] else {
        panic!("one rule above: {:?}", first.rules)
    };
    assert!(!above.below && above.colour == 255 && above.line == 2);
    assert_eq!(above.x, 1440.0 + 720.0);
    assert_eq!(above.top(first), first.top);
    let [below] = &last.rules[..] else {
        panic!("one rule below")
    };
    assert!(below.below);
    assert_eq!(below.top(last), last.top + last.height - 20.0);
    assert_eq!(
        below.x + below.width,
        last.words.last().map_or(0.0, |w| w.x + w.width)
    );
    // Other, right aligned: one inch ending at the text's end.
    let other_line = lines.iter().find(|line| line.paragraph_index == 1).unwrap();
    let [rule] = &other_line.rules[..] else {
        panic!("one rule below")
    };
    assert_eq!((rule.x, rule.width), (1440.0 + 9360.0 - 1440.0, 1440.0));
    // Margins with Double spacing: the column, and 2 × 240 plus both spaces.
    let margins_line = lines.iter().find(|line| line.paragraph_index == 2).unwrap();
    assert_eq!(margins_line.height, 2.0 * LINE + 100.0 + 300.0);
    assert!(
        margins_line
            .rules
            .iter()
            .all(|rule| rule.x == 1440.0 && rule.width == 9360.0)
    );
    assert_eq!(margins_line.rules.len(), 2);
}

#[test]
fn frame_text_needs_room_for_its_space_below_with_add_in_always_fs5() {
    // VM fixtures FS4, FS5: in a frame, a paragraph's last line and its
    // space below must fit together with Add in Always, as on pages; with
    // When not at break the space below does not count at the end.
    let lines_in_frame = |add_at_break: bool| {
        let mut document = plain_text("Main");
        let id = document.paragraphs()[0].style;
        let style = document.styles_mut().get_mut(&id).unwrap();
        style.spacing = Spacing {
            above: 0,
            below: 288,
            add_at_break,
            ..Spacing::below(288)
        };
        let frame = fixed_frame(
            &mut document,
            1,
            Rect::new(1440, 1440, 4320, 1440 + 400),
            Wrap::NoWrapAround,
        );
        document.frame_mut(frame).unwrap().style.interior.margins = Margins::uniform(0);
        frame_text(&mut document, frame, "Line one.");
        let result = layout(&document);
        result.pages[0]
            .lines
            .iter()
            .filter(|line| line.container == Container::Frame(frame))
            .count()
    };
    assert_eq!(lines_in_frame(true), 0, "240 + 288 does not fit 400");
    assert_eq!(lines_in_frame(false), 1);
}

#[test]
fn tabs_reach_the_layouts_stops_the_rulers_and_the_hanging_indents_tb2() {
    use amipro_document::{TabKind, TabStop};
    // VM fixture TB2 and the decompile: the column's stops (Ami Pro's
    // default every half inch), a paragraph's ruler before them, and a
    // hanging indent's stop.
    let mut document = plain_text("a\tb\na\tbc\na\tb\nx\t\t\t\t\t\t\t\t\t\t\t\t\tz");
    document.paragraph_mut(1).format.tabs = Some(vec![TabStop::new(TabKind::Right, 4000)]);
    let id = document.paragraphs()[2].style;
    let mut hanging = document.styles()[&id].clone();
    hanging.id = StyleId(80);
    hanging.name = "Hanging".into();
    hanging.hanging = true;
    hanging.indents = Indents {
        all: 0,
        first: 0,
        rest: 1000,
        right: 0,
    };
    document.styles_mut().insert(hanging.id, hanging);
    document.paragraph_mut(2).style = StyleId(80);
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let word_x = |line: &LineBox, text: &str| {
        let paragraph = &document.paragraphs()[line.paragraph_index].text;
        line.words
            .iter()
            .find(|word| &paragraph[word.bytes.clone()] == text)
            .map(|word| word.x)
            .unwrap()
    };
    // The default stops: "b" at the first half inch.
    assert_eq!(word_x(lines[0], "b"), 1440.0 + 720.0);
    // The ruler's right stop: "bc" (240 twips) ends at 4000.
    assert_eq!(word_x(lines[1], "bc"), 1440.0 + 4000.0 - 240.0);
    // Hanging indent: a stop where the following lines start, ahead of
    // the half-inch stops before it.
    assert_eq!(word_x(lines[2], "b"), 1440.0 + 1000.0);
    // Past the last stop, the tab runs to the line's end and "z" wraps.
    let last: Vec<_> = lines
        .iter()
        .filter(|line| line.paragraph_index == 3)
        .collect();
    assert_eq!(last.len(), 2);
    assert_eq!(word_x(last[1], "z"), 1440.0);
    let tab = last[0].words.last().unwrap();
    assert_eq!(tab.x + tab.width, 1440.0 + 9360.0);
}

#[test]
fn every_column_uses_the_layouts_one_tab_list_g05() {
    use amipro_document::{TabKind, TabStop};
    // G05 stores the default stops after the first column and none after
    // the second; Ami Pro keeps the first list that is not empty for the
    // whole box (`1058:1843`), so the second column's tab reaches it too.
    let mut document = plain_text("Left\na\tb");
    two_columns(&mut document, false);
    document.document_layout_mut().right.layout.tabs =
        vec![vec![TabStop::new(TabKind::Left, 1000)], Vec::new()];
    append_object(
        &mut document,
        0,
        InlineObject::Break {
            kind: BreakKind::Column,
            centre: false,
            layout: None,
            restart: false,
        },
    );
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let second = result.pages[0].columns[1].x;
    assert_eq!(lines[1].x, second);
    let b = lines[1]
        .words
        .iter()
        .find(|word| &document.paragraphs()[1].text[word.bytes.clone()] == "b")
        .unwrap();
    assert_eq!(b.x, second + 1000.0);
}

#[test]
fn text_tightness_fits_more_or_less_on_a_line_tt1() {
    // VM fixture TT2: Tight wraps a paragraph later, Loose sooner.
    let lines_at = |tightness: u16| {
        let mut document = plain_text(&words(60));
        let id = document.paragraphs()[0].style;
        document
            .styles_mut()
            .get_mut(&id)
            .unwrap()
            .spacing
            .tightness = tightness;
        let result = layout(&document);
        main_lines(&result.pages[0])
            .iter()
            .map(|line| text_of(&document, line).len())
            .collect::<Vec<_>>()
    };
    let (tight, normal, loose) = (lines_at(90), lines_at(100), lines_at(115));
    assert!(
        tight[0] > normal[0] && normal[0] > loose[0],
        "{tight:?} {normal:?} {loose:?}"
    );
    assert!(tight.len() <= normal.len() && normal.len() < loose.len());
}

#[test]
fn an_empty_paragraph_takes_its_end_formats_font_and_text_lines_their_own() {
    // Decompile `10d0:0592`, `10d0:359b`: an empty paragraph's line is the
    // font in force at its end (a `<:f200,,>` line in a 12 pt style is
    // 10 pt high); a line with text is measured by its characters alone.
    let mut document = plain_text("\nsmall\nafter");
    let small = amipro_document::CharacterFormat {
        font_size_points: Some(10.0),
        ..amipro_document::CharacterFormat::default()
    };
    document.paragraph_mut(0).end_format = small.clone();
    document
        .paragraph_mut(1)
        .runs
        .push(amipro_document::CharacterRun {
            start: 0,
            end: 5,
            format: small.clone(),
        });
    // The end format of a paragraph with text does not change its line.
    document.paragraph_mut(1).end_format = amipro_document::CharacterFormat {
        font_size_points: Some(30.0),
        ..amipro_document::CharacterFormat::default()
    };
    let mut cache = ShapeCache::default();
    let result = layout_document(&document, &mut FixedShaper, &mut cache);
    let heights: Vec<f32> = main_lines(&result.pages[0])
        .iter()
        .map(|line| line.height)
        .collect();
    assert_eq!(heights, [200.0, 200.0, LINE]);
    // The shape cache notices an end format that changes.
    document.paragraph_mut(0).end_format = amipro_document::CharacterFormat::default();
    let changed = layout_document(&document, &mut FixedShaper, &mut cache);
    assert_eq!(main_lines(&changed.pages[0])[0].height, LINE);
}

#[test]
fn with_add_in_always_the_last_line_needs_room_for_its_space_below() {
    // Decompile: with Always, a paragraph's space below belongs to its last
    // line, and the two must fit in the column together; with When not at
    // break it is counted only before a following line.
    let base = Spacing {
        line: LineSpacing::Single,
        above: 0,
        below: 0,
        add_at_break: false,
        tightness: 100,
    };
    let mut document = plain_text(&vec!["Line"; 200].join("\n"));
    let style = document.paragraphs()[0].style;
    document.styles_mut().get_mut(&style).unwrap().spacing = base;
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let column = result.pages[0].columns[0];
    let last = *lines.last().unwrap();
    let room = column.bottom() - (last.top + last.height);
    assert!(room < last.height, "page 1 is full");
    // Page 1's last paragraph gets its own style, with a space below larger
    // than the room left under it.
    let index = document
        .paragraphs()
        .iter()
        .position(|paragraph| paragraph.id == last.paragraph)
        .unwrap();
    let own = StyleId(99);
    let mut spaced = document.styles()[&style].clone();
    spaced.id = own;
    spaced.name = "Spaced".to_owned();
    document.styles_mut().insert(own, spaced);
    document.paragraph_mut(index).style = own;
    let on_page_1 = |document: &Document, add_at_break: bool| {
        let mut document = document.clone();
        document.styles_mut().get_mut(&own).unwrap().spacing = Spacing {
            below: room.floor() as i32 + 1,
            add_at_break,
            ..base
        };
        document
    };
    let count = |document: &Document| main_lines(&layout(document).pages[0]).len();
    let not_at_break = count(&on_page_1(&document, false));
    let always = count(&on_page_1(&document, true));
    assert_eq!(not_at_break, lines.len());
    assert_eq!(
        always + 1,
        not_at_break,
        "the last line moves on with Always"
    );
}

#[test]
fn style_indents_set_the_first_line_the_others_and_the_right_edge_i1() {
    // I1: All 0.50, First 1.00, Rest 0.25, From right 1.50 in: the first
    // line starts 1.5 in into the column, the others 0.75 in, and none goes
    // past 1.5 in from its right.
    let mut document = plain_text(&words(60));
    let style = document.paragraphs()[0].style;
    document.styles_mut().get_mut(&style).unwrap().indents = Indents {
        all: 720,
        first: 1_440,
        rest: 360,
        right: 2_160,
    };
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    assert!(lines.len() > 2);
    let column = result.pages[0].columns[0];
    assert_eq!(lines[0].x, column.x + 2_160.0);
    assert_eq!(lines[1].x, column.x + 1_080.0);
    assert_eq!(lines[0].slot_width, column.width - 2_160.0 - 2_160.0);
    assert_eq!(lines[1].slot_width, column.width - 1_080.0 - 2_160.0);
    assert!(
        lines
            .iter()
            .all(|line| line.x + line.width <= column.right() - 2_160.0 + 0.5)
    );

    // Indents wider than a narrow column still leave 0.10 in for text.
    document.styles_mut().get_mut(&style).unwrap().indents.all = 20_000;
    let result = layout(&document);
    let line = main_lines(&result.pages[0])[0];
    assert_eq!(line.slot_width, 144.0);
    assert!(line.x + line.slot_width <= column.right() + 0.5);
}

#[test]
fn paragraph_formats_replace_the_style_indents_and_line_spacing_o3() {
    // O3: Text › Indention and Text › Spacing on single paragraphs. A 1 1/2
    // or Double without a stored height is 1.5 or 2 times the single line
    // (LS1); Custom is the height set.
    let mut document = plain_text(&format!(
        "{}\n{}\n{}\n{}",
        words(60),
        words(60),
        words(60),
        words(60)
    ));
    let style = document.paragraphs()[0].style;
    document.styles_mut().get_mut(&style).unwrap().indents = Indents {
        all: 720,
        ..Indents::default()
    };
    document.paragraph_mut(0).format.indents = Some(Indents {
        all: 360,
        first: 720,
        rest: 1_080,
        right: 1_440,
    });
    document.paragraph_mut(1).format.line_spacing = Some(LineSpacing::OneAndAHalf(0));
    document.paragraph_mut(2).format.line_spacing = Some(LineSpacing::Custom(432));
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let column = result.pages[0].columns[0];
    let of = |index: usize| -> Vec<&LineBox> {
        let id = document.paragraphs()[index].id;
        lines
            .iter()
            .copied()
            .filter(|line| line.paragraph == id)
            .collect()
    };
    let indented = of(0);
    assert_eq!(indented[0].x, column.x + 1_080.0);
    assert_eq!(indented[1].x, column.x + 1_440.0);
    assert_eq!(indented[0].slot_width, column.width - 1_080.0 - 1_440.0);
    // The style's own indent still applies where nothing overrides it.
    let single = of(3);
    assert_eq!(single[0].x, column.x + 720.0);
    let half = of(1);
    assert_eq!(half[0].x, column.x + 720.0);
    assert_eq!(
        half[0].height,
        single[0].height + (single[0].height / 2.0).floor()
    );
    assert_eq!(of(2)[0].height, 432.0);

    // Over a style with a stored 1 1/2 height, the stored height is still
    // only the dialog's cache: the line is h + ⌊h/2⌋ (decompile).
    document.styles_mut().get_mut(&style).unwrap().spacing.line = LineSpacing::OneAndAHalf(500);
    let result = layout(&document);
    let id = document.paragraphs()[1].id;
    let line = main_lines(&result.pages[0])
        .into_iter()
        .find(|line| line.paragraph == id)
        .unwrap();
    let single = single[0].height;
    assert_eq!(line.height, single + (single / 2.0).floor());
}

#[test]
fn alignment_moves_lines_in_their_slot_o2() {
    // O2: Center, Right, Justify, and Left over single paragraphs. A
    // justified paragraph's last line stays left.
    let mut document = plain_text(&format!("Centre\nRight\n{}\nLeft", words(60)));
    document.paragraph_mut(0).format.alignment = Some(Alignment::Centre);
    document.paragraph_mut(1).format.alignment = Some(Alignment::Right);
    let style = document.paragraphs()[2].style;
    // Over the style: the paragraph's own alignment wins.
    document.styles_mut().get_mut(&style).unwrap().alignment = Alignment::Justify;
    document.paragraph_mut(3).format.alignment = Some(Alignment::Left);
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let column = result.pages[0].columns[0];
    let of = |index: usize| -> Vec<&LineBox> {
        let id = document.paragraphs()[index].id;
        lines
            .iter()
            .copied()
            .filter(|line| line.paragraph == id)
            .collect()
    };
    let end = |line: &LineBox| {
        let last = line
            .words
            .iter()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        last.x + last.width
    };
    let centre = of(0)[0];
    assert!((centre.words[0].x - (column.x + (column.width - centre.width) / 2.0)).abs() < 0.01);
    let right = of(1)[0];
    assert!((end(right) - column.right()).abs() < 0.01);
    let justified = of(2);
    assert!(justified.len() > 2);
    let text = &document.paragraphs()[2].text;
    for line in &justified[..justified.len() - 1] {
        assert_eq!(line.words[0].x, line.x);
        // The last word that is not a space ends at the slot's right edge
        // (trailing spaces hang past it).
        let last_text = line
            .words
            .iter()
            .filter(|word| !text[word.bytes.clone()].trim().is_empty())
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        assert!(
            (last_text.x + last_text.width - (line.x + line.slot_width)).abs() < 0.01,
            "{} + {} against {} + {} (content {}); words {:?}",
            last_text.x,
            last_text.width,
            line.x,
            line.slot_width,
            line.width,
            line.words
                .iter()
                .map(|w| (w.x, w.width, &text[w.bytes.clone()]))
                .collect::<Vec<_>>()
        );
    }
    let last = justified.last().unwrap();
    let packed = last
        .words
        .windows(2)
        .all(|pair| (pair[1].x - (pair[0].x + pair[0].width)).abs() < 0.01);
    assert!(packed, "the last line is set left, without extra space");
    assert_eq!(of(3)[0].words[0].x, column.x);
}

#[test]
fn flow_with_text_frames_follow_their_aligned_line() {
    let mut document = plain_text("Before");
    let frame = document.create_text_frame(
        Placement::Inline {
            y_offset: 0,
            size: Size {
                width: 600,
                height: 400,
            },
        },
        FrameStyle::default(),
    );
    document.frame_stack_mut().push(frame);
    let paragraph = document.paragraph_mut(0);
    paragraph.text = format!("Before {OBJECT}");
    paragraph.objects.push(InlineObject::FrameAnchor(frame));
    paragraph.format.alignment = Some(Alignment::Right);
    let result = layout(&document);
    let column = result.pages[0].columns[0];
    let (_, bounds) = bounds_of(&result, frame);
    assert!((bounds.right() - column.right()).abs() < 0.01);
}

#[test]
fn flow_with_text_frames_dragged_down_hang_below_the_baseline_f18_f20() {
    // F18: dragged down, a Flow with text frame hangs below the baseline;
    // F20: the next line starts below the frame.
    let mut document = plain_text("Before\nNext line");
    let frame = document.create_text_frame(
        Placement::Inline {
            y_offset: 400,
            size: Size {
                width: 1_200,
                height: 1_000,
            },
        },
        FrameStyle::default(),
    );
    document.frame_stack_mut().push(frame);
    let paragraph = document.paragraph_mut(0);
    paragraph.text = format!("Before {OBJECT}");
    paragraph.objects.push(InlineObject::FrameAnchor(frame));
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    assert_eq!(lines[0].height, 600.0 + 400.0, "600 above, 400 below");
    let (_, bounds) = bounds_of(&result, frame);
    assert_eq!(
        (bounds.y, bounds.height),
        (lines[0].baseline - 600.0, 1_000.0)
    );
    assert!(lines[1].top >= bounds.bottom());

    // Raised above the text, it keeps its height and leaves a gap.
    let mut raised = document.clone();
    raised.frame_mut(frame).unwrap().placement = Placement::Inline {
        y_offset: -300,
        size: Size {
            width: 1_200,
            height: 1_000,
        },
    };
    let result = layout(&raised);
    let line = main_lines(&result.pages[0])[0];
    let (_, bounds) = bounds_of(&result, frame);
    assert_eq!(
        (bounds.y, bounds.height),
        (line.baseline - 1_300.0, 1_000.0)
    );
}

#[test]
fn text_wraps_around_beside_or_through_frames_f01_f04_f05_v14() {
    // A frame covering the right part of the column from y 2,000 to 3,000.
    let text = words(400);
    let frame_rect = Rect::new(7_000, 2_000, 10_800, 3_000);
    let lines_for = |wrap: Wrap| {
        let mut document = plain_text(&text);
        fixed_frame(&mut document, 1, frame_rect, wrap);
        let result = layout(&document);
        main_lines(&result.pages[0])
            .into_iter()
            .map(|line| (line.top, line.x, line.slot_width))
            .collect::<Vec<_>>()
    };
    let beside = |line: &(f32, f32, f32)| line.0 + LINE > 2_000.0 && line.0 < 3_000.0;
    // Wrap around: lines beside the frame end at its left edge.
    let around = lines_for(Wrap::Around);
    assert!(
        around
            .iter()
            .filter(|line| beside(line))
            .all(|line| line.2 == 7_000.0 - 1_440.0)
    );
    assert!(
        around
            .iter()
            .filter(|line| !beside(line))
            .all(|line| line.2 == 9_360.0)
    );
    // No wrap around: text runs through.
    assert!(
        lines_for(Wrap::NoWrapAround)
            .iter()
            .all(|line| line.2 == 9_360.0)
    );
    // No wrap beside: nothing beside the frame; text continues below it.
    let none_beside = lines_for(Wrap::NoWrapBeside);
    assert!(none_beside.iter().all(|line| !beside(line)));
    assert!(none_beside.iter().any(|line| line.0 == 3_000.0));
}

#[test]
fn header_text_wraps_around_page_frames_v14() {
    let mut document = plain_text("Body");
    set_header(&mut document, false, &words(20));
    fixed_frame(
        &mut document,
        1,
        Rect::new(8_000, 600, 11_000, 1_200),
        Wrap::Around,
    );
    let result = layout(&document);
    let header = header_lines(&result.pages[0]);
    assert_eq!(header[0].slot_width, 8_000.0 - 1_440.0);
}

#[test]
fn frame_interiors_have_derived_columns_and_corner_insets_f01_f03_c01() {
    let mut document = plain_text("Main");
    let frame = fixed_frame(
        &mut document,
        1,
        Rect::new(2_160, 2_880, 5_040, 5_040),
        Wrap::Around,
    );
    {
        let style = &mut document.frame_mut(frame).unwrap().style;
        style.corners = 18;
        style.interior = LayoutBox {
            margins: Margins::uniform(144),
            columns: 2,
            gutter: 240,
            column_edges: None,
            tabs: vec![Vec::new(), Vec::new()],
            balance: false,
            line_between: None,
        };
    }
    frame_text(&mut document, frame, &words(12));
    let result = layout(&document);
    let frame_box = result.pages[0]
        .frames
        .iter()
        .find(|frame_box| frame_box.frame == frame)
        .unwrap();
    // F01: Ami Pro's integer inset (`1530:0892`), (2,160 − 2 × 10) × 18 × 3
    // / 2000 = 57 twips, F01's stored column edge.
    let inset = frame_box.interior.x - 2_160.0 - 144.0;
    assert_eq!(inset, 57.0, "corner inset");
    let lines: Vec<_> = result.pages[0]
        .lines
        .iter()
        .filter(|line| line.container == Container::Frame(frame))
        .collect();
    assert!(!lines.is_empty());
    let column_width = (frame_box.interior.width - 240.0) / 2.0;
    assert!(lines.iter().all(|line| line.slot_width == column_width));
    assert!(
        lines
            .iter()
            .all(|line| line.top + line.height <= frame_box.interior.bottom() + 0.5)
    );
}

#[test]
fn tables_lay_out_a_grid_with_cell_text_tb1() {
    let mut document = plain_text("Main");
    let cells: Vec<TableCell> = ["A1", "B1", "A2", "B2"]
        .iter()
        .map(|text| {
            let flow = document.create_flow(StyleId(1));
            document
                .flow_mut(flow)
                .unwrap()
                .paragraph_mut(0)
                .unwrap()
                .text = (*text).into();
            TableCell {
                content: CellContent::Text(flow),
                span: CellSpan::SINGLE,
                frame: None,
                format: Default::default(),
                formula: None,
                legacy: LegacyData::default(),
            }
        })
        .collect();
    let frame = document.allocate_frame_id();
    document.insert_frame(Frame {
        id: frame,
        placement: Placement::Fixed {
            page: 1,
            rect: Rect::new(1_440, 4_000, 6_440, 6_000),
        },
        style: FrameStyle {
            interior: LayoutBox::single_column(Margins::uniform(0)),
            ..FrameStyle::default()
        },
        group: None,
        content: FrameContent::Table(Table {
            columns: vec![2_000, 3_000],
            rows: vec![0, 600],
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
    document.frame_stack_mut().push(frame);
    let result = layout(&document);
    let frame_box = result.pages[0]
        .frames
        .iter()
        .find(|frame_box| frame_box.frame == frame)
        .unwrap();
    let FrameBoxKind::Table { cells, .. } = &frame_box.kind else {
        panic!("a table box");
    };
    // A cell is its column's text width and its row's text height plus the
    // cell margins (86 each way; TB1, V20). Row 0 is as tall as its text;
    // row 1 keeps its stored 600.
    assert_eq!(
        cells[0],
        Bounds {
            x: 1_440.0,
            y: 4_000.0,
            width: 2_000.0 + 86.0,
            height: 240.0 + AFTER + 86.0
        }
    );
    assert_eq!(cells[3].x, 1_440.0 + 2_086.0);
    assert_eq!(cells[3].height, 600.0 + 86.0);
    let texts: Vec<_> = result.pages[0]
        .lines
        .iter()
        .filter(|line| matches!(line.container, Container::Cell { frame: f, .. } if f == frame))
        .map(|line| text_of(&document, line))
        .collect();
    assert_eq!(texts, ["A1", "B1", "A2", "B2"]);
}

/// A table frame anchored at the start of main paragraph `paragraph`, with
/// one paragraph of text per cell.
fn anchored_table(
    document: &mut Document,
    paragraph: usize,
    placement: Placement,
    grid: (&[i32], &[i32], CellMargins),
    cells: &[(&str, CellSpan)],
) -> FrameId {
    let (columns, rows, cell_margins) = grid;
    let cells = cells
        .iter()
        .map(|(text, span)| {
            let flow = document.create_flow(StyleId(1));
            document
                .flow_mut(flow)
                .unwrap()
                .paragraph_mut(0)
                .unwrap()
                .text = (*text).into();
            TableCell {
                content: CellContent::Text(flow),
                span: *span,
                frame: None,
                format: Default::default(),
                formula: None,
                legacy: LegacyData::default(),
            }
        })
        .collect();
    let frame = document.allocate_frame_id();
    document.insert_frame(Frame {
        id: frame,
        placement,
        style: FrameStyle {
            wrap: match placement {
                Placement::Inline { .. } => Wrap::Around,
                _ => Wrap::NoWrapAround,
            },
            interior: LayoutBox::single_column(Margins::uniform(0)),
            ..FrameStyle::default()
        },
        group: None,
        content: FrameContent::Table(Table {
            columns: columns.to_vec(),
            rows: rows.to_vec(),
            cell_margins,
            spacing: Default::default(),
            row_options: Default::default(),
            layout: Default::default(),
            read_rows: Default::default(),
            cells,
            legacy: LegacyData::default(),
        }),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().push(frame);
    let target = document.paragraph_mut(paragraph);
    target.text.insert(0, OBJECT);
    target.objects.insert(0, InlineObject::FrameAnchor(frame));
    frame
}

fn table_cells(result: &DocumentLayout, frame: FrameId) -> (Bounds, Vec<Bounds>) {
    let frame_box = result
        .pages
        .iter()
        .flat_map(|page| &page.frames)
        .find(|frame_box| frame_box.frame == frame)
        .unwrap();
    let FrameBoxKind::Table { cells, .. } = &frame_box.kind else {
        panic!("a table box");
    };
    (frame_box.bounds, cells.clone())
}

/// Ami Pro's grid (`CELL_FRAME_NOTES.md`, VM TF14, TF14X, BPS): column c's
/// text starts at the left inset plus each earlier column's width and
/// gutter, whatever the gutter; rows the same from the top inset; a row or
/// column can have a gutter of its own; a cell's box reaches the middle of
/// the gutters beside it.
#[test]
fn tables_lay_out_on_ami_pros_grid_tf14() {
    let lines_in = |result: &DocumentLayout, frame: FrameId, cell: usize| -> Vec<(f32, f32)> {
        result.pages[0]
            .lines
            .iter()
            .filter(|line| line.container == Container::Cell { frame, cell })
            .map(|line| (line.x, line.top))
            .collect()
    };
    let make = |spacing: TableSpacing| {
        let mut document = plain_text("Before\nAfter the table");
        for style in document.styles_mut().values_mut() {
            style.spacing.below = 0;
        }
        let frame = anchored_table(
            &mut document,
            1,
            Placement::BelowParagraph {
                x_offset: 0,
                size: Size {
                    width: 5_659,
                    height: 1_475,
                },
            },
            (
                &[1_440, 1_440, 1_440],
                &[284, 284],
                CellMargins {
                    horizontal: 432,
                    vertical: 432,
                },
            ),
            &[
                ("a", CellSpan::SINGLE),
                ("b", CellSpan::SINGLE),
                ("c", CellSpan::SINGLE),
                ("d", CellSpan::SINGLE),
                ("e", CellSpan::SINGLE),
                ("f", CellSpan::SINGLE),
            ],
        );
        let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
            unreachable!()
        };
        table.spacing = spacing;
        (document, frame)
    };
    // TF14: gutters of 0.30 in; the first column 43 in, the next one after
    // its width and the whole gutter (3355 on the page for a table at 1440).
    let (document, frame) = make(TableSpacing::default());
    let result = layout(&document);
    let (bounds, cells) = table_cells(&result, frame);
    let (left, top) = (bounds.x, bounds.y);
    assert_eq!(lines_in(&result, frame, 0)[0].0, left + 43.0);
    assert_eq!(
        lines_in(&result, frame, 1)[0].0,
        left + 43.0 + 1_440.0 + 432.0
    );
    assert_eq!(lines_in(&result, frame, 0)[0].1, top + 43.0);
    assert_eq!(lines_in(&result, frame, 4)[0].1, top + 43.0 + 284.0 + 432.0);
    assert_eq!(bounds.height, 43.0 + 2.0 * (284.0 + 432.0), "TF14's 1475");
    // The grid lines fall in the middle of the gutters (measured on TF14's
    // screen), the outer ones at the table's edge and half a gutter beyond
    // the last text.
    assert_eq!(cells[0].x, left);
    assert_eq!(cells[0].right(), left + 43.0 + 1_440.0 + 216.0);
    assert_eq!(cells[1].x, cells[0].right());
    assert_eq!(
        cells[2].right(),
        left + 43.0 + 3.0 * 1_440.0 + 2.0 * 432.0 + 216.0
    );

    // TF14X: a top inset of 300 moves every row down, nothing across.
    let (document, frame) = make(TableSpacing {
        top: 300,
        ..TableSpacing::default()
    });
    let result = layout(&document);
    let (bounds, _) = table_cells(&result, frame);
    assert_eq!(
        lines_in(&result, frame, 1)[0],
        (bounds.x + 43.0 + 1_872.0, bounds.y + 300.0)
    );

    // A row and a column with gutters of their own (`[h]`/`[w]` flag 4).
    let mut own = TableSpacing::default();
    own.row_gutters.insert(0, 72);
    own.column_gutters.insert(0, 28);
    let (document, frame) = make(own);
    let result = layout(&document);
    let (bounds, _) = table_cells(&result, frame);
    assert_eq!(
        lines_in(&result, frame, 4)[0],
        (
            bounds.x + 43.0 + 1_440.0 + 28.0,
            bounds.y + 43.0 + 284.0 + 72.0
        )
    );
    assert_eq!(
        lines_in(&result, frame, 5)[0].0,
        bounds.x + 43.0 + 2.0 * 1_440.0 + 28.0 + 432.0
    );
}

/// `CELL_FRAME_NOTES.md`: a frame in a cell covers the cell's text area,
/// joined cells and gutters included (VM TF1, TF13, TF14), right above its
/// table in the page's frames.
#[test]
fn cell_frames_cover_their_cells_text_area_tf1_tf13_tf14() {
    let make = |gutter: i32, spans: [CellSpan; 2]| {
        let mut document = plain_text("Before\nAfter the table");
        for style in document.styles_mut().values_mut() {
            style.spacing.below = 0;
        }
        let frame = anchored_table(
            &mut document,
            1,
            Placement::BelowParagraph {
                x_offset: 0,
                size: Size {
                    width: 9_352,
                    height: 783,
                },
            },
            (
                &[3_017, 3_017],
                &[284],
                CellMargins {
                    horizontal: gutter,
                    vertical: gutter,
                },
            ),
            &[("a", spans[0]), ("", spans[1])],
        );
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
        let FrameContent::Table(table) = &mut document.frame_mut(frame).unwrap().content else {
            unreachable!()
        };
        // The second cell, or the joined one.
        let holder = if spans[1].is_covered() { 0 } else { 1 };
        table.cells[holder].frame = Some(picture);
        let result = layout(&document);
        let frames = &result.pages[0].frames;
        let at = frames
            .iter()
            .position(|frame_box| frame_box.frame == frame)
            .unwrap();
        assert_eq!(frames[at + 1].frame, picture, "right above its table");
        (frames[at].bounds, frames[at + 1].bounds)
    };
    // TF1: the second cell of 3017-twip columns, gutters 86.
    let (table, picture) = make(86, [CellSpan::SINGLE, CellSpan::SINGLE]);
    assert_eq!(
        picture,
        Bounds {
            x: table.x + 43.0 + 3_017.0 + 86.0,
            y: table.y + 43.0,
            width: 3_017.0,
            height: 284.0
        }
    );
    // TF14: a 432-twip gutter, the first column still 43 in.
    let (table, picture) = make(432, [CellSpan::SINGLE, CellSpan::SINGLE]);
    assert_eq!(picture.x, table.x + 43.0 + 3_017.0 + 432.0);
    // TF13: two cells joined: both columns and the gutter between.
    let (table, picture) = make(
        86,
        [
            CellSpan {
                rows: 1,
                columns: 2,
            },
            CellSpan::COVERED,
        ],
    );
    assert_eq!(
        (picture.x, picture.width),
        (table.x + 43.0, 3_017.0 + 86.0 + 3_017.0)
    );
}

/// TB1, V20, and README31: with Ami Pro's default gutters (86) a cell is
/// its column's text width and its row's text height plus 86, and the frame
/// is the cells plus 43 twips. A joined cell takes its whole block, and a
/// table is as tall as its rows need, so text after it starts below it.
#[test]
fn tables_take_cell_margins_joined_cells_and_their_text_height_tb1_readme31() {
    let mut document = plain_text("Before\nAfter the table");
    // Table Text has no space after.
    for style in document.styles_mut().values_mut() {
        style.spacing.below = 0;
    }
    let tb1 = Size {
        width: 9_353,
        height: 783,
    };
    let frame = anchored_table(
        &mut document,
        1,
        Placement::BelowParagraph {
            x_offset: 0,
            size: tb1,
        },
        (&[4_569, 4_569], &[284, 284], CellMargins::default()),
        &[
            ("A1", CellSpan::SINGLE),
            ("B1", CellSpan::SINGLE),
            ("A2", CellSpan::SINGLE),
            ("B2", CellSpan::SINGLE),
        ],
    );
    let result = layout(&document);
    let (bounds, cells) = table_cells(&result, frame);
    // 2 × (284 + 86) + 43: TB1's stored height, so the frame keeps it.
    assert_eq!(bounds.height, 783.0);
    assert_eq!(
        cells[3],
        Bounds {
            x: 1_440.0 + 4_655.0,
            y: bounds.y + 370.0,
            width: 4_655.0,
            height: 370.0
        }
    );
    assert_eq!(main_lines(&result.pages[0])[1].top, bounds.bottom());

    // README31's merge-field table: no vertical margin, and a second row
    // joined over both columns, whose text fits on one line there.
    let joined_cells = |first: &str| {
        let mut document = plain_text("Before\nAfter the table");
        for style in document.styles_mut().values_mut() {
            style.spacing.below = 0;
        }
        let frame = anchored_table(
            &mut document,
            1,
            Placement::BelowParagraph {
                x_offset: 0,
                size: Size {
                    width: 2_215,
                    height: 523,
                },
            },
            (
                &[1_000, 1_000],
                &[240, 240],
                CellMargins {
                    horizontal: 86,
                    vertical: 0,
                },
            ),
            &[
                (first, CellSpan::SINGLE),
                ("y", CellSpan::SINGLE),
                (
                    "wordy wordy wordy",
                    CellSpan {
                        rows: 1,
                        columns: 2,
                    },
                ),
                ("", CellSpan::COVERED),
            ],
        );
        (document, frame)
    };
    let (document, frame) = joined_cells("x");
    let result = layout(&document);
    let (bounds, cells) = table_cells(&result, frame);
    assert_eq!(bounds.height, 523.0, "2 × 240 + 43, as stored");
    assert_eq!(cells[2].width, 2.0 * 1_086.0);
    assert_eq!(
        cells[3], cells[2],
        "a covered cell takes the joined cell's bounds"
    );
    let cell_lines = |cell: usize| {
        result.pages[0]
            .lines
            .iter()
            .filter(|line| line.container == Container::Cell { frame, cell })
            .count()
    };
    assert_eq!((cell_lines(2), cell_lines(3)), (1, 0));

    // Four lines of text in the first cell: the row and the frame grow, and
    // the text after the table moves below it.
    let (document, frame) = joined_cells("wordy wordy wordy wordy");
    let result = layout(&document);
    let (bounds, cells) = table_cells(&result, frame);
    // The first row's box runs from the table's edge: the inset above its
    // text (Ami Pro's grid, TF14X), then its four lines, with no gutter.
    assert_eq!(cells[0].height, 43.0 + 4.0 * LINE);
    assert_eq!(bounds.height, 4.0 * LINE + 240.0 + 43.0);
    assert_eq!(main_lines(&result.pages[0])[1].top, bounds.bottom());

    // A Flow with text table grows its line the same way.
    let mut document = plain_text("Before\nAfter");
    let frame = anchored_table(
        &mut document,
        1,
        Placement::Inline {
            y_offset: 0,
            size: Size {
                width: 1_129,
                height: 200,
            },
        },
        (&[1_000], &[240], CellMargins::default()),
        &[("x", CellSpan::SINGLE)],
    );
    let result = layout(&document);
    let (bounds, _) = table_cells(&result, frame);
    // One line of Body Text (240 and 120 after) with the margins, plus 43.
    assert_eq!(bounds.height, LINE + AFTER + 86.0 + 43.0);
    // The table stands on a line of its own, and the paragraph's text goes
    // on below it (VM TE20).
    let after = main_lines(&result.pages[0])
        .into_iter()
        .find(|line| line.paragraph_index == 1 && !line.words.is_empty() && line.width > 0.0)
        .expect("the text after the table");
    assert!(
        after.top >= bounds.bottom() - 0.5,
        "{} {}",
        after.top,
        bounds.bottom()
    );
}

#[test]
fn display_list_lines_name_their_flow_paragraph_and_bytes() {
    let document = plain_text(&format!("short\n{}", words(30)));
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    assert_eq!(lines[0].flow, document.main_flow_id());
    assert_eq!(lines[0].paragraph, document.paragraphs()[0].id);
    assert_eq!(lines[1].paragraph_index, 1);
    assert_eq!(lines[1].bytes.start, 0);
    assert_eq!(
        lines[2].bytes.start, lines[1].bytes.end,
        "consecutive lines cover the paragraph"
    );
    // 78 characters fit a line: 13 words of five plus their spaces.
    assert_eq!(text_of(&document, lines[1]).trim_end(), words(13));
    assert_eq!(lines[1].words[0].x, 1_440.0);
}

#[test]
fn hard_returns_force_lines_and_soft_hyphens_show_when_used_v19() {
    let document = plain_text(&format!(
        "one\u{2028}two {}co\u{AD}operation",
        "x".repeat(70)
    ));
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    assert_eq!(text_of(&document, lines[0]), "one\u{2028}");
    assert!(
        lines[1].hyphen,
        "the line ends at the soft hyphen with a hyphen"
    );
    assert_eq!(text_of(&document, lines[2]), "operation");
}

#[test]
fn every_flow_id_the_layout_names_exists() {
    let mut document = plain_text(&words(50));
    set_header(&mut document, false, "Header");
    let result = layout(&document);
    let flows: Vec<FlowId> = result
        .pages
        .iter()
        .flat_map(|page| page.lines.iter().map(|line| line.flow))
        .collect();
    assert!(flows.iter().all(|flow| document.flow(*flow).is_some()));
}

#[test]
fn bullets_stand_at_the_left_indent_and_numbers_count_their_runs_bn1_bn3() {
    use amipro_document::Bullets;
    use amipro_layout::BulletPiece;
    // VM fixtures BN1 and BN3 and the decompile (`1200:12de`): the bullet
    // or number stands at the left indent (All), and the first line's text
    // at the first-line indent, or after the bullet when it is wider.
    let mut document = plain_text(&format!("one\ntwo\n{}\nafter\nthree", words(40)));
    let body = document.paragraphs()[0].style;
    let mut numbered = document.styles()[&body].clone();
    numbered.id = StyleId(81);
    numbered.name = "Num".into();
    numbered.bullets = Bullets {
        text: "\u{1A}.".into(),
        ..Bullets::default()
    };
    numbered.indents = Indents {
        all: 1000,
        first: 432,
        rest: 0,
        right: 0,
    };
    let mut tight = numbered.clone();
    tight.id = StyleId(82);
    tight.name = "Tight".into();
    tight.indents.first = 0;
    document.styles_mut().insert(numbered.id, numbered);
    document.styles_mut().insert(tight.id, tight);
    for index in [0, 1, 2, 4] {
        document.paragraph_mut(index).style = StyleId(81);
    }
    document.paragraph_mut(2).style = StyleId(82);
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let number = |line: &LineBox| match &line.bullet.as_ref().unwrap().pieces[..] {
        [BulletPiece::Text { text, .. }] => text.clone(),
        other => panic!("{other:?}"),
    };
    // "1." (240 twips) fits in the first-line indent: the text keeps it.
    let first = lines[0];
    assert_eq!(first.bullet.as_ref().unwrap().x, 1440.0 + 1000.0);
    assert_eq!(first.words[0].x, 1440.0 + 1000.0 + 432.0);
    assert_eq!(number(first), "1.");
    assert_eq!(number(lines[1]), "2.");
    // Another numbered style starts its own run (BN2); without a
    // first-line indent the text follows the number. Only the first line
    // has one.
    let third: Vec<_> = lines
        .iter()
        .filter(|line| line.paragraph_index == 2)
        .collect();
    assert!(third.len() > 1);
    assert_eq!(number(third[0]), "1.");
    assert_eq!(third[0].words[0].x, 1440.0 + 1000.0 + 240.0);
    assert!(third[1].bullet.is_none());
    assert_eq!(third[1].words[0].x, 1440.0 + 1000.0);
    // Body Text breaks the run too: the next starts again at 1.
    let after = lines.iter().find(|line| line.paragraph_index == 3).unwrap();
    assert!(after.bullet.is_none());
    let last = lines.iter().find(|line| line.paragraph_index == 4).unwrap();
    assert_eq!(number(last), "1.");
}

#[test]
fn right_aligned_numbers_end_a_space_before_the_text_1108_4107() {
    use amipro_document::Bullets;
    // `_DEFAULT.STY`'s Number List: 0.25 in indents, "1." with Right align.
    // The number ends one space (120 twips in the fixed shaper) before the
    // first line's text, so "9." and "10." end at the same place; the text
    // does not move. A number wider than the room stays at the left indent.
    let paragraphs: Vec<String> = (1..=10).map(|n| format!("item {n}")).collect();
    let mut document = plain_text(&paragraphs.join("\n"));
    let body = document.paragraphs()[0].style;
    let mut list = document.styles()[&body].clone();
    list.id = StyleId(83);
    list.name = "Number List".into();
    list.bullets = Bullets {
        text: "\u{1A}.".into(),
        flags: 16,
        ..Bullets::default()
    };
    list.indents = Indents {
        all: 360,
        first: 360,
        rest: 360,
        right: 0,
    };
    document.styles_mut().insert(list.id, list.clone());
    for index in 0..10 {
        document.paragraph_mut(index).style = list.id;
    }
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let right = |line: &LineBox| {
        let bullet = line.bullet.as_ref().unwrap();
        bullet.x + bullet.width
    };
    let text_x = 1440.0 + 720.0;
    for line in &lines[..10] {
        assert_eq!(line.words[0].x, text_x, "the text keeps its indent");
    }
    // "9." is 240 wide: it ends 120 before the text.
    assert_eq!(right(lines[8]), text_x - 120.0);
    assert_eq!(lines[8].bullet.as_ref().unwrap().x, text_x - 360.0);
    // "10." (360) plus a space does not fit in 360: it stays at All.
    assert_eq!(lines[9].bullet.as_ref().unwrap().x, 1440.0 + 360.0);
    // Without Right align every number stands at All.
    list.bullets.flags = 0;
    document.styles_mut().insert(list.id, list);
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    assert_eq!(lines[8].bullet.as_ref().unwrap().x, 1440.0 + 360.0);
}

#[test]
fn superscript_bullet_text_is_set_smaller_but_not_asterisks() {
    use amipro_document::Bullets;
    use amipro_layout::BulletPiece;
    // `1200:168b`: superscript bullet text is measured at 7/12 of the size;
    // asterisk and dagger numbers never are.
    let mut document = plain_text("one\ntwo");
    let body = document.paragraphs()[0].style;
    for (index, text) in ["Note", "\u{1F}"].into_iter().enumerate() {
        let mut style = document.styles()[&body].clone();
        style.id = StyleId(90 + index as u64);
        style.name = format!("S{index}");
        style.bullets = Bullets {
            text: text.into(),
            attributes: 256,
            ..Bullets::default()
        };
        document.styles_mut().insert(style.id, style.clone());
        document.paragraph_mut(index).style = style.id;
    }
    let result = layout(&document);
    let lines = main_lines(&result.pages[0]);
    let width = |line: &LineBox| match &line.bullet.as_ref().unwrap().pieces[..] {
        [BulletPiece::Text { width, .. }] => *width,
        other => panic!("{other:?}"),
    };
    let bullet = |line: &LineBox| line.bullet.clone().unwrap();
    // Four letters at 7 pt in the fixed shaper (half the size each).
    assert_eq!(width(lines[0]), 4.0 * 7.0 * 20.0 / 2.0);
    assert!(bullet(lines[0]).superscript);
    // One asterisk at full size.
    assert_eq!(width(lines[1]), 120.0);
    assert!(!bullet(lines[1]).superscript);
}

#[test]
fn hyphenated_styles_split_the_word_that_does_not_fit_within_the_zone() {
    // Decompile `1310:0000`, `10d0:09b9`: with Hyphenation on, the word
    // that does not fit is split where the hyphenator allows (here every
    // second letter, `FixedShaper`) if the line has the document's zone
    // (in average characters, 120 twips here) left before it.
    let text = format!("{}extensiveness end", "abcdefghij ".repeat(6));
    let lines_of = |hyphenation: bool, zone: u16| {
        let mut document = plain_text(&text);
        let id = document.paragraphs()[0].style;
        document.styles_mut().get_mut(&id).unwrap().hyphenation = hyphenation;
        document.set_hyphenation_zone(zone);
        let result = layout(&document);
        main_lines(&result.pages[0])
            .iter()
            .map(|line| (text_of(&document, line).to_owned(), line.hyphen))
            .collect::<Vec<_>>()
    };
    // 66 characters leave 12 (1,440 twips): "extensiven" and a hyphen fit.
    let hyphenated = lines_of(true, 5);
    assert_eq!(
        hyphenated[0].0,
        format!("{}extensiven", "abcdefghij ".repeat(6))
    );
    assert!(hyphenated[0].1);
    assert_eq!(hyphenated[1].0, "ess end");
    // Without Hyphenation, or with a zone wider than the room left, the
    // word moves whole.
    for (hyphenation, zone) in [(false, 5), (true, 13)] {
        let lines = lines_of(hyphenation, zone);
        assert_eq!(lines[0].0, "abcdefghij ".repeat(6), "{hyphenation} {zone}");
        assert!(!lines[0].1);
    }
}

#[test]
fn page_lines_sit_where_ami_pro_draws_them_ap1_to_ap5() {
    use amipro_document::{Border, ColumnLine, LinePosition, Sides};
    // Probes AP1–AP5 (PL.6): two-inch left, top, and right margins, a 4 pt
    // line (style 4, 80 twips), two columns with a one-inch gutter and the
    // line between them. Ami Pro's screen, at 87.4 pixels an inch from the
    // page's left edge: Inside 2.04–2.10 in (its inner edge on the margin),
    // Close to inside 1.48–1.53, Middle 0.97–1.03, Close to outside
    // 0.47–0.53, Outside 0–0.06 (its outer edge on the page's edge); the
    // line between columns centred in the gutter.
    for (position, left) in [
        (LinePosition::Inside, 2_800.0),
        (LinePosition::CloseToInside, 2_120.0),
        (LinePosition::Middle, 1_400.0),
        (LinePosition::CloseToOutside, 680.0),
        (LinePosition::Outside, 0.0),
    ] {
        let mut document = plain_text("Text");
        let geometry = &mut document.document_layout_mut().right;
        geometry.layout.margins.left = 2_880;
        geometry.layout.margins.top = 2_880;
        geometry.layout.margins.right = 2_880;
        geometry.layout.columns = 2;
        geometry.layout.gutter = 1_440;
        geometry.layout.tabs = vec![Vec::new(), Vec::new()];
        geometry.layout.line_between = Some(ColumnLine {
            style: 4,
            colour: 0,
        });
        geometry.border = Some(Border {
            sides: Sides::ALL,
            style: 4,
            colour: 255,
            position,
        });
        let result = layout(&document);
        let page = &result.pages[0];
        let vertical: Vec<_> = page
            .strokes
            .iter()
            .filter(|stroke| stroke.bounds.width == 80.0)
            .map(|stroke| stroke.bounds.x)
            .collect();
        // Left line, right line (mirrored from the right edge), and the
        // line between columns at the gutter's centre, 6,120 twips.
        assert_eq!(
            vertical,
            [left, 12_240.0 - left - 80.0, 6_080.0],
            "{position:?}"
        );
        let top = page
            .strokes
            .iter()
            .find(|stroke| stroke.bounds.height == 80.0)
            .unwrap();
        assert_eq!((top.bounds.y, top.colour), (left, 255), "{position:?}");
        let between = page.strokes.last().unwrap();
        assert_eq!(
            (between.bounds.y, between.bounds.height),
            (2_880.0, 15_840.0 - 2_880.0 - 1_440.0)
        );
    }
}

#[test]
fn a_page_break_balances_columns_and_the_line_between_follows_them() {
    use amipro_document::ColumnLine;
    // SETUP.SAM in Ami Pro (PL.11): two balanced columns with the line
    // between, and a page break after four short paragraphs: page 1's
    // columns hold two lines each, and the line between is as tall.
    let mut document = plain_text("a\nb\nc\nd\ne");
    two_columns(&mut document, true);
    document.document_layout_mut().right.layout.line_between = Some(ColumnLine {
        style: 1,
        colour: 0,
    });
    append_object(&mut document, 3, page_break(None, false));
    let result = layout(&document);
    let page = &result.pages[0];
    let columns = &page.columns;
    let lines = main_lines(page);
    assert_eq!(lines.len(), 4);
    assert_eq!(
        lines.iter().filter(|line| line.x == columns[1].x).count(),
        2,
        "balanced"
    );
    let between = page
        .strokes
        .iter()
        .find(|stroke| stroke.between_columns)
        .unwrap();
    assert_eq!(between.bounds.height, 2.0 * LINE, "two lines a column");
    // Without balance the line runs the columns' height.
    two_columns(&mut document, false);
    document.document_layout_mut().right.layout.line_between = Some(ColumnLine {
        style: 1,
        colour: 0,
    });
    let result = layout(&document);
    let page = &result.pages[0];
    let between = page
        .strokes
        .iter()
        .find(|stroke| stroke.between_columns)
        .unwrap();
    assert_eq!(between.bounds.height, page.columns[0].height);
}

/// Uneven columns lay out at their edges (VM fixture RU2: 1440–7050 and
/// 7290–10800 on Letter), and text fills the narrow column after the wide
/// one.
#[test]
fn uneven_columns_lay_out_at_their_edges_ru2() {
    let mut document = plain_text(&words(2_000));
    let page_box = &mut document.document_layout_mut().right.layout;
    page_box.set_edges(
        vec![
            amipro_document::ColumnEdges {
                left: 0,
                right: 5_610,
            },
            amipro_document::ColumnEdges {
                left: 5_850,
                right: 9_360,
            },
        ],
        9_360,
    );
    let result = layout(&document);
    let page = &result.pages[0];
    let columns: Vec<_> = page
        .columns
        .iter()
        .map(|column| (column.x, column.width))
        .collect();
    assert_eq!(columns, [(1_440.0, 5_610.0), (7_290.0, 3_510.0)]);
    let lines = main_lines(page);
    assert!(
        lines
            .iter()
            .any(|line| line.x == 7_290.0 && line.width <= 3_510.0)
    );
    assert!(
        lines
            .iter()
            .filter(|line| line.x == 1_440.0)
            .all(|line| line.width <= 5_610.0)
    );
}

/// FC7, FC7E, FC7G: a With para above frame beside a Wrap around Where
/// placed frame takes the free part of the column at its height, narrowed
/// so that it and its right shadow end there; beside a No wrap around frame
/// it keeps its place and width.
#[test]
fn with_para_above_frames_narrow_beside_wrap_around_frames_fc7() {
    for (left, wrap, expected) in [
        (3_600, Wrap::Around, (1_440.0, 2_060.0)),
        (2_776, Wrap::Around, (1_440.0, 1_236.0)),
        (2_776, Wrap::NoWrapAround, (2_160.0, 2_880.0)),
    ] {
        let mut document =
            plain_text("Main text paragraph one, long enough to wrap.\nSecond main paragraph.");
        let fixed = fixed_frame(
            &mut document,
            1,
            Rect::new(left, 3_600, left + 2_880, 5_760),
            wrap,
        );
        let anchored = document.create_text_frame(
            Placement::BelowParagraph {
                x_offset: 720,
                size: Size {
                    width: 2_880,
                    height: 2_160,
                },
            },
            FrameStyle {
                wrap: Wrap::NoWrapBeside,
                shadow: Shadow {
                    right: 100,
                    bottom: 100,
                    ..Shadow::default()
                },
                ..FrameStyle::default()
            },
        );
        document.frame_stack_mut().push(anchored);
        append_object(&mut document, 1, InlineObject::FrameAnchor(anchored));
        let result = layout(&document);
        let (_, bounds) = bounds_of(&result, anchored);
        assert_eq!((bounds.x, bounds.width), expected, "{left} {wrap:?}");
        let (_, other) = bounds_of(&result, fixed);
        assert_eq!(other.x, left as f32);
    }
}

/// WB1–WB3: beside a Wrap around frame in the middle of the column, the
/// text fills the part left of it down the frame's band, then the part
/// right of it (past its shadow) from the band's top, then goes on across
/// the column below the lowest line beside the frame.
#[test]
fn text_fills_each_side_of_a_wrap_around_frame_in_turn_wb1() {
    let words = "wordy ".repeat(400);
    let mut document = Document::from_plain_text(&format!("{}\n{words}", "line ".repeat(135)));
    let body = document.body_text_style();
    document.styles_mut().get_mut(&body).unwrap().spacing = Spacing::default();
    let frame = fixed_frame(
        &mut document,
        1,
        Rect::new(3_600, 3_600, 6_480, 5_760),
        Wrap::Around,
    );
    document.frame_mut(frame).unwrap().style.shadow = Shadow {
        right: 100,
        bottom: 100,
        ..Shadow::default()
    };
    let result = layout(&document);
    let lines: Vec<&LineBox> = main_lines(&result.pages[0])
        .into_iter()
        .filter(|line| line.paragraph_index == 1)
        .collect();
    // The first paragraph's nine lines end at 3,600: the band starts there
    // and ends at the shadow's bottom, 5,860, so ten lines start in it.
    let tops: Vec<f32> = (0..10).map(|line| 3_600.0 + 240.0 * line as f32).collect();
    let left: Vec<_> = lines[..10].iter().map(|line| (line.x, line.top)).collect();
    assert_eq!(
        left,
        tops.iter().map(|top| (1_440.0, *top)).collect::<Vec<_>>()
    );
    assert!(lines[..10].iter().all(|line| line.slot_width == 2_160.0));
    let right: Vec<_> = lines[10..20]
        .iter()
        .map(|line| (line.x, line.top))
        .collect();
    assert_eq!(
        right,
        tops.iter().map(|top| (6_580.0, *top)).collect::<Vec<_>>()
    );
    assert!(lines[10..20].iter().all(|line| line.slot_width == 4_220.0));
    assert_eq!((lines[20].x, lines[20].top), (1_440.0, 6_000.0));
    assert_eq!(lines[20].slot_width, 9_360.0);
}
