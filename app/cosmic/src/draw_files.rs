//! Drawings in and out (`UI_SPEC.md` §33, DW6): Draw › Import Drawing…
//! (an Ami Pro drawing's objects, an SVG's shapes as objects, a picture as
//! a bitmap object), Save Drawing As… (`.sdw`, SVG, PNG), and the
//! clipboard (the app's objects with SVG and PNG beside them; text pasted
//! as a text object per line, Ami Pro's rule).

use std::borrow::Cow;

use amipro_pictures::sdw::{self, Colour, Drawing, Fill, Object, Pen, Point, Rect, Shape};
use amipro_pictures::{Raster, dib, svg};

/// The clipboard type of the app's own drawing objects (a `.sdw` stream).
pub const DRAWING_TYPE: &str = "application/x-amipro-drawing";
const SVG_TYPE: &str = "image/svg+xml";
const PNG_TYPE: &str = "image/png";
/// A screen pixel (SVG's unit, 96 dpi) in twips.
const PX: f64 = 15.0;

/// The formats Save Drawing As… writes, by extension.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    Sdw,
    Svg,
    Png,
}

impl Format {
    pub fn of(path: &std::path::Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "sdw" => Some(Self::Sdw),
            "svg" => Some(Self::Svg),
            "png" => Some(Self::Png),
            _ => None,
        }
    }
}

/// `drawing` as SVG at its natural size.
pub fn svg_bytes(drawing: &Drawing) -> Vec<u8> {
    let size = drawing.natural_size();
    svg::svg(&sdw::paint(drawing), size.width, size.height).into_bytes()
}

/// `drawing` as a PNG at `dpi`.
pub fn png_bytes(drawing: &Drawing, dpi: f64) -> Option<Vec<u8>> {
    let size = drawing.natural_size();
    let scale = dpi / 1440.0;
    let (width, height) = (
        (f64::from(size.width) * scale).ceil().max(1.0) as u32,
        (f64::from(size.height) * scale).ceil().max(1.0) as u32,
    );
    let pixmap = crate::metafile_render::render(
        &sdw::paint(drawing),
        width.min(8_000),
        height.min(8_000),
        (dpi / 96.0) as f32,
    )?;
    let rgba = crate::metafile_render::rgba_of(&pixmap);
    let mut out = Vec::new();
    image::ImageEncoder::write_image(
        image::codecs::png::PngEncoder::new(&mut out),
        &rgba,
        pixmap.width(),
        pixmap.height(),
        image::ExtendedColorType::Rgba8,
    )
    .ok()?;
    Some(out)
}

/// The bytes Save Drawing As… writes.
pub fn file_bytes(drawing: &Drawing, format: Format) -> Option<Vec<u8>> {
    match format {
        Format::Sdw => Some(sdw::write(drawing)),
        Format::Svg => Some(svg_bytes(drawing)),
        Format::Png => png_bytes(drawing, 300.0),
    }
}

/// `objects` moved so that their joint rectangle starts at the drawing's
/// origin, as Ami Pro places what it imports or pastes.
fn at_origin(mut objects: Vec<Object>) -> Vec<Object> {
    let Some(bounds) = objects
        .iter()
        .map(|object| object.rect)
        .reduce(|a, b| a.union(&b))
    else {
        return objects;
    };
    let map = sdw::Affine::translate(-f64::from(bounds.left), -f64::from(bounds.top));
    for object in &mut objects {
        object.transform(&map);
    }
    objects
}

/// The objects a file (or clipboard data) of `kind` holds: an Ami Pro
/// drawing's, an SVG's shapes, a picture as a bitmap object; `None` when
/// it does not read.
pub fn objects_of(bytes: &[u8], kind: &str) -> Option<Vec<Object>> {
    match kind {
        "sdw" | DRAWING_TYPE => sdw::read(bytes).map(|drawing| drawing.objects),
        "svg" | SVG_TYPE => svg_objects(bytes).map(at_origin),
        _ => image_object(bytes).map(|object| vec![object]),
    }
}

/// A picture as a bitmap object at its natural size (Ami Pro's Import
/// Drawing of a Windows bitmap).
pub fn image_object(bytes: &[u8]) -> Option<Object> {
    image_extras::register();
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    let raster = Raster {
        width,
        height,
        pixels: image.into_raw(),
    };
    bitmap_object(&raster, (0.0, 0.0), None)
}

