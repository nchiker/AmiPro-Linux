//! Reads every Ami Pro drawing in a folder (drawings and charts in SAM
//! files, and `.sdw` files) and counts what they hold, whether each reads,
//! and whether the writer gives its bytes back (`docs/DRAWING_RUNBOOK.md`,
//! DR.2). Prints aggregate numbers only, never file names.
//!
//! Usage: `drawing_census DIRECTORY...`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use amipro_document::{FrameContent, PictureFormat, PictureSource};
use amipro_pictures::sdw::{self, Object, Shape};

fn files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("sam") || extension.eq_ignore_ascii_case("sdw")
        }) {
            out.push(path);
        }
    }
}

fn name(object: &Object) -> &'static str {
    match &object.shape {
        Shape::Line { .. } => "line",
        Shape::Polyline { .. } => "polyline",
        Shape::Polygon { .. } => "polygon",
        Shape::Rectangle { .. } => "rectangle",
        Shape::RoundedRectangle { .. } => "rounded rectangle",
        Shape::Ellipse { .. } => "ellipse",
        Shape::Arc { .. } => "arc",
        Shape::Text(_) => "text",
        Shape::Group { .. } => "group",
        Shape::Bitmap { .. } => "bitmap",
        Shape::Other(_) if object.kind == sdw::kind::CHART => "chart data",
        Shape::Other(_) => "kept as stored",
    }
}

fn count(objects: &[Object], kinds: &mut BTreeMap<&'static str, usize>) {
    for object in objects {
        *kinds.entry(name(object)).or_default() += 1;
        if let Shape::Group { objects, .. } = &object.shape {
            count(objects, kinds);
        }
    }
}

/// The stream with what Ami Pro rebuilds on reading zeroed: each block's
/// handles, bounds, and bytes used, each record's links.
fn normal(bytes: &[u8], end: usize) -> Vec<u8> {
    let mut bytes = bytes[..end].to_vec();
    let mut at = 0;
    while at + 16 <= bytes.len() {
        if &bytes[at..at + 2] == b"SM" {
            bytes[at + 4..at + 8].fill(0);
            bytes[at + 10..at + 22].fill(0);
            at += 22;
            continue;
        }
        bytes[at + 12..at + 16].fill(0);
        let length = usize::from(u16::from_le_bytes([bytes[at + 2], bytes[at + 3]]));
        if length < 16 {
            break;
        }
        at += length;
    }
    bytes
}

fn main() {
    let directories: Vec<String> = std::env::args().skip(1).collect();
    if directories.is_empty() {
        eprintln!("usage: drawing_census DIRECTORY...");
        std::process::exit(2);
    }
    let mut paths = Vec::new();
    for directory in &directories {
        files(Path::new(directory), &mut paths);
    }
    let mut streams: Vec<Vec<u8>> = Vec::new();
    let (mut sam_files, mut sdw_files) = (0, 0);
    for path in &paths {
        let is_sdw = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sdw"));
        if is_sdw {
            sdw_files += 1;
            if let Ok(bytes) = std::fs::read(path) {
                streams.push(bytes);
            }
            continue;
        }
        let Ok(import) = amipro_formats::import_sam_path(path) else {
            continue;
        };
        sam_files += 1;
        let document = import.document;
        for frame in document.frames() {
            let FrameContent::Picture(picture) = &frame.content else {
                continue;
            };
            if !matches!(
                picture.format(),
                PictureFormat::Drawing | PictureFormat::Chart
            ) {
                continue;
            }
            if let PictureSource::Embedded(object) = &picture.source
                && let Some(object) = document.objects().get(object)
            {
                streams.push(object.bytes.to_vec());
            }
        }
    }
    let mut kinds = BTreeMap::new();
    let (mut read, mut same_bytes, mut same_bounds, mut round_trip) = (0, 0, 0, 0);
    let started = Instant::now();
    for bytes in &streams {
        let Some(drawing) = sdw::read(bytes) else {
            continue;
        };
        read += 1;
        count(&drawing.objects, &mut kinds);
        let written = sdw::write(&drawing);
        if sdw::read(&written).as_ref() == Some(&drawing) {
            round_trip += 1;
        }
        if written.len() <= bytes.len()
            && normal(&written, written.len()) == normal(bytes, written.len())
        {
            same_bytes += 1;
        }
        if bytes.get(12..20) == written.get(12..20) {
            same_bounds += 1;
        }
    }
    println!(
        "files: {sam_files} SAM, {sdw_files} .sdw; drawings: {}",
        streams.len()
    );
    println!("read: {read}; read back equal after writing: {round_trip}");
    println!(
        "written byte for byte but for handles, bounds, bytes used, links: {same_bytes}; root bounds as Ami Pro's: {same_bounds}"
    );
    for (kind, count) in &kinds {
        println!("{kind}: {count}");
    }
    println!(
        "time: {:.0} ms to read and write them all",
        started.elapsed().as_secs_f64() * 1_000.0
    );
}
