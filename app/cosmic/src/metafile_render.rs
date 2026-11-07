//! Metafiles drawn with tiny-skia (`docs/Design/UI_SPEC.md` §25): the
//! display list `amipro_pictures::wmf::play` makes, at the size the picture
//! is shown, so it stays sharp at every zoom. Text goes through COSMIC Text
//! with the shaper's substitutes for Windows' fonts (`page_layout.rs`).

use amipro_pictures::Raster;
use amipro_pictures::wmf::{Drawing, Fill, Font, Op, PathElement, Point, Rect, Rop, Stroke};
use cosmic::iced::advanced::graphics::text::font_system;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Style, SwashCache, Weight};
use tiny_skia::{
    BlendMode, FillRule, FilterQuality, LineCap, LineJoin, Mask, Paint, PathBuilder, Pattern,
    Pixmap, PixmapPaint, Shader, SpreadMode, Transform,
};

/// Draws `drawing` into a transparent `width` × `height` pixmap of `pixel`
/// pixmap pixels to a screen pixel, so that GDI's one-pixel lines stay one
/// screen pixel wide.
pub(crate) fn render(drawing: &Drawing, width: u32, height: u32, pixel: f32) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(width.max(1), height.max(1))?;
    let mut painter = Painter {
        width: pixmap.width() as f32,
        height: pixmap.height() as f32,
        pixel: pixel.max(1.0),
        mask: None,
        glyphs: SwashCache::new(),
    };
    for op in &drawing.ops {
        painter.op(&mut pixmap, op);
    }
    Some(pixmap)
}

/// A raster as a tiny-skia pixmap (premultiplied).
pub(crate) fn pixmap_of(raster: &Raster) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(raster.width, raster.height)?;
    for (target, source) in pixmap
        .pixels_mut()
        .iter_mut()
        .zip(raster.pixels.as_chunks::<4>().0)
    {
        *target =
            tiny_skia::ColorU8::from_rgba(source[0], source[1], source[2], source[3]).premultiply();
    }
    Some(pixmap)
}

/// `pixmap` turned `degrees` clockwise as seen, into a pixmap the size of
/// its bounding box, the corners transparent (Ami Pro's Rotate, PIRT30,
/// PIRT90: the box is then stretched onto the picture's rectangle).
pub(crate) fn rotated(pixmap: &Pixmap, degrees: f32) -> Option<Pixmap> {
    let (width, height) = (pixmap.width() as f32, pixmap.height() as f32);
    let (sin, cos) = degrees.to_radians().sin_cos();
    let wide = (width * cos.abs() + height * sin.abs()).ceil().max(1.0);
    let high = (width * sin.abs() + height * cos.abs()).ceil().max(1.0);
    let mut turned = Pixmap::new(wide as u32, high as u32)?;
    let transform = Transform::from_translate(-width / 2.0, -height / 2.0)
        .post_rotate(degrees)
        .post_translate(wide / 2.0, high / 2.0);
    turned.draw_pixmap(
        0,
        0,
        pixmap.as_ref(),
        &PixmapPaint {
            opacity: 1.0,
            blend_mode: BlendMode::SourceOver,
            quality: FilterQuality::Bilinear,
        },
        transform,
        None,
    );
    Some(turned)
}

/// A pixmap's pixels as straight RGBA, as iced's images take them.
pub(crate) fn rgba_of(pixmap: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        let colour = pixel.demultiply();
        out.extend_from_slice(&[colour.red(), colour.green(), colour.blue(), colour.alpha()]);
    }
    out
}

