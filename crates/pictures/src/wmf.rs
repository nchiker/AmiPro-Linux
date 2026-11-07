//! Windows metafiles (Ami Pro's `AmiWMF` driver; the published MS-WMF
//! format), played into a display list.
//!
//! Ami Pro plays a metafile into the picture's rectangle with the
//! metafile's window mapped onto it (`PICTURE_NOTES.md`). Here the window
//! in effect when each record is played maps onto the unit square, (0, 0)
//! its top-left corner and (1, 1) its bottom-right, so the app draws the
//! list at any size and the picture stays sharp at every zoom.
//!
//! The records are those the corpora's 73 distinct metafiles use (counted
//! 2026-10-18): the window, palettes, bitmaps (`StretchDIBits`,
//! `DibStretchBlt`, `DibBitBlt`), pens, brushes, fonts, polygons,
//! polylines, rectangles, ellipses, arcs, `PatBlt`, `ExtTextOut`, and the
//! modes they set; and a few more of the same kinds (rounded rectangles,
//! pies, chords, `MoveTo` and `LineTo`, `TextOut`, saving and restoring the
//! state, clip rectangles). Escapes, comments, and records that change no
//! drawing are skipped.

use std::f32::consts::PI;
use std::sync::Arc;

use crate::dib::{self, ColourTable, Rgb};
use crate::{Bytes, Raster, Twips};

/// A point in the window's unit square.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// A rectangle in the unit square; `left` may exceed `right` (and `top`
/// `bottom`) where the picture is mirrored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathElement {
    MoveTo(Point),
    LineTo(Point),
    CubicTo(Point, Point, Point),
    Close,
}

/// How a path is filled.
#[derive(Clone, Debug, PartialEq)]
pub enum Fill {
    Solid(Rgb),
    /// One of GDI's six hatches (0 horizontal, 1 vertical, 2 forward
    /// diagonal, 3 backward diagonal, 4 cross, 5 diagonal cross), lines of
    /// `colour` eight device pixels apart, on `background` when the
    /// background mode is opaque.
    Hatch {
        style: u16,
        colour: Rgb,
        background: Option<Rgb>,
    },
    /// A pattern repeated from the device's origin.
    Pattern(Arc<Raster>),
}

/// How a path is outlined. `width` is a share of the window's width; 0 is
/// one device pixel, whatever the size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub colour: Rgb,
    pub width: f32,
    /// GDI's pen style: 0 solid, 1 dash, 2 dot, 3 dash dot, 4 dash dot dot,
    /// 6 inside frame (drawn as solid).
    pub style: u16,
}

/// How an image or a fill meets what is drawn already (the raster
/// operations; every metafile in the corpora copies).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rop {
    Copy,
    /// Source OR destination (`SRCPAINT`), drawn as lighten.
    Or,
    /// Source AND destination (`SRCAND`), drawn as darken.
    And,
    /// Source XOR destination (`SRCINVERT`, `PATINVERT`), drawn as
    /// difference.
    Xor,
    /// The inverted source (`NOTSRCCOPY`).
    NotCopy,
    Black,
    White,
    /// The destination inverted (`DSTINVERT`).
    Invert,
    /// The destination left as it is.
    Keep,
}

/// A font as `CreateFontIndirect` describes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    /// The face's name, such as `Arial` or `Helv`.
    pub face: String,
    /// The height as a share of the window's height: the em when the
    /// metafile gave a character height (negative), the cell otherwise.
    pub height: f32,
    pub cell_height: bool,
    /// Tenths of a degree, counterclockwise.
    pub escapement: i16,
    pub weight: i16,
    pub italic: bool,
    pub underline: bool,
    pub strike_out: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// A filled and outlined path, closed when filled.
    Path {
        path: Vec<PathElement>,
        fill: Option<Fill>,
        /// Fill with the nonzero rule (`WINDING`) rather than even-odd.
        nonzero: bool,
        stroke: Option<Stroke>,
    },
    /// Part of a bitmap stretched onto `dest`; `source` is in the raster's
    /// pixels: x, y from the top, width, height.
    Image {
        raster: Arc<Raster>,
        source: [u32; 4],
        dest: Rect,
        rop: Rop,
    },
    /// A rectangle filled by a raster operation (`PatBlt`).
    FillRect { rect: Rect, fill: Fill, rop: Rop },
    /// Text at `origin` (the text alignment's reference point), with the
    /// advances `ExtTextOut` gave, as shares of the window's width.
    Text {
        origin: Point,
        text: String,
        font: Font,
        colour: Rgb,
        /// The background drawn behind the text (opaque background mode).
        background: Option<Rgb>,
        /// `SetTextAlign`'s flags: 0 top left; 2 right, 6 centre; 8
        /// bottom, 24 baseline.
        align: u16,
        advances: Option<Vec<f32>>,
        /// The width, as a share of the window's, the text is spread to:
        /// the space it lacks shared between its characters (Ami Pro's
        /// drawings, `sdw`).
        spread: Option<f32>,
        /// `ETO_CLIPPED`'s rectangle, or `ETO_OPAQUE`'s filled one.
        clip: Option<Rect>,
        opaque: Option<Rect>,
    },
    /// What follows is drawn only inside this rectangle (or everywhere).
    Clip(Option<Rect>),
}

