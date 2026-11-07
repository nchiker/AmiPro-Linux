//! Ami Pro's drawings (`AmiDraw`, `.sdw`; `docs/research/DRAWING_NOTES.md`):
//! the objects a drawing holds, read from and written to the stream Ami
//! Pro stores (a `.sdw` file and a drawing's bytes in a SAM's `[Embedded]`
//! section are the same), their natural size, and their painting into the
//! display list metafiles play into ([`crate::wmf::Drawing`]), so drawings
//! are drawn as Ami Pro paints them at any size.
//!
//! The stream is a block (`SM`, a version, the object count) of objects,
//! each a 16-byte header (type, flags, length, bounding rectangle, two
//! runtime links) and a body; a group's own block follows its record.
//! Coordinates are twips with y growing downward from the drawing's
//! origin; shapes are stored as points (rectangles as four corners,
//! ellipses and rounded rectangles as cubic Béziers), so turning and
//! flipping are in the points and only text keeps an angle. Everything the
//! reader does not interpret (chart data, unknown records, stale legacy
//! fields) is kept and written back as it was.

use crate::{Bytes, wmf};

/// The version every drawing in the corpora has; older ones Ami Pro
/// converts when reading.
pub const VERSION: u16 = 0x0102;
/// Blocks nest at most this deep (4 in the corpora).
const MAX_DEPTH: usize = 16;
/// Header sizes: a block's and an object's.
const BLOCK: usize = 0x16;
const HEADER: usize = 0x10;

/// The object types (`DrawMan`'s table of names).
pub mod kind {
    pub const LINE: u8 = 2;
    /// A line made with Shift: kept at 0, 45, or 90 degrees.
    pub const PERPENDICULAR_LINE: u8 = 3;
    pub const POLYLINE: u8 = 4;
    pub const POLYGON: u8 = 5;
    pub const RECTANGLE: u8 = 6;
    pub const SQUARE: u8 = 7;
    pub const ROUNDED_RECTANGLE: u8 = 8;
    pub const ROUNDED_SQUARE: u8 = 9;
    pub const ELLIPSE: u8 = 10;
    pub const CIRCLE: u8 = 11;
    pub const ARC: u8 = 12;
    pub const TEXT: u8 = 13;
    pub const GROUP: u8 = 14;
    /// AmiChart's data, never painted.
    pub const CHART: u8 = 15;
    pub const BITMAP: u8 = 18;
}

/// The flag Ami Pro sets on the objects of a chart.
pub const CHART_FLAG: u8 = 4;

/// A point in twips, y down.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Point {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

/// A rectangle in twips: left, top, right, bottom.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct Rect {
    pub left: i16,
    pub top: i16,
    pub right: i16,
    pub bottom: i16,
}

impl Rect {
    pub const fn new(left: i16, top: i16, right: i16, bottom: i16) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> i32 {
        i32::from(self.right) - i32::from(self.left)
    }

    pub fn height(&self) -> i32 {
        i32::from(self.bottom) - i32::from(self.top)
    }

    /// The smallest rectangle holding both.
    pub fn union(&self, other: &Rect) -> Rect {
        Rect::new(
            self.left.min(other.left),
            self.top.min(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
        )
    }

    /// The smallest rectangle holding `points` as Ami Pro gives a shape's
    /// bounds: their extremes, a twip wider or taller where they would be
    /// empty.
    pub fn of_points(points: &[Point]) -> Rect {
        let mut rect = match points.first() {
            Some(first) => Rect::new(first.x, first.y, first.x, first.y),
            None => return Rect::default(),
        };
        for point in points {
            rect = rect.union(&Rect::new(point.x, point.y, point.x, point.y));
        }
        if rect.right == rect.left {
            rect.right = rect.right.saturating_add(1);
        }
        if rect.bottom == rect.top {
            rect.bottom = rect.bottom.saturating_add(1);
        }
        rect
    }
}

/// A `COLORREF`: red, green, blue, and the flags byte (2 for a palette
/// colour), kept as stored.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct Colour(pub [u8; 4]);

impl Colour {
    pub const BLACK: Colour = Colour([0, 0, 0, 0]);
    pub const WHITE: Colour = Colour([255, 255, 255, 0]);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 0])
    }

    pub fn rgb_bytes(&self) -> [u8; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }
}

/// How an object's outline is drawn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Pen {
    /// Twips; 0 draws no line.
    pub width: u8,
    /// GDI's pen styles: 0 solid, 1 dash, 2 dot, 3 dash dot, 4 dash dot
    /// dot, 5 none, 6 inside frame. Any but solid is drawn a pixel wide.
    pub style: u8,
    pub colour: Colour,
    /// The endings at the start and at the end (0 none, 1–9; lines,
    /// polylines, and arcs only).
    pub start: u8,
    pub end: u8,
}

impl Default for Pen {
    /// Ami Pro's line for new objects: 1 pt, solid, black (DR1).
    fn default() -> Self {
        Self {
            width: 20,
            style: 0,
            colour: Colour::BLACK,
            start: 0,
            end: 0,
        }
    }
}

impl Pen {
    fn endings(&self) -> u8 {
        (self.start & 0x0f) | (self.end << 4)
    }

    fn set_endings(&mut self, byte: u8) {
        self.start = byte & 0x0f;
        self.end = byte >> 4;
    }

    /// Whether anything is drawn.
    pub fn visible(&self) -> bool {
        self.width > 0 && self.style != 5
    }
}

/// How a closed object is filled.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Fill {
    pub foreground: Colour,
    pub background: Colour,
    /// 0 none, 1–4 dots (12.5, 25, 50, 75 %), 5 solid, 6–11 the hatches
    /// (horizontal, vertical, `\`, `/`, cross, diagonal cross), 12 the
    /// custom bits.
    pub pattern: u16,
    pub custom: [u8; 8],
}

impl Default for Fill {
    /// Ami Pro's fill for new objects: none, black on white (DR1).
    fn default() -> Self {
        Self {
            foreground: Colour::BLACK,
            background: Colour::WHITE,
            pattern: 0,
            custom: [0; 8],
        }
    }
}

impl Fill {
    pub const NONE: u16 = 0;
    pub const SOLID: u16 = 5;

    pub fn solid(colour: Colour) -> Self {
        Self {
            foreground: colour,
            pattern: Self::SOLID,
            ..Self::default()
        }
    }

    pub fn visible(&self) -> bool {
        self.pattern != Self::NONE
    }
}

/// A text object: one line in one font, spread to its box when unturned.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Text {
    /// The text cell's top-left corner (turned with the text).
    pub origin: Point,
    /// The unturned box's width and height.
    pub size: Point,
    pub face: String,
    pub pitch_and_family: u8,
    /// Twips: negative the character height (−240 is 12 pt), positive the
    /// cell's.
    pub height: i16,
    pub colour: Colour,
    /// 1 bold, 2 italic, 4 or 8 underline, 128 strikeout.
    pub attributes: u8,
    pub attributes2: u8,
    /// Tenths of a degree, clockwise as seen.
    pub angle: i16,
    pub text: String,
    /// What Ami Pro keeps without using it: two bytes after the
    /// attributes, the spacing it last computed, the byte after the NUL,
    /// and bytes of the face field after its NUL.
    pub kept: TextKept,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct TextKept {
    pub unused: [u8; 2],
    pub spacing: i16,
    pub after: u8,
    pub face_tail: Vec<u8>,
}

impl Text {
    pub const BOLD: u8 = 1;
    pub const ITALIC: u8 = 2;
    pub const UNDERLINE: u8 = 4;
    pub const WORD_UNDERLINE: u8 = 8;
    pub const STRIKEOUT: u8 = 128;

    /// The corners of the text's box as turned, clockwise from the
    /// origin.
    pub fn corners(&self) -> [Point; 4] {
        let (w, h) = (f64::from(self.size.x), f64::from(self.size.y));
        let radians = f64::from(self.angle) / 10.0 * std::f64::consts::PI / 180.0;
        let (sin, cos) = radians.sin_cos();
        let at = |x: f64, y: f64| {
            Point::new(
                clamp16(f64::from(self.origin.x) + x * cos - y * sin),
                clamp16(f64::from(self.origin.y) + x * sin + y * cos),
            )
        };
        [at(0.0, 0.0), at(w, 0.0), at(w, h), at(0.0, h)]
    }
}

/// What an object is.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Shape {
    /// Kinds 2 and 3.
    Line {
        points: [Point; 2],
        pen: Pen,
    },
    Polyline {
        points: Vec<Point>,
        pen: Pen,
    },
    /// Closed implicitly.
    Polygon {
        points: Vec<Point>,
        pen: Pen,
        fill: Fill,
    },
    /// Kinds 6 and 7: four corners, drawn as a polygon; `original` is the
    /// legacy unturned rectangle, kept.
    Rectangle {
        original: Rect,
        corners: [Point; 4],
        pen: Pen,
        fill: Fill,
    },
    /// Kinds 8 and 9: four cubic Béziers, one a corner, joined by straight
    /// sides; `radii` legacy, kept.
    RoundedRectangle {
        original: Rect,
        radii: (i16, i16),
        points: [Point; 16],
        pen: Pen,
        fill: Fill,
    },
    /// Kinds 10 and 11: four cubic Béziers sharing their ends (the 13th
    /// point is the first again).
    Ellipse {
        original: Rect,
        points: [Point; 13],
        pen: Pen,
        fill: Fill,
    },
    /// One cubic Bézier; `original` and `ends` legacy, kept.
    Arc {
        original: Rect,
        ends: [Point; 2],
        points: [Point; 4],
        pen: Pen,
    },
    Text(Text),
    Group {
        handle: u16,
        objects: Vec<Object>,
    },
    /// A packed DIB stretched onto the object's rectangle; `kept` four
    /// bytes Ami Pro does not use.
    Bitmap {
        kept: [u8; 4],
        dib: Vec<u8>,
    },
    /// Chart data and anything else: the body as stored.
    Other(Vec<u8>),
}

