//! The SAM writer's round-trip audit (Phase 4): reads every SAM under a
//! directory, writes it again with the SAM writer, reads the written file
//! back with the same style sheet, and compares. It prints aggregate counts
//! only, so that its output can be recorded without the corpus's names.
//!
//! Usage: `sam_round_trip DIRECTORY [--detail FILE] [--out DIRECTORY]`
//!
//! `AMIPRO_STYLES` is the style-sheet path, as for the corpus audit.
//! `--detail` writes each file's differences to FILE (keep it outside Git);
//! `--out` writes each written SAM there, under its own name; `--show
//! TEXT` prints both sides of each difference for the files whose path
//! contains TEXT (on the terminal only: it shows the documents' text).

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use amipro_formats::roundtrip::{Change, describe, differences, is_writer_addition};
use amipro_formats::{
    LegacyKind, SamOptions, Section, import_sam, import_sam_path_with_style_directories, parse,
    sam_document,
};

/// A fixed save time, so that written files are repeatable.
const SAVED_AT: u32 = 1_791_958_088;

fn main() {
    let mut arguments = env::args_os().skip(1);
    let Some(root) = arguments.next().map(PathBuf::from) else {
        eprintln!("usage: sam_round_trip DIRECTORY [--detail FILE] [--out DIRECTORY]");
        std::process::exit(2);
    };
    let mut detail_path = None;
    let mut out_directory = None;
    let mut show = None;
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--detail") => detail_path = arguments.next().map(PathBuf::from),
            Some("--show") => show = arguments.next().and_then(|text| text.into_string().ok()),
            Some("--out") => out_directory = arguments.next().map(PathBuf::from),
            _ => {
                eprintln!("unknown argument {argument:?}");
                std::process::exit(2);
            }
        }
    }
    let style_directories: Vec<PathBuf> = env::var_os("AMIPRO_STYLES")
        .map(|paths| env::split_paths(&paths).collect())
        .unwrap_or_default();
    let mut files = Vec::new();
    visit(&root, &mut files);
    files.sort();

    let mut audit = Audit::default();
    let mut detail = String::new();
    for path in &files {
        audit.files += 1;
        let Ok(original) = import_sam_path_with_style_directories(path, &style_directories) else {
            audit.unreadable += 1;
            continue;
        };
        audit.read += 1;
        let original_warnings = original.report.warnings.clone();
        let document = original.document;
        // What File › Save As would list (the app's own check).
        match amipro_formats::sam_losses(&document, &SamOptions::at(SAVED_AT)) {
            Ok(losses) if losses.is_empty() => audit.no_losses += 1,
            Ok(losses) => {
                let listed: Vec<String> = losses.iter().map(|loss| loss.message()).collect();
                detail.push_str(&format!(
                    "{}: Save As would list: {}\n",
                    path.display(),
                    listed.join("; ")
                ));
            }
            Err(error) => {
                detail.push_str(&format!("{}: loss check failed: {error}\n", path.display()));
            }
        }
        let style_data = original
            .report
            .associated_style_path
            .as_ref()
            .and_then(|path| fs::read(path).ok());
        let started = Instant::now();
        let written = sam_document(&document, &SamOptions::at(SAVED_AT));
        let write_time = started.elapsed();
        audit.write_time += write_time;
        if write_time > audit.slowest_write.0 {
            audit.slowest_write = (write_time, written.len());
        }
        if let Some(directory) = &out_directory
            && let Some(name) = path.file_name()
        {
            let _ = fs::create_dir_all(directory);
            let _ = fs::write(directory.join(name), &written);
        }
        let started = Instant::now();
        let copy = import_sam(&written, style_data.as_deref(), document.title.clone());
        let read_time = started.elapsed();
        audit.read_time += read_time;
        if audit.slowest_write.1 == written.len() {
            audit.slowest_read = read_time;
        }
        let copy = match copy {
            Ok(copy) => {
                // A warning the original did not raise: the written file has
                // a shape the reader (and maybe Ami Pro) does not expect.
                let new: Vec<_> = copy
                    .report
                    .warnings
                    .iter()
                    .filter(|warning| !original_warnings.contains(warning))
                    .collect();
                if !new.is_empty() {
                    audit.new_warnings += 1;
                    for warning in new.iter().take(5) {
                        detail
                            .push_str(&format!("{}: read back warns: {warning}\n", path.display()));
                    }
                }
                copy.document
            }
            Err(error) => {
                audit.not_read_back += 1;
                detail.push_str(&format!("{}: not read back: {error}\n", path.display()));
                continue;
            }
        };
        audit.read_back += 1;
        let found: Vec<_> = differences(&document, &copy)
            .into_iter()
            .filter(|difference| !is_writer_addition(difference))
            .collect();
        if let Some(text) = &show
            && path.to_string_lossy().contains(text.as_str())
        {
            show_differences(path, &document, &copy);
        }
        if found.is_empty() {
            audit.equal += 1;
        }
        let but_line_heights = found.iter().all(|difference| {
            difference.place == "document" && difference.aspect == "line heights"
        });
        if but_line_heights {
            audit.equal_but_line_heights += 1;
        }
        // The main text alone (slice 4.4's measure): every fact whose
        // place is the main text.
        let main_equal = !found
            .iter()
            .any(|difference| difference.place.starts_with("main"));
        if main_equal {
            audit.main_equal += 1;
        }
        if document.frame_stack().is_empty() {
            audit.without_page_frames += 1;
            if main_equal {
                audit.main_equal_without_page_frames += 1;
            }
        }
        // Text and styles only: no page frames, and the page layout is
        // the style sheet's (slice 4.3's measure).
        if document.frame_stack().is_empty()
            && document
                .style_sheet()
                .is_some_and(|link| link.layout_from_sheet)
        {
            audit.text_only += 1;
            if but_line_heights {
                audit.text_only_equal += 1;
            }
        }
        let mut aspects_here: BTreeMap<String, usize> = BTreeMap::new();
        for difference in &found {
            *aspects_here
                .entry(format!(
                    "{} {}",
                    area(&difference.place),
                    difference.aspect_kind()
                ))
                .or_default() += 1;
        }
        for aspect in aspects_here.keys() {
            *audit.aspects.entry(aspect.clone()).or_default() += 1;
        }
        if !found.is_empty() {
            detail.push_str(&format!(
                "{}: {} differences\n",
                path.display(),
                found.len()
            ));
            for difference in found.iter().take(40) {
                detail.push_str(&format!(
                    "  {:?} {} / {}\n",
                    difference.change, difference.place, difference.aspect
                ));
            }
        }
        // The written file against the original, section by section.
        let original_bytes = fs::read(path).unwrap_or_default();
        audit.sections.add(&original_bytes, &written);
    }

    println!("SAM files: {}", audit.files);
    println!("Read: {} (not readable: {})", audit.read, audit.unreadable);
    println!(
        "Read back: {} (not read back: {})",
        audit.read_back, audit.not_read_back
    );
    println!(
        "Read back with warnings the original did not raise: {}",
        audit.new_warnings
    );
    println!("Read back equal: {} of {}", audit.equal, audit.read);
    println!(
        "Save As would list nothing (the app's loss check): {} of {}",
        audit.no_losses, audit.read
    );
    println!(
        "Read back equal but for line heights (from the paragraph caches): {} of {}",
        audit.equal_but_line_heights, audit.read
    );
    println!(
        "  of them with text and styles only (no page frames, the sheet's layout): {} of {}",
        audit.text_only_equal, audit.text_only
    );
    println!(
        "Main text read back equal: {} of {} ({} of {} documents without page frames)",
        audit.main_equal,
        audit.read,
        audit.main_equal_without_page_frames,
        audit.without_page_frames
    );
    println!("Files with differences, by part and aspect:");
    for (aspect, count) in &audit.aspects {
        println!("  {aspect}: {count}");
    }
    let sections = &audit.sections;
    println!(
        "Sections equal to the original: {} of {} (missing {}, added {})",
        sections.equal, sections.original, sections.missing, sections.added
    );
    println!(
        "Bodies equal to the original: {} of {}",
        sections.bodies_equal, audit.read
    );
    println!(
        "Files equal to the original but for [desc] and [pg]: {} of {}",
        sections.files_equal, audit.read
    );
    println!(
        "Write time: {:.1} ms in all, slowest {:.2} ms ({} bytes, read back in {:.2} ms); read back {:.1} ms in all",
        audit.write_time.as_secs_f64() * 1e3,
        audit.slowest_write.0.as_secs_f64() * 1e3,
        audit.slowest_write.1,
        audit.slowest_read.as_secs_f64() * 1e3,
        audit.read_time.as_secs_f64() * 1e3,
    );
    if let Some(path) = detail_path {
        let _ = fs::write(path, detail);
    }
}

