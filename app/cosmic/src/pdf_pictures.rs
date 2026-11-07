//! Pictures in a PDF as themselves (`docs/Design/UI_SPEC.md` §29, PO2):
//! Windows metafiles as the paths, images, and text they draw, and SVG
//! pictures as their paths, so that both stay sharp and their text stays
//! text. What has no PDF counterpart (raster operations other than
//! copying, hatched and pattern fills, SVG masks, filters, and embedded
//! images) leaves the picture to the screen's drawing at 300 dpi.

use std::sync::Arc;

use amipro_pictures::Raster;
use amipro_pictures::wmf::{Drawing, Fill, Font, Op, PathElement, Point, Rect, Rop, Stroke};
use cosmic::iced::Rectangle;
use cosmic::iced::advanced::graphics::text::font_system;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Style, Weight};
use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Transform};
use krilla::num::NormalizedF32;
use krilla::paint::{self, FillRule, LineCap, LineJoin, StrokeDash};
use krilla::surface::Surface;
use krilla::text::{GlyphId, KrillaGlyph};
use resvg::usvg;

use crate::pdf::PdfResources;

/// The thinnest line a metafile draws (GDI's zero-width pen is one device
/// pixel): a dot at 300 dpi, in twips.
const THINNEST: f32 = 1_440.0 / 300.0;

/// What in `drawing` has no PDF counterpart here, by kind (for the census).
#[cfg(test)]
pub(crate) fn blockers(drawing: &Drawing) -> std::collections::BTreeSet<String> {
    let mut found = std::collections::BTreeSet::new();
    for op in &drawing.ops {
        match op {
            Op::Path {
                fill: Some(Fill::Hatch { .. }),
                ..
            } => found.insert("path hatch".into()),
            Op::Path {
                fill: Some(Fill::Pattern(_)),
                ..
            } => found.insert("path pattern".into()),
            Op::Image { rop, .. } if !matches!(rop, Rop::Copy | Rop::NotCopy | Rop::Keep) => {
                found.insert(format!("image {rop:?}"))
            }
            Op::FillRect { fill, rop, .. }
                if !(matches!(rop, Rop::Black | Rop::White | Rop::Keep)
                    || (*rop == Rop::Copy && matches!(fill, Fill::Solid(_)))) =>
            {
                found.insert(format!(
                    "fill {rop:?} {}",
                    match fill {
                        Fill::Solid(_) => "solid",
                        Fill::Hatch { .. } => "hatch",
                        Fill::Pattern(_) => "pattern",
                    }
                ))
            }
            _ => false,
        };
    }
    found
}

/// Whether every operation of `drawing` has a PDF counterpart.
pub(crate) fn metafile_is_vector(drawing: &Drawing) -> bool {
    drawing.ops.iter().all(|op| match op {
        Op::Path { .. } | Op::Clip(_) => true,
        Op::Image { rop, .. } => matches!(rop, Rop::Copy | Rop::NotCopy | Rop::Keep),
        Op::FillRect { rop, .. } => {
            matches!(rop, Rop::Copy | Rop::Black | Rop::White | Rop::Keep)
        }
        Op::Text { .. } => true,
    })
}

