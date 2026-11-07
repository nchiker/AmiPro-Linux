//! Pictures on the page (`docs/Design/UI_SPEC.md` §25, decisions P1–P4):
//! each picture decoded once and kept per session, off the typing path.
//!
//! Bitmaps, metafiles, and Ami Pro's screen image go through the project's
//! own readers (`amipro-pictures`), Ami Pro's drawings and charts from
//! their objects (`sdw`); PNG, JPEG, GIF, and TIFF through
//! `image`, PCX through `image-extras`, SVG through `resvg`. Metafiles and
//! SVG are drawn with tiny-skia at the size they are shown, so they stay
//! sharp at every zoom. A kind no reader takes is shown from Ami Pro's
//! screen image, else as a labelled box (P4). A linked picture is read from
//! its file as it is now: its path, else its name in the document's folder
//! in any case (P2).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

use amipro_document::{Document, Picture, PictureFormat, PictureSource, Size};
use amipro_pictures::{Raster, adjust, dib, emf, ole, screen, sdw, wmf};
use cosmic::iced::advanced::image::Handle;

use crate::metafile_render;

/// The longest side a picture is drawn from: larger bitmaps are reduced
/// once, larger vector pictures drawn at this size and scaled.
const MAX_SIDE: u32 = 4_096;
/// How many pictures a session keeps decoded before the least recently
/// used go.
const KEPT: usize = 256;

/// What the page draws for a picture.
#[derive(Clone, Debug)]
pub(crate) enum Shown {
    /// The picture as an image, to be stretched to its rectangle.
    Picture(Handle),
    /// Ami Pro's screen image of the picture area (P4).
    Screen(Handle),
    /// A box with this label (P4).
    Box(String),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Key {
    /// Bytes in the document, by their allocation, which the entry keeps
    /// alive so that the address stays theirs.
    Bytes(usize, usize),
    File(PathBuf),
}

enum Content {
    Raster(Handle),
    Metafile(Arc<wmf::Drawing>),
    Svg(Arc<resvg::usvg::Tree>),
    Unreadable,
}

struct Entry {
    _hold: Option<Arc<[u8]>>,
    modified: Option<SystemTime>,
    natural: Option<Size>,
    content: Content,
    /// A vector picture drawn at a pixel size and rotation.
    drawn: Option<((u32, u32, u16), Handle)>,
    /// A raster picture with its Image Processing settings applied
    /// (DO.5, OD8; the bytes are never changed).
    adjusted: Option<(adjust::Settings, Handle)>,
    /// A raster picture turned by a rotation, with the settings it was
    /// adjusted by.
    turned: Option<((u16, adjust::Settings), Handle)>,
    used: u64,
}

/// A session's decoded pictures.
#[derive(Default)]
pub(crate) struct Pictures {
    cache: RefCell<HashMap<Key, Entry>>,
    clock: std::cell::Cell<u64>,
    /// The stand-ins the PDF draws as vectors, by their handles' ids.
    printed: RefCell<HashMap<cosmic::iced::advanced::image::Id, crate::pdf_pictures::Printed>>,
}

impl std::fmt::Debug for Pictures {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Pictures({})", self.cache.borrow().len())
    }
}

/// The bytes of a picture's file, by the decision P2 rule, with the time
/// it last changed.
pub(crate) fn linked_file(path: &str, folder: Option<&Path>) -> Option<(PathBuf, SystemTime)> {
    let direct = Path::new(path);
    if direct.is_absolute()
        && let Ok(metadata) = std::fs::metadata(direct)
        && metadata.is_file()
    {
        return Some((direct.to_owned(), metadata.modified().ok()?));
    }
    let name = path.rsplit(['\\', '/']).next()?.to_lowercase();
    let folder = folder?;
    std::fs::read_dir(folder).ok()?.flatten().find_map(|entry| {
        let metadata = entry.metadata().ok()?;
        (metadata.is_file() && entry.file_name().to_string_lossy().to_lowercase() == name)
            .then(|| Some((entry.path(), metadata.modified().ok()?)))
            .flatten()
    })
}

/// The pixels a metre a PNG (`pHYs`) or JPEG (`JFIF`) gives, if any.
fn resolution(bytes: &[u8]) -> Option<(i32, i32)> {
    if bytes.starts_with(b"\x89PNG") {
        let mut at = 8;
        while at + 8 <= bytes.len() {
            let length = u32::from_be_bytes(bytes[at..at + 4].try_into().ok()?) as usize;
            let kind = &bytes[at + 4..at + 8];
            if kind == b"pHYs" && length == 9 {
                let data = bytes.get(at + 8..at + 17)?;
                let x = u32::from_be_bytes(data[0..4].try_into().ok()?);
                let y = u32::from_be_bytes(data[4..8].try_into().ok()?);
                return (data[8] == 1).then_some((x as i32, y as i32));
            }
            if kind == b"IDAT" {
                return None;
            }
            at = at.checked_add(12 + length)?;
        }
        return None;
    }
    if bytes.starts_with(&[0xff, 0xd8]) && bytes.get(6..11) == Some(b"JFIF\0") {
        let units = *bytes.get(13)?;
        let x = f64::from(u16::from_be_bytes([*bytes.get(14)?, *bytes.get(15)?]));
        let y = f64::from(u16::from_be_bytes([*bytes.get(16)?, *bytes.get(17)?]));
        let per_metre = match units {
            1 => 39.370_08,
            2 => 100.0,
            _ => return None,
        };
        return Some((
            (x * per_metre).round() as i32,
            (y * per_metre).round() as i32,
        ));
    }
    None
}

fn twips(size: amipro_pictures::Twips) -> Size {
    Size {
        width: size.width,
        height: size.height,
    }
}

/// The metafile extent Ami Pro keeps in the driver's state (words 2 and 3,
/// in twips; PI2), the only record of a plain metafile's size.
pub(crate) fn kept_extent(picture: &Picture) -> Option<Size> {
    let values: Vec<&str> = picture.legacy.get("isd")?.split('\n').collect();
    let count: usize = values.get(11)?.trim().parse().ok()?;
    if count < 4 {
        return None;
    }
    let word = |index: usize| -> Option<i32> { values.get(12 + index)?.trim().parse().ok() };
    let size = Size {
        width: word(2)?,
        height: word(3)?,
    };
    size.is_valid().then_some(size)
}

/// A reduced copy of a raster no side of which passes `MAX_SIDE`.
fn reduced(raster: Raster) -> Raster {
    let longest = raster.width.max(raster.height);
    if longest <= MAX_SIDE {
        return raster;
    }
    let scale = f64::from(MAX_SIDE) / f64::from(longest);
    let width = ((f64::from(raster.width) * scale).round() as u32).max(1);
    let height = ((f64::from(raster.height) * scale).round() as u32).max(1);
    let Some(image) = image::RgbaImage::from_raw(raster.width, raster.height, raster.pixels) else {
        return Raster {
            width: 1,
            height: 1,
            pixels: vec![0; 4],
        };
    };
    let small =
        image::imageops::resize(&image, width, height, image::imageops::FilterType::Triangle);
    Raster {
        width,
        height,
        pixels: small.into_raw(),
    }
}

fn raster_handle(raster: Raster) -> Handle {
    let raster = reduced(raster);
    Handle::from_rgba(raster.width, raster.height, raster.pixels)
}

/// The system's fonts, for text in SVG pictures, loaded once.
fn svg_fonts() -> Arc<resvg::usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut fonts = resvg::usvg::fontdb::Database::new();
            fonts.load_system_fonts();
            Arc::new(fonts)
        })
        .clone()
}

