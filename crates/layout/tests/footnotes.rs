//! Footnotes and endnotes laid out as Ami Pro lays them out (VM fixtures
//! RNF1–RNF5, RNF71S–RNF74S; `docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`),
//! with the deterministic [`FixedShaper`]: 12 pt Body Text on 240-twip
//! lines, no space after; Letter with 1-inch margins, so the text area is
//! 12,960 twips high from y = 1,440 and holds 54 lines.

use amipro_document::{Document, FlowId, FootnoteOptions, InlineObject, StyleId};
use amipro_layout::{
    Container, DocumentLayout, FixedShaper, LineBox, PageBox, ShapeCache, layout_document,
};

const LINE: f32 = 240.0;
const TOP: f32 = 1_440.0;
const BOTTOM: f32 = 14_400.0;

fn lines(count: usize) -> Document {
    let text: Vec<String> = (0..count).map(|index| format!("p{index}")).collect();
    Document::from_plain_text(&text.join("\n"))
}

/// Adds a footnote of `lines` one-line paragraphs at the end of paragraph
/// `index`.
fn footnote(document: &mut Document, index: usize, lines: usize) -> FlowId {
    let style: StyleId = document.body_text_style();
    let flow = document.create_flow(style);
    let paragraphs: Vec<_> = (0..lines)
        .map(|line| std::sync::Arc::new(document.new_paragraph(format!("f{line}"), style)))
        .collect();
    document.flow_mut(flow).unwrap().paragraphs = paragraphs;
    let paragraph = document.paragraph_mut(index);
    paragraph.text.push('\u{FFFC}');
    paragraph.objects.push(InlineObject::Footnote { flow });
    flow
}

fn layout(document: &Document) -> DocumentLayout {
    document.validate().unwrap();
    layout_document(document, &mut FixedShaper, &mut ShapeCache::default())
}

fn of(page: &PageBox, container: impl Fn(&Container) -> bool) -> Vec<&LineBox> {
    page.lines
        .iter()
        .filter(|line| container(&line.container))
        .collect()
}

fn main(page: &PageBox) -> Vec<&LineBox> {
    of(page, |container| *container == Container::Main)
}

fn notes(page: &PageBox) -> Vec<&LineBox> {
    of(page, |container| {
        matches!(container, Container::Footnote(_))
    })
}

#[test]
fn a_footnote_stands_at_the_page_foot_under_its_separator_rnf1() {
    let mut document = lines(10);
    let flow = footnote(&mut document, 2, 1);
    let laid = layout(&document);
    let page = &laid.pages[0];
    assert_eq!(main(page).len(), 10);
    let foot = notes(page);
    assert_eq!(foot.len(), 1);
    assert_eq!(foot[0].container, Container::Footnote(flow));
    assert_eq!(foot[0].top, BOTTOM - LINE);
    let separator = page.footnote_separator.unwrap();
    assert_eq!(
        (separator.y, separator.x, separator.width),
        (BOTTOM - LINE - 100.0, TOP, 9_360.0)
    );
    // The reference shows the footnote's number.
    let reference = main(page)[2]
        .words
        .iter()
        .find(|word| word.object.is_some())
        .unwrap();
    assert_eq!(reference.text.as_deref(), Some("1"));
}

#[test]
fn the_referring_line_moves_when_the_separator_and_a_line_do_not_fit_rnf72() {
    // Paragraph 51 ends at 13,920: the separator and a line (340) fit
    // above 14,400, and the column ends at 14,060 (RNF71).
    let mut document = lines(70);
    footnote(&mut document, 51, 3);
    let laid = layout(&document);
    assert_eq!(main(&laid.pages[0]).len(), 52);
    assert_eq!(notes(&laid.pages[0]).len(), 1);
    assert_eq!(notes(&laid.pages[1]).len(), 2);
    // Paragraph 53 ends at the text area's bottom: its footnote cannot
    // follow it, so it goes to page 2 with all its lines (RNF72–RNF74).
    let mut document = lines(70);
    footnote(&mut document, 53, 3);
    let laid = layout(&document);
    assert_eq!(main(&laid.pages[0]).len(), 53);
    assert!(notes(&laid.pages[0]).is_empty());
    assert_eq!(main(&laid.pages[1])[0].paragraph_index, 53);
    assert_eq!(notes(&laid.pages[1]).len(), 3);
}

