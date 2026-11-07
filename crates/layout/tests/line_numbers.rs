//! Line Numbering drawn as Ami Pro 3.1 draws it (VM fixtures LN2–LN20,
//! `docs/research/COMMANDS_NOTES.md` §8.5), with the deterministic
//! [`FixedShaper`]: 12 pt Body Text on 240-twip lines, 120-twip characters;
//! Letter with 1-inch margins, so a page holds 54 lines from y = 1,440.

use std::sync::Arc;

use amipro_document::{Document, LineNumbering, LineNumberingEvery, LineNumberingMode, StyleId};
use amipro_layout::{FixedShaper, LineNumberBox, ShapeCache, layout_document, line_numbers};

fn lines(count: usize) -> Document {
    let text: Vec<String> = (1..=count).map(|index| format!("p{index}")).collect();
    Document::from_plain_text(&text.join("\n"))
}

fn numbered(
    document: &mut Document,
    mode: LineNumberingMode,
    every: LineNumberingEvery,
    reset: bool,
) {
    let numbering = document.line_numbering().with(mode, every, reset);
    document.set_line_numbering(numbering);
}

fn numbers(document: &Document) -> Vec<Vec<LineNumberBox>> {
    document.validate().unwrap();
    let layout = layout_document(document, &mut FixedShaper, &mut ShapeCache::default());
    assert_eq!(
        layout.line_numbers,
        line_numbers(document, &layout, &mut FixedShaper)
    );
    let mut pages = layout.line_numbers;
    pages.resize(layout.pages.len(), Vec::new());
    pages
}

fn texts(page: &[LineNumberBox]) -> Vec<u32> {
    page.iter()
        .map(|number| number.text.parse().unwrap())
        .collect()
}

#[test]
fn off_draws_nothing_ln1() {
    let document = lines(10);
    assert!(numbers(&document).iter().all(Vec::is_empty));
}

#[test]
fn text_lines_count_on_across_pages_in_the_left_margin_ln3() {
    let mut document = lines(60);
    numbered(
        &mut document,
        LineNumberingMode::TextLines,
        LineNumberingEvery::Line,
        false,
    );
    let pages = numbers(&document);
    assert_eq!(texts(&pages[0]), (1..=54).collect::<Vec<_>>());
    assert_eq!(texts(&pages[1]), (55..=60).collect::<Vec<_>>());
    // "1 " ends at the left margin: 1,440 less a space and a digit.
    assert_eq!(pages[0][0].x, 1_440.0 - 120.0 - 120.0);
    assert_eq!(pages[0][9].x, 1_440.0 - 120.0 - 240.0, "two digits");
    assert_eq!(pages[0][0].baseline, 1_440.0 + 192.0);
    assert_eq!(pages[0][1].baseline - pages[0][0].baseline, 240.0);
    assert_eq!(pages[0][0].style, document.body_text_style());
}

#[test]
fn empty_paragraphs_are_not_counted_ln9() {
    let mut document = Document::from_plain_text("one\n\n\ntwo\n \nthree");
    numbered(
        &mut document,
        LineNumberingMode::TextLines,
        LineNumberingEvery::Line,
        false,
    );
    let page = &numbers(&document)[0];
    assert_eq!(texts(page), [1, 2, 3]);
    // Beside one, two, and three: lines 1, 4, and 6.
    let baselines: Vec<f32> = page.iter().map(|number| number.baseline).collect();
    assert_eq!(baselines, [1_632.0, 1_632.0 + 720.0, 1_632.0 + 1_200.0]);
}