/// Decodes a picture's bytes: what to draw and its natural size in twips
/// (`kept` the metafile extent Ami Pro kept, if any).
fn decode(format: PictureFormat, bytes: &[u8], kept: Option<Size>) -> (Content, Option<Size>) {
    match format {
        PictureFormat::Bitmap => {
            let decoded = if bytes.starts_with(b"BM") {
                dib::bmp_header(bytes).zip(dib::decode_bmp(bytes))
            } else {
                dib::Header::read(bytes).zip(dib::decode_packed(bytes, dib::ColourTable::Rgb))
            };
            match decoded {
                Some((header, raster)) => (
                    Content::Raster(raster_handle(raster)),
                    Some(twips(header.natural_size())),
                ),
                None => (Content::Unreadable, None),
            }
        }
        PictureFormat::Metafile => match wmf::play(bytes) {
            Some(drawing) => {
                let natural = kept.or_else(|| drawing.placeable.map(twips)).or_else(|| {
                    // A plain metafile nobody sized: its window as
                    // twips, at most 22 in.
                    let (width, height) = drawing.window;
                    let scale = (31_680.0 / f64::from(width.max(height).max(1))).min(1.0);
                    Some(Size {
                        width: (f64::from(width) * scale) as i32,
                        height: (f64::from(height) * scale) as i32,
                    })
                });
                (Content::Metafile(Arc::new(drawing)), natural)
            }
            None => (Content::Unreadable, None),
        },
        PictureFormat::Emf => match emf::play(bytes) {
            Some(drawing) => {
                let natural = kept.or_else(|| drawing.placeable.map(twips));
                (Content::Metafile(Arc::new(drawing)), natural)
            }
            None => (Content::Unreadable, None),
        },
        PictureFormat::Svg => {
            let options = resvg::usvg::Options {
                fontdb: svg_fonts(),
                ..resvg::usvg::Options::default()
            };
            match resvg::usvg::Tree::from_data(bytes, &options) {
                Ok(tree) => {
                    let size = tree.size();
                    // CSS pixels are 1/96 in, 15 twips.
                    let natural = dib::raster_natural_size(
                        size.width().ceil() as u32,
                        size.height().ceil() as u32,
                        None,
                    );
                    (Content::Svg(Arc::new(tree)), Some(twips(natural)))
                }
                Err(_) => (Content::Unreadable, None),
            }
        }
        PictureFormat::Png
        | PictureFormat::Jpeg
        | PictureFormat::Gif
        | PictureFormat::Tiff
        | PictureFormat::GrayTiff
        | PictureFormat::Pcx
        | PictureFormat::Other => {
            image_extras::register();
            match image::load_from_memory(bytes) {
                Ok(image) => {
                    let rgba = image.into_rgba8();
                    let (width, height) = rgba.dimensions();
                    let natural = dib::raster_natural_size(width, height, resolution(bytes));
                    let raster = Raster {
                        width,
                        height,
                        pixels: rgba.into_raw(),
                    };
                    (Content::Raster(raster_handle(raster)), Some(twips(natural)))
                }
                Err(_) => (Content::Unreadable, None),
            }
        }
        // Ami Pro's drawings and charts, from their objects (`UI_SPEC.md`
        // §33, DW7), at the size `LIBSIZE` gives them.
        PictureFormat::Drawing | PictureFormat::Chart => match sdw::read(bytes) {
            Some(drawing) => (
                Content::Metafile(Arc::new(sdw::paint(&drawing))),
                Some(twips(drawing.natural_size())),
            ),
            None => (Content::Unreadable, None),
        },
        // OLE objects from the metafile of their stored presentation
        // (DO.3, OD7); one without a readable presentation keeps the
        // screen image or the box (P4, OD9).
        PictureFormat::Ole => match ole_drawing(bytes) {
            Some((drawing, natural)) => (Content::Metafile(Arc::new(drawing)), Some(natural)),
            None => (Content::Unreadable, None),
        },
        // Equations (DO.9): Ami Pro repaints them from their TeX, so the
        // app lays the text out and draws it.
        PictureFormat::Equation => match equation_drawing(bytes) {
            Some((drawing, natural)) => (Content::Metafile(Arc::new(drawing)), Some(natural)),
            None => (Content::Unreadable, None),
        },
        // EPS: its preview.
        _ => (Content::Unreadable, None),
    }
}

/// The drawing of an equation object's bytes (two text lines) and its
/// natural size in twips; `None` when the text does not parse or lay out.
pub(crate) fn equation_drawing(bytes: &[u8]) -> Option<(wmf::Drawing, Size)> {
    let stored = amipro_pictures::equation::Stored::read(bytes)?;
    let layout = stored.layout().ok()?;
    let (face, _) = stored.face();
    let drawing = layout.to_drawing(&face, amipro_pictures::equation::layout::Colours::BLACK);
    Some((
        drawing,
        Size {
            width: layout.width,
            height: layout.height,
        },
    ))
}

/// The drawing of an OLE object stream's presentation metafile and its
/// natural size: the presentation's width and height are in hundredths of
/// a millimetre (the height is negative in the files), 1/2540 in.
pub(crate) fn ole_drawing(bytes: &[u8]) -> Option<(wmf::Drawing, Size)> {
    let object = ole::read(bytes)?;
    let drawing = wmf::play(object.metafile(bytes)?)?;
    if drawing.ops.is_empty() {
        return None;
    }
    let presentation = object.presentation.as_ref()?;
    let twips = |value: u32| {
        let value = i64::from((value as i32).unsigned_abs());
        (value * 1_440 / 2_540).clamp(1, 31_680) as i32
    };
    let natural = if presentation.width == 0 || presentation.height == 0 {
        let (width, height) = drawing.window;
        let scale = (31_680.0 / f64::from(width.max(height).max(1))).min(1.0);
        Size {
            width: (f64::from(width) * scale) as i32,
            height: (f64::from(height) * scale) as i32,
        }
    } else {
        Size {
            width: twips(presentation.width),
            height: twips(presentation.height),
        }
    };
    Some((drawing, natural))
}

/// The natural size, in twips, of a picture file's bytes of `format`, or
/// `None` when no reader takes them (Insert › Image… refuses the file).
pub(crate) fn natural_of(format: PictureFormat, bytes: &[u8]) -> Option<Size> {
    match format {
        // Raster files by their headers alone: a large photo is not decoded
        // to learn its size.
        PictureFormat::Png
        | PictureFormat::Jpeg
        | PictureFormat::Gif
        | PictureFormat::Tiff
        | PictureFormat::GrayTiff
        | PictureFormat::Pcx => {
            image_extras::register();
            let (width, height) = image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()
                .ok()?
                .into_dimensions()
                .ok()?;
            Some(twips(dib::raster_natural_size(
                width,
                height,
                resolution(bytes),
            )))
        }
        _ => match decode(format, bytes, None) {
            (Content::Unreadable, _) => None,
            (_, natural) => natural,
        },
    }
}

/// The most dots per inch a picture Save As converts keeps at its size on
/// the page (P8).
const SAM_DPI: i64 = 300;

