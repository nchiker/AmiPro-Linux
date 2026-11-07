//! Frame Properties › Picture (`docs/Design/UI_SPEC.md` §25, decisions P5,
//! P7): Ami Pro's Graphics Scaling (`FRAMESCALEPROC`) in the rail, with
//! where the picture is moved in its frame, Reset, Fit Frame to Picture,
//! and a description. Each change is one command list, one undo step.

use amipro_document::{
    Document, FrameContent, FrameId, Offset, PICTURE_MAX, Picture, PictureFormat, PictureScaling,
    PictureSizing, PictureSource, Rect, Size, Twips,
};
use amipro_document::{Halftone, ImageAdjust};
use amipro_editing::Command;
use amipro_layout::PageBox;

use crate::frame_inspector::{Choice, Field, format_length, parse_length};

/// The Picture section's sizing choices, in Ami Pro's dialog order.
pub const SIZINGS: [PictureSizing; 4] = [
    PictureSizing::Original,
    PictureSizing::FitInFrame,
    PictureSizing::Percentage,
    PictureSizing::Custom,
];
pub const HALFTONES: [Halftone; 5] = Halftone::ALL;
pub const HALFTONE_LABELS: [&str; 5] = [
    "Fastest",
    "Best picture quality",
    "Automatically selected",
    "Posterize",
    "Printer driver",
];
pub const SIZING_LABELS: [&str; 4] = ["Original size", "Fit in frame", "Percentage", "Custom"];

/// What a sizing does, said under the choice: Percentage alone did not
/// say what it is a percentage of (user, 2026-10-18).
pub fn sizing_note(sizing: PictureSizing, keep_aspect: bool) -> &'static str {
    match sizing {
        PictureSizing::Original => "Shown at the size it was made.",
        PictureSizing::FitInFrame if keep_aspect => "As large as the frame allows, in its shape.",
        PictureSizing::FitInFrame => "Stretched to fill the frame.",
        PictureSizing::Percentage => "A percentage of the size it was made: 50 is half.",
        PictureSizing::Custom => "At the width and height below.",
    }
}

/// What the Picture section shows.
#[derive(Clone, Debug, PartialEq)]
pub struct PictureView {
    /// The kind and its natural size, as one line.
    pub about: String,
    /// The linked file, if the picture is linked (P2).
    pub linked: Option<String>,
    pub sizing: PictureSizing,
    pub keep_aspect: bool,
    /// Ami Pro's Rotate is for bitmaps and drawings, not metafiles.
    pub rotatable: bool,
    pub values: Vec<(Field, String)>,
    /// Image Processing (DO.5) for a bitmap a reader takes.
    pub image: Option<ImageView>,
}

/// The Image section (`UI_SPEC.md` §42.3, OD8): the four 0 to 100 settings,
/// Invert, the halftone mode, and what is kept where.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageView {
    pub adjust: ImageAdjust,
    /// The typed boxes' values, one per setting.
    pub values: Vec<(Field, String)>,
    /// Whether the settings are kept in Ami Pro's own form (a gray-scale
    /// TIFF) or are baked into the bitmap by Save As SAM.
    pub kept_by_ami_pro: bool,
    /// Whether anything differs from the neutral state (Revert is offered).
    pub changed: bool,
}

impl ImageView {
    pub fn value(&self, field: Field) -> &str {
        self.values
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map_or("", |(_, value)| value)
    }
}

/// The Image section's slider fields in the order shown, with the label.
pub const IMAGE_SLIDERS: [(Field, &str); 4] = [
    (Field::ImageBrightness, "Brightness"),
    (Field::ImageContrast, "Contrast"),
    (Field::ImageEdge, "Edge enhancement"),
    (Field::ImageSmoothing, "Smoothing"),
];

/// What the section says about saving.
pub const IMAGE_NOTE_AMI_PRO: &str =
    "Kept as Ami Pro keeps it: the TIFF is unchanged and Ami Pro applies these settings.";
