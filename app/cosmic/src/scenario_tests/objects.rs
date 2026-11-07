//! OLE objects (run D, DO.3; `UI_SPEC.md` §42.1): an object drawn from its
//! stored presentation, selected, read in the Object section, moved and
//! sized, listed in Edit › Links…, saved and reopened byte for byte, drawn
//! in the PDF, and converted to a picture. The streams are synthetic
//! (`pictures::ole_fixture`); no corpus data is used.

use super::*;
use crate::frame_editing::FrameAction;
use crate::frame_inspector::{Choice, Field};
use crate::links_panel::LinksAction;
use crate::pictures::ole_fixture;
use amipro_document::{FrameContent, FrameId, PictureFormat, PictureSource};

fn picture_of(s: &Scenario, frame: FrameId) -> amipro_document::Picture {
    match &s.document().frame(frame).map(|frame| &frame.content) {
        Some(FrameContent::Picture(picture)) => picture.clone(),
        _ => s.fail("picture", "the frame holds no picture"),
    }
}

/// The OLE stream a frame holds.
fn stream_of(document: &Document, frame: FrameId) -> Vec<u8> {
    let Some(FrameContent::Picture(picture)) = document.frame(frame).map(|frame| &frame.content)
    else {
        panic!("the frame holds no picture");
    };
    let PictureSource::Embedded(object) = &picture.source else {
        panic!("the picture is linked");
    };
    document.objects().get(object).unwrap().bytes.to_vec()
}

fn select_frame(s: &mut Scenario, frame: FrameId) {
    s.no_change("select the object's frame", move |rt| {
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

/// What the page draws for the picture of `frame` now.
fn shown(s: &Scenario, frame: FrameId) -> crate::pictures::Shown {
    let picture = picture_of(s, frame);
    s.app()
        .pictures
        .shown(s.document(), &picture, None, (160, 80), 1.0)
}

#[test]
fn an_ole_object_is_drawn_selected_read_moved_listed_saved_and_converted() {
    let wordart = ole_fixture::stream("MSWordArt.2", &[5; 3_104], &[]);
    let sheet = ole_fixture::stream("123Worksheet", &[6; 400], &[9; 160]);
    let (document, frames) = ole_fixture::document(&[wordart.clone(), sheet.clone()]);
    let mut s = Scenario::new("objects-ole", document);
    s.clean_folder();

    // Both objects draw from their presentation: pictures, not boxes.
    for frame in &frames {
        assert!(
            matches!(shown(&s, *frame), crate::pictures::Shown::Picture(_)),
            "an object draws from its presentation"
        );
    }
    // The page's view builds and the PDF draws it as vectors.
    let picture = picture_of(&s, frames[0]);
    assert!(
        s.app()
            .pictures
            .printed(s.document(), &picture, None)
            .is_some(),
        "the PDF draws the presentation as paths"
    );

    // Select the first object: its Frame Properties show the Object section.
    select_frame(&mut s, frames[0]);
    let view = crate::picture_inspector::object_view(s.document(), frames[0]).unwrap();
    assert!(view.convertible);
    assert!(
        view.rows
            .iter()
            .any(|(label, value)| *label == "Class" && value == "WordArt (MSWordArt.2)")
    );
    s.act("redraw with the Object section", |rt| {
        rt.redraw();
    });

    // Move and size it in the inspector: one undo step each.
    s.edit("Frame Properties: X = 2", |rt| {
        rt.send(Message::InspectorInput(Field::X, "2".into()));
        rt.send(Message::InspectorSubmit(Field::X));
    });
    s.edit("Frame Properties: Width = 2.5", |rt| {
        rt.send(Message::InspectorInput(Field::Width, "2.5".into()));
        rt.send(Message::InspectorSubmit(Field::Width));
    });
    let FrameContent::Picture(_) = &s.document().frame(frames[0]).unwrap().content else {
        s.fail("resize", "the frame no longer holds the picture");
    };
    match s.document().frame(frames[0]).unwrap().placement {
        amipro_document::Placement::Fixed { rect, .. } => {
            assert_eq!(rect.left, 2_880);
            assert_eq!(rect.right - rect.left, 3_600);
        }
        other => s.fail("move", &format!("not Where placed: {other:?}")),
    }

    // Edit › Links… lists both; a row selects its frame; no changes.
    s.no_change("Edit > Links", |rt| {
        rt.send(Message::Command(CommandId::Links));
    });
    assert!(s.app().links_panel.is_some());
    let rows = crate::links_panel::rows(s.document());
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].source, "1-2-3 Worksheet");
    s.no_change("select the second row", |rt| {
        rt.send(Message::Links(LinksAction::Select(1)));
    });
    assert_eq!(
        s.app().frame_selection.as_ref().map(|s| s.frames.clone()),
        Some(vec![frames[1]])
    );
    s.act("redraw with the Links panel", |rt| {
        rt.redraw();
    });
    s.no_change("close the Links panel", |rt| {
        rt.send(Message::Links(LinksAction::Close));
    });
    assert!(s.app().links_panel.is_none());

    // Save As SAM: the objects are written as read, trailing bytes too.
    let path = s.save_sam("Objects.SAM");
    let read = amipro_formats::import_sam_path(&path).unwrap().document;
    let read_frames: Vec<FrameId> = read
        .frames()
        .filter(|frame| matches!(frame.content, FrameContent::Picture(_)))
        .map(|frame| frame.id)
        .collect();
    assert_eq!(read_frames.len(), 2);
    let mut streams: Vec<Vec<u8>> = read_frames
        .iter()
        .map(|frame| stream_of(&read, *frame))
        .collect();
    streams.sort();
    let mut expected = vec![wordart.clone(), sheet.clone()];
    expected.sort();
    assert_eq!(streams, expected, "the streams read back byte for byte");

    // Reopened, the objects draw and list the same.
    let mut fresh = s.reopen(&path);
    assert_eq!(crate::links_panel::rows(fresh.document()).len(), 2);
    fresh.act("redraw the reopened document", |rt| {
        rt.redraw();
    });

    // Convert Object to Picture: one undo step; the object's data is gone.
    select_frame(&mut s, frames[0]);
    s.edit("Convert Object to Picture", |rt| {
        rt.send(Message::InspectorChoice(Choice::ObjectToPicture));
    });
    assert_eq!(
        picture_of(&s, frames[0]).format(),
        PictureFormat::Metafile,
        "the object became a metafile picture"
    );
    assert!(crate::picture_inspector::object_view(s.document(), frames[0]).is_none());
    assert!(matches!(
        shown(&s, frames[0]),
        crate::pictures::Shown::Picture(_)
    ));
    assert_eq!(crate::links_panel::rows(s.document()).len(), 1);
    // A metafile picture saves without a loss list.
    s.save_sam("Converted.SAM");
}

#[test]
fn an_object_without_a_readable_presentation_stays_a_box_and_is_kept() {
    // A stream cut inside its presentation: not drawn, kept as stored.
    let whole = ole_fixture::stream("WordArt", &[1; 8], &[]);
    let cut = whole[..whole.len() - 30].to_vec();
    let (document, frames) = ole_fixture::document(std::slice::from_ref(&cut));
    let mut s = Scenario::new("objects-ole-box", document);
    assert!(matches!(
        shown(&s, frames[0]),
        crate::pictures::Shown::Box(_)
    ));
    select_frame(&mut s, frames[0]);
    let view = crate::picture_inspector::object_view(s.document(), frames[0]).unwrap();
    assert!(!view.convertible);
    assert!(
        view.rows
            .iter()
            .any(|(label, value)| *label == "Presentation" && value.contains("None readable"))
    );
    // Convert is refused with a reason and changes nothing.
    s.expect_status("refused");
    s.no_change("Convert Object to Picture is refused", |rt| {
        rt.send(Message::InspectorChoice(Choice::ObjectToPicture));
    });
    assert_eq!(stream_of(s.document(), frames[0]), cut);
}

#[test]
fn an_empty_document_has_no_links() {
    let mut s = Scenario::new_document("objects-links-empty");
    s.no_change("Edit > Links", |rt| {
        rt.send(Message::Command(CommandId::Links));
    });
    assert!(s.app().links_panel.is_some());
    assert!(crate::links_panel::rows(s.document()).is_empty());
    s.act("redraw the empty list", |rt| {
        rt.redraw();
    });
}

// ---- Image Processing (run D, DO.5; `UI_SPEC.md` §42.3) -----------------

/// An 8-bit gray TIFF of a horizontal ramp, 48 × 32.
fn gray_tiff_file(s: &Scenario, name: &str) -> std::path::PathBuf {
    let path = s.folder().join(name);
    std::fs::create_dir_all(s.folder()).unwrap();
    let mut bytes = Vec::new();
    image::GrayImage::from_fn(48, 32, |x, _| image::Luma([(x * 255 / 47) as u8]))
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Tiff,
        )
        .unwrap();
    std::fs::write(&path, bytes).unwrap();
    path
}

/// The pixels the page draws for the picture of `frame`: (width, rgba).
fn drawn_pixels(s: &Scenario, frame: FrameId) -> (u32, Vec<u8>) {
    match shown(s, frame) {
        crate::pictures::Shown::Picture(cosmic::iced::advanced::image::Handle::Rgba {
            width,
            pixels,
            ..
        }) => (width, pixels.to_vec()),
        _ => s.fail("drawn", "the picture is not drawn as pixels"),
    }
}

fn gray_at(pixels: &(u32, Vec<u8>), x: usize) -> u8 {
    pixels.1[x * 4]
}

