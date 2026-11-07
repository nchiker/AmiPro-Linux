//! Enhanced metafiles (the published MS-EMF format: Word's and
//! LibreOffice's drawings and charts), played into the display list Windows
//! metafiles play into ([`wmf::Drawing`]).
//!
//! Each record's logical coordinates go through the world transform and the
//! mapping mode onto the reference device, and the header's frame (its
//! picture's rectangle, in hundredths of a millimetre on that device) maps
//! onto the unit square, so the app draws the list at any size as it draws
//! a Windows metafile. The records are those drawing programs write: the
//! mapping, the world transform, pens, brushes, fonts, the poly records in
//! both sizes, rectangles, ellipses, arcs, paths, clips, bitmaps
//! (`StretchDIBits`, `BitBlt`, `StretchBlt`, `SetDIBitsToDevice`), and
//! `ExtTextOut` in both kinds. EMF+ comments are skipped: a file with them
//! carries its EMF records too.

use std::f32::consts::PI;
use std::sync::Arc;

use crate::dib::{self, ColourTable, Rgb};
use crate::wmf::{
    Drawing, Fill, Font, Op, PathElement, Point, Rect, Rop, Stroke, arc, cp1252, ellipse_path,
    intersect, raster_op, sweep_angle,
};
use crate::{Bytes, Twips};

/// ` EMF` in the header.
const SIGNATURE: u32 = 0x464D_4520;
/// The most records a file may have, as a bound on malformed files.
const MAX_RECORDS: usize = 2_000_000;
/// The most objects a file may define.
const MAX_OBJECTS: usize = 65_536;

/// The header's frame: the picture's rectangle in hundredths of a
/// millimetre, and the reference device's pixels a millimetre.
struct Header {
    frame: [i32; 4],
    bounds: [i32; 4],
    pixels_per_mm: (f64, f64),
}

fn header(bytes: &[u8]) -> Option<Header> {
    let read = Bytes(bytes);
    if read.u32(0)? != 1 || read.u32(40)? != SIGNATURE {
        return None;
    }
    let rect = |at: usize| -> Option<[i32; 4]> {
        Some([
            read.i32(at)?,
            read.i32(at + 4)?,
            read.i32(at + 8)?,
            read.i32(at + 12)?,
        ])
    };
    let device = (read.i32(72)?, read.i32(76)?);
    let millimetres = (read.i32(80)?, read.i32(84)?);
    let per = |pixels: i32, mm: i32| {
        if pixels > 0 && mm > 0 {
            f64::from(pixels) / f64::from(mm)
        } else {
            // 96 pixels an inch.
            96.0 / 25.4
        }
    };
    Some(Header {
        bounds: rect(8)?,
        frame: rect(24)?,
        pixels_per_mm: (per(device.0, millimetres.0), per(device.1, millimetres.1)),
    })
}

/// The picture's size, in twips, from its header's frame (or its bounds on
/// the reference device where the frame is empty).
pub fn size(bytes: &[u8]) -> Option<Twips> {
    let header = header(bytes)?;
    let [left, top, right, bottom] = header.frame;
    let (width, height) = (
        i64::from(right) - i64::from(left),
        i64::from(bottom) - i64::from(top),
    );
    if width > 0 && height > 0 {
        return Some(Twips {
            width: ((width * 1_440 + 1_270) / 2_540).min(i64::from(i32::MAX)) as i32,
            height: ((height * 1_440 + 1_270) / 2_540).min(i64::from(i32::MAX)) as i32,
        });
    }
    let [left, top, right, bottom] = header.bounds;
    let (width, height) = (
        f64::from(right) - f64::from(left) + 1.0,
        f64::from(bottom) - f64::from(top) + 1.0,
    );
    (width > 0.0 && height > 0.0).then(|| Twips {
        width: (width / header.pixels_per_mm.0 * 1_440.0 / 25.4) as i32,
        height: (height / header.pixels_per_mm.1 * 1_440.0 / 25.4) as i32,
    })
}

/// A world transform: `x' = x·m11 + y·m21 + dx`, `y' = x·m12 + y·m22 + dy`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Transform {
    m11: f64,
    m12: f64,
    m21: f64,
    m22: f64,
    dx: f64,
    dy: f64,
}

impl Transform {
    const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        dx: 0.0,
        dy: 0.0,
    };

    fn read(p: &Bytes<'_>, at: usize) -> Option<Self> {
        let f = |index: usize| {
            p.u32(at + 4 * index)
                .map(|bits| f64::from(f32::from_bits(bits)))
        };
        let transform = Self {
            m11: f(0)?,
            m12: f(1)?,
            m21: f(2)?,
            m22: f(3)?,
            dx: f(4)?,
            dy: f(5)?,
        };
        [
            transform.m11,
            transform.m12,
            transform.m21,
            transform.m22,
            transform.dx,
            transform.dy,
        ]
        .iter()
        .all(|value| value.is_finite())
        .then_some(transform)
    }

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x * self.m11 + y * self.m21 + self.dx,
            x * self.m12 + y * self.m22 + self.dy,
        )
    }

    /// `self` first, then `other`.
    fn then(&self, other: &Self) -> Self {
        Self {
            m11: self.m11 * other.m11 + self.m12 * other.m21,
            m12: self.m11 * other.m12 + self.m12 * other.m22,
            m21: self.m21 * other.m11 + self.m22 * other.m21,
            m22: self.m21 * other.m12 + self.m22 * other.m22,
            dx: self.dx * other.m11 + self.dy * other.m21 + other.dx,
            dy: self.dx * other.m12 + self.dy * other.m22 + other.dy,
        }
    }
}