/// A line of text's natural width and height in twips, in `face` at
/// `height` twips (the em, or the cell when `cell_height`), as the page
/// shows it: a drawing's text object's box (`UI_SPEC.md` §33).
pub(crate) fn text_extent(
    face: &str,
    height: f32,
    cell_height: bool,
    bold: bool,
    italic: bool,
    text: &str,
) -> (f64, f64) {
    let Ok(mut fonts) = font_system().write() else {
        return (0.0, f64::from(height));
    };
    let fonts = fonts.raw();
    let size = if cell_height { height * 0.84 } else { height }.max(1.0);
    let family = crate::page_layout::substitute(fonts, face);
    let attrs = Attrs::new()
        .family(Family::Name(&family))
        .weight(Weight(if bold { 700 } else { 400 }))
        .style(if italic { Style::Italic } else { Style::Normal });
    let mut buffer = Buffer::new(fonts, Metrics::new(size, size * 1.15));
    buffer.set_size(None, None);
    buffer.set_text(text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(fonts, false);
    let width = buffer.layout_runs().next().map_or(0.0, |run| run.line_w);
    (f64::from(width), f64::from(size * 1.15))
}

struct Painter {
    width: f32,
    height: f32,
    pixel: f32,
    mask: Option<Mask>,
    glyphs: SwashCache,
}

fn blend(rop: Rop) -> Option<BlendMode> {
    match rop {
        Rop::Copy | Rop::NotCopy | Rop::Black | Rop::White => Some(BlendMode::SourceOver),
        Rop::Or => Some(BlendMode::Lighten),
        Rop::And => Some(BlendMode::Darken),
        Rop::Xor | Rop::Invert => Some(BlendMode::Difference),
        Rop::Keep => None,
    }
}

/// GDI's hatches as an 8 × 8 tile.
pub(crate) fn hatch(style: u16, colour: [u8; 3], background: Option<[u8; 3]>) -> Option<Pixmap> {
    let mut tile = Pixmap::new(8, 8)?;
    if let Some([r, g, b]) = background {
        tile.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    }
    let ink = tiny_skia::ColorU8::from_rgba(colour[0], colour[1], colour[2], 255).premultiply();
    for y in 0..8usize {
        for x in 0..8usize {
            let on = match style {
                0 => y == 0,
                1 => x == 0,
                2 => x == y,
                3 => x + y == 7,
                4 => x == 0 || y == 0,
                _ => x == y || x + y == 7,
            };
            if on {
                tile.pixels_mut()[y * 8 + x] = ink;
            }
        }
    }
    Some(tile)
}

impl Painter {
    fn point(&self, point: Point) -> (f32, f32) {
        (point.x * self.width, point.y * self.height)
    }

    fn rect(&self, rect: Rect) -> Option<tiny_skia::Rect> {
        let (a, b) = (
            self.point(Point {
                x: rect.left,
                y: rect.top,
            }),
            self.point(Point {
                x: rect.right,
                y: rect.bottom,
            }),
        );
        tiny_skia::Rect::from_ltrb(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
    }

    /// A paint for `fill`, with the tile kept alive in `tile`.
    fn paint<'a>(&self, fill: &Fill, tile: &'a mut Option<Pixmap>) -> Option<Paint<'a>> {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        match fill {
            Fill::Solid([r, g, b]) => paint.set_color_rgba8(*r, *g, *b, 255),
            Fill::Hatch {
                style,
                colour,
                background,
            } => {
                *tile = hatch(*style, *colour, *background);
            }
            Fill::Pattern(raster) => *tile = pixmap_of(raster),
        }
        if let Some(tile) = tile.as_ref() {
            paint.shader = Pattern::new(
                tile.as_ref(),
                SpreadMode::Repeat,
                FilterQuality::Nearest,
                1.0,
                Transform::identity(),
            );
        }
        if matches!(paint.shader, Shader::SolidColor(_)) || tile.is_some() {
            Some(paint)
        } else {
            None
        }
    }

    fn op(&mut self, pixmap: &mut Pixmap, op: &Op) {
        match op {
            Op::Path {
                path,
                fill,
                nonzero,
                stroke,
            } => self.path(pixmap, path, fill.as_ref(), *nonzero, stroke.as_ref()),
            Op::Image {
                raster,
                source,
                dest,
                rop,
            } => self.image(pixmap, raster, *source, *dest, *rop),
            Op::FillRect { rect, fill, rop } => {
                let (Some(rect), Some(mode)) = (self.rect(*rect), blend(*rop)) else {
                    return;
                };
                let mut tile = None;
                let fill = match rop {
                    Rop::Invert => Fill::Solid([255, 255, 255]),
                    _ => fill.clone(),
                };
                if let Some(mut paint) = self.paint(&fill, &mut tile) {
                    paint.blend_mode = mode;
                    paint.anti_alias = false;
                    pixmap.fill_rect(rect, &paint, Transform::identity(), self.mask.as_ref());
                }
            }
            Op::Text {
                origin,
                text,
                font,
                colour,
                background,
                align,
                advances,
                spread,
                clip,
                opaque,
            } => {
                if let Some(rect) = opaque.and_then(|rect| self.rect(rect)) {
                    let mut paint = Paint::default();
                    let [r, g, b] = background.unwrap_or([255, 255, 255]);
                    paint.set_color_rgba8(r, g, b, 255);
                    pixmap.fill_rect(rect, &paint, Transform::identity(), self.mask.as_ref());
                }
                let clip = clip.and_then(|rect| self.rect(rect));
                self.text(
                    pixmap,
                    *origin,
                    text,
                    font,
                    *colour,
                    background.filter(|_| opaque.is_none()),
                    *align,
                    advances.as_deref(),
                    *spread,
                    clip,
                );
            }
            Op::Clip(rect) => {
                self.mask = rect.and_then(|rect| self.rect(rect)).and_then(|rect| {
                    let mut mask = Mask::new(pixmap.width(), pixmap.height())?;
                    let path = PathBuilder::from_rect(rect);
                    mask.fill_path(&path, FillRule::Winding, false, Transform::identity());
                    Some(mask)
                });
            }
        }
    }

    fn path(
        &self,
        pixmap: &mut Pixmap,
        elements: &[PathElement],
        fill: Option<&Fill>,
        nonzero: bool,
        stroke: Option<&Stroke>,
    ) {
        let mut builder = PathBuilder::new();
        for element in elements {
            match *element {
                PathElement::MoveTo(point) => {
                    let (x, y) = self.point(point);
                    builder.move_to(x, y);
                }
                PathElement::LineTo(point) => {
                    let (x, y) = self.point(point);
                    builder.line_to(x, y);
                }
                PathElement::CubicTo(a, b, c) => {
                    let ((ax, ay), (bx, by), (cx, cy)) =
                        (self.point(a), self.point(b), self.point(c));
                    builder.cubic_to(ax, ay, bx, by, cx, cy);
                }
                PathElement::Close => builder.close(),
            }
        }
        let Some(path) = builder.finish() else {
            return;
        };
        if let Some(fill) = fill {
            let mut tile = None;
            if let Some(paint) = self.paint(fill, &mut tile) {
                let rule = if nonzero {
                    FillRule::Winding
                } else {
                    FillRule::EvenOdd
                };
                pixmap.fill_path(
                    &path,
                    &paint,
                    rule,
                    Transform::identity(),
                    self.mask.as_ref(),
                );
            }
        }
        if let Some(stroke) = stroke {
            let mut paint = Paint {
                anti_alias: true,
                ..Paint::default()
            };
            let [r, g, b] = stroke.colour;
            paint.set_color_rgba8(r, g, b, 255);
            let width = (stroke.width * self.width).max(self.pixel);
            let dash = match stroke.style {
                1 => vec![18.0, 6.0],
                2 => vec![3.0, 3.0],
                3 => vec![9.0, 6.0, 3.0, 6.0],
                4 => vec![9.0, 3.0, 3.0, 3.0, 3.0, 3.0],
                _ => Vec::new(),
            };
            let dash = dash.into_iter().map(|length| length * self.pixel).collect();
            let line = tiny_skia::Stroke {
                width,
                line_cap: if width > 1.5 * self.pixel {
                    LineCap::Round
                } else {
                    LineCap::Butt
                },
                line_join: if width > 1.5 * self.pixel {
                    LineJoin::Round
                } else {
                    LineJoin::Miter
                },
                dash: tiny_skia::StrokeDash::new(dash, 0.0),
                ..tiny_skia::Stroke::default()
            };
            pixmap.stroke_path(
                &path,
                &paint,
                &line,
                Transform::identity(),
                self.mask.as_ref(),
            );
        }
    }

    fn image(&self, pixmap: &mut Pixmap, raster: &Raster, source: [u32; 4], dest: Rect, rop: Rop) {
        let Some(mode) = blend(rop) else {
            return;
        };
        let [x, y, width, height] = source;
        if width == 0 || height == 0 {
            return;
        }
        let mut part = Raster {
            width,
            height,
            pixels: Vec::with_capacity(width as usize * height as usize * 4),
        };
        for row in y..y + height {
            for column in x..x + width {
                let mut pixel =
                    raster.pixel(column.min(raster.width - 1), row.min(raster.height - 1));
                if rop == Rop::NotCopy {
                    pixel = [255 - pixel[0], 255 - pixel[1], 255 - pixel[2], pixel[3]];
                }
                part.pixels.extend_from_slice(&pixel);
            }
        }
        let Some(source) = pixmap_of(&part) else {
            return;
        };
        let (left, top) = self.point(Point {
            x: dest.left,
            y: dest.top,
        });
        let (right, bottom) = self.point(Point {
            x: dest.right,
            y: dest.bottom,
        });
        let transform = Transform::from_row(
            (right - left) / width as f32,
            0.0,
            0.0,
            (bottom - top) / height as f32,
            left,
            top,
        );
        let quality = if (right - left).abs() < width as f32 {
            FilterQuality::Bilinear
        } else {
            FilterQuality::Nearest
        };
        pixmap.draw_pixmap(
            0,
            0,
            source.as_ref(),
            &PixmapPaint {
                opacity: 1.0,
                blend_mode: mode,
                quality,
            },
            transform,
            self.mask.as_ref(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        pixmap: &mut Pixmap,
        origin: Point,
        text: &str,
        font: &Font,
        colour: [u8; 3],
        background: Option<[u8; 3]>,
        align: u16,
        advances: Option<&[f32]>,
        spread: Option<f32>,
        clip: Option<tiny_skia::Rect>,
    ) {
        let mut fonts = font_system().write().unwrap();
        let fonts = fonts.raw();
        // A positive height is the cell's, about a sixth more than the em.
        let height = font.height * self.height;
        let size = if font.cell_height {
            height * 0.84
        } else {
            height
        }
        .max(1.0);
        let family = crate::page_layout::substitute(fonts, &font.face);
        let attrs = Attrs::new()
            .family(Family::Name(&family))
            .weight(Weight(font.weight.clamp(100, 900) as u16))
            .style(if font.italic {
                Style::Italic
            } else {
                Style::Normal
            });
        let mut buffer = Buffer::new(fonts, Metrics::new(size, size * 1.2));
        buffer.set_size(None, None);
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(fonts, false);
        let Some(run) = buffer.layout_runs().next() else {
            return;
        };
        let glyphs: Vec<_> = run.glyphs.to_vec();
        let ascent = run.line_y - run.line_top;
        let line_height = run.line_height;
        let natural: f32 = run.line_w;
        let char_index = |byte: usize| text[..byte.min(text.len())].chars().count();
        // Each character's place: the advances `ExtTextOut` gave, else as
        // shaped, spread to a width where one is given (Ami Pro's drawings).
        let starts: Option<Vec<f32>> = advances.map(|advances| {
            let mut at = 0.0;
            let mut starts = Vec::with_capacity(advances.len());
            for advance in advances {
                starts.push(at);
                at += advance * self.width;
            }
            starts.push(at);
            starts
        });
        let characters = text.chars().count();
        let extra = match (&starts, spread) {
            (None, Some(spread)) if characters > 1 => {
                (spread * self.width - natural) / (characters - 1) as f32
            }
            _ => 0.0,
        };
        let total = starts
            .as_ref()
            .and_then(|starts| starts.last().copied())
            .unwrap_or(natural + extra * characters.saturating_sub(1) as f32);
        let (mut x, mut y) = self.point(origin);
        match align & 6 {
            2 => x -= total,
            6 => x -= total / 2.0,
            _ => {}
        }
        match align & 24 {
            24 => y -= ascent,
            8 => y -= line_height,
            _ => {}
        }
        let offsets: Vec<f32> = glyphs
            .iter()
            .map(|glyph| {
                let index = char_index(glyph.start);
                match &starts {
                    Some(starts) => starts.get(index).copied().unwrap_or(glyph.x) - glyph.x,
                    None => extra * index as f32,
                }
            })
            .collect();
        let line = TextLine {
            colour,
            background,
            ascent,
            line_height,
            total,
            size,
            underline: font.underline,
            strike_out: font.strike_out,
        };
        if font.escapement == 0 {
            let mask = self.mask.take();
            self.draw_line(
                fonts,
                pixmap,
                &glyphs,
                &offsets,
                (x, y),
                &line,
                mask.as_ref(),
                clip,
            );
            self.mask = mask;
            return;
        }
        // Turned text (counter-clockwise tenths of a degree): drawn level
        // into a layer, then turned about its origin onto the picture.
        let pad = size.ceil();
        let Some(mut layer) = Pixmap::new(
            (total.abs() + 2.0 * pad).ceil().max(1.0) as u32,
            (line_height + 2.0 * pad).ceil().max(1.0) as u32,
        ) else {
            return;
        };
        self.draw_line(
            fonts,
            &mut layer,
            &glyphs,
            &offsets,
            (pad, pad),
            &line,
            None,
            None,
        );
        let degrees = -f32::from(font.escapement) / 10.0;
        let transform = Transform::from_translate(x, y)
            .pre_concat(Transform::from_rotate(degrees))
            .pre_translate(-pad, -pad);
        pixmap.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &PixmapPaint {
                quality: FilterQuality::Bilinear,
                ..PixmapPaint::default()
            },
            transform,
            self.mask.as_ref(),
        );
    }

    /// One line of shaped text at `(x, y)`, its top-left corner: the
    /// background, the glyphs moved by `offsets`, the underline and
    /// strikeout.
    #[allow(clippy::too_many_arguments)]
    fn draw_line(
        &mut self,
        fonts: &mut cosmic_text::FontSystem,
        pixmap: &mut Pixmap,
        glyphs: &[cosmic_text::LayoutGlyph],
        offsets: &[f32],
        (x, y): (f32, f32),
        line: &TextLine,
        mask: Option<&Mask>,
        clip: Option<tiny_skia::Rect>,
    ) {
        if let Some([r, g, b]) = line.background
            && let Some(rect) =
                tiny_skia::Rect::from_xywh(x, y, line.total.max(1.0), line.line_height)
        {
            let mut paint = Paint::default();
            paint.set_color_rgba8(r, g, b, 255);
            pixmap.fill_rect(rect, &paint, Transform::identity(), mask);
        }
        let colour = line.colour;
        let ink = cosmic_text::Color::rgb(colour[0], colour[1], colour[2]);
        let (width, height) = (pixmap.width() as i32, pixmap.height() as i32);
        for (glyph, offset) in glyphs.iter().zip(offsets) {
            let physical = glyph.physical((x + offset, y + line.ascent), 1.0);
            let pixels = pixmap.pixels_mut();
            self.glyphs
                .with_pixels(fonts, physical.cache_key, ink, |gx, gy, colour| {
                    let (px, py) = (physical.x + gx, physical.y + gy);
                    if px < 0 || py < 0 || px >= width || py >= height {
                        return;
                    }
                    if let Some(clip) = clip
                        && !(clip.left() <= px as f32
                            && (px as f32) < clip.right()
                            && clip.top() <= py as f32
                            && (py as f32) < clip.bottom())
                    {
                        return;
                    }
                    let index = py as usize * width as usize + px as usize;
                    let coverage = u32::from(colour.a())
                        * mask.map_or(255, |mask| u32::from(mask.data()[index]))
                        / 255;
                    if coverage == 0 {
                        return;
                    }
                    let under = pixels[index].demultiply();
                    let mix = |top: u8, bottom: u8| {
                        ((u32::from(top) * coverage + u32::from(bottom) * (255 - coverage)) / 255)
                            as u8
                    };
                    let alpha =
                        (coverage + u32::from(under.alpha()) * (255 - coverage) / 255) as u8;
                    pixels[index] = tiny_skia::ColorU8::from_rgba(
                        mix(colour.r(), under.red()),
                        mix(colour.g(), under.green()),
                        mix(colour.b(), under.blue()),
                        alpha,
                    )
                    .premultiply();
                });
        }
        if line.underline || line.strike_out {
            let mut paint = Paint::default();
            paint.set_color_rgba8(colour[0], colour[1], colour[2], 255);
            let thickness = (line.size / 14.0).max(1.0);
            for (on, at) in [
                (line.underline, y + line.ascent + thickness * 1.5),
                (line.strike_out, y + line.ascent * 0.65),
            ] {
                if on && let Some(rect) = tiny_skia::Rect::from_xywh(x, at, line.total, thickness) {
                    pixmap.fill_rect(rect, &paint, Transform::identity(), mask);
                }
            }
        }
    }
}