pub const IMAGE_NOTE_BAKED: &str =
    "Kept in .amipro files; Save As an Ami Pro document bakes them into the bitmap.";

/// The kinds Image Processing takes: bitmaps.
pub fn is_bitmap(format: PictureFormat) -> bool {
    matches!(
        format,
        PictureFormat::Bitmap
            | PictureFormat::Png
            | PictureFormat::Jpeg
            | PictureFormat::Gif
            | PictureFormat::Tiff
            | PictureFormat::GrayTiff
            | PictureFormat::Pcx
    )
}

/// Whether `bytes` are an 8-bit gray-scale TIFF (what Ami Pro's image
/// module takes as its own, IMV1).
pub fn gray_scale_tiff(bytes: &[u8]) -> bool {
    if !(bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*")) {
        return false;
    }
    let Ok(reader) = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()
    else {
        return false;
    };
    reader
        .into_decoder()
        .is_ok_and(|decoder| image::ImageDecoder::color_type(&decoder) == image::ColorType::L8)
}

/// The adjustment of a picture, from the typed value of one setting.
fn with_setting(adjust: ImageAdjust, field: Field, value: u8) -> ImageAdjust {
    let mut adjust = adjust;
    match field {
        Field::ImageBrightness => adjust.brightness = value,
        Field::ImageContrast => adjust.contrast = value,
        Field::ImageEdge => adjust.edge = value,
        Field::ImageSmoothing => adjust.smoothing = value,
        _ => {}
    }
    adjust
}

/// The command that gives the picture of `frame` `adjust`: a gray-scale
/// TIFF takes Ami Pro's `.tgf` form once it has settings to keep.
fn adjust_commands(
    document: &Document,
    frame: FrameId,
    adjust: ImageAdjust,
) -> Result<Vec<Command>, String> {
    let FrameContent::Picture(picture) =
        &document.frame(frame).ok_or("frame does not exist")?.content
    else {
        return Err("the frame holds no picture".into());
    };
    if !is_bitmap(picture.format()) {
        return Err("Image Processing is for bitmaps".into());
    }
    let gray_tiff = adjust != ImageAdjust::default()
        && !picture.is_gray_tiff()
        && match &picture.source {
            PictureSource::Embedded(object) => document
                .objects()
                .get(object)
                .is_some_and(|object| gray_scale_tiff(&object.bytes)),
            PictureSource::Linked(_) => false,
        };
    Ok(vec![Command::SetImageAdjust {
        frame,
        adjust,
        gray_tiff,
    }])
}

/// The command a slider at `value` makes.
pub fn slide_command(
    document: &Document,
    frame: FrameId,
    field: Field,
    value: u8,
) -> Result<Vec<Command>, String> {
    let FrameContent::Picture(picture) =
        &document.frame(frame).ok_or("frame does not exist")?.content
    else {
        return Err("the frame holds no picture".into());
    };
    adjust_commands(
        document,
        frame,
        with_setting(picture.adjust, field, value.min(100)),
    )
}

impl PictureView {
    pub fn value(&self, field: Field) -> &str {
        self.values
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map_or("", |(_, value)| value)
    }
}

/// The picture of `frame` with its frame's picture area on `page`, twips.
fn picture_and_area<'a>(
    document: &'a Document,
    page: &PageBox,
    frame: FrameId,
) -> Option<(&'a Picture, amipro_layout::Bounds)> {
    let FrameContent::Picture(picture) = &document.frame(frame)?.content else {
        return None;
    };
    let frame_box = page
        .frames
        .iter()
        .find(|frame_box| frame_box.frame == frame)?;
    Some((picture, crate::page_surface::picture_area(frame_box)))
}

