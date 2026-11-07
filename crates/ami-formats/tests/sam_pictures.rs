//! Pictures through the SAM writer and back (`docs/Design/UI_SPEC.md` §25,
//! `docs/research/PICTURE_NOTES.md`): a picture Ami Pro stored is written
//! as read while unchanged; a changed or new one gets Ami Pro's record from
//! the model, keeping the driver's saved state Ami Pro wrote, and without
//! the screen image of the old look; Save As names what Ami Pro cannot
//! hold.

use std::sync::Arc;

use amipro_document::{
    Document, Frame, FrameContent, FrameId, FrameStyle, LegacyData, Offset, Picture,
    PictureScaling, PictureSizing, PictureSource, Placement, Rect, Size,
};
use amipro_formats::{LossKind, SamOptions, import_sam, sam_document, sam_losses};

const OPTIONS: SamOptions = SamOptions::at(0);

/// A document with one Where placed picture frame holding the picture
/// `make` makes in it.
fn with_picture(make: impl FnOnce(&mut Document) -> Picture) -> (Document, FrameId) {
    let mut document = Document::from_plain_text("A picture follows.");
    let picture = make(&mut document);
    let frame = document.allocate_frame_id();
    document.insert_frame(Frame {
        id: frame,
        placement: Placement::Fixed {
            page: 1,
            rect: Rect::new(1_440, 1_440, 2_880, 2_304),
        },
        style: FrameStyle::default(),
        group: None,
        content: FrameContent::Picture(picture),
        legacy: LegacyData::default(),
    });
    // Imported frames keep their number (PI8's is 3).
    document
        .frame_mut(frame)
        .unwrap()
        .legacy
        .push("frame.field13", "3");
    document.frame_stack_mut().push(frame);
    document.validate().unwrap();
    (document, frame)
}

fn embedded(document: &mut Document, bytes: &[u8]) -> PictureSource {
    PictureSource::Embedded(document.add_object(Arc::from(bytes)))
}

/// The values of the written `[isd]` record.
fn isd_values(written: &str) -> Vec<String> {
    let start = written.find("\t[isd]\r\n").expect("an [isd] record") + "\t[isd]\r\n".len();
    written[start..]
        .split("\r\n")
        .take_while(|line| line.starts_with("\t\t"))
        .map(|line| line.trim().to_owned())
        .collect()
}

/// The `[Embedded]` directory's line for the picture.
fn embedded_entry(written: &str) -> String {
    let start = written.rfind("[Embedded]\r\n").unwrap() + "[Embedded]\r\n".len();
    written[start..].split("\r\n").next().unwrap().to_owned()
}

fn picture_of(document: &Document, frame: FrameId) -> &Picture {
    match &document.frame(frame).unwrap().content {
        FrameContent::Picture(picture) => picture,
        _ => panic!("no picture"),
    }
}

/// PI8's record: Custom 1.00 × 0.25 in, moved 936 right and 144 down, and
/// a driver state of two words (shortened from the bitmap driver's 48).
const PI8: [&str; 18] = [
    ".X3", ".bmp", ".X3", "8", "1", "0", "0", "1440", "65176", "100", "0", "2", "7", "9", ".bmp",
    "936", "65392", "0",
];

#[test]
fn a_stored_picture_is_written_as_read_until_it_changes_pi8() {
    let (mut document, frame) = with_picture(|document| {
        let source = embedded(document, b"BM bytes");
        let mut picture = Picture::new(
            source,
            ".bmp",
            Size {
                width: 1,
                height: 1,
            },
        );
        picture.legacy.push("isd", PI8.join("\n"));
        let (scaling, pan) = picture.kept_scaling().unwrap();
        picture.scaling = scaling;
        picture.pan = pan;
        picture
    });
    let rendering = document.add_object(Arc::from(&b"SS screen"[..]));
    let FrameContent::Picture(picture) = &mut document.frame_mut(frame).unwrap().content else {
        unreachable!();
    };
    picture.rendering = Some(rendering);
    let number =
        isd_values(&String::from_utf8(sam_document(&document, &OPTIONS)).unwrap())[0].clone();
    let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
    assert_eq!(isd_values(&written), PI8, "as read");
    assert!(
        !embedded_entry(&written).ends_with(" 0 0 "),
        "the screen image goes with it: {}",
        embedded_entry(&written)
    );
    assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);

    // Moved one step right in picture mode: Ami Pro's record from the
    // model, the driver's state as read, no screen image.
    let FrameContent::Picture(picture) = &mut document.frame_mut(frame).unwrap().content else {
        unreachable!();
    };
    picture.pan.x += 72;
    let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
    let mut expected = PI8.map(str::to_owned).to_vec();
    expected[0] = number.clone();
    expected[2] = number;
    expected[15] = "1008".into();
    assert_eq!(isd_values(&written), expected);
    assert!(
        embedded_entry(&written).ends_with(" 0 0 "),
        "no screen image"
    );
    let back = import_sam(written.as_bytes(), None, "Back")
        .unwrap()
        .document;
    let read = picture_of(&back, back.frame_stack()[0]);
    assert_eq!(read.pan, Offset { x: 1_008, y: 144 });
    assert_eq!(read.scaling, picture_of(&document, frame).scaling);
    assert!(read.kept_is_current());
    assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
}

