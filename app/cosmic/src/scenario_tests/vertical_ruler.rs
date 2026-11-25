//! View › Vertical Ruler in use (`UI_SPEC.md` §48): shown and remembered,
//! the margins dragged on its markers on the first and a later page with
//! typing between, undone, saved, and read back.

use amipro_document::Twips;

use super::*;

fn margins(s: &Scenario) -> (Twips, Twips) {
    let margins = s.document().document_layout().right.layout.margins;
    (margins.top, margins.bottom)
}

/// Drags the vertical ruler's marker at page twips `y` of the page at
/// `page_index` by `by` twips, as one undo step.
fn drag_marker(s: &mut Scenario, what: &str, page_index: usize, y: f32, by: f32) {
    let on_page = page_point(&mut s.rt, page_index, 0.0, y);
    let (bounds, view) = s.rt.workspace_view();
    let from = cosmic::iced::Point::new(bounds.x - crate::ruler_vertical::WIDTH + 11.0, on_page.y);
    assert!(
        from.y > bounds.y && from.y < bounds.y + bounds.height,
        "the marker is on the screen: {from:?} in {bounds:?}"
    );
    let to = cosmic::iced::Point::new(from.x, from.y + by * view.scale);
    s.edit(what, move |rt| {
        rt.drag(from, to);
    });
}

/// The top of page 1's first line of text.
fn first_line_top(s: &Scenario) -> f32 {
    s.app().pages.layout().pages[0].lines[0].top
}

#[test]
fn the_vertical_ruler_is_remembered_and_its_margin_markers_work_through_a_session() {
    let text: Vec<String> = (0..70)
        .map(|index| format!("Paragraph {index} of the report."))
        .collect();
    let mut s = Scenario::new(
        "vertical-ruler",
        Document::from_plain_text(&text.join("\n")),
    );
    let folder = s.clean_folder();
    let (settings, preferences) = scratch_settings(&folder);
    assert!(!preferences.vertical_ruler_visible);
    s.app_mut().settings = Some(settings);
    s.act("zoom 50%", |rt| {
        rt.send(Message::ZoomTo(50));
        rt.redraw();
    });
    let without = s.rt.workspace_view().0;

    s.command_act(CommandId::VerticalRuler);
    assert_eq!(
        crate::menu_check(s.app(), CommandId::VerticalRuler),
        Some(true)
    );
    assert!(scratch_settings(&folder).1.vertical_ruler_visible, "saved");
    s.act("redraw", |rt| {
        rt.redraw();
    });
    let with = s.rt.workspace_view().0;
    assert!((with.x - without.x - crate::ruler_vertical::WIDTH).abs() <= 1.0);

    // The top margin down an inch: the text moves with it.
    assert_eq!(margins(&s), (1_440, 1_440));
    let top = first_line_top(&s);
    let pages = s.app().pages.layout().pages.len();
    drag_marker(&mut s, "drag the top margin down", 0, 1_440.0, 1_440.0);
    assert_eq!(margins(&s), (2_880, 1_440));
    assert_eq!(first_line_top(&s), top + 1_440.0);
    drag_marker(&mut s, "drag the bottom margin up", 0, 14_400.0, -1_440.0);
    assert_eq!(margins(&s), (2_880, 2_880));
    assert!(s.app().pages.layout().pages.len() >= pages);

    // Typing between the drags, then each is undone in turn.
    s.caret_to_end();
    s.type_text(" Typed between the drags.");
    for expected in [(2_880, 2_880), (2_880, 1_440), (1_440, 1_440)] {
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        assert_eq!(margins(&s), expected);
    }
    for expected in [(2_880, 1_440), (2_880, 2_880), (2_880, 2_880)] {
        s.act("Redo", |rt| {
            rt.send(Message::Command(CommandId::Redo));
        });
        assert_eq!(margins(&s), expected);
    }
    assert!(s.main_text().ends_with("Typed between the drags."));

    // On page 2, scrolled into view, the markers are that page's.
    s.act("back to the top, then Next Page", |rt| {
        rt.send(Message::Command(CommandId::PreviousPage));
        rt.send(Message::Command(CommandId::PreviousPage));
        rt.send(Message::Command(CommandId::NextPage));
        rt.redraw();
    });
    assert_eq!(s.app().page_selection.page, 2);
    drag_marker(&mut s, "drag page 2's top margin up", 1, 2_880.0, -720.0);
    assert_eq!(margins(&s), (2_160, 2_880));

    let path = s.save_sam("VRULER.SAM");
    let reopened = s.reopen(&path);
    assert_eq!(margins(&reopened), (2_160, 2_880));

    // A restart keeps the ruler; turning it off is kept too.
    let (_, preferences) = scratch_settings(&folder);
    assert!(preferences.vertical_ruler_visible);
    let mut restarted = Scenario::new_document("vertical-ruler");
    restarted.app_mut().preferences = preferences;
    restarted.act("redraw after the restart", |rt| {
        rt.redraw();
    });
    assert_eq!(restarted.rt.workspace_view().0.x, with.x);
    s.command_act(CommandId::VerticalRuler);
    assert!(!scratch_settings(&folder).1.vertical_ruler_visible);
    s.act("redraw", |rt| {
        rt.redraw();
    });
    assert_eq!(s.rt.workspace_view().0.x, without.x);
}