#[derive(Clone, Debug)]
struct LogFont {
    face: String,
    height: i32,
    escapement: i32,
    weight: i32,
    italic: bool,
    underline: bool,
    strike_out: bool,
}

#[derive(Clone, Debug)]
enum Object {
    /// A pen: its colour, its width in logical units (0 one device pixel),
    /// and its style; `None` the null pen.
    Pen(Option<(Rgb, f64, u16)>),
    Brush(Option<Fill>),
    Font(LogFont),
    Other,
}

#[derive(Clone, Debug)]
struct State {
    map_mode: u32,
    window_origin: (f64, f64),
    window_extent: (f64, f64),
    viewport_origin: (f64, f64),
    viewport_extent: (f64, f64),
    world: Transform,
    pen: Option<(Rgb, f64, u16)>,
    brush: Option<Fill>,
    font: Option<LogFont>,
    text_colour: Rgb,
    background: Rgb,
    opaque_background: bool,
    nonzero: bool,
    text_align: u32,
    position: (f64, f64),
    clip: Option<Rect>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            map_mode: 1,
            window_origin: (0.0, 0.0),
            window_extent: (1.0, 1.0),
            viewport_origin: (0.0, 0.0),
            viewport_extent: (1.0, 1.0),
            world: Transform::IDENTITY,
            // A device context starts with a black pen, a white brush.
            pen: Some(([0, 0, 0], 0.0, 0)),
            brush: Some(Fill::Solid([255, 255, 255])),
            font: None,
            text_colour: [0, 0, 0],
            background: [255, 255, 255],
            opaque_background: true,
            nonzero: false,
            text_align: 0,
            position: (0.0, 0.0),
            clip: None,
        }
    }
}

struct Player {
    state: State,
    saved: Vec<State>,
    objects: Vec<Option<Object>>,
    ops: Vec<Op>,
    /// The frame on the reference device: left, top, width, height.
    frame: (f64, f64, f64, f64),
    pixels_per_mm: (f64, f64),
    /// The path being built (between `BeginPath` and `EndPath`), and the
    /// last one built.
    building: bool,
    path: Vec<PathElement>,
}

/// Plays an enhanced metafile into a [`Drawing`]; `None` when it does not
/// read as one.
pub fn play(bytes: &[u8]) -> Option<Drawing> {
    let header = header(bytes)?;
    let natural = size(bytes)?;
    let (ppx, ppy) = header.pixels_per_mm;
    let [left, top, right, bottom] = header.frame;
    let mut frame = (
        f64::from(left) / 100.0 * ppx,
        f64::from(top) / 100.0 * ppy,
        f64::from(right - left) / 100.0 * ppx,
        f64::from(bottom - top) / 100.0 * ppy,
    );
    if !(frame.2 > 0.0 && frame.3 > 0.0) {
        let [left, top, right, bottom] = header.bounds;
        frame = (
            f64::from(left),
            f64::from(top),
            f64::from(right) - f64::from(left) + 1.0,
            f64::from(bottom) - f64::from(top) + 1.0,
        );
    }
    if !(frame.2 > 0.0 && frame.3 > 0.0) {
        return None;
    }
    let read = Bytes(bytes);
    let objects = usize::from(read.u16(56)?).min(MAX_OBJECTS);
    let mut player = Player {
        state: State::default(),
        saved: Vec::new(),
        objects: vec![None; objects.min(4_096)],
        ops: Vec::new(),
        frame,
        pixels_per_mm: header.pixels_per_mm,
        building: false,
        path: Vec::new(),
    };
    let mut at = 0usize;
    for _ in 0..MAX_RECORDS {
        let kind = read.u32(at)?;
        let size = read.u32(at + 4)? as usize;
        if size < 8 || size % 4 != 0 {
            break;
        }
        let end = at.checked_add(size)?;
        let Some(record) = bytes.get(at..end) else {
            break;
        };
        if kind == 14 {
            break;
        }
        player.record(kind, &Bytes(record));
        at = end;
        if at >= bytes.len() {
            break;
        }
    }
    Some(Drawing {
        ops: player.ops,
        window: (frame.2.round() as i32, frame.3.round() as i32),
        placeable: Some(natural),
    })
}

impl Player {
    /// The page's mapping onto the device: a point's scale and offset.
    fn device_scale(&self) -> (f64, f64) {
        let state = &self.state;
        let (px, py) = self.pixels_per_mm;
        let fixed = |mm: f64| (mm * px, -mm * py);
        match state.map_mode {
            2 => fixed(0.1),
            3 => fixed(0.01),
            4 => fixed(0.254),
            5 => fixed(0.0254),
            6 => fixed(25.4 / 1_440.0),
            7 | 8 => {
                let (wx, wy) = state.window_extent;
                let (vx, vy) = state.viewport_extent;
                let mut sx = if wx != 0.0 { vx / wx } else { 1.0 };
                let mut sy = if wy != 0.0 { vy / wy } else { 1.0 };
                if state.map_mode == 7 {
                    // Isotropic: the smaller scale both ways.
                    let both = sx.abs().min(sy.abs());
                    sx = both.copysign(sx);
                    sy = both.copysign(sy);
                }
                (sx, sy)
            }
            _ => (1.0, 1.0),
        }
    }