/// The size the picture is drawn at now, twips (its stored size when no
/// reader gives its natural size).
fn drawn(picture: &Picture, natural: Option<Size>, area: amipro_layout::Bounds) -> Size {
    let area = Size {
        width: area.width.round() as Twips,
        height: area.height.round() as Twips,
    };
    match natural {
        Some(natural) => picture.scaling.drawn_size(natural, area),
        None if picture.scaling.sizing == PictureSizing::FitInFrame => area,
        None => picture.scaling.size,
    }
}

/// A signed length: a pan may be negative (up or left).
fn parse_signed(text: &str) -> Result<Twips, String> {
    let trimmed = text.trim();
    match trimmed.strip_prefix('-') {
        Some(rest) => parse_length(rest).map(|twips| -twips),
        None => parse_length(trimmed),
    }
}

/// The section for the picture in `frame`, its natural size `natural` when
/// a reader gives one.
pub fn view(
    document: &Document,
    page: &PageBox,
    frame: FrameId,
    natural: Option<Size>,
) -> Option<PictureView> {
    let (picture, area) = picture_and_area(document, page, frame)?;
    let size = drawn(picture, natural, area);
    let format = picture.format();
    let about = match natural {
        Some(natural) => format!(
            "{} · {} × {} in",
            format.name(),
            format_length(natural.width),
            format_length(natural.height)
        ),
        None => format.name().to_owned(),
    };
    let linked = match &picture.source {
        PictureSource::Linked(path) => Some(path.clone()),
        PictureSource::Embedded(_) => None,
    };
    let scaling = picture.scaling;
    // The percentage is always there (typing one chooses Percentage); in
    // the other sizings it says how large the picture is now.
    let percent = match natural {
        Some(natural) if scaling.sizing != PictureSizing::Percentage && natural.width > 0 => {
            ((i64::from(size.width) * 100 + i64::from(natural.width) / 2)
                / i64::from(natural.width))
            .to_string()
        }
        _ => scaling.percent.to_string(),
    };
    Some(PictureView {
        about,
        linked,
        sizing: scaling.sizing,
        keep_aspect: scaling.keep_aspect,
        rotatable: !matches!(
            format,
            PictureFormat::Metafile
                | PictureFormat::Emf
                | PictureFormat::Equation
                | PictureFormat::Eps
                | PictureFormat::Ole
        ),
        values: vec![
            (Field::PicturePercent, percent),
            (Field::PictureWidth, format_length(size.width)),
            (Field::PictureHeight, format_length(size.height)),
            (Field::PictureRotation, scaling.rotation.to_string()),
            (Field::PictureRight, format_length(picture.pan.x)),
            (Field::PictureDown, format_length(picture.pan.y)),
            (Field::PictureDescription, picture.description.clone()),
        ],
        image: (is_bitmap(format) && natural.is_some()).then(|| ImageView {
            adjust: picture.adjust,
            values: vec![
                (
                    Field::ImageBrightness,
                    picture.adjust.brightness.to_string(),
                ),
                (Field::ImageContrast, picture.adjust.contrast.to_string()),
                (Field::ImageEdge, picture.adjust.edge.to_string()),
                (Field::ImageSmoothing, picture.adjust.smoothing.to_string()),
            ],
            kept_by_ami_pro: picture.is_gray_tiff(),
            changed: picture.adjust != ImageAdjust::default(),
        }),
    })
}

fn scaling_command(frame: FrameId, scaling: PictureScaling) -> Vec<Command> {
    vec![Command::SetPictureScaling { frame, scaling }]
}