/// `raster` as a bitmap object at `at` (twips), `size` twips or its
/// natural size at 96 dpi.
fn bitmap_object(raster: &Raster, at: (f64, f64), size: Option<(f64, f64)>) -> Option<Object> {
    let natural = dib::raster_natural_size(raster.width, raster.height, None);
    let (width, height) = size.unwrap_or((f64::from(natural.width), f64::from(natural.height)));
    // A packed DIB: the 24-bit file without its 14-byte file header.
    let file = dib::encode_bmp24(raster, (3_780, 3_780));
    let dib = file.get(14..)?.to_vec();
    let clamp = |value: f64| {
        value
            .round()
            .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
    };
    let rect = Rect::new(
        clamp(at.0),
        clamp(at.1),
        clamp(at.0 + width),
        clamp(at.1 + height),
    );
    Some(Object {
        kind: sdw::kind::BITMAP,
        flags: 0,
        rect,
        shape: Shape::Bitmap { kept: [0; 4], dib },
    })
}

/// An SVG's shapes as drawing objects: each subpath a polygon (filled or
/// closed) or a polyline, curves followed in short straight pieces, with
/// its stroke and a solid fill (a gradient's first colour); text as its
/// shapes; images as bitmap objects. SVG pixels are 15 twips.
pub fn svg_objects(bytes: &[u8]) -> Option<Vec<Object>> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(bytes, &options).ok()?;
    let mut objects = Vec::new();
    group_objects(tree.root(), &mut objects);
    (!objects.is_empty()).then_some(objects)
}

fn colour_of(paint: &resvg::usvg::Paint) -> Option<Colour> {
    use resvg::usvg::Paint;
    let colour = match paint {
        Paint::Color(colour) => *colour,
        Paint::LinearGradient(gradient) => gradient.stops().first()?.color(),
        Paint::RadialGradient(gradient) => gradient.stops().first()?.color(),
        Paint::Pattern(_) => return None,
    };
    Some(Colour::rgb(colour.red, colour.green, colour.blue))
}

fn group_objects(group: &resvg::usvg::Group, objects: &mut Vec<Object>) {
    use resvg::usvg::Node;
    for node in group.children() {
        match node {
            Node::Group(group) => group_objects(group, objects),
            Node::Path(path) if path.is_visible() => path_objects(path, objects),
            Node::Path(_) => {}
            Node::Text(text) => group_objects(text.flattened(), objects),
            Node::Image(image) => {
                if let resvg::usvg::ImageKind::PNG(data)
                | resvg::usvg::ImageKind::JPEG(data)
                | resvg::usvg::ImageKind::GIF(data)
                | resvg::usvg::ImageKind::WEBP(data) = image.kind()
                    && let Ok(decoded) = image::load_from_memory(data)
                {
                    let rgba = decoded.into_rgba8();
                    let (width, height) = rgba.dimensions();
                    let raster = Raster {
                        width,
                        height,
                        pixels: rgba.into_raw(),
                    };
                    let bounds = image.abs_bounding_box();
                    if let Some(object) = bitmap_object(
                        &raster,
                        (f64::from(bounds.x()) * PX, f64::from(bounds.y()) * PX),
                        Some((
                            f64::from(bounds.width()) * PX,
                            f64::from(bounds.height()) * PX,
                        )),
                    ) {
                        objects.push(object);
                    }
                }
            }
        }
    }
}

