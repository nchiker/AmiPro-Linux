//! Isolated BetterOffice layout experiment.
//!
//! This crate is deliberately outside both production workspaces. It projects
//! immutable `amipro_document::Document` snapshots and never participates in
//! the application editing path.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use amipro_document::{CharacterFormat, Document, Paragraph, ParagraphStyle};
use docx_layout::display_list::DisplayList;
use docx_layout::measure_blocks::{MeasurementConfig, measure_blocks};
use docx_layout::types::{
    BlockExtent, Input, Layout, LayoutBlock, LayoutOptions, MeasuredBlock, PageMargins, Size,
};
use serde_json::json;

/// BetterOffice lays pages out in CSS pixels while the AmiPro-Linux model uses
/// typographic points.
pub const PX_PER_POINT: f64 = 96.0 / 72.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub paragraph: usize,
    pub byte_offset: usize,
}

#[derive(Clone, Debug)]
pub struct ParagraphPositionMap {
    pub paragraph: usize,
    pub paragraph_start: i64,
    pub text_start: i64,
    pub text_end: i64,
    pub paragraph_end: i64,
    utf16_to_byte: Vec<(i64, usize)>,
}

impl ParagraphPositionMap {
    fn source_location(&self, position: i64) -> SourceLocation {
        let relative = position.clamp(self.text_start, self.text_end) - self.text_start;
        let byte_offset = self
            .utf16_to_byte
            .iter()
            .rev()
            .find(|(utf16, _)| *utf16 <= relative)
            .map_or(0, |(_, byte)| *byte);
        SourceLocation {
            paragraph: self.paragraph,
            byte_offset,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SourceMap {
    pub paragraphs: Vec<ParagraphPositionMap>,
}

impl SourceMap {
    pub fn resolve(&self, position: i64) -> Option<SourceLocation> {
        self.paragraphs
            .iter()
            .find(|span| position >= span.paragraph_start && position <= span.paragraph_end)
            .map(|span| span.source_location(position))
    }
}

#[derive(Debug)]
pub struct Projection {
    pub input: Input,
    pub source_map: SourceMap,
}

#[derive(Debug)]
pub struct PocLayout {
    pub projection: Projection,
    pub layout: Layout,
    pub display_list: DisplayList,
    pub measurement_time: Duration,
    pub pagination_time: Duration,
    pub display_list_time: Duration,
}

/// Project the current model without changing it. `page_breaks_before` is an
/// experiment-only overlay because the Phase 1 semantic model does not yet
/// contain an explicit-break node.
pub fn project_document(
    document: &Document,
    page_breaks_before: &BTreeSet<usize>,
) -> Result<Projection, String> {
    project_document_with_measurement(document, page_breaks_before, &fallback_measurement_config())
}

pub fn project_document_with_measurement(
    document: &Document,
    page_breaks_before: &BTreeSet<usize>,
    measurement: &MeasurementConfig,
) -> Result<Projection, String> {
    document.validate().map_err(|error| error.to_string())?;
    let page_width = f64::from(document.page.width_points) * PX_PER_POINT;
    let page_height = f64::from(document.page.height_points) * PX_PER_POINT;
    let [top, right, bottom, left] = document.page.margin_points.map(f64::from);
    let content_width = (page_width - (left + right) * PX_PER_POINT).max(1.0);
    let mut blocks = Vec::new();
    let mut spans = Vec::with_capacity(document.paragraphs.len());
    let mut position = 0_i64;

    for (paragraph_index, paragraph) in document.paragraphs.iter().enumerate() {
        if paragraph_index > 0 && page_breaks_before.contains(&paragraph_index) {
            blocks.push(
                serde_json::from_value(json!({
                    "kind": "pageBreak",
                    "id": format!("amipro-break-{paragraph_index}"),
                    "pmStart": position,
                    "pmEnd": position + 1
                }))
                .map_err(|error| error.to_string())?,
            );
            position += 1;
        }

        let style = document
            .styles
            .get(&paragraph.style)
            .ok_or_else(|| format!("paragraph {paragraph_index} has no style"))?;
        let paragraph_start = position;
        let text_start = paragraph_start + 1;
        let utf16_len = paragraph.text.encode_utf16().count() as i64;
        let text_end = text_start + utf16_len;
        let paragraph_end = text_end + 1;
        let runs = projected_runs(paragraph, style, text_start);
        blocks.push(
            serde_json::from_value(json!({
                "kind": "paragraph",
                "id": format!("amipro-p-{paragraph_index}"),
                "paraId": format!("amipro-{paragraph_index:08x}"),
                "runs": runs,
                "attrs": {
                    "defaultFontFamily": style.font_family,
                    "defaultFontSize": style.font_size_points,
                    "spacing": {
                        "after": f64::from(style.space_after_points) * PX_PER_POINT
                    }
                },
                "pmStart": paragraph_start,
                "pmEnd": paragraph_end
            }))
            .map_err(|error| error.to_string())?,
        );
        spans.push(ParagraphPositionMap {
            paragraph: paragraph_index,
            paragraph_start,
            text_start,
            text_end,
            paragraph_end,
            utf16_to_byte: utf16_to_byte_boundaries(&paragraph.text),
        });
        position = paragraph_end;
    }

    let extents = measure_blocks(&mut blocks, content_width, measurement)?;
    let measured = blocks
        .into_iter()
        .zip(extents)
        .map(|(block, measure)| MeasuredBlock { block, measure })
        .collect();
    Ok(Projection {
        input: Input {
            measured,
            options: LayoutOptions {
                page_size: Some(Size {
                    w: page_width,
                    h: page_height,
                }),
                margins: Some(PageMargins {
                    top: top * PX_PER_POINT,
                    right: right * PX_PER_POINT,
                    bottom: bottom * PX_PER_POINT,
                    left: left * PX_PER_POINT,
                    header: None,
                    footer: None,
                }),
                page_gap: Some(20.0),
                ..LayoutOptions::default()
            },
        },
        source_map: SourceMap { paragraphs: spans },
    })
}

pub fn layout_document(
    document: &Document,
    page_breaks_before: &BTreeSet<usize>,
) -> Result<PocLayout, String> {
    layout_document_with_measurement(document, page_breaks_before, &fallback_measurement_config())
}

pub fn layout_document_with_measurement(
    document: &Document,
    page_breaks_before: &BTreeSet<usize>,
    measurement: &MeasurementConfig,
) -> Result<PocLayout, String> {
    let measurement_started = Instant::now();
    let mut projection =
        project_document_with_measurement(document, page_breaks_before, measurement)?;
    let measurement_time = measurement_started.elapsed();
    let pagination_started = Instant::now();
    let layout = docx_layout::compute_layout_input(&mut projection.input)
        .map_err(|error| error.to_string())?;
    let pagination_time = pagination_started.elapsed();
    let display_started = Instant::now();
    let extras = json!({ "fontChains": measurement.font_chains }).to_string();
    let display_list =
        docx_layout::build_display_list_value_from_resident(&projection.input, &layout, &extras)?;
    let display_list_time = display_started.elapsed();
    Ok(PocLayout {
        projection,
        layout,
        display_list,
        measurement_time,
        pagination_time,
        display_list_time,
    })
}

pub fn fallback_measurement_config() -> MeasurementConfig {
    MeasurementConfig {
        font_chains: Default::default(),
        defaults: json!({
            "fontSize": 12.0,
            "fontFamily": "Noto Serif"
        }),
        compat: serde_json::Value::Null,
        authoritative_shaping: true,
    }
}

fn projected_runs(
    paragraph: &Paragraph,
    style: &ParagraphStyle,
    text_start: i64,
) -> Vec<serde_json::Value> {
    let mut boundaries = BTreeSet::from([0, paragraph.text.len()]);
    for run in &paragraph.runs {
        boundaries.insert(run.start);
        boundaries.insert(run.end);
    }
    let boundaries = boundaries.into_iter().collect::<Vec<_>>();
    if boundaries.len() == 1 {
        return vec![run_json("", style, None, text_start, text_start)];
    }

    boundaries
        .windows(2)
        .filter_map(|window| {
            let start = window[0];
            let end = window[1];
            (start < end).then(|| {
                let direct = paragraph
                    .runs
                    .iter()
                    .find(|run| run.start <= start && run.end >= end)
                    .map(|run| &run.format);
                let pm_start = text_start + paragraph.text[..start].encode_utf16().count() as i64;
                let pm_end = text_start + paragraph.text[..end].encode_utf16().count() as i64;
                run_json(&paragraph.text[start..end], style, direct, pm_start, pm_end)
            })
        })
        .collect()
}

fn run_json(
    text: &str,
    style: &ParagraphStyle,
    direct: Option<&CharacterFormat>,
    pm_start: i64,
    pm_end: i64,
) -> serde_json::Value {
    let family = direct
        .and_then(|format| format.font_family.as_deref())
        .unwrap_or(&style.font_family);
    let size = direct
        .and_then(|format| format.font_size_points)
        .unwrap_or(style.font_size_points);
    let bold = direct.and_then(|format| format.bold).unwrap_or(style.bold);
    let italic = direct
        .and_then(|format| format.italic)
        .unwrap_or(style.italic);
    let underline = direct.and_then(|format| format.underline).unwrap_or(false);
    json!({
        "kind": "text",
        "text": text,
        "fontFamily": family,
        "fontSize": size,
        "bold": bold,
        "italic": italic,
        "underline": underline,
        "pmStart": pm_start,
        "pmEnd": pm_end
    })
}

fn utf16_to_byte_boundaries(text: &str) -> Vec<(i64, usize)> {
    let mut output = vec![(0, 0)];
    let mut utf16 = 0_i64;
    for (byte, character) in text.char_indices() {
        utf16 += character.len_utf16() as i64;
        output.push((utf16, byte + character.len_utf8()));
    }
    output
}

/// Directly exercise advanced BetterOffice inputs that the current Ami model
/// cannot yet represent. This is capability evidence, not a model projection.
pub fn probe_advanced_blocks() -> Result<Vec<&'static str>, String> {
    let paragraph = |id: &str, text: &str| {
        json!({
            "kind": "paragraph", "id": id,
            "runs": [{"kind": "text", "text": text, "fontSize": 12.0}],
            "attrs": {"defaultFontSize": 12.0, "defaultFontFamily": "Noto Serif"}
        })
    };
    let values = vec![
        (
            "table",
            json!({
                "kind": "table", "id": "table", "columnWidths": [180.0, 180.0],
                "rows": [{"id": 0, "cells": [
                    {"id": 0, "blocks": [paragraph("cell-a", "left cell")]},
                    {"id": 1, "blocks": [paragraph("cell-b", "right cell")]}
                ]}]
            }),
        ),
        (
            "textBox",
            json!({
                "kind": "textBox", "id": "frame", "width": 240.0, "height": 100.0,
                "content": [paragraph("frame-p", "frame text")],
                "position": {"horizontal": {"posOffset": 120.0, "relativeTo": "page"},
                             "vertical": {"posOffset": 160.0, "relativeTo": "page"}},
                "wrapType": "square"
            }),
        ),
        (
            "anchoredImage",
            json!({
                "kind": "image", "id": "image", "src": "poc:transparent",
                "width": 96.0, "height": 72.0,
                "anchor": {"isAnchored": true, "offsetH": 144.0, "offsetV": 180.0,
                           "wrapType": "square", "wrapText": "bothSides"}
            }),
        ),
    ];
    let mut supported = Vec::new();
    for (name, value) in values {
        let mut blocks: Vec<LayoutBlock> =
            vec![serde_json::from_value(value).map_err(|error| format!("{name}: {error}"))?];
        let measures = measure_blocks(&mut blocks, 624.0, &fallback_measurement_config())?;
        if matches!(measures.first(), Some(BlockExtent::Unsupported) | None) {
            return Err(format!("{name}: unsupported measurement"));
        }
        supported.push(name);
    }
    Ok(supported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{CharacterRun, Paragraph, StyleId};
    use docx_layout::display_list::Primitive;
    use docx_layout::hit::{caret_rect, hit_test, range_rects};

    fn representative_document() -> Document {
        let mut document = Document::default();
        let mut first = Paragraph {
            text: "Small text, BIG TEXT, bold, italic, and underlined.".into(),
            style: StyleId(1),
            runs: Vec::new(),
        };
        let big_start = first.text.find("BIG").unwrap();
        first.runs.push(CharacterRun {
            start: big_start,
            end: big_start + "BIG TEXT".len(),
            format: CharacterFormat {
                font_size_points: Some(22.0),
                bold: Some(true),
                underline: Some(true),
                ..CharacterFormat::default()
            },
        });
        document.paragraphs = std::iter::once(first)
            .chain((0..75).map(|index| Paragraph {
                text: format!(
                    "Paragraph {index}: enough representative text to wrap across the available page width and exercise repeatable multipage flow."
                ),
                style: if index % 11 == 0 { StyleId(2) } else { StyleId(1) },
                runs: if index % 7 == 0 {
                    vec![CharacterRun {
                        start: 0,
                        end: "Paragraph".len(),
                        format: CharacterFormat {
                            italic: Some(true),
                            ..CharacterFormat::default()
                        },
                    }]
                } else {
                    Vec::new()
                },
            }))
            .collect();
        document
    }

    #[test]
    fn projection_is_read_only_and_preserves_rich_run_attributes() {
        let document = representative_document();
        let unchanged = document.clone();
        let projected = project_document(&document, &BTreeSet::new()).unwrap();
        assert_eq!(document, unchanged);

        let LayoutBlock::Paragraph(first) = &projected.input.measured[0].block else {
            panic!("first block should be a paragraph");
        };
        let encoded = serde_json::to_value(first).unwrap();
        let runs = encoded["runs"].as_array().unwrap();
        assert!(runs.iter().any(|run| run["fontSize"] == 22.0));
        assert!(runs.iter().any(|run| run["bold"] == true));
        assert!(runs.iter().any(|run| run["underline"] == true));
        assert!(runs.iter().any(|run| run["italic"] == false));
    }

    #[test]
    fn explicit_break_and_natural_flow_produce_multiple_pages() {
        let document = representative_document();
        let result = layout_document(&document, &BTreeSet::from([20])).unwrap();
        assert!(result.layout.pages.len() >= 3);
        assert_eq!(result.layout.pages.len(), result.display_list.pages.len());

        let before = &result.projection.source_map.paragraphs[19];
        let after = &result.projection.source_map.paragraphs[20];
        let before_page = caret_rect(&result.display_list, before.text_start)
            .expect("paragraph before break has a caret")
            .page_index;
        let after_page = caret_rect(&result.display_list, after.text_start)
            .expect("paragraph after break has a caret")
            .page_index;
        assert!(after_page > before_page);
    }

    #[test]
    fn source_positions_and_hits_cross_paragraph_and_page_boundaries() {
        let mut document = representative_document();
        document.paragraphs[20].text = "Unicode café 🙂 after the page break".into();
        document.paragraphs[20].runs.clear();
        let result = layout_document(&document, &BTreeSet::from([20])).unwrap();

        for paragraph_index in [0, 20, 60] {
            let span = &result.projection.source_map.paragraphs[paragraph_index];
            let caret = caret_rect(&result.display_list, span.text_start)
                .expect("projected paragraph has caret geometry");
            let hit = hit_test(
                &result.display_list,
                caret.page_index,
                caret.x,
                caret.y + caret.height / 2.0,
            )
            .expect("caret point hits text");
            let source = result.projection.source_map.resolve(hit).unwrap();
            assert_eq!(source.paragraph, paragraph_index);
            assert_eq!(source.byte_offset, 0);
        }

        let from = result.projection.source_map.paragraphs[19].text_start;
        let to = result.projection.source_map.paragraphs[21].text_end;
        let rectangles = range_rects(&result.display_list, from, to);
        let pages = rectangles
            .iter()
            .map(|rectangle| rectangle.page_index)
            .collect::<BTreeSet<_>>();
        assert!(
            pages.len() >= 2,
            "selection should cross a forced page boundary"
        );
    }

    #[test]
    fn display_list_retains_renderer_work_for_rich_text() {
        let result = layout_document(&representative_document(), &BTreeSet::new()).unwrap();
        let text_runs = result.display_list.pages[0]
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Text(_) | Primitive::GlyphRun(_)))
            .count();
        let decorations = result.display_list.pages[0]
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Decoration(_)))
            .count();
        assert!(text_runs > 1);
        assert!(
            decorations > 0,
            "underline should require a paint primitive"
        );
    }

    #[test]
    fn advanced_api_probe_measures_supported_block_kinds() {
        assert_eq!(
            probe_advanced_blocks().unwrap(),
            vec!["table", "textBox", "anchoredImage"]
        );
    }
}