#[derive(Default)]
struct Audit {
    files: usize,
    read: usize,
    unreadable: usize,
    read_back: usize,
    not_read_back: usize,
    new_warnings: usize,
    no_losses: usize,
    equal: usize,
    equal_but_line_heights: usize,
    text_only: usize,
    text_only_equal: usize,
    main_equal: usize,
    without_page_frames: usize,
    main_equal_without_page_frames: usize,
    aspects: BTreeMap<String, usize>,
    sections: SectionAudit,
    write_time: Duration,
    read_time: Duration,
    slowest_write: (Duration, usize),
    slowest_read: Duration,
}

/// The part of the document a place belongs to.
fn area(place: &str) -> &str {
    let first = place.split(' ').next().unwrap_or_default();
    match first {
        "document" | "style" | "layout" | "frame" | "main" => first,
        _ => "other",
    }
}

trait AspectKind {
    fn aspect_kind(&self) -> String;
}

impl AspectKind for amipro_formats::roundtrip::Difference {
    /// The aspect without numbers (`object 2` → `object`), and how it
    /// changed.
    fn aspect_kind(&self) -> String {
        let aspect: String = self
            .aspect
            .split(' ')
            .filter(|word| word.parse::<usize>().is_err() && !word.starts_with('#'))
            .collect::<Vec<_>>()
            .join(" ");
        let change = match self.change {
            Change::Changed => "changed",
            Change::Missing => "missing",
            Change::Added => "added",
        };
        format!("{aspect} ({change})")
    }
}