/// A played metafile.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawing {
    pub ops: Vec<Op>,
    /// The first window's extent in logical units, absolute.
    pub window: (i32, i32),
    /// The placeable header's bounds in twips, when the file has one.
    pub placeable: Option<Twips>,
}

const PLACEABLE: u32 = 0x9ac6_cdd7;
/// The most records a metafile may have, as a bound on malformed files.
const MAX_RECORDS: usize = 2_000_000;

/// The size a placeable metafile's header gives it, in twips (its bounds
/// at its units per inch); `None` for a plain metafile, whose size Ami Pro
/// keeps in the driver's state.
pub fn placeable_size(bytes: &[u8]) -> Option<Twips> {
    let read = Bytes(bytes);
    if read.u32(0)? != PLACEABLE {
        return None;
    }
    let (left, top) = (i32::from(read.i16(6)?), i32::from(read.i16(8)?));
    let (right, bottom) = (i32::from(read.i16(10)?), i32::from(read.i16(12)?));
    let inch = i32::from(read.u16(14)?);
    if inch == 0 {
        return None;
    }
    Some(Twips {
        width: ((right - left).abs() as i64 * 1_440 / i64::from(inch)) as i32,
        height: ((bottom - top).abs() as i64 * 1_440 / i64::from(inch)) as i32,
    })
}

#[derive(Clone, Debug)]
enum Object {
    Pen(Option<Stroke>, (i32, i32)),
    Brush(Option<Fill>),
    Font(RawFont),
    Palette(Vec<Rgb>),
    Other,
}

#[derive(Clone, Debug)]
struct RawFont {
    face: String,
    height: i16,
    escapement: i16,
    weight: i16,
    italic: bool,
    underline: bool,
    strike_out: bool,
}

#[derive(Clone, Debug)]
struct State {
    window_origin: (i32, i32),
    window_extent: (i32, i32),
    /// The pen's style and its width in logical units, or no pen.
    pen: Option<(Rgb, i32, u16)>,
    brush: Option<Fill>,
    /// A hatch's background follows the mode when drawn.
    font: Option<RawFont>,
    text_colour: Rgb,
    background: Rgb,
    opaque_background: bool,
    nonzero: bool,
    text_align: u16,
    position: (i32, i32),
    palette: Vec<Rgb>,
    clip: Option<Rect>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            window_origin: (0, 0),
            window_extent: (1, 1),
            // A device context starts with a black pen, a white brush.
            pen: Some(([0, 0, 0], 0, 0)),
            brush: Some(Fill::Solid([255, 255, 255])),
            font: None,
            text_colour: [0, 0, 0],
            background: [255, 255, 255],
            opaque_background: true,
            nonzero: false,
            text_align: 0,
            position: (0, 0),
            palette: Vec::new(),
            clip: None,
        }
    }
}

impl State {
    fn point(&self, x: i32, y: i32) -> Point {
        let (ox, oy) = self.window_origin;
        let (ex, ey) = self.window_extent;
        Point {
            x: (x - ox) as f32 / ex as f32,
            y: (y - oy) as f32 / ey as f32,
        }
    }

    fn rect(&self, left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        let a = self.point(left, top);
        let b = self.point(right, bottom);
        Rect {
            left: a.x,
            top: a.y,
            right: b.x,
            bottom: b.y,
        }
    }

    fn colour(&self, value: u32) -> Rgb {
        // PALETTEINDEX: an entry of the selected palette.
        if value >> 24 == 1 {
            return self
                .palette
                .get((value & 0xffff) as usize)
                .copied()
                .unwrap_or([0, 0, 0]);
        }
        [value as u8, (value >> 8) as u8, (value >> 16) as u8]
    }

    fn stroke(&self) -> Option<Stroke> {
        let (colour, width, style) = self.pen?;
        Some(Stroke {
            colour,
            width: width.abs() as f32 / self.window_extent.0.unsigned_abs().max(1) as f32,
            style,
        })
    }

    fn fill(&self) -> Option<Fill> {
        match &self.brush {
            Some(Fill::Hatch { style, colour, .. }) => Some(Fill::Hatch {
                style: *style,
                colour: *colour,
                background: self.opaque_background.then_some(self.background),
            }),
            other => other.clone(),
        }
    }
}