/// The commands a value typed in the Picture section makes. Typing a
/// percentage selects Percentage, and a width or height Custom, as in Ami
/// Pro's dialog; with the aspect ratio kept, one sets the other.
pub fn text_command(
    document: &Document,
    page: &PageBox,
    frame: FrameId,
    natural: Option<Size>,
    field: Field,
    text: &str,
) -> Result<Vec<Command>, String> {
    let (picture, area) =
        picture_and_area(document, page, frame).ok_or("the frame holds no picture")?;
    let mut scaling = picture.scaling;
    let size = drawn(picture, natural, area);
    match field {
        Field::PicturePercent => {
            // Ami Pro's limit keeps the picture within 22 in (`FRAMESCALEPROC`).
            let longest = natural.map_or(1, |natural| natural.width.max(natural.height).max(1));
            let most = (i64::from(PICTURE_MAX) * 100 / i64::from(longest)).clamp(1, 9_999) as u16;
            let percent: u16 = text
                .trim()
                .trim_end_matches('%')
                .trim()
                .parse()
                .ok()
                .filter(|percent| (1..=most).contains(percent))
                .ok_or_else(|| format!("a percentage from 1 to {most}"))?;
            scaling.sizing = PictureSizing::Percentage;
            scaling.percent = percent;
            Ok(scaling_command(frame, scaling))
        }
        Field::PictureWidth | Field::PictureHeight => {
            let value = parse_length(text)?;
            if value < 1 {
                return Err("a picture is at least 0.01 in".into());
            }
            let (mut width, mut height) = (size.width.max(1), size.height.max(1));
            if field == Field::PictureWidth {
                if scaling.keep_aspect {
                    height = (i64::from(height) * i64::from(value) / i64::from(width)) as Twips;
                }
                width = value;
            } else {
                if scaling.keep_aspect {
                    width = (i64::from(width) * i64::from(value) / i64::from(height)) as Twips;
                }
                height = value;
            }
            scaling.sizing = PictureSizing::Custom;
            scaling.size = Size {
                width: width.clamp(1, PICTURE_MAX),
                height: height.clamp(1, PICTURE_MAX),
            };
            Ok(scaling_command(frame, scaling))
        }
        Field::PictureRotation => {
            let degrees: u16 = text
                .trim()
                .trim_end_matches('°')
                .trim()
                .parse()
                .ok()
                .filter(|degrees| *degrees < 360)
                .ok_or("a rotation from 0 to 359 degrees")?;
            scaling.rotation = degrees;
            Ok(scaling_command(frame, scaling))
        }
        Field::PictureRight | Field::PictureDown => {
            let value = parse_signed(text)?;
            let mut pan = picture.pan;
            if field == Field::PictureRight {
                pan.x = value;
            } else {
                pan.y = value;
            }
            Ok(vec![Command::MovePictureInFrame { frame, pan }])
        }
        Field::PictureDescription => Ok(vec![Command::SetPictureDescription {
            frame,
            description: text.trim().to_owned(),
        }]),
        Field::ImageBrightness
        | Field::ImageContrast
        | Field::ImageEdge
        | Field::ImageSmoothing => {
            let value: u8 = text
                .trim()
                .parse()
                .ok()
                .filter(|value| *value <= 100)
                .ok_or("a whole number from 0 to 100")?;
            slide_command(document, frame, field, value)
        }
        _ => Err("not a picture field".into()),
    }
}

