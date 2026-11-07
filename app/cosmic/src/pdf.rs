//! A renderer that writes PDF (`docs/Design/UI_SPEC.md` §29): iced's
//! renderer traits recorded as iced records them (layers of quads, images,
//! and text, each layer clipped to its bounds and drawn after the one
//! before it, its quads before its images and its images before its text)
//! and written into a krilla page. The page surface draws a PDF with the
//! code that draws the screen; text keeps its glyphs, fonts (embedded and
//! subset by krilla), and characters, so it can be selected and searched.

use std::collections::HashMap;
use std::sync::Arc;

use cosmic::iced::advanced::graphics::text::{Editor, Paragraph, Raw, font_system};
use cosmic::iced::advanced::{image, renderer, text};
use cosmic::iced::{
    Background, Color, Font, Pixels, Point, Rectangle, Size, Transformation, alignment,
};
use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Transform};
use krilla::num::NormalizedF32;
use krilla::paint::{Fill, FillRule};
use krilla::surface::Surface;
use krilla::text::{Font as PdfFont, GlyphId, KrillaGlyph};

/// A picture the page surface hands over as itself rather than as the
/// screen's raster (`UI_SPEC.md` §29, PO2): drawn into `bounds` by the
/// writer that knows it.
pub(crate) trait VectorPictures {
    /// Draws the picture behind `handle` into `bounds` (the renderer's
    /// coordinates, the page's transform applied), or returns `false` when
    /// `handle` is an ordinary raster.
    fn draw(
        &self,
        handle: &image::Handle,
        surface: &mut Surface<'_>,
        resources: &mut PdfResources,
        bounds: Rectangle,
    ) -> bool;
}

/// One recorded layer: what iced's tiny-skia and wgpu renderers keep per
/// layer, in the order they draw it.
struct Layer {
    bounds: Rectangle,
    quads: Vec<(renderer::Quad, Background)>,
    images: Vec<(image::Image<image::Handle>, Rectangle, Rectangle)>,
    text: Vec<TextItem>,
}

impl Layer {
    fn new(bounds: Rectangle) -> Self {
        Self {
            bounds,
            quads: Vec::new(),
            images: Vec::new(),
            text: Vec::new(),
        }
    }
}

/// Shaped text to draw: a COSMIC Text buffer's glyphs from `position`, at
/// `scale` (the transformation's), in `colour` where a glyph has none.
struct TextItem {
    paragraph: Paragraph,
    position: Point,
    scale: f32,
    colour: Color,
}

/// Records what a widget draws, as iced's renderers record it, for one
/// PDF page at a time.
pub(crate) struct PdfRenderer {
    layers: Vec<Layer>,
    current: usize,
    previous: Vec<usize>,
    transformations: Vec<Transformation>,
    default_font: Font,
    default_size: Pixels,
}

impl Default for PdfRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfRenderer {
    pub(crate) fn new() -> Self {
        Self {
            layers: vec![Layer::new(Rectangle::INFINITE)],
            current: 0,
            previous: Vec::new(),
            transformations: vec![Transformation::IDENTITY],
            default_font: Font::default(),
            default_size: Pixels(14.0),
        }
    }

    fn transformation(&self) -> Transformation {
        self.transformations
            .last()
            .copied()
            .unwrap_or(Transformation::IDENTITY)
    }

    /// Forgets what was recorded, for the next page.
    pub(crate) fn clear(&mut self) {
        self.layers.clear();
        self.layers.push(Layer::new(Rectangle::INFINITE));
        self.current = 0;
        self.previous.clear();
        self.transformations.clear();
        self.transformations.push(Transformation::IDENTITY);
    }

    /// The recorded quads, layer by layer, for tests.
    #[cfg(test)]
    pub(crate) fn quads(&self) -> Vec<(Rectangle, Background)> {
        self.layers
            .iter()
            .flat_map(|layer| layer.quads.iter().map(|(quad, fill)| (quad.bounds, *fill)))
            .collect()
    }

    /// The recorded text, for tests: each buffer's lines joined.
    #[cfg(test)]
    pub(crate) fn texts(&self) -> Vec<String> {
        self.layers
            .iter()
            .flat_map(|layer| {
                layer.text.iter().map(|item| {
                    item.paragraph
                        .buffer()
                        .layout_runs()
                        .map(|run| run.text)
                        .collect::<Vec<_>>()
                        .join("\n")
                })
            })
            .collect()
    }

