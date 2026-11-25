//! Selecting a long document through the window's mouse (`UI_SPEC.md`
//! §47): one drag from page 1 across the page break to page 2's text and on
//! past the window's edge, Copy, and Shift+click to extend and shrink what
//! is selected.

use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::mouse::{Button, Event as Mouse};
use cosmic::iced::{Event, Point};

use super::*;

fn moved(to: Point) -> (Event, Point) {
    (Event::Mouse(Mouse::CursorMoved { position: to }), to)
}

fn pressed(at: Point) -> (Event, Point) {
    (Event::Mouse(Mouse::ButtonPressed(Button::Left)), at)
}

fn released(at: Point) -> (Event, Point) {
    (Event::Mouse(Mouse::ButtonReleased(Button::Left)), at)
}

fn redrawn(at: Point) -> (Event, Point) {
    (
        Event::Window(cosmic::iced::window::Event::RedrawRequested(
            std::time::Instant::now(),
        )),
        at,
    )
}

/// The main text from the selection's start to its end, taken from the
/// document itself.
fn expected_selection(s: &Scenario) -> String {
    let (start, end) = s.app().page_selection.ordered();
    let main = s.main_flow();
    assert_eq!((start.flow, end.flow), (main, main));
    let paragraphs = &s.document().main_flow().paragraphs;
    if start.paragraph == end.paragraph {
        return paragraphs[start.paragraph].text[start.byte..end.byte].to_owned();
    }
    let mut parts = vec![paragraphs[start.paragraph].text[start.byte..].to_owned()];
    parts.extend(
        paragraphs[start.paragraph + 1..end.paragraph]
            .iter()
            .map(|paragraph| paragraph.text.to_string()),
    );
    parts.push(paragraphs[end.paragraph].text[..end.byte].to_owned());
    parts.join("\n")
}

fn copy_and_compare(s: &mut Scenario, what: &str) -> String {
    let expected = expected_selection(s);
    s.no_change(what, |rt| {
        rt.clipboard = None;
        rt.send(Message::Command(CommandId::Copy));
    });
    assert_eq!(s.rt.clipboard.as_deref(), Some(expected.as_str()), "{what}");
    expected
}

/// A Shift+click at `at`: the modifier as the window reports it, the click,
/// and the key let go.
fn shift_click(s: &mut Scenario, what: &str, at: Point) {
    s.no_change(what, move |rt| {
        let modifiers = |modifiers| {
            (
                Event::Keyboard(cosmic::iced::keyboard::Event::ModifiersChanged(modifiers)),
                at,
            )
        };
        rt.events(&[modifiers(Modifiers::SHIFT), moved(at)]);
        rt.send(Message::Modifiers(Modifiers::SHIFT));
        rt.events(&[pressed(at), released(at), redrawn(at)]);
        rt.events(&[modifiers(Modifiers::empty())]);
        rt.send(Message::Modifiers(Modifiers::empty()));
    });
}

/// A point of the main text that is on the screen now, about `down` of the
/// way down the workspace: the middle of a line, and that line's paragraph.
fn visible_text(s: &mut Scenario, down: f32) -> (Point, usize) {
    let main = s.main_flow();
    let lines: Vec<(usize, f32, f32, usize)> = s
        .app()
        .pages
        .layout()
        .pages
        .iter()
        .enumerate()
        .flat_map(|(page, content)| {
            content
                .lines
                .iter()
                .filter(|line| line.flow == main)
                .map(move |line| {
                    (
                        page,
                        line.x + line.width * 0.5,
                        line.top + line.height / 2.0,
                        line.paragraph_index,
                    )
                })
        })
        .collect();
    let (bounds, _) = s.rt.workspace_view();
    let wanted = bounds.y + bounds.height * down;
    lines
        .into_iter()
        .map(|(page, x, y, paragraph)| (page_point(&mut s.rt, page, x, y), paragraph))
        .min_by(|a, b| (a.0.y - wanted).abs().total_cmp(&(b.0.y - wanted).abs()))
        .unwrap()
}

