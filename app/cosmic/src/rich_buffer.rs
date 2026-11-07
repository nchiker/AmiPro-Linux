use amipro_document::{CharacterFormat, Document, ParagraphStyle};
use cosmic_text::{
    Attrs, Buffer, Cursor, Family, FontSystem, Metrics, Shaping, Style, UnderlineStyle, Weight,
};

const SCREEN_PIXELS_PER_POINT: f32 = 96.0 / 72.0;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the next custom editor widget gate")
)]
pub struct GlyphGeometry {
    pub start: usize,
    pub end: usize,
    pub x: f32,
    pub width: f32,
    pub font_size: f32,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the next custom editor widget gate")
)]
pub struct LineGeometry {
    pub paragraph: usize,
    pub top: f32,
    pub baseline: f32,
    pub height: f32,
    pub width: f32,
    pub glyphs: Vec<GlyphGeometry>,
    pub decoration_spans: usize,
}

/// Shaped view state derived from the semantic document.
///
/// The font system is retained with the buffer because its font identifiers are
/// meaningful only within that system. Neither value is serialized or treated
/// as editable document state.
pub struct DocumentLayout {
    font_system: FontSystem,
    buffer: Buffer,
    width: f32,
}

impl DocumentLayout {
    pub fn new(document: &Document, width: f32) -> Self {
        let mut layout = Self {
            font_system: FontSystem::new(),
            buffer: document_buffer(document),
            width,
        };
        layout.reshape();
        layout
    }

    #[cfg(test)]
    pub fn paragraph_count(&self) -> usize {
        self.buffer.lines.len()
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "consumed by the next custom editor widget gate")
    )]
    pub fn geometry(&self) -> Vec<LineGeometry> {
        self.buffer
            .layout_runs()
            .map(|run| LineGeometry {
                paragraph: run.line_i,
                top: run.line_top,
                baseline: run.line_y,
                height: run.line_height,
                width: run.line_w,
                glyphs: run
                    .glyphs
                    .iter()
                    .map(|glyph| GlyphGeometry {
                        start: glyph.start,
                        end: glyph.end,
                        x: glyph.x,
                        width: glyph.w,
                        font_size: glyph.font_size,
                    })
                    .collect(),
                decoration_spans: run.decorations.len(),
            })
            .collect()
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "consumed by the next custom editor widget gate")
    )]
    pub fn hit(&self, x: f32, y: f32) -> Option<Cursor> {
        self.buffer.hit(x, y)
    }

    fn reshape(&mut self) {
        self.buffer.set_size(Some(self.width.max(1.0)), None);
        self.buffer.shape_until_scroll(&mut self.font_system, false);
    }
}

/// Builds a derived COSMIC Text buffer from the semantic document.
///
/// This is a layout/view cache for the custom page-widget increment, not a
/// second document model. Edits continue to originate in `Document` commands.
pub fn document_buffer(document: &Document) -> Buffer {
    let default_metrics = metrics(12.0, 0.0);
    let default_attrs = Attrs::new().metrics(default_metrics);
    let default_format = CharacterFormat::default();
    let mut spans = Vec::new();

    for (paragraph_index, paragraph) in document.paragraphs().iter().enumerate() {
        let style = &document.styles()[&paragraph.style];
        let base_attrs = resolved_attrs(style, &default_format);
        if paragraph.text.is_empty() {
            spans.push(("", base_attrs.clone()));
        } else {
            let mut cursor = 0;
            for run in &paragraph.runs {
                if cursor < run.start {
                    spans.push((&paragraph.text[cursor..run.start], base_attrs.clone()));
                }
                spans.push((
                    &paragraph.text[run.start..run.end],
                    resolved_attrs(style, &run.format),
                ));
                cursor = run.end;
            }
            if cursor < paragraph.text.len() {
                spans.push((&paragraph.text[cursor..], base_attrs.clone()));
            }
        }
        if paragraph_index + 1 < document.paragraphs().len() {
            spans.push(("\n", base_attrs));
        }
    }

    let mut buffer = Buffer::new_empty(default_metrics);
    buffer.set_rich_text(spans, &default_attrs, Shaping::Advanced, None);
    buffer
}

