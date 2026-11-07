//! Drawings (`docs/Design/UI_SPEC.md` §33; `docs/DRAWING_RUNBOOK.md`,
//! DR.5): Draw mode's operations on a drawing's objects, and a drawing
//! made and edited by commands, written as an Ami Pro document and read
//! back with its objects, no driver state, and no stale screen image
//! (DRW0).

use std::sync::Arc;

use amipro_document::{
    Document, FrameContent, FrameStyle, PictureSizing, PictureSource, Placement, Size,
};
use amipro_editing::drawing::{self, Align, Order, StyleChange};
use amipro_editing::{Command, NewFrameContent, TextPosition, apply};
use amipro_formats::{SamOptions, import_sam, sam_document, sam_losses};
use amipro_pictures::sdw::{self, Colour, Drawing, Fill, Object, Pen, Point, Rect, Shape};

const OPTIONS: SamOptions = SamOptions::at(1_791_958_088);

fn rectangle(left: i16, top: i16, right: i16, bottom: i16) -> Object {
    Object::rectangle(
        Rect::new(left, top, right, bottom),
        Pen::default(),
        Fill::default(),
        false,
    )
}

fn three() -> Drawing {
    Drawing {
        objects: vec![
            rectangle(0, 0, 100, 100),
            rectangle(500, 50, 700, 150),
            rectangle(2000, 0, 2100, 300),
        ],
        ..Drawing::new()
    }
}

fn lefts(drawing: &Drawing) -> Vec<i16> {
    drawing
        .objects
        .iter()
        .map(|object| object.rect.left)
        .collect()
}

#[test]
fn objects_restack_group_align_and_distribute() {
    let mut drawing = three();
    // Bring to Front keeps the chosen ones' own order.
    assert_eq!(drawing::order(&mut drawing, &[0, 1], Order::Front), [1, 2]);
    assert_eq!(lefts(&drawing), [2000, 0, 500]);
    assert_eq!(drawing::order(&mut drawing, &[2], Order::Back), [0]);
    assert_eq!(lefts(&drawing), [500, 2000, 0]);
    assert_eq!(drawing::order(&mut drawing, &[0], Order::Forward), [1]);
    assert_eq!(lefts(&drawing), [2000, 500, 0]);
    assert_eq!(drawing::order(&mut drawing, &[2], Order::Backward), [1]);
    assert_eq!(lefts(&drawing), [2000, 0, 500]);
    // Group where the front one was; Ungroup puts them back.
    let mut drawing = three();
    assert_eq!(drawing::group(&mut drawing, &[0, 2]), Some(1));
    assert_eq!(drawing.objects.len(), 2);
    assert_eq!(drawing.objects[1].rect, Rect::new(0, 0, 2100, 300));
    assert_eq!(drawing::ungroup(&mut drawing, 1), [1, 2]);
    assert_eq!(lefts(&drawing), [500, 0, 2000]);
    // Align tops, then distribute across: equal gaps, the ends staying.
    let mut drawing = three();
    drawing::align(&mut drawing, &[0, 1, 2], Align::Top);
    assert!(drawing.objects.iter().all(|object| object.rect.top == 0));
    drawing::distribute(&mut drawing, &[0, 1, 2], true);
    // 2,100 wide, 400 of objects: two gaps of 850.
    assert_eq!(lefts(&drawing), [0, 950, 2000]);
}

#[test]
fn moves_sizes_flips_turns_and_restyles() {
    let mut drawing = three();
    drawing::move_by(&mut drawing, &[0], 50, 25);
    assert_eq!(drawing.objects[0].rect, Rect::new(50, 25, 150, 125));
    drawing::resize(&mut drawing, &[0, 1], (50.0, 25.0, 1350.0, 325.0));
    assert_eq!(drawing.objects[1].rect, Rect::new(950, 85, 1350, 325));
    // A flip keeps the selection where it is.
    drawing::flip(&mut drawing, &[0, 1], true);
    assert_eq!(
        drawing::bounds(&drawing, &[0, 1]),
        Some(Rect::new(50, 25, 1350, 325))
    );
    assert_eq!(drawing.objects[1].rect.left, 50);
    drawing::rotate(&mut drawing, &[2], 900, None);
    assert_eq!(drawing.objects[2].rect, Rect::new(1900, 100, 2200, 200));
    // Line Style and Fill Pattern; a colour with no fill makes it solid.
    drawing::set_style(
        &mut drawing,
        &[0],
        &StyleChange {
            width: Some(60),
            foreground: Some(Colour::rgb(255, 0, 0)),
            ..StyleChange::default()
        },
    );
    let object = &drawing.objects[0];
    assert_eq!(object.pen().unwrap().width, 60);
    assert_eq!(object.fill().unwrap().pattern, Fill::SOLID);
    // Duplicate puts copies in front.
    assert_eq!(drawing::duplicate(&mut drawing, &[0], 100, 100), [3]);
    assert_eq!(
        drawing.objects[3].rect.left,
        drawing.objects[0].rect.left + 100
    );
    drawing::delete(&mut drawing, &[0, 3]);
    assert_eq!(drawing.objects.len(), 2);
}

