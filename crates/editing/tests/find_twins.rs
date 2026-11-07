//! Find & Replace against Ami Pro (FD.4): Ami Pro's own input files from the
//! VM, run through the app's Replace All with the options Ami Pro was given,
//! give the text Ami Pro saved (`docs/research/FIND_NOTES.md`, fixtures
//! FR1–FR6); and a sweep that finds every paragraph of a corpus in its
//! stream.
//!
//! Private: `AMIPRO_VM_LAB` names the VM folder (`vm-lab`, holding `IN` and
//! `OUT`), `AMIPRO_CORPUS` a folder of SAM files, `AMIPRO_STYLES` their style
//! sheets:
//!
//! AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab cargo test -p amipro-editing \
//!     --test find_twins -- --ignored --nocapture

use std::path::{Path, PathBuf};

use amipro_document::Document;
use amipro_editing::apply;
use amipro_editing::find::{FindOptions, find_all, replace_all, search_streams, streams};
use amipro_formats::{import_sam_path, import_sam_path_with_style_directories};

/// Every stream's paragraphs' text, in Ami Pro's order.
fn stream_texts(document: &Document) -> Vec<Vec<String>> {
    streams(document)
        .into_iter()
        .map(|stream| {
            document
                .flow(stream.flow)
                .unwrap()
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.text.clone())
                .collect()
        })
        .collect()
}

fn replace_all_from_main(
    document: &Document,
    find: &str,
    replace: &str,
    options: &FindOptions,
) -> Document {
    let scope = search_streams(document, document.main_flow_id(), options);
    let found = find_all(document, find, &scope, options, None);
    let command = replace_all(document, &found, replace, options).expect("something to replace");
    apply(document, &command).unwrap().document
}

#[test]
#[ignore = "needs the VM fixtures (AMIPRO_VM_LAB)"]
fn replace_all_gives_what_ami_pro_gave() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB").map(PathBuf::from) else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let import = |path: PathBuf| import_sam_path(&path).unwrap().document;
    let defaults = FindOptions::default();
    let cases = [
        (
            "IN/FR1.SAM",
            "status bar",
            "window pane",
            defaults.clone(),
            "OUT/FR1A.SAM",
        ),
        (
            "OUT/FR1A.SAM",
            "the",
            "aN",
            defaults.clone(),
            "OUT/FR1B.SAM",
        ),
        (
            "IN/FR1.SAM",
            "status bar",
            "Window PANE",
            FindOptions {
                keep_capitals: false,
                ..defaults.clone()
            },
            "OUT/FR2.SAM",
        ),
        (
            "IN/FR1.SAM",
            "phon*",
            "telephon*",
            FindOptions {
                whole_word: false,
                ..defaults.clone()
            },
            "OUT/FR3.SAM",
        ),
        (
            "IN/FR5.SAM",
            "alpha",
            "beta",
            defaults.clone(),
            "OUT/FR5A.SAM",
        ),
        (
            "IN/FR5.SAM",
            "alpha",
            "beta",
            FindOptions {
                other_streams: false,
                ..defaults.clone()
            },
            "OUT/FR6.SAM",
        ),
    ];
    for (input, find, replace, options, output) in cases {
        let ours = replace_all_from_main(&import(lab.join(input)), find, replace, &options);
        let theirs = import(lab.join(output));
        assert_eq!(stream_texts(&ours), stream_texts(&theirs), "{output}");
        eprintln!("{output}: as Ami Pro's");
    }
}

/// The SAM files under `folder`.
fn sam_files(folder: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sam_files(&path, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
        {
            found.push(path);
        }
    }
}

/// Every paragraph's first words are found where they are, in their
/// stream, and every flow that holds text is a stream.
#[test]
#[ignore = "needs a corpus (AMIPRO_CORPUS)"]
fn every_paragraph_is_found_in_its_stream() {
    let Some(corpus) = std::env::var_os("AMIPRO_CORPUS").map(PathBuf::from) else {
        eprintln!("AMIPRO_CORPUS is not set; skipping");
        return;
    };
    let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    let mut files = Vec::new();
    sam_files(&corpus, &mut files);
    files.sort();
    let options = FindOptions {
        whole_word: false,
        match_case: true,
        ..FindOptions::default()
    };
    let (mut documents, mut searched, mut found_there, mut streams_seen) = (0, 0, 0, 0);
    let mut missed = Vec::new();
    let mut outside: std::collections::BTreeMap<String, usize> = Default::default();
    for path in &files {
        let Ok(imported) = import_sam_path_with_style_directories(path, &styles) else {
            continue;
        };
        let document = imported.document;
        documents += 1;
        let all = streams(&document);
        streams_seen += all.len();
        // Every flow that shows is a stream: covered cells of joined blocks
        // show nothing (notes are marks, not flows).
        for (flow, _) in document.flows() {
            if all.iter().any(|stream| stream.flow == flow) {
                continue;
            }
            *outside.entry(owner_of(&document, flow)).or_default() += 1;
        }
        for stream in &all {
            let paragraphs = &document.flow(stream.flow).unwrap().paragraphs;
            for (index, paragraph) in paragraphs.iter().enumerate() {
                // Up to five words without wildcards, codes, or objects.
                let words: String = paragraph
                    .text
                    .split(' ')
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(" ");
                let words: String = words
                    .chars()
                    .take_while(|c| {
                        !matches!(c, '?' | '*' | '<' | '»' | '¶' | '\t' | '\u{2028}')
                            && *c != amipro_document::OBJECT_REPLACEMENT_CHARACTER
                    })
                    .collect();
                if words.trim().is_empty() {
                    continue;
                }
                searched += 1;
                let found = find_all(&document, &words, &[stream.flow], &options, None);
                if found
                    .iter()
                    .any(|m| m.start.paragraph == index && m.start.byte == 0)
                {
                    found_there += 1;
                } else if missed.len() < 10 {
                    missed.push(format!("{words:?}"));
                }
            }
        }
    }
    eprintln!(
        "documents {documents}, streams {streams_seen}, paragraphs searched {searched}, found where they are {found_there}; flows outside the streams {outside:?}"
    );
    assert_eq!(found_there, searched, "missed, for example: {missed:?}");
    assert!(
        outside.keys().all(|owner| owner == "covered cell"),
        "flows that show but are no stream: {outside:?}"
    );
}

/// What holds a flow that is no stream, for the sweep's report.
fn owner_of(document: &Document, flow: amipro_document::FlowId) -> String {
    for frame in document.frames() {
        match &frame.content {
            amipro_document::FrameContent::Text(text) if *text == flow => {
                return format!("text frame {:?}", frame.placement)
                    .chars()
                    .take(30)
                    .collect();
            }
            amipro_document::FrameContent::Table(table) => {
                for cell in &table.cells {
                    let amipro_document::CellContent::Text(text) = cell.content;
                    if text == flow {
                        return if cell.span.is_covered() {
                            "covered cell".into()
                        } else {
                            format!("cell, table {:?}", frame.placement)
                                .chars()
                                .take(30)
                                .collect()
                        };
                    }
                }
            }
            _ => {}
        }
    }
    for (holder, content) in document.flows() {
        for paragraph in &content.paragraphs {
            for object in &paragraph.objects {
                if object.owned_flow() == Some(flow) {
                    return if holder == document.main_flow_id() {
                        "main text object".into()
                    } else {
                        "object in another flow".into()
                    };
                }
            }
        }
    }
    "no owner".into()
}