/// A fill as the PDF paints it: a colour, or a hatch's or a pattern
/// brush's tile repeated from the picture's corner, a tile pixel a dot at
/// 300 dpi (on screen a screen pixel).
fn fill_paint(surface: &mut Surface<'_>, fill: &Fill, rule: FillRule) -> Option<paint::Fill> {
    let tile = match fill {
        Fill::Solid(rgb) => return Some(solid(*rgb, rule)),
        Fill::Hatch {
            style,
            colour,
            background,
        } => {
            let pixmap = crate::metafile_render::hatch(*style, *colour, *background)?;
            Raster {
                width: pixmap.width(),
                height: pixmap.height(),
                pixels: crate::metafile_render::rgba_of(&pixmap),
            }
        }
        Fill::Pattern(raster) => (**raster).clone(),
    };
    if tile.width == 0 || tile.height == 0 {
        return None;
    }
    let (width, height) = (tile.width as f32 * THINNEST, tile.height as f32 * THINNEST);
    let mut builder = surface.stream_builder();
    let mut tile_surface = builder.surface();
    tile_surface.draw_image(
        krilla::image::Image::from_rgba8(tile.pixels, tile.width, tile.height),
        krilla::geom::Size::from_wh(width, height)?,
    );
    tile_surface.finish();
    let stream = builder.finish();
    Some(paint::Fill {
        paint: paint::Pattern {
            stream,
            transform: Transform::identity(),
            width,
            height,
        }
        .into(),
        opacity: NormalizedF32::ONE,
        rule,
    })
}

fn colour([r, g, b]: [u8; 3]) -> rgb::Color {
    rgb::Color::new(r, g, b)
}

fn solid(rgb: [u8; 3], rule: FillRule) -> paint::Fill {
    paint::Fill {
        paint: colour(rgb).into(),
        opacity: NormalizedF32::ONE,
        rule,
    }
}

/// Draws a played metafile over `bounds` (twips on the page), as
/// `metafile_render` draws it on screen.
pub(crate) fn draw_metafile(
    surface: &mut Surface<'_>,
    resources: &mut PdfResources,
    drawing: &Drawing,
    bounds: Rectangle,
) {
    let mut painter = Painter {
        bounds,
        clipped: false,
    };
    for op in &drawing.ops {
        painter.op(surface, resources, op);
    }
    if painter.clipped {
        surface.pop();
    }
}

struct Painter {
    bounds: Rectangle,
    /// Whether a clip from `Op::Clip` is pushed.
    clipped: bool,
}

impl Painter {
    fn point(&self, point: Point) -> (f32, f32) {
        (
            self.bounds.x + point.x * self.bounds.width,
            self.bounds.y + point.y * self.bounds.height,
        )
    }