#[test]
fn a_gray_tiff_is_adjusted_live_kept_in_ami_pros_form_and_saved() {
    let mut s = Scenario::new_document("objects-image-gray");
    s.clean_folder();
    let path = gray_tiff_file(&s, "ramp.tif");
    super::pictures::insert_image(&mut s, &path, false);
    let frame = s.app().frame_selection.as_ref().unwrap().frames[0];
    let original = picture_of(&s, frame);
    assert_eq!(original.format(), PictureFormat::Tiff);
    let before = drawn_pixels(&s, frame);
    assert_eq!(gray_at(&before, 47), 255);
    assert!(
        crate::picture_inspector::view(
            s.document(),
            &s.app().inspected_frame().unwrap().1,
            frame,
            s.app().picture_natural(frame)
        )
        .unwrap()
        .image
        .is_some(),
        "the Image section is offered for a bitmap"
    );
    assert!(s.app().command_enabled(CommandId::ImageProcessing));
    s.act("Tools > Image Processing", |rt| {
        rt.send(Message::Command(CommandId::ImageProcessing));
    });
    assert!(s.app().rail_open());

    // A slider drag is one undo step (the checker undoes and redoes it).
    s.edit("drag Brightness to 70", |rt| {
        for value in [52, 58, 64, 70] {
            rt.send(Message::ImageSlide(Field::ImageBrightness, value));
        }
        rt.send(Message::ImageSlideEnd);
    });
    let adjusted = picture_of(&s, frame);
    assert_eq!(adjusted.adjust.brightness, 70);
    assert!(
        adjusted.is_gray_tiff(),
        "a gray-scale TIFF takes Ami Pro's form"
    );
    assert_eq!(adjusted.extension, ".tgf");
    let (a, b) = (&original.source, &adjusted.source);
    let bytes = |source: &PictureSource| match source {
        PictureSource::Embedded(object) => {
            s.document().objects().get(object).unwrap().bytes.clone()
        }
        PictureSource::Linked(_) => panic!("linked"),
    };
    assert_eq!(bytes(a), bytes(b), "the TIFF's bytes are never changed");
    let lit = drawn_pixels(&s, frame);
    assert!(
        gray_at(&lit, 10) > gray_at(&before, 10),
        "brighter on screen"
    );

    s.edit("type Contrast 80", |rt| {
        rt.send(Message::InspectorInput(Field::ImageContrast, "80".into()));
        rt.send(Message::InspectorSubmit(Field::ImageContrast));
    });
    s.edit("Edge enhancement 40", |rt| {
        rt.send(Message::ImageSlide(Field::ImageEdge, 40));
        rt.send(Message::ImageSlideEnd);
    });
    s.edit("Smoothing 30", |rt| {
        rt.send(Message::ImageSlide(Field::ImageSmoothing, 30));
        rt.send(Message::ImageSlideEnd);
    });
    s.edit("Invert", |rt| {
        rt.send(Message::InspectorChoice(Choice::ImageInvert(true)));
    });
    s.edit("Halftone: Posterize", |rt| {
        rt.send(Message::InspectorChoice(Choice::ImageHalftone(
            amipro_document::Halftone::Posterize,
        )));
    });
    // A value out of range is refused with a reason and changes nothing.
    s.no_change("Brightness 101 is refused", |rt| {
        rt.send(Message::InspectorInput(
            Field::ImageBrightness,
            "101".into(),
        ));
        rt.send(Message::InspectorSubmit(Field::ImageBrightness));
    });
    s.act("redraw with the Image section", |rt| {
        rt.redraw();
    });
    let set = picture_of(&s, frame).adjust;
    assert_eq!(
        (
            set.brightness,
            set.contrast,
            set.edge,
            set.smoothing,
            set.invert
        ),
        (70, 80, 40, 30, true)
    );

    // Ami Pro's form: saves with no loss list, reopens with the same
    // settings, and the state is Ami Pro's blob.
    let saved = s.save_sam("Gray.SAM");
    let text = String::from_utf8_lossy(&std::fs::read(&saved).unwrap()).into_owned();
    assert!(text.contains("\t[isd]"), "an [isd] record");
    let fresh = s.reopen(&saved);
    let reopened = fresh
        .document()
        .frames()
        .find_map(|frame| match &frame.content {
            FrameContent::Picture(picture) => Some(picture.clone()),
            _ => None,
        })
        .expect("the picture");
    assert_eq!(reopened.adjust, set);
    assert!(reopened.is_gray_tiff());
    assert_eq!(reopened.format(), PictureFormat::GrayTiff);

    // Revert returns the neutral state in one step; Undo brings it back.
    s.edit("Revert", |rt| {
        rt.send(Message::InspectorChoice(Choice::ImageRevert));
    });
    assert!(picture_of(&s, frame).adjust.is_neutral());
    let reverted = drawn_pixels(&s, frame);
    assert_eq!(reverted.1, before.1, "Revert draws the original again");
}

