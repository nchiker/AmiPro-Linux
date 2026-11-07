//! Imports every document under a folder (DOCX, ODT, FODT, RTF, text) and
//! prints aggregate counts: read, refused and why, findings by kind, and
//! the time taken. A panic in a reader stops the run with the file's name
//! on standard error (never in Git: these are private or sample files).
//!
//! `cargo run --release -p amipro-exchange --example import -- DIR [--parts]`
//!
//! With `--parts` it only opens each package and parses its XML parts (the
//! readers' ground, IM.2).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use amipro_exchange::import::{ImportOptions, SourceFormat, detect, read, style_option};
use amipro_exchange::realise::{RealiseOptions, realise};
use amipro_exchange::xml_tree::Tree;
use amipro_exchange::zip::Archive;

fn inflate(data: &[u8], size: usize) -> Option<Vec<u8>> {
    let bytes = miniz_oxide::inflate::decompress_to_vec_with_limit(data, size).ok()?;
    (bytes.len() == size).then_some(bytes)
}

fn files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == ".git") {
                continue;
            }
            files(&path, out);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "docx" | "docm" | "dotx" | "odt" | "ott" | "fodt" | "rtf" | "txt"
                )
            })
        {
            out.push(path);
        }
    }
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(directory) = arguments.next() else {
        eprintln!("usage: import DIR [--parts]");
        std::process::exit(2);
    };
    let parts_only = arguments.any(|argument| argument == "--parts");
    // File names on standard error, for looking into one locally.
    let verbose = std::env::var_os("IMPORT_VERBOSE").is_some();
    let mut paths = Vec::new();
    if Path::new(&directory).is_file() {
        paths.push(PathBuf::from(&directory));
    } else {
        files(Path::new(&directory), &mut paths);
    }
    let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut formats: BTreeMap<&str, usize> = BTreeMap::new();
    let mut total = Duration::ZERO;
    let mut slowest = Duration::ZERO;
    let (mut xml_parts, mut xml_refused) = (0, 0);
    for path in &paths {
        let Ok(bytes) = std::fs::read(path) else {
            *outcomes.entry("not readable".into()).or_default() += 1;
            continue;
        };
        let extension = path.extension().and_then(|extension| extension.to_str());
        let format = detect(&bytes, extension);
        *formats
            .entry(format.map_or("none", SourceFormat::name))
            .or_default() += 1;
        let started = Instant::now();
        if parts_only {
            match format {
                Some(SourceFormat::Docx | SourceFormat::Odt) => match Archive::open(&bytes) {
                    Ok(archive) => {
                        let names: Vec<String> = archive.names().map(str::to_owned).collect();
                        for name in names {
                            let lower = name.to_ascii_lowercase();
                            if !(lower.ends_with(".xml") || lower.ends_with(".rels")) {
                                continue;
                            }
                            match archive.read(&name, &inflate) {
                                Ok(part) => {
                                    xml_parts += 1;
                                    let parsing = Instant::now();
                                    if let Err(error) = Tree::parse(&part) {
                                        xml_refused += 1;
                                        if verbose {
                                            eprintln!("{} {name}: {error}", path.display());
                                        }
                                    }
                                    if verbose && parsing.elapsed().as_millis() > 200 {
                                        eprintln!(
                                            "slow {} {name}: {} ms, {} bytes",
                                            path.display(),
                                            parsing.elapsed().as_millis(),
                                            part.len()
                                        );
                                    }
                                }
                                Err(error) => {
                                    *outcomes.entry(format!("part: {error}")).or_default() += 1;
                                }
                            }
                        }
                        *outcomes.entry("package opened".into()).or_default() += 1;
                    }
                    Err(error) => *outcomes.entry(format!("package: {error}")).or_default() += 1,
                },
                Some(SourceFormat::FlatOdt) => {
                    xml_parts += 1;
                    if Tree::parse(&bytes).is_err() {
                        xml_refused += 1;
                    }
                    *outcomes.entry("flat file parsed".into()).or_default() += 1;
                }
                _ => *outcomes.entry("not a package".into()).or_default() += 1,
            }
        }
        if !parts_only {
            let options = ImportOptions::default();
            let outcome = std::panic::catch_unwind(|| {
                read(&bytes, extension, &options, &inflate).map(|imported| {
                    let base = amipro_document::Document::default();
                    let realise_options = RealiseOptions {
                        styles: style_option(imported.format, &options),
                        base: &base,
                        inserting: false,
                        into_text: false,
                        title: "Imported".into(),
                        now: 0,
                    };
                    let mut findings = imported.findings.clone();
                    let realised = realise(
                        &imported.document,
                        &realise_options,
                        &mut amipro_layout::FixedShaper,
                        &mut findings,
                    );
                    let problem = realised
                        .document
                        .validate()
                        .err()
                        .map(|error| error.to_string());
                    (problem, findings)
                })
            });
            match outcome {
                Ok(Ok((problem, found))) => {
                    match &problem {
                        None => *outcomes.entry("imported".into()).or_default() += 1,
                        Some(problem) => {
                            *outcomes
                                .entry(format!("imported, not valid: {problem}"))
                                .or_default() += 1;
                            if verbose {
                                eprintln!("not valid: {}: {problem}", path.display());
                            }
                        }
                    }
                    for finding in found.iter() {
                        *kinds
                            .entry(
                                format!("{:?}", finding.kind)
                                    .split('(')
                                    .next()
                                    .unwrap_or("")
                                    .to_owned(),
                            )
                            .or_default() += finding.count as usize;
                    }
                }
                Ok(Err(error)) => *outcomes.entry(format!("refused: {error}")).or_default() += 1,
                Err(_) => {
                    eprintln!("PANIC: {}", path.display());
                    *outcomes.entry("panicked".into()).or_default() += 1;
                }
            }
        }
        let elapsed = started.elapsed();
        if verbose && elapsed.as_millis() > 2_000 {
            eprintln!("slow {}: {} ms", path.display(), elapsed.as_millis());
        }
        total += elapsed;
        slowest = slowest.max(elapsed);
    }
    println!("Files: {}", paths.len());
    for (format, count) in &formats {
        println!("  detected {format}: {count}");
    }
    for (outcome, count) in &outcomes {
        println!("  {outcome}: {count}");
    }
    if parts_only {
        println!("XML parts parsed: {xml_parts}, refused {xml_refused}");
    }
    for (kind, count) in &kinds {
        println!("  finding {kind}: {count}");
    }
    println!(
        "Time: {:.1} ms in all, slowest {:.1} ms",
        total.as_secs_f64() * 1e3,
        slowest.as_secs_f64() * 1e3
    );
}
