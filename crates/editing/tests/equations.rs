//! Equations in documents (`docs/OBJECTS_RUNBOOK.md`, DO.9): an equation
//! frame holds Ami Pro's two text lines, is written as an Ami Pro document
//! with its size and baseline in the picture's words and read back, and is
//! kept by the native format. The twin for Ami Pro (EQV1: does it open and
//! repaint what the app wrote?) is written when `AMIPRO_TWIN_OUT` names a
//! folder.

use amipro_document::{
    Document, FrameContent, FrameId, FrameStyle, PictureFormat, Placement, Rect, Size,
};
use amipro_editing::{Command, NewFrameContent, apply};
use amipro_formats::{SamOptions, import_sam, sam_document, sam_losses};
use amipro_pictures::equation::Stored;

const OPTIONS: SamOptions = SamOptions::at(1_791_958_088);

/// A document with one equation frame per text, one under the other.
fn with_equations(texts: &[&str]) -> (Document, Vec<FrameId>) {
    let mut document = Document::from_plain_text("Equations follow.");
    let mut frames = Vec::new();
    for (index, tex) in texts.iter().enumerate() {
        let stored = Stored::new(tex);
        let layout = stored.layout().expect("lays out");
        let top = 1_296 + index as i32 * 700;
        let bytes: std::sync::Arc<[u8]> = stored.bytes().into();
        let natural = Size {
            width: layout.width,
            height: layout.height,
        };
        let mut style = FrameStyle::default();
        style.border.sides = amipro_document::Sides::NONE;
        style.interior.margins = amipro_document::Margins::uniform(0);
        let made = apply(
            &document,
            &Command::CreateFrame {
                placement: Placement::Fixed {
                    page: 1,
                    rect: Rect::new(1_296, top, 1_296 + natural.width, top + natural.height),
                },
                style,
                content: NewFrameContent::EmbeddedPicture {
                    bytes: bytes.clone(),
                    extension: ".tex".into(),
                },
                anchor: None,
                layout: None,
            },
        )
        .unwrap();
        let frame = *made.report.frames.iter().next().unwrap();
        let set = apply(
            &made.document,
            &Command::SetEquation {
                frame,
                bytes,
                natural,
                baseline: layout.baseline,
                fit_frame: true,
            },
        )
        .unwrap();
        document = set.document;
        frames.push(frame);
    }
    (document, frames)
}

fn stored_of(document: &Document, frame: FrameId) -> Stored {
    let FrameContent::Picture(picture) = &document.frame(frame).unwrap().content else {
        panic!("a picture");
    };
    let amipro_document::PictureSource::Embedded(object) = &picture.source else {
        panic!("embedded");
    };
    Stored::read(&document.objects().get(object).unwrap().bytes).unwrap()
}

const TEXTS: [&str; 8] = [
    "$\\frac ab$",
    "$\\sqrt{x}$",
    "$x_2^2$",
    "$\\sum_a^b$",
    "$\\int_0^1$ f(x) dx",
    "$\\matrix a & b \\\\ c & d\\endmatrix $",
    "$\\left[ x\\right] $",
    "x$_i^2$",
];