/// `document` as Save As writes it to an Ami Pro file (P8, P2): pictures in
/// formats Ami Pro cannot read become 24-bit Windows bitmaps, reduced to
/// 300 dpi at their size on the page and keeping their natural size (the
/// bitmap's resolution says it, as `AmiPaint` reads it); pictures linked to
/// files Ami Pro cannot reach are copied in. Ami Pro's kept records go
/// with the old bytes; the writer writes new ones. The loss check lists
/// both before the user saves.
pub(crate) fn for_ami_pro(document: &Document, folder: Option<&Path>) -> Document {
    let mut out = document.clone();
    let frames: Vec<amipro_document::FrameId> = document
        .frames()
        .filter(|frame| matches!(frame.content, amipro_document::FrameContent::Picture(_)))
        .map(|frame| frame.id)
        .collect();
    for id in frames {
        let Some(frame) = out.frame(id).cloned() else {
            continue;
        };
        let amipro_document::FrameContent::Picture(mut picture) = frame.content else {
            continue;
        };
        let linux_link = matches!(&picture.source,
            PictureSource::Linked(path) if !amipro_formats::ami_pro_path(path));
        // Image adjustments of a bitmap other than a gray-scale TIFF are
        // baked into the bitmap Save As writes (OD8).
        let bake = !picture.adjust.is_neutral() && !picture.is_gray_tiff();
        let convert = !picture.ami_pro_stores() || bake;
        if !linux_link && !convert {
            continue;
        }
        let bytes = match &picture.source {
            PictureSource::Embedded(object) => out.objects().get(object).map(|o| o.bytes.to_vec()),
            PictureSource::Linked(path) => {
                linked_file(path, folder).and_then(|(file, _)| std::fs::read(file).ok())
            }
        };
        let Some(mut bytes) = bytes else {
            continue;
        };
        if convert {
            let Some(converted) = sam_bitmap(&picture, &frame.placement, &frame.style, &bytes)
            else {
                continue;
            };
            bytes = converted;
            picture.extension = ".bmp".into();
            picture.adjust = amipro_document::ImageAdjust::default();
        }
        picture.source = PictureSource::Embedded(out.add_object(Arc::from(bytes)));
        picture.rendering = None;
        picture.legacy = amipro_document::LegacyData::default();
        if let Some(target) = out.frame_mut(id) {
            target.content = amipro_document::FrameContent::Picture(picture);
        }
    }
    out.drop_unreferenced_objects();
    out
}

/// A picture's bytes as a 24-bit bitmap for Ami Pro (P8).
fn sam_bitmap(
    picture: &Picture,
    placement: &amipro_document::Placement,
    style: &amipro_document::FrameStyle,
    bytes: &[u8],
) -> Option<Vec<u8>> {
    let format = picture.format();
    let natural = natural_of(format, bytes)?;
    // Its size on the page: the scaling in the frame's picture area.
    let area = placement.size().map_or(natural, |size| {
        let insets = style.text_insets(size.width, size.height);
        Size {
            width: (size.width - insets.left - insets.right).max(1),
            height: (size.height - insets.top - insets.bottom).max(1),
        }
    });
    let shown = picture.scaling.drawn_size(natural, area);
    let most = |twips: i32| (i64::from(twips.max(1)) * SAM_DPI / 1_440).max(1) as u32;
    let (most_wide, most_high) = (most(shown.width), most(shown.height));
    let raster = match format {
        PictureFormat::Svg => {
            let options = resvg::usvg::Options {
                fontdb: svg_fonts(),
                ..resvg::usvg::Options::default()
            };
            let tree = resvg::usvg::Tree::from_data(bytes, &options).ok()?;
            let pixmap = svg_pixmap(&tree, (most_wide.min(MAX_SIDE), most_high.min(MAX_SIDE)))?;
            Raster {
                width: pixmap.width(),
                height: pixmap.height(),
                pixels: metafile_render::rgba_of(&pixmap),
            }
        }
        PictureFormat::Emf => {
            // Drawn at 300 dpi, a screen pixel's lines three pixels wide.
            let drawing = emf::play(bytes)?;
            let pixel = SAM_DPI as f32 / 96.0;
            let pixmap = metafile_render::render(
                &drawing,
                most_wide.min(MAX_SIDE),
                most_high.min(MAX_SIDE),
                pixel,
            )?;
            Raster {
                width: pixmap.width(),
                height: pixmap.height(),
                pixels: metafile_render::rgba_of(&pixmap),
            }
        }
        _ => {
            image_extras::register();
            let image = image::load_from_memory(bytes).ok()?.into_rgba8();
            let (width, height) = image.dimensions();
            let scale = (f64::from(most_wide) / f64::from(width))
                .min(f64::from(most_high) / f64::from(height))
                .min(1.0);
            let image = if scale < 1.0 {
                image::imageops::resize(
                    &image,
                    ((f64::from(width) * scale).round() as u32).max(1),
                    ((f64::from(height) * scale).round() as u32).max(1),
                    image::imageops::FilterType::Triangle,
                )
            } else {
                image
            };
            let (width, height) = image.dimensions();
            Raster {
                width,
                height,
                pixels: image.into_raw(),
            }
        }
    };
    let raster = if picture.adjust.is_neutral() || picture.is_gray_tiff() {
        raster
    } else {
        adjust::apply(&raster, settings_of(picture))
    };
    // The resolution that gives the natural size back (`AmiPaint`: pixels
    // × 56,693 ÷ pixels a metre).
    let ppm = |pixels: u32, twips: i32| {
        ((i64::from(pixels) * 56_693 + i64::from(twips.max(1)) / 2) / i64::from(twips.max(1)))
            .clamp(1, i64::from(i32::MAX)) as i32
    };
    Some(dib::encode_bmp24(
        &raster,
        (
            ppm(raster.width, natural.width),
            ppm(raster.height, natural.height),
        ),
    ))
}

/// The label of a box drawn for a picture no reader takes.
fn label(picture: &Picture) -> String {
    match &picture.source {
        PictureSource::Linked(path) => format!("{} not found: {path}", picture.format().name()),
        PictureSource::Embedded(_) => picture.format().name().to_owned(),
    }
}

impl Pictures {
    /// The cache key of a picture's source, decoding it on first use.
    fn entry(&self, document: &Document, picture: &Picture, folder: Option<&Path>) -> Option<Key> {
        let now = self.clock.get() + 1;
        self.clock.set(now);
        let mut cache = self.cache.borrow_mut();
        let (key, hold, bytes, modified) = match &picture.source {
            PictureSource::Embedded(object) => {
                let bytes = Arc::clone(&document.objects().get(object)?.bytes);
                let key = Key::Bytes(bytes.as_ptr() as usize, bytes.len());
                if let Some(entry) = cache.get_mut(&key) {
                    entry.used = now;
                    return Some(key);
                }
                (key, Some(Arc::clone(&bytes)), bytes.to_vec(), None)
            }
            PictureSource::Linked(path) => {
                let (file, modified) = linked_file(path, folder)?;
                let key = Key::File(file.clone());
                if let Some(entry) = cache.get_mut(&key)
                    && entry.modified == Some(modified)
                {
                    entry.used = now;
                    return Some(key);
                }
                (key, None, std::fs::read(&file).ok()?, Some(modified))
            }
        };
        let format = match &picture.source {
            // A linked file is read by its own name's kind (Ami Pro's linked
            // TIFFs name the bitmap driver).
            PictureSource::Linked(path) => PictureFormat::from_extension(
                Path::new(&path.replace('\\', "/"))
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or(&picture.extension),
            ),
            PictureSource::Embedded(_) => picture.format(),
        };
        let (content, natural) = decode(format, &bytes, kept_extent(picture));
        if cache.len() >= KEPT {
            let mut ages: Vec<u64> = cache.values().map(|entry| entry.used).collect();
            ages.sort_unstable();
            let cut = ages[ages.len() / 2];
            cache.retain(|_, entry| entry.used > cut);
        }
        cache.insert(
            key.clone(),
            Entry {
                _hold: hold,
                modified,
                natural,
                content,
                drawn: None,
                adjusted: None,
                turned: None,
                used: now,
            },
        );
        Some(key)
    }

    /// The picture's natural size in twips, when a reader takes it.
    pub(crate) fn natural_size(
        &self,
        document: &Document,
        picture: &Picture,
        folder: Option<&Path>,
    ) -> Option<Size> {
        let key = self.entry(document, picture, folder)?;
        self.cache.borrow().get(&key)?.natural
    }