    /// The recorded text that would print, for tests: each run's glyphs
    /// that are not hidden (protected text on a preprinted form).
    #[cfg(test)]
    pub(crate) fn printed_texts(&self) -> Vec<String> {
        self.layers
            .iter()
            .flat_map(|layer| layer.text.iter())
            .flat_map(|item| {
                item.paragraph.buffer().layout_runs().map(|run| {
                    let mut text = String::new();
                    for glyph in run.glyphs {
                        let alpha = glyph.color_opt.map_or(item.colour.a, |colour| {
                            f32::from(colour.a()) / 255.0 * item.colour.a
                        });
                        if alpha > 0.0 {
                            text.push_str(&run.text[glyph.start..glyph.end]);
                        }
                    }
                    text
                })
            })
            .collect()
    }

    /// The recorded text for tests: each run's text with the lightest weight
    /// among the faces its glyphs are drawn in.
    #[cfg(test)]
    pub(crate) fn run_weights(&self) -> Vec<(String, u16)> {
        let mut system = font_system().write().unwrap();
        let mut out = Vec::new();
        for item in self.layers.iter().flat_map(|layer| layer.text.iter()) {
            for run in item.paragraph.buffer().layout_runs() {
                let weight = run
                    .glyphs
                    .iter()
                    .filter_map(|glyph| {
                        system
                            .raw()
                            .db()
                            .face(glyph.font_id)
                            .map(|face| face.weight.0)
                    })
                    .min();
                out.push((run.text.to_string(), weight.unwrap_or(0)));
            }
        }
        out
    }

    /// Writes what was recorded into `surface`, `to_page` taking the
    /// renderer's coordinates to the page's points.
    pub(crate) fn write(
        &self,
        surface: &mut Surface<'_>,
        to_page: Transform,
        resources: &mut PdfResources,
        vectors: Option<&dyn VectorPictures>,
    ) {
        surface.push_transform(&to_page);
        for layer in &self.layers {
            let clipped = layer.bounds != Rectangle::INFINITE;
            if clipped {
                let Some(path) = rect_path(layer.bounds) else {
                    continue;
                };
                surface.push_clip_path(&path, &FillRule::NonZero);
            }
            for (quad, background) in &layer.quads {
                write_quad(surface, quad, background);
            }
            for (image, bounds, clip) in &layer.images {
                let drawn = vectors.is_some_and(|vectors| {
                    let Some(path) = rect_path(*clip) else {
                        return true;
                    };
                    surface.push_clip_path(&path, &FillRule::NonZero);
                    let drawn = vectors.draw(&image.handle, surface, resources, *bounds);
                    surface.pop();
                    drawn
                });
                if !drawn {
                    write_image(surface, image, *bounds, *clip, resources);
                }
            }
            for item in &layer.text {
                write_text(surface, item, resources);
            }
            if clipped {
                surface.pop();
            }
        }
        surface.pop();
    }
}

impl renderer::Renderer for PdfRenderer {
    fn start_layer(&mut self, bounds: Rectangle) {
        // As iced's layer stack: the bounds transformed and clipped to the
        // current layer's; each new layer drawn after every earlier one.
        let bounds = bounds * self.transformation();
        let bounds = bounds
            .intersection(&self.layers[self.current].bounds)
            .unwrap_or(Rectangle {
                width: 0.0,
                height: 0.0,
                ..bounds
            });
        self.previous.push(self.current);
        self.layers.push(Layer::new(bounds));
        self.current = self.layers.len() - 1;
    }

    fn end_layer(&mut self) {
        self.current = self.previous.pop().unwrap_or(0);
    }

    fn start_transformation(&mut self, transformation: Transformation) {
        self.transformations
            .push(self.transformation() * transformation);
    }

    fn end_transformation(&mut self) {
        if self.transformations.len() > 1 {
            self.transformations.pop();
        }
    }