/// One object of a drawing.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Object {
    pub kind: u8,
    pub flags: u8,
    /// The bounding rectangle Ami Pro stores (and paints and sizes by).
    pub rect: Rect,
    pub shape: Shape,
}

/// A drawing: its objects back to front.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Drawing {
    pub version: u16,
    pub objects: Vec<Object>,
}

fn clamp16(value: f64) -> i16 {
    value
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

/// Reads a drawing; `None` when the bytes are not one (or are damaged).
/// Bytes after the drawing (stale tails of shipped files) are ignored.
pub fn read(bytes: &[u8]) -> Option<Drawing> {
    let read = Bytes(bytes);
    if bytes.get(..2)? != b"SM" {
        return None;
    }
    let version = read.u16(2)?;
    let (objects, _) = read_block(bytes, 0, 0)?;
    Some(Drawing { version, objects })
}

fn read_block(bytes: &[u8], at: usize, depth: usize) -> Option<(Vec<Object>, usize)> {
    if depth > MAX_DEPTH || bytes.get(at..at + 2)? != b"SM" {
        return None;
    }
    let count = usize::from(Bytes(bytes).u16(at + 8)?);
    // Each object takes at least its header.
    if count > bytes.len().saturating_sub(at) / HEADER + 1 {
        return None;
    }
    let mut at = at.checked_add(BLOCK)?;
    let mut objects = Vec::with_capacity(count);
    for _ in 0..count {
        let (object, next) = read_object(bytes, at, depth)?;
        objects.push(object);
        at = next;
    }
    Some((objects, at))
}

fn points(bytes: &[u8], at: usize, count: usize) -> Option<Vec<Point>> {
    let read = Bytes(bytes);
    (0..count)
        .map(|index| {
            let at = at.checked_add(index * 4)?;
            Some(Point::new(read.i16(at)?, read.i16(at + 2)?))
        })
        .collect()
}

fn rect_at(read: &Bytes<'_>, at: usize) -> Option<Rect> {
    Some(Rect::new(
        read.i16(at)?,
        read.i16(at + 2)?,
        read.i16(at + 4)?,
        read.i16(at + 6)?,
    ))
}

fn colour_at(bytes: &[u8], at: usize) -> Option<Colour> {
    Some(Colour(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// The pen as rectangles, polygons, and the other closed shapes store it:
/// width, style, colour.
fn shape_pen(bytes: &[u8], at: usize) -> Option<Pen> {
    Some(Pen {
        width: *bytes.get(at)?,
        style: *bytes.get(at + 1)?,
        colour: colour_at(bytes, at + 2)?,
        start: 0,
        end: 0,
    })
}

fn fill_at(bytes: &[u8], at: usize) -> Option<Fill> {
    Some(Fill {
        foreground: colour_at(bytes, at)?,
        background: colour_at(bytes, at + 4)?,
        pattern: Bytes(bytes).u16(at + 8)?,
        custom: bytes.get(at + 10..at + 18)?.try_into().ok()?,
    })
}

fn read_object(bytes: &[u8], at: usize, depth: usize) -> Option<(Object, usize)> {
    let read = Bytes(bytes);
    let kind = read.u8(at)?;
    let flags = read.u8(at + 1)?;
    let length = usize::from(read.u16(at + 2)?);
    if length < HEADER {
        return None;
    }
    let rect = rect_at(&read, at + 4)?;
    let record = bytes.get(at..at.checked_add(length)?)?;
    let mut next = at + length;
    let shape = match kind {
        kind::GROUP if length == HEADER + 2 => {
            let handle = Bytes(record).u16(HEADER)?;
            let (objects, end) = read_block(bytes, next, depth + 1)?;
            next = end;
            Shape::Group { handle, objects }
        }
        _ => parse_body(kind, record).unwrap_or_else(|| Shape::Other(record[HEADER..].to_vec())),
    };
    Some((
        Object {
            kind,
            flags,
            rect,
            shape,
        },
        next,
    ))
}

/// The shape a record holds, or `None` where its length is not the
/// type's (kept as stored then).
fn parse_body(kind: u8, record: &[u8]) -> Option<Shape> {
    let read = Bytes(record);
    let length = record.len();
    Some(match kind {
        kind::LINE | kind::PERPENDICULAR_LINE if length == 31 => {
            let ends = points(record, 0x10, 2)?;
            let mut pen = Pen {
                width: record[0x18],
                style: record[0x1a],
                colour: colour_at(record, 0x1b)?,
                start: 0,
                end: 0,
            };
            pen.set_endings(record[0x19]);
            Shape::Line {
                points: [ends[0], ends[1]],
                pen,
            }
        }
        kind::POLYLINE => {
            let count = usize::from(read.u16(0x17)?);
            if length != 0x19 + 4 * count {
                return None;
            }
            let mut pen = Pen {
                width: record[0x10],
                style: record[0x12],
                colour: colour_at(record, 0x13)?,
                start: 0,
                end: 0,
            };
            pen.set_endings(record[0x11]);
            Shape::Polyline {
                points: points(record, 0x19, count)?,
                pen,
            }
        }
        kind::POLYGON => {
            let count = usize::from(read.u16(0x28)?);
            if length != 0x2a + 4 * count {
                return None;
            }
            Shape::Polygon {
                points: points(record, 0x2a, count)?,
                pen: shape_pen(record, 0x10)?,
                fill: fill_at(record, 0x16)?,
            }
        }
        kind::RECTANGLE | kind::SQUARE if length == 0x40 => Shape::Rectangle {
            original: rect_at(&read, 0x10)?,
            pen: shape_pen(record, 0x18)?,
            fill: fill_at(record, 0x1e)?,
            corners: points(record, 0x30, 4)?.try_into().ok()?,
        },
        kind::ROUNDED_RECTANGLE | kind::ROUNDED_SQUARE if length == 0x74 => {
            Shape::RoundedRectangle {
                original: rect_at(&read, 0x10)?,
                pen: shape_pen(record, 0x18)?,
                fill: fill_at(record, 0x1e)?,
                radii: (read.i16(0x30)?, read.i16(0x32)?),
                points: points(record, 0x34, 16)?.try_into().ok()?,
            }
        }
        kind::ELLIPSE | kind::CIRCLE if length == 0x64 => Shape::Ellipse {
            original: rect_at(&read, 0x10)?,
            pen: shape_pen(record, 0x18)?,
            fill: fill_at(record, 0x1e)?,
            points: points(record, 0x30, 13)?.try_into().ok()?,
        },
        kind::ARC if length == 0x37 => {
            let ends = points(record, 0x18, 2)?;
            let mut pen = shape_pen(record, 0x20)?;
            pen.set_endings(record[0x26]);
            Shape::Arc {
                original: rect_at(&read, 0x10)?,
                ends: [ends[0], ends[1]],
                pen,
                points: points(record, 0x27, 4)?.try_into().ok()?,
            }
        }
        kind::TEXT if length >= 0x49 => {
            let face_field = &record[0x18..0x38];
            let face_end = face_field.iter().position(|&b| b == 0).unwrap_or(32);
            let text_field = &record[0x47..length - 1];
            let text_end = text_field.iter().position(|&b| b == 0)?;
            if text_end + 1 != text_field.len() {
                return None;
            }
            Shape::Text(Text {
                origin: Point::new(read.i16(0x10)?, read.i16(0x12)?),
                size: Point::new(read.i16(0x14)?, read.i16(0x16)?),
                face: cp1252(&face_field[..face_end]),
                pitch_and_family: record[0x38],
                height: read.i16(0x39)?,
                colour: colour_at(record, 0x3b)?,
                attributes: record[0x3f],
                attributes2: record[0x40],
                angle: read.i16(0x43)?,
                text: cp1252(&text_field[..text_end]),
                kept: TextKept {
                    unused: [record[0x41], record[0x42]],
                    spacing: read.i16(0x45)?,
                    after: record[length - 1],
                    face_tail: {
                        let tail = face_field.get(face_end + 1..).unwrap_or_default();
                        let used = tail
                            .iter()
                            .rposition(|&b| b != 0)
                            .map_or(0, |last| last + 1);
                        tail[..used].to_vec()
                    },
                },
            })
        }
        kind::BITMAP if length > 0x14 => Shape::Bitmap {
            kept: record[0x10..0x14].try_into().ok()?,
            dib: record[0x14..].to_vec(),
        },
        _ => return None,
    })
}

/// Windows-1252 as Ami Pro's text is stored.
fn cp1252(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| crate::wmf::cp1252(b)).collect()
}

/// A character as Windows-1252 stores it (`?` for one it cannot).
pub(crate) fn to_cp1252(c: char) -> u8 {
    if (c as u32) < 0x80 || (0xa0..=0xff).contains(&(c as u32)) {
        return c as u32 as u8;
    }
    (0x80..=0x9f)
        .find(|&b| crate::wmf::cp1252(b) == c)
        .unwrap_or(b'?')
}

/// Writes a drawing as Ami Pro writes one: version 0x0102 blocks, every
/// object's record at its exact length with its bounding rectangle, the
/// runtime links and handles 0 (Ami Pro rebuilds them; DRW0).
pub fn write(drawing: &Drawing) -> Vec<u8> {
    let mut out = Vec::new();
    write_block(&mut out, drawing.version, &drawing.objects);
    out
}

fn write_block(out: &mut Vec<u8>, version: u16, objects: &[Object]) {
    let start = out.len();
    let bounds = objects
        .iter()
        .filter(|object| object.kind != kind::CHART)
        .map(|object| object.rect)
        .reduce(|a, b| a.union(&b))
        .unwrap_or_default();
    out.extend_from_slice(b"SM");
    for word in [version, 0, 0, objects.len() as u16, 0] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for value in [bounds.left, bounds.top, bounds.right, bounds.bottom] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    let used_at = out.len();
    out.extend_from_slice(&0u16.to_le_bytes());
    for object in objects {
        write_object(out, version, object);
    }
    let used = (out.len() - start).min(usize::from(u16::MAX)) as u16;
    out[used_at..used_at + 2].copy_from_slice(&used.to_le_bytes());
}

fn put_points(body: &mut Vec<u8>, points: &[Point]) {
    for point in points {
        body.extend_from_slice(&point.x.to_le_bytes());
        body.extend_from_slice(&point.y.to_le_bytes());
    }
}

fn put_rect(body: &mut Vec<u8>, rect: Rect) {
    for value in [rect.left, rect.top, rect.right, rect.bottom] {
        body.extend_from_slice(&value.to_le_bytes());
    }
}

fn put_fill(body: &mut Vec<u8>, fill: &Fill) {
    body.extend_from_slice(&fill.foreground.0);
    body.extend_from_slice(&fill.background.0);
    body.extend_from_slice(&fill.pattern.to_le_bytes());
    body.extend_from_slice(&fill.custom);
}

fn put_shape_pen(body: &mut Vec<u8>, pen: &Pen) {
    body.push(pen.width);
    body.push(pen.style);
    body.extend_from_slice(&pen.colour.0);
}

fn write_object(out: &mut Vec<u8>, version: u16, object: &Object) {
    let mut body = Vec::new();
    match &object.shape {
        Shape::Line { points, pen } => {
            put_points(&mut body, points);
            body.extend_from_slice(&[pen.width, pen.endings(), pen.style]);
            body.extend_from_slice(&pen.colour.0);
        }
        Shape::Polyline { points, pen } => {
            body.extend_from_slice(&[pen.width, pen.endings(), pen.style]);
            body.extend_from_slice(&pen.colour.0);
            body.extend_from_slice(&(points.len() as u16).to_le_bytes());
            put_points(&mut body, points);
        }
        Shape::Polygon { points, pen, fill } => {
            put_shape_pen(&mut body, pen);
            put_fill(&mut body, fill);
            body.extend_from_slice(&(points.len() as u16).to_le_bytes());
            put_points(&mut body, points);
        }
        Shape::Rectangle {
            original,
            corners,
            pen,
            fill,
        } => {
            put_rect(&mut body, *original);
            put_shape_pen(&mut body, pen);
            put_fill(&mut body, fill);
            put_points(&mut body, corners);
        }
        Shape::RoundedRectangle {
            original,
            radii,
            points,
            pen,
            fill,
        } => {
            put_rect(&mut body, *original);
            put_shape_pen(&mut body, pen);
            put_fill(&mut body, fill);
            body.extend_from_slice(&radii.0.to_le_bytes());
            body.extend_from_slice(&radii.1.to_le_bytes());
            put_points(&mut body, points);
        }
        Shape::Ellipse {
            original,
            points,
            pen,
            fill,
        } => {
            put_rect(&mut body, *original);
            put_shape_pen(&mut body, pen);
            put_fill(&mut body, fill);
            put_points(&mut body, points);
        }
        Shape::Arc {
            original,
            ends,
            points,
            pen,
        } => {
            put_rect(&mut body, *original);
            put_points(&mut body, ends);
            put_shape_pen(&mut body, pen);
            body.push(pen.endings());
            put_points(&mut body, points);
        }
        Shape::Text(text) => {
            put_points(&mut body, &[text.origin, text.size]);
            let mut face: Vec<u8> = text.face.chars().map(to_cp1252).take(31).collect();
            face.push(0);
            if face.len() + text.kept.face_tail.len() <= 32 {
                face.extend_from_slice(&text.kept.face_tail);
            }
            face.resize(32, 0);
            body.extend_from_slice(&face);
            body.push(text.pitch_and_family);
            body.extend_from_slice(&text.height.to_le_bytes());
            body.extend_from_slice(&text.colour.0);
            body.extend_from_slice(&[text.attributes, text.attributes2]);
            body.extend_from_slice(&text.kept.unused);
            body.extend_from_slice(&text.angle.to_le_bytes());
            body.extend_from_slice(&text.kept.spacing.to_le_bytes());
            body.extend(text.text.chars().map(to_cp1252).filter(|&b| b != 0));
            body.push(0);
            body.push(text.kept.after);
        }
        Shape::Group { handle, .. } => body.extend_from_slice(&handle.to_le_bytes()),
        Shape::Bitmap { kept, dib } => {
            body.extend_from_slice(kept);
            body.extend_from_slice(dib);
        }
        Shape::Other(bytes) => body.extend_from_slice(bytes),
    }
    let length = (HEADER + body.len()).min(usize::from(u16::MAX)) as u16;
    out.extend_from_slice(&[object.kind, object.flags & !0xc0]);
    out.extend_from_slice(&length.to_le_bytes());
    put_rect(out, object.rect);
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&body[..usize::from(length) - HEADER]);
    if let Shape::Group { objects, .. } = &object.shape {
        write_block(out, version, objects);
    }
}

impl Object {
    /// How far Ami Pro inflates the object's rectangle for its size
    /// (`DrawMan` `4cc2`): its pen's width, and an ending's on lines,
    /// polylines, and arcs.
    fn inflation(&self) -> i32 {
        let with_endings = |pen: &Pen, per: i32| {
            let width = i32::from(pen.width);
            if pen.start != 0 || pen.end != 0 {
                width + (width / 60 + 1) * per
            } else {
                width
            }
        };
        match &self.shape {
            Shape::Line { pen, .. } => with_endings(pen, 160),
            Shape::Polyline { pen, .. } | Shape::Arc { pen, .. } => with_endings(pen, 120),
            Shape::Polygon { pen, .. }
            | Shape::Rectangle { pen, .. }
            | Shape::RoundedRectangle { pen, .. }
            | Shape::Ellipse { pen, .. } => i32::from(pen.width),
            _ => 0,
        }
    }

    /// The rectangle Ami Pro sizes the drawing by: the stored one inflated
    /// by half the pen and endings; `None` for chart data, which takes no
    /// room.
    pub fn extent(&self) -> Option<(i32, i32, i32, i32)> {
        if self.kind == kind::CHART {
            return None;
        }
        if let Shape::Group { objects, .. } = &self.shape {
            return objects.iter().filter_map(Object::extent).reduce(union4);
        }
        let half = self.inflation() / 2;
        Some((
            i32::from(self.rect.left) - half,
            i32::from(self.rect.top) - half,
            i32::from(self.rect.right) + half,
            i32::from(self.rect.bottom) + half,
        ))
    }
}

fn union4(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
}

/// What an empty drawing measures: 3 inches square.
pub const EMPTY_SIZE: i32 = 4_320;

impl Drawing {
    pub fn new() -> Self {
        Self {
            version: VERSION,
            objects: Vec::new(),
        }
    }

    /// The drawing's natural size in twips (`LIBSIZE`): the right and
    /// bottom of its objects' extent measured from its origin, 3 inches
    /// where that is nothing.
    pub fn natural_size(&self) -> crate::Twips {
        let extent = self
            .objects
            .iter()
            .filter_map(Object::extent)
            .reduce(union4);
        let (right, bottom) = extent.map_or((0, 0), |(_, _, right, bottom)| (right, bottom));
        crate::Twips {
            width: if right > 0 { right } else { EMPTY_SIZE },
            height: if bottom > 0 { bottom } else { EMPTY_SIZE },
        }
    }

    /// Whether any object belongs to a chart.
    pub fn is_chart(&self) -> bool {
        self.objects.iter().any(|object| object.kind == kind::CHART)
    }
}

/// An affine map of twips: `x' = xx·x + xy·y + dx`, `y' = yx·x + yy·y + dy`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub xx: f64,
    pub xy: f64,
    pub yx: f64,
    pub yy: f64,
    pub dx: f64,
    pub dy: f64,
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        xx: 1.0,
        xy: 0.0,
        yx: 0.0,
        yy: 1.0,
        dx: 0.0,
        dy: 0.0,
    };

    pub fn translate(dx: f64, dy: f64) -> Self {
        Self {
            dx,
            dy,
            ..Self::IDENTITY
        }
    }

    /// The map taking rectangle `from` onto `to` (a flip where `to` is
    /// inverted).
    pub fn rect_to_rect(from: Rect, to: (f64, f64, f64, f64)) -> Self {
        let sx = (to.2 - to.0) / f64::from(from.width().max(1));
        let sy = (to.3 - to.1) / f64::from(from.height().max(1));
        Self {
            xx: sx,
            xy: 0.0,
            yx: 0.0,
            yy: sy,
            dx: to.0 - f64::from(from.left) * sx,
            dy: to.1 - f64::from(from.top) * sy,
        }
    }

    /// A turn of `tenths` of a degree clockwise as seen (y down) about
    /// `centre`.
    pub fn rotate(tenths: i32, centre: (f64, f64)) -> Self {
        let radians = f64::from(tenths) / 10.0 * std::f64::consts::PI / 180.0;
        let (sin, cos) = radians.sin_cos();
        Self {
            xx: cos,
            xy: -sin,
            yx: sin,
            yy: cos,
            dx: centre.0 - cos * centre.0 + sin * centre.1,
            dy: centre.1 - sin * centre.0 - cos * centre.1,
        }
    }

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.xx * x + self.xy * y + self.dx,
            self.yx * x + self.yy * y + self.dy,
        )
    }

    fn point(&self, point: Point) -> Point {
        let (x, y) = self.apply(f64::from(point.x), f64::from(point.y));
        Point::new(clamp16(x), clamp16(y))
    }

    /// Whether the map keeps axes (no turn or shear).
    fn keeps_axes(&self) -> bool {
        self.xy.abs() < 1e-9 && self.yx.abs() < 1e-9
    }
}

