//! Charts against Ami Pro's own (`docs/research/OBJECTS_NOTES.md` §11 and
//! §12.1): the user's corpus and the CHV2 fixtures are read-only inputs, so
//! these tests are `#[ignore]`d and skip where the folders are absent
//! (commands in `docs/TEST_COMMANDS.md`). The corpus is
//! `../AmiPro-Reference/Charlie-Sample/SAM` (set `AMIPRO_STYLES` to its
//! `STYLES`), the fixtures `../AmiPro-Reference/vm-lab/OUT/objects/CHV2*.SAM`.
//! Nothing here names a corpus file or keeps its bytes.

use std::path::{Path, PathBuf};

use amipro_document::{FrameContent, PictureFormat, PictureSource, Placement};
use amipro_pictures::chart::{Chart, chart_of};
use amipro_pictures::chart_gen::{Metrics, compare, generate};
use amipro_pictures::sdw::{self, Drawing, Shape};

struct Found {
    /// The frame's size in twips, when it is a fixed one.
    size: Option<(i32, i32)>,
    drawing: Drawing,
    body: Vec<u8>,
}

fn sam_files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sam_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
        {
            out.push(path);
        }
    }
}

fn charts_in(paths: &[PathBuf]) -> Vec<Found> {
    let mut found = Vec::new();
    for path in paths {
        let Ok(import) = amipro_formats::import_sam_path(path) else {
            continue;
        };
        let document = import.document;
        for frame in document.frames() {
            let FrameContent::Picture(picture) = &frame.content else {
                continue;
            };
            if picture.format() != PictureFormat::Chart {
                continue;
            }
            let PictureSource::Embedded(object) = &picture.source else {
                continue;
            };
            let Some(object) = document.objects().get(object) else {
                continue;
            };
            let Some(drawing) = sdw::read(&object.bytes) else {
                continue;
            };
            let Some(record) = drawing.objects.iter().find(|o| o.kind == sdw::kind::CHART) else {
                continue;
            };
            let Shape::Other(body) = &record.shape else {
                continue;
            };
            let size = match &frame.placement {
                Placement::Fixed { rect, .. } => {
                    Some((rect.right - rect.left, rect.bottom - rect.top))
                }
                _ => None,
            };
            found.push(Found {
                size,
                body: body.clone(),
                drawing,
            });
        }
    }
    found
}

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../AmiPro-Reference")
        .join(path)
}

fn corpus() -> Option<Vec<PathBuf>> {
    let folder = &reference("Charlie-Sample/SAM");
    if !folder.is_dir() {
        eprintln!("corpus absent; skipped");
        return None;
    }
    let mut paths = Vec::new();
    sam_files(folder, &mut paths);
    Some(paths)
}

fn fixtures() -> Option<Vec<PathBuf>> {
    let folder = &reference("vm-lab/OUT/objects");
    let paths: Vec<PathBuf> = ["CHV2A.SAM", "CHV2B.SAM", "CHV2C.SAM"]
        .iter()
        .map(|name| folder.join(name))
        .filter(|path| path.is_file())
        .collect();
    if paths.is_empty() {
        eprintln!("CHV2 fixtures absent; skipped");
        return None;
    }
    Some(paths)
}

/// DO.6: every chart record in the user's corpus reads and writes back
/// equal to the byte.
#[test]
#[ignore = "reads the user's corpus"]
fn every_corpus_record_parses_and_writes_back_equal() {
    let Some(paths) = corpus() else { return };
    let found = charts_in(&paths);
    let mut distinct = std::collections::BTreeSet::new();
    let mut equal = 0;
    for chart in &found {
        distinct.insert(chart.body.clone());
        if let Some(parsed) = Chart::parse(&chart.body)
            && parsed.to_body() == chart.body
        {
            equal += 1;
        }
    }
    eprintln!(
        "corpus charts {}, distinct records {}, parsed and equal {}",
        found.len(),
        distinct.len(),
        equal
    );
    assert_eq!(equal, found.len());
    // The census the run's notes quote (107 charts in 34 files).
    assert!(found.len() >= 100);
    // The drawing as a whole writes back through the model unchanged too.
    for chart in &found {
        let (index, parsed) = chart_of(&chart.drawing).unwrap();
        assert_eq!(chart.drawing.objects[index].flags, sdw::CHART_FLAG);
        assert_eq!(parsed.to_body(), chart.body);
    }
}

fn describe(chart: &Chart) -> String {
    format!(
        "{:?} v{} 3D{} grid{} leg{} {}x{}",
        chart.chart_type,
        chart.variation,
        chart.three_d(),
        chart.grid as u8,
        chart.legend as u8,
        chart.columns,
        chart.rows
    )
}

fn report(label: &str, found: &[Found], metrics: impl Fn(&Found) -> Metrics) {
    eprintln!("--- {label}");
    for (index, item) in found.iter().enumerate() {
        let Some((_, chart)) = chart_of(&item.drawing) else {
            continue;
        };
        let metrics = metrics(item);
        let generated = generate(&chart, &metrics);
        let result = compare(&generated, &item.drawing.objects, 3);
        if std::env::var("CHART_DIFF")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            == Some(index)
        {
            show_diff(&generated, &item.drawing.objects);
        }
        eprintln!(
            "{index:2} {:36} gen {:?}/{} st {:?}/{} aligned {} within3 {} worst {} mean {:.1}",
            describe(&chart),
            (
                result.generated.lines,
                result.generated.polygons,
                result.generated.texts,
                result.generated.groups
            ),
            result.generated.total(),
            (
                result.stored.lines,
                result.stored.polygons,
                result.stored.texts,
                result.stored.groups
            ),
            result.stored.total(),
            result.aligned,
            result.within,
            result.worst,
            result.mean
        );
    }
}