fn resolved_attrs<'a>(style: &'a ParagraphStyle, format: &'a CharacterFormat) -> Attrs<'a> {
    let family = format.font_family.as_deref().unwrap_or(&style.font_family);
    let size = format.font_size_points.unwrap_or(style.font_size_points);
    let bold = format.bold.unwrap_or(style.bold);
    let italic = format.italic.unwrap_or(style.italic);
    let underline = format.underline.unwrap_or(false);
    let attrs = Attrs::new()
        .family(Family::Name(family))
        .weight(if bold { Weight::BOLD } else { Weight::NORMAL })
        .style(if italic { Style::Italic } else { Style::Normal })
        .metrics(metrics(size, style.spacing.below as f32 / 20.0));
    if underline {
        attrs.underline(UnderlineStyle::Single)
    } else {
        attrs
    }
}

fn metrics(font_size_points: f32, space_after_points: f32) -> Metrics {
    let font_size = font_size_points * SCREEN_PIXELS_PER_POINT;
    Metrics::new(
        font_size,
        font_size.mul_add(1.2, space_after_points * SCREEN_PIXELS_PER_POINT),
    )
}

#[cfg(test)]
mod tests {
    use amipro_document::{CharacterFormat, Document, StyleId};

    use super::*;

    #[test]
    fn rich_buffer_carries_paragraph_metrics_and_direct_decoration() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Heading detail".into();
        document.paragraph_mut(0).style = StyleId(2);
        document
            .paragraph_mut(0)
            .apply_character_format(
                8,
                14,
                &CharacterFormat {
                    font_size_points: Some(9.0),
                    italic: Some(true),
                    underline: Some(true),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();

        let buffer = document_buffer(&document);
        let heading = buffer.lines[0].attrs_list().get_span(0);
        let detail = buffer.lines[0].attrs_list().get_span(8);
        let heading_metrics: Metrics = heading.metrics_opt.unwrap().into();
        let detail_metrics: Metrics = detail.metrics_opt.unwrap().into();

        assert_eq!(heading_metrics.font_size, 20.0 * SCREEN_PIXELS_PER_POINT);
        assert_eq!(heading.weight, Weight::BOLD);
        assert_eq!(detail_metrics.font_size, 9.0 * SCREEN_PIXELS_PER_POINT);
        assert_eq!(detail.style, Style::Italic);
        assert_eq!(detail.text_decoration.underline, UnderlineStyle::Single);
    }

    #[test]
    fn rich_buffer_uses_one_line_per_document_paragraph() {
        let document = Document::from_plain_text("first\n\nthird");

        let buffer = document_buffer(&document);

        assert_eq!(buffer.lines.len(), 3);
        assert_eq!(buffer.lines[0].text(), "first");
        assert_eq!(buffer.lines[1].text(), "");
        assert_eq!(buffer.lines[2].text(), "third");
    }

    #[test]
    fn shaped_geometry_preserves_mixed_metrics_decorations_and_utf8_hits() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Large café detail".into();
        document.paragraph_mut(0).style = StyleId(2);
        let paragraph_len = document.paragraphs()[0].text.len();
        document
            .paragraph_mut(0)
            .apply_character_format(
                11,
                paragraph_len,
                &CharacterFormat {
                    font_size_points: Some(9.0),
                    underline: Some(true),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();

        let layout = DocumentLayout::new(&document, 220.0);
        let geometry = layout.geometry();
        let glyphs: Vec<_> = geometry.iter().flat_map(|line| &line.glyphs).collect();

        assert_eq!(layout.paragraph_count(), 1);
        assert!(glyphs.iter().any(|glyph| glyph.font_size > 20.0));
        assert!(glyphs.iter().any(|glyph| glyph.font_size < 13.0));
        assert!(geometry.iter().any(|line| line.decoration_spans > 0));

        let end = layout.hit(10_000.0, geometry[0].top + 1.0).unwrap();
        assert!(document.paragraphs()[0].text.is_char_boundary(end.index));
    }
}