/// Ami Pro's rounded corner: 30 screen pixels (450 twips at 100 %), at
/// most a third of the side (DR2B).
pub const CORNER_RADIUS: i32 = 450;

/// An ellipse's thirteen points in `rect` (`DrawMan` `8080`; DR2B):
/// handles 9/40 of the size from the ends of the axes.
fn ellipse_points(rect: Rect) -> [Point; 13] {
    let (l, t, r, b) = (
        i32::from(rect.left),
        i32::from(rect.top),
        i32::from(rect.right),
        i32::from(rect.bottom),
    );
    let (w, h) = (r - l, b - t);
    let (kw, kh) = (w * 9 / 40, h * 9 / 40);
    let (cx, cy) = (l + w / 2, t + h / 2);
    let p = |x: i32, y: i32| Point::new(clamp16(f64::from(x)), clamp16(f64::from(y)));
    [
        p(l, cy),
        p(l, t + kh),
        p(l + kw, t),
        p(cx, t),
        p(r - kw, t),
        p(r, t + kh),
        p(r, cy),
        p(r, b - kh),
        p(r - kw, b),
        p(cx, b),
        p(l + kw, b),
        p(l, b - kh),
        p(l, cy),
    ]
}

/// A rounded rectangle's sixteen points and radii (`8080`; DR2B): each
/// corner a cubic whose control points sit 3/20 of the radius from the
/// corner.
fn rounded_points(rect: Rect) -> ([Point; 16], (i16, i16)) {
    let (l, t, r, b) = (
        i32::from(rect.left),
        i32::from(rect.top),
        i32::from(rect.right),
        i32::from(rect.bottom),
    );
    let rx = CORNER_RADIUS.min((r - l) / 3);
    let ry = CORNER_RADIUS.min((b - t) / 3);
    let (cx, cy) = (rx * 3 / 20, ry * 3 / 20);
    let p = |x: i32, y: i32| Point::new(clamp16(f64::from(x)), clamp16(f64::from(y)));
    (
        [
            p(l, t + ry),
            p(l, t + cy),
            p(l + cx, t),
            p(l + rx, t),
            p(r - rx, t),
            p(r - cx, t),
            p(r, t + cy),
            p(r, t + ry),
            p(r, b - ry),
            p(r, b - cy),
            p(r - cx, b),
            p(r - rx, b),
            p(l + rx, b),
            p(l + cx, b),
            p(l, b - cy),
            p(l, b - ry),
        ],
        (rx as i16, -(ry as i16)),
    )
}