#[test]
fn an_equation_is_written_for_ami_pro_and_read_back_byte_for_byte() {
    let (document, frames) = with_equations(&TEXTS);
    for (frame, tex) in frames.iter().zip(TEXTS) {
        let FrameContent::Picture(picture) = &document.frame(*frame).unwrap().content else {
            panic!();
        };
        assert_eq!(picture.format(), PictureFormat::Equation);
        assert_eq!(stored_of(&document, *frame).tex, tex);
        let layout = stored_of(&document, *frame).layout().unwrap();
        // The size is the scaling's, the baseline the pan's (`[isd]` words).
        assert_eq!(
            (picture.scaling.size.width, picture.scaling.size.height),
            (layout.width, layout.height)
        );
        assert_eq!(picture.pan.y, layout.baseline);
        assert_eq!(picture.placed_pan(), amipro_document::Offset::default());
        let size = document.frame(*frame).unwrap().placement.size().unwrap();
        assert_eq!((size.width, size.height), (layout.width, layout.height));
    }
    let written = sam_document(&document, &OPTIONS);
    let text = String::from_utf8_lossy(&written);
    // The words as Ami Pro wrote them: `.tex`, width, negative height,
    // percent, rotation, an empty state, the type, pan x, negative baseline.
    let first = stored_of(&document, frames[0]).layout().unwrap();
    let words = format!(
        "\t\t.tex\r\n\t\t.X1\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t{}\r\n\t\t{}\r\n\t\t100\r\n\t\t0\r\n\t\t0\r\n\t\t.tex\r\n\t\t0\r\n\t\t{}\r\n\t\t0\r\n",
        first.width,
        65_536 - first.height,
        65_536 - first.baseline
    );
    let at = text.find("[isd]").expect("an [isd]");
    assert!(
        text.contains(&words),
        "the [isd] words of the first equation: {:?} in {:?}",
        words,
        &text[at..at + 260]
    );
    assert!(sam_losses(&document, &OPTIONS).unwrap().is_empty());

    let copy = import_sam(&written, None, "Equations").unwrap().document;
    let copied: Vec<FrameId> = copy
        .frame_stack()
        .iter()
        .copied()
        .filter(|id| {
            matches!(&copy.frame(*id).unwrap().content,
                FrameContent::Picture(p) if p.format() == PictureFormat::Equation)
        })
        .collect();
    assert_eq!(copied.len(), TEXTS.len());
    for (frame, tex) in copied.iter().zip(TEXTS) {
        let read = stored_of(&copy, *frame);
        assert_eq!(read.tex, tex);
        assert_eq!(read.settings, "TimesNewRomanPS,18,12,0,0,0,0,0");
    }
    // Written again, the equations are the same.
    let again = import_sam(&sam_document(&copy, &OPTIONS), None, "Equations")
        .unwrap()
        .document;
    let texts: Vec<String> = again
        .frame_stack()
        .iter()
        .filter(|id| {
            matches!(&again.frame(**id).unwrap().content,
                FrameContent::Picture(p) if p.format() == PictureFormat::Equation)
        })
        .map(|id| stored_of(&again, *id).tex)
        .collect();
    assert_eq!(texts, TEXTS);

    // For the VM (EQV1): Ami Pro opens the written document.
    if let Some(out) = std::env::var_os("AMIPRO_TWIN_OUT") {
        std::fs::write(std::path::Path::new(&out).join("EQT2.SAM"), &written).unwrap();
    }
}

#[test]
fn the_native_format_keeps_the_equation() {
    let (document, frames) = with_equations(&TEXTS[..3]);
    let encoded = document.encode_native().unwrap();
    let decoded = Document::decode_native(&encoded).unwrap();
    let decoded_frames: Vec<FrameId> = decoded
        .frame_stack()
        .iter()
        .copied()
        .filter(|id| {
            matches!(&decoded.frame(*id).unwrap().content,
                FrameContent::Picture(p) if p.format() == PictureFormat::Equation)
        })
        .collect();
    assert_eq!(decoded_frames.len(), 3);
    for (before, after) in frames.iter().zip(&decoded_frames) {
        assert_eq!(stored_of(&decoded, *after), stored_of(&document, *before));
    }
}

#[test]
fn set_equation_refuses_what_is_not_an_equation() {
    let document = Document::from_plain_text("x");
    let made = apply(
        &document,
        &Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: Rect::new(1_296, 1_296, 2_296, 2_296),
            },
            style: FrameStyle::default(),
            content: NewFrameContent::EmbeddedPicture {
                bytes: b"BM".to_vec().into(),
                extension: ".bmp".into(),
            },
            anchor: None,
            layout: None,
        },
    )
    .unwrap();
    let frame = *made.report.frames.iter().next().unwrap();
    let stored = Stored::new("$x$");
    let result = apply(
        &made.document,
        &Command::SetEquation {
            frame,
            bytes: stored.bytes().into(),
            natural: Size {
                width: 138,
                height: 257,
            },
            baseline: 192,
            fit_frame: false,
        },
    );
    assert!(result.is_err());
}