struct Player {
    state: State,
    saved: Vec<State>,
    objects: Vec<Option<Object>>,
    ops: Vec<Op>,
    window: Option<(i32, i32)>,
}

/// Plays a metafile (with or without its placeable header) into a
/// [`Drawing`]; `None` when it does not read as one.
pub fn play(bytes: &[u8]) -> Option<Drawing> {
    let read = Bytes(bytes);
    let placeable = placeable_size(bytes);
    let start = if placeable.is_some() { 22 } else { 0 };
    let header = Bytes(bytes.get(start..)?);
    // The header: its type (1 memory, 2 disk), its size in words (9), the
    // version, the file's size, the object count, the largest record, 0.
    if !matches!(header.u16(0)?, 1 | 2) || header.u16(2)? != 9 {
        return None;
    }
    let objects = usize::from(header.u16(10)?).min(4_096);
    let mut player = Player {
        state: State::default(),
        saved: Vec::new(),
        objects: vec![None; objects],
        ops: Vec::new(),
        window: None,
    };
    let mut at = start + 18;
    for _ in 0..MAX_RECORDS {
        let size = read.u32(at)? as usize;
        let function = read.u16(at + 4)?;
        if function == 0 || size < 3 {
            break;
        }
        let end = at.checked_add(size.checked_mul(2)?)?;
        let parameters = bytes.get(at + 6..end.min(bytes.len()))?;
        player.record(function, size, &Bytes(parameters));
        at = end;
        if at >= bytes.len() {
            break;
        }
    }
    let window = player.window.unwrap_or((1, 1));
    Some(Drawing {
        ops: player.ops,
        window: (window.0.abs(), window.1.abs()),
        placeable,
    })
}

/// Ellipse quarter arcs as cubic Béziers: the control points' distance.
const KAPPA: f32 = 0.552_284_8;

impl Player {
    fn add_object(&mut self, object: Object) {
        match self.objects.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => *slot = Some(object),
            None => self.objects.push(Some(object)),
        }
    }

    fn select(&mut self, index: usize) {
        let Some(Some(object)) = self.objects.get(index) else {
            return;
        };
        match object.clone() {
            Object::Pen(pen, (width, _)) => {
                self.state.pen = pen.map(|stroke| (stroke.colour, width, stroke.style));
            }
            Object::Brush(brush) => self.state.brush = brush,
            Object::Font(font) => self.state.font = Some(font),
            Object::Palette(entries) => self.state.palette = entries,
            Object::Other => {}
        }
    }

    fn shape(&mut self, path: Vec<PathElement>, filled: bool) {
        let fill = if filled { self.state.fill() } else { None };
        let stroke = self.state.stroke();
        if fill.is_none() && stroke.is_none() {
            return;
        }
        self.ops.push(Op::Path {
            path,
            fill,
            nonzero: self.state.nonzero,
            stroke,
        });
    }

    fn polygon_path(&self, points: &[(i32, i32)], close: bool) -> Vec<PathElement> {
        let mut path = Vec::with_capacity(points.len() + 1);
        for (index, &(x, y)) in points.iter().enumerate() {
            let point = self.state.point(x, y);
            path.push(if index == 0 {
                PathElement::MoveTo(point)
            } else {
                PathElement::LineTo(point)
            });
        }
        if close && !path.is_empty() {
            path.push(PathElement::Close);
        }
        path
    }
}

/// An arc of the ellipse in `bounds` from `start` to `end` degrees in
/// radians, counterclockwise as seen, as Béziers of at most a quarter.
pub(crate) fn arc(bounds: Rect, start: f32, end: f32, path: &mut Vec<PathElement>, first: bool) {
    let (cx, cy) = (
        (bounds.left + bounds.right) / 2.0,
        (bounds.top + bounds.bottom) / 2.0,
    );
    let (rx, ry) = (
        (bounds.right - bounds.left).abs() / 2.0,
        (bounds.bottom - bounds.top).abs() / 2.0,
    );
    let mut sweep = end - start;
    if sweep <= 0.0 {
        sweep += 2.0 * PI;
    }
    let at = |angle: f32| Point {
        x: cx + rx * angle.cos(),
        y: cy - ry * angle.sin(),
    };
    if first {
        path.push(PathElement::MoveTo(at(start)));
    } else {
        path.push(PathElement::LineTo(at(start)));
    }
    let pieces = (sweep / (PI / 2.0)).ceil().max(1.0) as usize;
    let step = sweep / pieces as f32;
    let handle = 4.0 / 3.0 * (step / 4.0).tan();
    let mut angle = start;
    for _ in 0..pieces {
        let next = angle + step;
        let (a, b) = (at(angle), at(next));
        path.push(PathElement::CubicTo(
            Point {
                x: a.x - handle * rx * angle.sin(),
                y: a.y - handle * ry * angle.cos(),
            },
            Point {
                x: b.x + handle * rx * next.sin(),
                y: b.y + handle * ry * next.cos(),
            },
            b,
        ));
        angle = next;
    }
}