    /// What to draw for `picture` shown at `pixels` (width, height), `pixel`
    /// of them to a screen pixel: the picture, else Ami Pro's screen image,
    /// else a box.
    pub(crate) fn shown(
        &self,
        document: &Document,
        picture: &Picture,
        folder: Option<&Path>,
        pixels: (u32, u32),
        pixel: f32,
    ) -> Shown {
        if let Some(key) = self.entry(document, picture, folder) {
            let mut cache = self.cache.borrow_mut();
            if let Some(entry) = cache.get_mut(&key) {
                let wanted = (pixels.0.clamp(1, MAX_SIDE), pixels.1.clamp(1, MAX_SIDE));
                let rotation = picture.scaling.rotation % 360;
                let settings = settings_of(picture);
                match &entry.content {
                    Content::Raster(handle) => {
                        let source = if settings.is_neutral() {
                            handle.clone()
                        } else if let Some((done, adjusted)) = &entry.adjusted
                            && *done == settings
                        {
                            adjusted.clone()
                        } else {
                            let adjusted = adjusted_handle(handle, settings);
                            entry.adjusted = Some((settings, adjusted.clone()));
                            adjusted
                        };
                        if rotation == 0 {
                            return Shown::Picture(source);
                        }
                        if let Some((turned, handle)) = &entry.turned
                            && *turned == (rotation, settings)
                        {
                            return Shown::Picture(handle.clone());
                        }
                        if let Some(turned) = handle_pixmap(&source).and_then(|pixmap| {
                            metafile_render::rotated(&pixmap, f32::from(rotation))
                        }) {
                            let handle = pixmap_handle(&turned);
                            entry.turned = Some(((rotation, settings), handle.clone()));
                            return Shown::Picture(handle);
                        }
                        return Shown::Picture(source);
                    }
                    Content::Metafile(_) | Content::Svg(_) => {
                        let key = (wanted.0, wanted.1, rotation);
                        if let Some((drawn, handle)) = &entry.drawn
                            && *drawn == key
                        {
                            return Shown::Picture(handle.clone());
                        }
                        let pixmap = match &entry.content {
                            Content::Metafile(drawing) => {
                                metafile_render::render(drawing, wanted.0, wanted.1, pixel)
                            }
                            Content::Svg(tree) => svg_pixmap(tree, wanted),
                            _ => None,
                        };
                        let pixmap = match pixmap {
                            Some(pixmap) if rotation != 0 => {
                                metafile_render::rotated(&pixmap, f32::from(rotation))
                            }
                            other => other,
                        };
                        if let Some(pixmap) = pixmap {
                            let handle = pixmap_handle(&pixmap);
                            entry.drawn = Some((key, handle.clone()));
                            return Shown::Picture(handle);
                        }
                    }
                    Content::Unreadable => {}
                }
            }
        }
        // Ami Pro's screen image (P4), else a box.
        if let Some(bytes) = picture
            .rendering
            .and_then(|object| document.objects().get(&object))
            .map(|object| Arc::clone(&object.bytes))
        {
            let key = Key::Bytes(bytes.as_ptr() as usize, bytes.len());
            let mut cache = self.cache.borrow_mut();
            let entry = cache.entry(key).or_insert_with(|| Entry {
                content: screen::decode(&bytes).map_or(Content::Unreadable, |raster| {
                    Content::Raster(raster_handle(raster))
                }),
                _hold: Some(Arc::clone(&bytes)),
                modified: None,
                natural: None,
                drawn: None,
                adjusted: None,
                turned: None,
                used: self.clock.get(),
            });
            if let Content::Raster(handle) = &entry.content {
                return Shown::Screen(handle.clone());
            }
        }
        Shown::Box(label(picture))
    }
}

impl Pictures {
    /// A stand-in for `picture` that the PDF draws as vectors (`UI_SPEC.md`
    /// §29, PO2): a metafile or SVG picture, unturned, whose every part has
    /// a PDF counterpart; `None` for the rest, which print as the screen
    /// draws them.
    pub(crate) fn printed(
        &self,
        document: &Document,
        picture: &Picture,
        folder: Option<&Path>,
    ) -> Option<Handle> {
        use crate::pdf_pictures::{Printed, metafile_is_vector, svg_is_vector};
        if !picture.scaling.rotation.is_multiple_of(360) {
            return None;
        }
        let key = self.entry(document, picture, folder)?;
        let printed = match &self.cache.borrow().get(&key)?.content {
            Content::Metafile(drawing) if metafile_is_vector(drawing) => {
                Printed::Metafile(Arc::clone(drawing))
            }
            Content::Svg(tree) if svg_is_vector(tree) => Printed::Svg(Arc::clone(tree)),
            _ => return None,
        };
        let handle = Handle::from_rgba(1, 1, vec![0; 4]);
        self.printed.borrow_mut().insert(handle.id(), printed);
        Some(handle)
    }
}

#[cfg(test)]
impl Pictures {
    /// What keeps a metafile from the PDF's vectors (the census).
    pub(crate) fn blockers(
        &self,
        document: &Document,
        picture: &Picture,
        folder: Option<&Path>,
    ) -> Option<std::collections::BTreeSet<String>> {
        let key = self.entry(document, picture, folder)?;
        match &self.cache.borrow().get(&key)?.content {
            Content::Metafile(drawing) => Some(crate::pdf_pictures::blockers(drawing)),
            _ => None,
        }
    }
}

/// Pictures handed to the PDF as themselves (`UI_SPEC.md` §29, PO2).
impl crate::pdf::VectorPictures for Pictures {
    fn draw(
        &self,
        handle: &Handle,
        surface: &mut krilla::surface::Surface<'_>,
        resources: &mut crate::pdf::PdfResources,
        bounds: cosmic::iced::Rectangle,
    ) -> bool {
        let Some(printed) = self.printed.borrow().get(&handle.id()).cloned() else {
            return false;
        };
        match printed {
            crate::pdf_pictures::Printed::Metafile(drawing) => {
                crate::pdf_pictures::draw_metafile(surface, resources, &drawing, bounds);
            }
            crate::pdf_pictures::Printed::Svg(tree) => {
                crate::pdf_pictures::draw_svg(surface, &tree, bounds);
            }
        }
        true
    }
}