/// A new picture's record is Ami Pro's with an empty driver state, as
/// PIR1, which Ami Pro opened and drew: `.X3 .bmp .X3 18 1 0 0 816 64992
/// 100 0 0 .bmp 0 0 0`.
#[test]
fn a_new_picture_is_written_as_ami_pro_writes_one_pir1() {
    let (document, _) = with_picture(|document| {
        let source = embedded(document, b"BM bytes");
        let mut picture = Picture::new(
            source,
            ".bmp",
            Size {
                width: 816,
                height: 544,
            },
        );
        picture.scaling = PictureScaling::imported(Size {
            width: 816,
            height: 544,
        });
        picture
    });
    let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
    let values = isd_values(&written);
    let number = &values[0];
    assert!(number.starts_with(".X"));
    assert_eq!(
        values,
        [
            number.as_str(),
            ".bmp",
            number,
            "18",
            "1",
            "0",
            "0",
            "816",
            "64992",
            "100",
            "0",
            "0",
            ".bmp",
            "0",
            "0",
            "0"
        ]
    );
    let back = import_sam(written.as_bytes(), None, "Back")
        .unwrap()
        .document;
    let read = picture_of(&back, back.frame_stack()[0]);
    assert_eq!(read.scaling.sizing, PictureSizing::FitInFrame);
    assert!(read.scaling.keep_aspect);
    assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
}

/// Save As names what Ami Pro cannot hold: a description, a format it
/// cannot read (written as a bitmap, P8), a link to a Linux path (written
/// with a copy, P2). A link Ami Pro wrote is kept as it is.
#[test]
fn save_as_names_descriptions_conversions_and_linux_links() {
    let (mut document, _) = with_picture(|document| {
        let source = embedded(document, b"\x89PNG");
        let mut picture = Picture::new(
            source,
            ".png",
            Size {
                width: 600,
                height: 400,
            },
        );
        picture.description = "The logo".into();
        picture
    });
    let linked = document.allocate_frame_id();
    let linked_picture = |path: &str| {
        FrameContent::Picture(Picture::new(
            PictureSource::Linked(path.into()),
            ".bmp",
            Size {
                width: 600,
                height: 400,
            },
        ))
    };
    document.insert_frame(Frame {
        id: linked,
        placement: Placement::Fixed {
            page: 1,
            rect: Rect::new(4_320, 1_440, 5_760, 2_304),
        },
        style: FrameStyle::default(),
        group: None,
        content: linked_picture("/home/user/Pictures/logo.bmp"),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().push(linked);
    let kinds: Vec<(LossKind, usize)> = sam_losses(&document, &OPTIONS)
        .unwrap()
        .into_iter()
        .map(|loss| (loss.kind, loss.count))
        .collect();
    assert_eq!(
        kinds,
        [
            (LossKind::PictureDescriptions, 1),
            (LossKind::ConvertedPictures, 1),
            (LossKind::LinkedPictures, 1),
        ]
    );
    assert_eq!(
        amipro_formats::Loss {
            kind: LossKind::LinkedPictures,
            count: 1
        }
        .message(),
        "1 picture linked to a file Ami Pro cannot reach, written with a copy of the file"
    );
    // One picture is one bitmap (user, 2026-10-18: "1 picture … written
    // as Windows bitmaps").
    let converted = |count| {
        amipro_formats::Loss {
            kind: LossKind::ConvertedPictures,
            count,
        }
        .message()
    };
    assert_eq!(
        converted(1),
        "1 picture in a format Ami Pro cannot read, written as a Windows bitmap \
         (reduced above 300 dpi)"
    );
    assert_eq!(
        converted(2),
        "2 pictures in a format Ami Pro cannot read, written as Windows bitmaps \
         (reduced above 300 dpi)"
    );
    document.frame_mut(linked).unwrap().content = linked_picture("L:\\IN\\PIC.BMP");
    assert!(
        sam_losses(&document, &OPTIONS)
            .unwrap()
            .iter()
            .all(|loss| loss.kind != LossKind::LinkedPictures)
    );
}

/// Records from older versions of Ami Pro (the corpora's `.G0x` linked
/// metafiles) end otherwise after the rotation and have no pan: they read
/// as not moved and are written back as read.
#[test]
fn an_older_versions_record_is_written_as_read() {
    let older = [
        ".G00", ".wmf", ".G01", "2", "1", "3077", "1620", "4008", "64778", "100", "0", "72", "0",
        "0", "4320", "0", "0", "0", "0", "0",
    ];
    let (document, frame) = with_picture(|_| {
        let mut picture = Picture::new(
            PictureSource::Linked("C:\\AMIPRO\\DOCS\\CHART.G01".into()),
            ".wmf",
            Size {
                width: 1,
                height: 1,
            },
        );
        picture.legacy.push("isd", older.join("\n"));
        let (scaling, pan) = picture.kept_scaling().expect("reads");
        picture.scaling = scaling;
        picture.pan = pan;
        picture
    });
    let picture = picture_of(&document, frame);
    assert_eq!(picture.scaling.sizing, PictureSizing::FitInFrame);
    assert_eq!(picture.pan, Offset::default());
    let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
    assert_eq!(isd_values(&written), older);
    assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
}
