//! Pictures and drawings: inserted, scaled, moved, pasted, dropped, drawn in
//! Draw mode, styled, deleted, saved and reopened (`UI_SPEC.md` §25, §33, §24).

use super::*;
use crate::draw_mode::{Event as DrawEvent, Key as DrawKey, Keys, Tool};
use crate::frame_editing::FrameAction;
use crate::frame_inspector::{Choice, Field, PlacementKind};
use amipro_editing::drawing::{StyleChange, drawing_of};
use amipro_pictures::sdw::{Colour, Shape};

/// A PNG of `width` × `height` pixels in the scenario's folder.
fn png_file(s: &Scenario, name: &str, width: u32, height: u32) -> std::path::PathBuf {
    let path = s.folder().join(name);
    std::fs::create_dir_all(s.folder()).unwrap();
    let mut png = Vec::new();
    image::RgbaImage::from_fn(width, height, |x, y| {
        image::Rgba([(x * 255 / width) as u8, (y * 255 / height) as u8, 128, 255])
    })
    .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
    .unwrap();
    std::fs::write(&path, png).unwrap();
    path
}

/// A 60 × 40 BMP (a format Ami Pro holds as it is) in the scratch folder.
pub(super) fn bmp_file(s: &Scenario, name: &str) -> std::path::PathBuf {
    let path = s.folder().join(name);
    std::fs::create_dir_all(s.folder()).unwrap();
    let raster = amipro_pictures::Raster::filled(60, 40, [255, 0, 255, 255]).unwrap();
    std::fs::write(&path, amipro_pictures::dib::encode_bmp24(&raster, (0, 0))).unwrap();
    path
}

fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(width, height, image::Rgba([0, 128, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    png
}

/// Insert › Image… with the chooser's answer, as an editing step.
pub(super) fn insert_image(s: &mut Scenario, path: &std::path::Path, link: bool) {
    let path = path.to_owned();
    s.edit(&format!("Insert Image {}", path.display()), move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::ImageChosen {
            session_id,
            result: FileDialogResult::Selected(path),
            link,
        });
    });
}

/// The picture frames in the document, in the stack's order.
pub(super) fn picture_frames(s: &Scenario) -> Vec<FrameId> {
    s.document()
        .frame_stack()
        .iter()
        .copied()
        .filter(|id| {
            matches!(
                s.document().frame(*id).map(|frame| &frame.content),
                Some(FrameContent::Picture(_))
            )
        })
        .collect()
}

fn picture_of(s: &Scenario, frame: FrameId) -> amipro_document::Picture {
    match &s.document().frame(frame).map(|frame| &frame.content) {
        Some(FrameContent::Picture(picture)) => picture.clone(),
        _ => s.fail("picture", "the frame holds no picture"),
    }
}

fn select_frame(s: &mut Scenario, frame: FrameId) {
    s.no_change("select the frame", move |rt| {
        rt.send(Message::Frame(FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        }));
    });
    if s.app().frame_selection.is_none() {
        s.fail("select", "the frame is not selected");
    }
}

fn inspector(s: &mut Scenario, field: Field, text: &str) {
    let text = text.to_owned();
    s.edit(
        &format!("Frame Properties: {field:?} = {text}"),
        move |rt| {
            rt.send(Message::InspectorInput(field, text));
            rt.send(Message::InspectorSubmit(field));
        },
    );
}

fn frame_rect(s: &Scenario, frame: FrameId) -> amipro_document::Rect {
    match s.document().frame(frame).unwrap().placement {
        amipro_document::Placement::Fixed { rect, .. } => rect,
        other => s.fail("frame", &format!("not Where placed: {other:?}")),
    }
}

/// The frame's box on the first page, from the layout.
fn frame_bounds(s: &mut Scenario, frame: FrameId) -> amipro_layout::Bounds {
    s.rt.redraw();
    s.app().pages.layout().pages[0]
        .frames
        .iter()
        .find(|frame_box| frame_box.frame == frame)
        .map(|frame_box| frame_box.bounds)
        .unwrap_or_else(|| s.fail("layout", "the frame has no box on page 1"))
}