impl Pictures {
    /// `picture` as the export formats take it (`UI_SPEC.md` §30, EC3): a
    /// metafile stays a Windows metafile (with a placeable header), PNG and
    /// JPEG stay themselves, SVG goes with a PNG of it for programs that
    /// cannot draw it, and every other picture (bitmaps, GIF, TIFF, PCX,
    /// turned pictures, Ami Pro's drawings and charts from their screen
    /// image) is a PNG of what the page shows, at 300 dpi of its natural
    /// size; with that natural size (none for a screen image, which covers
    /// its picture area).
    pub(crate) fn exported(
        &self,
        document: &Document,
        picture: &Picture,
        folder: Option<&Path>,
    ) -> Option<amipro_exchange::build::SunkPicture> {
        use amipro_exchange::{Image, ImageKind};
        let natural = self.natural_size(document, picture, folder);
        let unturned = picture.scaling.rotation.is_multiple_of(360);
        let source: Option<Arc<[u8]>> = match &picture.source {
            PictureSource::Embedded(object) => document
                .objects()
                .get(object)
                .map(|object| Arc::clone(&object.bytes)),
            PictureSource::Linked(path) => linked_file(path, folder)
                .and_then(|(file, _)| std::fs::read(file).ok())
                .map(Arc::from),
        };
        let image = |bytes: Arc<[u8]>, kind, fallback| Image {
            bytes,
            kind,
            fallback,
            pixels: None,
        };
        if unturned
            && picture.adjust.is_neutral()
            && let (Some(bytes), Some(natural)) = (&source, natural)
        {
            let found = match picture.format() {
                PictureFormat::Metafile => self
                    .placeable(document, picture, folder, bytes, natural)
                    .map(|wmf| image(wmf.into(), ImageKind::Wmf, None)),
                PictureFormat::Emf => Some(image(Arc::clone(bytes), ImageKind::Emf, None)),
                PictureFormat::Png => Some(image(Arc::clone(bytes), ImageKind::Png, None)),
                PictureFormat::Jpeg => Some(image(Arc::clone(bytes), ImageKind::Jpeg, None)),
                PictureFormat::Svg => self
                    .png_of(document, picture, folder, natural)
                    .map(|(png, _)| image(Arc::clone(bytes), ImageKind::Svg, Some(png.into()))),
                // Drawings and charts as SVG drawn from their objects, with
                // a PNG for programs that take no SVG (`UI_SPEC.md` §33).
                PictureFormat::Drawing | PictureFormat::Chart => {
                    sdw::read(bytes).and_then(|drawing| {
                        let svg: Arc<[u8]> = crate::draw_files::svg_bytes(&drawing).into();
                        self.png_of(document, picture, folder, natural)
                            .map(|(png, _)| image(svg, ImageKind::Svg, Some(png.into())))
                    })
                }
                _ => None,
            };
            if let Some(found) = found {
                return Some(amipro_exchange::build::SunkPicture {
                    image: found,
                    natural: Some(natural),
                });
            }
        }
        let size = natural.unwrap_or(picture.scaling.size);
        let (png, pixels) = self.png_of(document, picture, folder, size)?;
        Some(amipro_exchange::build::SunkPicture {
            image: Image {
                bytes: png.into(),
                kind: ImageKind::Png,
                fallback: None,
                pixels: Some(pixels),
            },
            natural,
        })
    }