#[test]
fn a_drawing_made_and_edited_writes_for_ami_pro_and_reads_back() {
    let document = Document::from_plain_text("One.");
    let main = document.main_flow_id();
    let document = apply(
        &document,
        &Command::CreateFrame {
            placement: Placement::Inline {
                y_offset: 0,
                size: Size {
                    width: 4320,
                    height: 1440,
                },
            },
            style: FrameStyle::default(),
            content: NewFrameContent::EmbeddedPicture {
                bytes: drawing::empty_drawing(),
                extension: ".sdw".into(),
            },
            anchor: Some(TextPosition {
                flow: main,
                paragraph: 0,
                byte: 4,
            }),
            layout: None,
        },
    )
    .unwrap()
    .document;
    let frame = document.frame_stack()[0];
    let FrameContent::Picture(picture) = &document.frame(frame).unwrap().content else {
        panic!();
    };
    // Tools › Drawing leaves a new drawing at its own size (DR1).
    assert_eq!(picture.scaling.sizing, PictureSizing::Original);
    let mut objects = drawing::drawing_of(&document, frame).unwrap();
    assert!(objects.objects.is_empty());
    drawing::add(
        &mut objects,
        vec![
            Object::ellipse(
                Rect::new(100, 100, 1100, 700),
                Pen::default(),
                Fill::solid(Colour::rgb(0, 0, 255)),
                false,
            ),
            Object::line(Point::new(0, 0), Point::new(1200, 0), Pen::default(), true),
        ],
    );
    let applied = apply(
        &document,
        &Command::SetDrawing {
            frame,
            drawing: Arc::new(objects.clone()),
        },
    )
    .unwrap();
    assert!(applied.report.frames.contains(&frame));
    let document = applied.document;
    assert_eq!(drawing::drawing_of(&document, frame).unwrap(), objects);
    // Setting the same objects again changes nothing.
    let same = apply(
        &document,
        &Command::SetDrawing {
            frame,
            drawing: Arc::new(objects.clone()),
        },
    )
    .unwrap();
    assert!(same.report.frames.is_empty());
    let written = sam_document(&document, &OPTIONS);
    let copy = import_sam(&written, None, "Drawn").unwrap().document;
    let copied = copy.frame_stack()[0];
    assert_eq!(drawing::drawing_of(&copy, copied).unwrap(), objects);
    let FrameContent::Picture(read) = &copy.frame(copied).unwrap().content else {
        panic!();
    };
    assert_eq!(read.extension, ".sdw");
    assert_eq!(read.scaling.sizing, PictureSizing::Original);
    assert!(read.rendering.is_none());
    assert!(matches!(read.source, PictureSource::Embedded(_)));
    // The [isd] record: no driver state (count 0) after the scaling values.
    let text = String::from_utf8_lossy(&written);
    let isd: Vec<&str> = text
        .split("[isd]\r\n")
        .nth(1)
        .unwrap()
        .lines()
        .map(str::trim)
        .take(13)
        .collect();
    assert_eq!(&isd[..4], [".X1", ".sdw", ".X1", "1"]);
    assert_eq!(isd[11], "0");
    assert!(sam_losses(&document, &OPTIONS).unwrap().is_empty());
    // A frame holding no drawing refuses the command.
    let text_frame = apply(
        &copy,
        &Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: amipro_document::Rect::new(1440, 1440, 2880, 2880),
            },
            style: FrameStyle::default(),
            content: NewFrameContent::Text,
            anchor: None,
            layout: None,
        },
    )
    .unwrap()
    .document;
    let made = *text_frame.frame_stack().last().unwrap();
    assert!(
        apply(
            &text_frame,
            &Command::SetDrawing {
                frame: made,
                drawing: Arc::new(Drawing::new()),
            },
        )
        .is_err()
    );
    let _ = Shape::Other(Vec::new());
    let _ = sdw::VERSION;
}