    fn rect(&self, rect: Rect) -> Option<krilla::geom::Rect> {
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
        krilla::geom::Rect::from_ltrb(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
    }

    fn rect_path(&self, rect: Rect) -> Option<Path> {
        let mut builder = PathBuilder::new();
        builder.push_rect(self.rect(rect)?);
        builder.finish()
    }

    fn op(&mut self, surface: &mut Surface<'_>, resources: &mut PdfResources, op: &Op) {
        match op {
            Op::Path {
                path,
                fill,
                nonzero,
                stroke,
            } => self.path(surface, path, fill.as_ref(), *nonzero, stroke.as_ref()),
            Op::Image {
                raster,
                source,
                dest,
                rop,
            } => self.image(surface, raster, *source, *dest, *rop),
            Op::FillRect { rect, fill, rop } => {
                let fill = match rop {
                    Rop::Black => Fill::Solid([0, 0, 0]),
                    Rop::White => Fill::Solid([255, 255, 255]),
                    Rop::Copy => fill.clone(),
                    _ => return,
                };
                // krilla fills black where neither fill nor stroke is set.
                if let Some(path) = self.rect_path(*rect)
                    && let Some(paint) = fill_paint(surface, &fill, FillRule::NonZero)
                {
                    surface.set_stroke(None);
                    surface.set_fill(Some(paint));
                    surface.draw_path(&path);
                    surface.set_fill(None);
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
                if let Some(path) = opaque.and_then(|rect| self.rect_path(rect)) {
                    surface.set_stroke(None);
                    surface.set_fill(Some(solid(
                        background.unwrap_or([255, 255, 255]),
                        FillRule::NonZero,
                    )));
                    surface.draw_path(&path);
                }
                let clip = clip.and_then(|rect| self.rect_path(rect));
                if let Some(clip) = &clip {
                    surface.push_clip_path(clip, &FillRule::NonZero);
                }
                self.text(
                    surface,
                    resources,
                    TextOp {
                        origin: *origin,
                        text,
                        font,
                        colour: *colour,
                        background: background.filter(|_| opaque.is_none()),
                        align: *align,
                        advances: advances.as_deref(),
                        spread: *spread,
                    },
                );
                if clip.is_some() {
                    surface.pop();
                }
            }
            Op::Clip(rect) => {
                if self.clipped {
                    surface.pop();
                    self.clipped = false;
                }
                if let Some(path) = rect.and_then(|rect| self.rect_path(rect)) {
                    surface.push_clip_path(&path, &FillRule::NonZero);
                    self.clipped = true;
                }
            }
        }
    }

    fn path(
        &self,
        surface: &mut Surface<'_>,
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
        let rule = if nonzero {
            FillRule::NonZero
        } else {
            FillRule::EvenOdd
        };
        let paint = fill.and_then(|fill| fill_paint(surface, fill, rule));
        // krilla fills black where neither fill nor stroke is set.
        if paint.is_none() && stroke.is_none() {
            return;
        }
        surface.set_fill(paint);
        surface.set_stroke(stroke.map(|stroke| {
            let width = (stroke.width * self.bounds.width).max(THINNEST);
            // GDI's pen styles, their dashes in device pixels.
            let dash: &[f32] = match stroke.style {
                1 => &[18.0, 6.0],
                2 => &[3.0, 3.0],
                3 => &[9.0, 6.0, 3.0, 6.0],
                4 => &[9.0, 3.0, 3.0, 3.0, 3.0, 3.0],
                _ => &[],
            };
            let wide = width > 1.5 * THINNEST;
            paint::Stroke {
                paint: colour(stroke.colour).into(),
                width,
                line_cap: if wide { LineCap::Round } else { LineCap::Butt },
                line_join: if wide {
                    LineJoin::Round
                } else {
                    LineJoin::Miter
                },
                dash: (!dash.is_empty()).then(|| StrokeDash {
                    array: dash.iter().map(|length| length * THINNEST).collect(),
                    offset: 0.0,
                }),
                ..paint::Stroke::default()
            }
        }));
        surface.draw_path(&path);
        surface.set_fill(None);
        surface.set_stroke(None);
    }

    fn image(
        &self,
        surface: &mut Surface<'_>,
        raster: &Raster,
        source: [u32; 4],
        dest: Rect,
        rop: Rop,
    ) {
        if rop == Rop::Keep {
            return;
        }
        let [x, y, width, height] = source;
        if width == 0 || height == 0 || raster.width == 0 || raster.height == 0 {
            return;
        }
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for row in y..y + height {
            for column in x..x + width {
                let mut pixel =
                    raster.pixel(column.min(raster.width - 1), row.min(raster.height - 1));
                if rop == Rop::NotCopy {
                    pixel = [255 - pixel[0], 255 - pixel[1], 255 - pixel[2], pixel[3]];
                }
                pixels.extend_from_slice(&pixel);
            }
        }
        let (left, top) = self.point(Point {
            x: dest.left,
            y: dest.top,
        });
        let (right, bottom) = self.point(Point {
            x: dest.right,
            y: dest.bottom,
        });
        let (Some(size), image) = (
            krilla::geom::Size::from_wh(
                (right - left).abs().max(0.01),
                (bottom - top).abs().max(0.01),
            ),
            krilla::image::Image::from_rgba8(pixels, width, height),
        ) else {
            return;
        };
        // A mirrored destination flips the image about its box.
        let transform = Transform::from_row(
            if right < left { -1.0 } else { 1.0 },
            0.0,
            0.0,
            if bottom < top { -1.0 } else { 1.0 },
            left,
            top,
        );
        surface.push_transform(&transform);
        surface.draw_image(image, size);
        surface.pop();
    }

    fn text(&self, surface: &mut Surface<'_>, resources: &mut PdfResources, op: TextOp<'_>) {
        let font = op.font;
        // A positive height is the cell's, about a sixth more than the em.
        let height = font.height * self.bounds.height;
        let size = if font.cell_height {
            height * 0.84
        } else {
            height
        }
        .max(1.0);
        let (glyphs, ascent, line_height, natural) = {
            let Ok(mut fonts) = font_system().write() else {
                return;
            };
            let fonts = fonts.raw();
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
            buffer.set_text(op.text, &attrs, Shaping::Advanced, None);
            buffer.shape_until_scroll(fonts, false);
            let Some(run) = buffer.layout_runs().next() else {
                return;
            };
            (
                run.glyphs.to_vec(),
                run.line_y - run.line_top,
                run.line_height,
                run.line_w,
            )
        };
        // Each character's place: the advances `ExtTextOut` gave, else as
        // shaped.
        let starts: Option<Vec<f32>> = op.advances.map(|advances| {
            let mut at = 0.0;
            let mut starts = Vec::with_capacity(advances.len() + 1);
            for advance in advances {
                starts.push(at);
                at += advance * self.bounds.width;
            }
            starts.push(at);
            starts
        });
        // Ami Pro's drawings spread a text to its box's width.
        let characters = op.text.chars().count();
        let extra = match (&starts, op.spread) {
            (None, Some(spread)) if characters > 1 => {
                (spread * self.bounds.width - natural) / (characters - 1) as f32
            }
            _ => 0.0,
        };
        let total = starts
            .as_ref()
            .and_then(|starts| starts.last().copied())
            .unwrap_or(natural + extra * characters.saturating_sub(1) as f32);
        let (mut x, mut y) = self.point(op.origin);
        // Turned text (counter-clockwise tenths of a degree) turns about
        // its origin.
        let turned = font.escapement != 0;
        if turned {
            surface.push_transform(&Transform::from_rotate_at(
                -f32::from(font.escapement) / 10.0,
                x,
                y,
            ));
        }
        match op.align & 6 {
            2 => x -= total,
            6 => x -= total / 2.0,
            _ => {}
        }
        match op.align & 24 {
            24 => y -= ascent,
            8 => y -= line_height,
            _ => {}
        }
        if let Some(rgb) = op.background {
            let mut builder = PathBuilder::new();
            if let Some(rect) = krilla::geom::Rect::from_xywh(x, y, total.max(1.0), line_height) {
                builder.push_rect(rect);
            }
            if let Some(path) = builder.finish() {
                surface.set_stroke(None);
                surface.set_fill(Some(solid(rgb, FillRule::NonZero)));
                surface.draw_path(&path);
            }
        }
        let char_index = |byte: usize| op.text[..byte.min(op.text.len())].chars().count();
        // Runs of one face, each glyph where its place puts it.
        let mut index = 0;
        while index < glyphs.len() {
            let face = glyphs[index].font_id;
            let end = glyphs[index..]
                .iter()
                .position(|glyph| glyph.font_id != face)
                .map_or(glyphs.len(), |offset| index + offset);
            let place = |glyph: &cosmic_text::LayoutGlyph| match &starts {
                Some(starts) => starts
                    .get(char_index(glyph.start))
                    .copied()
                    .unwrap_or(glyph.x),
                None => glyph.x + extra * char_index(glyph.start) as f32,
            };
            let run = &glyphs[index..end];
            if let Some(pdf_font) = resources.font(face) {
                let mut placed: Vec<KrillaGlyph> = run
                    .iter()
                    .map(|glyph| {
                        KrillaGlyph::new(
                            GlyphId::new(u32::from(glyph.glyph_id)),
                            0.0,
                            glyph.x_offset,
                            glyph.y_offset,
                            0.0,
                            glyph.start..glyph.end,
                            None,
                        )
                    })
                    .collect();
                for (offset, glyph) in placed.iter_mut().enumerate() {
                    glyph.x_advance = match run.get(offset + 1) {
                        Some(next) => (place(next) - place(&run[offset])) / size,
                        None => run[offset].w / size,
                    };
                }
                surface.set_stroke(None);
                surface.set_fill(Some(solid(op.colour, FillRule::NonZero)));
                surface.draw_glyphs(
                    krilla::geom::Point::from_xy(x + place(&run[0]), y + ascent),
                    &placed,
                    pdf_font,
                    op.text,
                    size,
                    false,
                );
            }
            index = end;
        }
        if font.underline || font.strike_out {
            let thickness = (size / 14.0).max(THINNEST);
            for (on, at) in [
                (font.underline, y + ascent + thickness * 1.5),
                (font.strike_out, y + ascent * 0.65),
            ] {
                let mut builder = PathBuilder::new();
                if on && let Some(rect) = krilla::geom::Rect::from_xywh(x, at, total, thickness) {
                    builder.push_rect(rect);
                }
                if let Some(path) = builder.finish() {
                    surface.set_stroke(None);
                    surface.set_fill(Some(solid(op.colour, FillRule::NonZero)));
                    surface.draw_path(&path);
                }
            }
        }
        if turned {
            surface.pop();
        }
    }
}

/// One metafile text record's fields.
struct TextOp<'a> {
    origin: Point,
    text: &'a str,
    font: &'a Font,
    colour: [u8; 3],
    background: Option<[u8; 3]>,
    align: u16,
    advances: Option<&'a [f32]>,
    spread: Option<f32>,
}

/// Whether every node of an SVG picture has a PDF counterpart here.
pub(crate) fn svg_is_vector(tree: &usvg::Tree) -> bool {
    fn vector(group: &usvg::Group) -> bool {
        group.clip_path().is_none()
            && group.mask().is_none()
            && group.filters().is_empty()
            && group.blend_mode() == usvg::BlendMode::Normal
            && group.children().iter().all(|node| match node {
                usvg::Node::Group(child) => vector(child),
                usvg::Node::Path(path) => {
                    path.fill()
                        .is_none_or(|fill| matches!(fill.paint(), usvg::Paint::Color(_)))
                        && path
                            .stroke()
                            .is_none_or(|stroke| matches!(stroke.paint(), usvg::Paint::Color(_)))
                }
                usvg::Node::Text(text) => vector(text.flattened()),
                usvg::Node::Image(_) => false,
            })
    }
    vector(tree.root())
}

/// Draws an SVG picture stretched over `bounds`, as `svg_pixmap` draws it.
pub(crate) fn draw_svg(surface: &mut Surface<'_>, tree: &usvg::Tree, bounds: Rectangle) {
    let size = tree.size();
    surface.push_transform(&Transform::from_row(
        bounds.width / size.width().max(1.0),
        0.0,
        0.0,
        bounds.height / size.height().max(1.0),
        bounds.x,
        bounds.y,
    ));
    svg_group(surface, tree.root());
    surface.pop();
}

fn svg_group(surface: &mut Surface<'_>, group: &usvg::Group) {
    let t = group.transform();
    surface.push_transform(&Transform::from_row(t.sx, t.ky, t.kx, t.sy, t.tx, t.ty));
    let opacity = group.opacity().get();
    if opacity < 1.0 {
        surface.push_opacity(NormalizedF32::new(opacity).unwrap_or(NormalizedF32::ONE));
    }
    for node in group.children() {
        match node {
            usvg::Node::Group(child) => svg_group(surface, child),
            usvg::Node::Path(path) => svg_path(surface, path),
            usvg::Node::Text(text) => svg_group(surface, text.flattened()),
            usvg::Node::Image(_) => {}
        }
    }
    if opacity < 1.0 {
        surface.pop();
    }
    surface.pop();
}

fn svg_colour(paint: &usvg::Paint) -> Option<rgb::Color> {
    match paint {
        usvg::Paint::Color(colour) => Some(rgb::Color::new(colour.red, colour.green, colour.blue)),
        _ => None,
    }
}

fn svg_path(surface: &mut Surface<'_>, path: &usvg::Path) {
    if !path.is_visible() {
        return;
    }
    let mut builder = PathBuilder::new();
    for segment in path.data().segments() {
        match segment {
            usvg::tiny_skia_path::PathSegment::MoveTo(point) => builder.move_to(point.x, point.y),
            usvg::tiny_skia_path::PathSegment::LineTo(point) => builder.line_to(point.x, point.y),
            usvg::tiny_skia_path::PathSegment::QuadTo(a, b) => builder.quad_to(a.x, a.y, b.x, b.y),
            usvg::tiny_skia_path::PathSegment::CubicTo(a, b, c) => {
                builder.cubic_to(a.x, a.y, b.x, b.y, c.x, c.y);
            }
            usvg::tiny_skia_path::PathSegment::Close => builder.close(),
        }
    }
    let Some(data) = builder.finish() else {
        return;
    };
    surface.set_fill(path.fill().and_then(|fill| {
        Some(paint::Fill {
            paint: svg_colour(fill.paint())?.into(),
            opacity: NormalizedF32::new(fill.opacity().get()).unwrap_or(NormalizedF32::ONE),
            rule: match fill.rule() {
                usvg::FillRule::NonZero => FillRule::NonZero,
                usvg::FillRule::EvenOdd => FillRule::EvenOdd,
            },
        })
    }));
    surface.set_stroke(path.stroke().and_then(|stroke| {
        Some(paint::Stroke {
            paint: svg_colour(stroke.paint())?.into(),
            width: stroke.width().get(),
            miter_limit: stroke.miterlimit().get(),
            line_cap: match stroke.linecap() {
                usvg::LineCap::Butt => LineCap::Butt,
                usvg::LineCap::Round => LineCap::Round,
                usvg::LineCap::Square => LineCap::Square,
            },
            line_join: match stroke.linejoin() {
                usvg::LineJoin::Round => LineJoin::Round,
                usvg::LineJoin::Bevel => LineJoin::Bevel,
                _ => LineJoin::Miter,
            },
            opacity: NormalizedF32::new(stroke.opacity().get()).unwrap_or(NormalizedF32::ONE),
            dash: stroke.dasharray().map(|array| StrokeDash {
                array: array.to_vec(),
                offset: stroke.dashoffset(),
            }),
        })
    }));
    // krilla fills black where neither fill nor stroke is set.
    if surface.get_fill().is_some() || surface.get_stroke().is_some() {
        surface.draw_path(&data);
    }
    surface.set_fill(None);
    surface.set_stroke(None);
}

/// A picture registered to be drawn as vectors.
#[derive(Clone)]
pub(crate) enum Printed {
    Metafile(Arc<Drawing>),
    Svg(Arc<usvg::Tree>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(fill: Fill) -> Op {
        let point = |x, y| Point { x, y };
        Op::Path {
            path: vec![
                PathElement::MoveTo(point(0.1, 0.1)),
                PathElement::LineTo(point(0.9, 0.1)),
                PathElement::LineTo(point(0.9, 0.9)),
                PathElement::Close,
            ],
            fill: Some(fill),
            nonzero: true,
            stroke: Some(Stroke {
                colour: [0, 0, 0],
                width: 0.0,
                style: 1,
            }),
        }
    }

    #[test]
    fn metafiles_with_raster_operations_are_drawn_as_the_screen_draws_them() {
        // Hatched and pattern fills become tiling patterns; an image that
        // combines with what is under it (XOR) has no PDF counterpart.
        let mut drawing = Drawing {
            ops: vec![
                square(Fill::Solid([255, 0, 0])),
                square(Fill::Hatch {
                    style: 0,
                    colour: [0, 0, 0],
                    background: None,
                }),
            ],
            window: (100, 100),
            placeable: None,
        };
        assert!(metafile_is_vector(&drawing));
        drawing.ops.push(Op::Image {
            raster: Arc::new(Raster::filled(2, 2, [0, 0, 0, 255]).unwrap()),
            source: [0, 0, 2, 2],
            dest: Rect {
                left: 0.0,
                top: 0.0,
                right: 0.5,
                bottom: 0.5,
            },
            rop: Rop::Xor,
        });
        assert!(!metafile_is_vector(&drawing));
    }

    #[test]
    fn a_metafile_becomes_paths_in_the_pdf() {
        let drawing = Drawing {
            ops: vec![square(Fill::Solid([255, 0, 0]))],
            window: (100, 100),
            placeable: None,
        };
        let mut document = krilla::Document::new();
        let mut resources = PdfResources::default();
        let mut page = document.start_page_with(krilla::page::PageSettings::new(
            krilla::geom::Size::from_wh(100.0, 100.0).unwrap(),
        ));
        let mut surface = page.surface();
        draw_metafile(
            &mut surface,
            &mut resources,
            &drawing,
            Rectangle::new(
                cosmic::iced::Point::ORIGIN,
                cosmic::iced::Size::new(100.0, 100.0),
            ),
        );
        surface.finish();
        page.finish();
        let pdf = document.finish().unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        // No image: the square is a path.
        let contains = |needle: &[u8]| pdf.windows(needle.len()).any(|window| window == needle);
        assert!(!contains(b"/Subtype /Image"));
    }

    /// `AMIPRO_CENSUS_DIR=DIR` (and `AMIPRO_STYLES`): how many metafiles
    /// and SVG pictures in a folder's SAM files the PDF draws as vectors
    /// (aggregate counts only).
    #[test]
    #[ignore = "needs AMIPRO_CENSUS_DIR"]
    fn vector_picture_census() {
        use amipro_document::{FrameContent, PictureFormat};
        let Some(directory) = std::env::var_os("AMIPRO_CENSUS_DIR") else {
            eprintln!("AMIPRO_CENSUS_DIR is not set; skipping");
            return;
        };
        let styles: Vec<std::path::PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|styles| std::env::split_paths(&styles).collect())
            .unwrap_or_default();
        let mut files = Vec::new();
        let mut stack = vec![std::path::PathBuf::from(directory)];
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
        let (mut metafiles, mut vector_metafiles, mut svgs, mut vector_svgs) = (0, 0, 0, 0);
        let mut blockers = std::collections::BTreeMap::new();
        for path in files {
            let Ok(import) = amipro_formats::import_sam_path_with_style_directories(&path, &styles)
            else {
                continue;
            };
            let document = import.document;
            let pictures = crate::pictures::Pictures::default();
            for frame in document.frames() {
                let FrameContent::Picture(picture) = &frame.content else {
                    continue;
                };
                let printed = pictures
                    .printed(&document, picture, path.parent())
                    .is_some();
                if !printed
                    && picture.format() == PictureFormat::Metafile
                    && let Some(reasons) = pictures.blockers(&document, picture, path.parent())
                {
                    for reason in reasons {
                        *blockers.entry(reason).or_insert(0usize) += 1;
                    }
                }
                match picture.format() {
                    PictureFormat::Metafile => {
                        metafiles += 1;
                        vector_metafiles += usize::from(printed);
                    }
                    PictureFormat::Svg => {
                        svgs += 1;
                        vector_svgs += usize::from(printed);
                    }
                    _ => {}
                }
            }
        }
        eprintln!(
            "metafiles: {vector_metafiles} of {metafiles} as vectors; SVG: {vector_svgs} of {svgs}"
        );
        eprintln!("what keeps the others raster (pictures): {blockers:?}");
    }
}