fn path_objects(path: &resvg::usvg::Path, objects: &mut Vec<Object>) {
    use resvg::tiny_skia::PathSegment;
    let transform = path.abs_transform();
    let point = |x: f32, y: f32| {
        let mut p = resvg::tiny_skia::Point::from_xy(x, y);
        transform.map_point(&mut p);
        (f64::from(p.x) * PX, f64::from(p.y) * PX)
    };
    let fill = path
        .fill()
        .and_then(|fill| colour_of(fill.paint()))
        .map(Fill::solid);
    let pen = path.stroke().and_then(|stroke| {
        let colour = colour_of(stroke.paint())?;
        let scale = (transform.sx.abs() + transform.sy.abs()) / 2.0;
        let width = (f64::from(stroke.width().get() * scale) * PX)
            .round()
            .clamp(1.0, 255.0) as u8;
        Some(Pen {
            width,
            style: if stroke.dasharray().is_some() { 1 } else { 0 },
            colour,
            start: 0,
            end: 0,
        })
    });
    if fill.is_none() && pen.is_none() {
        return;
    }
    let pen = pen.unwrap_or(Pen {
        width: 0,
        ..Pen::default()
    });
    let mut subpaths: Vec<(Vec<(f64, f64)>, bool)> = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    let mut last = (0.0f32, 0.0f32);
    let flush = |current: &mut Vec<(f64, f64)>, closed: bool, subpaths: &mut Vec<_>| {
        if current.len() >= 2 {
            subpaths.push((std::mem::take(current), closed));
        } else {
            current.clear();
        }
    };
    for segment in path.data().segments() {
        match segment {
            PathSegment::MoveTo(p) => {
                flush(&mut current, false, &mut subpaths);
                current.push(point(p.x, p.y));
                last = (p.x, p.y);
            }
            PathSegment::LineTo(p) => {
                current.push(point(p.x, p.y));
                last = (p.x, p.y);
            }
            PathSegment::QuadTo(c, p) => {
                for step in 1..=8 {
                    let t = step as f32 / 8.0;
                    let u = 1.0 - t;
                    current.push(point(
                        u * u * last.0 + 2.0 * u * t * c.x + t * t * p.x,
                        u * u * last.1 + 2.0 * u * t * c.y + t * t * p.y,
                    ));
                }
                last = (p.x, p.y);
            }
            PathSegment::CubicTo(a, b, p) => {
                for step in 1..=12 {
                    let t = step as f32 / 12.0;
                    let u = 1.0 - t;
                    current.push(point(
                        u * u * u * last.0
                            + 3.0 * u * u * t * a.x
                            + 3.0 * u * t * t * b.x
                            + t * t * t * p.x,
                        u * u * u * last.1
                            + 3.0 * u * u * t * a.y
                            + 3.0 * u * t * t * b.y
                            + t * t * t * p.y,
                    ));
                }
                last = (p.x, p.y);
            }
            PathSegment::Close => flush(&mut current, true, &mut subpaths),
        }
    }
    flush(&mut current, false, &mut subpaths);
    let clamp = |value: f64| {
        value
            .round()
            .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
    };
    for (points, closed) in subpaths {
        let points: Vec<Point> = points
            .into_iter()
            .map(|(x, y)| Point::new(clamp(x), clamp(y)))
            .collect();
        objects.push(match fill {
            Some(fill) => Object::polygon(points, pen, fill),
            None if closed => Object::polygon(points, pen, Fill::default()),
            None => Object::polyline(points, pen),
        });
    }
}

/// Text pasted into a drawing: a text object per line in `look`, one below
/// another from the origin (Ami Pro's Paste of text).
pub fn text_objects(
    text: &str,
    look: &crate::draw_mode::TextLook,
    measure: &dyn Fn(&sdw::Text) -> (f64, f64),
) -> Vec<Object> {
    let mut objects = Vec::new();
    let mut y = 0.0;
    for line in text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
    {
        let mut text = sdw::Text {
            origin: Point::new(0, y as i16),
            size: Point::new(1, 1),
            face: look.face.clone(),
            pitch_and_family: 0,
            height: look.height,
            colour: look.colour,
            attributes: look.attributes,
            attributes2: 0,
            angle: 0,
            text: line.chars().take(255).collect(),
            kept: sdw::TextKept::default(),
        };
        let (width, height) = measure(&text);
        text.size = Point::new(
            width.round().max(1.0) as i16,
            height.round().max(1.0) as i16,
        );
        y += height.max(1.0);
        objects.push(Object::text(text));
        if y > f64::from(i16::MAX) {
            break;
        }
    }
    objects
}

/// What Copy puts on the clipboard in Draw mode.
#[derive(Clone, Debug)]
pub struct DrawingClip {
    pub sdw: Vec<u8>,
    pub svg: Vec<u8>,
    pub png: Option<Vec<u8>>,
}

impl DrawingClip {
    pub fn of(objects: Vec<Object>) -> Self {
        let drawing = Drawing {
            objects,
            ..Drawing::new()
        };
        Self {
            sdw: sdw::write(&drawing),
            svg: svg_bytes(&drawing),
            png: png_bytes(&drawing, 192.0),
        }
    }
}