/// The commands a Picture section choice makes.
pub fn choice_command(
    document: &Document,
    page: &PageBox,
    frame: FrameId,
    natural: Option<Size>,
    choice: Choice,
) -> Result<Vec<Command>, String> {
    if choice == Choice::ObjectToPicture {
        return convert_command(document, frame);
    }
    if matches!(
        choice,
        Choice::ImageInvert(_) | Choice::ImageHalftone(_) | Choice::ImageRevert
    ) {
        let FrameContent::Picture(picture) =
            &document.frame(frame).ok_or("frame does not exist")?.content
        else {
            return Err("the frame holds no picture".into());
        };
        let mut adjust = picture.adjust;
        match choice {
            Choice::ImageInvert(on) => adjust.invert = on,
            Choice::ImageHalftone(mode) => adjust.halftone = mode,
            _ => adjust = ImageAdjust::default(),
        }
        return adjust_commands(document, frame, adjust);
    }
    let (picture, area) =
        picture_and_area(document, page, frame).ok_or("the frame holds no picture")?;
    let mut scaling = picture.scaling;
    let size = drawn(picture, natural, area);
    match choice {
        Choice::PictureSizing(sizing) => {
            scaling.sizing = sizing;
            // Custom starts from the size the picture has now, as the
            // dialog's boxes show it.
            scaling.size = Size {
                width: size.width.clamp(1, PICTURE_MAX),
                height: size.height.clamp(1, PICTURE_MAX),
            };
            Ok(scaling_command(frame, scaling))
        }
        Choice::PictureAspect(keep) => {
            scaling.keep_aspect = keep;
            Ok(scaling_command(frame, scaling))
        }
        // Reset (P7): Fit in frame with the aspect ratio kept, 100 %, not
        // turned or moved, as Import Picture leaves a picture.
        Choice::PictureReset => {
            let reset = PictureScaling::imported(scaling.size);
            Ok(vec![
                Command::SetPictureScaling {
                    frame,
                    scaling: reset,
                },
                Command::MovePictureInFrame {
                    frame,
                    pan: Offset::default(),
                },
            ])
        }
        // Fit Frame to Picture (P7): the frame takes the picture's size
        // around it, its margins and corner inset kept, and the picture
        // goes back to the corner.
        Choice::PictureFitFrame => {
            let found = document.frame(frame).ok_or("frame does not exist")?;
            let bounds = page
                .frames
                .iter()
                .find(|frame_box| frame_box.frame == frame)
                .ok_or("the frame is not on this page")?
                .bounds;
            let mut outer = Size {
                width: size.width + (bounds.width - area.width).round() as Twips,
                height: size.height + (bounds.height - area.height).round() as Twips,
            };
            // Twice: the corner inset depends on the frame's size.
            for _ in 0..2 {
                let insets = found.style.text_insets(outer.width, outer.height);
                outer = Size {
                    width: size.width + insets.left + insets.right,
                    height: size.height + insets.top + insets.bottom,
                };
            }
            let left = bounds.x.round() as Twips;
            let top = bounds.y.round() as Twips;
            let mut commands = vec![Command::ResizeFrame {
                frame,
                rect: Rect::new(left, top, left + outer.width, top + outer.height),
            }];
            if picture.pan != Offset::default() {
                commands.push(Command::MovePictureInFrame {
                    frame,
                    pan: Offset::default(),
                });
            }
            // Fit in frame with the aspect kept stays the picture's size.
            if scaling.sizing == PictureSizing::FitInFrame && !scaling.keep_aspect {
                scaling.keep_aspect = true;
                commands.push(Command::SetPictureScaling { frame, scaling });
            }
            Ok(commands)
        }
        _ => Err("not a picture choice".into()),
    }
}

/// What the Object section shows for an OLE frame (`UI_SPEC.md` §42.1,
/// OD7): read-only facts about the stored object. Editing needs the
/// program that made it, which this app does not run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectView {
    /// Label and value rows, in the order shown.
    pub rows: Vec<(&'static str, String)>,
    /// Whether Convert to Picture is offered: an embedded object whose
    /// presentation metafile reads.
    pub convertible: bool,
}

/// The line under the rows (OD7).
pub const OBJECT_CAPTION: &str = "Editing needs the program that made it.";
/// What Convert to Picture does, said beside it.
pub const CONVERT_NOTE: &str = "The object's own data is dropped; Undo brings it back.";

