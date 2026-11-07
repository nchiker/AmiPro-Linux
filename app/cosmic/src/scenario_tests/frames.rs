//! Frames (`UI_SPEC.md` §11, §20): Insert › Frame…, Draw Frame, typing in a
//! frame, move, resize, Frame Properties, Bring to Front and Send to Back,
//! Group and Ungroup, Delete (which asks), and Undo of each.

use amipro_document::{FrameContent, FrameId, Placement, Rect};
use cosmic::iced::mouse::Button;

use crate::frame_editing::{FrameAction, FrameSelection};
use crate::frame_inspector::{Choice, Field, PlacementKind};
use crate::page_dialogs::{CreateFrameAction, GeometryField};

use super::*;

/// The Where placed rectangle of `frame`.
fn rect_of(s: &Scenario, frame: FrameId) -> Rect {
    match s.document().frame(frame).unwrap().placement {
        Placement::Fixed { rect, .. } => rect,
        other => s.fail(
            "frame",
            &format!("{frame:?} is not Where placed: {other:?}"),
        ),
    }
}

fn last_frame(s: &Scenario) -> FrameId {
    *s.document()
        .frame_stack()
        .last()
        .unwrap_or_else(|| s.fail("frame", "the document has no frames"))
}

/// Text typed without the Undo and Redo check (see
/// `typing_in_a_frame_survives_undo_and_redo_with_the_caret`).
fn type_unchecked(s: &mut Scenario, text: &str) {
    let text = text.to_owned();
    s.act(&format!("type {text:?} (not undo-checked)"), move |rt| {
        rt.send(Message::Page(PageAction::Insert(text)));
    });
}

/// Insert › Frame… filled in with inches, then Create: one undo step.
fn create_frame(s: &mut Scenario, width: &str, height: &str, top: &str, left: &str) {
    let values = [
        (GeometryField::Width, width.to_owned()),
        (GeometryField::Height, height.to_owned()),
        (GeometryField::Top, top.to_owned()),
        (GeometryField::Left, left.to_owned()),
    ];
    s.edit(
        &format!("Insert › Frame… {width} × {height} at {top}, {left}"),
        move |rt| {
            rt.send(Message::Command(CommandId::Frame));
            assert!(
                rt.app.create_frame_dialog.is_some(),
                "the Create Frame dialog opens"
            );
            assert!(cosmic::Application::dialog(&rt.app).is_some());
            for (field, text) in values {
                rt.send(Message::CreateFrame(CreateFrameAction::Input(field, text)));
            }
            rt.send(Message::CreateFrame(CreateFrameAction::Create));
            assert!(rt.app.create_frame_dialog.is_none(), "the dialog closes");
            // The new frame is selected, with Frame Properties in the rail.
            let frame = *rt.app.document.frame_stack().last().unwrap();
            assert_eq!(
                rt.app.frame_selection,
                Some(FrameSelection::one(frame, 1)),
                "the new frame is selected"
            );
            assert!(rt.app.frame_inspector_view().is_some());
        },
    );
    // (After the step's Undo and Redo the frame is no longer selected:
    // `redo_does_not_select_the_recreated_frame` notes it.)
    let frame = last_frame(s);
    s.act("select the new frame", move |rt| {
        rt.send(Message::Frame(FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        }));
    });
}

#[test]
fn redo_of_insert_frame_selects_the_frame_again() {
    // Redo of a step that selected what it made (Insert › Frame…,
    // Draw Frame) leaves the same selection as the step did.
    let mut s = Scenario::new_document("frames-redo-selection");
    s.type_text("Text");
    s.rt.send(Message::Command(CommandId::Frame));
    s.rt.send(Message::CreateFrame(CreateFrameAction::Create));
    let frame = last_frame(&s);
    assert_eq!(s.app().frame_selection, Some(FrameSelection::one(frame, 1)));
    s.rt.send(Message::Command(CommandId::Undo));
    assert_eq!(s.app().frame_selection, None);
    s.rt.send(Message::Command(CommandId::Redo));
    assert_eq!(
        s.app().frame_selection,
        Some(FrameSelection::one(frame, 1)),
        "Redo brings the frame back selected, as Insert › Frame… left it"
    );
}