impl Object {
    fn of(kind: u8, shape: Shape) -> Self {
        let mut object = Self {
            kind,
            flags: 0,
            rect: Rect::default(),
            shape,
        };
        object.refresh_rect();
        object
    }

    /// A line; `constrained` makes Ami Pro's Shift line (kept level,
    /// plumb, or at 45 degrees).
    pub fn line(a: Point, b: Point, pen: Pen, constrained: bool) -> Self {
        let kind = if constrained {
            kind::PERPENDICULAR_LINE
        } else {
            kind::LINE
        };
        Self::of(
            kind,
            Shape::Line {
                points: [a, b],
                pen,
            },
        )
    }

    pub fn polyline(points: Vec<Point>, pen: Pen) -> Self {
        Self::of(kind::POLYLINE, Shape::Polyline { points, pen })
    }

    pub fn polygon(points: Vec<Point>, pen: Pen, fill: Fill) -> Self {
        Self::of(kind::POLYGON, Shape::Polygon { points, pen, fill })
    }

    /// A rectangle (a square when `square`, Ami Pro's Shift shape).
    pub fn rectangle(rect: Rect, pen: Pen, fill: Fill, square: bool) -> Self {
        let corners = [
            Point::new(rect.left, rect.top),
            Point::new(rect.right, rect.top),
            Point::new(rect.right, rect.bottom),
            Point::new(rect.left, rect.bottom),
        ];
        let kind = if square {
            kind::SQUARE
        } else {
            kind::RECTANGLE
        };
        Self::of(
            kind,
            Shape::Rectangle {
                original: rect,
                corners,
                pen,
                fill,
            },
        )
    }

    pub fn rounded_rectangle(rect: Rect, pen: Pen, fill: Fill, square: bool) -> Self {
        let (points, radii) = rounded_points(rect);
        let kind = if square {
            kind::ROUNDED_SQUARE
        } else {
            kind::ROUNDED_RECTANGLE
        };
        Self::of(
            kind,
            Shape::RoundedRectangle {
                original: rect,
                radii,
                points,
                pen,
                fill,
            },
        )
    }

    pub fn ellipse(rect: Rect, pen: Pen, fill: Fill, circle: bool) -> Self {
        let kind = if circle { kind::CIRCLE } else { kind::ELLIPSE };
        Self::of(
            kind,
            Shape::Ellipse {
                original: rect,
                points: ellipse_points(rect),
                pen,
                fill,
            },
        )
    }

    /// Ami Pro's arc from `a` to `b`: a quarter ellipse leaving `a`
    /// vertically and reaching `b` level (DR2B: control points at 11/20 of
    /// the height and 9/20 of the width).
    pub fn arc(a: Point, b: Point, pen: Pen) -> Self {
        let (x0, y0, x3, y3) = (
            i32::from(a.x),
            i32::from(a.y),
            i32::from(b.x),
            i32::from(b.y),
        );
        let first = Point::new(a.x, clamp16(f64::from(y0 + (y3 - y0) * 11 / 20)));
        let second = Point::new(clamp16(f64::from(x0 + (x3 - x0) * 9 / 20)), b.y);
        let (w, h) = (x3 - x0, y3 - y0);
        let original = Rect::new(
            a.x,
            clamp16(f64::from(y0 - h)),
            clamp16(f64::from(x0 + 2 * w)),
            b.y,
        );
        Self::of(
            kind::ARC,
            Shape::Arc {
                original,
                ends: [a, b],
                points: [a, first, second, b],
                pen,
            },
        )
    }

    pub fn text(text: Text) -> Self {
        Self::of(kind::TEXT, Shape::Text(text))
    }

    /// A group of `objects`, in their order.
    pub fn group(objects: Vec<Object>) -> Self {
        Self::of(kind::GROUP, Shape::Group { handle: 0, objects })
    }

    /// Whether the object keeps its proportions or angle when sized (Ami
    /// Pro's Shift shapes).
    pub fn constrained(&self) -> bool {
        matches!(
            self.kind,
            kind::PERPENDICULAR_LINE | kind::SQUARE | kind::ROUNDED_SQUARE | kind::CIRCLE
        )
    }

    /// Whether Ami Pro flips it (not text or bitmaps).
    pub fn flippable(&self) -> bool {
        match &self.shape {
            Shape::Text(_) | Shape::Bitmap { .. } | Shape::Other(_) => false,
            Shape::Group { objects, .. } => objects.iter().any(Object::flippable),
            _ => true,
        }
    }

    /// Whether the object has a line, and a fill (Extract and Apply Line &
    /// Fill).
    pub fn has_pen(&self) -> bool {
        self.pen().is_some()
    }

    pub fn has_fill(&self) -> bool {
        self.fill().is_some()
    }

    pub fn pen(&self) -> Option<&Pen> {
        match &self.shape {
            Shape::Line { pen, .. }
            | Shape::Polyline { pen, .. }
            | Shape::Polygon { pen, .. }
            | Shape::Rectangle { pen, .. }
            | Shape::RoundedRectangle { pen, .. }
            | Shape::Ellipse { pen, .. }
            | Shape::Arc { pen, .. } => Some(pen),
            _ => None,
        }
    }

    pub fn pen_mut(&mut self) -> Option<&mut Pen> {
        match &mut self.shape {
            Shape::Line { pen, .. }
            | Shape::Polyline { pen, .. }
            | Shape::Polygon { pen, .. }
            | Shape::Rectangle { pen, .. }
            | Shape::RoundedRectangle { pen, .. }
            | Shape::Ellipse { pen, .. }
            | Shape::Arc { pen, .. } => Some(pen),
            _ => None,
        }
    }