/// `n` with thousands separators.
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The Object section of `frame`: `None` unless it holds an OLE object
/// stored in the document.
pub fn object_view(document: &Document, frame: FrameId) -> Option<ObjectView> {
    let FrameContent::Picture(picture) = &document.frame(frame)?.content else {
        return None;
    };
    if picture.format() != PictureFormat::Ole {
        return None;
    }
    let bytes = match &picture.source {
        PictureSource::Embedded(object) => Some(&document.objects().get(object)?.bytes),
        PictureSource::Linked(_) => None,
    };
    let mut rows: Vec<(&'static str, String)> = Vec::new();
    let Some(object) = bytes.and_then(|bytes| amipro_pictures::ole::read(bytes)) else {
        rows.push((
            "Object",
            "An OLE object whose stream is not readable".into(),
        ));
        rows.push(("Kept", "Stored as it is and written back as it is".into()));
        return Some(ObjectView {
            rows,
            convertible: false,
        });
    };
    use amipro_pictures::ole::Kind;
    let label = object.label();
    rows.push((
        "Class",
        if label == object.class {
            object.class.clone()
        } else {
            format!("{label} ({})", object.class)
        },
    ));
    rows.push((
        "Kind",
        match object.kind {
            Kind::Embedded => "Embedded in the document",
            Kind::Linked => "Linked to a file",
            Kind::Static => "Static picture",
        }
        .into(),
    ));
    if object.kind == Kind::Linked {
        rows.push(("Topic", object.topic.clone()));
        rows.push(("Item", object.item.clone()));
    }
    if object.native.is_some() {
        rows.push((
            "Native data",
            format!("{} bytes", grouped(object.native_len())),
        ));
    }
    let metafile = object
        .metafile(bytes.map_or(&[][..], |bytes| &bytes[..]))
        .is_some();
    rows.push((
        "Presentation",
        match &object.presentation {
            Some(presentation) if metafile => {
                let twips = |value: u32| {
                    (i64::from((value as i32).unsigned_abs()) * 1_440 / 2_540) as Twips
                };
                format!(
                    "Metafile, {} × {} in",
                    format_length(twips(presentation.width)),
                    format_length(twips(presentation.height))
                )
            }
            Some(presentation) => format!("{}, not drawn by this app", presentation.format),
            None => "None readable: Ami Pro's screen image or a box".into(),
        },
    ));
    if object.trailing > 0 {
        rows.push((
            "Extra data",
            format!(
                "{} bytes after the presentation, kept as they are",
                grouped(object.trailing)
            ),
        ));
    }
    Some(ObjectView {
        rows,
        convertible: metafile && object.kind == Kind::Embedded,
    })
}

/// Convert Object to Picture (OD7b, OD11): the frame's picture becomes
/// the object's presentation metafile, at the presentation's size, one
/// `ReplacePicture`, so one undo step; the frame keeps its scaling.
fn convert_command(document: &Document, frame: FrameId) -> Result<Vec<Command>, String> {
    let FrameContent::Picture(picture) =
        &document.frame(frame).ok_or("frame does not exist")?.content
    else {
        return Err("the frame holds no picture".into());
    };
    let PictureSource::Embedded(object) = &picture.source else {
        return Err("only an embedded object converts".into());
    };
    let bytes = &document
        .objects()
        .get(object)
        .ok_or("the object's bytes are missing")?
        .bytes;
    let (_, natural) = crate::pictures::ole_drawing(bytes)
        .ok_or("the object has no presentation this app can draw")?;
    let stream = amipro_pictures::ole::read(bytes).ok_or("the object's stream is not readable")?;
    let metafile = stream
        .metafile(bytes)
        .ok_or("the object has no presentation metafile")?;
    let placeable =
        amipro_pictures::ole::placeable_metafile(metafile, natural.width, natural.height);
    Ok(vec![Command::ReplacePicture {
        frame,
        picture: amipro_editing::NewPicture {
            source: amipro_editing::NewPictureSource::Embedded(placeable.into()),
            extension: ".wmf".into(),
            size: natural,
        },
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_object_section_reads_the_stream_and_offers_conversion() {
        use crate::pictures::ole_fixture;
        let (document, frames) = ole_fixture::document(&[
            ole_fixture::stream("MSWordArt.2", &[7; 3_104], &[]),
            ole_fixture::stream("123Worksheet", &[7; 20], &[9; 160]),
            b"not an object stream".to_vec(),
        ]);
        let view = object_view(&document, frames[0]).unwrap();
        let row = |view: &ObjectView, label: &str| {
            view.rows
                .iter()
                .find(|(name, _)| *name == label)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(
            row(&view, "Class").as_deref(),
            Some("WordArt (MSWordArt.2)")
        );
        assert_eq!(
            row(&view, "Kind").as_deref(),
            Some("Embedded in the document")
        );
        assert_eq!(row(&view, "Native data").as_deref(), Some("3,104 bytes"));
        assert_eq!(
            row(&view, "Presentation").as_deref(),
            Some("Metafile, 2 × 1 in")
        );
        assert_eq!(row(&view, "Extra data"), None);
        assert!(view.convertible);
        // The trailing bytes are said, and kept.
        let view = object_view(&document, frames[1]).unwrap();
        assert_eq!(
            row(&view, "Extra data").as_deref(),
            Some("160 bytes after the presentation, kept as they are")
        );
        assert_eq!(
            row(&view, "Class").as_deref(),
            Some("1-2-3 Worksheet (123Worksheet)")
        );
        // A stream that is not one says so and offers no conversion.
        let view = object_view(&document, frames[2]).unwrap();
        assert!(!view.convertible);
        assert!(view.rows[0].1.contains("not readable"));
    }

    #[test]
    fn converting_an_object_makes_a_metafile_picture_in_one_command() {
        use crate::pictures::ole_fixture;
        let (document, frames) =
            ole_fixture::document(&[ole_fixture::stream("MSWordArt.2", &[7; 30], &[])]);
        let commands = convert_command(&document, frames[0]).unwrap();
        assert_eq!(commands.len(), 1);
        let applied = amipro_editing::apply(&document, &commands[0]).unwrap();
        let FrameContent::Picture(picture) = &applied.document.frame(frames[0]).unwrap().content
        else {
            panic!("not a picture");
        };
        assert_eq!(picture.format(), PictureFormat::Metafile);
        let PictureSource::Embedded(object) = &picture.source else {
            panic!("not embedded");
        };
        let bytes = &applied.document.objects().get(object).unwrap().bytes;
        let size = amipro_pictures::wmf::placeable_size(bytes).unwrap();
        assert_eq!((size.width, size.height), (2_880, 1_440));
        assert!(amipro_pictures::wmf::play(bytes).is_some());
        // The object's own bytes are gone from the document.
        assert_eq!(applied.document.objects().len(), 1);
        assert!(object_view(&applied.document, frames[0]).is_none());
        // A stream that is not readable converts to nothing.
        let (bad, frames) = ole_fixture::document(&[b"junk".to_vec()]);
        assert!(convert_command(&bad, frames[0]).is_err());
    }

    use amipro_document::{FrameStyle, LegacyData, Placement};
    use amipro_layout::{Bounds, FrameBox, FrameBoxKind};

    fn fixture() -> (Document, FrameId, PageBox) {
        let mut document = Document::from_plain_text("Text");
        let object = document.add_object(std::sync::Arc::from(&b"BM"[..]));
        let frame = document.allocate_frame_id();
        document.insert_frame(amipro_document::Frame {
            id: frame,
            placement: Placement::Fixed {
                page: 1,
                rect: Rect::new(1_440, 1_440, 2_880, 2_304),
            },
            style: FrameStyle::default(),
            group: None,
            content: FrameContent::Picture(Picture::new(
                PictureSource::Embedded(object),
                ".bmp",
                Size {
                    width: 816,
                    height: 544,
                },
            )),
            legacy: LegacyData::default(),
        });
        document.frame_stack_mut().push(frame);
        let page = PageBox {
            number: 1,
            shown: 1,
            layout: document.layouts()[0].id,
            side: amipro_document::PageSide::Right,
            width: 12_240.0,
            height: 15_840.0,
            columns: Vec::new(),
            frames: vec![FrameBox {
                frame,
                bounds: Bounds {
                    x: 1_440.0,
                    y: 1_440.0,
                    width: 1_440.0,
                    height: 864.0,
                },
                // Inside the default frame's 0.10 in margins.
                interior: Bounds {
                    x: 1_584.0,
                    y: 1_584.0,
                    width: 1_152.0,
                    height: 576.0,
                },
                strokes: Vec::new(),
                layer: amipro_layout::Layer::Page,
                kind: FrameBoxKind::Picture,
            }],
            lines: Vec::new(),
            strokes: Vec::new(),
            footnote_separator: None,
        };
        (document, frame, page)
    }

    const NATURAL: Size = Size {
        width: 1_800,
        height: 1_200,
    };

    /// PI1's numbers: a 1,800 × 1,200 bitmap fitted with its aspect kept in
    /// a 1,152 × 576 area (0.10 in margins) shows 864 × 576.
    #[test]
    fn the_picture_section_shows_the_scaling_as_ami_pros_dialog() {
        let (document, frame, page) = fixture();
        let view = view(&document, &page, frame, Some(NATURAL)).unwrap();
        assert_eq!(view.about, "Windows Bitmap · 1.25 × 0.83 in");
        assert_eq!(view.sizing, PictureSizing::FitInFrame);
        assert_eq!(view.value(Field::PictureWidth), "0.6");
        assert_eq!(view.value(Field::PictureHeight), "0.4");
        // Fitted at 864 of 1,800 twips: 48 % of its size, said in the
        // percentage that is always shown.
        assert_eq!(view.value(Field::PicturePercent), "48");
        assert!(view.rotatable);
        assert_eq!(
            sizing_note(PictureSizing::Percentage, true),
            "A percentage of the size it was made: 50 is half."
        );
        assert_eq!(
            sizing_note(PictureSizing::FitInFrame, false),
            "Stretched to fill the frame."
        );
    }

    /// Typing a width selects Custom and, with the aspect kept, sets the
    /// height (`FRAMESCALEPROC`); a percentage past 22 in, a rotation of
    /// 360, and a width of 0 are refused.
    #[test]
    fn typed_values_scale_rotate_and_move_the_picture() {
        let (document, frame, page) = fixture();
        let command =
            |field, text| text_command(&document, &page, frame, Some(NATURAL), field, text);
        let [Command::SetPictureScaling { scaling, .. }] = command(Field::PictureWidth, "1.2")
            .unwrap()
            .as_slice()
            .to_owned()[..]
        else {
            panic!("one scaling");
        };
        assert_eq!(scaling.sizing, PictureSizing::Custom);
        assert_eq!(
            scaling.size,
            Size {
                width: 1_728,
                height: 1_152
            }
        );
        assert!(
            command(Field::PicturePercent, "2000").is_err(),
            "past 22 in"
        );
        assert!(command(Field::PicturePercent, "50").is_ok());
        assert!(command(Field::PictureRotation, "360").is_err());
        assert!(command(Field::PictureWidth, "0").is_err());
        assert_eq!(
            command(Field::PictureRight, "-0.05").unwrap(),
            [Command::MovePictureInFrame {
                frame,
                pan: Offset { x: -72, y: 0 }
            }]
        );
    }

    /// Reset and Fit Frame to Picture: each one undo step of its commands.
    #[test]
    fn reset_and_fit_frame_to_picture() {
        let (document, frame, page) = fixture();
        let choose = |choice| choice_command(&document, &page, frame, Some(NATURAL), choice);
        let reset = choose(Choice::PictureReset).unwrap();
        assert!(matches!(reset[0], Command::SetPictureScaling { .. }));
        let fit = choose(Choice::PictureFitFrame).unwrap();
        let Command::ResizeFrame { rect, .. } = fit[0] else {
            panic!("{fit:?}");
        };
        // The picture (864 × 576) and the frame's 0.10 in margins.
        assert_eq!((rect.width(), rect.height()), (864 + 288, 576 + 288));
    }
}