#[test]
fn a_picture_inserted_typed_around_scaled_moved_and_saved() {
    let mut s = Scenario::new_document("pictures-insert");
    s.clean_folder();
    s.type_text("Before the picture. After it.");
    let picture = bmp_file(&s, "photo.bmp");
    let main = s.main_flow();
    s.caret(main, 0, "Before the picture.".len(), 1, false);

    // Insert › Image… puts a Flow with text frame at the caret, selected.
    insert_image(&mut s, &picture, false);
    let frames = picture_frames(&s);
    assert_eq!(frames.len(), 1);
    let frame = frames[0];
    assert!(matches!(
        picture_of(&s, frame).source,
        amipro_document::PictureSource::Embedded(_)
    ));
    // (Undo and Redo of the insertion, which the checker does, leave the
    // frame unselected; select it as a click does.)
    select_frame(&mut s, frame);

    // The anchor is a character of the paragraph: type before and after it.
    s.caret(main, 0, 0, 1, false);
    s.type_text(">> ");
    s.caret_to_end();
    s.type_text(" <<");
    assert!(s.main_text().starts_with(">> Before the picture."));
    assert!(s.main_text().ends_with("After it. <<"));
    // A frame anchor's character is never removed by Backspace or Delete
    // (V05, by design): the picture goes with its frame (Delete frame).
    let anchor_at = s.document().paragraphs()[0]
        .text
        .find('\u{FFFC}')
        .expect("an anchor character");
    s.caret(main, 0, anchor_at + '\u{FFFC}'.len_utf8(), 1, false);
    s.no_change("Backspace just after the anchor", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    s.caret(main, 0, anchor_at, 1, false);
    s.no_change("Delete just before the anchor", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert_eq!(picture_frames(&s), [frame]);

    // Frame Properties › Picture.
    select_frame(&mut s, frame);
    s.command_act(CommandId::GraphicsScaling);
    inspector(&mut s, Field::PictureWidth, "1");
    assert_eq!(
        picture_of(&s, frame).scaling.sizing,
        amipro_document::PictureSizing::Custom
    );
    inspector(&mut s, Field::PictureDown, "0.05");
    inspector(&mut s, Field::PictureRight, "0.1");
    inspector(&mut s, Field::PictureRotation, "90");
    s.edit("Reset the picture", |rt| {
        rt.send(Message::InspectorChoice(Choice::PictureReset));
    });
    s.edit("Original size", |rt| {
        rt.send(Message::InspectorChoice(Choice::PictureSizing(
            amipro_document::PictureSizing::Original,
        )));
    });
    // A refused rotation keeps the picture and says why.
    s.expect_status("is not");
    s.no_change("a rotation of 400 is refused", |rt| {
        rt.send(Message::InspectorInput(
            Field::PictureRotation,
            "400".into(),
        ));
        rt.send(Message::InspectorSubmit(Field::PictureRotation));
    });
    s.clear_expected_status();

    // Where placed, then the frame is moved and sized.
    s.edit("Where placed", |rt| {
        rt.send(Message::InspectorChoice(Choice::Placement(
            PlacementKind::WherePlaced,
        )));
    });
    let start = frame_rect(&s, frame);
    s.edit("move by the arrow keys", |rt| {
        rt.send(Message::Frame(FrameAction::Move {
            dx: 144,
            dy: 72,
            key: true,
        }));
    });
    let moved = frame_rect(&s, frame);
    assert_eq!((moved.left - start.left, moved.top - start.top), (144, 72));
    s.edit("resize by the keys", |rt| {
        rt.send(Message::Frame(FrameAction::ResizeBy { dx: 144, dy: 144 }));
    });
    assert_eq!(frame_rect(&s, frame).width(), moved.width() + 144);
    s.edit("Fit Frame to Picture", |rt| {
        rt.send(Message::InspectorChoice(Choice::PictureFitFrame));
    });
    inspector(&mut s, Field::X, "2");
    inspector(&mut s, Field::Width, "3");
    // A drag in the window moves it.
    let bounds = frame_bounds(&mut s, frame);
    let from = s.rt.on_page(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    );
    let to = cosmic::iced::Point::new(from.x + 40.0, from.y + 30.0);
    let before = frame_rect(&s, frame);
    s.edit("drag the frame in the window", move |rt| {
        rt.drag(from, to);
    });
    let after = frame_rect(&s, frame);
    assert!(after.left > before.left && after.top > before.top);
    // Resize with a handle, as the surface sends it.
    s.edit("resize by a handle", move |rt| {
        rt.send(Message::Frame(FrameAction::Resize {
            from: after,
            to: amipro_document::Rect::new(
                after.left,
                after.top,
                after.left + after.width() + 360,
                after.top + after.height() + 120,
            ),
            with_picture: false,
        }));
    });

    // Picture mode: Enter, pan by a drag and by keys, Esc.
    s.act("Enter: picture mode", |rt| {
        rt.send(Message::Frame(FrameAction::EnterText { at: None }));
    });
    if s.app().picture_mode != Some(frame) {
        s.fail("picture mode", "Enter did not start picture mode");
    }
    s.edit("pan by a drag", |rt| {
        rt.send(Message::Frame(FrameAction::Pan {
            dx: 100,
            dy: -50,
            key: false,
        }));
    });
    s.edit("pan by keys", |rt| {
        rt.send(Message::Frame(FrameAction::Pan {
            dx: 72,
            dy: 0,
            key: true,
        }));
    });
    s.act("Esc leaves picture mode", |rt| {
        rt.send(Message::Frame(FrameAction::LeavePicture));
    });
    assert_eq!(s.app().picture_mode, None);

    // The frame's own name and a description.
    inspector(&mut s, Field::Name, "Photo");

    // Back to the text; typing goes on.
    s.caret_to_end();
    s.type_text(" Done.");

    let path = s.save_sam("PICIN.SAM");
    let mut s = s.reopen(&path);
    let frames = picture_frames(&s);
    assert_eq!(frames.len(), 1, "the picture is in the reopened file");
    assert!(matches!(
        picture_of(&s, frames[0]).source,
        amipro_document::PictureSource::Embedded(_)
    ));
    // Delete frame asks first; Cancel keeps it; Confirm deletes it.
    select_frame(&mut s, frames[0]);
    s.no_change("Delete asks first", |rt| {
        rt.send(Message::Frame(FrameAction::Delete));
    });
    if !s.app().pending_frame_delete {
        s.fail("Delete", "no confirmation was asked");
    }
    s.no_change("Cancel keeps the frame", |rt| {
        rt.send(Message::CancelFrameDelete);
    });
    assert_eq!(picture_frames(&s).len(), 1);
    s.act("Delete asks again", |rt| {
        rt.send(Message::Frame(FrameAction::Delete));
    });
    s.edit("Confirm the deletion", |rt| {
        rt.send(Message::ConfirmFrameDelete);
    });
    assert!(picture_frames(&s).is_empty());
    s.save_sam("PICOUT.SAM");
}

#[test]
fn pictures_pasted_dropped_linked_and_refused() {
    let mut s = Scenario::new_document("pictures-paste");
    s.clean_folder();
    s.type_text("Alpha");
    s.enter();
    s.type_text("Beta");
    s.enter();
    s.type_text("Gamma");
    let main = s.main_flow();

    // Paste of an image on the clipboard (the app's message path).
    s.act("the clipboard offers an image", |rt| {
        rt.app.clipboard_has_text = Some(false);
        rt.send(Message::ClipboardImageAvailability {
            has_image: true,
            retry: false,
        });
    });
    if !s.app().command_enabled(CommandId::Paste) {
        s.fail("Paste", "an image alone does not enable Paste");
    }
    s.caret(main, 0, 5, 1, false);
    let png = png_bytes(30, 20);
    let bytes = png.clone();
    s.edit("paste the image", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::ClipboardImage {
            session_id,
            image: Some(crate::picture_insert::ClipboardImage {
                bytes,
                mime: "image/png".into(),
            }),
            text: None,
        });
    });
    assert_eq!(picture_frames(&s).len(), 1);
    // A browser's Copy Image leaves the address as text too: the image wins.
    s.caret(main, 1, 4, 1, false);
    let bytes = png.clone();
    s.edit(
        "paste an image whose address is also on the clipboard",
        move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::ClipboardImage {
                session_id,
                image: Some(crate::picture_insert::ClipboardImage {
                    bytes,
                    mime: "image/png".into(),
                }),
                text: Some("https://example.com/logo.png".into()),
            });
        },
    );
    assert_eq!(picture_frames(&s).len(), 2);
    assert!(!s.main_text().contains("example.com"));
    // Without an image the address pastes as text.
    s.caret(main, 2, 5, 1, false);
    s.edit("paste an address as text", |rt| {
        let session_id = rt.app.id;
        rt.send(Message::ClipboardImage {
            session_id,
            image: None,
            text: Some("https://example.com/logo.png".into()),
        });
    });
    assert!(s.main_text().contains("example.com"));

    // A dropped picture file goes in at the caret.
    let file = png_file(&s, "dropped pic.png", 60, 40);
    s.caret(main, 0, 0, 1, false);
    let uri = format!("file://{}\r\n", file.display()).replace(' ', "%20");
    s.edit("drop a picture file", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::Dropped {
            session_id,
            data: uri.into_bytes(),
            mime: "text/uri-list".into(),
        });
    });
    assert_eq!(picture_frames(&s).len(), 3);
    s.expect_status("could not read");
    // Insert › Image… of a file no reader takes is refused.
    let text = s.folder().join("not-a-picture.png");
    std::fs::write(&text, b"plain text").unwrap();
    let refused = text.clone();
    s.no_change("Insert Image of a text file", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::ImageChosen {
            session_id,
            result: FileDialogResult::Selected(refused),
            link: false,
        });
    });

    // A linked picture keeps its path.
    let linked = png_file(&s, "linked.png", 50, 50);
    s.caret(main, 2, 0, 1, false);
    insert_image(&mut s, &linked, true);
    s.clear_expected_status();
    let frames = picture_frames(&s);
    let last = *frames.last().unwrap();
    select_frame(&mut s, last);
    assert!(matches!(
        picture_of(&s, last).source,
        amipro_document::PictureSource::Linked(_)
    ));
    // With the picture frame selected, Insert › Image… replaces its picture.
    let replacement = png_file(&s, "replacement.png", 40, 80);
    insert_image(&mut s, &replacement, false);
    assert!(matches!(
        picture_of(&s, last).source,
        amipro_document::PictureSource::Embedded(_)
    ));

    // Save As SAM: every picture is embedded, so the list is empty. (A
    // linked Linux path would be listed.)
    // (The PNGs are written as bitmaps, and the save lists them: P8.)
    let path = s.save_sam_expecting(
        "PICPASTE.SAM",
        &["pictures in a format Ami Pro cannot read"],
    );
    let s = s.reopen(&path);
    assert_eq!(picture_frames(&s).len(), 4);
}

