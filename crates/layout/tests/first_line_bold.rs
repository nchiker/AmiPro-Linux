//! Modify Style's "1st line bold" (`docs/research/FIRST_LINE_BOLD_NOTES.md`,
//! VM fixtures FLB0 to FLB2): a paragraph's first laid-out line is set in
//! bold and broken with the bold widths; the later lines are regular.
//!
//! [`WideBold`] is the deterministic [`FixedShaper`] with bold text 1.5 times
//! as wide: 12 pt Body Text sets 120 twips a character, and 180 in bold. The
//! default page text area is 9,360 twips wide.

use amipro_document::{
    CharacterFormat, CharacterRun, Document, LineHeights, ParagraphStyle, StyleId,
};
use amipro_editing::{Command, TextPosition, apply};
use amipro_layout::{
    Container, DocumentLayout, FixedShaper, IncrementalLayout, LineBox, ShapeCache, ShapeRequest,
    ShapedParagraph, Shaper, TextExtent, layout_document, shape_with_first_line_bold,
};

struct WideBold;

impl Shaper for WideBold {
    fn shape(&mut self, request: ShapeRequest<'_>) -> ShapedParagraph {
        shape_with_first_line_bold(request, |request| {
            let mut shaped = FixedShaper.shape_plain(request);
            if request.style.bold {
                for word in &mut shaped.words {
                    word.width *= 1.5;
                }
            }
            shaped
        })
    }

    fn measure(&mut self, text: &str, style: &ParagraphStyle) -> f32 {
        FixedShaper.measure(text, style)
    }

    fn measure_text(
        &mut self,
        text: &str,
        style: &ParagraphStyle,
        format: &CharacterFormat,
        line_heights: &LineHeights,
    ) -> TextExtent {
        FixedShaper.measure_text(text, style, format, line_heights)
    }
}

fn layout(document: &Document) -> DocumentLayout {
    document.validate().unwrap();
    layout_document(document, &mut WideBold, &mut ShapeCache::default())
}

fn lines(layout: &DocumentLayout) -> Vec<&LineBox> {
    layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.container == Container::Main)
        .collect()
}

/// A document of one Body Text paragraph, the style 1st line bold or not.
fn document(text: &str, first_line_bold: bool) -> Document {
    let mut document = Document::from_plain_text(text);
    let body = document.body_text_style();
    document
        .styles_mut()
        .get_mut(&body)
        .unwrap()
        .first_line_bold = first_line_bold;
    document
}

/// "wordy wordy …" of `count` words.
fn words(count: usize) -> String {
    vec!["wordy"; count].join(" ")
}

/// The shown characters of a line, without the blanks that hang at its end.
fn shown(document: &Document, line: &LineBox) -> usize {
    document.paragraphs()[line.paragraph_index].text[line.bytes.clone()]
        .trim_end()
        .chars()
        .count()
}

#[test]
fn the_first_line_breaks_with_bold_widths_and_the_rest_are_regular() {
    let text = words(60);
    let plain = document(&text, false);
    let bold = document(&text, true);
    let (plain_layout, bold_layout) = (layout(&plain), layout(&bold));
    let (plain_lines, bold_lines) = (lines(&plain_layout), lines(&bold_layout));
    // Regular: 12 words and a blank fill 77 characters of 120 twips.
    assert_eq!(shown(&plain, plain_lines[0]), 77);
    assert_eq!(plain_lines[0].width, 77.0 * 120.0);
    // Bold: 180 twips a character in 9,360 leaves 52 characters, eight words
    // and the blank after (the ninth would end at 53).
    assert_eq!(shown(&bold, bold_lines[0]), 47);
    assert_eq!(bold_lines[0].width, 47.0 * 180.0);
    assert!(bold_lines[0].bytes.end < plain_lines[0].bytes.end);
    // The later lines are regular.
    for line in &bold_lines[1..bold_lines.len() - 1] {
        assert_eq!(shown(&bold, line), 77, "{line:?}");
        assert_eq!(line.width, 77.0 * 120.0);
    }
    assert!(bold_lines.len() >= plain_lines.len());
}