    /// Whether the pen takes endings (lines, polylines, arcs).
    pub fn has_endings(&self) -> bool {
        matches!(
            self.shape,
            Shape::Line { .. } | Shape::Polyline { .. } | Shape::Arc { .. }
        )
    }

    pub fn fill(&self) -> Option<&Fill> {
        match &self.shape {
            Shape::Polygon { fill, .. }
            | Shape::Rectangle { fill, .. }
            | Shape::RoundedRectangle { fill, .. }
            | Shape::Ellipse { fill, .. } => Some(fill),
            _ => None,
        }
    }

    pub fn fill_mut(&mut self) -> Option<&mut Fill> {
        match &mut self.shape {
            Shape::Polygon { fill, .. }
            | Shape::Rectangle { fill, .. }
            | Shape::RoundedRectangle { fill, .. }
            | Shape::Ellipse { fill, .. } => Some(fill),
            _ => None,
        }
    }

    /// The object's shape points (what point editing moves), in order; a
    /// text's box corners.
    pub fn points(&self) -> Vec<Point> {
        match &self.shape {
            Shape::Line { points, .. } => points.to_vec(),
            Shape::Polyline { points, .. } | Shape::Polygon { points, .. } => points.clone(),
            Shape::Rectangle { corners, .. } => corners.to_vec(),
            Shape::RoundedRectangle { points, .. } => points.to_vec(),
            Shape::Ellipse { points, .. } => points[..12].to_vec(),
            Shape::Arc { points, .. } => points.to_vec(),
            Shape::Text(text) => text.corners().to_vec(),
            Shape::Group { objects, .. } => objects.iter().flat_map(Object::points).collect(),
            Shape::Bitmap { .. } | Shape::Other(_) => {
                let r = self.rect;
                vec![
                    Point::new(r.left, r.top),
                    Point::new(r.right, r.top),
                    Point::new(r.right, r.bottom),
                    Point::new(r.left, r.bottom),
                ]
            }
        }
    }

    /// Recomputes the bounding rectangle Ami Pro stores from the object's
    /// points (`6646`).
    pub fn refresh_rect(&mut self) {
        self.rect = match &mut self.shape {
            Shape::Group { objects, .. } => {
                for object in objects.iter_mut() {
                    object.refresh_rect();
                }
                objects
                    .iter()
                    .filter(|object| object.kind != kind::CHART)
                    .map(|object| object.rect)
                    .reduce(|a, b| a.union(&b))
                    .unwrap_or_default()
            }
            Shape::Bitmap { .. } | Shape::Other(_) => return,
            _ => Rect::of_points(&self.points()),
        };
    }

    /// Moves, sizes, or flips the object by `map` (turning is
    /// [`Object::rotate`]). Text keeps its angle, its size following the
    /// map's scale; bitmaps take the mapped rectangle.
    pub fn transform(&mut self, map: &Affine) {
        let points = |list: &mut [Point]| {
            for point in list {
                *point = map.point(*point);
            }
        };
        let rect = |rect: &mut Rect| {
            let (a, b) = (
                map.point(Point::new(rect.left, rect.top)),
                map.point(Point::new(rect.right, rect.bottom)),
            );
            *rect = Rect::new(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y));
        };
        match &mut self.shape {
            Shape::Line { points: p, .. } => points(p),
            Shape::Polyline { points: p, .. } | Shape::Polygon { points: p, .. } => points(p),
            Shape::Rectangle {
                original, corners, ..
            } => {
                rect(original);
                points(corners);
            }
            Shape::RoundedRectangle {
                original,
                points: p,
                ..
            } => {
                rect(original);
                points(p);
            }
            Shape::Ellipse {
                original,
                points: p,
                ..
            } => {
                rect(original);
                points(p);
            }
            Shape::Arc {
                original,
                ends,
                points: p,
                ..
            } => {
                rect(original);
                points(ends);
                points(p);
            }
            Shape::Text(text) => {
                let (sx, sy) = (
                    (map.xx * map.xx + map.yx * map.yx).sqrt(),
                    (map.xy * map.xy + map.yy * map.yy).sqrt(),
                );
                // A mirrored text stays readable: it keeps the corner
                // that becomes its box's top-left.
                let corners = text.corners().map(|corner| map.point(corner));
                let origin = if map.keeps_axes() && text.angle == 0 {
                    Point::new(
                        corners.iter().map(|c| c.x).min().unwrap_or(0),
                        corners.iter().map(|c| c.y).min().unwrap_or(0),
                    )
                } else {
                    map.point(text.origin)
                };
                text.origin = origin;
                text.size = Point::new(
                    clamp16(f64::from(text.size.x) * sx),
                    clamp16(f64::from(text.size.y) * sy),
                );
                text.height = clamp16(f64::from(text.height) * sy);
                if text.height == 0 {
                    text.height = -1;
                }
            }
            Shape::Group { objects, .. } => {
                for object in objects {
                    object.transform(map);
                }
            }
            Shape::Bitmap { .. } | Shape::Other(_) => {
                rect(&mut self.rect);
                return;
            }
        }
        self.refresh_rect();
    }

    /// Turns the object `tenths` of a degree clockwise as seen about
    /// `centre` (`DrawMan` `53da`): points turn, text's origin turns and
    /// its angle grows; bitmaps stay as they are.
    pub fn rotate(&mut self, tenths: i32, centre: (f64, f64)) {
        if let Shape::Bitmap { .. } | Shape::Other(_) = self.shape {
            return;
        }
        if let Shape::Group { objects, .. } = &mut self.shape {
            for object in objects {
                object.rotate(tenths, centre);
            }
            self.refresh_rect();
            return;
        }
        let map = Affine::rotate(tenths, centre);
        if let Shape::Text(text) = &mut self.shape {
            text.origin = map.point(text.origin);
            text.angle = (i32::from(text.angle) + tenths).rem_euclid(3_600) as i16;
            self.refresh_rect();
            return;
        }
        self.transform(&map);
    }
}

/// The dot patterns 1–4 as Ami Pro's `DrawMan` holds them: rows from the
/// top, the leftmost pixel the high bit, 0 the foreground (DRW1).
const DOTS: [[u8; 8]; 4] = [
    [0x77, 0xFF, 0xDD, 0xFF, 0x77, 0xFF, 0xDD, 0xFF],
    [0xEE, 0xBB, 0xEE, 0xBB, 0xEE, 0xBB, 0xEE, 0xBB],
    [0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA],
    [0x11, 0x44, 0x11, 0x44, 0x11, 0x44, 0x11, 0x44],
];

/// An 8 × 8 pattern of device pixels: 0 bits `foreground`, 1 bits
/// `background`.
fn pattern(rows: &[u8; 8], foreground: Colour, background: Colour) -> crate::Raster {
    let [fr, fg, fb] = foreground.rgb_bytes();
    let [br, bg, bb] = background.rgb_bytes();
    let mut pixels = Vec::with_capacity(8 * 8 * 4);
    for row in rows {
        for bit in 0..8 {
            if row & (0x80 >> bit) == 0 {
                pixels.extend_from_slice(&[fr, fg, fb, 255]);
            } else {
                pixels.extend_from_slice(&[br, bg, bb, 255]);
            }
        }
    }
    crate::Raster {
        width: 8,
        height: 8,
        pixels,
    }
}

impl Fill {
    /// The brush Ami Pro paints with (DRW1): none, dots, solid, a hatch in
    /// the foreground on the background, or the custom bits.
    pub fn brush(&self) -> Option<wmf::Fill> {
        let (foreground, background) = (self.foreground, self.background);
        match self.pattern {
            1..=4 | 12 if foreground.rgb_bytes() == background.rgb_bytes() => {
                Some(wmf::Fill::Solid(foreground.rgb_bytes()))
            }
            1..=4 => Some(wmf::Fill::Pattern(std::sync::Arc::new(pattern(
                &DOTS[usize::from(self.pattern) - 1],
                foreground,
                background,
            )))),
            Self::SOLID => Some(wmf::Fill::Solid(foreground.rgb_bytes())),
            6..=11 => Some(wmf::Fill::Hatch {
                style: self.pattern - 6,
                colour: foreground.rgb_bytes(),
                background: Some(background.rgb_bytes()),
            }),
            12 => Some(wmf::Fill::Pattern(std::sync::Arc::new(pattern(
                &self.custom,
                foreground,
                background,
            )))),
            _ => None,
        }
    }
}

/// Paints a drawing as Ami Pro does (`LIBPAINT`; DRW1–DRW3) into a display
/// list whose unit square is the drawing's natural size: objects in order,
/// shapes filled then outlined, styled lines a pixel wide, endings, text
/// spread to its box and turned, bitmaps stretched to their rectangles.
/// Chart data is not painted (a chart's look is its other objects).
pub fn paint(drawing: &Drawing) -> wmf::Drawing {
    let size = drawing.natural_size();
    let mut painter = Painter {
        width: size.width as f32,
        height: size.height as f32,
        ops: Vec::new(),
    };
    for object in &drawing.objects {
        painter.object(object);
    }
    wmf::Drawing {
        ops: painter.ops,
        window: (size.width, size.height),
        placeable: Some(size),
    }
}