#[test]
fn a_linked_picture_is_listed_by_save_as_sam() {
    let mut s = Scenario::new_document("pictures-linked");
    s.clean_folder();
    s.type_text("A linked picture.");
    let file = png_file(&s, "linked.png", 50, 50);
    insert_image(&mut s, &file, true);
    // Ami Pro cannot follow a Linux path: the save lists it and copies the
    // picture in.
    let path = s.save_sam_expecting("LINKED.SAM", &["picture"]);
    let s = s.reopen(&path);
    let frames = picture_frames(&s);
    assert_eq!(frames.len(), 1);
    assert!(matches!(
        picture_of(&s, frames[0]).source,
        amipro_document::PictureSource::Embedded(_)
    ));
}

/// Insert a 2 × 2 table at the caret (Insert › Table…).
fn make_table(s: &mut Scenario) {
    s.command_act(CommandId::Table);
    s.act("Insert the table", |rt| {
        rt.send(Message::InsertTable(
            crate::table_commands::InsertTableAction::Pick(2, 2),
        ));
        rt.send(Message::InsertTable(
            crate::table_commands::InsertTableAction::Insert,
        ));
    });
}

#[test]
fn a_picture_in_a_table_cell_takes_delete_and_ignores_typing() {
    let mut s = Scenario::new_document("pictures-cell");
    s.clean_folder();
    s.type_text("Cells with pictures");
    make_table(&mut s);
    let frame = *s.document().frame_stack().last().unwrap();
    let first = cell(s.document(), frame, 0, 0);
    let second = cell(s.document(), frame, 0, 1);
    // Text in the second cell: no picture can go there.
    s.caret(second, 0, 0, 1, false);
    s.act("type text in the second cell", |rt| {
        rt.send(Message::Page(PageAction::Insert("text".into())));
    });
    s.caret(first, 0, 0, 1, false);
    if !s.app().command_enabled(CommandId::Image) {
        s.fail("Insert Image", "disabled in an empty cell");
    }
    let picture = bmp_file(&s, "cellpic.bmp");
    let path = picture.clone();
    s.edit("Insert Image in the empty cell", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::ImageChosen {
            session_id,
            result: FileDialogResult::Selected(path),
            link: false,
        });
    });
    let FrameContent::Table(table) = &s.document().frame(frame).unwrap().content else {
        panic!("a table");
    };
    let held = table.cells[0].frame.expect("the cell holds the picture");
    assert_eq!(
        s.document().frame(held).unwrap().placement,
        amipro_document::Placement::InCell
    );
    // A cell's picture is not selected (T1); typing does nothing there.
    assert!(s.app().frame_selection.is_none());
    s.caret(first, 0, 0, 1, false);
    s.no_change("typing in the picture's cell does nothing", |rt| {
        rt.send(Message::Page(PageAction::Insert("x".into())));
    });
    // Insert › Image… refused in a cell with a picture or text.
    if s.app().command_enabled(CommandId::Image) {
        s.fail("Insert Image", "enabled in a cell holding a picture");
    }
    s.caret(second, 0, 4, 1, false);
    if s.app().command_enabled(CommandId::Image) {
        s.fail("Insert Image", "enabled in a cell holding text");
    }
    // The picture is saved and read back in its cell.
    // (The save lists "1 table" falsely: see the next scenario.)
    let path = s.save_sam("CELLPIC.SAM");
    let mut s = s.reopen(&path);
    let frame = *s
        .document()
        .frame_stack()
        .iter()
        .find(|id| {
            matches!(
                s.document().frame(**id).unwrap().content,
                FrameContent::Table(_)
            )
        })
        .unwrap();
    let first = cell(s.document(), frame, 0, 0);
    s.caret(first, 0, 0, 1, false);
    // Delete removes the picture, one undo step.
    s.edit("Delete removes the cell's picture", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    let FrameContent::Table(table) = &s.document().frame(frame).unwrap().content else {
        panic!("a table");
    };
    assert!(table.cells[0].frame.is_none());
    s.save_sam("CELLPIC2.SAM");
}

