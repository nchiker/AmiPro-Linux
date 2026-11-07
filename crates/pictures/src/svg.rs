//! A display list ([`wmf::Drawing`]) written as SVG (`UI_SPEC.md` §33,
//! DW6): Draw mode's Save Drawing As… and Copy, and exports of drawings.
//! The SVG's user units are twips (`viewBox` the drawing's size) and its
//! size is in points, so it prints at the drawing's own size. Fills,
//! Windows' hatches, and pixel patterns are SVG patterns of 8 screen pixels
//! (120 twips); one-pixel lines stay one pixel at any size; bitmaps are
//! embedded as PNG.

use std::fmt::Write as _;

use crate::Raster;
use crate::wmf::{Drawing, Fill, Op, PathElement, Point, Rect, Stroke};

/// A screen pixel at 96 dpi, in twips: the patterns' unit.
const PIXEL: f64 = 15.0;

/// `drawing` as an SVG document `width` × `height` twips.
pub fn svg(drawing: &Drawing, width: i32, height: i32) -> String {
    let (w, h) = (f64::from(width.max(1)), f64::from(height.max(1)));
    let mut writer = Writer {
        w,
        h,
        defs: String::new(),
        body: String::new(),
        patterns: 0,
        clipped: false,
    };
    for op in &drawing.ops {
        writer.op(op);
    }
    if writer.clipped {
        writer.body.push_str("</g>\n");
    }
    let mut out = String::new();
    let _ = write!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" \
         width=\"{}pt\" height=\"{}pt\" viewBox=\"0 0 {} {}\">\n",
        w / 20.0,
        h / 20.0,
        w,
        h
    );
    if !writer.defs.is_empty() {
        let _ = write!(out, "<defs>\n{}</defs>\n", writer.defs);
    }
    out.push_str(&writer.body);
    out.push_str("</svg>\n");
    out
}

struct Writer {
    w: f64,
    h: f64,
    defs: String,
    body: String,
    patterns: usize,
    clipped: bool,
}

fn hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c if (c as u32) < 0x20 && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

