//! Image Processing's state through the SAM writer and back (DO.5, OD8,
//! notes 12.3): a gray-scale TIFF frame (`.tgf`) keeps the TIFF's bytes and
//! the adjustments in `[isd]`, written as Ami Pro writes them.

use std::sync::Arc;

use amipro_document::{
    Document, Frame, FrameContent, FrameId, FrameStyle, Halftone, ImageAdjust, LegacyData, Picture,
    PictureSource, Placement, Rect, Size,
};
use amipro_formats::{SamOptions, import_sam, sam_document};

const OPTIONS: SamOptions = SamOptions::at(0);

fn gray_tiff_document(adjust: ImageAdjust) -> (Document, FrameId) {
    let mut document = Document::from_plain_text("An image follows.");
    let source = PictureSource::Embedded(document.add_object(Arc::from(&b"II*\0 tiff bytes"[..])));
    let mut picture = Picture::new(
        source,
        ".tgf",
        Size {
            width: 4_320,
            height: 4_320,
        },
    );
    picture.adjust = adjust;
    let frame = document.allocate_frame_id();
    document.insert_frame(Frame {
        id: frame,
        placement: Placement::Fixed {
            page: 1,
            rect: Rect::new(1_440, 1_440, 5_760, 5_760),
        },
        style: FrameStyle::default(),
        group: None,
        content: FrameContent::Picture(picture),
        legacy: LegacyData::default(),
    });
    document
        .frame_mut(frame)
        .unwrap()
        .legacy
        .push("frame.field13", "3");
    document.frame_stack_mut().push(frame);
    document.validate().unwrap();
    (document, frame)
}

fn isd_values(written: &str) -> Vec<String> {
    let start = written.find("\t[isd]\r\n").expect("an [isd] record") + "\t[isd]\r\n".len();
    written[start..]
        .split("\r\n")
        .take_while(|line| line.starts_with("\t\t"))
        .map(|line| line.trim().to_owned())
        .collect()
}

fn picture_of(document: &Document, frame: FrameId) -> Picture {
    let FrameContent::Picture(picture) = &document.frame(frame).unwrap().content else {
        panic!("no picture");
    };
    picture.clone()
}

#[test]
fn the_state_words_are_amipros_for_the_default_and_each_control() {
    // IMV1A..L (brightness 49, contrast 50 for a full-range image).
    let words = |adjust: ImageAdjust| {
        adjust
            .tgf_words()
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
    };
    let base = ImageAdjust {
        brightness: 49,
        ..ImageAdjust::default()
    };
    let expected = words(base);
    assert_eq!(expected[..2], ["56283", "8"]);
    assert_eq!(&expected[expected.len() - 8..], ["0"; 8]);
    // Brightness 100 changes exactly the brightness byte (IMV1B).
    let hundred = ImageAdjust {
        brightness: 100,
        ..base
    };
    assert_ne!(words(hundred), expected);
    assert_eq!(
        ImageAdjust::from_tgf_words(&hundred.tgf_words()),
        Some(hundred)
    );
    for halftone in Halftone::ALL {
        let adjust = ImageAdjust { halftone, ..base };
        assert_eq!(
            ImageAdjust::from_tgf_words(&adjust.tgf_words()),
            Some(adjust)
        );
    }
}

#[test]
fn a_gray_tiff_frame_round_trips_its_adjustments() {
    let adjust = ImageAdjust {
        brightness: 30,
        contrast: 70,
        edge: 12,
        smoothing: 34,
        invert: true,
        halftone: Halftone::Posterize,
    };
    let (document, _) = gray_tiff_document(adjust);
    let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
    let values = isd_values(&written);
    assert_eq!(values[1], ".tgf");
    assert_eq!(values[11], "32");
    assert_eq!(values[12 + 32], ".tgf");
    let read = import_sam(written.as_bytes(), None, "Back")
        .unwrap()
        .document;
    let frame = read.frame_stack()[0];
    let picture = picture_of(&read, frame);
    assert_eq!(picture.adjust, adjust);
    assert!(picture.is_gray_tiff());
    let again = String::from_utf8(sam_document(&read, &OPTIONS)).unwrap();
    assert_eq!(isd_values(&again), values);
}

/// The VM's IMV1 files hold the oracle (outside Git); with them absent the
/// test has nothing to compare.
#[test]
fn the_written_state_equals_the_vms_for_the_same_settings() {
    let folder = std::path::Path::new("../../../AmiPro-Reference/vm-lab/OUT/objects");
    // (file, brightness, contrast, edge, smoothing, invert, halftone)
    let cases = [
        ("IMV1A", 49, 50, 0, 0, false, 1),
        ("IMV1B", 100, 50, 0, 0, false, 1),
        ("IMV1C", 0, 50, 0, 0, false, 1),
        ("IMV1D", 49, 100, 0, 0, false, 1),
        ("IMV1E", 49, 0, 0, 0, false, 1),
        ("IMV1F", 49, 50, 100, 0, false, 1),
        ("IMV1G", 49, 50, 0, 100, false, 1),
        ("IMV1H", 49, 50, 0, 0, true, 1),
        ("IMV1I", 30, 70, 0, 0, false, 1),
        ("IMV1J", 49, 50, 0, 0, false, 4),
        ("IMV1K", 49, 50, 0, 0, false, 5),
        ("IMV1L", 49, 50, 0, 0, false, 2),
    ];
    let mut compared = 0;
    for (name, brightness, contrast, edge, smoothing, invert, halftone) in cases {
        let Ok(vm) = std::fs::read(folder.join(format!("{name}.SAM"))) else {
            continue;
        };
        let vm = String::from_utf8_lossy(&vm).replace('\r', "");
        let start = vm.find("\t[isd]\n").unwrap() + "\t[isd]\n".len();
        let vm: Vec<&str> = vm[start..]
            .split('\n')
            .take_while(|line| line.starts_with("\t\t"))
            .map(str::trim)
            .collect();
        let adjust = ImageAdjust {
            brightness,
            contrast,
            edge,
            smoothing,
            invert,
            halftone: Halftone::from_code(halftone).unwrap(),
        };
        // Ami Pro's state words, after the count, equal ours.
        let ours: Vec<String> = adjust.tgf_words().iter().map(u16::to_string).collect();
        assert_eq!(vm[12..12 + 32], ours[..], "{name}");
        compared += 1;
        // And the frame as Ami Pro wrote it reads to those settings.
        let (document, _) = gray_tiff_document(adjust);
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert_eq!(isd_values(&written)[11..12 + 33], vm[11..12 + 33], "{name}");
    }
    eprintln!(
        "compared with the VM's files: {compared} of {}",
        cases.len()
    );
}
