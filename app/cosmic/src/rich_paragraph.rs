use std::ops::Range;

use amipro_document::{CharacterFormat, Document, Paragraph, ParagraphStyle};
use cosmic::iced::advanced::text::{LineHeight, Span};
use cosmic::iced::{Font, Pixels};

use crate::document_highlighter::document_font;
use crate::workspace_geometry::SCREEN_PIXELS_PER_POINT;

#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the next custom editor widget gate")
)]
pub struct ProjectedSpan<'a> {
    pub source: Range<usize>,
    pub display: Span<'a, (), Font>,
}

#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the next custom editor widget gate")
)]
pub struct RichParagraphProjection<'a> {
    pub paragraph: usize,
    pub spans: Vec<ProjectedSpan<'a>>,
    pub space_after: f32,
}

pub fn project_paragraph(
    document: &Document,
    paragraph_index: usize,
    display_scale: f32,
) -> Option<RichParagraphProjection<'_>> {
    let paragraph = document.paragraphs().get(paragraph_index)?;
    let style = document.styles().get(&paragraph.style)?;
    let mut spans = Vec::with_capacity(paragraph.runs.len() * 2 + 1);
    let mut cursor = 0;

    for run in &paragraph.runs {
        if cursor < run.start {
            spans.push(project_span(
                paragraph,
                style,
                cursor..run.start,
                &CharacterFormat::default(),
                display_scale,
            ));
        }
        spans.push(project_span(
            paragraph,
            style,
            run.start..run.end,
            &run.format,
            display_scale,
        ));
        cursor = run.end;
    }
    if cursor < paragraph.text.len() || spans.is_empty() {
        spans.push(project_span(
            paragraph,
            style,
            cursor..paragraph.text.len(),
            &CharacterFormat::default(),
            display_scale,
        ));
    }

    Some(RichParagraphProjection {
        paragraph: paragraph_index,
        spans,
        space_after: style.spacing.below as f32 / 20.0 * SCREEN_PIXELS_PER_POINT * display_scale,
    })
}

fn project_span<'a>(
    paragraph: &'a Paragraph,
    style: &ParagraphStyle,
    source: Range<usize>,
    format: &CharacterFormat,
    display_scale: f32,
) -> ProjectedSpan<'a> {
    let size_points = format.font_size_points.unwrap_or(style.font_size_points);
    let size = size_points * SCREEN_PIXELS_PER_POINT * display_scale;
    let family = format.font_family.as_deref().unwrap_or(&style.font_family);
    let font = document_font(
        family,
        format.bold.unwrap_or(style.bold),
        format.italic.unwrap_or(style.italic),
    );
    let display = Span::new(&paragraph.text[source.clone()])
        .font(font)
        .size(Pixels(size))
        .line_height(LineHeight::Absolute(Pixels(size * 1.2)))
        .underline(format.underline.unwrap_or(false));
    ProjectedSpan { source, display }
}

#[cfg(test)]
mod tests {
    use amipro_document::{CharacterFormat, Document, StyleId};
    use cosmic::iced::font;

    use super::*;

    #[test]
    fn projection_keeps_source_ranges_metrics_underline_and_paragraph_spacing() {
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

        let projection = project_paragraph(&document, 0, 0.75).unwrap();

        assert_eq!(projection.paragraph, 0);
        assert_eq!(projection.spans.len(), 2);
        assert_eq!(projection.spans[0].source, 0..8);
        assert_eq!(projection.spans[0].display.text.as_ref(), "Heading ");
        assert_eq!(projection.spans[0].display.size, Some(Pixels(20.0)));
        assert_eq!(
            projection.spans[0].display.font.unwrap().weight,
            font::Weight::Bold
        );
        assert_eq!(projection.spans[1].source, 8..14);
        assert_eq!(projection.spans[1].display.text.as_ref(), "detail");
        assert_eq!(projection.spans[1].display.size, Some(Pixels(9.0)));
        assert_eq!(
            projection.spans[1].display.font.unwrap().style,
            font::Style::Italic
        );
        assert!(projection.spans[1].display.underline);
        assert_eq!(projection.space_after, 10.0);
    }

    #[test]
    fn empty_paragraph_has_a_caret_bearing_span() {
        let document = Document::default();
        let projection = project_paragraph(&document, 0, 1.0).unwrap();

        assert_eq!(projection.spans.len(), 1);
        assert_eq!(projection.spans[0].source, 0..0);
        assert_eq!(projection.spans[0].display.text.as_ref(), "");
    }
}
