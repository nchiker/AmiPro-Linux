use std::collections::BTreeSet;
use std::hint::black_box;
use std::time::Instant;

use amipro_document::{CharacterFormat, CharacterRun, Document, Paragraph, StyleId};
use betteroffice_layout_poc::{
    layout_document_with_measurement, probe_advanced_blocks, project_document_with_measurement,
};
use docx_layout::display_list::Primitive;
use docx_layout::measure_blocks::MeasurementConfig;
use serde_json::json;

fn benchmark_document() -> Document {
    Document {
        paragraphs: (0..300)
            .map(|index| {
                let text = format!(
                    "Performance paragraph {index}: Ami Pro layout projection with mixed formatting and enough words to wrap predictably over many pages."
                );
                Paragraph {
                    text,
                    style: if index % 13 == 0 {
                        StyleId(2)
                    } else {
                        StyleId(1)
                    },
                    runs: vec![CharacterRun {
                        start: 0,
                        end: "Performance".len(),
                        format: CharacterFormat {
                            bold: Some(index % 2 == 0),
                            italic: Some(index % 3 == 0),
                            underline: Some(index % 5 == 0),
                            font_size_points: (index % 17 == 0).then_some(18.0),
                            ..CharacterFormat::default()
                        },
                    }],
                }
            })
            .collect(),
        ..Document::default()
    }
}

fn main() {
    let document = benchmark_document();
    let mode = std::env::var("POC_MODE").unwrap_or_else(|_| "full".into());
    if mode == "current" {
        run_current_benchmark(&document);
        return;
    }
    let measurement = system_font_measurement();
    let iterations = std::env::var("POC_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(20);
    let started = Instant::now();
    if mode == "layout-only" {
        let mut pages = 0;
        for _ in 0..iterations {
            let mut projection = project_document_with_measurement(
                black_box(&document),
                &BTreeSet::new(),
                &measurement,
            )
            .unwrap();
            let layout = docx_layout::compute_layout_input(&mut projection.input).unwrap();
            pages = layout.pages.len();
            black_box((projection, layout));
        }
        let elapsed = started.elapsed();
        println!(
            "betteroffice_layout_only iterations={iterations} total_ms={:.3} mean_ms={:.3} pages={pages}",
            elapsed.as_secs_f64() * 1_000.0,
            elapsed.as_secs_f64() * 1_000.0 / f64::from(iterations)
        );
        return;
    }
    let mut betteroffice_pages = 0;
    let mut primitive_count = 0;
    let mut primitive_kinds = [0_usize; 7];
    let mut serialized_bytes = 0;
    let mut measurement_time = 0.0;
    let mut pagination_time = 0.0;
    let mut display_list_time = 0.0;
    for _ in 0..iterations {
        let result =
            layout_document_with_measurement(black_box(&document), &BTreeSet::new(), &measurement)
                .unwrap();
        betteroffice_pages = result.layout.pages.len();
        primitive_count = result
            .display_list
            .pages
            .iter()
            .map(|page| page.primitives.len())
            .sum();
        primitive_kinds = [0; 7];
        for primitive in result
            .display_list
            .pages
            .iter()
            .flat_map(|page| &page.primitives)
        {
            let slot = match primitive {
                Primitive::Text(_) => 0,
                Primitive::GlyphRun(_) => 1,
                Primitive::Rect(_) => 2,
                Primitive::Line(_) => 3,
                Primitive::Image(_) => 4,
                Primitive::Shape(_) => 5,
                Primitive::Decoration(_) => 6,
            };
            primitive_kinds[slot] += 1;
        }
        serialized_bytes = serde_json::to_vec(&result.display_list).unwrap().len();
        measurement_time += result.measurement_time.as_secs_f64() * 1_000.0;
        pagination_time += result.pagination_time.as_secs_f64() * 1_000.0;
        display_list_time += result.display_list_time.as_secs_f64() * 1_000.0;
        black_box(result);
    }
    let betteroffice = started.elapsed();

    println!("documents paragraphs={}", document.paragraphs.len());
    println!(
        "betteroffice iterations={iterations} total_ms={:.3} mean_ms={:.3} pages={betteroffice_pages} primitives={primitive_count} display_json_bytes={serialized_bytes}",
        betteroffice.as_secs_f64() * 1_000.0,
        betteroffice.as_secs_f64() * 1_000.0 / f64::from(iterations)
    );
    println!(
        "betteroffice_components mean_measure_ms={:.3} mean_paginate_ms={:.3} mean_display_list_ms={:.3}",
        measurement_time / f64::from(iterations),
        pagination_time / f64::from(iterations),
        display_list_time / f64::from(iterations)
    );
    println!(
        "primitive_kinds text={} glyph_run={} rect={} line={} image={} shape={} decoration={}",
        primitive_kinds[0],
        primitive_kinds[1],
        primitive_kinds[2],
        primitive_kinds[3],
        primitive_kinds[4],
        primitive_kinds[5],
        primitive_kinds[6]
    );
    run_current_benchmark(&document);
    println!("advanced_blocks={:?}", probe_advanced_blocks().unwrap());
}

fn run_current_benchmark(document: &Document) {
    let iterations = 2_000;
    let started = Instant::now();
    let mut pages = 0;
    for _ in 0..iterations {
        let result = amipro_layout::paginate(black_box(document));
        pages = result.len();
        black_box(result);
    }
    let elapsed = started.elapsed();
    println!(
        "current_estimator iterations={iterations} total_ms={:.3} mean_ms={:.6} pages={pages}",
        elapsed.as_secs_f64() * 1_000.0,
        elapsed.as_secs_f64() * 1_000.0 / f64::from(iterations)
    );
}

fn system_font_measurement() -> MeasurementConfig {
    docx_layout::clear_measure_fonts();
    let faces = [
        (
            "0|0",
            "/usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf",
        ),
        ("1|0", "/usr/share/fonts/truetype/noto/NotoSerif-Bold.ttf"),
        ("0|1", "/usr/share/fonts/truetype/noto/NotoSerif-Italic.ttf"),
        (
            "1|1",
            "/usr/share/fonts/truetype/noto/NotoSerif-BoldItalic.ttf",
        ),
    ];
    let mut font_chains = std::collections::BTreeMap::new();
    for (variant, path) in faces {
        let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("read {path}: {error}"));
        let id = docx_layout::register_measure_font(&bytes)
            .unwrap_or_else(|error| panic!("register {path}: {error:?}"));
        font_chains.insert(format!("noto serif|{variant}"), vec![id]);
    }
    MeasurementConfig {
        font_chains,
        defaults: json!({"fontSize": 12.0, "fontFamily": "Noto Serif"}),
        compat: serde_json::Value::Null,
        authoritative_shaping: true,
    }
}
