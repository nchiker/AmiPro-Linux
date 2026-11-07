use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use amipro_document::{Document, FrameContent, Placement};
use amipro_formats::{
    LegacyKind, SheetOptions, escape_text, import_sam, import_sam_path_with_style_directories,
    layout_record, parse, read_style_sheet, style_record, style_sheet,
};

fn main() {
    let Some(root) = env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: corpus_audit PRIVATE_CORPUS_DIRECTORY");
        std::process::exit(2);
    };
    // Ami Pro's style-sheet path: directories searched for each SAM's STY
    // after those beside it.
    let style_directories: Vec<PathBuf> = env::var_os("AMIPRO_STYLES")
        .map(|paths| env::split_paths(&paths).collect())
        .unwrap_or_default();
    let mut files = Vec::new();
    visit(&root, &mut files);
    files.sort();

    let mut imported = 0usize;
    let mut failed = Vec::new();
    let mut paragraphs = 0usize;
    let mut style_resolved = 0usize;
    let mut style_declared = 0usize;
    let mut controls = BTreeMap::new();
    let mut sections = BTreeSet::new();
    let mut native_round_trips = 0usize;
    let mut native_round_trip_failures = Vec::new();
    let mut documents_with_character_runs = 0usize;
    let mut character_runs = 0usize;
    let mut body_text_paragraphs = 0usize;
    // Imported text that still looks like control syntax suggests an
    // unbalanced control or an unrecognized escape.
    let mut control_like_paragraphs = Vec::new();
    // Pages and frames (Phase 2.4).
    let mut frames_by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut layouts_per_document: BTreeMap<usize, usize> = BTreeMap::new();
    let mut documents_with_frames = 0usize;
    let mut anchors_resolved = 0usize;
    let mut anchors_unresolved = Vec::new();
    let mut anchored_without_mark = 0usize;
    let mut unknown_frame_flags = 0u32;
    let mut nonuniform_page_columns = 0usize;
    let mut legacy_marks = 0usize;
    let mut dropped_caches = 0usize;
    let mut embedded_objects = 0usize;
    let mut control_leaks = Vec::new();
    let mut inline_objects: BTreeMap<&'static str, usize> = BTreeMap::new();
    // Layout with the fixed-advance shaper: a robustness check on real
    // structures, not a comparison with Ami Pro (that is 2.5d).
    let mut laid_out_pages = 0usize;
    let mut layout_time = std::time::Duration::ZERO;
    let mut most_pages = (0usize, String::new());
    let mut slowest = (std::time::Duration::ZERO, String::new());
    for path in files.iter().filter(|path| extension_is(path, "sam")) {
        match import_sam_path_with_style_directories(path, &style_directories) {
            Ok(result) => {
                imported += 1;
                let imported_runs = result
                    .document
                    .paragraphs()
                    .iter()
                    .map(|paragraph| paragraph.runs.len())
                    .sum::<usize>();
                documents_with_character_runs += usize::from(imported_runs > 0);
                character_runs += imported_runs;
                match result
                    .document
                    .encode_native()
                    .and_then(|encoded| Document::decode_native(&encoded))
                {
                    Ok(reopened) if reopened == result.document => native_round_trips += 1,
                    Ok(_) => native_round_trip_failures
                        .push(format!("{}: reopened model differs", path.display())),
                    Err(error) => {
                        native_round_trip_failures.push(format!("{}: {error}", path.display()))
                    }
                }
                paragraphs += result.report.paragraphs_imported;
                for paragraph in result.document.paragraphs() {
                    body_text_paragraphs +=
                        usize::from(result.document.styles()[&paragraph.style].name == "Body Text");
                    if looks_like_control(&paragraph.text) {
                        let excerpt: String = paragraph.text.chars().take(60).collect();
                        control_like_paragraphs.push(format!("{}: {excerpt:?}", path.display()));
                    }
                }
                let stats = &result.report.structure;
                let frames: Vec<_> = result.document.frames().collect();
                documents_with_frames += usize::from(
                    result.document.frame_stack().len()
                        + result
                            .document
                            .layouts()
                            .iter()
                            .map(|layout| layout.repeating.len())
                            .sum::<usize>()
                        > 0,
                );
                for frame in frames {
                    let content = match frame.content {
                        FrameContent::Text(_) => "text",
                        FrameContent::Table(_) => "table",
                        FrameContent::Picture(_) => "picture",
                        FrameContent::Legacy(_) => "legacy",
                    };
                    let placement = match frame.placement {
                        Placement::Fixed { .. } => "where placed",
                        Placement::BelowParagraph { .. } => "with para above",
                        Placement::Inline { .. } => "flow with text",
                        Placement::Repeating { .. } => "repeating",
                        Placement::Header { .. } => "header",
                        Placement::Footer { .. } => "footer",
                        Placement::InCell => "in a table cell",
                    };
                    *frames_by_kind
                        .entry(format!("{content}, {placement}"))
                        .or_default() += 1;
                }
                for paragraph in result.document.paragraphs() {
                    for object in &paragraph.objects {
                        let kind = match object {
                            amipro_document::InlineObject::Break {
                                kind: amipro_document::BreakKind::Page,
                                layout: Some(_),
                                ..
                            } => "page break switching layout",
                            amipro_document::InlineObject::Break {
                                kind: amipro_document::BreakKind::Page,
                                ..
                            } => "page break",
                            amipro_document::InlineObject::Break { .. } => "column break",
                            amipro_document::InlineObject::FloatingHeaderFooter { .. } => {
                                "floating header or footer"
                            }
                            amipro_document::InlineObject::FrameAnchor(_) => "frame anchor",
                            amipro_document::InlineObject::PageNumber(_) => "page number",
                            amipro_document::InlineObject::Bullet(_) => "inserted bullet",
                            amipro_document::InlineObject::Note { .. } => "note",
                            amipro_document::InlineObject::Footnote { .. } => "footnote",
                            amipro_document::InlineObject::TableFootnote { .. } => "table footnote",
                            amipro_document::InlineObject::Date(_) => "date or time",
                            amipro_document::InlineObject::DocInfo(_) => "Doc Info entry",
                            amipro_document::InlineObject::Seq(_) => "Seq field",
                        };
                        *inline_objects.entry(kind).or_default() += 1;
                    }
                }
                *layouts_per_document
                    .entry(result.document.layouts().len())
                    .or_default() += 1;
                anchors_resolved += stats.anchors_resolved;
                if stats.anchors_unresolved > 0 {
                    anchors_unresolved.push(format!(
                        "{}: {}",
                        path.display(),
                        stats.anchors_unresolved
                    ));
                }
                anchored_without_mark += stats.anchored_without_mark;
                unknown_frame_flags |= stats.unknown_frame_flags;
                nonuniform_page_columns += stats.nonuniform_page_columns;
                legacy_marks += stats.legacy_marks;
                dropped_caches += stats.dropped_caches;
                embedded_objects += stats.embedded_objects;
                if stats.unbalanced_controls > 0 {
                    control_leaks.push(format!(
                        "{}: {}",
                        path.display(),
                        stats.unbalanced_controls
                    ));
                }
                let started = std::time::Instant::now();
                let layout = amipro_layout::layout_document(
                    &result.document,
                    &mut amipro_layout::FixedShaper,
                    &mut amipro_layout::ShapeCache::default(),
                );
                let elapsed = started.elapsed();
                layout_time += elapsed;
                if elapsed > slowest.0 {
                    slowest = (elapsed, path.display().to_string());
                }
                laid_out_pages += layout.pages.len();
                if layout.pages.len() > most_pages.0 {
                    most_pages = (layout.pages.len(), path.display().to_string());
                }
                style_resolved += usize::from(result.report.associated_style_resolved);
                style_declared += usize::from(result.report.associated_style.is_some());
                for control in result.report.unsupported_controls {
                    *controls.entry(control).or_insert(0usize) += 1;
                }
                sections.extend(result.report.unsupported_sections);
            }
            Err(error) => failed.push(format!("{}: {error}", path.display())),
        }
    }
    let mut style_parsed = 0usize;
    let (mut records_compared, mut records_equal) = (0usize, 0usize);
    let mut record_differences = Vec::new();
    let mut style_failed = Vec::new();
    for path in files.iter().filter(|path| extension_is(path, "sty")) {
        match fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|data| {
                parse(&data, LegacyKind::StyleSheet).map_err(|error| error.to_string())
            }) {
            Ok(_) => style_parsed += 1,
            Err(error) => style_failed.push(format!("{}: {error}", path.display())),
        }
        // The STY writer (decision D3): each style record written back.
        if let Ok(data) = fs::read(path)
            && let Ok(sheet) = read_style_sheet(&data, "sheet")
        {
            let raw = raw_style_records(&data);
            let styles: BTreeMap<_, _> = sheet
                .styles
                .iter()
                .map(|style| (style.id, style.clone()))
                .collect();
            for style in &sheet.styles {
                let Some(theirs) = raw.get(&escape_text(&style.name)) else {
                    continue;
                };
                records_compared += 1;
                let ours = style_record(style, &styles);
                if &ours == theirs {
                    records_equal += 1;
                } else if record_differences.len() < 20 {
                    let ours = String::from_utf8_lossy(&ours).into_owned();
                    let theirs = String::from_utf8_lossy(theirs).into_owned();
                    let line = ours
                        .lines()
                        .zip(theirs.lines())
                        .position(|(a, b)| a != b)
                        .unwrap_or_else(|| ours.lines().count().min(theirs.lines().count()));
                    record_differences.push(format!(
                        "{}: {:?} line {line} of {} and {}: ours {:?} theirs {:?}",
                        path.display(),
                        style.name,
                        ours.lines().count(),
                        theirs.lines().count(),
                        ours.lines().nth(line).unwrap_or(""),
                        theirs.lines().nth(line).unwrap_or("")
                    ));
                }
            }
        }
    }

    // The STY writer's page layouts (decision D3, part 2): every page
    // layout written back from the model and compared with the file's,
    // its frames' text left empty as a sheet without contents writes it.
    let (mut layouts_compared, mut layouts_equal) = (0usize, 0usize);
    let mut layout_differences: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let (mut texts_compared, mut texts_equal) = (0usize, 0usize);
    let mut text_differences = Vec::new();
    for path in &files {
        let Ok(data) = fs::read(path) else { continue };
        let theirs = raw_layouts(&data);
        if theirs.is_empty() {
            continue;
        }
        let mut text = data[..data
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(data.len())]
            .to_vec();
        if !String::from_utf8_lossy(&text)
            .lines()
            .any(|line| line.trim() == "[edoc]")
        {
            text.extend_from_slice(b"\r\n[edoc]\r\n>\r\n");
        }
        let Ok(imported) = import_sam(&text, None, "layout") else {
            continue;
        };
        for (layout, theirs) in imported.document.layouts().iter().zip(&theirs) {
            layouts_compared += 1;
            let ours = layout_record(&imported.document, layout);
            // Read back: every frame's paragraphs as the model had them.
            let mut written = b"[ver]\r\n\t4\r\n".to_vec();
            written.extend_from_slice(&ours);
            written.extend_from_slice(b"[elay]\r\n[edoc]\r\n>\r\n");
            match import_sam(&written, None, "written") {
                Ok(reread) => {
                    let before = frame_paragraphs(&imported.document, layout);
                    let after = reread
                        .document
                        .layouts()
                        .first()
                        .map(|layout| frame_paragraphs(&reread.document, layout))
                        .unwrap_or_default();
                    for (index, before) in before.iter().enumerate() {
                        let Some(before) = before else { continue };
                        texts_compared += 1;
                        let after = after.get(index).cloned().flatten().unwrap_or_default();
                        if *before == after {
                            texts_equal += 1;
                            continue;
                        }
                        if text_differences.len() >= 10 {
                            continue;
                        }
                        let at = before
                            .char_indices()
                            .zip(after.chars())
                            .find(|((_, a), b)| a != b)
                            .map_or(before.len().min(after.len()), |((at, _), _)| at);
                        let from = before[..at]
                            .char_indices()
                            .rev()
                            .nth(80)
                            .map_or(0, |(at, _)| at);
                        let before_tail: String = before[from..].chars().take(240).collect();
                        let after_tail: String =
                            after.get(from..).unwrap_or("").chars().take(240).collect();
                        text_differences.push(format!(
                            "{}: …{before_tail:?} became …{after_tail:?}",
                            path.display()
                        ));
                    }
                }
                Err(error) => {
                    text_differences.push(format!("{}: reread failed: {error}", path.display()))
                }
            }
            if &ours == theirs {
                layouts_equal += 1;
                continue;
            }
            let ours = String::from_utf8_lossy(&ours).into_owned();
            let theirs = String::from_utf8_lossy(theirs).into_owned();
            let ours_lines: Vec<&str> = ours.lines().collect();
            let theirs_lines: Vec<&str> = theirs.lines().collect();
            let line = ours_lines
                .iter()
                .zip(&theirs_lines)
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| ours_lines.len().min(theirs_lines.len()));
            // The section the first difference is in, and its value index.
            let section = theirs_lines[..line.min(theirs_lines.len())]
                .iter()
                .rposition(|line| line.trim().starts_with('['))
                .map(|start| format!("{} +{}", theirs_lines[start].trim(), line - start))
                .unwrap_or_default();
            let entry = layout_differences
                .entry(section)
                .or_insert((0, String::new()));
            entry.0 += 1;
            if entry.1.is_empty() {
                entry.1 = format!(
                    "{}: line {line} of {} and {}: ours {:?} theirs {:?}",
                    path.display(),
                    ours_lines.len(),
                    theirs_lines.len(),
                    ours_lines.get(line).unwrap_or(&""),
                    theirs_lines.get(line).unwrap_or(&"")
                );
            }
        }
    }

    // Whole sheets (Save as a Style Sheet without contents): each STY read
    // as a document, written as a sheet, and read again as a sheet.
    let (mut sheets_compared, mut sheets_equal) = (0usize, 0usize);
    let mut sheet_differences = Vec::new();
    for path in files.iter().filter(|path| extension_is(path, "sty")) {
        let Ok(data) = fs::read(path) else { continue };
        let Ok(theirs) = read_style_sheet(&data, "sheet") else {
            continue;
        };
        let mut text = data[..data
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(data.len())]
            .to_vec();
        if !String::from_utf8_lossy(&text)
            .lines()
            .any(|line| line.trim() == "[edoc]")
        {
            text.extend_from_slice(b"\r\n[edoc]\r\n>\r\n");
        }
        let Ok(document) = import_sam(&text, None, "sheet") else {
            continue;
        };
        let written = style_sheet(
            &document.document,
            &SheetOptions {
                contents: false,
                description: "Written".into(),
                saved_at: 1_800_000_000,
            },
        );
        sheets_compared += 1;
        match read_style_sheet(&written, "sheet") {
            Ok(ours) => {
                let styles = |sheet: &amipro_document::StyleSheet| {
                    let mut styles: Vec<String> = sheet
                        .styles
                        .iter()
                        .map(|style| {
                            let mut style = style.clone();
                            style.id = amipro_document::StyleId(0);
                            style.next = None;
                            format!("{style:?}")
                        })
                        .collect();
                    styles.sort();
                    styles
                };
                let layout = |sheet: &amipro_document::StyleSheet| {
                    sheet.layout.as_ref().map(|layout| {
                        format!(
                            "{:?}",
                            (
                                &layout.name,
                                layout.paper,
                                layout.landscape,
                                &layout.right,
                                &layout.left,
                                layout.mirror
                            )
                        )
                    })
                };
                if styles(&ours) == styles(&theirs) && layout(&ours) == layout(&theirs) {
                    sheets_equal += 1;
                } else if sheet_differences.len() < 10 {
                    sheet_differences.push(format!(
                        "{}: styles {} layout {}",
                        path.display(),
                        if styles(&ours) == styles(&theirs) {
                            "equal"
                        } else {
                            "differ"
                        },
                        if layout(&ours) == layout(&theirs) {
                            "equal"
                        } else {
                            "differs"
                        }
                    ));
                }
            }
            Err(error) => sheet_differences.push(format!("{}: {error}", path.display())),
        }
    }

    println!("SAM files: {}", imported + failed.len());
    println!("Imported: {imported}");
    println!("Failed: {}", failed.len());
    println!("Exact native round trips: {native_round_trips}");
    println!(
        "Native round-trip failures: {}",
        native_round_trip_failures.len()
    );
    println!("Paragraphs: {paragraphs}");
    println!("Body Text paragraphs: {body_text_paragraphs}");
    println!(
        "Paragraphs with control-like text: {} (a heuristic; literal `<<` text matches it)",
        control_like_paragraphs.len()
    );
    println!("Control leaks: {}", control_leaks.len());
    for leak in &control_leaks {
        println!("LEAK {leak}");
    }
    println!("Documents with page or repeating frames: {documents_with_frames}");
    println!("Frames: {}", frames_by_kind.values().sum::<usize>());
    for (kind, count) in &frames_by_kind {
        println!("FRAMES {count}: {kind}");
    }
    for (kind, count) in &inline_objects {
        println!("OBJECTS {count}: {kind}");
    }
    println!(
        "Layouts per document: {}",
        layouts_per_document
            .iter()
            .map(|(layouts, documents)| format!("{layouts} in {documents}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("Anchors resolved: {anchors_resolved}");
    println!("Unresolved anchors: {}", anchors_unresolved.len());
    for document in &anchors_unresolved {
        println!("UNRESOLVED {document}");
    }
    println!("Anchored frames without a mark: {anchored_without_mark}");
    println!("Unknown frame flag bits: {unknown_frame_flags:#x}");
    println!("Pages with non-uniform stored columns: {nonuniform_page_columns}");
    println!("Legacy marks kept: {legacy_marks}");
    println!("Caches dropped: {dropped_caches}");
    println!("Embedded payloads: {embedded_objects}");
    println!(
        "Layout with the fixed shaper: {laid_out_pages} pages in {layout_time:?}; most: {} ({}); slowest: {:?} ({})",
        most_pages.0, most_pages.1, slowest.0, slowest.1
    );
    for example in control_like_paragraphs.iter().take(10) {
        println!("CONTROL-LIKE {example}");
    }
    println!("Documents with character runs: {documents_with_character_runs}");
    println!("Character runs: {character_runs}");
    println!("Associated STY declared: {style_declared}");
    println!("Associated STY resolved: {style_resolved}");
    println!("Unsupported section kinds: {}", sections.len());
    println!(
        "Unsupported sections: {}",
        sections.into_iter().collect::<Vec<_>>().join(", ")
    );
    println!("Distinct unsupported controls: {}", controls.len());
    let mut frequent_controls: Vec<_> = controls.into_iter().collect();
    frequent_controls
        .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (control, documents) in frequent_controls.into_iter().take(20) {
        println!("CONTROL {documents} documents: {control:?}");
    }
    println!("STY files: {}", style_parsed + style_failed.len());
    println!("STY parsed: {style_parsed}");
    println!("STY failed: {}", style_failed.len());
    println!("STY style records written back byte for byte: {records_equal} of {records_compared}");
    for difference in &record_differences {
        println!("RECORD DIFF {difference}");
    }
    println!(
        "Style sheets written that read back as their source: {sheets_equal} of {sheets_compared}"
    );
    for difference in &sheet_differences {
        println!("SHEET DIFF {difference}");
    }
    println!("Page layouts written back byte for byte: {layouts_equal} of {layouts_compared}");
    println!(
        "Layout frames whose text reads back as the model had it: {texts_equal} of {texts_compared}"
    );
    for difference in &text_differences {
        println!("TEXT DIFF {difference}");
    }
    for (section, (count, example)) in &layout_differences {
        println!("LAYOUT DIFF {count} at {section}: {example}");
    }
    for failure in failed {
        println!("FAIL {failure}");
    }
    for failure in native_round_trip_failures {
        println!("NATIVE FAIL {failure}");
    }
    for failure in style_failed {
        println!("FAIL {failure}");
    }
}

fn visit(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn extension_is(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

/// True when text contains `<` directly followed by a character that starts
/// SAM controls, such as `<:`, `<+`, `</`, or `<;`.
fn looks_like_control(text: &str) -> bool {
    text.match_indices('<').any(|(index, _)| {
        text[index + 1..]
            .chars()
            .next()
            .is_some_and(|next| ":+-/\\*;[@!\"#$%".contains(next))
    })
}

/// A style sheet's `[tag]` records as written, by their name line (the
/// escaped name), lines ending in CR LF: from `[tag]` to the next section
/// that is not inside it, up to the first NUL byte.
fn raw_style_records(data: &[u8]) -> BTreeMap<Vec<u8>, Vec<u8>> {
    let end = data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(data.len());
    let lines: Vec<&[u8]> = data[..end]
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .collect();
    let mut records = BTreeMap::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index] != b"[tag]" {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < lines.len() && !lines[index].starts_with(b"[") {
            index += 1;
        }
        let mut record = Vec::new();
        for line in &lines[start..index] {
            record.extend_from_slice(line);
            record.extend_from_slice(b"\r\n");
        }
        if let Some(name) = lines.get(start + 1) {
            let name = name.strip_prefix(b"\t").unwrap_or(name).to_vec();
            records.entry(name).or_insert(record);
        }
    }
    records
}

/// Each `[lay]` block of a file as written, up to the next layout or
/// `[elay]`, as a sheet without contents writes it: the text of frames with
/// flag 0x8 emptied, and Ami Pro's layout caches (`<:#…>`, which the model
/// does not keep) removed from the rest.
fn raw_layouts(data: &[u8]) -> Vec<Vec<u8>> {
    let end = data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(data.len());
    let mut layouts = Vec::new();
    let mut current: Option<Vec<u8>> = None;
    // The text being read: whether it is left out, and the control depth.
    let mut text: Option<bool> = None;
    let mut depth = 0usize;
    let mut flags_in = None;
    let mut flags = 0u32;
    for line in data[..end].split_inclusive(|byte| *byte == b'\n') {
        let trimmed = line.strip_suffix(b"\r\n").unwrap_or(line);
        if current.is_none() && trimmed == b"[lay]" {
            current = Some(Vec::new());
        } else if let Some(layout) = current.as_mut() {
            if let Some(empty) = text {
                if depth == 0 && trimmed == b">" {
                    text = None;
                    layout.extend_from_slice(line);
                    continue;
                }
                let mut chars = trimmed.iter().peekable();
                while let Some(byte) = chars.next() {
                    match byte {
                        b'<' if chars.peek() == Some(&&b'<') => {
                            chars.next();
                        }
                        b'<' => depth += 1,
                        b'>' => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                }
                if !empty {
                    layout.extend(without_caches(line));
                }
                continue;
            }
            if trimmed == b"[lay]" || trimmed == b"[elay]" || trimmed == b"[edoc]" {
                layouts.push(current.take().unwrap_or_default());
                if trimmed == b"[lay]" {
                    current = Some(Vec::new());
                } else {
                    break;
                }
            } else if trimmed == b"\t[txt]" {
                text = Some(flags & 0x8 != 0);
                depth = 0;
            } else if trimmed == b"\t[lyfrm]" {
                flags_in = Some(2);
            } else if let Some(left) = flags_in {
                if left == 1 {
                    flags = String::from_utf8_lossy(trimmed).trim().parse().unwrap_or(0);
                    flags_in = None;
                } else {
                    flags_in = Some(left - 1);
                }
            }
        }
        if let Some(layout) = current.as_mut() {
            layout.extend_from_slice(line);
        }
    }
    layouts.extend(current);
    layouts
}

/// A line without `<:#HEIGHT,WIDTH>` layout caches.
fn without_caches(line: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(line.len());
    let mut index = 0;
    while index < line.len() {
        if line[index..].starts_with(b"<:#")
            && let Some(close) = line[index..].iter().position(|byte| *byte == b'>')
            && line[index + 3..index + close]
                .iter()
                .all(|byte| byte.is_ascii_digit() || *byte == b',' || *byte == b'-')
        {
            index += close + 1;
            continue;
        }
        out.push(line[index]);
        index += 1;
    }
    out
}

/// A layout's frames' text, paragraph by paragraph, with style names
/// instead of IDs and adjacent runs of one format joined, for comparing a
/// written layout read back. Frames whose text a sheet without contents
/// leaves out (flag 0x8), and pictures (not written yet), are `None`.
fn frame_paragraphs(
    document: &Document,
    layout: &amipro_document::PageLayout,
) -> Vec<Option<String>> {
    layout
        .frames()
        .filter_map(|id| document.frame(id))
        .map(|frame| {
            let left_out = frame
                .legacy
                .get("frame.flags.unknown")
                .and_then(|flags| u32::from_str_radix(flags.trim_start_matches("0x"), 16).ok())
                .is_some_and(|flags| flags & 0x8 != 0);
            let FrameContent::Text(flow) = frame.content else {
                return None;
            };
            if left_out {
                return None;
            }
            let flow = document.flow(flow)?;
            Some(
                flow.paragraphs
                    .iter()
                    .map(|paragraph| {
                        let mut runs: Vec<amipro_document::CharacterRun> = Vec::new();
                        for run in &paragraph.runs {
                            match runs.last_mut() {
                                Some(last)
                                    if last.end == run.start && last.format == run.format =>
                                {
                                    last.end = run.end;
                                }
                                _ => runs.push(run.clone()),
                            }
                        }
                        format!(
                            "{:?} {:?} {:?} {:?} {:?} {:?} {:?}",
                            document
                                .styles()
                                .get(&paragraph.style)
                                .map(|style| &style.name),
                            paragraph.text,
                            paragraph.format,
                            runs,
                            paragraph.objects,
                            paragraph.marks,
                            paragraph.end_format
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        })
        .collect()
}