    /// A logical point on the unit square.
    fn point(&self, x: f64, y: f64) -> Point {
        let state = &self.state;
        let (x, y) = state.world.apply(x, y);
        let (sx, sy) = self.device_scale();
        let (wx, wy) = state.window_origin;
        let (vx, vy) = state.viewport_origin;
        let (dx, dy) = ((x - wx) * sx + vx, (y - wy) * sy + vy);
        let (left, top, width, height) = self.frame;
        Point {
            x: ((dx - left) / width) as f32,
            y: ((dy - top) / height) as f32,
        }
    }

    fn rect(&self, left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        let a = self.point(left, top);
        let b = self.point(right, bottom);
        Rect {
            left: a.x,
            top: a.y,
            right: b.x,
            bottom: b.y,
        }
    }

    /// A logical length across, as a share of the frame's width.
    fn across(&self, length: f64) -> f32 {
        let world = &self.state.world;
        let scale = world.m11.hypot(world.m12) * self.device_scale().0.abs();
        (length.abs() * scale / self.frame.2) as f32
    }

    /// A logical length down, as a share of the frame's height.
    fn down(&self, length: f64) -> f32 {
        let world = &self.state.world;
        let scale = world.m21.hypot(world.m22) * self.device_scale().1.abs();
        (length.abs() * scale / self.frame.3) as f32
    }

    fn colour(value: u32) -> Rgb {
        [value as u8, (value >> 8) as u8, (value >> 16) as u8]
    }

    fn stroke(&self) -> Option<Stroke> {
        let (colour, width, style) = self.state.pen?;
        Some(Stroke {
            colour,
            width: self.across(width),
            style,
        })
    }

    fn fill(&self) -> Option<Fill> {
        match &self.state.brush {
            Some(Fill::Hatch { style, colour, .. }) => Some(Fill::Hatch {
                style: *style,
                colour: *colour,
                background: self
                    .state
                    .opaque_background
                    .then_some(self.state.background),
            }),
            other => other.clone(),
        }
    }

    fn add_object(&mut self, index: u32, object: Object) {
        let index = index as usize;
        if index >= MAX_OBJECTS {
            return;
        }
        if index >= self.objects.len() {
            self.objects.resize(index + 1, None);
        }
        self.objects[index] = Some(object);
    }

    fn select(&mut self, index: u32) {
        if index & 0x8000_0000 != 0 {
            // The stock objects.
            let grey = |level: u8| Some(Fill::Solid([level; 3]));
            match index & 0x7FFF_FFFF {
                0 => self.state.brush = grey(255),
                1 => self.state.brush = grey(192),
                2 => self.state.brush = grey(128),
                3 => self.state.brush = grey(64),
                4 => self.state.brush = grey(0),
                5 => self.state.brush = None,
                6 => self.state.pen = Some(([255, 255, 255], 0.0, 0)),
                7 => self.state.pen = Some(([0, 0, 0], 0.0, 0)),
                8 => self.state.pen = None,
                10..=14 | 16 | 17 => self.state.font = None,
                _ => {}
            }
            return;
        }
        let Some(Some(object)) = self.objects.get(index as usize) else {
            return;
        };
        match object.clone() {
            Object::Pen(pen) => self.state.pen = pen,
            Object::Brush(brush) => self.state.brush = brush,
            Object::Font(font) => self.state.font = Some(font),
            Object::Other => {}
        }
    }

    /// A shape: into the path being built, or drawn.
    fn shape(&mut self, path: Vec<PathElement>, filled: bool) {
        if self.building {
            self.path.extend(path);
            return;
        }
        self.draw(path, filled, true);
    }

    fn draw(&mut self, path: Vec<PathElement>, filled: bool, stroked: bool) {
        let fill = if filled { self.fill() } else { None };
        let stroke = if stroked { self.stroke() } else { None };
        if fill.is_none() && stroke.is_none() || path.is_empty() {
            return;
        }
        self.ops.push(Op::Path {
            path,
            fill,
            nonzero: self.state.nonzero,
            stroke,
        });
    }

    fn lines(&self, points: &[(f64, f64)], close: bool, from_position: bool) -> Vec<PathElement> {
        let mut path = Vec::with_capacity(points.len() + 2);
        let mut all = Vec::with_capacity(points.len() + 1);
        if from_position {
            all.push(self.state.position);
        }
        all.extend_from_slice(points);
        for (index, &(x, y)) in all.iter().enumerate() {
            let point = self.point(x, y);
            path.push(
                if index == 0 && !(self.building && from_position && !self.path.is_empty()) {
                    PathElement::MoveTo(point)
                } else if index == 0 {
                    continue;
                } else {
                    PathElement::LineTo(point)
                },
            );
        }
        if close && !path.is_empty() {
            path.push(PathElement::Close);
        }
        path
    }

    fn beziers(&self, points: &[(f64, f64)], from_position: bool) -> Vec<PathElement> {
        let mut path = Vec::new();
        let mut rest = points;
        if from_position {
            if !(self.building && !self.path.is_empty()) {
                let (x, y) = self.state.position;
                path.push(PathElement::MoveTo(self.point(x, y)));
            }
        } else if let Some((&(x, y), tail)) = points.split_first() {
            path.push(PathElement::MoveTo(self.point(x, y)));
            rest = tail;
        }
        for triple in rest.chunks_exact(3) {
            let [a, b, c] = [triple[0], triple[1], triple[2]].map(|(x, y)| self.point(x, y));
            path.push(PathElement::CubicTo(a, b, c));
        }
        path
    }