fn zoom_100(s: &mut Scenario) {
    s.act("zoom 100%", |rt| {
        rt.send(Message::ZoomTo(100));
        rt.redraw();
        rt.redraw();
    });
}

fn paragraphs(s: &mut Scenario) {
    s.type_text("A letter that has a frame in it. The text of the page goes around the frame, as Ami Pro's Wrap around does, and goes on to a second line, a third, and beyond.");
    s.enter();
    s.type_text("The second paragraph of the page, which also wraps around the frame when it reaches it, with words enough to fill a few lines of the page.");
}

fn inspector(s: &mut Scenario, what: &str, message: Message) {
    s.edit(what, move |rt| {
        rt.send(message);
    });
}

#[test]
fn a_frame_is_made_with_the_dialog_typed_in_moved_resized_by_mouse_and_keys_and_saved() {
    let mut s = Scenario::new_document("frames-make-move-resize");
    s.clean_folder();
    paragraphs(&mut s);
    zoom_100(&mut s);

    // Bad values are refused in place; Cancel changes nothing.
    s.no_change(
        "Insert › Frame… with a width that does not read, then Cancel",
        |rt| {
            rt.send(Message::Command(CommandId::Frame));
            rt.send(Message::CreateFrame(CreateFrameAction::Input(
                GeometryField::Width,
                "wide".into(),
            )));
            rt.send(Message::CreateFrame(CreateFrameAction::Create));
            let dialog = rt
                .app
                .create_frame_dialog
                .as_ref()
                .expect("the dialog stays");
            assert!(dialog.error.is_some(), "the reason is shown");
            rt.send(Message::CreateFrame(CreateFrameAction::Cancel));
            assert!(rt.app.create_frame_dialog.is_none());
        },
    );

    // FC1: 3.00 × 1.50 in, 2.00 in down, 1.50 in in.
    create_frame(&mut s, "3.00", "1.50", "2.00", "1.50");
    let frame = last_frame(&s);
    assert_eq!(rect_of(&s, frame), Rect::new(2_160, 2_880, 6_480, 5_040));
    assert!(matches!(
        s.document().frame(frame).unwrap().content,
        FrameContent::Text(_)
    ));
    assert!(s.app().command_enabled(CommandId::FrameProperties));
    assert!(
        s.app().frame_inspector_view().is_some(),
        "Frame Properties shows"
    );

    // Text in the frame: Enter puts the caret there; Esc selects it again.
    s.act("Enter: the caret goes into the frame", |rt| {
        rt.send(Message::Frame(FrameAction::EnterText { at: None }));
    });
    let flow = flow_of(s.document(), frame);
    assert_eq!(s.app().page_selection.flow(), flow);
    assert_eq!(s.app().frame_selection, None);
    type_unchecked(&mut s, "Text typed in the frame");
    s.act("Esc: select the frame again", |rt| {
        rt.send(Message::Page(PageAction::Escape));
    });
    assert_eq!(s.app().frame_selection, Some(FrameSelection::one(frame, 1)));
    assert_eq!(
        s.document().flow(flow).unwrap().plain_text(),
        "Text typed in the frame"
    );

    // Move by the mouse: press on the frame's border area, drag, release.
    s.act("click an empty place on the page: nothing selected", |rt| {
        let empty = rt.on_page(9_000.0, 9_000.0);
        rt.click(Button::Left, empty);
    });
    s.act("click the frame: it is selected", |rt| {
        let inside = rt.on_page(3_000.0, 3_500.0);
        rt.click(Button::Left, inside);
    });
    assert_eq!(
        s.app().frame_selection,
        Some(FrameSelection::one(frame, 1)),
        "a click on a frame selects it"
    );
    let before = rect_of(&s, frame);
    s.edit("drag the frame 0.5 in right and down", |rt| {
        // (Far from the click above: two presses close together are a
        // double-click, which enters the frame's text.)
        let from = rt.on_page(5_400.0, 4_500.0);
        let to = rt.on_page(6_120.0, 5_220.0);
        rt.drag(from, to);
    });
    let moved = rect_of(&s, frame);
    assert_ne!(moved, before, "the frame moved");
    assert_eq!(
        (moved.width(), moved.height()),
        (before.width(), before.height()),
        "its size is unchanged"
    );
    assert!(moved.left > before.left && moved.top > before.top);

    // Resize by the bottom-right handle.
    s.edit("drag the bottom-right handle", |rt| {
        let corner = rt.on_page(moved.right as f32, moved.bottom as f32);
        let to = rt.on_page(moved.right as f32 + 720.0, moved.bottom as f32 + 360.0);
        rt.drag(corner, to);
    });
    let resized = rect_of(&s, frame);
    assert!(
        resized.right > moved.right && resized.bottom > moved.bottom,
        "{moved:?} -> {resized:?}"
    );
    assert_eq!((resized.left, resized.top), (moved.left, moved.top));

    // Arrow keys within a second are one step; Alt+arrows resize.
    s.edit("Right arrow ×3 (one step)", |rt| {
        for _ in 0..3 {
            rt.send(Message::Frame(FrameAction::Move {
                dx: 20,
                dy: 0,
                key: true,
            }));
        }
    });
    assert_eq!(rect_of(&s, frame), resized.offset(60, 0).unwrap());
    s.edit("Alt+Down resizes", |rt| {
        rt.send(Message::Frame(FrameAction::ResizeBy { dx: 0, dy: 100 }));
    });

    // The frame text is still there and the page still lays out around it.
    assert_eq!(
        s.document().flow(flow).unwrap().plain_text(),
        "Text typed in the frame"
    );
    assert!(
        s.app().pages.layout().pages[0]
            .frames
            .iter()
            .any(|b| b.frame == frame),
        "the frame is laid out on page 1"
    );

    let final_rect = rect_of(&s, frame);
    let path = s.save_sam("FRAMED.SAM");
    let s = s.reopen(&path);
    let frame = last_frame(&s);
    assert_eq!(s.document().frame_stack().len(), 1);
    assert_eq!(rect_of(&s, frame), final_rect);
    let flow = flow_of(s.document(), frame);
    assert_eq!(
        s.document().flow(flow).unwrap().plain_text(),
        "Text typed in the frame"
    );
}