fn chv2_metrics(item: &Found) -> Metrics {
    match item.size {
        Some((4608, 3888)) => Metrics {
            width: 4103,
            height: 3395,
            text: 233,
        },
        Some((w, h)) => Metrics::for_frame(w, h),
        None => Metrics {
            width: 4103,
            height: 3395,
            text: 233,
        },
    }
}

/// DO.7: the 18 CHV2 charts regenerated from their own records.
#[test]
#[ignore = "reads the CHV2 fixtures"]
fn chv2_charts_regenerate_close_to_ami_pros() {
    let Some(paths) = fixtures() else { return };
    let found = charts_in(&paths);
    assert_eq!(found.len(), 18);
    for item in &found {
        assert_eq!(Chart::parse(&item.body).unwrap().to_body(), item.body);
    }
    report("CHV2", &found, chv2_metrics);
    // The families the notes call confirmed: the same objects in the same
    // order (kinds and counts), and close in the coordinates. Not asserted:
    // 3 (perspective), 9 (3D expanded pie), 13 (a record that
    // disagrees with its own objects: the 3D click did not take), 15
    // (expanded pie, flat).
    let (mut aligned, mut within) = (0, 0);
    for index in [0, 1, 2, 4, 5, 6, 7, 8, 10, 11, 12, 14, 16, 17] {
        let item = &found[index];
        let (_, chart) = chart_of(&item.drawing).unwrap();
        let result = compare(
            &generate(&chart, &chv2_metrics(item)),
            &item.drawing.objects,
            3,
        );
        assert_eq!(result.generated, result.stored, "chart {index}: counts");
        assert_eq!(
            result.aligned,
            result.stored.total(),
            "chart {index}: order"
        );
        assert!(result.mean < 5.0, "chart {index}: mean {}", result.mean);
        aligned += result.aligned;
        within += result.within;
    }
    eprintln!("confirmed families: {within} of {aligned} objects within 3 twips");
    assert!(within * 100 >= aligned * 85);
}

/// DO.7: the corpus's charts regenerated; the rectangle and label size are
/// fitted from each chart's own objects (the frame rule is open).
#[test]
#[ignore = "reads the user's corpus"]
fn corpus_charts_regenerate_close_to_ami_pros() {
    let Some(paths) = corpus() else { return };
    let mut found = charts_in(&paths);
    let mut seen = std::collections::BTreeSet::new();
    found.retain(|item| seen.insert(sdw::write(&item.drawing)));
    report("corpus (distinct drawings)", &found, |item| {
        let mut right = 0;
        let mut bottom = 0;
        let mut text = 233;
        fn walk(objects: &[sdw::Object], right: &mut i32, bottom: &mut i32, text: &mut i32) {
            for object in objects {
                if object.flags & 4 == 0 {
                    continue;
                }
                if let Shape::Text(t) = &object.shape {
                    *text = i32::from(t.height).max(150);
                }
                if let Shape::Group { objects, .. } = &object.shape {
                    walk(objects, right, bottom, text);
                }
                if object.kind != sdw::kind::CHART {
                    *right = (*right).max(i32::from(object.rect.right));
                    *bottom = (*bottom).max(i32::from(object.rect.bottom));
                }
            }
        }
        walk(&item.drawing.objects, &mut right, &mut bottom, &mut text);
        Metrics {
            width: right + 144,
            height: bottom + 144,
            text,
        }
    });
}

fn flat<'a>(objects: &'a [sdw::Object], out: &mut Vec<&'a sdw::Object>) {
    for object in objects {
        out.push(object);
        if let Shape::Group { objects, .. } = &object.shape {
            flat(objects, out);
        }
    }
}

fn brief(object: &sdw::Object) -> String {
    match &object.shape {
        Shape::Line { points, .. } => format!(
            "line {:?}",
            points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>()
        ),
        Shape::Polygon { points, .. } => format!(
            "poly{} {:?}",
            points.len(),
            points
                .iter()
                .take(6)
                .map(|p| (p.x, p.y))
                .collect::<Vec<_>>()
        ),
        Shape::Text(t) => format!(
            "text {:?} at {:?} size {:?}",
            t.text,
            (t.origin.x, t.origin.y),
            (t.size.x, t.size.y)
        ),
        Shape::Group { .. } => "group".into(),
        _ => format!("kind {}", object.kind),
    }
}

fn show_diff(generated: &[sdw::Object], stored: &[sdw::Object]) {
    let (mut a, mut b) = (Vec::new(), Vec::new());
    flat(generated, &mut a);
    flat(stored, &mut b);
    for i in 0..a.len().max(b.len()) {
        let g = a.get(i).map(|o| brief(o)).unwrap_or_default();
        let s = b.get(i).map(|o| brief(o)).unwrap_or_default();
        eprintln!(
            "[{i}] {}\n     G {g}\n     S {s}",
            if g == s { "same" } else { "DIFF" }
        );
    }
}