    fn rectangle(&self, bounds: Rect) -> Vec<PathElement> {
        let p = |x: f32, y: f32| Point { x, y };
        vec![
            PathElement::MoveTo(p(bounds.left, bounds.top)),
            PathElement::LineTo(p(bounds.right, bounds.top)),
            PathElement::LineTo(p(bounds.right, bounds.bottom)),
            PathElement::LineTo(p(bounds.left, bounds.bottom)),
            PathElement::Close,
        ]
    }

    fn image(&mut self, bitmap: Option<Vec<u8>>, source: [i32; 4], dest: Rect, rop: u32) {
        let Some(dib) = bitmap else {
            return;
        };
        let Some(raster) = dib::decode_packed(&dib, ColourTable::Rgb) else {
            return;
        };
        let [x, y, width, height] = source;
        // A bitmap stored from the bottom counts its source rows from the
        // bottom row.
        let top_down = Bytes(&dib).i32(8).is_some_and(|height| height < 0);
        let total = raster.height as i32;
        let clamp = |value: i32, max: i32| value.clamp(0, max.max(0)) as u32;
        let (width, height) = (width.abs(), height.abs());
        let source_top = if top_down { y } else { total - y - height };
        let source = [
            clamp(x, raster.width as i32),
            clamp(source_top, total),
            clamp(width, raster.width as i32 - x.max(0)),
            clamp(height, total - source_top.max(0)),
        ];
        if source[2] == 0 || source[3] == 0 {
            return;
        }
        self.ops.push(Op::Image {
            raster: Arc::new(raster),
            source,
            dest,
            rop: raster_op(rop),
        });
    }

    /// A record's bitmap (its info at `bmi`, its bits at `bits`, offsets and
    /// sizes in the record) as a packed bitmap.
    fn bitmap(p: &Bytes<'_>, bmi: usize, bits: usize) -> Option<Vec<u8>> {
        let (bmi_at, bmi_size) = (p.u32(bmi)? as usize, p.u32(bmi + 4)? as usize);
        let (bits_at, bits_size) = (p.u32(bits)? as usize, p.u32(bits + 4)? as usize);
        if bmi_size == 0 {
            return None;
        }
        let info = p.0.get(bmi_at..bmi_at.checked_add(bmi_size)?)?;
        let data = p.0.get(bits_at..bits_at.checked_add(bits_size)?)?;
        let mut packed = Vec::with_capacity(info.len() + data.len());
        packed.extend_from_slice(info);
        packed.extend_from_slice(data);
        Some(packed)
    }

    fn text(&mut self, p: &Bytes<'_>, wide: bool) -> Option<()> {
        // The bounds, the graphics mode, the scales, then the text: its
        // reference point, count, string's offset, options, rectangle, and
        // the advances' offset.
        let (x, y) = (f64::from(p.i32(36)?), f64::from(p.i32(40)?));
        let count = (p.u32(44)? as usize).min(65_536);
        let string_at = p.u32(48)? as usize;
        let options = p.u32(52)?;
        let rect = [p.i32(56)?, p.i32(60)?, p.i32(64)?, p.i32(68)?].map(f64::from);
        let advances_at = p.u32(72)? as usize;
        if options & 0x10 != 0 || count == 0 {
            // Glyph indices: no characters to draw.
            return None;
        }
        let text: String = if wide {
            let units: Vec<u16> = (0..count)
                .map(|index| p.u16(string_at + 2 * index))
                .collect::<Option<_>>()?;
            char::decode_utf16(units)
                .map(|unit| unit.unwrap_or('\u{FFFD}'))
                .collect()
        } else {
            p.0.get(string_at..string_at + count)?
                .iter()
                .map(|&byte| cp1252(byte))
                .collect()
        };
        let pairs = options & 0x2000 != 0;
        let advances = (advances_at > 0)
            .then(|| {
                (0..count)
                    .map(|index| {
                        let at = advances_at + 4 * if pairs { 2 * index } else { index };
                        p.i32(at).map(|dx| self.across(f64::from(dx)))
                    })
                    .collect::<Option<Vec<f32>>>()
            })
            .flatten();
        let font = self.state.font.clone().unwrap_or(LogFont {
            face: "Arial".into(),
            height: -12,
            escapement: 0,
            weight: 400,
            italic: false,
            underline: false,
            strike_out: false,
        });
        let has_rect = rect[2] > rect[0] && rect[3] > rect[1];
        let bounds = has_rect.then(|| self.rect(rect[0], rect[1], rect[2], rect[3]));
        let origin = if self.state.text_align & 1 != 0 {
            // TA_UPDATECP: at the current position.
            let (x, y) = self.state.position;
            self.point(x, y)
        } else {
            self.point(x, y)
        };
        self.ops.push(Op::Text {
            origin,
            text,
            font: Font {
                face: font.face,
                height: self.down(f64::from(font.height)),
                cell_height: font.height > 0,
                escapement: font.escapement.clamp(-3_600, 3_600) as i16,
                weight: font.weight.clamp(0, 1_000) as i16,
                italic: font.italic,
                underline: font.underline,
                strike_out: font.strike_out,
            },
            colour: self.state.text_colour,
            background: self
                .state
                .opaque_background
                .then_some(self.state.background),
            align: (self.state.text_align & 0xFFFE) as u16,
            advances,
            spread: None,
            clip: bounds.filter(|_| options & 0x4 != 0),
            opaque: bounds.filter(|_| options & 0x2 != 0),
        });
        Some(())
    }