    fn fill_quad(&mut self, quad: renderer::Quad, background: impl Into<Background>) {
        let mut quad = quad;
        quad.bounds = quad.bounds * self.transformation();
        self.layers[self.current]
            .quads
            .push((quad, background.into()));
    }

    fn reset(&mut self, _new_bounds: Rectangle) {
        self.clear();
    }

    fn allocate_image(
        &mut self,
        handle: &image::Handle,
        callback: impl FnOnce(Result<image::Allocation, image::Error>) + Send + 'static,
    ) {
        let result = match handle {
            image::Handle::Rgba { width, height, .. } => {
                // SAFETY: as iced's own renderers, which allocate for the
                // handle's size; the allocation only names the handle.
                Ok(unsafe { image::allocate(handle, Size::new(*width, *height)) })
            }
            _ => Err(image::Error::Unsupported),
        };
        callback(result);
    }
}

impl text::Renderer for PdfRenderer {
    type Font = Font;
    type Paragraph = Paragraph;
    type Editor = Editor;
    type Raw = Raw;

    const ICON_FONT: Font = Font::with_name("Iced-Icons");
    const CHECKMARK_ICON: char = '\u{f00c}';
    const ARROW_DOWN_ICON: char = '\u{e800}';
    const ICED_LOGO: char = '\u{e801}';
    const SCROLL_UP_ICON: char = '\u{e802}';
    const SCROLL_DOWN_ICON: char = '\u{e803}';
    const SCROLL_LEFT_ICON: char = '\u{e804}';
    const SCROLL_RIGHT_ICON: char = '\u{e805}';

    fn default_font(&self) -> Font {
        self.default_font
    }

    fn default_size(&self) -> Pixels {
        self.default_size
    }

    fn fill_paragraph(
        &mut self,
        paragraph: &Paragraph,
        position: Point,
        colour: Color,
        _clip_bounds: Rectangle,
    ) {
        let transformation = self.transformation();
        self.layers[self.current].text.push(TextItem {
            paragraph: paragraph.clone(),
            position: position * transformation,
            scale: transformation.scale_factor(),
            colour,
        });
    }

    fn fill_editor(
        &mut self,
        _editor: &Editor,
        _position: Point,
        _colour: Color,
        _clip_bounds: Rectangle,
    ) {
        // Nothing on the page draws text editors.
    }

    fn fill_raw(&mut self, _raw: Raw) {
        // Nothing on the page draws raw buffers.
    }

    fn fill_text(
        &mut self,
        content: text::Text<String, Font>,
        position: Point,
        colour: Color,
        clip_bounds: Rectangle,
    ) {
        use cosmic::iced::advanced::text::Paragraph as _;
        let paragraph = Paragraph::with_text(content.as_ref());
        let size = paragraph.min_bounds();
        // Anchored as iced's renderers anchor cached text.
        let x = match content.align_x {
            text::Alignment::Center => position.x - size.width / 2.0,
            text::Alignment::Right => position.x - size.width,
            _ => position.x,
        };
        let y = match content.align_y {
            alignment::Vertical::Center => position.y - size.height / 2.0,
            alignment::Vertical::Bottom => position.y - size.height,
            alignment::Vertical::Top => position.y,
        };
        self.fill_paragraph(&paragraph, Point::new(x, y), colour, clip_bounds);
    }
}

impl image::Renderer for PdfRenderer {
    type Handle = image::Handle;

    fn load_image(&self, handle: &image::Handle) -> Result<image::Allocation, image::Error> {
        match handle {
            image::Handle::Rgba { width, height, .. } => {
                // SAFETY: as in `allocate_image`.
                Ok(unsafe { image::allocate(handle, Size::new(*width, *height)) })
            }
            _ => Err(image::Error::Unsupported),
        }
    }

    fn measure_image(&self, handle: &image::Handle) -> Option<Size<u32>> {
        match handle {
            image::Handle::Rgba { width, height, .. } => Some(Size::new(*width, *height)),
            _ => None,
        }
    }

    fn draw_image(
        &mut self,
        image: image::Image<image::Handle>,
        bounds: Rectangle,
        clip: Rectangle,
    ) {
        let transformation = self.transformation();
        self.layers[self.current].images.push((
            image,
            bounds * transformation,
            clip * transformation,
        ));
    }
}

