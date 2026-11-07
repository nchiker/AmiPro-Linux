//! Drawing end to end (`UI_SPEC.md` §33): a drawing frame made by Insert ›
//! Drawing…, shapes drawn with the tools, picked by a marquee, grouped,
//! moved, undone, ungrouped, kept by Save As SAM and drawn again after the
//! file is reopened.

use super::pictures::{draw_shape, objects, picture_frames};
use super::*;
use crate::draw_mode::{Event as DrawEvent, Keys, Tool};
use amipro_pictures::sdw::{Object, Shape};

fn drawn(s: &Scenario, frame: FrameId) -> Vec<Object> {
    objects(s, frame)
}

fn selected(s: &Scenario) -> usize {
    s.app()
        .draw
        .as_ref()
        .unwrap_or_else(|| s.fail("draw", "Draw mode is not open"))
        .selection
        .len()
}

/// A press, a drag and a release with the Select tool (a step that must
/// change the drawing).
fn drag(s: &mut Scenario, what: &str, from: (f64, f64), to: (f64, f64)) {
    let keys = Keys::default();
    s.edit(what, move |rt| {
        rt.send(Message::Draw(DrawEvent::Press {
            at: from,
            keys,
            double: false,
        }));
        rt.send(Message::Draw(DrawEvent::Drag { at: to, keys }));
        rt.send(Message::Draw(DrawEvent::Release { at: to, keys }));
    });
}

#[test]
fn a_drawing_is_made_picked_grouped_moved_undone_saved_and_drawn_again() {
    let mut s = Scenario::new_document("drawing-workflow");
    s.clean_folder();
    s.type_text("Text above the drawing.");
    let main = s.main_flow();
    s.caret(main, 0, "Text above the drawing.".len(), 1, false);

    // Insert › Drawing… makes the frame and opens Draw mode in it.
    s.command(CommandId::Drawing);
    if s.app().draw.is_none() {
        s.act("open Draw mode again", |rt| {
            rt.send(Message::Command(CommandId::Redo));
        });
    }
    let frame = s
        .app()
        .draw
        .as_ref()
        .map(|mode| mode.frame)
        .unwrap_or_else(|| s.fail("Drawing", "Draw mode is not open"));
    assert!(drawn(&s, frame).is_empty());

    // Three shapes with their tools.
    draw_shape(&mut s, Tool::Rectangle, (200.0, 200.0), (1_200.0, 800.0));
    draw_shape(&mut s, Tool::Ellipse, (1_400.0, 200.0), (2_200.0, 800.0));
    draw_shape(&mut s, Tool::Line, (200.0, 1_000.0), (2_200.0, 1_200.0));
    assert_eq!(drawn(&s, frame).len(), 3);

    // A marquee from empty space picks all three; it is not an edit.
    s.act("choose Select", |rt| {
        rt.send(Message::DrawTool(Tool::Select));
    });
    let keys = Keys::default();
    s.no_change("marquee over every shape", move |rt| {
        let (from, to) = ((50.0, 50.0), (2_400.0, 1_400.0));
        rt.send(Message::Draw(DrawEvent::Press {
            at: from,
            keys,
            double: false,
        }));
        rt.send(Message::Draw(DrawEvent::Drag { at: to, keys }));
        rt.send(Message::Draw(DrawEvent::Release { at: to, keys }));
    });
    assert_eq!(selected(&s), 3);

    // Group: one object holding the three.
    s.command(CommandId::DrawGroup);
    let objects_now = drawn(&s, frame);
    assert_eq!(objects_now.len(), 1);
    let Shape::Group {
        objects: inside, ..
    } = &objects_now[0].shape
    else {
        s.fail("group", "the object is not a group");
    };
    assert_eq!(inside.len(), 3);
    let group_left = objects_now[0].rect.left;
    let group_top = objects_now[0].rect.top;

    // Move the group by dragging its rectangle's edge; Undo puts it back
    // (the checker also undoes and redoes the step itself).
    drag(&mut s, "drag the group", (200.0, 500.0), (700.0, 900.0));
    let moved = drawn(&s, frame);
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].rect.left, group_left + 500);
    assert_eq!(moved[0].rect.top, group_top + 400);
    s.act("Undo the move", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(drawn(&s, frame)[0].rect.left, group_left);
    s.act("Redo the move", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert_eq!(drawn(&s, frame)[0].rect.left, group_left + 500);

    // Ungroup: three objects again, all moved.
    s.command(CommandId::DrawUngroup);
    let apart = drawn(&s, frame);
    assert_eq!(apart.len(), 3);
    assert!(
        apart
            .iter()
            .all(|object| !matches!(object.shape, Shape::Group { .. }))
    );

    // Leave Draw mode; the frame holds the drawing and text goes on around it.
    s.command_act(CommandId::DrawDone);
    assert!(s.app().draw.is_none());
    s.caret_to_end();
    s.type_text(" And below.");

    // Saved as SAM and opened by a fresh window: the same objects, drawn.
    let path = s.save_sam("DRAWING.SAM");
    let mut fresh = s.reopen(&path);
    let frames = picture_frames(&fresh);
    assert_eq!(frames.len(), 1);
    let back = drawn(&fresh, frames[0]);
    assert_eq!(back.len(), 3);
    for (before, after) in apart.iter().zip(&back) {
        assert_eq!(before.rect, after.rect, "a shape kept its place");
        assert_eq!(before.kind, after.kind, "a shape kept its kind");
    }
    fresh.act("redraw the reopened drawing", |rt| {
        rt.redraw();
    });
    assert!(fresh.main_text().ends_with("And below."));
}