/// The angle, as seen, of a point from the centre of `bounds`.
pub(crate) fn sweep_angle(bounds: Rect, point: Point) -> f32 {
    let (cx, cy) = (
        (bounds.left + bounds.right) / 2.0,
        (bounds.top + bounds.bottom) / 2.0,
    );
    let rx = ((bounds.right - bounds.left) / 2.0).abs().max(f32::EPSILON);
    let ry = ((bounds.bottom - bounds.top) / 2.0).abs().max(f32::EPSILON);
    (-(point.y - cy) / ry).atan2((point.x - cx) / rx)
}

/// The ellipse in `bounds` as four Béziers.
pub(crate) fn ellipse_path(bounds: Rect) -> Vec<PathElement> {
    let (cx, cy) = (
        (bounds.left + bounds.right) / 2.0,
        (bounds.top + bounds.bottom) / 2.0,
    );
    let (rx, ry) = (
        (bounds.right - bounds.left) / 2.0,
        (bounds.bottom - bounds.top) / 2.0,
    );
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let p = |x: f32, y: f32| Point { x, y };
    vec![
        PathElement::MoveTo(p(cx + rx, cy)),
        PathElement::CubicTo(p(cx + rx, cy + ky), p(cx + kx, cy + ry), p(cx, cy + ry)),
        PathElement::CubicTo(p(cx - kx, cy + ry), p(cx - rx, cy + ky), p(cx - rx, cy)),
        PathElement::CubicTo(p(cx - rx, cy - ky), p(cx - kx, cy - ry), p(cx, cy - ry)),
        PathElement::CubicTo(p(cx + kx, cy - ry), p(cx + rx, cy - ky), p(cx + rx, cy)),
        PathElement::Close,
    ]
}

impl Player {
    fn image(&mut self, dib: &[u8], source: [i32; 4], dest: [i32; 4], rop: u32) {
        let table = ColourTable::Rgb;
        let Some(raster) = dib::decode_packed(dib, table) else {
            return;
        };
        let [x, y, width, height] = source;
        // The source's y counts from the bitmap's bottom row for a bitmap
        // stored from the bottom, as every corpus bitmap is.
        let top_down = Bytes(dib).i32(8).is_some_and(|height| height < 0);
        let height_total = raster.height as i32;
        let clamp = |value: i32, max: i32| value.clamp(0, max) as u32;
        let width = width.abs();
        let height = height.abs();
        let source_top = if top_down {
            y
        } else {
            height_total - y - height
        };
        let source = [
            clamp(x, raster.width as i32),
            clamp(source_top, height_total),
            clamp(width, raster.width as i32 - x.max(0)),
            clamp(height, height_total - source_top.max(0)),
        ];
        let [dx, dy, dw, dh] = dest;
        self.ops.push(Op::Image {
            raster: Arc::new(raster),
            source,
            dest: self.state.rect(dx, dy, dx + dw, dy + dh),
            rop: raster_op(rop),
        });
    }

    fn text(
        &mut self,
        x: i32,
        y: i32,
        text: &[u8],
        advances: Option<Vec<i32>>,
        rects: [Option<Rect>; 2],
    ) {
        let font = self.state.font.clone().unwrap_or(RawFont {
            face: "System".into(),
            height: 16,
            escapement: 0,
            weight: 700,
            italic: false,
            underline: false,
            strike_out: false,
        });
        let window = self.state.window_extent;
        let text: String = text.iter().map(|&byte| cp1252(byte)).collect();
        let width = window.0.unsigned_abs().max(1) as f32;
        self.ops.push(Op::Text {
            origin: self.state.point(x, y),
            text,
            font: Font {
                face: font.face,
                height: f32::from(font.height).abs() / window.1.unsigned_abs().max(1) as f32,
                cell_height: font.height > 0,
                escapement: font.escapement,
                weight: font.weight,
                italic: font.italic,
                underline: font.underline,
                strike_out: font.strike_out,
            },
            colour: self.state.text_colour,
            background: self
                .state
                .opaque_background
                .then_some(self.state.background),
            align: self.state.text_align,
            advances: advances
                .map(|advances| advances.iter().map(|&dx| dx as f32 / width).collect()),
            spread: None,
            clip: rects[0],
            opaque: rects[1],
        });
    }