/// An app-made picture in an empty cell, saved as SAM: nothing is lost, so
/// no list. (The read-back keeps a cell record the in-memory cell lacks:
/// `drop_derived_values` skips cells with a frame.)
#[test]
fn a_picture_in_an_empty_cell_saves_without_a_loss_list() {
    let mut s = Scenario::new_document("pictures-cell-save");
    s.clean_folder();
    s.type_text("Cells with pictures");
    make_table(&mut s);
    let frame = *s.document().frame_stack().last().unwrap();
    let first = cell(s.document(), frame, 0, 0);
    s.caret(first, 0, 0, 1, false);
    let picture = bmp_file(&s, "cellpic.bmp");
    s.edit("Insert Image in the empty cell", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::ImageChosen {
            session_id,
            result: FileDialogResult::Selected(picture),
            link: false,
        });
    });
    s.save_sam("CELLPIC3.SAM");
}

// ----- Drawings (§33) -----

pub(super) fn objects(s: &Scenario, frame: FrameId) -> Vec<amipro_pictures::sdw::Object> {
    drawing_of(s.document(), frame)
        .unwrap_or_else(|| s.fail("drawing", "the frame holds no drawing"))
        .objects
}

/// A shape drawn with a tool by a press and a release (an editing step).
pub(super) fn draw_shape(s: &mut Scenario, tool: Tool, from: (f64, f64), to: (f64, f64)) {
    s.act("choose a tool", move |rt| {
        rt.send(Message::DrawTool(tool));
    });
    let keys = Keys::default();
    s.edit(&format!("draw with {tool:?}"), move |rt| {
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
fn a_drawing_made_in_draw_mode_styled_moved_grouped_deleted_and_saved() {
    let mut s = Scenario::new_document("pictures-drawing");
    s.clean_folder();
    s.type_text("A drawing follows.");
    s.enter();
    s.type_text("Text after the drawing.");
    let main = s.main_flow();
    s.caret(main, 0, "A drawing follows.".len(), 1, false);

    // Insert › Drawing… makes the frame and opens Draw mode in place.
    s.command(CommandId::Drawing);
    // (The checker's Undo/Redo of the step leaves Draw mode open again or
    // not; make sure it is open now.)
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
    assert!(objects(&s, frame).is_empty());

    draw_shape(&mut s, Tool::Rectangle, (200.0, 200.0), (1_200.0, 800.0));
    assert!(matches!(
        objects(&s, frame)[0].shape,
        Shape::Rectangle { .. }
    ));
    draw_shape(&mut s, Tool::Ellipse, (1_400.0, 200.0), (2_200.0, 800.0));
    draw_shape(&mut s, Tool::Line, (200.0, 1_000.0), (2_200.0, 1_200.0));
    draw_shape(
        &mut s,
        Tool::RoundedRectangle,
        (300.0, 1_300.0),
        (1_300.0, 1_900.0),
    );
    assert_eq!(objects(&s, frame).len(), 4);

    // A polygon by clicks, ended by a double-click.
    s.act("choose Polygon", |rt| {
        rt.send(Message::DrawTool(Tool::Polygon));
    });
    let keys = Keys::default();
    s.edit("draw a polygon", move |rt| {
        for point in [(1_500.0, 1_300.0), (2_200.0, 1_300.0), (1_900.0, 1_900.0)] {
            rt.send(Message::Draw(DrawEvent::Press {
                at: point,
                keys,
                double: false,
            }));
            rt.send(Message::Draw(DrawEvent::Release { at: point, keys }));
        }
        rt.send(Message::Draw(DrawEvent::Press {
            at: (1_900.0, 1_900.0),
            keys,
            double: true,
        }));
        rt.send(Message::Draw(DrawEvent::Release {
            at: (1_900.0, 1_900.0),
            keys,
        }));
    });
    assert!(
        objects(&s, frame)
            .iter()
            .any(|object| matches!(object.shape, Shape::Polygon { .. })),
        "a polygon was drawn: {:?}",
        objects(&s, frame)
            .iter()
            .map(|o| o.shape.clone())
            .collect::<Vec<_>>()
    );

    // Select the rectangle by its edge; style it; move it.
    s.act("choose Select", |rt| {
        rt.send(Message::DrawTool(Tool::Select));
    });
    s.no_change("click the rectangle's edge", move |rt| {
        rt.send(Message::Draw(DrawEvent::Press {
            at: (200.0, 500.0),
            keys,
            double: false,
        }));
        rt.send(Message::Draw(DrawEvent::Release {
            at: (200.0, 500.0),
            keys,
        }));
    });
    assert_eq!(s.app().draw.as_ref().unwrap().selection.len(), 1);
    s.edit("line colour red", |rt| {
        rt.send(Message::DrawStyle(StyleChange {
            colour: Some(Colour::rgb(255, 0, 0)),
            width: Some(40),
            ..StyleChange::default()
        }));
    });
    s.edit("fill pattern and colour", |rt| {
        rt.send(Message::DrawStyle(StyleChange {
            pattern: Some(9),
            foreground: Some(Colour::rgb(240, 200, 60)),
            ..StyleChange::default()
        }));
    });
    let left_before = objects(&s, frame)[0].rect.left;
    s.edit("drag the rectangle", move |rt| {
        rt.send(Message::Draw(DrawEvent::Press {
            at: (200.0, 500.0),
            keys,
            double: false,
        }));
        rt.send(Message::Draw(DrawEvent::Drag {
            at: (500.0, 500.0),
            keys,
        }));
        rt.send(Message::Draw(DrawEvent::Release {
            at: (500.0, 500.0),
            keys,
        }));
    });
    assert_eq!(objects(&s, frame)[0].rect.left, left_before + 300);
    s.edit("arrow key nudge", |rt| {
        rt.send(Message::Draw(DrawEvent::Key(
            DrawKey::Right,
            Keys::default(),
        )));
    });
    s.command(CommandId::DrawDuplicate);
    assert_eq!(objects(&s, frame).len(), 6);
    s.command(CommandId::DrawRotateLeft);
    s.command(CommandId::DrawFlipHorizontal);
    s.command(CommandId::DrawBack);
    // Delete removes the selection; Undo brings it back.
    s.edit("Delete the duplicate", |rt| {
        rt.send(Message::Draw(DrawEvent::Key(
            DrawKey::Delete,
            Keys::default(),
        )));
    });
    assert_eq!(objects(&s, frame).len(), 5);
    // Select all, group, ungroup.
    s.act("select all", |rt| {
        rt.send(Message::Command(CommandId::DrawSelectAll));
    });
    s.command(CommandId::DrawGroup);
    s.command(CommandId::DrawUngroup);
    s.command(CommandId::DrawAlignLeft);
    s.command_act(CommandId::DrawSnap);

    // A text object typed in place.
    s.act("choose Text", |rt| {
        rt.send(Message::DrawTool(Tool::Text));
    });
    s.edit("place text", move |rt| {
        rt.send(Message::Draw(DrawEvent::Press {
            at: (600.0, 2_100.0),
            keys,
            double: false,
        }));
        rt.send(Message::Draw(DrawEvent::Release {
            at: (600.0, 2_100.0),
            keys,
        }));
        rt.send(Message::Draw(DrawEvent::Text("Label".into())));
    });
    assert!(
        objects(&s, frame)
            .iter()
            .any(|object| matches!(object.shape, Shape::Text(_)))
    );

    // Leave Draw mode; type around the drawing's frame.
    s.command_act(CommandId::DrawDone);
    assert!(s.app().draw.is_none());
    s.caret_to_end();
    s.type_text(" The end.");
    s.caret(main, 0, 0, 1, false);
    s.type_text("1. ");

    let path = s.save_sam("DRAW.SAM");
    let mut s = s.reopen(&path);
    let frames = picture_frames(&s);
    assert_eq!(frames.len(), 1);
    let drawn = objects(&s, frames[0]);
    assert!(drawn.len() >= 5, "{} objects read back", drawn.len());

    // Reopen in Draw mode: select the frame, Enter, draw one more, leave.
    select_frame(&mut s, frames[0]);
    s.act("Enter opens the drawing", |rt| {
        rt.send(Message::Frame(FrameAction::EnterText { at: None }));
    });
    if s.app().draw.is_none() {
        s.fail("Enter", "Draw mode did not open on the selected drawing");
    }
    draw_shape(
        &mut s,
        Tool::Ellipse,
        (2_300.0, 1_900.0),
        (2_800.0, 2_300.0),
    );
    s.command_act(CommandId::DrawDone);
    s.save();
}

#[test]
fn draw_mode_clipboard_import_and_leaving_by_a_click() {
    let mut s = Scenario::new_document("pictures-draw-files");
    s.clean_folder();
    s.type_text("One.");
    s.enter();
    s.type_text("Two.");
    s.command(CommandId::Drawing);
    if s.app().draw.is_none() {
        s.act("open Draw mode again", |rt| {
            rt.send(Message::Command(CommandId::Redo));
        });
    }
    let frame = s.app().draw.as_ref().unwrap().frame;
    draw_shape(&mut s, Tool::Ellipse, (100.0, 100.0), (900.0, 600.0));
    // Copy and Paste of objects.
    s.command_act(CommandId::Copy);
    s.edit("Paste", |rt| {
        rt.send(Message::Command(CommandId::Paste));
    });
    assert_eq!(objects(&s, frame).len(), 2);
    s.command(CommandId::Cut);
    assert_eq!(objects(&s, frame).len(), 1);
    // SVG on the clipboard pastes as shapes; text as a text object.
    s.edit("paste an SVG", |rt| {
        rt.clipboard_data = Some((
            b"<svg xmlns='http://www.w3.org/2000/svg' width='20' height='10'><rect width='20' height='10' fill='#00ff00'/></svg>".to_vec(),
            "image/svg+xml".into(),
        ));
        rt.send(Message::Command(CommandId::Paste));
    });
    s.edit("paste text", |rt| {
        rt.clipboard_data = Some((b"Hello\nWorld".to_vec(), "text/plain;charset=utf-8".into()));
        rt.send(Message::Command(CommandId::Paste));
    });
    // Import Drawing… of an SVG file; Save Drawing As… as SVG and SDW.
    let file = s.folder().join("shape.svg");
    std::fs::write(
        &file,
        "<svg xmlns='http://www.w3.org/2000/svg' width='40' height='40'><circle cx='20' cy='20' r='18' stroke='black' fill='none'/></svg>",
    )
    .unwrap();
    s.edit("Import Drawing…", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::DrawImportChosen {
            session_id,
            result: FileDialogResult::Selected(file),
        });
    });
    for name in ["out.svg", "out.sdw"] {
        let target = s.folder().join(name);
        s.act(&format!("Save Drawing As {name}"), move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::DrawSaveChosen {
                session_id,
                result: FileDialogResult::Selected(target),
            });
        });
        assert!(s.folder().join(name).exists());
    }
    // A click on the text below leaves Draw mode and acts there.
    let below = s.rt.on_page(6_000.0, 6_500.0);
    s.act("click outside the drawing", move |rt| {
        rt.click(cosmic::iced::mouse::Button::Left, below);
    });
    if s.app().draw.is_some() {
        s.fail("click", "Draw mode stayed open after a click outside");
    }
    s.type_text(" tail");
    let path = s.save_sam("DRAWFILES.SAM");
    let s = s.reopen(&path);
    assert_eq!(picture_frames(&s).len(), 1);
}
