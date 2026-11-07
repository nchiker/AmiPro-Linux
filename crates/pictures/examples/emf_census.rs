//! Plays every enhanced metafile (`.emf`) under a folder and prints
//! aggregate counts: played or refused, the display list's operations, the
//! record types the files use and how often, and the slowest file. Never
//! prints file names.
//!
//! Usage: `emf_census DIRECTORY`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use amipro_pictures::{emf, wmf};

fn emf_files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            emf_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("emf"))
        {
            out.push(path);
        }
    }
}

/// The record types of a file, in order.
fn record_types(bytes: &[u8]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let (Some(kind), Some(size)) = (bytes.get(at..at + 4), bytes.get(at + 4..at + 8)) {
        let kind = u32::from_le_bytes(kind.try_into().unwrap_or_default());
        let size = u32::from_le_bytes(size.try_into().unwrap_or_default()) as usize;
        out.push(kind);
        if size < 8 || kind == 14 {
            break;
        }
        at += size;
    }
    out
}

fn main() {
    let Some(directory) = std::env::args().nth(1) else {
        eprintln!("usage: emf_census DIRECTORY");
        std::process::exit(2);
    };
    let mut files = Vec::new();
    emf_files(Path::new(&directory), &mut files);
    let mut played = 0;
    let mut empty = 0;
    let mut refused = 0;
    let mut ops: BTreeMap<&str, usize> = BTreeMap::new();
    // Record type → (records, files using it).
    let mut types: BTreeMap<u32, (usize, usize)> = BTreeMap::new();
    let mut slowest = 0.0f64;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let mut seen = std::collections::BTreeSet::new();
        for kind in record_types(&bytes) {
            let entry = types.entry(kind).or_default();
            entry.0 += 1;
            if seen.insert(kind) {
                entry.1 += 1;
            }
        }
        let timer = Instant::now();
        let drawing = emf::play(&bytes);
        slowest = slowest.max(timer.elapsed().as_secs_f64() * 1_000.0);
        match drawing {
            Some(drawing) if drawing.ops.is_empty() => empty += 1,
            Some(drawing) => {
                played += 1;
                for op in &drawing.ops {
                    let name = match op {
                        wmf::Op::Path { .. } => "path",
                        wmf::Op::Image { .. } => "image",
                        wmf::Op::FillRect { .. } => "fill",
                        wmf::Op::Text { .. } => "text",
                        wmf::Op::Clip(_) => "clip",
                    };
                    *ops.entry(name).or_default() += 1;
                }
            }
            None => refused += 1,
        }
    }
    println!(
        "EMF files {}: played {played}, nothing drawn {empty}, refused {refused}; slowest {slowest:.1} ms",
        files.len()
    );
    println!("operations: {ops:?}");
    println!("record types (records, files):");
    for (kind, (records, users)) in types {
        println!("  {kind:3}: {records} in {users}");
    }
}