/// What a document's pages share: each font and picture embedded once.
#[derive(Default)]
pub(crate) struct PdfResources {
    fonts: HashMap<cosmic_text::fontdb::ID, Option<PdfFont>>,
    images: HashMap<image::Id, Option<krilla::image::Image>>,
}

impl PdfResources {
    /// The PDF font for a COSMIC Text face, read from its file once.
    pub(crate) fn font(&mut self, id: cosmic_text::fontdb::ID) -> Option<PdfFont> {
        self.fonts
            .entry(id)
            .or_insert_with(|| {
                let mut system = font_system().write().ok()?;
                let (data, index) = system
                    .raw()
                    .db()
                    .with_face_data(id, |data, index| (data.to_vec(), index))?;
                PdfFont::new(Arc::new(data).into(), index)
            })
            .clone()
    }

    fn image(&mut self, handle: &image::Handle) -> Option<krilla::image::Image> {
        self.images
            .entry(handle.id())
            .or_insert_with(|| match handle {
                image::Handle::Rgba {
                    width,
                    height,
                    pixels,
                    ..
                } => Some(krilla::image::Image::from_rgba8(
                    pixels.to_vec(),
                    *width,
                    *height,
                )),
                _ => None,
            })
            .clone()
    }
}

fn pdf_colour(colour: Color) -> (rgb::Color, NormalizedF32) {
    let [r, g, b, a] = colour.into_rgba8();
    (
        rgb::Color::new(r, g, b),
        NormalizedF32::new(f32::from(a) / 255.0).unwrap_or(NormalizedF32::ONE),
    )
}

fn fill_with(colour: Color, rule: FillRule) -> Fill {
    let (colour, opacity) = pdf_colour(colour);
    Fill {
        paint: colour.into(),
        opacity,
        rule,
    }
}

pub(crate) fn rect_path(bounds: Rectangle) -> Option<Path> {
    let mut builder = PathBuilder::new();
    builder.push_rect(krilla::geom::Rect::from_xywh(
        bounds.x,
        bounds.y,
        bounds.width.max(0.0),
        bounds.height.max(0.0),
    )?);
    builder.finish()
}

/// A rectangle with each corner rounded by its own radius (top left, top
/// right, bottom right, bottom left), as iced rounds quads.
fn rounded(builder: &mut PathBuilder, bounds: Rectangle, radii: [f32; 4]) {
    let limit = bounds.width.min(bounds.height) / 2.0;
    let [tl, tr, br, bl] = radii.map(|radius| radius.clamp(0.0, limit.max(0.0)));
    let (left, top) = (bounds.x, bounds.y);
    let (right, bottom) = (bounds.x + bounds.width, bounds.y + bounds.height);
    // The quarter circle's control points (4/3 · tan(π/8)).
    const K: f32 = 0.552_284_8;
    builder.move_to(left + tl, top);
    builder.line_to(right - tr, top);
    if tr > 0.0 {
        builder.cubic_to(
            right - tr + tr * K,
            top,
            right,
            top + tr - tr * K,
            right,
            top + tr,
        );
    }
    builder.line_to(right, bottom - br);
    if br > 0.0 {
        builder.cubic_to(
            right,
            bottom - br + br * K,
            right - br + br * K,
            bottom,
            right - br,
            bottom,
        );
    }
    builder.line_to(left + bl, bottom);
    if bl > 0.0 {
        builder.cubic_to(
            left + bl - bl * K,
            bottom,
            left,
            bottom - bl + bl * K,
            left,
            bottom - bl,
        );
    }
    builder.line_to(left, top + tl);
    if tl > 0.0 {
        builder.cubic_to(
            left,
            top + tl - tl * K,
            left + tl - tl * K,
            top,
            left + tl,
            top,
        );
    }
    builder.close();
}