#[test]
fn a_drag_across_pages_and_past_the_window_is_copied_and_shift_clicked() {
    let text: Vec<String> = (0..420)
        .map(|index| format!("Paragraph {index} of a long report, with a few more words."))
        .collect();
    let mut s = Scenario::new(
        "selecting-pages",
        Document::from_plain_text(&text.join("\n")),
    );
    s.clean_folder();
    s.act("zoom 25%: the end of page 1 and page 2 are in view", |rt| {
        rt.send(Message::ZoomTo(25));
        rt.redraw();
    });
    assert!(s.app().pages.layout().pages.len() >= 6);
    let main = s.main_flow();

    // One gesture: from page 1's text over its footer, the gap and page 2's
    // header to page 2's text, then below the window, held there while the
    // timer scrolls, and released.
    let start = page_point(&mut s.rt, 0, 3_000.0, 12_000.0);
    let path = [
        page_point(&mut s.rt, 0, 3_000.0, 15_200.0),
        page_point(&mut s.rt, 1, 3_000.0, 300.0),
        page_point(&mut s.rt, 1, 3_000.0, 2_400.0),
    ];
    let (bounds, _) = s.rt.workspace_view();
    assert!(path[2].y < bounds.y + bounds.height);
    let below = Point::new(start.x, bounds.y + bounds.height + 12.0);
    assert!(below.y < s.rt.window.height);
    let mut anchor = None;
    let mut on_page_two = 0;
    s.no_change("drag from page 1 to page 2, hold below, release", |rt| {
        rt.events(&[moved(start), pressed(start)]);
        let first = rt.app.page_selection.anchor;
        anchor = Some(first);
        for point in path {
            rt.events(&[moved(point)]);
            assert_eq!(rt.app.page_selection.anchor, first, "at {point:?}");
            assert_eq!(rt.app.page_selection.focus.flow, first.flow);
        }
        assert_eq!(rt.app.page_selection.page, 2);
        on_page_two = rt.app.page_selection.focus.paragraph;
        let before = rt.workspace_scroll();
        rt.events(&[moved(below)]);
        assert!(rt.app.workspace.drag_edge > 0.0);
        for _ in 0..50 {
            rt.send(Message::DragScrollTick);
            rt.events(&[redrawn(below)]);
        }
        assert!(rt.workspace_scroll() > before + 100.0);
        rt.events(&[released(below)]);
        assert_eq!(rt.app.workspace.drag_edge, 0.0);
    });
    let anchor = anchor.unwrap();
    let selection = s.app().page_selection;
    assert_eq!(selection.anchor, anchor);
    assert_eq!(anchor.flow, main);
    assert!(
        selection.focus.paragraph > on_page_two + 5,
        "scrolling took in more: {on_page_two} to {}",
        selection.focus.paragraph
    );

    // The timer has stopped: a late tick moves nothing.
    let scroll = s.rt.workspace_scroll();
    s.no_change("a late tick", |rt| {
        rt.send(Message::DragScrollTick);
    });
    assert_eq!(s.rt.workspace_scroll(), scroll);
    assert_eq!(s.app().page_selection, selection);

    let dragged = copy_and_compare(&mut s, "Copy the dragged selection");
    assert!(
        dragged.contains("Paragraph 60 "),
        "text from page 2 onwards"
    );

    // Shift+click lower in the view extends from the same anchor.
    let (point, paragraph) = visible_text(&mut s, 0.9);
    shift_click(&mut s, "Shift+click below", point);
    let extended = s.app().page_selection;
    assert_eq!(extended.anchor, anchor);
    assert_eq!(extended.focus.paragraph, paragraph);
    let longer = copy_and_compare(&mut s, "Copy the extended selection");

    // Shift+click higher up shrinks it; the anchor still stays.
    let (point, paragraph) = visible_text(&mut s, 0.1);
    assert!(paragraph < extended.focus.paragraph);
    shift_click(&mut s, "Shift+click above", point);
    assert_eq!(s.app().page_selection.anchor, anchor);
    assert_eq!(s.app().page_selection.focus.paragraph, paragraph);
    let shorter = copy_and_compare(&mut s, "Copy the shrunk selection");
    assert!(shorter.len() < longer.len());

    // Delete takes exactly what was selected, in one step.
    let before = s.main_text();
    s.edit("Delete the selection", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert_eq!(s.main_text().len(), before.len() - shorter.len());
    s.save_sam("SELECT.SAM");
}