/// What one line of metafile text draws besides its glyphs.
struct TextLine {
    colour: [u8; 3],
    background: Option<[u8; 3]>,
    ascent: f32,
    line_height: f32,
    total: f32,
    size: f32,
    underline: bool,
    strike_out: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A red rectangle over the left half and a blue 1 × 1 bitmap over the
    /// right half, drawn at 40 × 20: each half its colour.
    /// Ami Pro's drawings' text: spread to its box, and turned about its
    /// origin (`sdw`; DRW3).
    #[test]
    fn drawing_text_spreads_to_its_box_and_turns() {
        let text = |escapement, spread| Drawing {
            ops: vec![Op::Text {
                origin: Point {
                    x: 0.1,
                    y: if escapement == 0 { 0.1 } else { 0.9 },
                },
                text: "Wide box".into(),
                font: Font {
                    face: "Arial".into(),
                    height: 0.08,
                    cell_height: false,
                    escapement,
                    weight: 400,
                    italic: false,
                    underline: false,
                    strike_out: false,
                },
                colour: [0, 0, 0],
                background: None,
                align: 0,
                advances: None,
                spread,
                clip: None,
                opaque: None,
            }],
            window: (1_000, 1_000),
            placeable: None,
        };
        // The inked columns and rows of a 200 × 200 drawing.
        let extent = |drawing: &Drawing| {
            let pixmap = render(drawing, 200, 200, 1.0).unwrap();
            let (mut columns, mut rows) = ((u32::MAX, 0), (u32::MAX, 0));
            for y in 0..200 {
                for x in 0..200 {
                    if pixmap.pixel(x, y).unwrap().alpha() > 128 {
                        columns = (columns.0.min(x), columns.1.max(x));
                        rows = (rows.0.min(y), rows.1.max(y));
                    }
                }
            }
            (columns, rows)
        };
        let (natural, _) = extent(&text(0, None));
        let (spread, _) = extent(&text(0, Some(0.8)));
        assert!(natural.1 - natural.0 < 120, "{natural:?}");
        // Spread to 0.8 of 200 pixels from x = 20.
        assert!((170..=180).contains(&spread.1), "{spread:?}");
        // Turned 90° counter-clockwise: the text runs upward from its
        // origin (20, 180), its top to the left: tall and narrow.
        let (columns, rows) = extent(&text(900, None));
        assert!(
            rows.1 - rows.0 > 60 && columns.1 - columns.0 < 25,
            "{columns:?} {rows:?}"
        );
        assert!(rows.1 <= 182 && columns.1 <= 40, "{columns:?} {rows:?}");
    }