fn write_quad(surface: &mut Surface<'_>, quad: &renderer::Quad, background: &Background) {
    let radius = quad.border.radius;
    let radii = [
        radius.top_left,
        radius.top_right,
        radius.bottom_right,
        radius.bottom_left,
    ];
    let colour = match background {
        Background::Color(colour) => *colour,
        // Nothing on the page fills with gradients; their first stop is
        // their colour here.
        Background::Gradient(cosmic::iced::Gradient::Linear(linear)) => linear
            .stops
            .iter()
            .flatten()
            .next()
            .map_or(Color::TRANSPARENT, |stop| stop.color),
    };
    if colour.a > 0.0 {
        let mut builder = PathBuilder::new();
        rounded(&mut builder, quad.bounds, radii);
        if let Some(path) = builder.finish() {
            surface.set_stroke(None);
            surface.set_fill(Some(fill_with(colour, FillRule::NonZero)));
            surface.draw_path(&path);
        }
    }
    // The border inside the quad, as iced draws it: the ring between the
    // quad and the quad inset by the border's width.
    let width = quad.border.width;
    if width > 0.0 && quad.border.color.a > 0.0 {
        let inner = Rectangle {
            x: quad.bounds.x + width,
            y: quad.bounds.y + width,
            width: quad.bounds.width - 2.0 * width,
            height: quad.bounds.height - 2.0 * width,
        };
        let mut builder = PathBuilder::new();
        rounded(&mut builder, quad.bounds, radii);
        if inner.width > 0.0 && inner.height > 0.0 {
            rounded(
                &mut builder,
                inner,
                radii.map(|radius| (radius - width).max(0.0)),
            );
        }
        if let Some(path) = builder.finish() {
            surface.set_stroke(None);
            surface.set_fill(Some(fill_with(quad.border.color, FillRule::EvenOdd)));
            surface.draw_path(&path);
        }
    }
}

fn write_image(
    surface: &mut Surface<'_>,
    image: &image::Image<image::Handle>,
    bounds: Rectangle,
    clip: Rectangle,
    resources: &mut PdfResources,
) {
    let Some(picture) = resources.image(&image.handle) else {
        return;
    };
    let (Some(size), Some(clip)) = (
        krilla::geom::Size::from_wh(bounds.width.max(0.01), bounds.height.max(0.01)),
        rect_path(clip),
    ) else {
        return;
    };
    surface.push_clip_path(&clip, &FillRule::NonZero);
    surface.push_transform(&Transform::from_translate(bounds.x, bounds.y));
    surface.draw_image(picture, size);
    surface.pop();
    surface.pop();
}

/// A run of glyphs drawn together: one face, size, colour, and slant.
struct GlyphRun {
    font: cosmic_text::fontdb::ID,
    size: f32,
    colour: Color,
    italic: bool,
    start: (f32, f32),
    glyphs: Vec<KrillaGlyph>,
    /// Each glyph's x, to give the one before it its advance.
    xs: Vec<f32>,
}

fn write_text(surface: &mut Surface<'_>, item: &TextItem, resources: &mut PdfResources) {
    let scale = item.scale;
    for run in item.paragraph.buffer().layout_runs() {
        let mut runs: Vec<GlyphRun> = Vec::new();
        for glyph in run.glyphs {
            let colour = glyph.color_opt.map_or(item.colour, |colour| {
                let [r, g, b, a] = colour.as_rgba();
                Color::from_rgba8(r, g, b, f32::from(a) / 255.0 * item.colour.a)
            });
            // Hidden text (protected text on a preprinted form) is not
            // in the PDF at all, as Ami Pro does not print it.
            if colour.a <= 0.0 {
                continue;
            }
            let italic = glyph
                .cache_key_flags
                .contains(cosmic_text::CacheKeyFlags::FAKE_ITALIC);
            let size = glyph.font_size * scale;
            let x = item.position.x + glyph.x * scale;
            let y = item.position.y + (run.line_y + glyph.y) * scale;
            let continues = runs.last().is_some_and(|last| {
                last.font == glyph.font_id
                    && last.size == size
                    && last.colour == colour
                    && last.italic == italic
                    && last.start.1 == y
            });
            if !continues {
                runs.push(GlyphRun {
                    font: glyph.font_id,
                    size,
                    colour,
                    italic,
                    start: (x, y),
                    glyphs: Vec::new(),
                    xs: Vec::new(),
                });
            }
            let current = runs.last_mut().expect("a run was pushed");
            current.glyphs.push(KrillaGlyph::new(
                GlyphId::new(u32::from(glyph.glyph_id)),
                glyph.w * scale / size.max(f32::EPSILON),
                glyph.x_offset,
                glyph.y_offset,
                0.0,
                glyph.start..glyph.end,
                None,
            ));
            current.xs.push(x);
        }
        for mut glyph_run in runs {
            let Some(font) = resources.font(glyph_run.font) else {
                continue;
            };
            // Each glyph where the layout put it: its advance is the step
            // to the next glyph of the run.
            let size = glyph_run.size.max(f32::EPSILON);
            for index in 0..glyph_run.glyphs.len().saturating_sub(1) {
                glyph_run.glyphs[index].x_advance =
                    (glyph_run.xs[index + 1] - glyph_run.xs[index]) / size;
            }
            surface.set_stroke(None);
            surface.set_fill(Some(fill_with(glyph_run.colour, FillRule::NonZero)));
            let (x, y) = glyph_run.start;
            if glyph_run.italic {
                // COSMIC Text's synthetic italic: a 14° slant about the
                // baseline.
                surface.push_transform(&Transform::from_row(1.0, 0.0, -0.25, 1.0, 0.25 * y, 0.0));
            }
            surface.draw_glyphs(
                krilla::geom::Point::from_xy(x, y),
                &glyph_run.glyphs,
                font,
                run.text,
                glyph_run.size,
                false,
            );
            if glyph_run.italic {
                surface.pop();
            }
        }
    }
}