#[test]
fn every_other_fifth_n_and_reset_ln4_to_ln8() {
    let mut document = lines(60);
    for (every, interval, first, second) in [
        (
            LineNumberingEvery::OtherLine,
            1,
            vec![1, 3, 5],
            vec![55, 57, 59],
        ),
        (
            LineNumberingEvery::FifthLine,
            1,
            vec![5, 10, 15],
            vec![55, 60],
        ),
        (LineNumberingEvery::Lines, 7, vec![7, 14, 21], vec![56]),
    ] {
        let numbering = LineNumbering {
            interval,
            ..document
                .line_numbering()
                .with(LineNumberingMode::TextLines, every, false)
        };
        document.set_line_numbering(numbering);
        let pages = numbers(&document);
        assert_eq!(&texts(&pages[0])[..first.len()], first, "{every:?}");
        assert_eq!(texts(&pages[1]), second, "{every:?}");
    }
    numbered(
        &mut document,
        LineNumberingMode::TextLines,
        LineNumberingEvery::Line,
        true,
    );
    assert_eq!(texts(&numbers(&document)[1]), [1, 2, 3, 4, 5, 6], "reset");
}

#[test]
fn all_lines_number_every_slot_to_the_bottom_margin_ln2_ln8() {
    let mut document = lines(60);
    numbered(
        &mut document,
        LineNumberingMode::AllLines,
        LineNumberingEvery::Line,
        false,
    );
    let pages = numbers(&document);
    assert_eq!(texts(&pages[0]), (1..=54).collect::<Vec<_>>());
    // Page 2 has six lines of text and 54 slots.
    assert_eq!(texts(&pages[1]), (55..=108).collect::<Vec<_>>());
    assert_eq!(pages[1][53].baseline, 1_440.0 + 53.0 * 240.0 + 192.0);
    numbered(
        &mut document,
        LineNumberingMode::AllLines,
        LineNumberingEvery::Line,
        true,
    );
    assert_eq!(texts(&numbers(&document)[1]).last(), Some(&54));
}

#[test]
fn all_lines_take_the_based_on_styles_pitch_and_font_ln13_ln14() {
    let mut document = lines(3);
    let body = document.body_text_style();
    let mut large = document.styles()[&body].clone();
    large.id = StyleId(900);
    large.name = "Large".into();
    large.font_size_points = 24.0;
    large.spacing.above = 288;
    large.spacing.below = 144;
    document.styles_mut().insert(large.id, large);
    let numbering = LineNumbering {
        style: "Large".into(),
        ..document.line_numbering().with(
            LineNumberingMode::AllLines,
            LineNumberingEvery::Line,
            false,
        )
    };
    document.set_line_numbering(numbering.clone());
    let page = &numbers(&document)[0];
    // 480-twip slots (no space above or below): 27 in 12,960 twips.
    assert_eq!(page.len(), 27);
    assert_eq!(page[1].baseline - page[0].baseline, 480.0);
    assert!(page.iter().all(|number| number.style == StyleId(900)));
    // "1 " in 24 pt: 240-twip characters.
    assert_eq!(page[0].x, 1_440.0 - 480.0);
    // Text lines ignore the stored style.
    document.set_line_numbering(LineNumbering {
        flags: LineNumbering::TEXT_LINES | LineNumbering::EVERY_LINE,
        ..numbering
    });
    let page = &numbers(&document)[0];
    assert_eq!(texts(page), [1, 2, 3]);
    assert!(page.iter().all(|number| number.style == body));
}

#[test]
fn only_the_first_column_counts_and_each_page_has_its_own_margin_ln15_ln20() {
    let mut document = lines(120);
    {
        let layout = Arc::make_mut(&mut document.layouts_mut()[0]);
        let mut left = layout.right.clone();
        layout.right.layout.columns = 2;
        let tabs = layout.right.layout.tabs[0].clone();
        layout.right.layout.tabs = vec![tabs.clone(), tabs];
        left.layout.margins.left = 2_880;
        layout.left = Some(left);
    }
    numbered(
        &mut document,
        LineNumberingMode::TextLines,
        LineNumberingEvery::Line,
        false,
    );
    let pages = numbers(&document);
    // Page 1: two columns of 54 lines; only the first is numbered.
    assert_eq!(texts(&pages[0]), (1..=54).collect::<Vec<_>>());
    // Page 2, a left page in one column with a 2-inch margin, goes on at 55.
    assert_eq!(pages[1][0].text, "55");
    assert_eq!(pages[1][0].x, 2_880.0 - 120.0 - 240.0);
}