#[test]
fn typing_in_a_frame_survives_undo_and_redo_with_the_caret() {
    // Undo and Redo of text typed in the main text return the caret where
    // the step left it; the same must hold in a frame's text.
    let mut s = Scenario::new_document("frames-typing-undo");
    s.type_text("Main text");
    create_frame(&mut s, "2.00", "1.00", "3.00", "2.00");
    s.act("Enter: the caret goes into the frame", |rt| {
        rt.send(Message::Frame(FrameAction::EnterText { at: None }));
    });
    s.type_text("Frame text");
}

#[test]
fn frame_properties_change_a_frame_live_one_undo_step_each() {
    let mut s = Scenario::new_document("frames-properties");
    s.clean_folder();
    paragraphs(&mut s);
    create_frame(&mut s, "3.00", "1.50", "2.00", "1.50");
    let frame = last_frame(&s);
    let style = |s: &Scenario| s.document().frame(frame).unwrap().style.clone();

    // A typed value waits for Enter; a refused one keeps the old value and
    // says why.
    s.no_change("type a width without Enter", |rt| {
        rt.send(Message::InspectorInput(Field::Width, "3.5 in".into()));
    });
    assert_eq!(rect_of(&s, frame).width(), 4_320);
    s.edit("Width 3.5 in, Enter", |rt| {
        rt.send(Message::InspectorSubmit(Field::Width));
    });
    assert_eq!(rect_of(&s, frame).width(), 5_040);
    s.no_change("Corners 250 is refused", |rt| {
        rt.send(Message::InspectorInput(Field::Corners, "250".into()));
        rt.send(Message::InspectorSubmit(Field::Corners));
        assert!(
            rt.app.inspector_error.as_ref().is_some_and(
                |(field, reason)| *field == Field::Corners && reason.contains("0 to 100")
            )
        );
    });
    s.edit("Corners 40", |rt| {
        rt.send(Message::InspectorInput(Field::Corners, "40".into()));
        rt.send(Message::InspectorSubmit(Field::Corners));
    });
    assert_eq!(style(&s).corners, 40);
    inspector(
        &mut s,
        "Transparent",
        Message::InspectorChoice(Choice::Opaque(false)),
    );
    assert!(!style(&s).opaque);
    inspector(
        &mut s,
        "Wrap: no wrap around",
        Message::InspectorChoice(Choice::Wrap(amipro_document::Wrap::NoWrapAround)),
    );
    inspector(
        &mut s,
        "Shadow: normal",
        Message::InspectorChoice(Choice::ShadowDepth(Some(72))),
    );
    inspector(
        &mut s,
        "Line: no top",
        Message::InspectorChoice(Choice::Side(2, false)),
    );
    s.edit("Columns 2", |rt| {
        rt.send(Message::InspectorInput(Field::Columns, "2".into()));
        rt.send(Message::InspectorSubmit(Field::Columns));
    });
    assert_eq!(style(&s).interior.columns, 2);
    inspector(
        &mut s,
        "Line between columns",
        Message::InspectorChoice(Choice::LineBetween(true)),
    );
    s.edit("Margin top 0.2", |rt| {
        rt.send(Message::InspectorInput(Field::MarginTop, "0.2".into()));
        rt.send(Message::InspectorSubmit(Field::MarginTop));
    });
    s.edit("Name: Sidebar", |rt| {
        rt.send(Message::InspectorInput(Field::Name, "Sidebar".into()));
        rt.send(Message::InspectorSubmit(Field::Name));
    });
    assert_eq!(style(&s).name.as_deref(), Some("Sidebar"));

    // Placement: With para above keeps the frame selected where it is drawn.
    // (Undo and Redo of a placement change are checked on their own below.)
    s.act("Placement: With para above", |rt| {
        rt.send(Message::InspectorChoice(Choice::Placement(
            PlacementKind::WithParaAbove,
        )));
    });
    assert!(matches!(
        s.document().frame(frame).unwrap().placement,
        Placement::BelowParagraph { .. }
    ));
    let selection = s.app().frame_selection.clone().expect("still selected");
    assert!(
        s.app()
            .page_box(selection.page)
            .unwrap()
            .frames
            .iter()
            .any(|b| b.frame == frame)
    );
    s.act("Placement: Where placed", |rt| {
        rt.send(Message::InspectorChoice(Choice::Placement(
            PlacementKind::WherePlaced,
        )));
    });

    // Make Default keeps what Ami Pro keeps; the next frame takes it.
    s.act("Make Default for New Frames", |rt| {
        rt.send(Message::NewFrameDefault(NewFrameDefault::Make));
    });
    create_frame(&mut s, "2.00", "1.00", "5.00", "4.00");
    let next = last_frame(&s);
    assert_ne!(next, frame);
    assert!(
        !s.document().frame(next).unwrap().style.opaque,
        "transparent, as made default"
    );
    s.act("Restore Ami Pro's Default", |rt| {
        rt.send(Message::NewFrameDefault(NewFrameDefault::Restore));
    });

    let path = s.save_sam("PROPS.SAM");
    let s = s.reopen(&path);
    assert_eq!(s.document().frame_stack().len(), 2);
    let named = s
        .document()
        .frames()
        .find(|f| f.style.name.as_deref() == Some("Sidebar"))
        .unwrap_or_else(|| s.fail("reopen", "the frame's name was lost"));
    assert_eq!(named.style.interior.columns, 2);
    assert_eq!(named.style.corners, 40);
}