/// The page sizes and drawing of a whole document, written as one PDF.
pub(crate) struct PdfWriter {
    document: krilla::Document,
    resources: PdfResources,
}

/// What a page carries besides its drawing: crop marks (lines x1, y1,
/// x2, y2 in points) and links (a rectangle and where it goes).
#[derive(Clone, Copy, Default)]
pub(crate) struct PageExtras<'a> {
    pub crop: &'a [[f32; 4]],
    pub links: &'a [(krilla::geom::Rect, krilla::destination::XyzDestination)],
}

impl PdfWriter {
    pub(crate) fn new(settings: krilla::SerializeSettings) -> Self {
        Self {
            document: krilla::Document::new_with(settings),
            resources: PdfResources::default(),
        }
    }

    /// Adds a page `size` points wide and high, drawing what `renderer`
    /// recorded through `to_page`, then `marks` (lines x1, y1, x2, y2 in
    /// points: crop marks) as hairlines.
    pub(crate) fn page(
        &mut self,
        size: (f32, f32),
        settings: impl FnOnce(krilla::page::PageSettings) -> krilla::page::PageSettings,
        renderer: &PdfRenderer,
        to_page: Transform,
        vectors: Option<&dyn VectorPictures>,
        extras: PageExtras<'_>,
    ) {
        let (marks, links) = (extras.crop, extras.links);
        let Some(page_size) = krilla::geom::Size::from_wh(size.0.max(1.0), size.1.max(1.0)) else {
            return;
        };
        let mut page = self
            .document
            .start_page_with(settings(krilla::page::PageSettings::new(page_size)));
        let mut surface = page.surface();
        renderer.write(&mut surface, to_page, &mut self.resources, vectors);
        if !marks.is_empty() {
            let mut builder = PathBuilder::new();
            for [x1, y1, x2, y2] in marks {
                builder.move_to(*x1, *y1);
                builder.line_to(*x2, *y2);
            }
            if let Some(path) = builder.finish() {
                surface.set_fill(None);
                surface.set_stroke(Some(krilla::paint::Stroke {
                    paint: rgb::Color::black().into(),
                    width: 0.25,
                    ..krilla::paint::Stroke::default()
                }));
                surface.draw_path(&path);
                surface.set_stroke(None);
            }
        }
        surface.finish();
        // Links (the TOC's rows to their headings, TX6).
        for (rect, destination) in links {
            page.add_annotation(
                krilla::annotation::LinkAnnotation::new(
                    *rect,
                    krilla::annotation::Target::Destination(krilla::destination::Destination::Xyz(
                        destination.clone(),
                    )),
                )
                .into(),
            );
        }
        page.finish();
    }