struct Painter {
    width: f32,
    height: f32,
    ops: Vec<wmf::Op>,
}

/// A point in twips as floats, for endings.
#[derive(Clone, Copy)]
struct At(f32, f32);

impl At {
    fn of(point: Point) -> Self {
        Self(f32::from(point.x), f32::from(point.y))
    }

    fn plus(self, x: f32, y: f32) -> Self {
        Self(self.0 + x, self.1 + y)
    }
}

impl Painter {
    fn unit(&self, at: At) -> wmf::Point {
        wmf::Point {
            x: at.0 / self.width,
            y: at.1 / self.height,
        }
    }

    fn point(&self, point: Point) -> wmf::Point {
        self.unit(At::of(point))
    }

    fn stroke(&self, pen: &Pen) -> Option<wmf::Stroke> {
        pen.visible().then(|| wmf::Stroke {
            colour: pen.colour.rgb_bytes(),
            // A styled pen is one device pixel whatever its width.
            width: if pen.style == 0 {
                f32::from(pen.width) / self.width
            } else {
                0.0
            },
            style: u16::from(pen.style),
        })
    }

    fn path(&mut self, path: Vec<wmf::PathElement>, fill: Option<&Fill>, pen: Option<&Pen>) {
        let fill = fill.and_then(Fill::brush);
        let stroke = pen.and_then(|pen| self.stroke(pen));
        if fill.is_none() && stroke.is_none() {
            return;
        }
        self.ops.push(wmf::Op::Path {
            path,
            fill,
            nonzero: false,
            stroke,
        });
    }

    fn polygon(&self, points: &[Point]) -> Vec<wmf::PathElement> {
        let mut path = Vec::with_capacity(points.len() + 1);
        for (index, point) in points.iter().enumerate() {
            let point = self.point(*point);
            path.push(if index == 0 {
                wmf::PathElement::MoveTo(point)
            } else {
                wmf::PathElement::LineTo(point)
            });
        }
        path.push(wmf::PathElement::Close);
        path
    }

    fn object(&mut self, object: &Object) {
        match &object.shape {
            Shape::Line { points, pen } => {
                let [a, mut b] = *points;
                // Ami Pro straightens a line one twip off level or plumb.
                if (i32::from(b.x) - i32::from(a.x)).abs() == 1 {
                    b.x = a.x;
                }
                if (i32::from(b.y) - i32::from(a.y)).abs() == 1 {
                    b.y = a.y;
                }
                let path = vec![
                    wmf::PathElement::MoveTo(self.point(a)),
                    wmf::PathElement::LineTo(self.point(b)),
                ];
                self.path(path, None, Some(pen));
                self.endings(pen, (a, b), (b, a));
            }
            Shape::Polyline { points, pen } => {
                if points.len() < 2 {
                    return;
                }
                let mut path = self.polygon(points);
                path.pop();
                self.path(path, None, Some(pen));
                let last = points.len() - 1;
                self.endings(
                    pen,
                    (points[0], points[1]),
                    (points[last], points[last - 1]),
                );
            }
            Shape::Polygon { points, pen, fill } => {
                if points.len() >= 2 {
                    let path = self.polygon(points);
                    self.path(path, Some(fill), Some(pen));
                }
            }
            Shape::Rectangle {
                corners, pen, fill, ..
            } => {
                let path = self.polygon(corners);
                self.path(path, Some(fill), Some(pen));
            }
            Shape::RoundedRectangle {
                points, pen, fill, ..
            } => {
                let p = |index: usize| self.point(points[index]);
                let mut path = vec![wmf::PathElement::MoveTo(p(0))];
                for corner in 0..4 {
                    let at = corner * 4;
                    if corner > 0 {
                        path.push(wmf::PathElement::LineTo(p(at)));
                    }
                    path.push(wmf::PathElement::CubicTo(p(at + 1), p(at + 2), p(at + 3)));
                }
                path.push(wmf::PathElement::Close);
                self.path(path, Some(fill), Some(pen));
            }
            Shape::Ellipse {
                points, pen, fill, ..
            } => {
                let p = |index: usize| self.point(points[index]);
                let mut path = vec![wmf::PathElement::MoveTo(p(0))];
                for segment in 0..4 {
                    let at = segment * 3;
                    path.push(wmf::PathElement::CubicTo(p(at + 1), p(at + 2), p(at + 3)));
                }
                path.push(wmf::PathElement::Close);
                self.path(path, Some(fill), Some(pen));
            }
            Shape::Arc { points, pen, .. } => {
                let p = |index: usize| self.point(points[index]);
                let path = vec![
                    wmf::PathElement::MoveTo(p(0)),
                    wmf::PathElement::CubicTo(p(1), p(2), p(3)),
                ];
                self.path(path, None, Some(pen));
                self.endings(pen, (points[0], points[1]), (points[3], points[2]));
            }
            Shape::Text(text) => {
                // Unturned text fills its stored rectangle, turned text its
                // box (`DrawMan` `6ea2`, `7c4c`).
                let width = if text.angle == 0 {
                    object.rect.width()
                } else {
                    i32::from(text.size.x)
                };
                self.text(text, width);
            }
            Shape::Group { objects, .. } => {
                for object in objects {
                    self.object(object);
                }
            }
            Shape::Bitmap { dib, .. } => {
                if let Some(raster) = crate::dib::decode_packed(dib, crate::dib::ColourTable::Rgb) {
                    let (width, height) = (raster.width, raster.height);
                    let rect = object.rect;
                    let (a, b) = (
                        self.point(Point::new(rect.left, rect.top)),
                        self.point(Point::new(rect.right, rect.bottom)),
                    );
                    self.ops.push(wmf::Op::Image {
                        raster: std::sync::Arc::new(raster),
                        source: [0, 0, width, height],
                        dest: wmf::Rect {
                            left: a.x,
                            top: a.y,
                            right: b.x,
                            bottom: b.y,
                        },
                        rop: wmf::Rop::Copy,
                    });
                }
            }
            Shape::Other(_) => {}
        }
    }

    /// A pen's endings (`DrawMan` `34e8`; DRW2): `start` and `end` are each
    /// an end point and the point the line leaves it towards.
    fn endings(&mut self, pen: &Pen, start: (Point, Point), end: (Point, Point)) {
        if !pen.visible() {
            return;
        }
        for (kind, (tip, towards)) in [(pen.start, start), (pen.end, end)] {
            if kind != 0 {
                self.ending(pen, kind, At::of(tip), At::of(towards));
            }
        }
    }

    fn ending(&mut self, pen: &Pen, kind: u8, tip: At, towards: At) {
        let unit = f32::from(pen.width / 60 + 1);
        let (length, half) = (120.0 * unit, 60.0 * unit);
        let (dx, dy) = (towards.0 - tip.0, towards.1 - tip.1);
        let norm = (dx * dx + dy * dy).sqrt();
        // Into the line, and across it.
        let (ux, uy) = if norm > 0.0 {
            (dx / norm, dy / norm)
        } else {
            (1.0, 0.0)
        };
        let (nx, ny) = (-uy, ux);
        let along = |at: At, by: f32| at.plus(ux * by, uy * by);
        let across = |at: At, by: f32| at.plus(nx * by, ny * by);
        let solid = Fill::solid(pen.colour);
        let shape = |points: &[At], closed: bool| {
            let mut path: Vec<wmf::PathElement> = points
                .iter()
                .enumerate()
                .map(|(index, &at)| {
                    let point = self.unit(at);
                    if index == 0 {
                        wmf::PathElement::MoveTo(point)
                    } else {
                        wmf::PathElement::LineTo(point)
                    }
                })
                .collect();
            if closed {
                path.push(wmf::PathElement::Close);
            }
            path
        };
        // Arrowheads 1–3 point out of the line; 4–6 the same turned back
        // (a tail).
        let (direction, kind) = match kind {
            1..=3 => (1.0, kind),
            4..=6 => (-1.0, kind - 3),
            _ => (0.0, kind),
        };
        let base = along(tip, direction * length);
        let notch = along(tip, direction * length / 2.0);
        let (left, right) = (across(base, half), across(base, -half));
        let (path, filled) = match kind {
            1 if direction != 0.0 => (shape(&[tip, left, right], true), true),
            2 if direction != 0.0 => (shape(&[tip, left, notch, right], true), true),
            3 if direction != 0.0 => (shape(&[left, tip, right], false), false),
            7 => (
                shape(&[across(tip, half), across(tip, -half)], false),
                false,
            ),
            8 => {
                let corner = |a: f32, b: f32| across(along(tip, a), b);
                (
                    shape(
                        &[
                            corner(half, half),
                            corner(half, -half),
                            corner(-half, -half),
                            corner(-half, half),
                        ],
                        true,
                    ),
                    true,
                )
            }
            9 => {
                // A circle of radius `half` as four cubics.
                let k = half * 0.552_284_8;
                let c = |x: f32, y: f32| self.unit(tip.plus(x, y));
                (
                    vec![
                        wmf::PathElement::MoveTo(c(half, 0.0)),
                        wmf::PathElement::CubicTo(c(half, k), c(k, half), c(0.0, half)),
                        wmf::PathElement::CubicTo(c(-k, half), c(-half, k), c(-half, 0.0)),
                        wmf::PathElement::CubicTo(c(-half, -k), c(-k, -half), c(0.0, -half)),
                        wmf::PathElement::CubicTo(c(k, -half), c(half, -k), c(half, 0.0)),
                        wmf::PathElement::Close,
                    ],
                    true,
                )
            }
            _ => return,
        };
        self.path(path, filled.then_some(&solid), Some(pen));
    }