#[test]
fn without_the_style_flag_nothing_is_bold() {
    let text = words(60);
    let off = document(&text, false);
    let result = layout(&off);
    let all = lines(&result);
    assert!(all.iter().all(|line| line.width <= 77.0 * 120.0));
    assert_eq!(all[0].width, 77.0 * 120.0);
}

#[test]
fn a_one_line_paragraph_is_wholly_bold() {
    let bold = document("Short second paragraph.", true);
    let result = layout(&bold);
    let all = lines(&result);
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].width, 23.0 * 180.0);
}

#[test]
fn a_forced_line_break_ends_the_first_line_and_the_next_is_regular() {
    let bold = document("First segment\u{2028}Second segment after a break", true);
    let result = layout(&bold);
    let all = lines(&result);
    assert_eq!(all.len(), 2);
    // "First segment" and the break (no width).
    assert_eq!(all[0].width, 13.0 * 180.0);
    assert_eq!(all[1].width, 28.0 * 120.0, "regular");
}

#[test]
fn a_paragraph_continued_on_the_next_page_is_not_bold_there() {
    let text = words(1200);
    let bold = document(&text, true);
    let result = layout(&bold);
    assert!(result.pages.len() > 1);
    let first_on_page_two = result.pages[1]
        .lines
        .iter()
        .find(|line| line.container == Container::Main)
        .unwrap();
    assert!(first_on_page_two.bytes.start > 0);
    assert_eq!(shown(&bold, first_on_page_two), 77);
    assert_eq!(first_on_page_two.width, 77.0 * 120.0);
    let first = lines(&result)[0];
    assert_eq!(first.width, 47.0 * 180.0);
}

#[test]
fn italic_and_direct_bold_toggles_in_the_first_line_change_nothing_in_its_breaks() {
    let text = words(60);
    let mut toggled = document(&text, true);
    toggled.paragraph_mut(0).runs = vec![
        CharacterRun {
            start: 6,
            end: 17,
            format: CharacterFormat {
                bold: Some(false),
                italic: Some(true),
                ..CharacterFormat::default()
            },
        },
        CharacterRun {
            start: 18,
            end: 23,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        },
    ];
    let (plain, with_runs) = (document(&text, true), toggled);
    let (a, b) = (layout(&plain), layout(&with_runs));
    let strip = |lines: Vec<&LineBox>| -> Vec<_> {
        lines
            .into_iter()
            .map(|line| (line.bytes.clone(), line.width, line.height))
            .collect()
    };
    assert_eq!(strip(lines(&a)), strip(lines(&b)));
}

#[test]
fn the_flag_is_part_of_the_shape_caches_key_and_incremental_layout_follows_it() {
    let text = words(60);
    let mut document = document(&text, false);
    document.push_paragraph("Another paragraph below it.", StyleId(1));
    let mut cache = ShapeCache::default();
    let before = layout_document(&document, &mut WideBold, &mut cache);
    let mut incremental = IncrementalLayout::new();
    let first = incremental.layout(&document, &mut WideBold).clone();
    assert_eq!(before.pages, first.pages);
    let body = document.body_text_style();
    document
        .styles_mut()
        .get_mut(&body)
        .unwrap()
        .first_line_bold = true;
    let after = layout_document(&document, &mut WideBold, &mut cache);
    assert_ne!(
        before.pages, after.pages,
        "the cache does not keep the old shaping"
    );
    let incremental_after = incremental.layout(&document, &mut WideBold).clone();
    assert_eq!(after.pages, incremental_after.pages);
    assert_eq!(
        after.pages,
        layout(&document).pages,
        "equal to a fresh layout"
    );
    // An edit in the first paragraph re-breaks it; the result stays fresh.
    let position = TextPosition {
        flow: document.main_flow_id(),
        paragraph: 0,
        byte: 0,
    };
    let document = apply(&document, &Command::insert(position, "Start of it "))
        .unwrap()
        .document;
    let edited = incremental.layout(&document, &mut WideBold).clone();
    assert_eq!(edited.pages, layout(&document).pages);
}