#[test]
fn frames_are_restacked_grouped_cycled_and_deleted_after_asking() {
    let mut s = Scenario::new_document("frames-stack-group-delete");
    s.clean_folder();
    paragraphs(&mut s);
    create_frame(&mut s, "2.00", "1.00", "2.00", "1.50");
    let first = last_frame(&s);
    create_frame(&mut s, "2.00", "1.00", "4.00", "1.50");
    let second = last_frame(&s);
    assert_ne!(first, second);

    // Restack: Send to Back puts the second behind the first.
    let position = |s: &Scenario, frame| {
        s.document()
            .frame_stack()
            .iter()
            .position(|id| *id == frame)
            .unwrap()
    };
    assert!(position(&s, second) > position(&s, first));
    s.command(CommandId::SendToBack);
    assert!(position(&s, second) < position(&s, first));
    s.command(CommandId::BringToFront);
    assert!(position(&s, second) > position(&s, first));

    // Tab cycles through the frames.
    s.act("select the first frame", |rt| {
        rt.send(Message::Frame(FrameAction::Select {
            frame: first,
            page: 1,
            extend: false,
        }));
    });
    s.act("Tab: the next frame", |rt| {
        rt.send(Message::Frame(FrameAction::Cycle { backwards: false }));
    });
    assert_eq!(
        s.app().frame_selection,
        Some(FrameSelection::one(second, 1))
    );

    // Shift+click gathers both: Group makes one group, in one step; the
    // group moves as one.
    s.act("Shift+click the first frame", |rt| {
        rt.send(Message::Frame(FrameAction::Select {
            frame: first,
            page: 1,
            extend: true,
        }));
    });
    assert!(s.app().command_enabled(CommandId::GroupFrames));
    s.command(CommandId::GroupFrames);
    let group = s.document().frame(first).unwrap().group;
    assert!(group.is_some() && s.document().frame(second).unwrap().group == group);
    assert!(s.app().command_enabled(CommandId::UngroupFrames));
    assert!(!s.app().command_enabled(CommandId::GroupFrames));
    let (a, b) = (rect_of(&s, first), rect_of(&s, second));
    s.edit("move the group", |rt| {
        rt.send(Message::Frame(FrameAction::Move {
            dx: 100,
            dy: 40,
            key: false,
        }));
    });
    assert_eq!(rect_of(&s, first), a.offset(100, 40).unwrap());
    assert_eq!(rect_of(&s, second), b.offset(100, 40).unwrap());
    s.command(CommandId::UngroupFrames);
    assert_eq!(s.document().frame(first).unwrap().group, None);

    // Delete asks first; Cancel keeps the frame; Delete (confirmed)
    // removes it, one undo step.
    s.act("select the second frame", |rt| {
        rt.send(Message::Frame(FrameAction::Select {
            frame: second,
            page: 1,
            extend: false,
        }));
    });
    s.no_change("Delete asks; Cancel keeps the frame", |rt| {
        rt.send(Message::Command(CommandId::Delete));
        assert!(rt.app.pending_frame_delete);
        assert!(cosmic::Application::dialog(&rt.app).is_some());
        rt.send(Message::CancelFrameDelete);
        assert!(!rt.app.pending_frame_delete);
        assert!(rt.app.document.frame(second).is_some());
    });
    s.edit("Delete, confirmed", |rt| {
        rt.send(Message::Command(CommandId::Delete));
        rt.send(Message::ConfirmFrameDelete);
    });
    assert!(s.document().frame(second).is_none());
    assert_eq!(s.app().frame_selection, None);
    assert_eq!(s.app().page_selection.flow(), s.main_flow());

    // Draw Frame: a crosshair, then a drag on the page makes a frame.
    zoom_100(&mut s);
    s.edit("Insert › Draw Frame and drag on the page", |rt| {
        rt.send(Message::Command(CommandId::DrawFrame));
        assert!(rt.app.drawing_frame);
        let from = rt.on_page(7_000.0, 2_000.0);
        let to = rt.on_page(9_500.0, 3_400.0);
        rt.drag(from, to);
        assert!(!rt.app.drawing_frame);
    });
    let drawn = last_frame(&s);
    let rect = rect_of(&s, drawn);
    assert!(
        (rect.left - 7_000).abs() <= 30 && (rect.bottom - 3_400).abs() <= 30,
        "{rect:?}"
    );
    // A click draws nothing and ends drawing.
    s.no_change("Draw Frame, then a click", |rt| {
        rt.send(Message::Command(CommandId::DrawFrame));
        let empty = rt.on_page(9_000.0, 8_000.0);
        rt.click(Button::Left, empty);
        assert!(!rt.app.drawing_frame);
    });

    let path = s.save_sam("STACKED.SAM");
    let s = s.reopen(&path);
    assert_eq!(s.document().frame_stack().len(), 2);
}

#[test]
fn a_placement_change_is_redone_with_the_caret_where_the_step_left_it() {
    let mut s = Scenario::new_document("frames-placement-redo");
    s.type_text("Some text in the main flow of the page.");
    create_frame(&mut s, "2.00", "1.00", "3.00", "2.00");
    s.edit("Placement: With para above", |rt| {
        rt.send(Message::InspectorChoice(Choice::Placement(
            PlacementKind::WithParaAbove,
        )));
    });
}
