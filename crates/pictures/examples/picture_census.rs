//! Counts, over a folder of SAM files, what each kind of picture is drawn
//! from (`docs/PICTURES_RUNBOOK.md`, PI.4): a reader of its own bytes or
//! file, Ami Pro's screen image (decision P4), or nothing (a box). Prints
//! aggregate numbers only, never file names.
//!
//! Usage: `picture_census DIRECTORY`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use amipro_document::{FrameContent, PictureFormat, PictureSource};
use amipro_pictures::{dib, emf, ole, screen, sdw, wmf};

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

/// A linked picture's file: its path, then its name in the document's
/// folder in any case (decision P2).
fn linked_file(document: &Path, path: &str) -> Option<Vec<u8>> {
    if let Ok(bytes) = std::fs::read(path) {
        return Some(bytes);
    }
    let name = path.rsplit(['\\', '/']).next()?.to_ascii_lowercase();
    let folder = document.parent()?;
    std::fs::read_dir(folder).ok()?.flatten().find_map(|entry| {
        (entry.file_name().to_string_lossy().to_ascii_lowercase() == name)
            .then(|| std::fs::read(entry.path()).ok())
            .flatten()
    })
}

/// Whether one of the readers draws these bytes of this kind.
fn readable(format: PictureFormat, bytes: &[u8]) -> bool {
    match format {
        PictureFormat::Bitmap => {
            if bytes.starts_with(b"BM") {
                dib::decode_bmp(bytes).is_some()
            } else {
                dib::decode_packed(bytes, dib::ColourTable::Rgb).is_some()
            }
        }
        PictureFormat::Metafile => wmf::play(bytes).is_some_and(|drawing| !drawing.ops.is_empty()),
        PictureFormat::Emf => emf::play(bytes).is_some_and(|drawing| !drawing.ops.is_empty()),
        PictureFormat::Drawing | PictureFormat::Chart => sdw::read(bytes).is_some(),
        // An OLE object from its presentation metafile (DO.3).
        PictureFormat::Ole => ole::read(bytes)
            .and_then(|object| object.metafile(bytes).and_then(wmf::play))
            .is_some_and(|drawing| !drawing.ops.is_empty()),
        _ => false,
    }
}

fn main() {
    let Some(directory) = std::env::args().nth(1) else {
        eprintln!("usage: picture_census DIRECTORY");
        std::process::exit(2);
    };
    let mut files = Vec::new();
    sam_files(Path::new(&directory), &mut files);
    // Kind → (pictures, drawn by a reader, from the screen image, boxes).
    let mut counts: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    let mut slowest = (0.0f64, 0usize);
    let started = Instant::now();
    for path in &files {
        let Ok(import) = amipro_formats::import_sam_path(path) else {
            continue;
        };
        let document = import.document;
        for frame in document.frames() {
            let FrameContent::Picture(picture) = &frame.content else {
                continue;
            };
            let format = picture.format();
            let linked = matches!(picture.source, PictureSource::Linked(_));
            let kind = format!("{}{}", format.name(), if linked { " (linked)" } else { "" });
            let row = counts.entry(kind).or_default();
            row[0] += 1;
            let bytes = match &picture.source {
                PictureSource::Embedded(object) => document
                    .objects()
                    .get(object)
                    .map(|object| object.bytes.to_vec()),
                PictureSource::Linked(file) => linked_file(path, file),
            };
            let timer = Instant::now();
            let drawn = bytes
                .as_deref()
                .is_some_and(|bytes| readable(format, bytes));
            let elapsed = timer.elapsed().as_secs_f64() * 1_000.0;
            if elapsed > slowest.0 {
                slowest = (elapsed, bytes.as_ref().map_or(0, Vec::len));
            }
            if drawn {
                row[1] += 1;
            } else if picture
                .rendering
                .and_then(|object| document.objects().get(&object))
                .is_some_and(|object| screen::decode(&object.bytes).is_some())
            {
                row[2] += 1;
            } else {
                row[3] += 1;
            }
        }
    }
    println!("SAM files: {}", files.len());
    println!("kind | pictures | drawn by a reader | from Ami Pro's screen image | boxes");
    let mut total = [0; 4];
    for (kind, row) in &counts {
        println!("{kind} | {} | {} | {} | {}", row[0], row[1], row[2], row[3]);
        for (sum, value) in total.iter_mut().zip(row) {
            *sum += value;
        }
    }
    println!(
        "all | {} | {} | {} | {}",
        total[0], total[1], total[2], total[3]
    );
    println!(
        "time: {:.0} ms in all; slowest reading {:.2} ms ({} bytes)",
        started.elapsed().as_secs_f64() * 1_000.0,
        slowest.0,
        slowest.1
    );
}