    #[test]
    fn metafile_ops_draw_where_the_window_puts_them() {
        let half = |left: f32, right: f32| Rect {
            left,
            top: 0.0,
            right,
            bottom: 1.0,
        };
        let drawing = Drawing {
            ops: vec![
                Op::FillRect {
                    rect: half(0.0, 0.5),
                    fill: Fill::Solid([255, 0, 0]),
                    rop: Rop::Copy,
                },
                Op::Image {
                    raster: Arc::new(Raster::filled(1, 1, [0, 0, 255, 255]).unwrap()),
                    source: [0, 0, 1, 1],
                    dest: half(0.5, 1.0),
                    rop: Rop::Copy,
                },
            ],
            window: (100, 100),
            placeable: None,
        };
        let pixmap = render(&drawing, 40, 20, 1.0).unwrap();
        let rgba = rgba_of(&pixmap);
        let at = |x: usize, y: usize| &rgba[(y * 40 + x) * 4..][..4];
        assert_eq!(at(5, 10), [255, 0, 0, 255]);
        assert_eq!(at(35, 10), [0, 0, 255, 255]);
    }

    /// Every enhanced metafile under `AMIPRO_EMF_DIR` drawn to a PNG in
    /// `AMIPRO_EMF_OUT` at its natural size at 96 dpi (at most 1,200 pixels
    /// a side), for looking at; files numbered, never named.
    #[test]
    #[ignore]
    fn emf_renderings() {
        let (Some(directory), Some(out)) = (
            std::env::var_os("AMIPRO_EMF_DIR"),
            std::env::var_os("AMIPRO_EMF_OUT"),
        ) else {
            eprintln!("AMIPRO_EMF_DIR and AMIPRO_EMF_OUT are not set");
            return;
        };
        let mut files: Vec<_> = std::fs::read_dir(&directory)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("emf"))
            })
            .collect();
        files.sort();
        std::fs::create_dir_all(&out).unwrap();
        let mut drawn = 0;
        for (index, path) in files.iter().enumerate() {
            let bytes = std::fs::read(path).unwrap();
            let Some(drawing) = amipro_pictures::emf::play(&bytes) else {
                continue;
            };
            let size = drawing.placeable.unwrap();
            let (width, height) = (size.width as f32 / 15.0, size.height as f32 / 15.0);
            let scale = (1_200.0 / width.max(height)).min(1.0);
            let Some(mut pixmap) = render(
                &drawing,
                (width * scale).max(1.0) as u32,
                (height * scale).max(1.0) as u32,
                1.0,
            ) else {
                continue;
            };
            // On white, as a page shows it.
            let mut page = Pixmap::new(pixmap.width(), pixmap.height()).unwrap();
            page.fill(tiny_skia::Color::WHITE);
            page.draw_pixmap(
                0,
                0,
                pixmap.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
            pixmap = page;
            pixmap
                .save_png(std::path::Path::new(&out).join(format!("{index:05}.png")))
                .unwrap();
            drawn += 1;
        }
        eprintln!("drawn {drawn} of {}", files.len());
    }
}