#[test]
fn a_long_footnote_continues_with_ami_pros_messages_rnf5() {
    let mut document = lines(30);
    footnote(&mut document, 0, 80);
    let mut options = FootnoteOptions {
        continued_on: "Continued".into(),
        continued_from: "From before".into(),
        ..FootnoteOptions::default()
    };
    options.set(FootnoteOptions::CONTINUED_ON, true);
    options.set(FootnoteOptions::CONTINUED_FROM, true);
    options.set(FootnoteOptions::CONTINUED_ON_LEFT, true);
    document.set_footnote_options(options);
    let laid = layout(&document);
    let first = notes(&laid.pages[0]);
    let second = notes(&laid.pages[1]);
    // Every line of the footnote is shown once, and each page's area ends
    // with a message or the footnote.
    let message = |page: &[&LineBox], text: &str| {
        page.iter().any(|line| {
            line.paragraph_index == usize::MAX
                && line.words.first().and_then(|word| word.text.as_deref()) == Some(text)
        })
    };
    assert!(message(&first, "Continued"));
    assert!(message(&second, "From before"));
    let shown = |page: &[&LineBox]| {
        page.iter()
            .filter(|line| line.paragraph_index != usize::MAX)
            .count()
    };
    assert_eq!(shown(&first) + shown(&second), 80);
    assert_eq!(main(&laid.pages[0]).len(), 1);
    assert!(main(&laid.pages[1]).len() > 1);
    // The areas stay within the text area.
    for page in &laid.pages {
        for line in notes(page) {
            assert!(line.top + line.height <= BOTTOM + 0.5);
        }
    }
}

#[test]
fn endnotes_follow_the_text_on_a_new_page_without_a_separator_rnf2() {
    let mut document = lines(3);
    footnote(&mut document, 0, 2);
    footnote(&mut document, 1, 1);
    let mut options = FootnoteOptions::default();
    options.set(FootnoteOptions::ENDNOTES, true);
    document.set_footnote_options(options);
    let laid = layout(&document);
    assert_eq!(laid.pages.len(), 2);
    assert!(notes(&laid.pages[0]).is_empty());
    assert!(laid.pages[0].footnote_separator.is_none());
    let end = notes(&laid.pages[1]);
    assert_eq!(end.len(), 3);
    assert_eq!(end[0].top, TOP);
    assert!(laid.pages[1].footnote_separator.is_none());
}

#[test]
fn numbers_start_from_the_starting_number_and_may_reset_each_page_rnf3() {
    let number = |document: &Document, laid: &DocumentLayout, page: usize, index: usize| {
        let line = main(&laid.pages[page])
            .into_iter()
            .find(|line| line.paragraph_index == index)
            .unwrap();
        let _ = document;
        line.words
            .iter()
            .find(|word| word.object.is_some())
            .and_then(|word| word.text.clone())
            .unwrap()
    };
    let mut document = lines(80);
    footnote(&mut document, 1, 1);
    footnote(&mut document, 60, 1);
    let mut options = FootnoteOptions {
        start: 3,
        ..FootnoteOptions::default()
    };
    document.set_footnote_options(options.clone());
    let laid = layout(&document);
    assert_eq!(number(&document, &laid, 0, 1), "3");
    assert_eq!(number(&document, &laid, 1, 60), "4");
    options.set(FootnoteOptions::RESET_EACH_PAGE, true);
    document.set_footnote_options(options);
    let laid = layout(&document);
    assert_eq!(number(&document, &laid, 1, 60), "3");
}

#[test]
fn a_member_of_a_master_document_numbers_its_pages_and_footnotes_after_those_before_md3() {
    use amipro_document::contents::{ContentsSettings, Sequence};
    let mut document = lines(80);
    footnote(&mut document, 1, 1);
    footnote(&mut document, 60, 1);
    // Its own starting number gives way to the carried footnotes (1058:0000).
    document.set_footnote_options(FootnoteOptions {
        start: 7,
        ..FootnoteOptions::default()
    });
    document.set_contents(ContentsSettings {
        sequence: Some(Sequence {
            page_base: 4,
            footnotes: 4,
            ..Sequence::default()
        }),
        ..ContentsSettings::default()
    });
    let laid = layout(&document);
    let shown: Vec<u32> = laid.pages.iter().map(|page| page.shown).collect();
    assert_eq!(shown, [5, 6]);
    let reference = |page: usize, index: usize| {
        main(&laid.pages[page])
            .into_iter()
            .find(|line| line.paragraph_index == index)
            .and_then(|line| line.words.iter().find(|word| word.object.is_some()))
            .and_then(|word| word.text.clone())
            .unwrap()
    };
    assert_eq!(reference(0, 1), "5");
    assert_eq!(reference(1, 60), "6");
    // What the member passes on: its last page shown, its footnotes added.
    let next = amipro_layout::master::next_sequence(&document, &laid, &[]);
    assert_eq!((next.page_base, next.footnotes), (6, 6));
}