    /// A page of several recorded drawings, each through its own transform
    /// (labels on a sheet).
    pub(crate) fn sheet(
        &mut self,
        size: (f32, f32),
        pieces: &[(&PdfRenderer, Transform)],
        vectors: Option<&dyn VectorPictures>,
    ) {
        let Some(page_size) = krilla::geom::Size::from_wh(size.0.max(1.0), size.1.max(1.0)) else {
            return;
        };
        let mut page = self
            .document
            .start_page_with(krilla::page::PageSettings::new(page_size));
        let mut surface = page.surface();
        for (renderer, to_page) in pieces {
            renderer.write(&mut surface, *to_page, &mut self.resources, vectors);
        }
        surface.finish();
        page.finish();
    }

    pub(crate) fn document_mut(&mut self) -> &mut krilla::Document {
        &mut self.document
    }

    pub(crate) fn finish(self) -> Result<Vec<u8>, String> {
        self.document.finish().map_err(|error| format!("{error:?}"))
    }
}

#[cfg(test)]
mod tests {
    use cosmic::iced::advanced::Renderer as _;
    use cosmic::iced::advanced::text::{Paragraph as _, Renderer as _};
    use cosmic::iced::{Border, Vector};

    use super::*;

    fn paragraph(content: &str, size: f32) -> Paragraph {
        Paragraph::with_text(text::Text {
            content,
            bounds: Size::INFINITE,
            size: Pixels(size),
            line_height: text::LineHeight::default(),
            font: Font::with_name("Liberation Serif"),
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
            ellipsize: text::Ellipsize::None,
        })
    }

    #[test]
    fn layers_draw_in_the_order_iced_draws_them() {
        let mut renderer = PdfRenderer::new();
        let red = Color::from_rgb(1.0, 0.0, 0.0);
        let blue = Color::from_rgb(0.0, 0.0, 1.0);
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle::new(Point::ORIGIN, Size::new(10.0, 10.0)),
                ..renderer::Quad::default()
            },
            red,
        );
        renderer.with_layer(
            Rectangle::new(Point::new(2.0, 2.0), Size::new(4.0, 4.0)),
            |renderer| {
                renderer.with_translation(Vector::new(1.0, 1.0), |renderer| {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle::new(Point::ORIGIN, Size::new(5.0, 5.0)),
                            ..renderer::Quad::default()
                        },
                        blue,
                    );
                });
            },
        );
        // Recorded in the parent after the child layer: drawn before it.
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle::new(Point::new(20.0, 0.0), Size::new(1.0, 1.0)),
                ..renderer::Quad::default()
            },
            red,
        );
        let quads = renderer.quads();
        assert_eq!(quads.len(), 3);
        assert_eq!(quads[1].0.x, 20.0, "the parent's quads come first");
        assert_eq!(quads[2].0.position(), Point::new(1.0, 1.0), "translated");
        assert_eq!(
            renderer.layers[1].bounds,
            Rectangle::new(Point::new(2.0, 2.0), Size::new(4.0, 4.0))
        );
    }

    #[test]
    fn a_page_with_text_and_quads_is_a_pdf_with_the_text_and_its_font() {
        let mut renderer = PdfRenderer::new();
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle::new(Point::new(10.0, 10.0), Size::new(100.0, 40.0)),
                border: Border {
                    color: Color::BLACK,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..renderer::Quad::default()
            },
            Color::from_rgb(0.9, 0.9, 0.9),
        );
        let text = paragraph("Ami Pro prints", 12.0);
        renderer.fill_paragraph(
            &text,
            Point::new(20.0, 20.0),
            Color::BLACK,
            Rectangle::INFINITE,
        );
        assert_eq!(renderer.texts(), ["Ami Pro prints"]);
        let mut writer = PdfWriter::new(krilla::SerializeSettings::default());
        writer.page(
            (200.0, 100.0),
            |settings| settings,
            &renderer,
            Transform::identity(),
            None,
            PageExtras::default(),
        );
        let pdf = writer.finish().unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        // The font is embedded (a TrueType program) with a map back to the
        // characters, so the text can be found and copied.
        let contains = |needle: &[u8]| pdf.windows(needle.len()).any(|window| window == needle);
        assert!(
            contains(b"/FontFile2") || contains(b"/FontFile3"),
            "an embedded font"
        );
        assert!(contains(b"/ToUnicode"), "text that can be extracted");
    }
}