    fn record(&mut self, function: u16, size: usize, p: &Bytes<'_>) {
        let w = |index: usize| p.i16(2 * index).map(i32::from);
        let d = |index: usize| p.u32(2 * index);
        // Points of a polygon or polyline: a count, then x and y.
        let points = |from: usize, count: usize| -> Option<Vec<(i32, i32)>> {
            (0..count)
                .map(|i| Some((w(from + 2 * i)?, w(from + 2 * i + 1)?)))
                .collect()
        };
        let _ = (|| -> Option<()> {
            match function {
                // SetWindowOrg, SetWindowExt (y first).
                0x020B => self.state.window_origin = (w(1)?, w(0)?),
                0x020C => {
                    let extent = (w(1)?, w(0)?);
                    if extent.0 != 0 && extent.1 != 0 {
                        self.state.window_extent = extent;
                        self.window.get_or_insert(extent);
                    }
                }
                0x020F => {
                    let (dx, dy) = (w(1)?, w(0)?);
                    let (x, y) = self.state.window_origin;
                    self.state.window_origin = (x + dx, y + dy);
                }
                // SetBkColor, SetTextColor, SetBkMode, SetPolyFillMode,
                // SetTextAlign.
                0x0201 => self.state.background = self.state.colour(d(0)?),
                0x0209 => self.state.text_colour = self.state.colour(d(0)?),
                0x0102 => self.state.opaque_background = w(0)? == 2,
                0x0106 => self.state.nonzero = w(0)? == 2,
                0x012E => self.state.text_align = w(0)? as u16,
                // SaveDC, RestoreDC.
                0x001E => self.saved.push(self.state.clone()),
                0x0127 => {
                    let back = w(0)?;
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
                // CreatePenIndirect: style, width (x, y), colour.
                0x02FA => {
                    let style = w(0)? as u16 & 0xf;
                    let width = w(1)?;
                    let colour = self.state.colour(d(3)?);
                    let pen = (style != 5).then_some(Stroke {
                        colour,
                        width: 0.0,
                        style,
                    });
                    self.add_object(Object::Pen(pen, (width, 0)));
                }
                // CreateBrushIndirect: style, colour, hatch.
                0x02FC => {
                    let style = w(0)?;
                    let colour = self.state.colour(d(1)?);
                    let brush = match style {
                        1 => None,
                        2 => Some(Fill::Hatch {
                            style: w(3)? as u16,
                            colour,
                            background: None,
                        }),
                        _ => Some(Fill::Solid(colour)),
                    };
                    self.add_object(Object::Brush(brush));
                }
                // DibCreatePatternBrush: style, usage, a packed bitmap.
                0x0142 => {
                    let dib = p.0.get(4..)?;
                    let brush = dib::decode_packed(dib, ColourTable::Rgb)
                        .map(|raster| Fill::Pattern(Arc::new(raster)));
                    self.add_object(Object::Brush(brush));
                }
                // CreatePatternBrush: a 16-bit bitmap, monochrome in the
                // corpora: drawn in the text and background colours.
                0x01F9 => {
                    let (width, height) = (w(1)?.max(0) as u32, w(2)?.max(0) as u32);
                    let stride = w(3)?.max(0) as usize;
                    let bits = p.0.get(32..)?;
                    let mut raster = Raster::filled(width.min(64), height.min(64), [0, 0, 0, 255]);
                    if let Some(raster) = raster.as_mut() {
                        for y in 0..raster.height {
                            for x in 0..raster.width {
                                let byte = bits.get(y as usize * stride + x as usize / 8).copied();
                                let set = byte.is_some_and(|byte| byte >> (7 - x % 8) & 1 == 1);
                                let [r, g, b] = if set {
                                    self.state.background
                                } else {
                                    self.state.text_colour
                                };
                                raster.put(x, y, [r, g, b, 255]);
                            }
                        }
                    }
                    self.add_object(Object::Brush(
                        raster.map(|raster| Fill::Pattern(Arc::new(raster))),
                    ));
                }
                // CreateFontIndirect.
                0x02FB => {
                    let face: Vec<u8> =
                        p.0.get(18..50.min(p.0.len()))
                            .unwrap_or_default()
                            .iter()
                            .copied()
                            .take_while(|&byte| byte != 0)
                            .collect();
                    self.add_object(Object::Font(RawFont {
                        face: face.iter().map(|&byte| cp1252(byte)).collect(),
                        height: w(0)? as i16,
                        escapement: w(2)? as i16,
                        weight: w(4)? as i16,
                        italic: p.u8(10)? != 0,
                        underline: p.u8(11)? != 0,
                        strike_out: p.u8(12)? != 0,
                    }));
                }
                // CreatePalette: version, count, entries (r, g, b, flags).
                0x00F7 => {
                    let count = usize::from(p.u16(2)?).min(256);
                    let entries = (0..count)
                        .map(|i| Some([p.u8(4 + 4 * i)?, p.u8(5 + 4 * i)?, p.u8(6 + 4 * i)?]))
                        .collect::<Option<Vec<_>>>()?;
                    self.add_object(Object::Palette(entries));
                }
                // CreateRegion and other objects take a slot.
                0x06FF | 0x01F8 => self.add_object(Object::Other),
                0x012D | 0x0234 => self.select(w(0)? as usize),
                0x01F0 => {
                    if let Some(slot) = self.objects.get_mut(w(0)? as usize) {
                        *slot = None;
                    }
                }
                // MoveTo, LineTo.
                0x0214 => self.state.position = (w(1)?, w(0)?),
                0x0213 => {
                    let (x, y) = (w(1)?, w(0)?);
                    let path = self.polygon_path(&[self.state.position, (x, y)], false);
                    self.shape(path, false);
                    self.state.position = (x, y);
                }
                // Polygon, Polyline.
                0x0324 | 0x0325 => {
                    let count = usize::from(p.u16(0)?);
                    let points = points(1, count)?;
                    let filled = function == 0x0324;
                    let path = self.polygon_path(&points, filled);
                    self.shape(path, filled);
                }
                // PolyPolygon: the count of polygons, each one's count, the
                // points.
                0x0538 => {
                    let polygons = usize::from(p.u16(0)?);
                    let counts: Vec<usize> = (0..polygons)
                        .map(|i| p.u16(2 + 2 * i).map(usize::from))
                        .collect::<Option<_>>()?;
                    let mut from = 1 + polygons;
                    let mut path = Vec::new();
                    for count in counts {
                        path.extend(self.polygon_path(&points(from, count)?, true));
                        from += 2 * count;
                    }
                    self.shape(path, true);
                }
                // Rectangle, Ellipse (bottom, right, top, left).
                0x041B | 0x0418 => {
                    let bounds = self.state.rect(w(3)?, w(2)?, w(1)?, w(0)?);
                    let path = if function == 0x041B {
                        vec![
                            PathElement::MoveTo(Point {
                                x: bounds.left,
                                y: bounds.top,
                            }),
                            PathElement::LineTo(Point {
                                x: bounds.right,
                                y: bounds.top,
                            }),
                            PathElement::LineTo(Point {
                                x: bounds.right,
                                y: bounds.bottom,
                            }),
                            PathElement::LineTo(Point {
                                x: bounds.left,
                                y: bounds.bottom,
                            }),
                            PathElement::Close,
                        ]
                    } else {
                        ellipse_path(bounds)
                    };
                    self.shape(path, true);
                }
                // RoundRect (height, width of the corners; bottom, right,
                // top, left), drawn with elliptical corners.
                0x061C => {
                    let bounds = self.state.rect(w(5)?, w(4)?, w(3)?, w(2)?);
                    let corner = self.state.rect(0, 0, w(1)?, w(0)?);
                    let (cw, ch) = (
                        (corner.right - corner.left)
                            .abs()
                            .min((bounds.right - bounds.left).abs()),
                        (corner.bottom - corner.top)
                            .abs()
                            .min((bounds.bottom - bounds.top).abs()),
                    );
                    let (left, right) =
                        (bounds.left.min(bounds.right), bounds.left.max(bounds.right));
                    let (top, bottom) =
                        (bounds.top.min(bounds.bottom), bounds.top.max(bounds.bottom));
                    let mut path = Vec::new();
                    let corner_box = |x: f32, y: f32| Rect {
                        left: x,
                        top: y,
                        right: x + cw,
                        bottom: y + ch,
                    };
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
                // Arc, Pie, Chord (end y, x; start y, x; bottom, right,
                // top, left).
                0x0817 | 0x081A | 0x0830 => {
                    let bounds = self.state.rect(w(7)?, w(6)?, w(5)?, w(4)?);
                    let start = sweep_angle(bounds, self.state.point(w(3)?, w(2)?));
                    let end = sweep_angle(bounds, self.state.point(w(1)?, w(0)?));
                    let mut path = Vec::new();
                    arc(bounds, start, end, &mut path, true);
                    let filled = function != 0x0817;
                    if function == 0x081A {
                        path.push(PathElement::LineTo(Point {
                            x: (bounds.left + bounds.right) / 2.0,
                            y: (bounds.top + bounds.bottom) / 2.0,
                        }));
                    }
                    if filled {
                        path.push(PathElement::Close);
                    }
                    self.shape(path, filled);
                }
                // PatBlt: raster operation, height, width, y, x.
                0x061D => {
                    let (height, width, y, x) = (w(2)?, w(3)?, w(4)?, w(5)?);
                    let rop = raster_op(d(0)?);
                    let fill = match rop {
                        Rop::Black => Fill::Solid([0, 0, 0]),
                        Rop::White => Fill::Solid([255, 255, 255]),
                        _ => self.state.fill().unwrap_or(Fill::Solid([255, 255, 255])),
                    };
                    if self.state.brush.is_some() || matches!(rop, Rop::Black | Rop::White) {
                        self.ops.push(Op::FillRect {
                            rect: self.state.rect(x, y, x + width, y + height),
                            fill,
                            rop,
                        });
                    }
                }
                // StretchDIBits: rop, usage, source height, width, y, x,
                // destination height, width, y, x, the bitmap.
                0x0F43 => {
                    let source = [w(6)?, w(5)?, w(4)?, w(3)?];
                    let dest = [w(10)?, w(9)?, w(8)?, w(7)?];
                    self.image(p.0.get(22..)?, source, dest, d(0)?);
                }
                // DibStretchBlt: rop, source height, width, y, x,
                // destination height, width, y, x, the bitmap (none: a
                // pattern fill, with a reserved word).
                0x0B41 => {
                    if size == usize::from(function >> 8) + 3 {
                        return None;
                    }
                    let source = [w(5)?, w(4)?, w(3)?, w(2)?];
                    let dest = [w(9)?, w(8)?, w(7)?, w(6)?];
                    self.image(p.0.get(20..)?, source, dest, d(0)?);
                }
                // DibBitBlt: rop, y, x of the source, height, width, y, x.
                0x0940 => {
                    if size == usize::from(function >> 8) + 3 {
                        return None;
                    }
                    let (height, width) = (w(4)?, w(5)?);
                    let source = [w(3)?, w(2)?, width, height];
                    let dest = [w(7)?, w(6)?, width, height];
                    self.image(p.0.get(16..)?, source, dest, d(0)?);
                }
                // TextOut: count, the string, y, x.
                0x0521 => {
                    let count = usize::from(p.u16(0)?);
                    let text = p.0.get(2..2 + count)?;
                    let at = 2 + count.div_ceil(2) * 2;
                    let (y, x) = (p.i16(at)?, p.i16(at + 2)?);
                    self.text(i32::from(x), i32::from(y), text, None, [None, None]);
                }
                // ExtTextOut: y, x, count, options, the rectangle (when
                // clipped or opaque), the string, the advances.
                0x0A32 => {
                    let (y, x) = (w(0)?, w(1)?);
                    let count = usize::from(p.u16(4)?);
                    let options = p.u16(6)?;
                    let mut at = 8;
                    let mut rect = None;
                    if options & 0x6 != 0 {
                        rect = Some(self.state.rect(w(4)?, w(5)?, w(6)?, w(7)?));
                        at += 8;
                    }
                    let text = p.0.get(at..at + count)?;
                    at += count.div_ceil(2) * 2;
                    let advances = (p.0.len() >= at + 2 * count && count > 0).then(|| {
                        (0..count)
                            .map(|i| i32::from(p.i16(at + 2 * i).unwrap_or(0)))
                            .collect()
                    });
                    let clip = rect.filter(|_| options & 0x4 != 0);
                    let opaque = rect.filter(|_| options & 0x2 != 0);
                    self.text(x, y, text, advances, [clip, opaque]);
                }
                // IntersectClipRect (bottom, right, top, left).
                0x0416 => {
                    let rect = self.state.rect(w(3)?, w(2)?, w(1)?, w(0)?);
                    let clip = match self.state.clip {
                        Some(clip) => intersect(clip, rect),
                        None => rect,
                    };
                    self.state.clip = Some(clip);
                    self.ops.push(Op::Clip(Some(clip)));
                }
                // SelectClipRegion with no region: everywhere again.
                0x012C if w(0)? == 0 => {
                    self.state.clip = None;
                    self.ops.push(Op::Clip(None));
                }
                _ => {}
            }
            Some(())
        })();
    }
}

pub(crate) fn intersect(a: Rect, b: Rect) -> Rect {
    let order = |a: f32, b: f32| (a.min(b), a.max(b));
    let (al, ar) = order(a.left, a.right);
    let (at, ab) = order(a.top, a.bottom);
    let (bl, br) = order(b.left, b.right);
    let (bt, bb) = order(b.top, b.bottom);
    Rect {
        left: al.max(bl),
        top: at.max(bt),
        right: ar.min(br).max(al.max(bl)),
        bottom: ab.min(bb).max(at.max(bt)),
    }
}

pub(crate) fn raster_op(value: u32) -> Rop {
    match value {
        0x00CC_0020 | 0x00F0_0021 => Rop::Copy,
        0x00EE_0086 | 0x00FB_0A09 => Rop::Or,
        0x0088_00C6 | 0x00A0_00C9 => Rop::And,
        0x0066_0046 | 0x005A_0049 => Rop::Xor,
        0x0033_0008 => Rop::NotCopy,
        0x0000_0042 => Rop::Black,
        0x00FF_0062 => Rop::White,
        0x0055_0009 => Rop::Invert,
        0x00AA_0029 => Rop::Keep,
        _ => Rop::Copy,
    }
}

/// A Windows-1252 byte as a character.
pub(crate) fn cp1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    match byte {
        0x80..=0x9f => HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A metafile of `records` (function, parameter words), with its
    /// header and end record.
    fn metafile(records: &[(u16, Vec<u16>)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (function, words) in records {
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

    fn words(values: &[i32]) -> Vec<u16> {
        values.iter().map(|&value| value as i16 as u16).collect()
    }

    fn colour(value: u32) -> Vec<u16> {
        vec![value as u16, (value >> 16) as u16]
    }

    #[test]
    fn shapes_map_the_window_onto_the_unit_square() {
        let drawing = play(&metafile(&[
            (0x020B, words(&[100, 200])),
            (0x020C, words(&[400, 800])),
            // A red brush, a null pen, selected.
            (0x02FC, [vec![0], colour(0x0000ff), vec![0]].concat()),
            (0x02FA, words(&[5, 0, 0, 0, 0])),
            (0x012D, vec![0]),
            (0x012D, vec![1]),
            (0x041B, words(&[500, 1000, 100, 200])),
        ]))
        .unwrap();
        assert_eq!(drawing.window, (800, 400));
        let Op::Path {
            path, fill, stroke, ..
        } = &drawing.ops[0]
        else {
            panic!("{:?}", drawing.ops);
        };
        assert_eq!(*fill, Some(Fill::Solid([255, 0, 0])));
        assert_eq!(*stroke, None);
        assert_eq!(path[0], PathElement::MoveTo(Point { x: 0.0, y: 0.0 }));
        assert_eq!(path[2], PathElement::LineTo(Point { x: 1.0, y: 1.0 }));
    }

    #[test]
    fn bitmaps_text_and_pattern_fills_play() {
        // A 1 × 1 24-bit bitmap stretched over the right half.
        let mut dib = Vec::new();
        for value in [40u32, 1, 1] {
            dib.extend_from_slice(&value.to_le_bytes());
        }
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&24u16.to_le_bytes());
        dib.extend_from_slice(&[0; 24]);
        dib.extend_from_slice(&[0, 0, 255, 0]);
        let dib_words: Vec<u16> = dib
            .chunks(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let mut stretch = colour(0x00CC_0020);
        stretch.extend(words(&[0, 1, 1, 0, 0, 10, 5, 0, 5]));
        stretch.extend(dib_words);
        let text = b"Hi\x80";
        let mut ext = words(&[2, 1, 3, 0]);
        ext.extend([u16::from_le_bytes([text[0], text[1]]), u16::from(text[2])]);
        let drawing = play(&metafile(&[
            (0x020C, words(&[10, 10])),
            (0x0F43, stretch),
            (0x0A32, ext),
            (0x061D, [colour(0x00F0_0021), words(&[2, 2, 0, 0])].concat()),
        ]))
        .unwrap();
        let Op::Image {
            raster,
            dest,
            source,
            ..
        } = &drawing.ops[0]
        else {
            panic!("{:?}", drawing.ops);
        };
        assert_eq!(raster.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(*source, [0, 0, 1, 1]);
        assert_eq!(dest.left, 0.5);
        assert_eq!(dest.bottom, 1.0);
        let Op::Text { text, origin, .. } = &drawing.ops[1] else {
            panic!("{:?}", drawing.ops);
        };
        assert_eq!(text, "Hi€");
        assert_eq!(*origin, Point { x: 0.1, y: 0.2 });
        assert!(matches!(
            drawing.ops[2],
            Op::FillRect { rop: Rop::Copy, .. }
        ));
    }

    #[test]
    fn placeable_headers_give_the_size_and_damage_is_refused() {
        let mut file = Vec::new();
        file.extend_from_slice(&PLACEABLE.to_le_bytes());
        for word in [0u16, 0, 0, 2_000, 1_000, 1_000] {
            file.extend_from_slice(&word.to_le_bytes());
        }
        file.extend_from_slice(&[0; 6]);
        file.extend(metafile(&[(0x020C, words(&[1_000, 2_000]))]));
        assert_eq!(
            placeable_size(&file),
            Some(Twips {
                width: 2_880,
                height: 1_440
            })
        );
        assert_eq!(play(&file).unwrap().placeable, placeable_size(&file));
        assert!(play(b"not a metafile").is_none());
        for cut in 0..file.len() {
            let _ = play(&file[..cut]);
        }
        // An arc, a pie, an ellipse, and a polygon play without a pen.
        let drawing = play(&metafile(&[
            (0x020C, words(&[100, 100])),
            (0x0817, words(&[0, 100, 50, 0, 100, 100, 0, 0])),
            (0x081A, words(&[0, 100, 50, 0, 100, 100, 0, 0])),
            (0x0418, words(&[100, 100, 0, 0])),
            (0x0324, words(&[3, 0, 0, 100, 0, 50, 100])),
        ]))
        .unwrap();
        assert_eq!(drawing.ops.len(), 4);
    }
}