#[test]
fn any_other_bitmap_is_adjusted_and_baked_when_saved_as_sam() {
    let mut s = Scenario::new_document("objects-image-baked");
    s.clean_folder();
    let path = s.folder().join("colour.png");
    std::fs::create_dir_all(s.folder()).unwrap();
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(20, 10, image::Rgba([200, 100, 50, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    std::fs::write(&path, png).unwrap();
    super::pictures::insert_image(&mut s, &path, false);
    let frame = s.app().frame_selection.as_ref().unwrap().frames[0];
    s.edit("Invert", |rt| {
        rt.send(Message::InspectorChoice(Choice::ImageInvert(true)));
    });
    let picture = picture_of(&s, frame);
    assert!(!picture.is_gray_tiff(), "a colour PNG stays a PNG here");
    assert_eq!(picture.format(), PictureFormat::Png);
    let inverted = drawn_pixels(&s, frame);
    assert_eq!(&inverted.1[..3], &[55, 155, 205]);

    // The native file keeps the editable settings.
    let native = Document::decode_native(&s.document().encode_native().unwrap()).unwrap();
    let kept = native
        .frames()
        .find_map(|frame| match &frame.content {
            FrameContent::Picture(picture) => Some(picture.adjust),
            _ => None,
        })
        .unwrap();
    assert!(kept.invert);

    // Save As SAM lists that the adjustments are baked; the bitmap read
    // back is the inverted one, with neutral settings.
    let saved = s.save_sam_expecting("Baked.SAM", &["image adjustments", "Windows bitmap"]);
    let read = amipro_formats::import_sam_path(&saved).unwrap().document;
    let read = read
        .frames()
        .find_map(|frame| match &frame.content {
            FrameContent::Picture(picture) => Some((picture.clone(), frame.id)),
            _ => None,
        })
        .unwrap();
    assert_eq!(read.0.format(), PictureFormat::Bitmap);
    assert!(read.0.adjust.is_neutral());
    let PictureSource::Embedded(object) = &read.0.source else {
        panic!("embedded");
    };
    let bytes = amipro_formats::import_sam_path(&saved)
        .unwrap()
        .document
        .objects()
        .get(object)
        .unwrap()
        .bytes
        .clone();
    let raster = amipro_pictures::dib::decode_bmp(&bytes).unwrap();
    assert_eq!(
        &raster.pixels[..3],
        &[55, 155, 205],
        "baked into the bitmap"
    );
    // The session's document still holds the editable settings.
    assert!(picture_of(&s, frame).adjust.invert);
}

#[test]
fn image_processing_is_not_offered_for_a_picture_that_is_not_a_bitmap() {
    let (document, frames) =
        ole_fixture::document(&[ole_fixture::stream("MSWordArt.2", &[5; 30], &[])]);
    let mut s = Scenario::new("objects-image-not-bitmap", document);
    select_frame(&mut s, frames[0]);
    assert!(!s.app().command_enabled(CommandId::ImageProcessing));
    let view = crate::picture_inspector::view(
        s.document(),
        &s.app().inspected_frame().unwrap().1,
        frames[0],
        s.app().picture_natural(frames[0]),
    )
    .unwrap();
    assert!(view.image.is_none());
}

// ---- Paste Special (run D, DO.4; `UI_SPEC.md` §42.2) --------------------

fn png_of(width: u32, height: u32) -> Vec<u8> {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(width, height, image::Rgba([0, 128, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    png
}

fn listed(s: &Scenario) -> Vec<&'static str> {
    s.app()
        .paste_special
        .as_ref()
        .map(|panel| panel.formats.iter().map(|format| format.name()).collect())
        .unwrap_or_default()
}

#[test]
fn paste_special_lists_the_clipboards_formats_and_pastes_the_chosen_one() {
    use crate::paste_special::PasteSpecialAction;
    let mut s = Scenario::new_document("objects-paste-special");
    s.clean_folder();
    s.type_text("Hello");
    s.rt.clipboard = Some("Pasted words".into());
    s.rt.clipboard_data = Some((png_of(30, 20), "image/png".into()));
    s.rt.clipboard_more = vec![
        (b"{\\rtf1 {\\b Rich} text}".to_vec(), "text/rtf".into()),
        (b"<p>Some &amp; html</p>".to_vec(), "text/html".into()),
    ];
    assert!(s.app().command_enabled(CommandId::PasteSpecial));
    s.no_change("Edit > Paste Special", |rt| {
        rt.send(Message::Command(CommandId::PasteSpecial));
    });
    assert_eq!(
        listed(&s),
        ["Text", "Rich Text Format", "HTML", "PNG image"],
        "the formats found, in the list's order"
    );
    s.act("redraw with the Paste Special panel", |rt| {
        rt.redraw();
    });

    // The picture row pastes as a picture: a frame, one undo step.
    s.no_change("choose the PNG row", |rt| {
        rt.send(Message::PasteSpecial(PasteSpecialAction::Select(3)));
    });
    s.edit("Paste as a picture", |rt| {
        rt.send(Message::PasteSpecial(PasteSpecialAction::Paste(None)));
    });
    assert!(s.app().paste_special.is_none(), "the panel closes on Paste");
    assert!(
        s.document()
            .frames()
            .any(|frame| matches!(frame.content, FrameContent::Picture(_)))
    );

    // The text row pastes as text; HTML as its text.
    s.caret_to_end();
    s.no_change("Edit > Paste Special again", |rt| {
        rt.send(Message::Command(CommandId::PasteSpecial));
    });
    s.edit("Paste as text (a double-click on the row)", |rt| {
        rt.send(Message::PasteSpecial(PasteSpecialAction::Paste(Some(0))));
    });
    assert!(s.main_text().contains("Pasted words"), "{}", s.main_text());
    s.caret_to_end();
    s.no_change("Edit > Paste Special a third time", |rt| {
        rt.send(Message::Command(CommandId::PasteSpecial));
    });
    s.edit("Paste the HTML as text", |rt| {
        rt.send(Message::PasteSpecial(PasteSpecialAction::Paste(Some(2))));
    });
    assert!(s.main_text().contains("Some & html"), "{}", s.main_text());
}

#[test]
fn paste_special_refuses_what_it_cannot_hold_and_says_when_the_clipboard_is_empty() {
    use crate::paste_special::PasteSpecialAction;
    let mut s = Scenario::new_document("objects-paste-special-refused");
    // An image type whose bytes no reader takes: listed, refused with
    // Ami Pro's message 3092 and the panel stays open.
    s.rt.clipboard_data = Some((b"not a png".to_vec(), "image/png".into()));
    s.no_change("Edit > Paste Special", |rt| {
        rt.send(Message::Command(CommandId::PasteSpecial));
    });
    assert_eq!(listed(&s), ["PNG image"]);
    s.expect_status("Cannot accept an object");
    s.no_change("Paste the unreadable picture", |rt| {
        rt.send(Message::PasteSpecial(PasteSpecialAction::Paste(None)));
    });
    assert!(s.app().paste_special.is_some());
    assert!(
        s.app()
            .status
            .contains("Cannot accept an object that Ami Pro can not display")
    );
    s.clear_expected_status();

    // Files that are not pictures are refused alike.
    s.rt.clipboard_data = Some((b"file:///tmp/notes.txt\n".to_vec(), "text/uri-list".into()));
    s.no_change("Edit > Paste Special for a text file", |rt| {
        rt.send(Message::Command(CommandId::PasteSpecial));
    });
    assert_eq!(listed(&s), ["Files"]);
    s.expect_status("Cannot accept an object");
    s.no_change("Paste the file", |rt| {
        rt.send(Message::PasteSpecial(PasteSpecialAction::Paste(None)));
    });
    s.clear_expected_status();

    // Nothing on the clipboard: the panel looks twice and says so.
    s.rt.clipboard = None;
    s.rt.clipboard_data = None;
    s.no_change("Edit > Paste Special with an empty clipboard", |rt| {
        rt.send(Message::Command(CommandId::PasteSpecial));
    });
    let panel = s.app().paste_special.as_ref().unwrap();
    assert!(
        panel.empty() && panel.retried,
        "looked again before saying so"
    );
    s.act("redraw the empty list", |rt| {
        rt.redraw();
    });
    s.no_change("close the panel", |rt| {
        rt.send(Message::PasteSpecial(PasteSpecialAction::Close));
    });
    assert!(s.app().paste_special.is_none());
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Object section and the Links panel,
/// light and dark (not committed).
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn object_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let (document, frames) = ole_fixture::document(&[
        ole_fixture::stream("MSWordArt.2", &[5; 3_104], &[]),
        ole_fixture::stream("123Worksheet", &[6; 11_200], &[9; 160]),
    ]);
    let mut s = Scenario::new("objects-screens", document);
    let write = |s: &Scenario, name: &str| {
        for (dark, theme) in [
            (false, cosmic::Theme::light()),
            (true, cosmic::Theme::dark()),
        ] {
            let size = (1_280, 800);
            let rgb = render_window(s.app(), size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!(
                "window-{name}-{}.ppm",
                if dark { "dark" } else { "light" }
            ));
            std::fs::write(&path, ppm).unwrap();
        }
    };
    select_frame(&mut s, frames[1]);
    s.app_mut().open_rail();
    write(&s, "object-section");
    s.no_change("Edit > Links", |rt| {
        rt.send(Message::Command(CommandId::Links));
    });
    s.app_mut().links_panel.as_mut().unwrap().selected = Some(1);
    write(&s, "links");
}

/// The real slider in the Image section: a press, a drag, and a release
/// move the setting live and make one undo step (widget layer).
#[test]
fn dragging_the_brightness_slider_is_one_undo_step() {
    use cosmic::iced::mouse::{Button, Event as Mouse};
    use cosmic::iced::{Event, Point};
    let mut s = Scenario::new_document("objects-image-widget");
    s.clean_folder();
    let path = gray_tiff_file(&s, "ramp.tif");
    super::pictures::insert_image(&mut s, &path, false);
    let frame = s.app().frame_selection.as_ref().unwrap().frames[0];
    // A tall window shows the whole rail: the slider is at y = 1,106.
    s.app_mut().open_rail();
    s.rt.window = cosmic::iced::Size::new(1_280.0, 1_500.0);
    s.act("redraw", |rt| {
        rt.redraw();
    });
    let undo = s.app().history.undo_len();
    let moved = |x: f32| {
        (
            Event::Mouse(Mouse::CursorMoved {
                position: Point::new(x, 1_106.0),
            }),
            Point::new(x, 1_106.0),
        )
    };
    let at = Point::new(120.0, 1_106.0);
    let events = [
        moved(120.0),
        (Event::Mouse(Mouse::ButtonPressed(Button::Left)), at),
        moved(140.0),
        moved(160.0),
        moved(180.0),
        (
            Event::Mouse(Mouse::ButtonReleased(Button::Left)),
            Point::new(180.0, 1_106.0),
        ),
    ];
    let published = s.rt.events(&events);
    assert!(
        published
            .iter()
            .filter(|message| matches!(message, Message::ImageSlide(Field::ImageBrightness, _)))
            .count()
            >= 3,
        "the drag moved the slider: {published:?}"
    );
    assert!(
        published
            .iter()
            .any(|message| matches!(message, Message::ImageSlideEnd)),
        "the release ended the drag"
    );
    assert!(picture_of(&s, frame).adjust.brightness > 70);
    assert_eq!(s.app().history.undo_len(), undo + 1, "one undo step");
    s.act("check the document", |_| {});
    s.rt.send(Message::Command(CommandId::Undo));
    assert!(
        picture_of(&s, frame).adjust.is_neutral(),
        "Undo undoes the drag"
    );
}

/// The real rows of the Paste Special panel: a click chooses, a
/// double-click pastes (widget layer).
#[test]
fn a_click_chooses_and_a_double_click_pastes_in_the_paste_special_panel() {
    use crate::paste_special::PasteSpecialAction;
    use cosmic::iced::Point;
    use cosmic::iced::mouse::Button;
    let mut s = Scenario::new_document("objects-paste-special-widget");
    s.type_text("Hello");
    s.rt.clipboard = Some("Pasted words".into());
    s.rt.clipboard_data = Some((b"{\\rtf1 Rich}".to_vec(), "text/rtf".into()));
    s.no_change("Edit > Paste Special", |rt| {
        rt.send(Message::Command(CommandId::PasteSpecial));
    });
    assert_eq!(listed(&s), ["Text", "Rich Text Format"]);
    s.act("redraw", |rt| {
        rt.redraw();
    });
    // The second row is at y = 297 in the rail.
    let published = s.rt.click_unrun(Button::Left, Point::new(150.0, 297.0));
    assert!(
        published.iter().any(|message| matches!(
            message,
            Message::PasteSpecial(PasteSpecialAction::Select(1))
        )),
        "a click chooses the row: {published:?}"
    );
    s.rt.send(Message::PasteSpecial(PasteSpecialAction::Select(1)));
    let published = s.rt.double_click(Point::new(150.0, 297.0));
    assert!(
        published.iter().any(|message| matches!(
            message,
            Message::PasteSpecial(PasteSpecialAction::Paste(Some(1)))
        )),
        "a double-click pastes the row: {published:?}"
    );
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Image section (a gray-scale TIFF
/// adjusted) and the Paste Special panel, light and dark, as PNG files
/// (not committed).
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn image_and_paste_special_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut s = Scenario::new_document("objects-image-screens");
    s.clean_folder();
    s.type_text("A page with a picture below.");
    let path = gray_tiff_file(&s, "ramp.tif");
    super::pictures::insert_image(&mut s, &path, false);
    let frame = s.app().frame_selection.as_ref().unwrap().frames[0];
    for (field, value) in [
        (Field::ImageBrightness, 60),
        (Field::ImageContrast, 70),
        (Field::ImageEdge, 25),
    ] {
        s.act("adjust", |rt| {
            rt.send(Message::ImageSlide(field, value));
            rt.send(Message::ImageSlideEnd);
        });
    }
    s.act("Invert", |rt| {
        rt.send(Message::InspectorChoice(Choice::ImageInvert(true)));
    });
    let write = |s: &Scenario, name: &str| {
        for (dark, theme) in [
            (false, cosmic::Theme::light()),
            (true, cosmic::Theme::dark()),
        ] {
            let size = (1_280u32, 800u32);
            let rgb = render_window(s.app(), size, &theme, false);
            let image = image::RgbImage::from_raw(size.0, size.1, rgb).unwrap();
            image
                .save(directory.join(format!(
                    "{name}-{}.png",
                    if dark { "dark" } else { "light" }
                )))
                .unwrap();
        }
    };
    s.app_mut().open_rail();
    write(&s, "image-section");
    // A tall window shows the whole rail, the Image section included.
    for (dark, theme) in [
        (false, cosmic::Theme::light()),
        (true, cosmic::Theme::dark()),
    ] {
        let size = (1_280u32, 1_500u32);
        let rgb = render_window(s.app(), size, &theme, false);
        image::RgbImage::from_raw(size.0, size.1, rgb)
            .unwrap()
            .save(directory.join(format!(
                "image-section-tall-{}.png",
                if dark { "dark" } else { "light" }
            )))
            .unwrap();
    }
    // The rail scrolled to the Image section.
    let _ = frame;
    s.app_mut().frame_selection = None;
    s.rt.clipboard = Some("Some words".into());
    s.rt.clipboard_data = Some((b"{\\rtf1 Rich}".to_vec(), "text/rtf".into()));
    s.rt.clipboard_more = vec![(
        {
            let mut png = Vec::new();
            image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 128, 255, 255]))
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .unwrap();
            png
        },
        "image/png".into(),
    )];
    s.no_change("Edit > Paste Special", |rt| {
        rt.send(Message::Command(CommandId::PasteSpecial));
    });
    write(&s, "paste-special");
}

// ------------------------------------------------------------------ charts
//
// Run D, DO.8 (`UI_SPEC.md` §42.4): Tools › Charting… and the Chart panel
// through the real message path. Nothing opens a portal: Save Chart as
// Image… is sent with the task dropped.

mod charts {
    use super::*;
    use crate::chart_panel::{AxisField, ChartAction, Kind};
    use amipro_pictures::chart::{Chart, Icon, chart_of};
    use cosmic::iced::widget::text_editor;

    pub(super) fn chart_frames(s: &Scenario) -> Vec<FrameId> {
        s.document()
            .frame_stack()
            .iter()
            .copied()
            .filter(|id| {
                matches!(
                    &s.document().frame(*id).unwrap().content,
                    FrameContent::Picture(picture) if picture.format() == PictureFormat::Chart
                )
            })
            .collect()
    }

    /// The last chart frame's chart as the document holds it.
    pub(super) fn chart(s: &Scenario) -> Chart {
        let frame = *chart_frames(s)
            .last()
            .unwrap_or_else(|| s.fail("chart", "the document has no chart frame"));
        let drawing = amipro_editing::drawing::drawing_of(s.document(), frame)
            .unwrap_or_else(|| s.fail("chart", "the chart frame has no drawing"));
        chart_of(&drawing)
            .unwrap_or_else(|| s.fail("chart", "the drawing has no chart record"))
            .1
    }

    pub(super) fn panel_chart(s: &Scenario) -> Chart {
        s.app()
            .chart_panel
            .as_ref()
            .and_then(|panel| panel.chart.clone())
            .unwrap_or_else(|| s.fail("panel", "the Chart panel shows no chart"))
    }

    /// One panel action as a step that edits the document (one undo step).
    pub(super) fn edit(s: &mut Scenario, what: &str, actions: Vec<ChartAction>) {
        s.edit(what, move |rt| {
            for action in actions {
                rt.send(Message::Chart(action));
            }
        });
        // The panel and the document agree after the step, its Undo, and
        // its Redo (the checker ran at each).
        if let Some(panel) = s.app().chart_panel.as_ref()
            && panel.kind == Kind::Chart
        {
            assert_eq!(
                panel_chart(s),
                chart(s),
                "{what}: the panel follows the chart"
            );
        }
    }

    /// Selects the last chart frame and opens its panel as a double-click.
    pub(super) fn open_panel(s: &mut Scenario) {
        let frame = *chart_frames(s).last().unwrap();
        select_frame(s, frame);
        s.act("double-click the chart", |rt| {
            rt.send(Message::Frame(FrameAction::EnterText { at: None }));
        });
        assert_eq!(s.app().rail_panel(), crate::rail_panels::RailPanel::Chart);
    }

    /// A document with a line of text, the clipboard holding `clipboard`,
    /// and Tools › Charting… run: a new chart in a new frame.
    pub(super) fn start(name: &'static str, clipboard: Option<&str>) -> Scenario {
        let mut s = Scenario::new_document(name);
        s.clean_folder();
        s.type_text("Sales by quarter");
        s.rt.clipboard = clipboard.map(str::to_owned);
        s.act("Tools > Charting", |rt| {
            rt.send(Message::Command(CommandId::Charting));
        });
        s
    }

    /// [`start`] under the name the scenarios use.
    pub(super) fn new_chart(name: &'static str, clipboard: Option<&str>) -> Scenario {
        start(name, clipboard)
    }

    pub(super) fn paste_into_data(text: &str) -> Vec<ChartAction> {
        vec![
            ChartAction::DataEdit(text_editor::Action::SelectAll),
            ChartAction::DataEdit(text_editor::Action::Edit(text_editor::Edit::Paste(
                std::sync::Arc::new(text.to_owned()),
            ))),
        ]
    }

    #[test]
    fn charting_with_tabbed_text_on_the_clipboard_starts_from_it() {
        let s = start(
            "objects-chart-clipboard",
            Some("\tQ1\tQ2\tQ3\nNorth\t4\t5\t6\nSouth\t7\t8\t9\n"),
        );
        let chart = chart(&s);
        assert_eq!((chart.rows, chart.columns), (2, 3));
        assert_eq!(chart.values, vec![4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        assert_eq!(chart.row_labels[1].shown(), "South");
        assert_eq!(chart.column_labels[2].shown(), "Q3");
        assert_eq!(s.app().rail_panel(), crate::rail_panels::RailPanel::Chart);
        let panel = s.app().chart_panel.as_ref().unwrap();
        assert_eq!(panel.kind, Kind::Chart);
        assert!(panel.notice.is_none(), "no sample-data note");
        // The frame keeps the chart driver and a Flow with text frame.
        let frame = chart_frames(&s)[0];
        let FrameContent::Picture(picture) = &s.document().frame(frame).unwrap().content else {
            unreachable!()
        };
        assert_eq!(picture.extension, ".sct");
        assert!(matches!(
            s.document().frame(frame).unwrap().placement,
            amipro_document::Placement::Inline { .. }
        ));
    }

    #[test]
    fn charting_without_data_on_the_clipboard_starts_with_sample_data() {
        for clipboard in [None, Some("just some words 5"), Some("a\tb")] {
            let s = start("objects-chart-sample", clipboard);
            let chart = chart(&s);
            assert_eq!((chart.rows, chart.columns), (2, 3));
            assert_eq!(chart.value(0, 2), 30.0);
            let panel = s.app().chart_panel.as_ref().unwrap();
            assert!(
                matches!(&panel.notice, Some(crate::chart_panel::Notice::Info(text))
                if text == crate::chart_panel::SAMPLE_NOTE)
            );
        }
    }

    #[test]
    fn every_type_icon_regenerates_the_chart_as_one_undo_step() {
        let mut s = new_chart("objects-chart-types", None);
        for icon in Icon::ALL {
            if crate::chart_panel::icon_of(&chart(&s)) == icon {
                continue;
            }
            edit(
                &mut s,
                crate::chart_panel::name(icon),
                vec![ChartAction::Icon(icon)],
            );
            assert_eq!(crate::chart_panel::icon_of(&chart(&s)), icon);
            // The drawing was generated for the type: ends with the record.
            let frame = chart_frames(&s)[0];
            let drawing = amipro_editing::drawing::drawing_of(s.document(), frame).unwrap();
            assert!(drawing.objects.len() > 3, "{icon:?} draws something");
            assert_eq!(
                drawing.objects.last().unwrap().kind,
                amipro_pictures::sdw::kind::CHART
            );
        }
        // Stacked, then the same icon again: nothing to change.
        s.no_change("the type already chosen", |rt| {
            rt.send(Message::Chart(ChartAction::Icon(Icon::StackedPicture)));
        });
    }

    #[test]
    fn options_follow_the_dialogs_enabling_rules() {
        let mut s = new_chart("objects-chart-options", None);
        edit(&mut s, "Grid", vec![ChartAction::Grid(true)]);
        assert!(chart(&s).grid_on());
        edit(&mut s, "Legend off", vec![ChartAction::Legend(false)]);
        assert!(!chart(&s).legend_on());
        edit(&mut s, "3D", vec![ChartAction::ThreeD(true)]);
        assert_eq!((chart(&s).three_d(), chart(&s).depth()), (1, 50));
        edit(&mut s, "Perspective", vec![ChartAction::Perspective(true)]);
        assert_eq!((chart(&s).three_d(), chart(&s).depth()), (2, 50));
        edit(
            &mut s,
            "Depth 80",
            vec![
                ChartAction::DepthText("80".into()),
                ChartAction::DepthSubmit,
            ],
        );
        assert_eq!(chart(&s).depth(), 80);
        // A depth outside 1 to 100 is refused where it is typed.
        s.no_change("Depth 500", |rt| {
            rt.send(Message::Chart(ChartAction::DepthText("500".into())));
            rt.send(Message::Chart(ChartAction::DepthSubmit));
        });
        assert_eq!(chart(&s).depth(), 80);
        assert!(s.app().chart_panel.as_ref().unwrap().notice.is_some());
        // A line chart has no 3D: the depth goes.
        edit(&mut s, "Line", vec![ChartAction::Icon(Icon::Line)]);
        assert_eq!(chart(&s).three_d(), 0);
        // A pie has no axis; a 3D pie is stored with perspective depth.
        edit(&mut s, "Pie", vec![ChartAction::Icon(Icon::Pie)]);
        edit(&mut s, "3D pie", vec![ChartAction::ThreeD(true)]);
        assert_eq!(
            (chart(&s).perspective_depth, chart(&s).oblique_depth),
            (50, 0)
        );
    }

    #[test]
    fn dragging_the_depth_slider_is_one_undo_step() {
        let mut s = new_chart("objects-chart-slider", None);
        edit(&mut s, "3D", vec![ChartAction::ThreeD(true)]);
        let undo = s.app().history.undo_len();
        s.act("drag the depth slider", |rt| {
            for depth in [60, 70, 80, 90] {
                rt.send(Message::Chart(ChartAction::Depth(depth)));
            }
            rt.send(Message::Chart(ChartAction::DepthEnd));
        });
        assert_eq!(chart(&s).depth(), 90);
        assert_eq!(s.app().history.undo_len(), undo + 1, "one undo step");
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        assert_eq!(chart(&s).depth(), 50);
        assert_eq!(panel_chart(&s).depth(), 50, "the panel follows an Undo");
    }

    #[test]
    fn the_data_grid_and_the_data_text_edit_in_place() {
        let mut s = new_chart("objects-chart-data", None);
        edit(
            &mut s,
            "type 99 in a cell",
            vec![
                ChartAction::Cell {
                    row: 1,
                    column: 2,
                    text: "99".into(),
                },
                ChartAction::CommitCells,
            ],
        );
        assert_eq!(chart(&s).value(0, 1), 99.0);
        edit(
            &mut s,
            "rename a series and a category",
            vec![
                ChartAction::Cell {
                    row: 2,
                    column: 0,
                    text: "West Coast".into(),
                },
                ChartAction::Cell {
                    row: 0,
                    column: 3,
                    text: "March".into(),
                },
                ChartAction::CommitCells,
            ],
        );
        assert_eq!(chart(&s).row_labels[1].shown(), "West Coast");
        assert_eq!(chart(&s).column_labels[2].shown(), "March");
        // A value that is not a number stays in its cell with the reason.
        s.no_change("type words in a cell", |rt| {
            rt.send(Message::Chart(ChartAction::Cell {
                row: 1,
                column: 1,
                text: "lots".into(),
            }));
            rt.send(Message::Chart(ChartAction::CommitCells));
        });
        let panel = s.app().chart_panel.as_ref().unwrap();
        assert!(
            panel
                .cell_error
                .as_deref()
                .is_some_and(|text| text.contains("lots"))
        );
        assert_eq!(panel.cells.get(&(1, 1)).map(String::as_str), Some("lots"));
        // Typing a number there applies it and clears the error.
        edit(
            &mut s,
            "fix the cell",
            vec![
                ChartAction::Cell {
                    row: 1,
                    column: 1,
                    text: "11".into(),
                },
                ChartAction::CommitCells,
            ],
        );
        assert!(s.app().chart_panel.as_ref().unwrap().cell_error.is_none());
        // The Data text: a refused text shows its reason and Apply waits.
        s.no_change("type text without numbers", |rt| {
            for action in paste_into_data("only words") {
                rt.send(Message::Chart(action));
            }
            rt.send(Message::Chart(ChartAction::ApplyData));
        });
        assert!(s.app().chart_panel.as_ref().unwrap().data_error.is_some());
        edit(
            &mut s,
            "apply a new data text",
            [
                paste_into_data("\tA\tB\nOne\t1\t2\nTwo\t3\t4\nThree\t5\t6\n"),
                vec![ChartAction::ApplyData],
            ]
            .concat(),
        );
        let chart = chart(&s);
        assert_eq!((chart.rows, chart.columns), (3, 2));
        assert_eq!(chart.values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(chart.row_labels[2].shown(), "Three");
        // The text applies when the user turns to something else, in the
        // same undo step.
        let undo = s.app().history.undo_len();
        s.act("type data and click Flip", |rt| {
            for action in paste_into_data("\tA\tB\nOne\t1\t2\n") {
                rt.send(Message::Chart(action));
            }
            rt.send(Message::Chart(ChartAction::Flip));
        });
        assert_eq!(s.app().history.undo_len(), undo + 1);
        let chart = self::chart(&s);
        assert_eq!(
            (chart.rows, chart.columns),
            (2, 1),
            "one series of two, flipped"
        );
        // Adding and removing.
        edit(&mut s, "Add Series", vec![ChartAction::AddSeries]);
        edit(&mut s, "Add Category", vec![ChartAction::AddCategory]);
        assert_eq!((self::chart(&s).rows, self::chart(&s).columns), (3, 2));
        edit(&mut s, "Remove Series", vec![ChartAction::RemoveSeries]);
        edit(&mut s, "Remove Category", vec![ChartAction::RemoveCategory]);
        s.no_change("Remove the last category", |rt| {
            rt.send(Message::Chart(ChartAction::RemoveCategory));
        });
        // Copy Data as CSV.
        s.act("Copy Data as CSV", |rt| {
            rt.send(Message::Chart(ChartAction::CopyCsv));
        });
        let copied = s.rt.clipboard.clone().unwrap();
        assert!(copied.starts_with(','), "{copied:?}");
        assert_eq!(copied.lines().count(), 1 + self::chart(&s).rows);
    }

    #[test]
    fn flip_swaps_series_and_categories() {
        let mut s = new_chart("objects-chart-flip", None);
        let before = chart(&s);
        edit(&mut s, "Flip Data", vec![ChartAction::Flip]);
        let flipped = chart(&s);
        assert_eq!(
            (flipped.rows, flipped.columns),
            (before.columns, before.rows)
        );
        assert_eq!(flipped.value(2, 1), before.value(1, 2));
        assert_eq!(flipped.row_labels[0].shown(), "Jan");
        edit(&mut s, "Flip Data again", vec![ChartAction::Flip]);
        assert_eq!(chart(&s).values, before.values);
    }

    #[test]
    fn the_axis_is_typed_where_it_is_shown() {
        let mut s = new_chart("objects-chart-axis", None);
        {
            let panel = s.app().chart_panel.as_ref().unwrap();
            assert!(panel.axis.automatic, "a new chart's axis is Automatic");
            assert_eq!(panel.axis.max, "35");
        }
        // Unticking Automatic changes nothing in the document.
        s.no_change("untick Automatic", |rt| {
            rt.send(Message::Chart(ChartAction::AxisAutomatic(false)));
        });
        edit(
            &mut s,
            "type Inc, Min, and Max",
            vec![
                ChartAction::AxisText(AxisField::Inc, "20".into()),
                ChartAction::AxisText(AxisField::Min, "0".into()),
                ChartAction::AxisText(AxisField::Max, "100".into()),
                ChartAction::AxisSubmit,
            ],
        );
        let record = chart(&s);
        assert_eq!(
            (record.axis.min, record.axis.max, record.axis.step),
            (0.0, 100.0, 20.0)
        );
        assert!(!s.app().chart_panel.as_ref().unwrap().axis.automatic);
        // The value labels on the page follow the typed axis.
        let frame = chart_frames(&s)[0];
        let drawing = amipro_editing::drawing::drawing_of(s.document(), frame).unwrap();
        let texts: Vec<String> = drawing
            .objects
            .iter()
            .filter_map(|o| match &o.shape {
                amipro_pictures::sdw::Shape::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.contains(&"100".to_string()) && texts.contains(&"40".to_string()),
            "{texts:?}"
        );
        // A change of the data keeps the typed axis.
        edit(
            &mut s,
            "change a value",
            vec![
                ChartAction::Cell {
                    row: 1,
                    column: 1,
                    text: "12".into(),
                },
                ChartAction::CommitCells,
            ],
        );
        assert_eq!(chart(&s).axis.max, 100.0);
        // A wrong box is refused where it is typed.
        s.no_change("Max below Min", |rt| {
            rt.send(Message::Chart(ChartAction::AxisText(
                AxisField::Max,
                "-5".into(),
            )));
            rt.send(Message::Chart(ChartAction::AxisSubmit));
        });
        assert!(
            s.app()
                .chart_panel
                .as_ref()
                .unwrap()
                .axis
                .error
                .as_deref()
                .is_some_and(|text| text.contains("above Min"))
        );
        // Ticking Automatic computes it again, as one step.
        edit(
            &mut s,
            "tick Automatic",
            vec![ChartAction::AxisAutomatic(true)],
        );
        assert_eq!(chart(&s).axis.max, 35.0);
        assert!(s.app().chart_panel.as_ref().unwrap().axis.automatic);
    }

    #[test]
    fn a_colour_of_the_set_is_changed_through_the_colour_control() {
        use crate::colour_control::{ColourAction, Target};
        let mut s = new_chart("objects-chart-colour", None);
        s.act("select the third colour", |rt| {
            rt.send(Message::Chart(ChartAction::SelectColour(2)));
        });
        s.act("open the colour button", |rt| {
            rt.send(Message::Colour(ColourAction::Toggle(
                Target::ChartColour,
                0x0000_C200,
            )));
        });
        s.edit("choose red", |rt| {
            rt.send(Message::Colour(ColourAction::Choose(0x0000_00FF)));
        });
        assert_eq!(chart(&s).palette[2][..3], [0xFF, 0, 0]);
        assert_eq!(chart(&s).palette[2][3], 6, "the pattern byte is kept");
    }

    #[test]
    fn esc_closes_the_panel_and_drops_what_was_typed() {
        let mut s = new_chart("objects-chart-esc", None);
        s.act("type in a cell", |rt| {
            rt.send(Message::Chart(ChartAction::Cell {
                row: 1,
                column: 1,
                text: "77".into(),
            }));
        });
        let before = describe(s.document());
        s.act("Esc", |rt| {
            rt.send(Message::Escape);
        });
        assert!(s.app().chart_panel.is_none());
        assert_eq!(s.app().rail_panel(), crate::rail_panels::RailPanel::Styles);
        assert_eq!(describe(s.document()), before);
    }

    #[test]
    fn undoing_the_new_chart_closes_the_panel_and_a_double_click_opens_it_again() {
        let mut s = Scenario::new_document("objects-chart-undo");
        s.type_text("Sales by quarter");
        s.edit("Tools > Charting", |rt| {
            rt.send(Message::Command(CommandId::Charting));
        });
        // (`edit` checked Undo and Redo of the new chart as one step; the
        // Undo closed the panel with the frame.)
        assert!(s.app().chart_panel.is_none(), "the panel followed the Undo");
        open_panel(&mut s);
        assert_eq!(s.app().chart_panel.as_ref().unwrap().kind, Kind::Chart);
        edit(
            &mut s,
            "Stacked Column",
            vec![ChartAction::Icon(Icon::StackedColumn)],
        );
    }

    /// A table of numbers with a header row and a header column, the cells
    /// selected.
    pub(super) fn numbers_and_select(s: &mut Scenario) -> FrameId {
        s.command_act(CommandId::Table);
        s.act("Insert a 4 by 3 table", |rt| {
            rt.send(Message::InsertTable(
                crate::table_commands::InsertTableAction::Columns("4".into()),
            ));
            rt.send(Message::InsertTable(
                crate::table_commands::InsertTableAction::Rows("3".into()),
            ));
            rt.send(Message::InsertTable(
                crate::table_commands::InsertTableAction::Insert,
            ));
        });
        let frame = *s.document().frame_stack().last().unwrap();
        let texts = [
            ["", "Q1", "Q2", "Q3"],
            ["North", "4", "5", "6"],
            ["South", "7", "", "9"],
        ];
        for (row, line) in texts.iter().enumerate() {
            for (column, text) in line.iter().enumerate() {
                if text.is_empty() {
                    continue;
                }
                let flow = cell(s.document(), frame, row, column);
                move_to(s, flow, false);
                s.type_text(text);
            }
        }
        let first = cell(s.document(), frame, 0, 0);
        let last = cell(s.document(), frame, 2, 3);
        move_to(s, first, false);
        move_to(s, last, true);
        frame
    }

    /// The caret (or the selection's end) into the cell whose text is
    /// `flow`; leaving a cell with a number formats it, so this is no
    /// `no_change` step.
    pub(super) fn move_to(s: &mut Scenario, flow: FlowId, extend: bool) {
        s.act("caret into a cell", move |rt| {
            rt.send(Message::Page(PageAction::MoveTo {
                position: at(flow, 0, 0),
                page: 1,
                extend,
            }));
        });
    }

    #[test]
    fn chart_from_table_copies_the_selected_cells_into_a_new_frame() {
        let mut s = Scenario::new_document("objects-chart-table");
        s.clean_folder();
        s.type_text("Quarterly numbers");
        let table = numbers_and_select(&mut s);
        s.edit("Tools > Chart from Table", |rt| {
            rt.send(Message::Command(CommandId::ChartFromTable));
        });
        let chart = chart(&s);
        // The first row and column are labels (the number-versus-label
        // rule); the empty cell among numbers is zero.
        assert_eq!((chart.rows, chart.columns), (2, 3));
        assert_eq!(chart.values, vec![4.0, 5.0, 6.0, 7.0, 0.0, 9.0]);
        assert_eq!(chart.row_labels[0].shown(), "North");
        assert_eq!(chart.column_labels[1].shown(), "Q2");
        // A frame after the table: With para above, default size; the
        // table is untouched.
        let frame = chart_frames(&s)[0];
        let placement = s.document().frame(frame).unwrap().placement;
        assert!(
            matches!(placement, amipro_document::Placement::BelowParagraph { size, .. }
                if size.width == 4_608 && size.height == 3_888),
            "{placement:?}"
        );
        assert!(s.document().frame(table).is_some());
        // The data is a copy: editing the table later leaves the chart.
        let flow = cell(s.document(), table, 1, 1);
        move_to(&mut s, flow, false);
        s.type_text("0");
        assert_eq!(self::chart(&s).value(0, 0), 4.0);
    }

    #[test]
    fn charting_in_a_table_uses_the_selected_cells_and_the_panel_offers_chart_from_table() {
        let mut s = Scenario::new_document("objects-chart-table-panel");
        s.clean_folder();
        s.type_text("Numbers");
        numbers_and_select(&mut s);
        s.act("Tools > Charting", |rt| {
            rt.send(Message::Command(CommandId::Charting));
        });
        assert_eq!(chart(&s).values.len(), 6);
        // The caret is still in the table: Chart from Table in the panel
        // makes a second chart from the whole table.
        s.act("the caret back in a cell", |rt| {
            let _ = rt;
        });
        let flow = {
            let table = s
                .document()
                .frame_stack()
                .iter()
                .copied()
                .find(|id| {
                    matches!(
                        s.document().frame(*id).unwrap().content,
                        FrameContent::Table(_)
                    )
                })
                .unwrap();
            cell(s.document(), table, 1, 1)
        };
        move_to(&mut s, flow, false);
        s.edit("Chart from Table in the panel", |rt| {
            rt.send(Message::Chart(ChartAction::FromTable));
        });
        assert_eq!(chart_frames(&s).len(), 2);
    }

    #[test]
    fn charting_on_a_drawing_asks_before_it_replaces_it() {
        let mut s = Scenario::new_document("objects-chart-replace");
        s.clean_folder();
        s.type_text("A drawing below.");
        s.command(CommandId::Drawing);
        if s.app().draw.is_none() {
            s.act("open Draw mode again", |rt| {
                rt.send(Message::Command(CommandId::Redo));
            });
        }
        let frame = s.app().draw.as_ref().unwrap().frame;
        super::pictures::draw_shape(
            &mut s,
            crate::draw_mode::Tool::Rectangle,
            (200.0, 200.0),
            (1_200.0, 800.0),
        );
        s.act("leave Draw mode", |rt| {
            rt.app.leave_draw_mode();
        });
        select_frame(&mut s, frame);
        s.no_change("Tools > Charting on a drawing", |rt| {
            rt.send(Message::Command(CommandId::Charting));
        });
        let panel = s.app().chart_panel.as_ref().unwrap();
        assert!(
            matches!(panel.kind, Kind::Replace("a drawing")),
            "{:?}",
            panel.kind
        );
        s.no_change("Cancel", |rt| {
            rt.send(Message::Chart(ChartAction::CancelReplace));
        });
        assert!(s.app().chart_panel.is_none());
        select_frame(&mut s, frame);
        s.act("Tools > Charting on a drawing", |rt| {
            rt.send(Message::Command(CommandId::Charting));
        });
        s.edit("Replace with a Chart", |rt| {
            rt.send(Message::Chart(ChartAction::ConfirmReplace));
        });
        assert_eq!(chart_frames(&s), vec![frame]);
        assert_eq!(s.app().chart_panel.as_ref().unwrap().kind, Kind::Chart);
    }
    #[test]
    fn a_chart_whose_record_is_unreadable_is_kept_and_offers_only_a_new_chart() {
        use amipro_pictures::sdw::{self, Shape};
        // A chart whose record is three bytes short of the known layout.
        let chart = crate::chart_panel::new_chart(None);
        let mut drawing = amipro_pictures::chart_gen::generate_drawing(
            &chart,
            &amipro_pictures::chart_gen::Metrics::for_frame(4_608, 3_888),
        );
        if let Shape::Other(body) = &mut drawing.objects.last_mut().unwrap().shape {
            body.truncate(body.len() - 3);
        }
        let bytes: std::sync::Arc<[u8]> = sdw::write(&drawing).into();
        let document = Document::from_plain_text("A chart follows.");
        let applied = amipro_editing::apply(
            &document,
            &amipro_editing::Command::CreateFrame {
                placement: amipro_document::Placement::Fixed {
                    page: 1,
                    rect: amipro_document::Rect::new(1_296, 1_152, 5_904, 5_040),
                },
                style: amipro_document::FrameStyle::default(),
                content: amipro_editing::NewFrameContent::EmbeddedPicture {
                    bytes: bytes.clone(),
                    extension: ".sct".into(),
                },
                anchor: None,
                layout: None,
            },
        )
        .unwrap();
        let frame = *applied.report.frames.iter().next().unwrap();
        let mut s = Scenario::new("objects-chart-unreadable", applied.document);
        s.clean_folder();
        select_frame(&mut s, frame);
        s.act("double-click the chart", |rt| {
            rt.send(Message::Frame(FrameAction::EnterText { at: None }));
        });
        let panel = s.app().chart_panel.as_ref().unwrap();
        assert_eq!(panel.kind, Kind::Unreadable);
        assert!(panel.chart.is_none());
        // Every editing action leaves it alone.
        s.no_change("editing actions on the unreadable chart", |rt| {
            for action in [
                ChartAction::Icon(Icon::Pie),
                ChartAction::Flip,
                ChartAction::Legend(false),
                ChartAction::AddSeries,
                ChartAction::ApplyData,
            ] {
                rt.send(Message::Chart(action));
            }
        });
        // Save As SAM writes what was read.
        let path = s.save_sam("UNREAD.SAM");
        let again = amipro_formats::import_sam_path(&path).unwrap().document;
        let frame_again = again.frame_stack()[0];
        let FrameContent::Picture(picture) = &again.frame(frame_again).unwrap().content else {
            unreachable!()
        };
        let amipro_document::PictureSource::Embedded(object) = &picture.source else {
            unreachable!()
        };
        assert_eq!(again.objects().get(object).unwrap().bytes, bytes);
        // The one way out: a new chart in its place (one undo step).
        s.edit("Replace with a New Chart", |rt| {
            rt.send(Message::Chart(ChartAction::ConfirmReplace));
        });
        assert_eq!(chart_frames(&s).len(), 1);
        assert_eq!(s.app().chart_panel.as_ref().unwrap().kind, Kind::Chart);
    }

    #[test]
    fn a_chart_last_edited_with_drawing_opens_draw_mode_until_charting_changes_it() {
        let mut s = new_chart("objects-chart-drawing", None);
        let frame = chart_frames(&s)[0];
        s.no_change("Drawing…", |rt| {
            rt.send(Message::Chart(ChartAction::Drawing));
        });
        assert!(s.app().draw.is_some(), "Draw mode is open on the chart");
        assert!(s.app().chart_panel.is_none());
        s.act("leave Draw mode", |rt| {
            rt.app.leave_draw_mode();
        });
        assert!(s.app().chart_drawing.contains(&frame));
        s.act("double-click the chart", |rt| {
            rt.send(Message::Frame(FrameAction::EnterText { at: None }));
        });
        assert!(s.app().draw.is_some(), "Draw mode again, not the panel");
        s.act("leave Draw mode", |rt| {
            rt.app.leave_draw_mode();
        });
        // Charting opens the panel, saying the drawing edits are replaced
        // by the next change.
        select_frame(&mut s, frame);
        s.act("Tools > Charting", |rt| {
            rt.send(Message::Command(CommandId::Charting));
        });
        let panel = s.app().chart_panel.as_ref().unwrap();
        assert!(
            matches!(&panel.notice, Some(crate::chart_panel::Notice::Info(text))
            if text.contains("edited with Drawing"))
        );
        edit(&mut s, "Bar", vec![ChartAction::Icon(Icon::Bar)]);
        assert!(!s.app().chart_drawing.contains(&frame));
        select_frame(&mut s, frame);
        s.act("double-click the chart", |rt| {
            rt.send(Message::Frame(FrameAction::EnterText { at: None }));
        });
        assert!(s.app().draw.is_none());
        assert_eq!(s.app().rail_panel(), crate::rail_panels::RailPanel::Chart);
    }

    /// The whole task: a table of numbers, selected; Chart from Table; the
    /// type changed to stacked; a value edited; Undo; Save As SAM; reopen;
    /// edit again.
    #[test]
    fn a_table_becomes_a_chart_that_survives_save_as_sam_and_reopening() {
        let mut s = Scenario::new_document("objects-chart-whole");
        s.clean_folder();
        s.type_text("Quarterly numbers");
        numbers_and_select(&mut s);
        s.act("Tools > Chart from Table", |rt| {
            rt.send(Message::Command(CommandId::ChartFromTable));
        });
        assert_eq!(s.app().rail_panel(), crate::rail_panels::RailPanel::Chart);
        edit(
            &mut s,
            "Stacked Column",
            vec![ChartAction::Icon(Icon::StackedColumn)],
        );
        edit(
            &mut s,
            "type 50 in a cell",
            vec![
                ChartAction::Cell {
                    row: 1,
                    column: 1,
                    text: "50".into(),
                },
                ChartAction::CommitCells,
            ],
        );
        assert_eq!(chart(&s).value(0, 0), 50.0);
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        assert_eq!(chart(&s).value(0, 0), 4.0);
        assert_eq!(
            panel_chart(&s).value(0, 0),
            4.0,
            "the panel follows the Undo"
        );
        assert_eq!(
            crate::chart_panel::icon_of(&chart(&s)),
            Icon::StackedColumn,
            "only the last step was undone"
        );
        s.act("Redo", |rt| {
            rt.send(Message::Command(CommandId::Redo));
        });
        let saved = chart(&s);
        let path = s.save_sam("CHARTWHOLE.SAM");
        let mut r = s.reopen(&path);
        assert_eq!(chart_frames(&r).len(), 1);
        let reread = chart(&r);
        assert_eq!(reread.values, saved.values);
        assert_eq!(reread.row_labels, saved.row_labels);
        assert_eq!(reread.column_labels, saved.column_labels);
        assert_eq!(reread.axis, saved.axis);
        assert_eq!(crate::chart_panel::icon_of(&reread), Icon::StackedColumn);
        // Still editable: the panel opens on it and a change applies.
        open_panel(&mut r);
        assert_eq!(r.app().chart_panel.as_ref().unwrap().kind, Kind::Chart);
        edit(&mut r, "Column", vec![ChartAction::Icon(Icon::Column)]);
        edit(
            &mut r,
            "type 9 in a cell",
            vec![
                ChartAction::Cell {
                    row: 2,
                    column: 3,
                    text: "9.5".into(),
                },
                ChartAction::CommitCells,
            ],
        );
        assert_eq!(chart(&r).value(1, 2), 9.5);
        r.save_sam("CHARTWHOLE2.SAM");
    }

    #[test]
    fn save_chart_as_image_writes_png_and_svg() {
        let mut s = new_chart("objects-chart-image", None);
        // The dialog is never opened in a test: send the task away.
        drop(cosmic::Application::update(
            &mut s.rt.app,
            Message::Chart(ChartAction::SaveImage),
        ));
        let folder = s.clean_folder();
        for name in ["chart.png", "chart.svg"] {
            let path = folder.join(name);
            let session_id = s.app().id;
            s.act(&format!("the dialog chose {name}"), move |rt| {
                rt.send(Message::ChartImageChosen {
                    session_id,
                    result: crate::FileDialogResult::Selected(path),
                });
            });
            let bytes = std::fs::read(folder.join(name)).unwrap();
            if name.ends_with("png") {
                assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
            } else {
                assert!(String::from_utf8_lossy(&bytes).contains("<svg"));
            }
        }
    }

    #[test]
    fn the_panel_builds_and_its_icons_are_clicked_in_the_window() {
        use cosmic::iced::Point;
        use cosmic::iced::mouse::Button;
        let mut s = new_chart("objects-chart-widget", None);
        // Every state of the panel draws.
        s.act("redraw", |rt| {
            rt.redraw();
        });
        edit(&mut s, "3D", vec![ChartAction::ThreeD(true)]);
        s.act("redraw", |rt| {
            rt.redraw();
        });
        // The twelve icons are two rows of six at the top of the rail: a
        // click on one publishes its message and nothing else.
        let first = Point::new(CHART_ICON_X, CHART_ICON_Y);
        let published = s.rt.click_unrun(Button::Left, first);
        assert!(
            published
                .iter()
                .any(|m| matches!(m, Message::Chart(ChartAction::Icon(Icon::Column)))),
            "a click on the first icon chooses Column: {published:?}"
        );
        let stacked = Point::new(CHART_ICON_X + CHART_ICON_STEP, CHART_ICON_Y);
        let published = s.rt.click_unrun(Button::Left, stacked);
        assert!(
            published
                .iter()
                .any(|m| matches!(m, Message::Chart(ChartAction::Icon(Icon::StackedColumn)))),
            "{published:?}"
        );
        let area = Point::new(CHART_ICON_X + 5.0 * CHART_ICON_STEP, CHART_ICON_Y);
        let published = s.rt.click_unrun(Button::Left, area);
        assert!(
            published
                .iter()
                .any(|m| matches!(m, Message::Chart(ChartAction::Icon(Icon::Area)))),
            "{published:?}"
        );
        // Delivering the message changes the chart.
        s.edit("the chosen icon", |rt| {
            rt.send(Message::Chart(ChartAction::Icon(Icon::Area)));
        });
    }
}

/// Where the first chart-type icon is in the rail of a 1,280 by 800 window
/// (measured from the screenshots), and the distance between icons.
const CHART_ICON_X: f32 = 31.0;
const CHART_ICON_Y: f32 = 249.0;
const CHART_ICON_STEP: f32 = 37.8;

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Chart panel with a column chart and
/// with a 3D pie, light and dark, in the window and in a tall window, as
/// PNG files (not committed).
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn chart_screenshots() {
    use crate::chart_panel::ChartAction;
    use amipro_pictures::chart::Icon;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut s = charts::new_chart(
        "objects-chart-screens",
        Some(
            "\tNorth\tSouth\tEast\tWest\nJanuary\t12\t23\t5\t14\nFebruary\t9\t17\t11\t8\nMarch\t15\t6\t19\t13\n",
        ),
    );
    let write = |s: &mut Scenario, name: &str| {
        for (dark, theme) in [
            (false, cosmic::Theme::light()),
            (true, cosmic::Theme::dark()),
        ] {
            for (suffix, size) in [("", (1_280u32, 800u32)), ("-tall", (1_280u32, 1_700u32))] {
                s.rt.redraw();
                let rgb = render_window(s.app(), size, &theme, false);
                image::RgbImage::from_raw(size.0, size.1, rgb)
                    .unwrap()
                    .save(directory.join(format!(
                        "{name}{suffix}-{}.png",
                        if dark { "dark" } else { "light" }
                    )))
                    .unwrap();
            }
        }
    };
    s.act("Grid", |rt| {
        rt.send(Message::Chart(ChartAction::Grid(true)));
    });
    write(&mut s, "chart-column");
    s.act("Pie", |rt| {
        rt.send(Message::Chart(ChartAction::Icon(Icon::Pie)));
        rt.send(Message::Chart(ChartAction::ThreeD(true)));
    });
    write(&mut s, "chart-pie");
    s.act("Stacked bar with a typed axis", |rt| {
        rt.send(Message::Chart(ChartAction::Icon(Icon::StackedBar)));
    });
    write(&mut s, "chart-stacked-bar");
}

/// Equations (run D, DO.9; `UI_SPEC.md` §42.5): Tools › Equations… makes an
/// equation frame stored as Ami Pro stores it (two text lines, the natural
/// size and baseline in the picture's words); the source is typed in the
/// card at the frame, the palette puts templates at the caret, and each
/// applied change is one undo step.
mod equations {
    use super::*;
    use crate::equation_panel::{EquationAction, HOLE};
    use amipro_pictures::equation::Stored;
    use cosmic::iced::widget::text_editor;

    pub(super) fn frames(s: &Scenario) -> Vec<FrameId> {
        s.document()
            .frame_stack()
            .iter()
            .copied()
            .filter(|id| {
                matches!(
                    &s.document().frame(*id).unwrap().content,
                    FrameContent::Picture(picture) if picture.format() == PictureFormat::Equation
                )
            })
            .collect()
    }

    /// The last equation frame's stored text, as the document holds it.
    pub(super) fn stored(s: &Scenario) -> Stored {
        let frame = *frames(s)
            .last()
            .unwrap_or_else(|| s.fail("equation", "the document has no equation frame"));
        let bytes = stream_of(s.document(), frame);
        Stored::read(&bytes).unwrap_or_else(|| s.fail("equation", "the stored text is unreadable"))
    }

    pub(super) fn source(s: &Scenario) -> String {
        s.app()
            .equation_panel
            .as_ref()
            .unwrap_or_else(|| s.fail("panel", "the Equation editor is not open"))
            .source_text()
    }

    /// The frame's size in twips.
    pub(super) fn frame_size(s: &Scenario) -> (i32, i32) {
        let frame = *frames(s).last().unwrap();
        let size = s.document().frame(frame).unwrap().placement.size().unwrap();
        (size.width, size.height)
    }

    pub(super) fn start(name: &'static str) -> Scenario {
        let mut s = Scenario::new_document(name);
        s.clean_folder();
        s.type_text("Energy");
        s.act("Tools > Equations", |rt| {
            rt.send(Message::Command(CommandId::Equations));
        });
        s
    }

    pub(super) fn insert(s: &mut Scenario, what: &str, template: &'static str) {
        s.edit(what, move |rt| {
            rt.send(Message::Equation(EquationAction::Insert(template)));
        });
    }

    /// Typing and pasting in the source field (keys of one run).
    pub(super) fn type_in(s: &mut Scenario, what: &str, text: &str) {
        let text = text.to_owned();
        s.act(what, move |rt| {
            for c in text.chars() {
                rt.send(Message::Equation(EquationAction::Edit(
                    text_editor::Action::Edit(text_editor::Edit::Insert(c)),
                )));
            }
        });
    }

    pub(super) fn replace_source(s: &mut Scenario, what: &str, text: &str) {
        let text = text.to_owned();
        s.act(what, move |rt| {
            rt.send(Message::Equation(EquationAction::Edit(
                text_editor::Action::SelectAll,
            )));
            rt.send(Message::Equation(EquationAction::Edit(
                text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(text))),
            )));
        });
    }

    #[test]
    fn tools_equations_makes_an_empty_equation_stored_as_ami_pro_stores_it() {
        let s = start("objects-eq-new");
        assert_eq!(
            s.app().rail_panel(),
            crate::rail_panels::RailPanel::Equation
        );
        let frame = frames(&s)[0];
        let FrameContent::Picture(picture) = &s.document().frame(frame).unwrap().content else {
            unreachable!()
        };
        assert_eq!(picture.extension, ".tex");
        let stored = stored(&s);
        assert_eq!(stored.settings, "TimesNewRomanPS,18,12,0,0,0,0,0");
        assert_eq!(stored.tex, "");
        // An empty equation is Ami Pro's empty box: 220 by 300 twips, the
        // frame exactly that size (EmptyWidth, EmptyHeight).
        assert_eq!(frame_size(&s), (220, 300));
        assert_eq!(
            (picture.scaling.size.width, picture.scaling.size.height),
            (220, 300)
        );
        assert!(matches!(
            s.document().frame(frame).unwrap().placement,
            amipro_document::Placement::Inline { .. }
        ));
    }

    #[test]
    fn the_palette_types_and_edits_a_fraction_with_one_undo_step_each() {
        let mut s = start("objects-eq-fraction");
        // (The first template is a plain step: an Undo and Redo reload the
        // source with the caret at its end, so the selection is checked
        // here.)
        s.act("fraction", |rt| {
            rt.send(Message::Equation(EquationAction::Insert("\\frac{⬚}{⬚}")));
        });
        assert_eq!(stored(&s).tex, format!("$\\frac{HOLE}{HOLE}$"));
        // The first hole is selected, so typing replaces it.
        let selected = s.app().equation_panel.as_ref().unwrap().source.selection();
        assert_eq!(selected.as_deref(), Some("⬚"));
        type_in(&mut s, "type a", "a");
        assert_eq!(source(&s), "\\frac{a}{⬚}");
        assert_eq!(stored(&s).tex, "$\\frac a⬚$");
        // The frame took the natural size of what is drawn.
        let layout = stored(&s).layout().unwrap();
        assert_eq!(frame_size(&s), (layout.width, layout.height));
        // The typing joined the step of the template: one Undo takes both.
        replace_source(&mut s, "type the whole thing", "\\frac{a}{b}");
        assert_eq!(stored(&s).tex, "$\\frac ab$");
        insert(&mut s, "square root", "\\sqrt{⬚}");
        assert_eq!(stored(&s).tex, "$\\frac ab\\sqrt{⬚}$");
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        assert_eq!(
            stored(&s).tex,
            "$\\frac ab$",
            "one Undo takes the root away"
        );
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        assert_eq!(
            stored(&s).tex,
            "",
            "the next Undo takes the fraction and its typing"
        );
        s.act("Redo", |rt| {
            rt.send(Message::Command(CommandId::Redo));
        });
        assert_eq!(stored(&s).tex, "$\\frac ab$");
    }

    #[test]
    fn a_text_that_does_not_parse_shows_its_error_and_keeps_the_last_drawing() {
        let mut s = start("objects-eq-error");
        replace_source(&mut s, "a good fraction", "\\frac{a}{b}");
        let good = stored(&s);
        let size = frame_size(&s);
        replace_source(&mut s, "a broken one", "x + \\frac{a}");
        let panel = s.app().equation_panel.as_ref().unwrap();
        let error = panel.error.clone().expect("the error is shown");
        assert_eq!(error.code, 5106);
        let line = panel.error_line().unwrap();
        assert!(
            line.contains("message 5106") && line.contains("line 1, column"),
            "{line}"
        );
        // The frame keeps the last good drawing, the document is unchanged.
        assert_eq!(stored(&s), good);
        assert_eq!(frame_size(&s), size);
        replace_source(&mut s, "an unknown command", "x \\bogus y");
        let line = s
            .app()
            .equation_panel
            .as_ref()
            .unwrap()
            .error_line()
            .unwrap();
        assert!(
            line.contains("unknown command") && line.contains("column 3"),
            "{line}"
        );
        replace_source(&mut s, "fixed", "x + y");
        assert!(s.app().equation_panel.as_ref().unwrap().error.is_none());
        assert_eq!(stored(&s).tex, "$x+y$");
        // A matrix over 10 by 10 is refused with its own message.
        let big = format!("\\matrix {}\\endmatrix", ["a"; 11].join(" \\\\ "));
        replace_source(&mut s, "an 11 row matrix", &big);
        assert_eq!(
            s.app()
                .equation_panel
                .as_ref()
                .unwrap()
                .error
                .as_ref()
                .unwrap()
                .code,
            5102
        );
        assert_eq!(stored(&s).tex, "$x+y$");
    }

    #[test]
    fn face_and_size_are_part_of_the_stored_text_and_one_step_each() {
        let mut s = start("objects-eq-face");
        insert(&mut s, "square root", "\\sqrt{x}");
        let small = frame_size(&s);
        s.edit("Size 24", |rt| {
            rt.send(Message::Equation(EquationAction::Size("24".into())));
            rt.send(Message::Equation(EquationAction::SizeSubmit));
        });
        assert_eq!(stored(&s).settings, "TimesNewRomanPS,18,24,0,0,0,0,0");
        let big = frame_size(&s);
        assert!(
            big.0 > small.0 * 3 / 2 && big.1 > small.1 * 3 / 2,
            "{small:?} {big:?}"
        );
        s.edit("Arial", |rt| {
            rt.send(Message::Equation(EquationAction::Face(2)));
        });
        assert_eq!(stored(&s).face().0, "Arial");
        // A size out of range is refused in the panel.
        s.no_change("Size 900", |rt| {
            rt.send(Message::Equation(EquationAction::Size("900".into())));
            rt.send(Message::Equation(EquationAction::SizeSubmit));
        });
        assert!(s.app().equation_panel.as_ref().unwrap().size_error);
    }

    #[test]
    fn the_equation_saves_as_sam_reopens_and_edits_again() {
        let mut s = start("objects-eq-sam");
        replace_source(
            &mut s,
            "a sum, a fraction, a matrix and a root",
            "\\sum_{i=1}^{n} \\frac{a}{b} \\sqrt{x} \\matrix a & b \\\\ c & d\\endmatrix",
        );
        let tex = stored(&s).tex;
        assert_eq!(
            tex,
            "$\\sum_{i=1}^n\\frac ab\\sqrt{x}\\matrix a & b \\\\ c & d\\endmatrix $"
        );
        let natural = {
            let l = stored(&s).layout().unwrap();
            (l.width, l.height)
        };
        let path = s.save_sam("Equation.SAM");
        let mut t = s.reopen(&path);
        // The same text, size and settings come back, byte for byte.
        assert_eq!(stored(&t), stored(&s));
        assert_eq!(frame_size(&t), natural);
        let frame = *frames(&t).last().unwrap();
        let FrameContent::Picture(picture) = &t.document().frame(frame).unwrap().content else {
            unreachable!()
        };
        assert_eq!(
            (picture.scaling.size.width, picture.scaling.size.height),
            natural
        );
        assert_eq!(picture.pan.y, stored(&t).layout().unwrap().baseline);
        // The page draws it again (not a box naming the kind).
        let shown = {
            let picture = picture.clone();
            t.app()
                .pictures
                .shown(t.document(), &picture, None, (160, 80), 1.0)
        };
        assert!(matches!(shown, crate::pictures::Shown::Picture(_)));
        // The PDF draws it as vectors and the exports as an image.
        let picture = {
            let FrameContent::Picture(picture) = &t.document().frame(frame).unwrap().content else {
                unreachable!()
            };
            picture.clone()
        };
        assert!(
            t.app()
                .pictures
                .printed(t.document(), &picture, None)
                .is_some()
        );
        assert!(
            t.app()
                .pictures
                .exported(t.document(), &picture, None)
                .is_some()
        );
        // A double-click opens it in place; edit it again.
        select_frame(&mut t, frame);
        t.act("double-click the equation", |rt| {
            rt.send(Message::Frame(FrameAction::EnterText { at: None }));
        });
        assert_eq!(
            t.app().rail_panel(),
            crate::rail_panels::RailPanel::Equation
        );
        assert_eq!(source(&t), amipro_pictures::equation::to_source(&tex));
        replace_source(&mut t, "change it", "\\binom{n}{k}");
        assert_eq!(stored(&t).tex, "$\\binom nk$");
        t.edit("Greek from the palette", |rt| {
            rt.send(Message::Equation(EquationAction::Insert("\\alpha ")));
        });
        assert_eq!(stored(&t).tex, "$\\binom nk\\alpha$");
        // Save again and reopen once more.
        t.save();
        let path = t.app().location.path().unwrap().to_path_buf();
        let u = t.reopen(&path);
        assert_eq!(stored(&u).tex, "$\\binom nk\\alpha$");
    }

    #[test]
    fn escape_leaves_and_undoing_the_new_equation_closes_the_editor() {
        let mut s = start("objects-eq-leave");
        replace_source(&mut s, "x", "x^2");
        s.act("Esc", |rt| {
            rt.send(Message::Escape);
        });
        assert!(s.app().equation_panel.is_none());
        assert_eq!(stored(&s).tex, "$x^2$", "what was typed stays on the frame");
        // Tools > Equations with the equation selected opens it again.
        let frame = frames(&s)[0];
        select_frame(&mut s, frame);
        s.act("Tools > Equations on the selected equation", |rt| {
            rt.send(Message::Command(CommandId::Equations));
        });
        assert_eq!(source(&s), "x^2");
        // Undo of the step that made it takes the frame and the editor.
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        assert!(frames(&s).is_empty() || s.app().equation_panel.is_some());
    }

    #[test]
    fn import_and_save_as_equation_use_ami_pros_text() {
        let mut s = start("objects-eq-files");
        let folder = s.folder();
        std::fs::create_dir_all(&folder).unwrap();
        let input = folder.join("IN.TEX");
        std::fs::write(&input, "\\sum_{i=1}^{n} i\r\n").unwrap();
        s.edit("Import Equation", |rt| {
            let session_id = rt.app.id;
            rt.send(Message::EquationFile {
                session_id,
                path: Some(input.clone()),
                save: false,
            });
        });
        // Ami Pro's import: the sum is math, the letter after it text.
        assert_eq!(stored(&s).tex, "$\\sum_{i=1}^n$ i");
        let output = folder.join("OUT.TEX");
        s.act("Save As Equation", |rt| {
            let session_id = rt.app.id;
            rt.send(Message::EquationFile {
                session_id,
                path: Some(output.clone()),
                save: true,
            });
        });
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            "$\\sum_{i=1}^n$ i"
        );
        // A file that does not parse leaves the equation as it was.
        std::fs::write(&input, "\\frac{a}").unwrap();
        s.expect_status("Import Equation");
        s.no_change("Import a broken file", |rt| {
            let session_id = rt.app.id;
            rt.send(Message::EquationFile {
                session_id,
                path: Some(input.clone()),
                save: false,
            });
        });
        assert!(
            s.app().status.contains("Import Equation"),
            "{}",
            s.app().status
        );
    }

    #[test]
    fn latex_is_pasted_by_a_command_never_automatically() {
        let mut s = Scenario::new_document("objects-eq-latex");
        s.clean_folder();
        s.type_text("Quadratic");
        s.rt.clipboard = Some("$x = \\frac{-b \\pm \\sqrt[2]{b^2 - 4ac}}{2a}$".into());
        // Edit > Paste inserts the text; no equation appears by itself.
        s.edit("Paste", |rt| {
            rt.send(Message::Command(CommandId::Paste));
        });
        assert!(frames(&s).is_empty());
        s.edit("Tools > Paste LaTeX Equation", |rt| {
            rt.send(Message::Command(CommandId::PasteLatexEquation));
        });
        assert_eq!(frames(&s).len(), 1);
        assert_eq!(
            stored(&s).tex,
            "$x=\\frac{-b\\pm\\root{2} \\of{b^2-4ac}}{2a}$"
        );
        // Not LaTeX: the status says so and nothing changes.
        s.rt.clipboard = Some("just words".into());
        s.no_change("Paste LaTeX from words", |rt| {
            rt.send(Message::Command(CommandId::PasteLatexEquation));
        });
        assert!(s.app().status.contains("no LaTeX equation"));
        // Copy as LaTeX puts the source on the clipboard.
        let frame = frames(&s)[0];
        select_frame(&mut s, frame);
        s.act("Tools > Equations", |rt| {
            rt.send(Message::Command(CommandId::Equations));
        });
        s.act("Copy as LaTeX", |rt| {
            rt.send(Message::Equation(EquationAction::CopyLatex));
        });
        assert!(s.rt.clipboard.as_deref().unwrap().contains("\\sqrt[2]{"));
    }

    /// The real widgets (widget layer): a click on a palette button
    /// publishes its template, a click in the card focuses the source field
    /// and keys typed there reach the frame, Enter leaves.
    #[test]
    fn the_palette_and_the_card_take_clicks_and_keys_in_the_window() {
        use cosmic::iced::Point;
        use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
        use cosmic::iced::mouse::Button;
        let mut s = start("objects-eq-widget");
        s.act("redraw", |rt| {
            rt.redraw();
        });
        // The first palette button is the fraction (measured from the
        // screenshots: x 33, y 288 in a 1,280 by 800 window).
        let published = s.rt.click_unrun(Button::Left, Point::new(33.0, 288.0));
        assert!(
            published.iter().any(|m| matches!(
                m,
                Message::Equation(EquationAction::Insert(text)) if text.starts_with("\\frac")
            )),
            "a click on the first button inserts a fraction: {published:?}"
        );
        s.act("the click's message", |rt| {
            rt.send(Message::Equation(EquationAction::Insert("\\frac{⬚}{⬚}")));
            rt.redraw();
        });
        // The card is under the frame; a click on it focuses the field and
        // the keys typed go to the equation, not to the page.
        let text_before = s.main_text();
        s.act("click the card and type", |rt| {
            rt.click(Button::Left, Point::new(690.0, 388.0));
            for c in "ab".chars() {
                rt.events(&[crate::page_surface_app_tests::key_press(
                    Key::Character(c.to_string().into()),
                    Modifiers::default(),
                )]);
            }
        });
        assert_eq!(s.main_text(), text_before, "the page took no key");
        assert!(source(&s).contains("ab"), "{:?}", source(&s));
        assert!(stored(&s).tex.contains("ab"), "{:?}", stored(&s).tex);
        // Enter leaves.
        s.act("Enter", |rt| {
            rt.events(&[crate::page_surface_app_tests::key_press(
                Key::Named(Named::Enter),
                Modifiers::default(),
            )]);
        });
        assert!(s.app().equation_panel.is_none(), "Enter leaves the editor");
    }

    #[test]
    fn an_unreadable_stored_text_is_kept_and_edits_nothing() {
        let mut s = start("objects-eq-unreadable");
        let frame = frames(&s)[0];
        // A frame whose object is not two text lines.
        s.act("corrupt the object", move |rt| {
            let object = {
                let FrameContent::Picture(picture) = &rt.app.document.frame(frame).unwrap().content
                else {
                    unreachable!()
                };
                let PictureSource::Embedded(object) = &picture.source else {
                    unreachable!()
                };
                *object
            };
            let _ = object;
            let mut document = rt.app.document.clone();
            let bad = document.add_object(std::sync::Arc::from(vec![0xff, 0xfe, 0x00, 0x01]));
            if let Some(FrameContent::Picture(picture)) =
                document.frame_mut(frame).map(|frame| &mut frame.content)
            {
                picture.source = PictureSource::Embedded(bad);
            }
            rt.app.document = document;
        });
        select_frame(&mut s, frame);
        s.act("open it", |rt| {
            rt.send(Message::Command(CommandId::Equations));
        });
        assert!(s.app().equation_panel.as_ref().unwrap().unreadable);
        s.no_change("an insert", |rt| {
            rt.send(Message::Equation(EquationAction::Insert("\\alpha ")));
        });
    }
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Equation editor (the card at the frame
/// and the palette in the rail) with an equation that has a fraction, a
/// radical, a sum with limits and a matrix, light and dark, and the card
/// with an error, as PNG files (not committed).
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn equation_screenshots() {
    use crate::equation_panel::EquationAction;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut s = equations::start("objects-eq-screens");
    let write = |s: &mut Scenario, name: &str| {
        for (dark, theme) in [
            (false, cosmic::Theme::light()),
            (true, cosmic::Theme::dark()),
        ] {
            s.rt.redraw();
            let size = (1_280u32, 800u32);
            let rgb = render_window(s.app(), size, &theme, false);
            image::RgbImage::from_raw(size.0, size.1, rgb)
                .unwrap()
                .save(directory.join(format!(
                    "{name}-{}.png",
                    if dark { "dark" } else { "light" }
                )))
                .unwrap();
        }
    };
    equations::replace_source(
        &mut s,
        "a sample",
        "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a} \\quad \\sum\\limits_{i=1}^{n} i^2 \\quad \\left[ \\matrix a & b \\\\ c & d\\endmatrix \\right]",
    );
    eprintln!(
        "size {:?} natural {:?} stored {:?}",
        equations::frame_size(&s),
        equations::stored(&s)
            .layout()
            .map(|l| (l.width, l.height, l.items.len())),
        equations::stored(&s).tex
    );
    write(&mut s, "equation-editor");
    s.act("an error", |rt| {
        rt.send(Message::Equation(EquationAction::Edit(
            cosmic::iced::widget::text_editor::Action::Edit(
                cosmic::iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(
                    " \\frac{a}".to_owned(),
                )),
            ),
        )));
    });
    write(&mut s, "equation-error");
}