    fn text(&mut self, text: &Text, width: i32) {
        if text.text.is_empty() {
            return;
        }
        let mut height = f32::from(text.height.unsigned_abs());
        // Ami Pro's small-type attributes (`DrawMan` `7344`; no file has
        // them).
        if text.attributes & 0x20 != 0 {
            height = height * 7.0 / 10.0;
        }
        if text.attributes2 & 3 != 0 {
            height = height * 7.0 / 12.0;
        }
        let characters = text.text.chars().count();
        self.ops.push(wmf::Op::Text {
            origin: self.point(text.origin),
            text: text.text.clone(),
            font: wmf::Font {
                face: text.face.clone(),
                height: height / self.height,
                cell_height: text.height > 0,
                escapement: if text.angle == 0 {
                    0
                } else {
                    (3_600 - i32::from(text.angle)).rem_euclid(3_600) as i16
                },
                weight: if text.attributes & Text::BOLD != 0 {
                    700
                } else {
                    400
                },
                italic: text.attributes & Text::ITALIC != 0,
                underline: text.attributes & (Text::UNDERLINE | Text::WORD_UNDERLINE) != 0,
                strike_out: text.attributes & Text::STRIKEOUT != 0,
            },
            colour: text.colour.rgb_bytes(),
            background: None,
            align: 0,
            advances: None,
            spread: (characters > 1 && width > 0).then(|| width as f32 / self.width),
            clip: None,
            opaque: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stream as Ami Pro writes one: block header, records.
    fn stream(records: &[Vec<u8>]) -> Vec<u8> {
        let mut out = b"SM".to_vec();
        for word in [VERSION, 7, 9, records.len() as u16, 0, 0, 0, 0, 0, 0] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        for record in records {
            out.extend_from_slice(record);
        }
        out
    }

    fn record(kind: u8, rect: Rect, body: &[u8]) -> Vec<u8> {
        let mut out = vec![kind, 0];
        out.extend_from_slice(&((HEADER + body.len()) as u16).to_le_bytes());
        put_rect(&mut out, rect);
        out.extend_from_slice(&[3, 0, 5, 0]);
        out.extend_from_slice(body);
        out
    }

    /// DR1's rectangle as Ami Pro wrote it.
    fn dr1_rectangle() -> Vec<u8> {
        let mut body = Vec::new();
        put_rect(&mut body, Rect::new(428, 165, 2076, 906));
        body.extend_from_slice(&[20, 0, 0, 0, 0, 0]);
        body.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0, 0, 0]);
        body.extend_from_slice(b"WOW\0WOW\0");
        put_points(
            &mut body,
            &[
                Point::new(428, 165),
                Point::new(2076, 165),
                Point::new(2076, 906),
                Point::new(428, 906),
            ],
        );
        record(kind::RECTANGLE, Rect::new(428, 165, 2076, 906), &body)
    }

    #[test]
    fn a_rectangle_reads_with_its_pen_fill_and_corners_dr1() {
        let drawing = read(&stream(&[dr1_rectangle()])).unwrap();
        assert_eq!(drawing.version, VERSION);
        let [object] = drawing.objects.as_slice() else {
            panic!("{drawing:?}");
        };
        let Shape::Rectangle {
            corners, pen, fill, ..
        } = &object.shape
        else {
            panic!("{object:?}");
        };
        assert_eq!(corners[2], Point::new(2076, 906));
        assert_eq!(*pen, Pen::default());
        assert_eq!(fill.pattern, 0);
        assert_eq!(&fill.custom, b"WOW\0WOW\0");
        // DR1's [isd] rectangle: 0 0 2086 -916.
        assert_eq!(
            drawing.natural_size(),
            crate::Twips {
                width: 2086,
                height: 916
            }
        );
    }

    #[test]
    fn writing_keeps_every_byte_but_the_runtime_links() {
        let mut text = Vec::new();
        put_points(&mut text, &[Point::new(300, 300), Point::new(1400, 280)]);
        let mut face = b"Arial".to_vec();
        face.resize(32, 0);
        face[20] = 9; // a stale byte after the face's NUL
        text.extend_from_slice(&face);
        text.push(0x22);
        text.extend_from_slice(&(-240i16).to_le_bytes());
        text.extend_from_slice(&[255, 0, 0, 0, 1, 0, 7, 8]);
        text.extend_from_slice(&300i16.to_le_bytes());
        text.extend_from_slice(&12i16.to_le_bytes());
        text.extend_from_slice(b"Caf\xe9\0\x55");
        let mut line = Vec::new();
        put_points(&mut line, &[Point::new(0, 0), Point::new(900, 450)]);
        line.extend_from_slice(&[120, 0x21, 0, 0, 0, 255, 0]);
        let mut group_child = stream(&[record(kind::LINE, Rect::new(0, 0, 900, 450), &line)]);
        group_child[4..8].fill(0);
        let original = stream(&[
            dr1_rectangle(),
            record(kind::TEXT, Rect::new(300, 300, 1700, 580), &text),
            record(kind::GROUP, Rect::new(0, 0, 900, 450), &[4, 0]),
        ]);
        let mut original = original;
        original.extend_from_slice(&group_child);
        let drawing = read(&original).unwrap();
        let Shape::Text(read_text) = &drawing.objects[1].shape else {
            panic!();
        };
        assert_eq!(read_text.text, "Café");
        assert_eq!(read_text.angle, 300);
        let Shape::Group { objects, .. } = &drawing.objects[2].shape else {
            panic!();
        };
        let Shape::Line { pen, .. } = &objects[0].shape else {
            panic!();
        };
        assert_eq!((pen.start, pen.end), (1, 2));
        let written = write(&drawing);
        assert_eq!(read(&written).unwrap(), drawing);
        // Byte for byte but for the block's handles, bounds, and bytes
        // used, and each record's links.
        assert_eq!(written.len(), original.len());
        let child_block = original.len() - group_child.len();
        let normal = |mut bytes: Vec<u8>| {
            for block in [0, child_block] {
                bytes[block + 4..block + 8].fill(0);
                bytes[block + 12..block + 22].fill(0);
            }
            let mut at = BLOCK;
            while at + HEADER <= bytes.len() {
                if &bytes[at..at + 2] == b"SM" {
                    at += BLOCK;
                    continue;
                }
                bytes[at + 12..at + 16].fill(0);
                at += usize::from(u16::from_le_bytes([bytes[at + 2], bytes[at + 3]]));
            }
            bytes
        };
        assert_eq!(normal(written), normal(original));
    }

    #[test]
    fn damaged_streams_are_refused_without_panicking() {
        let good = stream(&[dr1_rectangle()]);
        for cut in 0..good.len() {
            assert!(read(&good[..cut]).is_none(), "{cut}");
        }
        let mut bad = good.clone();
        bad[BLOCK + 2] = 4; // a record shorter than its header
        assert!(read(&bad).is_none());
        let mut nested = stream(&[record(kind::GROUP, Rect::default(), &[0, 0])]);
        for _ in 0..40 {
            nested.extend_from_slice(&stream(&[record(kind::GROUP, Rect::default(), &[0, 0])]));
        }
        assert!(read(&nested).is_none());
        // A rectangle record of the wrong length is kept as it is.
        let odd = stream(&[record(kind::RECTANGLE, Rect::new(0, 0, 5, 5), &[1, 2, 3])]);
        let drawing = read(&odd).unwrap();
        assert_eq!(drawing.objects[0].shape, Shape::Other(vec![1, 2, 3]));
        assert_eq!(read(&write(&drawing)).unwrap(), drawing);
    }

    #[test]
    fn the_natural_size_takes_pens_and_endings_and_ignores_chart_data() {
        let mut drawing = Drawing::new();
        assert_eq!(
            drawing.natural_size(),
            crate::Twips {
                width: EMPTY_SIZE,
                height: EMPTY_SIZE
            }
        );
        drawing.objects.push(Object {
            kind: kind::LINE,
            flags: 0,
            rect: Rect::new(100, 100, 1000, 101),
            shape: Shape::Line {
                points: [Point::new(100, 100), Point::new(1000, 100)],
                pen: Pen {
                    width: 120,
                    start: 1,
                    ..Pen::default()
                },
            },
        });
        drawing.objects.push(Object {
            kind: kind::CHART,
            flags: CHART_FLAG,
            rect: Rect::new(0, 0, 9000, 9000),
            shape: Shape::Other(vec![0; 4]),
        });
        // 120 + 3 × 160 = 600, half of it each side.
        assert_eq!(
            drawing.natural_size(),
            crate::Twips {
                width: 1300,
                height: 401
            }
        );
    }

    #[test]
    fn painting_fills_then_outlines_and_turns_text_as_ami_pro() {
        let rect = |pattern| Object {
            kind: kind::RECTANGLE,
            flags: 0,
            rect: Rect::new(0, 0, 1000, 500),
            shape: Shape::Rectangle {
                original: Rect::new(0, 0, 1000, 500),
                corners: [
                    Point::new(0, 0),
                    Point::new(1000, 0),
                    Point::new(1000, 500),
                    Point::new(0, 500),
                ],
                pen: Pen {
                    style: 2,
                    ..Pen::default()
                },
                fill: Fill {
                    foreground: Colour::rgb(255, 0, 0),
                    background: Colour::rgb(255, 255, 0),
                    pattern,
                    custom: [0; 8],
                },
            },
        };
        let drawing = Drawing {
            version: VERSION,
            objects: vec![rect(0), rect(1), rect(5), rect(8)],
        };
        let painted = paint(&drawing);
        assert_eq!(painted.window, (1010, 510));
        let fills: Vec<_> = painted
            .ops
            .iter()
            .map(|op| match op {
                wmf::Op::Path { fill, stroke, .. } => {
                    // Dotted: a pixel wide.
                    assert_eq!(stroke.unwrap().width, 0.0);
                    fill.clone()
                }
                _ => panic!("{op:?}"),
            })
            .collect();
        assert_eq!(fills[0], None);
        let Some(wmf::Fill::Pattern(dots)) = &fills[1] else {
            panic!("{:?}", fills[1]);
        };
        // 12.5 % red dots on yellow.
        assert_eq!(dots.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(dots.pixel(1, 0), [255, 255, 0, 255]);
        assert_eq!(dots.pixel(0, 1), [255, 255, 0, 255]);
        assert_eq!(dots.pixel(2, 2), [255, 0, 0, 255]);
        assert_eq!(fills[2], Some(wmf::Fill::Solid([255, 0, 0])));
        assert_eq!(
            fills[3],
            Some(wmf::Fill::Hatch {
                style: 2,
                colour: [255, 0, 0],
                background: Some([255, 255, 0])
            })
        );
        let text = Object {
            kind: kind::TEXT,
            flags: 0,
            rect: Rect::new(0, 0, 1400, 600),
            shape: Shape::Text(Text {
                origin: Point::new(100, 100),
                size: Point::new(1400, 280),
                face: "Arial".into(),
                pitch_and_family: 0x22,
                height: -240,
                colour: Colour::BLACK,
                attributes: Text::BOLD | Text::UNDERLINE,
                attributes2: 0,
                angle: 300,
                text: "Turned".into(),
                kept: TextKept::default(),
            }),
        };
        let painted = paint(&Drawing {
            version: VERSION,
            objects: vec![text],
        });
        let wmf::Op::Text { font, spread, .. } = &painted.ops[0] else {
            panic!();
        };
        // 30° clockwise is 330° counter-clockwise.
        assert_eq!(font.escapement, 3_300);
        assert_eq!(font.weight, 700);
        assert!(font.underline);
        assert_eq!(*spread, Some(1.0));
    }

    #[test]
    fn endings_point_out_of_the_line_and_back_dr2() {
        let line = |start| Object {
            kind: kind::LINE,
            flags: 0,
            rect: Rect::new(1000, 1000, 3000, 1001),
            shape: Shape::Line {
                points: [Point::new(1000, 1000), Point::new(3000, 1000)],
                pen: Pen {
                    width: 120,
                    start,
                    ..Pen::default()
                },
            },
        };
        let tips = |start| {
            let painted = paint(&Drawing {
                version: VERSION,
                objects: vec![line(start)],
            });
            let (width, height) = (painted.window.0 as f32, painted.window.1 as f32);
            let wmf::Op::Path { path, fill, .. } = &painted.ops[1] else {
                panic!();
            };
            let xs: Vec<f32> = path
                .iter()
                .filter_map(|element| match element {
                    wmf::PathElement::MoveTo(p) | wmf::PathElement::LineTo(p) => {
                        Some((p.x * width).round())
                    }
                    _ => None,
                })
                .collect();
            let _ = height;
            (xs, fill.is_some())
        };
        // Width 120: a unit of 3, 360 long, 180 to each side.
        assert_eq!(tips(1), (vec![1000.0, 1360.0, 1360.0], true));
        assert_eq!(tips(2), (vec![1000.0, 1360.0, 1180.0, 1360.0], true));
        assert_eq!(tips(3), (vec![1360.0, 1000.0, 1360.0], false));
        assert_eq!(tips(4), (vec![1000.0, 640.0, 640.0], true));
        // A line no wider than nothing has no ending either.
        let mut hidden = line(1);
        if let Shape::Line { pen, .. } = &mut hidden.shape {
            pen.width = 0;
        }
        assert!(
            paint(&Drawing {
                version: VERSION,
                objects: vec![hidden]
            })
            .ops
            .is_empty()
        );
    }

    #[test]
    fn new_shapes_are_built_as_ami_pro_builds_them_dr2b() {
        let p = Point::new;
        let ellipse = Object::ellipse(
            Rect::new(2373, 461, 3856, 1615),
            Pen::default(),
            Fill::default(),
            false,
        );
        let Shape::Ellipse { points, .. } = &ellipse.shape else {
            panic!();
        };
        assert_eq!(
            points.to_vec(),
            vec![
                p(2373, 1038),
                p(2373, 720),
                p(2706, 461),
                p(3114, 461),
                p(3523, 461),
                p(3856, 720),
                p(3856, 1038),
                p(3856, 1356),
                p(3523, 1615),
                p(3114, 1615),
                p(2706, 1615),
                p(2373, 1356),
                p(2373, 1038),
            ]
        );
        assert_eq!(ellipse.rect, Rect::new(2373, 461, 3856, 1615));
        let arc = Object::arc(p(4515, 461), p(6163, 1779), Pen::default());
        let Shape::Arc {
            points, original, ..
        } = &arc.shape
        else {
            panic!();
        };
        assert_eq!(
            *points,
            [p(4515, 461), p(4515, 1185), p(5256, 1779), p(6163, 1779)]
        );
        assert_eq!(*original, Rect::new(4515, -857, 7811, 1779));
        assert_eq!(arc.rect, Rect::new(4515, 461, 6163, 1779));
        // DR2B's rounded rectangle at its own radii; Ami Pro's radius is 30
        // of its screen pixels (494 twips at its 91 %), here 450, at most a
        // third of the side (384 of 1,154).
        let rounded = Object::rounded_rectangle(
            Rect::new(230, 461, 1878, 1615),
            Pen::default(),
            Fill::default(),
            false,
        );
        let Shape::RoundedRectangle { points, radii, .. } = &rounded.shape else {
            panic!();
        };
        assert_eq!(*radii, (450, -384));
        assert_eq!(points[0], p(230, 845));
        assert_eq!(points[1], p(230, 518));
        assert_eq!(points[3], p(680, 461));
        assert_eq!(points[15], p(230, 1231));
    }

    #[test]
    fn objects_move_size_flip_and_turn() {
        let p = Point::new;
        let mut rectangle = Object::rectangle(
            Rect::new(0, 0, 1000, 500),
            Pen::default(),
            Fill::default(),
            false,
        );
        rectangle.transform(&Affine::translate(100.0, 50.0));
        assert_eq!(rectangle.rect, Rect::new(100, 50, 1100, 550));
        // Sized onto a rectangle, then flipped across it.
        rectangle.transform(&Affine::rect_to_rect(
            rectangle.rect,
            (100.0, 50.0, 2100.0, 1050.0),
        ));
        assert_eq!(rectangle.rect, Rect::new(100, 50, 2100, 1050));
        rectangle.transform(&Affine::rect_to_rect(
            rectangle.rect,
            (2100.0, 50.0, 100.0, 1050.0),
        ));
        let Shape::Rectangle { corners, .. } = &rectangle.shape else {
            panic!();
        };
        assert_eq!(corners[0], p(2100, 50));
        assert_eq!(rectangle.rect, Rect::new(100, 50, 2100, 1050));
        // A quarter turn clockwise about its centre.
        let mut line = Object::line(p(0, 0), p(1000, 0), Pen::default(), false);
        line.rotate(900, (500.0, 0.0));
        assert_eq!(line.rect, Rect::new(500, -500, 501, 500));
        let Shape::Line { points, .. } = &line.shape else {
            panic!();
        };
        // Clockwise as seen: the left end goes up.
        assert_eq!(*points, [p(500, -500), p(500, 500)]);
        // Text turns its origin and adds to its angle; sizing scales it.
        let mut text = Object::text(Text {
            origin: p(0, 0),
            size: p(1000, 240),
            face: "Arial".into(),
            pitch_and_family: 0x22,
            height: -240,
            colour: Colour::BLACK,
            attributes: 0,
            attributes2: 0,
            angle: 0,
            text: "Hello".into(),
            kept: TextKept::default(),
        });
        assert_eq!(text.rect, Rect::new(0, 0, 1000, 240));
        text.transform(&Affine::rect_to_rect(text.rect, (0.0, 0.0, 2000.0, 480.0)));
        let Shape::Text(read) = &text.shape else {
            panic!();
        };
        assert_eq!((read.size, read.height), (p(2000, 480), -480));
        text.rotate(300, (0.0, 0.0));
        let Shape::Text(read) = &text.shape else {
            panic!();
        };
        assert_eq!(read.angle, 300);
        // A group moves and sizes its members.
        let mut group = Object::group(vec![line.clone(), rectangle.clone()]);
        assert_eq!(group.rect, Rect::new(100, -500, 2100, 1050));
        group.transform(&Affine::translate(10.0, 10.0));
        assert_eq!(group.rect, Rect::new(110, -490, 2110, 1060));
        assert!(group.flippable() && !text.flippable());
    }
}