    /// What the page shows of `picture` as a PNG, drawn at 300 dpi of
    /// `size` (at most `MAX_SIDE` a side), and its pixels.
    fn png_of(
        &self,
        document: &Document,
        picture: &Picture,
        folder: Option<&Path>,
        size: Size,
    ) -> Option<(Vec<u8>, (u32, u32))> {
        let side = |twips: i32| {
            ((i64::from(twips.max(1)) * EXPORT_DPI / 1_440).clamp(1, i64::from(MAX_SIDE))) as u32
        };
        let handle = match self.shown(
            document,
            picture,
            folder,
            (side(size.width), side(size.height)),
            1.0,
        ) {
            Shown::Picture(handle) | Shown::Screen(handle) => handle,
            Shown::Box(_) => return None,
        };
        let Handle::Rgba {
            width,
            height,
            pixels,
            ..
        } = &handle
        else {
            return None;
        };
        let mut png = Vec::new();
        image::ImageEncoder::write_image(
            image::codecs::png::PngEncoder::new(&mut png),
            pixels,
            *width,
            *height,
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;
        Some((png, (*width, *height)))
    }

    /// A metafile's bytes with a placeable header giving `natural`: as they
    /// are when they have one, else with one made from its window.
    fn placeable(
        &self,
        document: &Document,
        picture: &Picture,
        folder: Option<&Path>,
        bytes: &[u8],
        natural: Size,
    ) -> Option<Vec<u8>> {
        if wmf::placeable_size(bytes).is_some() {
            return Some(bytes.to_vec());
        }
        let key = self.entry(document, picture, folder)?;
        let (width, height) = match &self.cache.borrow().get(&key)?.content {
            Content::Metafile(drawing) => drawing.window,
            _ => return None,
        };
        let (width, height) = (width.max(1), height.max(1));
        // Units an inch that make the window `natural` twips wide.
        let inch =
            (i64::from(width) * 1_440 / i64::from(natural.width.max(1))).clamp(1, 0xFFFF) as u16;
        let mut header = Vec::with_capacity(22 + bytes.len());
        header.extend_from_slice(&0x9AC6_CDD7u32.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        for value in [
            0i16,
            0,
            width.min(i32::from(i16::MAX)) as i16,
            height.min(i32::from(i16::MAX)) as i16,
        ] {
            header.extend_from_slice(&value.to_le_bytes());
        }
        header.extend_from_slice(&inch.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
        let checksum = header.chunks(2).fold(0u16, |sum, word| {
            sum ^ u16::from_le_bytes([word[0], word[1]])
        });
        header.extend_from_slice(&checksum.to_le_bytes());
        header.extend_from_slice(bytes);
        Some(header)
    }
}

/// The resolution pictures are drawn at for export.
const EXPORT_DPI: i64 = 300;

/// The Image Processing settings of a picture (DO.5).
fn settings_of(picture: &Picture) -> adjust::Settings {
    let adjust = picture.adjust;
    adjust::Settings {
        brightness: adjust.brightness,
        contrast: adjust.contrast,
        edge: adjust.edge,
        smoothing: adjust.smoothing,
        invert: adjust.invert,
    }
}

/// `handle`'s pixels with the settings applied.
pub(crate) fn adjusted_handle(handle: &Handle, settings: adjust::Settings) -> Handle {
    let Handle::Rgba {
        width,
        height,
        pixels,
        ..
    } = handle
    else {
        return handle.clone();
    };
    let source = Raster {
        width: *width,
        height: *height,
        pixels: pixels.to_vec(),
    };
    let done = adjust::apply(&source, settings);
    Handle::from_rgba(done.width, done.height, done.pixels)
}

/// A raster image's pixels as a tiny-skia pixmap.
fn handle_pixmap(handle: &Handle) -> Option<tiny_skia::Pixmap> {
    let Handle::Rgba {
        width,
        height,
        pixels,
        ..
    } = handle
    else {
        return None;
    };
    metafile_render::pixmap_of(&Raster {
        width: *width,
        height: *height,
        pixels: pixels.to_vec(),
    })
}

fn pixmap_handle(pixmap: &tiny_skia::Pixmap) -> Handle {
    Handle::from_rgba(
        pixmap.width(),
        pixmap.height(),
        metafile_render::rgba_of(pixmap),
    )
}

fn svg_pixmap(tree: &resvg::usvg::Tree, (width, height): (u32, u32)) -> Option<tiny_skia::Pixmap> {
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let size = tree.size();
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / size.width().max(1.0),
        height as f32 / size.height().max(1.0),
    );
    resvg::render(tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

/// Synthetic OLE 1.0 object streams for tests (no corpus data): the
/// layout the 34 corpus objects share, `OBJECTS_NOTES.md` section 4.2.
#[cfg(test)]
pub(crate) mod ole_fixture {
    /// A plain metafile that fills the window (0..400 by 0..200) with red.
    pub(crate) fn metafile() -> Vec<u8> {
        let records: [(u16, Vec<u16>); 7] = [
            (0x020B, vec![0, 0]),
            (0x020C, vec![200, 400]),
            (0x02FC, vec![0, 0x00ff, 0, 0]),
            (0x02FA, vec![5, 0, 0, 0, 0]),
            (0x012D, vec![0]),
            (0x012D, vec![1]),
            (0x041B, vec![200, 400, 0, 0]),
        ];
        let mut body = Vec::new();
        for (function, words) in &records {
            body.extend_from_slice(&((3 + words.len()) as u32).to_le_bytes());
            body.extend_from_slice(&function.to_le_bytes());
            for word in words {
                body.extend_from_slice(&word.to_le_bytes());
            }
        }
        body.extend_from_slice(&3u32.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        let mut out = Vec::new();
        for word in [1u16, 9, 0x300] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&(((18 + body.len()) / 2) as u32).to_le_bytes());
        out.extend_from_slice(&8u16.to_le_bytes());
        out.extend_from_slice(&64u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    /// A document with one OLE frame for each of `streams`, side by side
    /// on page one, and the frames.
    pub(crate) fn document(
        streams: &[Vec<u8>],
    ) -> (amipro_document::Document, Vec<amipro_document::FrameId>) {
        use amipro_document::{FrameStyle, Placement, Rect};
        use amipro_editing::{Command, NewFrameContent, apply};
        let mut document = amipro_document::Document::from_plain_text("Text");
        let mut frames = Vec::new();
        for (index, stream) in streams.iter().enumerate() {
            let left = 1_440 + index as i32 * 3_600;
            let applied = apply(
                &document,
                &Command::CreateFrame {
                    placement: Placement::Fixed {
                        page: 1,
                        rect: Rect::new(left, 2_880, left + 2_880, 2_880 + 1_440),
                    },
                    style: FrameStyle::default(),
                    content: NewFrameContent::EmbeddedPicture {
                        bytes: stream.as_slice().into(),
                        extension: ".ole".into(),
                    },
                    anchor: None,
                    layout: None,
                },
            )
            .unwrap();
            document = applied.document;
            frames.push(applied.report.frames.into_iter().next().unwrap());
        }
        (document, frames)
    }

    fn string(out: &mut Vec<u8>, text: &str) {
        if text.is_empty() {
            out.extend_from_slice(&0u32.to_le_bytes());
        } else {
            out.extend_from_slice(&(text.len() as u32 + 1).to_le_bytes());
            out.extend_from_slice(text.as_bytes());
            out.push(0);
        }
    }

    /// An embedded object of `class` with `native` data, a `METAFILEPICT`
    /// presentation of 5,080 by -2,540 hundredths of a millimetre (2 × 1
    /// in; the height is negative in the files), and `trailing` bytes.
    pub(crate) fn stream(class: &str, native: &[u8], trailing: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0x501u32.to_le_bytes());
        out.extend_from_slice(&2u32.to_le_bytes());
        string(&mut out, class);
        string(&mut out, "");
        string(&mut out, "");
        out.extend_from_slice(&(native.len() as u32).to_le_bytes());
        out.extend_from_slice(native);
        out.extend_from_slice(&0x501u32.to_le_bytes());
        out.extend_from_slice(&5u32.to_le_bytes());
        string(&mut out, "METAFILEPICT");
        out.extend_from_slice(&5_080u32.to_le_bytes());
        out.extend_from_slice(&(-2_540i32).to_le_bytes());
        let mut data = Vec::new();
        for word in [8u16, 5_080, 2_540, 0] {
            data.extend_from_slice(&word.to_le_bytes());
        }
        data.extend_from_slice(&metafile());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        out.extend_from_slice(trailing);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An equation draws as ink on the page (text and rules), at its own
    /// natural size.
    #[test]
    fn an_equation_is_drawn_from_its_text() {
        let stored =
            amipro_pictures::equation::Stored::new("$\\frac{a}{b}+\\sqrt{x}+\\sum_{i=1}^n i$");
        let (drawing, natural) = equation_drawing(&stored.bytes()).expect("lays out");
        assert!(natural.width > 400 && natural.height >= 257);
        let pixmap = metafile_render::render(&drawing, 240, 60, 1.0).expect("renders");
        let inked = (0..60)
            .flat_map(|y| (0..240).map(move |x| (x, y)))
            .filter(|&(x, y)| pixmap.pixel(x, y).is_some_and(|p| p.alpha() > 128))
            .count();
        assert!(inked > 200, "{inked} inked pixels");
        assert!(equation_drawing(b"not two lines").is_none());
    }

    #[test]
    fn resolutions_come_from_png_and_jpeg_headers() {
        // A PNG's pHYs of 3,780 pixels a metre (96 dpi).
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[0; 13 + 4]);
        png.extend_from_slice(&9u32.to_be_bytes());
        png.extend_from_slice(b"pHYs");
        png.extend_from_slice(&3_780u32.to_be_bytes());
        png.extend_from_slice(&3_780u32.to_be_bytes());
        png.push(1);
        assert_eq!(resolution(&png), Some((3_780, 3_780)));
        // A JFIF of 300 dpi.
        let jpeg = [
            0xff, 0xd8, 0xff, 0xe0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 1, 1, 44, 1, 44,
        ];
        assert_eq!(resolution(&jpeg), Some((11_811, 11_811)));
        assert_eq!(resolution(b"GIF89a"), None);
    }

    /// An enhanced metafile 10 by 5 mm on a device of 100 pixels a
    /// millimetre: a red rectangle over the whole frame.
    fn red_emf() -> Vec<u8> {
        let records: [&[u32]; 5] = [
            &[39, 24, 1, 0, 0x0000_00ff, 0],
            &[38, 28, 2, 5, 0, 0, 0],
            &[37, 12, 1],
            &[37, 12, 2],
            &[43, 24, 0, 0, 1_000, 500],
        ];
        let mut body: Vec<u32> = records.concat();
        body.extend([14, 20, 0, 16, 20]);
        let mut words: Vec<u32> = vec![
            1,
            88,
            0,
            0,
            999,
            499,
            0,
            0,
            1_000,
            500,
            0x464D_4520,
            0x10000,
            0,
            7,
            3,
            0,
            0,
            0,
            100_000,
            50_000,
            1_000,
            500,
        ];
        words[12] = (4 * (words.len() + body.len())) as u32;
        words.extend(body);
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    #[test]
    fn enhanced_metafiles_are_drawn_and_converted_for_ami_pro() {
        let emf = red_emf();
        let (content, natural) = decode(PictureFormat::Emf, &emf, None);
        assert!(matches!(content, Content::Metafile(_)));
        let natural = natural.unwrap();
        assert_eq!(
            natural,
            Size {
                width: 567,
                height: 283
            }
        );
        // Save As SAM writes it as a bitmap at 300 dpi of its size.
        let picture = Picture::new(PictureSource::Linked("RED.EMF".into()), ".emf", natural);
        let placement = amipro_document::Placement::Inline {
            y_offset: 0,
            size: Size {
                width: 2_880,
                height: 1_440,
            },
        };
        let style = amipro_document::FrameStyle::default();
        let bitmap = sam_bitmap(&picture, &placement, &style, &emf).unwrap();
        let raster = dib::decode_bmp(&bitmap)
            .or_else(|| dib::decode_packed(&bitmap, dib::ColourTable::Rgb))
            .unwrap();
        assert!(
            raster.width > 50 && raster.height > 25,
            "{} × {}",
            raster.width,
            raster.height
        );
        assert_eq!(
            raster.pixel(raster.width / 2, raster.height / 2),
            [255, 0, 0, 255]
        );
    }

    #[test]
    fn an_ole_object_is_drawn_from_its_presentation_at_its_size() {
        let stream = ole_fixture::stream("MSWordArt.2", &[1; 40], &[]);
        // 5,080 by 2,540 hundredths of a millimetre: 2 by 1 in.
        assert_eq!(
            natural_of(PictureFormat::Ole, &stream),
            Some(Size {
                width: 2_880,
                height: 1_440
            })
        );
        let (content, natural) = decode(PictureFormat::Ole, &stream, None);
        let Content::Metafile(drawing) = content else {
            panic!("not drawn");
        };
        assert_eq!(natural, natural_of(PictureFormat::Ole, &stream));
        let pixmap = metafile_render::render(&drawing, 40, 20, 1.0).unwrap();
        let middle = pixmap.pixel(20, 10).unwrap().demultiply();
        assert_eq!((middle.red(), middle.green(), middle.blue()), (255, 0, 0));
    }

    #[test]
    fn an_ole_object_without_a_readable_presentation_is_not_drawn() {
        let stream = ole_fixture::stream("WordArt", &[], &[]);
        // Every cut that keeps the class but loses the presentation, and
        // streams that are not OLE at all, draw nothing and never panic.
        for length in 0..stream.len() {
            let cut = &stream[..length];
            let (content, _) = decode(PictureFormat::Ole, cut, None);
            let drawn = matches!(content, Content::Metafile(_));
            assert_eq!(drawn, length == stream.len(), "length {length}");
        }
        for junk in [&b"not an object"[..], &[0u8; 64][..], &[0xff; 64][..]] {
            assert!(natural_of(PictureFormat::Ole, junk).is_none());
        }
    }

    #[test]
    fn drawings_are_drawn_from_their_objects() {
        use amipro_pictures::sdw::{Colour, Drawing, Fill, Object, Pen, Point, Rect, Shape, kind};
        let corners = [
            Point::new(100, 100),
            Point::new(1_100, 100),
            Point::new(1_100, 600),
            Point::new(100, 600),
        ];
        let drawing = Drawing {
            objects: vec![Object {
                kind: kind::RECTANGLE,
                flags: 0,
                rect: Rect::new(100, 100, 1_100, 600),
                shape: Shape::Rectangle {
                    original: Rect::new(100, 100, 1_100, 600),
                    corners,
                    pen: Pen::default(),
                    fill: Fill::solid(Colour::rgb(255, 0, 0)),
                },
            }],
            ..Drawing::new()
        };
        let bytes = sdw::write(&drawing);
        for format in [PictureFormat::Drawing, PictureFormat::Chart] {
            let (content, natural) = decode(format, &bytes, None);
            let Content::Metafile(painted) = content else {
                panic!("not drawn");
            };
            assert_eq!(
                natural,
                Some(Size {
                    width: 1_110,
                    height: 610
                })
            );
            let pixmap = metafile_render::render(&painted, 111, 61, 1.0).unwrap();
            let middle = pixmap.pixel(60, 35).unwrap().demultiply();
            assert_eq!((middle.red(), middle.green(), middle.blue()), (255, 0, 0));
            // Nothing above or left of the drawing's origin's own objects.
            assert_eq!(pixmap.pixel(3, 3).unwrap().alpha(), 0);
        }
        assert_eq!(
            natural_of(PictureFormat::Drawing, &bytes),
            Some(Size {
                width: 1_110,
                height: 610
            })
        );
    }

    #[test]
    fn linked_pictures_are_found_by_name_in_the_documents_folder() {
        let folder = std::env::temp_dir().join(format!("amipro-linked-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("Pic.Bmp"), b"BM").unwrap();
        let found = linked_file("L:\\IN\\PIC.BMP", Some(&folder)).unwrap();
        assert_eq!(found.0, folder.join("Pic.Bmp"));
        assert!(linked_file("L:\\IN\\NONE.BMP", Some(&folder)).is_none());
        let absolute = folder.join("Pic.Bmp");
        assert!(linked_file(absolute.to_str().unwrap(), None).is_some());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    /// `AMIPRO_CORPUS=DIR` (and `AMIPRO_STYLES`, `AMIPRO_COMPARE_OUT=DIR`):
    /// each drawing and chart that keeps Ami Pro's screen image, drawn from
    /// its objects at that image's size and compared with it
    /// (`docs/DRAWING_RUNBOOK.md`, DR.3). The score is the share of inked
    /// pixels (not white) in either image that have an inked pixel of a
    /// like colour within two pixels in the other. Aggregate numbers only;
    /// with `AMIPRO_COMPARE_OUT` the worst pairs are written side by side
    /// (outside Git).
    #[test]
    #[ignore = "needs AMIPRO_CORPUS"]
    fn drawings_against_ami_pro() {
        use amipro_document::{FrameContent, PictureFormat};
        let Some(directory) = std::env::var_os("AMIPRO_CORPUS") else {
            eprintln!("AMIPRO_CORPUS is not set; skipping");
            return;
        };
        let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|styles| std::env::split_paths(&styles).collect())
            .unwrap_or_default();
        let out = std::env::var_os("AMIPRO_COMPARE_OUT").map(PathBuf::from);
        let mut files = Vec::new();
        let mut stack = vec![PathBuf::from(directory)];
        while let Some(folder) = stack.pop() {
            for entry in std::fs::read_dir(folder).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
                {
                    files.push(path);
                }
            }
        }
        files.sort();
        let mut scores: Vec<(f64, tiny_skia::Pixmap, tiny_skia::Pixmap)> = Vec::new();
        let mut skipped = 0;
        for path in files {
            let Ok(import) = amipro_formats::import_sam_path_with_style_directories(&path, &styles)
            else {
                continue;
            };
            let document = import.document;
            for frame in document.frames() {
                let FrameContent::Picture(picture) = &frame.content else {
                    continue;
                };
                if !matches!(
                    picture.format(),
                    PictureFormat::Drawing | PictureFormat::Chart
                ) {
                    continue;
                }
                let (Some(rendering), PictureSource::Embedded(source), Some(size)) = (
                    picture
                        .rendering
                        .and_then(|object| document.objects().get(&object))
                        .and_then(|object| screen::decode(&object.bytes)),
                    &picture.source,
                    frame.placement.size(),
                ) else {
                    skipped += 1;
                    continue;
                };
                let Some(drawing) = document
                    .objects()
                    .get(source)
                    .and_then(|object| sdw::read(&object.bytes))
                else {
                    skipped += 1;
                    continue;
                };
                let insets = frame.style.text_insets(size.width, size.height);
                let area = Size {
                    width: size.width - insets.left - insets.right,
                    height: size.height - insets.top - insets.bottom,
                };
                if area.width <= 0 || area.height <= 0 {
                    skipped += 1;
                    continue;
                }
                let (width, height) = (rendering.width, rendering.height);
                let (sx, sy) = (
                    width as f32 / area.width as f32,
                    height as f32 / area.height as f32,
                );
                let natural = twips(drawing.natural_size());
                let shown = picture.scaling.drawn_size(natural, area);
                let mut ours = tiny_skia::Pixmap::new(width, height).unwrap();
                ours.fill(tiny_skia::Color::WHITE);
                if let Some(painted) = metafile_render::render(
                    &sdw::paint(&drawing),
                    ((shown.width as f32 * sx).round() as u32).max(1),
                    ((shown.height as f32 * sy).round() as u32).max(1),
                    1.0,
                ) {
                    ours.draw_pixmap(
                        (picture.placed_pan().x as f32 * sx).round() as i32,
                        (picture.placed_pan().y as f32 * sy).round() as i32,
                        painted.as_ref(),
                        &tiny_skia::PixmapPaint::default(),
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
                let theirs = metafile_render::pixmap_of(&rendering).unwrap();
                scores.push((ink_agreement(&ours, &theirs), ours, theirs));
            }
        }
        let mut values: Vec<f64> = scores.iter().map(|(score, ..)| *score).collect();
        values.sort_by(f64::total_cmp);
        let share = |at: f64| values.iter().filter(|&&value| value >= at).count();
        println!(
            "drawings compared: {}; without a screen image or a size: {skipped}",
            values.len()
        );
        if let Some(median) = values.get(values.len() / 2) {
            println!(
                "median agreement {:.1}%; at least 95%: {}, 90%: {}, 75%: {}, 50%: {}",
                median * 100.0,
                share(0.95),
                share(0.90),
                share(0.75),
                share(0.50)
            );
        }
        if let Some(out) = out {
            std::fs::create_dir_all(&out).unwrap();
            scores.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (index, (score, ours, theirs)) in scores.iter().enumerate() {
                let mut pair = tiny_skia::Pixmap::new(ours.width() * 2 + 8, ours.height()).unwrap();
                pair.fill(tiny_skia::Color::from_rgba8(128, 128, 128, 255));
                for (x, image) in [(0, theirs), (ours.width() as i32 + 8, ours)] {
                    pair.draw_pixmap(
                        x,
                        0,
                        image.as_ref(),
                        &tiny_skia::PixmapPaint::default(),
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
                pair.save_png(out.join(format!("{index:03}-{:.0}.png", score * 100.0)))
                    .unwrap();
            }
        }
    }

    /// `AMIPRO_CORPUS=DIR` (and `AMIPRO_STYLES`): every OLE object in the
    /// SAM files read and drawn from its presentation (DO.3: 34 of 34 on
    /// the user's corpus); those that keep Ami Pro's screen image are drawn
    /// at that image's size and compared with it as `drawings_against_ami_pro`
    /// does. Aggregate numbers only (`AMIPRO_COMPARE_OUT=DIR` writes the
    /// pairs, outside Git).
    #[test]
    #[ignore = "needs AMIPRO_CORPUS"]
    fn ole_objects_against_ami_pro() {
        use amipro_document::FrameContent;
        let Some(directory) = std::env::var_os("AMIPRO_CORPUS") else {
            eprintln!("AMIPRO_CORPUS is not set; skipping");
            return;
        };
        let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|styles| std::env::split_paths(&styles).collect())
            .unwrap_or_default();
        let out = std::env::var_os("AMIPRO_COMPARE_OUT").map(PathBuf::from);
        let mut files = Vec::new();
        let mut stack = vec![PathBuf::from(directory)];
        while let Some(folder) = stack.pop() {
            for entry in std::fs::read_dir(folder).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
                {
                    files.push(path);
                }
            }
        }
        files.sort();
        let (mut objects, mut drawn, mut kept_screen) = (0, 0, 0);
        let mut classes: std::collections::BTreeMap<String, usize> = Default::default();
        let mut scores: Vec<(f64, tiny_skia::Pixmap, tiny_skia::Pixmap)> = Vec::new();
        for path in files {
            let Ok(import) = amipro_formats::import_sam_path_with_style_directories(&path, &styles)
            else {
                continue;
            };
            let document = import.document;
            for frame in document.frames() {
                let FrameContent::Picture(picture) = &frame.content else {
                    continue;
                };
                let PictureSource::Embedded(source) = &picture.source else {
                    continue;
                };
                if picture.format() != PictureFormat::Ole {
                    continue;
                }
                objects += 1;
                let Some(bytes) = document.objects().get(source).map(|o| Arc::clone(&o.bytes))
                else {
                    continue;
                };
                if let Some(object) = ole::read(&bytes) {
                    *classes.entry(object.label()).or_default() += 1;
                }
                let Some((drawing, natural)) = ole_drawing(&bytes) else {
                    continue;
                };
                drawn += 1;
                let (Some(rendering), Some(size)) = (
                    picture
                        .rendering
                        .and_then(|object| document.objects().get(&object))
                        .and_then(|object| screen::decode(&object.bytes)),
                    frame.placement.size(),
                ) else {
                    continue;
                };
                kept_screen += 1;
                let insets = frame.style.text_insets(size.width, size.height);
                let area = Size {
                    width: size.width - insets.left - insets.right,
                    height: size.height - insets.top - insets.bottom,
                };
                if area.width <= 0 || area.height <= 0 {
                    continue;
                }
                let (width, height) = (rendering.width, rendering.height);
                let (sx, sy) = (
                    width as f32 / area.width as f32,
                    height as f32 / area.height as f32,
                );
                let shown = picture.scaling.drawn_size(natural, area);
                let mut ours = tiny_skia::Pixmap::new(width, height).unwrap();
                ours.fill(tiny_skia::Color::WHITE);
                if let Some(painted) = metafile_render::render(
                    &drawing,
                    ((shown.width as f32 * sx).round() as u32).max(1),
                    ((shown.height as f32 * sy).round() as u32).max(1),
                    1.0,
                ) {
                    ours.draw_pixmap(
                        (picture.placed_pan().x as f32 * sx).round() as i32,
                        (picture.placed_pan().y as f32 * sy).round() as i32,
                        painted.as_ref(),
                        &tiny_skia::PixmapPaint::default(),
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
                let theirs = metafile_render::pixmap_of(&rendering).unwrap();
                scores.push((ink_agreement(&ours, &theirs), ours, theirs));
            }
        }
        println!("OLE objects: {objects}; drawn from the presentation: {drawn}");
        println!("by class: {classes:?}");
        let mut values: Vec<f64> = scores.iter().map(|(score, ..)| *score).collect();
        values.sort_by(f64::total_cmp);
        let share = |at: f64| values.iter().filter(|&&value| value >= at).count();
        println!(
            "with Ami Pro's screen image: {kept_screen}; compared {}",
            values.len()
        );
        if let Some(median) = values.get(values.len() / 2) {
            println!(
                "median agreement {:.1}%; at least 95%: {}, 90%: {}, 75%: {}, 50%: {}",
                median * 100.0,
                share(0.95),
                share(0.90),
                share(0.75),
                share(0.50)
            );
        }
        if let Some(out) = out {
            std::fs::create_dir_all(&out).unwrap();
            for (index, (score, ours, theirs)) in scores.iter().enumerate() {
                let mut pair = tiny_skia::Pixmap::new(ours.width() * 2 + 8, ours.height()).unwrap();
                pair.fill(tiny_skia::Color::from_rgba8(128, 128, 128, 255));
                for (x, image) in [(0, theirs), (ours.width() as i32 + 8, ours)] {
                    pair.draw_pixmap(
                        x,
                        0,
                        image.as_ref(),
                        &tiny_skia::PixmapPaint::default(),
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
                pair.save_png(out.join(format!("ole-{index:03}-{:.0}.png", score * 100.0)))
                    .unwrap();
            }
        }
        if objects > 0 {
            assert_eq!(
                drawn, objects,
                "every OLE object draws from its presentation"
            );
        }
    }

    /// The share of inked pixels in either image matched within two pixels
    /// by an inked pixel of a like colour in the other.
    fn ink_agreement(a: &tiny_skia::Pixmap, b: &tiny_skia::Pixmap) -> f64 {
        let (width, height) = (a.width().min(b.width()), a.height().min(b.height()));
        let colour = |image: &tiny_skia::Pixmap, x: u32, y: u32| {
            let pixel = image.pixel(x, y).unwrap().demultiply();
            [pixel.red(), pixel.green(), pixel.blue()]
        };
        let inked = |rgb: [u8; 3]| rgb.iter().any(|&channel| channel < 200);
        let near = |p: [u8; 3], q: [u8; 3]| {
            p.iter()
                .zip(q)
                .all(|(&a, b)| (i32::from(a) - i32::from(b)).abs() <= 80)
        };
        let matched = |from: &tiny_skia::Pixmap, to: &tiny_skia::Pixmap| {
            let (mut inked_pixels, mut found) = (0u64, 0u64);
            for y in 0..height {
                for x in 0..width {
                    let rgb = colour(from, x, y);
                    if !inked(rgb) {
                        continue;
                    }
                    inked_pixels += 1;
                    let hit = (y.saturating_sub(2)..(y + 3).min(height)).any(|v| {
                        (x.saturating_sub(2)..(x + 3).min(width)).any(|u| {
                            let other = colour(to, u, v);
                            inked(other) && near(rgb, other)
                        })
                    });
                    found += u64::from(hit);
                }
            }
            (inked_pixels, found)
        };
        let (a_inked, a_found) = matched(a, b);
        let (b_inked, b_found) = matched(b, a);
        if a_inked + b_inked == 0 {
            return 1.0;
        }
        (a_found + b_found) as f64 / (a_inked + b_inked) as f64
    }
}