    fn set_clip(&mut self, rect: Option<Rect>, intersecting: bool) {
        let clip = match (rect, self.state.clip) {
            (Some(rect), Some(clip)) if intersecting => Some(intersect(clip, rect)),
            (rect, _) => rect,
        };
        self.state.clip = clip;
        self.ops.push(Op::Clip(clip));
    }

    fn record(&mut self, kind: u32, p: &Bytes<'_>) {
        let i = |at: usize| p.i32(at).map(f64::from);
        let u = |at: usize| p.u32(at);
        // Points after a count at `count`: 16-bit or 32-bit pairs from `at`.
        let points = |at: usize, count: usize, short: bool| -> Option<Vec<(f64, f64)>> {
            (0..count.min(1 << 20))
                .map(|index| {
                    if short {
                        Some((
                            f64::from(p.i16(at + 4 * index)?),
                            f64::from(p.i16(at + 4 * index + 2)?),
                        ))
                    } else {
                        Some((
                            f64::from(p.i32(at + 8 * index)?),
                            f64::from(p.i32(at + 8 * index + 4)?),
                        ))
                    }
                })
                .collect()
        };
        let _ = (|| -> Option<()> {
            match kind {
                // SetWindowExtEx, SetWindowOrgEx, SetViewportExtEx,
                // SetViewportOrgEx.
                9 => {
                    let extent = (i(8)?, i(12)?);
                    if extent.0 != 0.0 && extent.1 != 0.0 {
                        self.state.window_extent = extent;
                    }
                }
                10 => self.state.window_origin = (i(8)?, i(12)?),
                11 => {
                    let extent = (i(8)?, i(12)?);
                    if extent.0 != 0.0 && extent.1 != 0.0 {
                        self.state.viewport_extent = extent;
                    }
                }
                12 => self.state.viewport_origin = (i(8)?, i(12)?),
                // SetMapMode, SetBkMode, SetPolyFillMode, SetTextAlign,
                // SetTextColor, SetBkColor.
                17 => self.state.map_mode = u(8)?,
                18 => self.state.opaque_background = u(8)? == 2,
                19 => self.state.nonzero = u(8)? == 2,
                22 => self.state.text_align = u(8)?,
                24 => self.state.text_colour = Self::colour(u(8)?),
                25 => self.state.background = Self::colour(u(8)?),
                // SaveDC, RestoreDC (relative, negative).
                33 => self.saved.push(self.state.clone()),
                34 => {
                    let back = p.i32(8)?;
                    let depth = if back < 0 {
                        self.saved.len().checked_sub(back.unsigned_abs() as usize)?
                    } else {
                        (back as usize).checked_sub(1)?
                    };
                    if depth < self.saved.len() {
                        self.state = self.saved[depth].clone();
                        self.saved.truncate(depth);
                        self.ops.push(Op::Clip(self.state.clip));
                    }
                }
                // SetWorldTransform, ModifyWorldTransform.
                35 => self.state.world = Transform::read(p, 8)?,
                36 => {
                    let transform = Transform::read(p, 8)?;
                    self.state.world = match u(32)? {
                        1 => Transform::IDENTITY,
                        2 => transform.then(&self.state.world),
                        3 => self.state.world.then(&transform),
                        4 => transform,
                        _ => return None,
                    };
                }
                // SelectObject, DeleteObject.
                37 => self.select(u(8)?),
                40 => {
                    if let Some(slot) = self.objects.get_mut(u(8)? as usize) {
                        *slot = None;
                    }
                }
                // CreatePen: index, style, width (x, y), colour.
                38 => {
                    let style = (u(12)? & 0xF) as u16;
                    let pen = (style != 5).then(|| {
                        (
                            Self::colour(u(24).unwrap_or(0)),
                            i(16).unwrap_or(0.0),
                            style,
                        )
                    });
                    self.add_object(u(8)?, Object::Pen(pen));
                }
                // ExtCreatePen: index, the bitmap's offsets, style, width,
                // brush style, colour.
                95 => {
                    let style = u(28)?;
                    let geometric = style & 0x0001_0000 != 0;
                    let width = if geometric { f64::from(u(32)?) } else { 0.0 };
                    let null = style & 0xF == 5 || u(36)? == 1;
                    let pen = (!null).then(|| {
                        (
                            Self::colour(u(40).unwrap_or(0)),
                            width,
                            (style & 0xF) as u16,
                        )
                    });
                    self.add_object(u(8)?, Object::Pen(pen));
                }
                // CreateBrushIndirect: index, style, colour, hatch.
                39 => {
                    let colour = Self::colour(u(16)?);
                    let brush = match u(12)? {
                        1 => None,
                        2 => Some(Fill::Hatch {
                            style: u(20)?.min(5) as u16,
                            colour,
                            background: None,
                        }),
                        _ => Some(Fill::Solid(colour)),
                    };
                    self.add_object(u(8)?, Object::Brush(brush));
                }
                // CreateDIBPatternBrushPt, CreateMonoBrush: index, usage,
                // the bitmap's offsets and sizes.
                93 | 94 => {
                    let brush = Self::bitmap(p, 16, 24)
                        .and_then(|dib| dib::decode_packed(&dib, ColourTable::Rgb))
                        .map(|raster| Fill::Pattern(Arc::new(raster)));
                    self.add_object(u(8)?, Object::Brush(brush.or(Some(Fill::Solid([0; 3])))));
                }
                // ExtCreateFontIndirectW: index, LOGFONTW.
                82 => {
                    let units: Vec<u16> = (0..32)
                        .map_while(|index| p.u16(40 + 2 * index))
                        .take_while(|&unit| unit != 0)
                        .collect();
                    self.add_object(
                        u(8)?,
                        Object::Font(LogFont {
                            face: String::from_utf16_lossy(&units),
                            height: p.i32(12)?,
                            escapement: p.i32(20)?,
                            weight: p.i32(28)?,
                            italic: p.u8(32)? != 0,
                            underline: p.u8(33)? != 0,
                            strike_out: p.u8(34)? != 0,
                        }),
                    );
                }
                // CreatePalette and other objects take a slot.
                49 => self.add_object(u(8)?, Object::Other),
                // MoveToEx, LineTo.
                27 => {
                    let (x, y) = (i(8)?, i(12)?);
                    self.state.position = (x, y);
                    if self.building {
                        let point = self.point(x, y);
                        self.path.push(PathElement::MoveTo(point));
                    }
                }
                54 => {
                    let (x, y) = (i(8)?, i(12)?);
                    let path = self.lines(&[(x, y)], false, true);
                    self.shape(path, false);
                    self.state.position = (x, y);
                }
                // Rectangle, Ellipse: the box.
                42 | 43 => {
                    let bounds = self.rect(i(8)?, i(12)?, i(16)?, i(20)?);
                    let path = if kind == 43 {
                        self.rectangle(bounds)
                    } else {
                        ellipse_path(bounds)
                    };
                    self.shape(path, true);
                }
                // RoundRect: the box, the corner's width and height.
                44 => {
                    let bounds = self.rect(i(8)?, i(12)?, i(16)?, i(20)?);
                    let corner = (self.across(i(24)?), self.down(i(28)?));
                    let (left, right) =
                        (bounds.left.min(bounds.right), bounds.left.max(bounds.right));
                    let (top, bottom) =
                        (bounds.top.min(bounds.bottom), bounds.top.max(bounds.bottom));
                    let (cw, ch) = (corner.0.min(right - left), corner.1.min(bottom - top));
                    let corner_box = |x: f32, y: f32| Rect {
                        left: x,
                        top: y,
                        right: x + cw,
                        bottom: y + ch,
                    };
                    let mut path = Vec::new();
                    arc(corner_box(right - cw, top), 0.0, PI / 2.0, &mut path, true);
                    arc(corner_box(left, top), PI / 2.0, PI, &mut path, false);
                    arc(
                        corner_box(left, bottom - ch),
                        PI,
                        1.5 * PI,
                        &mut path,
                        false,
                    );
                    arc(
                        corner_box(right - cw, bottom - ch),
                        1.5 * PI,
                        2.0 * PI,
                        &mut path,
                        false,
                    );
                    path.push(PathElement::Close);
                    self.shape(path, true);
                }
                // Arc, Chord, Pie, ArcTo: the box, the start, the end.
                45..=47 | 55 => {
                    let bounds = self.rect(i(8)?, i(12)?, i(16)?, i(20)?);
                    let start = sweep_angle(bounds, self.point(i(24)?, i(28)?));
                    let end = sweep_angle(bounds, self.point(i(32)?, i(36)?));
                    let mut path = Vec::new();
                    if kind == 55 {
                        let (x, y) = self.state.position;
                        path.push(PathElement::MoveTo(self.point(x, y)));
                    }
                    arc(bounds, start, end, &mut path, kind != 55);
                    if kind == 47 {
                        path.push(PathElement::LineTo(Point {
                            x: (bounds.left + bounds.right) / 2.0,
                            y: (bounds.top + bounds.bottom) / 2.0,
                        }));
                    }
                    let filled = matches!(kind, 46 | 47);
                    if filled {
                        path.push(PathElement::Close);
                    }
                    if kind == 55 {
                        self.state.position = (i(32)?, i(36)?);
                    }
                    self.shape(path, filled);
                }
                // The poly records: bounds, a count, the points (32-bit,
                // and the 16-bit ones from 85).
                2..=6 | 85..=89 => {
                    let short = kind >= 85;
                    let base = if short { kind - 83 } else { kind };
                    let count = u(24)? as usize;
                    let points = points(28, count, short)?;
                    let (path, filled) = match base {
                        2 => (self.beziers(&points, false), false),
                        3 => (self.lines(&points, true, false), true),
                        4 => (self.lines(&points, false, false), false),
                        5 => (self.beziers(&points, true), false),
                        _ => (self.lines(&points, false, true), false),
                    };
                    if let Some(&last) = points.last() {
                        if matches!(base, 5 | 6) {
                            self.state.position = last;
                        }
                    }
                    self.shape(path, filled);
                }
                // PolyPolyline, PolyPolygon (and their 16-bit forms): bounds,
                // the count of polygons, of points, each one's count, the
                // points.
                7 | 8 | 90 | 91 => {
                    let short = kind >= 90;
                    let polygons = (u(24)? as usize).min(1 << 16);
                    let counts: Vec<usize> = (0..polygons)
                        .map(|index| u(32 + 4 * index).map(|count| count as usize))
                        .collect::<Option<_>>()?;
                    let mut at = 32 + 4 * polygons;
                    let filled = matches!(kind, 8 | 91);
                    let mut path = Vec::new();
                    for count in counts {
                        let points = points(at, count, short)?;
                        path.extend(self.lines(&points, filled, false));
                        at += count * if short { 4 } else { 8 };
                    }
                    self.shape(path, filled);
                }
                // BeginPath, EndPath, CloseFigure, FillPath,
                // StrokeAndFillPath, StrokePath, AbortPath.
                59 => {
                    self.building = true;
                    self.path.clear();
                }
                60 => self.building = false,
                61 => {
                    if self.building {
                        self.path.push(PathElement::Close);
                    }
                }
                62..=64 => {
                    self.building = false;
                    let path = std::mem::take(&mut self.path);
                    self.draw(path, kind != 64, kind != 62);
                }
                68 => {
                    self.building = false;
                    self.path.clear();
                }
                // SelectClipPath: the path's bounds.
                67 => {
                    let path = std::mem::take(&mut self.path);
                    let mut bounds: Option<Rect> = None;
                    for element in &path {
                        let points: &[Point] = match element {
                            PathElement::MoveTo(point) | PathElement::LineTo(point) => {
                                std::slice::from_ref(point)
                            }
                            PathElement::CubicTo(a, b, c) => &[*a, *b, *c],
                            PathElement::Close => &[],
                        };
                        for point in points {
                            let rect = bounds.get_or_insert(Rect {
                                left: point.x,
                                top: point.y,
                                right: point.x,
                                bottom: point.y,
                            });
                            rect.left = rect.left.min(point.x);
                            rect.top = rect.top.min(point.y);
                            rect.right = rect.right.max(point.x);
                            rect.bottom = rect.bottom.max(point.y);
                        }
                    }
                    if bounds.is_some() {
                        self.set_clip(bounds, u(8)? == 1);
                    }
                }
                // IntersectClipRect: a logical rectangle.
                30 => {
                    let rect = self.rect(i(8)?, i(12)?, i(16)?, i(20)?);
                    self.set_clip(Some(rect), true);
                }
                // ExtSelectClipRgn: the region's size, the mode, the region
                // (its bounds in device pixels).
                75 => {
                    let size = u(8)?;
                    let mode = u(12)?;
                    if size == 0 {
                        if mode == 5 {
                            self.set_clip(None, false);
                        }
                        return Some(());
                    }
                    let bound = [i(32)?, i(36)?, i(40)?, i(44)?];
                    let (left, top, width, height) = self.frame;
                    let rect = Rect {
                        left: ((bound[0] - left) / width) as f32,
                        top: ((bound[1] - top) / height) as f32,
                        right: ((bound[2] - left) / width) as f32,
                        bottom: ((bound[3] - top) / height) as f32,
                    };
                    self.set_clip(Some(rect), mode == 1);
                }
                // BitBlt: bounds, destination x, y, width, height, the
                // raster operation, the source's x, y, transform, colour,
                // usage, and its bitmap's offsets (none: the brush fills).
                76 | 77 => {
                    let (x, y, width, height) = (i(24)?, i(28)?, i(32)?, i(36)?);
                    let rop = u(40)?;
                    let dest = self.rect(x, y, x + width, y + height);
                    let bitmap = Self::bitmap(p, 84, 92);
                    match bitmap {
                        Some(bitmap) => {
                            let (sx, sy) = (p.i32(44)?, p.i32(48)?);
                            let (sw, sh) = if kind == 77 {
                                (p.i32(100)?, p.i32(104)?)
                            } else {
                                (width as i32, height as i32)
                            };
                            self.image(Some(bitmap), [sx, sy, sw, sh], dest, rop);
                        }
                        None => {
                            let rop = raster_op(rop);
                            let fill = match rop {
                                Rop::Black => Some(Fill::Solid([0, 0, 0])),
                                Rop::White => Some(Fill::Solid([255, 255, 255])),
                                _ => self.fill(),
                            };
                            if let Some(fill) = fill {
                                self.ops.push(Op::FillRect {
                                    rect: dest,
                                    fill,
                                    rop,
                                });
                            }
                        }
                    }
                }
                // SetDIBitsToDevice: bounds, destination x, y, the source's
                // x, y, width, height, the bitmap's offsets.
                80 => {
                    let (x, y) = (i(24)?, i(28)?);
                    let source = [p.i32(32)?, p.i32(36)?, p.i32(40)?, p.i32(44)?];
                    let dest = self.rect(x, y, x + f64::from(source[2]), y + f64::from(source[3]));
                    self.image(Self::bitmap(p, 48, 56), source, dest, 0x00CC_0020);
                }
                // StretchDIBits: bounds, destination x, y, the source's x,
                // y, width, height, the bitmap's offsets, usage, the raster
                // operation, the destination's width and height.
                81 => {
                    let (x, y) = (i(24)?, i(28)?);
                    let source = [p.i32(32)?, p.i32(36)?, p.i32(40)?, p.i32(44)?];
                    let (width, height) = (i(72)?, i(76)?);
                    let dest = self.rect(x, y, x + width, y + height);
                    self.image(Self::bitmap(p, 48, 56), source, dest, u(68)?);
                }
                // ExtTextOutA, ExtTextOutW.
                83 => self.text(p, false)?,
                84 => self.text(p, true)?,
                _ => {}
            }
            Some(())
        })();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An enhanced metafile of `records` (type, parameters as 32-bit
    /// values), with its header: a 1,000 × 500 frame (10 × 5 mm) on a
    /// device of 100 pixels a millimetre.
    fn metafile(records: &[(u32, Vec<u32>)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (kind, values) in records {
            body.extend_from_slice(&kind.to_le_bytes());
            body.extend_from_slice(&((8 + 4 * values.len()) as u32).to_le_bytes());
            for value in values {
                body.extend_from_slice(&value.to_le_bytes());
            }
        }
        // EOF: palette entries, offset, size.
        for value in [14u32, 20, 0, 16, 20] {
            body.extend_from_slice(&value.to_le_bytes());
        }
        let mut header = Vec::new();
        let values: [u32; 22] = [
            1,
            88, // type, size
            0,
            0,
            999,
            499, // bounds
            0,
            0,
            1_000,
            500, // frame
            SIGNATURE,
            0x10000,
            (88 + body.len()) as u32,
            records.len() as u32 + 2,
            8, // handles (and the reserved half)
            0,
            0,
            0,
            100_000, // device pixels
            50_000,
            1_000, // millimetres
            500,
        ];
        for value in values {
            header.extend_from_slice(&value.to_le_bytes());
        }
        header.extend(body);
        header
    }

    fn signed(values: &[i32]) -> Vec<u32> {
        values.iter().map(|&value| value as u32).collect()
    }

    #[test]
    fn the_frame_maps_onto_the_unit_square() {
        let file = metafile(&[
            // A red brush and a null pen, selected; a rectangle over the
            // right half (device pixels, MM_TEXT).
            (39, vec![1, 0, 0x0000_00ff, 0]),
            (38, vec![2, 5, 0, 0, 0]),
            (37, vec![1]),
            (37, vec![2]),
            (43, signed(&[500, 0, 1_000, 500])),
        ]);
        assert_eq!(
            size(&file),
            Some(Twips {
                width: 567,
                height: 283
            })
        );
        let drawing = play(&file).unwrap();
        assert_eq!(drawing.window, (1_000, 500));
        let Op::Path {
            path, fill, stroke, ..
        } = &drawing.ops[0]
        else {
            panic!("{:?}", drawing.ops);
        };
        assert_eq!(*fill, Some(Fill::Solid([255, 0, 0])));
        assert_eq!(*stroke, None);
        assert_eq!(path[0], PathElement::MoveTo(Point { x: 0.5, y: 0.0 }));
        assert_eq!(path[2], PathElement::LineTo(Point { x: 1.0, y: 1.0 }));
    }

    #[test]
    fn mapping_modes_transforms_paths_and_text() {
        // MM_ANISOTROPIC: a 100 × 50 window on the whole frame, a world
        // transform doubling x; a path filled; a line of text.
        let mut text = signed(&[0, 0, 0, 0, 1]);
        text.extend([1.0f32.to_bits(), 1.0f32.to_bits()]);
        // EMRTEXT: reference (10, 20), 2 characters at 76 (after the
        // record's 18 values), no options, no rectangle, no advances.
        text.extend(signed(&[10, 20, 2, 76, 0, 0, 0, 0, 0, 0]));
        text.push(u32::from_le_bytes([b'H', 0, b'i', 0]));
        let file = metafile(&[
            (17, vec![8]),
            (9, vec![100, 50]),
            (11, vec![1_000, 500]),
            (
                35,
                [2.0f32, 0.0, 0.0, 1.0, 0.0, 0.0]
                    .iter()
                    .map(|value| value.to_bits())
                    .collect(),
            ),
            (59, vec![]),
            (27, vec![0, 0]),
            (54, vec![25, 50]),
            (54, vec![0, 50]),
            (61, vec![]),
            (60, vec![]),
            (62, signed(&[0, 0, 0, 0])),
            (84, text),
        ]);
        let drawing = play(&file).unwrap();
        let Op::Path { path, fill, .. } = &drawing.ops[0] else {
            panic!("{:?}", drawing.ops);
        };
        assert!(fill.is_some());
        assert_eq!(path[1], PathElement::LineTo(Point { x: 0.5, y: 1.0 }));
        assert_eq!(path.last(), Some(&PathElement::Close));
        let Op::Text { text, origin, .. } = &drawing.ops[1] else {
            panic!("{:?}", drawing.ops);
        };
        assert_eq!(text, "Hi");
        assert_eq!(*origin, Point { x: 0.2, y: 0.4 });
    }

    #[test]
    fn damage_is_refused_without_a_panic() {
        assert!(play(b"not a metafile").is_none());
        let file = metafile(&[
            (43, signed(&[0, 0, 10, 10])),
            (86, vec![0, 0, 0, 0, 3, 0, 0]),
        ]);
        for cut in 0..file.len() {
            let _ = play(&file[..cut]);
        }
        assert!(play(&file).is_some());
    }
}