/// Top-level sections of the written file against the original's.
#[derive(Default)]
struct SectionAudit {
    original: usize,
    equal: usize,
    missing: usize,
    added: usize,
    bodies_equal: usize,
    files_equal: usize,
}

impl SectionAudit {
    fn add(&mut self, original: &[u8], written: &[u8]) {
        let (Ok((original, _)), Ok((written, _))) = (
            parse(original, LegacyKind::Document),
            parse(written, LegacyKind::Document),
        ) else {
            return;
        };
        let comparable = |sections: &[Section]| -> Vec<(String, Vec<String>)> {
            sections
                .iter()
                .filter(|section| !section.name.eq_ignore_ascii_case("pg"))
                .map(|section| {
                    let mut values = section.values.clone();
                    if section.name.eq_ignore_ascii_case("desc") && values.len() > 6 {
                        values.drain(5..7);
                    }
                    (section.name.to_ascii_lowercase(), values)
                })
                .collect()
        };
        let before = comparable(&original.sections);
        let after = comparable(&written.sections);
        // Sections equal in order: each original section matches the next
        // equal one in the written file.
        let mut position = 0;
        let mut matched = 0;
        for section in &before {
            if section.0 == "edoc" {
                continue;
            }
            if let Some(offset) = after[position..].iter().position(|other| other == section) {
                position += offset + 1;
                matched += 1;
            } else if env::var_os("AMIPRO_ROUND_TRIP_SECTIONS").is_some() {
                eprintln!(
                    "unmatched section [{}] {:?}",
                    section.0,
                    &section.1[..section.1.len().min(4)]
                );
            }
        }
        let headers = |sections: &[(String, Vec<String>)]| {
            sections.iter().filter(|(name, _)| name != "edoc").count()
        };
        self.original += headers(&before);
        self.equal += matched;
        self.missing += headers(&before) - matched;
        self.added += headers(&after).saturating_sub(matched);
        let body = |sections: &[(String, Vec<String>)]| {
            sections
                .iter()
                .find(|(name, _)| name == "edoc")
                .map(|(_, values)| values.clone())
        };
        if body(&before) == body(&after) {
            self.bodies_equal += 1;
        }
        if before == after {
            self.files_equal += 1;
        }
    }
}

fn show_differences(
    path: &Path,
    original: &amipro_document::Document,
    copy: &amipro_document::Document,
) {
    let after: BTreeMap<(String, String), String> = describe(copy)
        .into_iter()
        .map(|fact| ((fact.place, fact.aspect), fact.value))
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    eprintln!("== {}", path.display());
    for fact in describe(original) {
        let key = (fact.place.clone(), fact.aspect.clone());
        seen.insert(key.clone());
        match after.get(&key) {
            Some(value) if *value == fact.value => {}
            Some(value) => {
                let (before, after) = around_difference(&fact.value, value);
                eprintln!("{} / {}\n  - {before}\n  + {after}", key.0, key.1)
            }
            None => eprintln!("{} / {}\n  - {}\n  + (none)", key.0, key.1, fact.value),
        }
    }
    for (key, value) in &after {
        if !seen.contains(key) {
            eprintln!("{} / {}\n  - (none)\n  + {}", key.0, key.1, value);
        }
    }
}

/// Both values from a little before their first difference.
fn around_difference(before: &str, after: &str) -> (String, String) {
    let common = before
        .char_indices()
        .zip(after.chars())
        .find(|((_, a), b)| a != b)
        .map_or(before.len().min(after.len()), |((index, _), _)| index);
    if env::var_os("AMIPRO_ROUND_TRIP_FULL").is_some() {
        return (before.to_owned(), after.to_owned());
    }
    let start = before[..common]
        .char_indices()
        .rev()
        .nth(60)
        .map_or(0, |(index, _)| index);
    let width = env::var("AMIPRO_ROUND_TRIP_WIDTH")
        .ok()
        .and_then(|width| width.parse().ok())
        .unwrap_or(200);
    let cut =
        |text: &str| -> String { text[start.min(text.len())..].chars().take(width).collect() };
    (cut(before), cut(after))
}

fn visit(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
        {
            files.push(path);
        }
    }
}