/// A number, short.
fn n(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == rounded.trunc() {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

impl Writer {
    fn x(&self, point: Point) -> String {
        n(f64::from(point.x) * self.w)
    }

    fn y(&self, point: Point) -> String {
        n(f64::from(point.y) * self.h)
    }

    fn at(&self, point: Point) -> String {
        format!("{} {}", self.x(point), self.y(point))
    }

    /// A pattern of `rows` (8 by 8 screen pixels) in `colour` on
    /// `background`; returns its paint.
    fn pattern(
        &mut self,
        cells: &dyn Fn(usize, usize) -> Option<[u8; 3]>,
        background: Option<[u8; 3]>,
    ) -> String {
        self.patterns += 1;
        let id = format!("p{}", self.patterns);
        let size = 8.0 * PIXEL;
        let _ = writeln!(
            self.defs,
            "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{}\" height=\"{}\">",
            n(size),
            n(size)
        );
        if let Some(background) = background {
            let _ = writeln!(
                self.defs,
                "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                n(size),
                n(size),
                hex(background)
            );
        }
        for y in 0..8 {
            for x in 0..8 {
                if let Some(colour) = cells(x, y) {
                    let _ = writeln!(
                        self.defs,
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                        n(x as f64 * PIXEL),
                        n(y as f64 * PIXEL),
                        n(PIXEL),
                        n(PIXEL),
                        hex(colour)
                    );
                }
            }
        }
        self.defs.push_str("</pattern>\n");
        format!("url(#{id})")
    }

    fn paint(&mut self, fill: &Fill) -> String {
        match fill {
            Fill::Solid(colour) => hex(*colour),
            Fill::Hatch {
                style,
                colour,
                background,
            } => {
                let (style, colour) = (*style, *colour);
                self.pattern(
                    &|x, y| {
                        let on = match style {
                            0 => y == 0,
                            1 => x == 0,
                            2 => x == y,
                            3 => x + y == 7,
                            4 => x == 0 || y == 0,
                            _ => x == y || x + y == 7,
                        };
                        on.then_some(colour)
                    },
                    *background,
                )
            }
            Fill::Pattern(raster) => {
                let raster = std::sync::Arc::clone(raster);
                self.pattern(
                    &|x, y| {
                        let (x, y) = (
                            x as u32 % raster.width.max(1),
                            y as u32 % raster.height.max(1),
                        );
                        let [r, g, b, a] = raster.pixel(x, y);
                        (a > 0).then_some([r, g, b])
                    },
                    None,
                )
            }
        }
    }

    fn stroke(stroke: &Stroke, w: f64) -> String {
        let dash = match stroke.style {
            1 => Some("6 2"),
            2 => Some("1 1"),
            3 => Some("3 2 1 2"),
            4 => Some("3 1 1 1 1 1"),
            _ => None,
        };
        let width = f64::from(stroke.width) * w;
        let mut out = format!(" stroke=\"{}\"", hex(stroke.colour));
        if width <= 0.0 || dash.is_some() {
            // A pixel wide at any size, as GDI draws a styled pen.
            out.push_str(" stroke-width=\"1\" vector-effect=\"non-scaling-stroke\"");
        } else {
            let _ = write!(
                out,
                " stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
                n(width)
            );
        }
        if let Some(dash) = dash {
            let _ = write!(out, " stroke-dasharray=\"{dash}\"");
        }
        out
    }

    fn op(&mut self, op: &Op) {
        match op {
            Op::Path {
                path,
                fill,
                nonzero,
                stroke,
            } => {
                let mut d = String::new();
                for element in path {
                    match element {
                        PathElement::MoveTo(p) => {
                            let _ = write!(d, "M{} ", self.at(*p));
                        }
                        PathElement::LineTo(p) => {
                            let _ = write!(d, "L{} ", self.at(*p));
                        }
                        PathElement::CubicTo(a, b, c) => {
                            let _ = write!(d, "C{} {} {} ", self.at(*a), self.at(*b), self.at(*c));
                        }
                        PathElement::Close => d.push_str("Z "),
                    }
                }
                let fill = match fill {
                    Some(fill) => self.paint(fill),
                    None => "none".into(),
                };
                let stroke = stroke
                    .as_ref()
                    .map_or_else(String::new, |stroke| Self::stroke(stroke, self.w));
                let rule = if *nonzero {
                    ""
                } else {
                    " fill-rule=\"evenodd\""
                };
                let _ = writeln!(
                    self.body,
                    "<path d=\"{}\" fill=\"{fill}\"{rule}{stroke}/>",
                    d.trim_end()
                );
            }
            Op::FillRect { rect, fill, .. } => {
                let paint = self.paint(fill);
                self.rect(*rect, &paint);
            }
            Op::Image {
                raster,
                source,
                dest,
                ..
            } => {
                let png = encode_png(&crop(raster, *source));
                let (left, right) = (
                    f64::from(dest.left) * self.w,
                    f64::from(dest.right) * self.w,
                );
                let (top, bottom) = (
                    f64::from(dest.top) * self.h,
                    f64::from(dest.bottom) * self.h,
                );
                let _ = writeln!(
                    self.body,
                    "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" \
                     href=\"data:image/png;base64,{}\"/>",
                    n(left.min(right)),
                    n(top.min(bottom)),
                    n((right - left).abs()),
                    n((bottom - top).abs()),
                    base64(&png)
                );
            }
            Op::Text {
                origin,
                text,
                font,
                colour,
                background: _,
                align,
                advances: _,
                spread,
                clip: _,
                opaque: _,
            } => {
                let size =
                    f64::from(font.height) * self.h * if font.cell_height { 0.84 } else { 1.0 };
                let (x, y) = (self.x(*origin), self.y(*origin));
                let mut attributes = format!(
                    "x=\"{x}\" y=\"{y}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\"",
                    escape(&font.face),
                    n(size),
                    hex(*colour)
                );
                attributes.push_str(match align & 24 {
                    24 => " dominant-baseline=\"alphabetic\"",
                    8 => " dominant-baseline=\"text-after-edge\"",
                    _ => " dominant-baseline=\"text-before-edge\"",
                });
                match align & 6 {
                    2 => attributes.push_str(" text-anchor=\"end\""),
                    6 => attributes.push_str(" text-anchor=\"middle\""),
                    _ => {}
                }
                if font.weight >= 600 {
                    attributes.push_str(" font-weight=\"bold\"");
                }
                if font.italic {
                    attributes.push_str(" font-style=\"italic\"");
                }
                match (font.underline, font.strike_out) {
                    (true, true) => {
                        attributes.push_str(" text-decoration=\"underline line-through\"")
                    }
                    (true, false) => attributes.push_str(" text-decoration=\"underline\""),
                    (false, true) => attributes.push_str(" text-decoration=\"line-through\""),
                    _ => {}
                }
                if let Some(spread) = spread {
                    let _ = write!(
                        attributes,
                        " textLength=\"{}\" lengthAdjust=\"spacing\"",
                        n(f64::from(*spread) * self.w)
                    );
                }
                if font.escapement != 0 {
                    let _ = write!(
                        attributes,
                        " transform=\"rotate({} {x} {y})\"",
                        n(-f64::from(font.escapement) / 10.0)
                    );
                }
                let _ = writeln!(self.body, "<text {attributes}>{}</text>", escape(text));
            }
            Op::Clip(rect) => {
                if self.clipped {
                    self.body.push_str("</g>\n");
                    self.clipped = false;
                }
                if let Some(rect) = rect {
                    self.patterns += 1;
                    let id = format!("c{}", self.patterns);
                    let _ = writeln!(self.defs, "<clipPath id=\"{id}\">");
                    let saved = std::mem::take(&mut self.body);
                    self.rect(*rect, "#000");
                    let shape = std::mem::replace(&mut self.body, saved);
                    self.defs.push_str(&shape);
                    self.defs.push_str("</clipPath>\n");
                    let _ = writeln!(self.body, "<g clip-path=\"url(#{id})\">");
                    self.clipped = true;
                }
            }
        }
    }

    fn rect(&mut self, rect: Rect, paint: &str) {
        let (left, right) = (
            f64::from(rect.left) * self.w,
            f64::from(rect.right) * self.w,
        );
        let (top, bottom) = (
            f64::from(rect.top) * self.h,
            f64::from(rect.bottom) * self.h,
        );
        let _ = writeln!(
            self.body,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{paint}\"/>",
            n(left.min(right)),
            n(top.min(bottom)),
            n((right - left).abs()),
            n((bottom - top).abs())
        );
    }
}

fn crop(raster: &Raster, [x, y, width, height]: [u32; 4]) -> Raster {
    if x == 0 && y == 0 && width == raster.width && height == raster.height {
        return raster.clone();
    }
    let (width, height) = (width.max(1), height.max(1));
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for row in y..y + height {
        for column in x..x + width {
            pixels.extend_from_slice(&raster.pixel(
                column.min(raster.width.saturating_sub(1)),
                row.min(raster.height.saturating_sub(1)),
            ));
        }
    }
    Raster {
        width,
        height,
        pixels,
    }
}

/// Base64, as data URIs take it.
pub fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let value = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(TABLE[(value >> (18 - 6 * index) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A PNG of `raster` (RGBA, stored without compression: the crate has no
/// dependencies).
pub fn encode_png(raster: &Raster) -> Vec<u8> {
    let mut data = Vec::with_capacity((raster.width as usize * 4 + 1) * raster.height as usize);
    for row in raster.pixels.chunks((raster.width as usize * 4).max(1)) {
        data.push(0);
        data.extend_from_slice(row);
    }
    // A zlib stream of stored blocks.
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = data.chunks(65_535).collect();
    for (index, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(index + 1 == blocks.len()));
        let length = block.len() as u16;
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    if blocks.is_empty() {
        zlib.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend_from_slice(&(b << 16 | a).to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], body: &[u8]| {
        png.extend_from_slice(&(body.len() as u32).to_be_bytes());
        let mut checked = kind.to_vec();
        checked.extend_from_slice(body);
        png.extend_from_slice(&checked);
        png.extend_from_slice(&crc32(&checked).to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&raster.width.to_be_bytes());
    header.extend_from_slice(&raster.height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &header);
    chunk(b"IDAT", &zlib);
    chunk(b"IEND", &[]);
    png
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdw::{self, Colour, Fill as SdwFill, Object, Pen, Rect as SdwRect};

    #[test]
    fn a_drawing_writes_as_svg_at_its_size() {
        let drawing = sdw::Drawing {
            objects: vec![
                Object::rectangle(
                    SdwRect::new(0, 0, 1440, 720),
                    Pen::default(),
                    SdwFill::solid(Colour::rgb(255, 0, 0)),
                    false,
                ),
                Object::ellipse(
                    SdwRect::new(100, 100, 900, 600),
                    Pen {
                        style: 2,
                        ..Pen::default()
                    },
                    SdwFill {
                        pattern: 8,
                        ..SdwFill::default()
                    },
                    false,
                ),
            ],
            ..sdw::Drawing::new()
        };
        let size = drawing.natural_size();
        let text = svg(&sdw::paint(&drawing), size.width, size.height);
        assert!(text.contains("viewBox=\"0 0 1450 730\""), "{text}");
        assert!(text.contains("width=\"72.5pt\""));
        assert!(text.contains("fill=\"#ff0000\""));
        assert!(text.contains("<pattern id=\"p1\""));
        assert!(text.contains("vector-effect=\"non-scaling-stroke\""));
        assert!(text.contains(" C"));
    }

    #[test]
    fn png_and_base64_are_well_formed() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        let raster = Raster::filled(3, 2, [255, 0, 0, 255]).unwrap();
        let png = encode_png(&raster);
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }
}