impl cosmic::iced::clipboard::mime::AsMimeTypes for DrawingClip {
    fn available(&self) -> Cow<'static, [String]> {
        let mut types = vec![DRAWING_TYPE.to_owned(), SVG_TYPE.to_owned()];
        if self.png.is_some() {
            types.push(PNG_TYPE.to_owned());
        }
        types.into()
    }

    fn as_bytes(&self, mime: &str) -> Option<Cow<'static, [u8]>> {
        match mime {
            DRAWING_TYPE => Some(self.sdw.clone().into()),
            SVG_TYPE => Some(self.svg.clone().into()),
            PNG_TYPE => self.png.clone().map(Into::into),
            _ => None,
        }
    }
}

/// What Paste takes in Draw mode, most wanted first.
#[derive(Clone, Debug)]
pub struct DrawingPaste {
    pub bytes: Vec<u8>,
    pub mime: String,
}

const PASTE_TYPES: [&str; 7] = [
    DRAWING_TYPE,
    SVG_TYPE,
    PNG_TYPE,
    "image/jpeg",
    "image/bmp",
    "text/plain;charset=utf-8",
    "text/plain",
];

impl TryFrom<(Vec<u8>, String)> for DrawingPaste {
    type Error = ();

    fn try_from((bytes, mime): (Vec<u8>, String)) -> Result<Self, ()> {
        PASTE_TYPES
            .contains(&mime.as_str())
            .then_some(Self { bytes, mime })
            .ok_or(())
    }
}

impl cosmic::iced::clipboard::mime::AllowedMimeTypes for DrawingPaste {
    fn allowed() -> Cow<'static, [String]> {
        PASTE_TYPES.iter().map(|&mime| mime.to_owned()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Drawing {
        Drawing {
            objects: vec![
                Object::rectangle(
                    Rect::new(0, 0, 1_500, 750),
                    Pen::default(),
                    Fill::solid(Colour::rgb(0, 128, 255)),
                    false,
                ),
                Object::line(
                    Point::new(0, 900),
                    Point::new(1_500, 900),
                    Pen::default(),
                    false,
                ),
            ],
            ..Drawing::new()
        }
    }

    #[test]
    fn svg_written_reads_back_as_like_objects() {
        let svg = svg_bytes(&sample());
        let objects = svg_objects(&svg).expect("objects");
        // The rectangle as a filled polygon, the line as a polyline.
        assert_eq!(objects.len(), 2);
        let Shape::Polygon { fill, .. } = &objects[0].shape else {
            panic!("{:?}", objects[0]);
        };
        assert_eq!(fill.foreground, Colour::rgb(0, 128, 255));
        let rect = objects[0].rect;
        assert!((rect.width() - 1_500).abs() <= 2, "{rect:?}");
        assert!(matches!(objects[1].shape, Shape::Polyline { .. }));
    }

    #[test]
    fn pictures_become_bitmap_objects_and_drawings_png() {
        let png = png_bytes(&sample(), 96.0).unwrap();
        let object = image_object(&png).unwrap();
        assert!(matches!(object.shape, Shape::Bitmap { .. }));
        // 101 pixels at 96 dpi: 1,515 twips.
        assert_eq!(object.rect.width(), 1_515);
        let painted = sdw::paint(&Drawing {
            objects: vec![object],
            ..Drawing::new()
        });
        assert!(matches!(
            painted.ops[0],
            amipro_pictures::wmf::Op::Image { .. }
        ));
        // The core's own PNG reads too.
        let raster = Raster::filled(4, 3, [9, 8, 7, 255]).unwrap();
        let decoded = image::load_from_memory(&svg::encode_png(&raster))
            .unwrap()
            .into_rgba8();
        assert_eq!(decoded.get_pixel(3, 2).0, [9, 8, 7, 255]);
    }

    #[test]
    fn text_pastes_as_a_text_object_a_line() {
        let measure = |text: &sdw::Text| (text.text.len() as f64 * 100.0, 276.0);
        let objects = text_objects(
            "One\n\nTwo lines",
            &crate::draw_mode::TextLook::default(),
            &measure,
        );
        assert_eq!(objects.len(), 2);
        let Shape::Text(second) = &objects[1].shape else {
            panic!();
        };
        assert_eq!((second.text.as_str(), second.origin.y), ("Two lines", 276));
    }
}
