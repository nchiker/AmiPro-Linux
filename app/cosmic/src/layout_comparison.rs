//! Phase 2.5d: the layout engine with COSMIC Text against Ami Pro's own
//! layout caches (`docs/research/LAYOUT_COMPARISON.md`). The report is an
//! ignored test over private files: `AMIPRO_VM_LAB` (the VM fixtures) and
//! `AMIPRO_CORPUS` (the extracted corpus), each searched for SAM files, with
//! `AMIPRO_STYLES` as the style-sheet path.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use amipro_document::{
    Document, FrameId, LineHeights, LineSpacing, Paragraph, ParagraphStyle, TabStop,
};
use amipro_formats::import_sam_path_with_style_directories;
use amipro_formats::oracle::{LayoutOracle, read_layout_oracle};
use amipro_layout::{
    Bounds, DocumentLayout, FrameBox, FrameBoxKind, ShapeRequest, Shaper, TablePart, bullet_box,
    next_line, paragraph_numbers, size_tabs, tab_stops,
};

use crate::page_layout::{SessionPages, substitute_family, with_shaper};

fn sam_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
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

/// The lines of `paragraph` as the engine breaks them in a column
/// `column` twips wide with tab stops `tabs`, the first line `first` twips
/// wide and the others `rest`: their count and total height.
#[allow(clippy::too_many_arguments)]
fn indented_paragraph_lines(
    shaper: &mut dyn Shaper,
    paragraph: &Paragraph,
    style: &ParagraphStyle,
    line_heights: &LineHeights,
    column: f32,
    tabs: &[TabStop],
    first: f32,
    rest: f32,
    number: amipro_layout::ParagraphNumber,
    zone: u16,
) -> (usize, f32) {
    let shaped = shaper.shape(ShapeRequest {
        text: &paragraph.text,
        style,
        runs: &paragraph.runs,
        end_format: &paragraph.end_format,
        line_heights,
        language: "en",
    });
    // The paragraph's own formatting over its style, its tab ruler too.
    let mut effective = style.clone();
    paragraph.format.apply(&mut effective);
    let indents = effective.indents;
    let stops = tab_stops(&effective, indents, tabs, column);
    // The first line's text follows its bullet or number.
    // It stands at the left indent: the text moves only by what the
    // first-line indent does not leave for it.
    let bullet = bullet_box(&effective.bullets, number, &effective, shaper)
        .map_or(0.0, |bullet| (bullet.width - indents.first as f32).max(0.0));
    let mut words = shaped.words.clone();
    let bold = shaped.first_line_bold().map(|bold| bold.words);
    let (mut lines, mut height) = (0, 0.0);
    let mut start = 0;
    while start < words.len() {
        if let Some(bold) = bold.as_ref().filter(|_| start == 0) {
            words.clone_from(bold);
        }
        let width = if lines == 0 {
            (first - bullet).max(1.0)
        } else {
            rest
        };
        let origin = column - width - indents.right.max(0) as f32;
        size_tabs(&mut words, start, origin, width, &stops, &paragraph.text);
        let zone = f32::from(zone) * shaped.average_width;
        let end = next_line(&words, start, width, shaped.hyphen_width, zone, |_| false).end;
        if bold.is_some() && start == 0 {
            words[end..].clone_from_slice(&shaped.words[end..]);
        }
        // As the engine: the line's characters, or the empty line's font
        // when nothing on it is measured (decompile `10d0:359b`).
        let measured = words[start..end]
            .iter()
            .filter(|word| word.ascent > 0.0 || word.descent > 0.0);
        let ascent = measured.clone().map(|word| word.ascent).fold(0.0, f32::max);
        let descent = measured.map(|word| word.descent).fold(0.0, f32::max);
        height += if ascent + descent > 0.0 {
            ascent + descent
        } else {
            shaped.ascent + shaped.descent
        };
        lines += 1;
        start = end;
    }
    if lines == 0 {
        (1, shaped.ascent + shaped.descent)
    } else {
        (lines, height)
    }
}

#[derive(Default)]
struct Comparison {
    name: String,
    paragraphs: usize,
    oracle_pages: Option<u32>,
    our_pages: usize,
    /// Cached paragraphs compared, equal line counts, within one line.
    lines: (usize, usize, usize),
    /// Compared paragraphs whose imported indents differ from their style's
    /// stored ones (should be none).
    indents_differ: usize,
    /// Paragraphs with equal line counts: summed and largest relative
    /// difference of the line height.
    heights: (f64, f64),
    /// The same split by the style's spacing: single, one and a half,
    /// double, or custom (`[spc]` flags), and styles neither the SAM nor a
    /// found STY defines, each counted and summed.
    by_mode: [(usize, f64); 3],
    /// Single-spaced paragraphs with equal line counts by the fonts their
    /// characters are set in: one font with a printer line height, one
    /// without, several all with one, several with one or more without;
    /// each counted and summed.
    by_fonts: [(usize, f64); 4],
    /// Page ends compared, on the same page, largest page difference.
    page_ends: (usize, usize, i64),
    frames: FrameStats,
    tables: TableStats,
    /// Paragraph caches or page ends that point past our paragraphs.
    out_of_range: usize,
    frames_mismatched: bool,
}

/// Ami Pro's page ends found in our layout: compared, on the same page, the
/// largest page difference, and page ends past our paragraphs.
fn page_ends(
    document: &Document,
    oracle: &LayoutOracle,
    layout: &DocumentLayout,
) -> (usize, usize, i64, usize) {
    let (mut compared, mut same, mut worst, mut missing) = (0, 0, 0i64, 0);
    let last = oracle.pages.len().saturating_sub(1);
    for (index, end) in oracle.pages.iter().enumerate().take(last) {
        let Some((paragraph_index, paragraph)) = end.paragraph.checked_sub(1).and_then(|index| {
            document
                .paragraphs()
                .get(index)
                .map(|paragraph| (index, paragraph))
        }) else {
            missing += 1;
            continue;
        };
        // Internal offsets count controls as several characters; the
        // character offset is close enough to find the line.
        let byte = paragraph
            .text
            .char_indices()
            .nth(end.line_start as usize)
            .map_or(paragraph.text.len(), |(byte, _)| byte);
        let Some(ours) = layout.page_of(paragraph_index, byte) else {
            continue;
        };
        let difference = i64::from(ours) - (index as i64 + 1);
        compared += 1;
        same += usize::from(difference == 0);
        if difference.abs() > worst.abs() {
            worst = difference;
        }
    }
    (compared, same, worst, missing)
}

/// The single-line height of each style in this document: the exact value
/// for exact spacing; otherwise the largest height, at least the font size
/// (and at most 1.6 times it for single spacing), of which every cached
/// height of the style is a multiple. The smallest cached height alone is
/// one line only when the style has a one-line paragraph.
fn line_heights(document: &Document, oracle: &LayoutOracle) -> BTreeMap<String, u32> {
    let mut cached: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for cache in &oracle.paragraphs {
        // A paragraph's own line spacing (`<:S…>`) is not its style's.
        if let Some(paragraph) = document
            .paragraphs()
            .get(cache.paragraph)
            .filter(|paragraph| paragraph.format.line_spacing.is_none())
        {
            // Without the space the style's paragraph rules take.
            let style = &document.styles()[&paragraph.style];
            cached
                .entry(style.name.to_lowercase())
                .or_default()
                .push(cache.height.saturating_sub(rule_space(style)));
        }
    }
    let mut heights = BTreeMap::new();
    for (name, cached) in cached {
        let spacing = oracle
            .styles
            .iter()
            .find(|style| style.name.to_lowercase() == name);
        if let Some(spacing) = spacing.filter(|style| style.flags & 0xE != 0 && style.line > 0) {
            heights.insert(name, spacing.line);
            continue;
        }
        let Some(smallest) = cached.iter().copied().min().filter(|height| *height > 0) else {
            continue;
        };
        let size = document
            .styles()
            .values()
            .find(|style| style.name.to_lowercase() == name)
            .map_or(1, |style| (style.font_size_points * 20.0) as u32)
            .max(1);
        let ceiling = match spacing {
            Some(spacing) if spacing.flags & 0xE == 0 => size * 8 / 5,
            _ => u32::MAX,
        };
        let line = (1..=smallest / size)
            .map(|lines| (smallest + lines / 2) / lines)
            .filter(|line| *line <= ceiling)
            .find(|line| {
                cached
                    .iter()
                    .all(|height| lines_in(*height, *line).is_some())
            })
            .unwrap_or(smallest);
        heights.insert(name, line);
    }
    heights
}

/// The height a style's paragraph rules add to a paragraph.
fn rule_space(style: &ParagraphStyle) -> u32 {
    let (above, below) = style.rules.space();
    (above + below).max(0) as u32
}

/// How many lines of `line` twips make `height`, allowing two twips plus
/// half a twip per line of rounding.
fn lines_in(height: u32, line: u32) -> Option<usize> {
    let lines = (height + line / 2) / line;
    (lines > 0 && height.abs_diff(lines * line) <= 2 + lines / 2).then_some(lines as usize)
}

/// Whether `AMIPRO_COMPARE_DETAIL` names this file.
fn detail_requested(path: &Path) -> bool {
    std::env::var("AMIPRO_COMPARE_DETAIL").is_ok_and(|name| {
        path.file_name()
            .is_some_and(|file| file.eq_ignore_ascii_case(&name))
    })
}

/// One style font family over a corpus: its substitute, the paragraphs
/// compared, those with equal line counts, and their summed height
/// difference.
#[derive(Default)]
struct FamilyStats {
    substitute: String,
    compared: usize,
    equal: usize,
    heights: f64,
}

fn compare(path: &Path, families: &mut BTreeMap<String, FamilyStats>) -> Option<Comparison> {
    let bytes = std::fs::read(path).ok()?;
    let mut oracle = read_layout_oracle(&bytes);
    let style_directories: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default();
    let imported = import_sam_path_with_style_directories(path, &style_directories).ok()?;
    crate::page_layout::register_face_kinds(imported.document.face_kinds());
    // Styles the SAM does not define come from its STY, as on import.
    if let Some(sheet) = imported
        .report
        .associated_style_path
        .as_deref()
        .and_then(|sheet| std::fs::read(sheet).ok())
    {
        for style in read_layout_oracle(&sheet).styles {
            if !oracle
                .styles
                .iter()
                .any(|known| known.name.eq_ignore_ascii_case(&style.name))
            {
                oracle.styles.push(style);
            }
        }
    }
    let document = imported.document;
    let mut pages = SessionPages::default();
    // Dates as Ami Pro last laid the file out, when it was last saved and
    // its caches written (the fields run).
    let info = document.info();
    let laid_out = info.saved.or(info.created).unwrap_or(0);
    pages.set_field_context(amipro_document::FieldContext {
        now: amipro_document::WallClock::from_ami_pro(laid_out),
        file_name: path.file_name()?.to_string_lossy().into_owned(),
        directory: path
            .parent()
            .map(|folder| folder.display().to_string())
            .unwrap_or_default(),
        ..amipro_document::FieldContext::default()
    });
    pages.update(&document);
    let layout: &DocumentLayout = pages.layout();
    let mut comparison = Comparison {
        name: path.file_name()?.to_string_lossy().into_owned(),
        paragraphs: document.paragraphs().len(),
        oracle_pages: oracle.page_count,
        our_pages: layout.pages.len(),
        ..Comparison::default()
    };
    for style in document.styles().values() {
        families
            .entry(style.font_family.clone())
            .or_insert_with(|| FamilyStats {
                substitute: substitute_family(&style.font_family),
                ..FamilyStats::default()
            });
    }

    // Line counts at Ami Pro's own text width: shaping and breaking alone,
    // with the document layout's first-column tabs.
    let heights = line_heights(&document, &oracle);
    let layout_tabs = document
        .document_layout()
        .right
        .layout
        .tabs
        .first()
        .cloned()
        .unwrap_or_default();
    let tabs = layout_tabs.as_slice();
    let numbers = paragraph_numbers(document.paragraphs(), document.styles());
    with_shaper(|shaper| {
        for cache in &oracle.paragraphs {
            let Some(paragraph) = document.paragraphs().get(cache.paragraph) else {
                comparison.out_of_range += 1;
                continue;
            };
            let style = &document.styles()[&paragraph.style];
            let line = match paragraph.format.line_spacing {
                None => heights.get(&style.name.to_lowercase()).copied(),
                Some(LineSpacing::Custom(height)) => u32::try_from(height).ok(),
                Some(_) => None,
            };
            let Some(line) = line else {
                continue;
            };
            // Ami Pro's cache includes the space of the style's rules.
            let rules = rule_space(style);
            let Some(theirs) =
                lines_in(cache.height.saturating_sub(rules), line).filter(|_| line > 0)
            else {
                continue;
            };
            // The cache's width is the column; the paragraph's indents (its
            // own, or its style's) narrow it as the engine does (I1, O3).
            let width = cache.width as f32;
            let indents = paragraph.format.indents.unwrap_or(style.indents);
            let (ours, height) = indented_paragraph_lines(
                shaper,
                paragraph,
                style,
                document.line_heights(),
                width,
                tabs,
                width - (indents.left(true) + indents.right) as f32,
                width - (indents.left(false) + indents.right) as f32,
                numbers.get(cache.paragraph).copied().unwrap_or_default(),
                document.hyphenation_zone(),
            );
            let height = height + rules as f32;
            comparison.lines.0 += 1;
            comparison.lines.1 += usize::from(ours == theirs);
            comparison.lines.2 += usize::from(ours.abs_diff(theirs) <= 1);
            // The model's indents should be the cache's own style's.
            if let Some(spacing) = oracle
                .styles
                .iter()
                .find(|spacing| spacing.name.eq_ignore_ascii_case(&style.name))
            {
                let [all, first, rest] = spacing.indents;
                let stored = (all, first, rest, spacing.right_indent);
                let mapped = (
                    style.indents.all as u32,
                    style.indents.first as u32,
                    style.indents.rest as u32,
                    style.indents.right as u32,
                );
                comparison.indents_differ += usize::from(stored != mapped);
            }
            let family = families
                .get_mut(&style.font_family)
                .expect("every style's family");
            family.compared += 1;
            family.equal += usize::from(ours == theirs);
            if ours == theirs {
                family.heights += (f64::from(height) / f64::from(cache.height) - 1.0).abs();
            }
            if ours != theirs && detail_requested(path) {
                let excerpt: String = paragraph.text.chars().take(50).collect();
                eprintln!(
                    "  lines {} {} {}pt at {}: Ami Pro {theirs} ours {ours}: {excerpt:?}",
                    style.name, style.font_family, style.font_size_points, cache.width
                );
            }
            if ours == theirs {
                let difference = (f64::from(height) / f64::from(cache.height) - 1.0).abs();
                comparison.heights.0 += difference;
                comparison.heights.1 = comparison.heights.1.max(difference);
                let bucket = match oracle
                    .styles
                    .iter()
                    .find(|spacing| spacing.name.eq_ignore_ascii_case(&style.name))
                {
                    Some(spacing) if spacing.flags & 0xE == 0 => 0,
                    Some(_) => 1,
                    None => 2,
                };
                comparison.by_mode[bucket].0 += 1;
                comparison.by_mode[bucket].1 += difference;
                if bucket == 0 {
                    let fonts = paragraph.fonts(style);
                    let calibrated = fonts
                        .iter()
                        .filter(|key| document.line_heights().get(key).is_some())
                        .count();
                    let kind = match (fonts.len() > 1, calibrated == fonts.len()) {
                        (false, true) => 0,
                        (false, false) => 1,
                        (true, true) => 2,
                        (true, false) => 3,
                    };
                    comparison.by_fonts[kind].0 += 1;
                    comparison.by_fonts[kind].1 += difference;
                }
                if detail_requested(path) {
                    eprintln!(
                        "  height {} {} {}pt: Ami Pro {} ours {:.1} ({} lines)",
                        style.name,
                        style.font_family,
                        style.font_size_points,
                        cache.height,
                        height,
                        ours
                    );
                }
            }
        }
    });

    let (compared, same, worst, missing) = page_ends(&document, &oracle, layout);
    comparison.page_ends = (compared, same, worst);
    comparison.out_of_range += missing;

    if std::env::var("AMIPRO_COMPARE_DETAIL")
        .is_ok_and(|name| name.eq_ignore_ascii_case(&comparison.name))
    {
        detail(&document, &oracle, layout);
    } else if std::env::var("AMIPRO_COMPARE_DETAIL_TABLES").is_ok_and(|name| name == "*") {
        // Every file's table rows that differ (`*`).
        eprintln!("  file {}", comparison.name);
        table_detail(&document, &oracle, layout);
    }

    // Frames: top-level frames in file order are the frame stack's order.
    if document.frame_stack().len() != oracle.frames.len() {
        comparison.frames_mismatched = !oracle.frames.is_empty();
        return Some(comparison);
    }
    comparison.frames = frames(&document, &oracle, layout);
    comparison.tables = tables(&document, &oracle, layout);
    // The rows the row cache keeps: equal with it, not without it.
    let mut uncached = document.clone();
    let ids: Vec<FrameId> = uncached.frames().map(|frame| frame.id).collect();
    let mut any = false;
    for id in ids {
        if let Some(frame) = uncached.frame_mut(id)
            && let amipro_document::FrameContent::Table(table) = &mut frame.content
            && !table.read_rows.is_empty()
        {
            table.read_rows.clear();
            any = true;
        }
    }
    if any {
        let mut plain = SessionPages::default();
        plain.update(&uncached);
        let without = tables(&uncached, &oracle, plain.layout());
        comparison.tables.cached = comparison.tables.rows.1.saturating_sub(without.rows.1);
    }
    Some(comparison)
}

/// Frame geometry against the stored rectangles and column edges.
#[derive(Clone, Copy, Debug, Default)]
struct FrameStats {
    /// Anchored frames compared, largest |dy|, |dx|, and size difference,
    /// and how many are on the stored page.
    anchored: (usize, f32, f32, f32, usize),
    /// Text frames whose stored column edges were compared, and the
    /// largest difference. Picture and table frames are left out: their
    /// stored edges are stale (TB1, README31).
    columns: (usize, f32),
    /// Frames in table cells compared: how many are on the stored page,
    /// how many lie where their stored rectangle puts them within their
    /// table's (to a twip; the grid, apart from where the table goes), and
    /// the largest difference there.
    cells: (usize, usize, usize, f32),
}

/// Tables against Ami Pro's caches (`TABLE_PAGES_NOTES.md`).
#[derive(Clone, Copy, Default)]
struct TableStats {
    /// Page ends in a table (Ami Pro's page table names the first row and
    /// the row count of the table's part on the page): compared, and with
    /// the same rows here.
    pages: (usize, usize),
    /// Rows of Automatic tables with Ami Pro's stored height, laid out whole
    /// here: compared, equal (to a twip), within 15 twips, and the summed
    /// absolute difference.
    rows: (usize, usize, usize, f64),
    /// Of the equal rows, those equal only through the row cache: laid out
    /// again without it, they differ.
    cached: usize,
}

/// The table anchored last in main-text paragraph `index`.
fn table_in(document: &Document, index: usize) -> Option<FrameId> {
    document
        .paragraphs()
        .get(index)?
        .objects
        .iter()
        .rev()
        .find_map(|object| match object {
            amipro_document::InlineObject::FrameAnchor(frame)
                if document.frame(*frame).is_some_and(|frame| {
                    matches!(frame.content, amipro_document::FrameContent::Table(_))
                }) =>
            {
                Some(*frame)
            }
            _ => None,
        })
}

/// The parts of `table` on page `index` (0-based), in the order laid out.
fn table_parts(layout: &DocumentLayout, index: usize, table: FrameId) -> Vec<&TablePart> {
    layout
        .pages
        .get(index)
        .into_iter()
        .flat_map(|page| &page.frames)
        .filter(|frame_box| frame_box.frame == table)
        .filter_map(|frame_box| match &frame_box.kind {
            FrameBoxKind::Table { part, .. } => Some(part),
            _ => None,
        })
        .collect()
}

fn tables(document: &Document, oracle: &LayoutOracle, layout: &DocumentLayout) -> TableStats {
    let mut stats = TableStats::default();
    for (index, end) in oracle.pages.iter().enumerate() {
        let Some((first, count)) = end.table_rows else {
            continue;
        };
        let Some(table) = end
            .paragraph
            .checked_sub(1)
            .and_then(|p| table_in(document, p))
        else {
            continue;
        };
        stats.pages.0 += 1;
        let same = table_parts(layout, index, table)
            .last()
            .is_some_and(|part| {
                (part.rows.start, part.rows.len()) == (first as usize, count as usize)
            });
        stats.pages.1 += usize::from(same);
    }
    for (cache, frame_id) in oracle.frames.iter().zip(document.frame_stack()) {
        if cache.table_rows.is_empty() || !cache.automatic || !cache.is_anchored() {
            continue;
        }
        // Each row's height where a part holds it whole.
        let mut ours: BTreeMap<usize, f32> = BTreeMap::new();
        for page in 0..layout.pages.len() {
            for part in table_parts(layout, page, *frame_id) {
                for (offset, row) in part.rows.clone().enumerate() {
                    let split =
                        (offset == 0 && part.continued()) || (part.cut && row + 1 == part.rows.end);
                    if !split && let Some(height) = part.heights.get(offset) {
                        ours.insert(row, *height);
                    }
                }
            }
        }
        for (row, stored) in cache.table_rows.iter().enumerate() {
            let Some(height) = ours.get(&row) else {
                continue;
            };
            let difference = (f64::from(*height) - f64::from(*stored)).abs();
            let rows = &mut stats.rows;
            rows.0 += 1;
            rows.1 += usize::from(difference < 1.0);
            rows.2 += usize::from(difference <= 15.0);
            rows.3 += difference;
        }
    }
    stats
}

fn frames(document: &Document, oracle: &LayoutOracle, layout: &DocumentLayout) -> FrameStats {
    let mut stats = FrameStats::default();
    for (cache, frame_id) in oracle.frames.iter().zip(document.frame_stack()) {
        // A table's stored rectangle is the part Ami Pro laid out last,
        // after opening a file its last (TP1S).
        let boxes: Vec<(u32, &FrameBox)> = layout
            .pages
            .iter()
            .flat_map(|page| {
                page.frames
                    .iter()
                    .filter(|frame_box| frame_box.frame == *frame_id)
                    .map(|frame_box| (page.number, frame_box))
            })
            .collect();
        let Some(&(page, frame_box)) = (if cache.table_rows.is_empty() {
            boxes.first()
        } else {
            boxes.last()
        }) else {
            continue;
        };
        let [left, top, right, bottom] = cache.rect.map(|value| value as f32);
        if cache.is_anchored() {
            let anchored = &mut stats.anchored;
            anchored.0 += 1;
            anchored.1 = anchored.1.max((frame_box.bounds.y - top).abs());
            anchored.2 = anchored.2.max((frame_box.bounds.x - left).abs());
            anchored.3 = anchored
                .3
                .max((frame_box.bounds.width - (right - left)).abs())
                .max((frame_box.bounds.height - (bottom - top)).abs());
            anchored.4 += usize::from(page == cache.page);
        }
        if let Some((table, _)) = document.cell_holding(*frame_id)
            && let Some(index) = document.frame_stack().iter().position(|id| *id == table)
            && let Some(table_cache) = oracle.frames.get(index)
            && let Some(table_box) = page_frame_box(layout, table)
        {
            let [table_left, table_top, ..] = table_cache.rect.map(|value| value as f32);
            let ours = [
                frame_box.bounds.x - table_box.x,
                frame_box.bounds.y - table_box.y,
                frame_box.bounds.width,
                frame_box.bounds.height,
            ];
            let stored = [
                left - table_left,
                top - table_top,
                right - left,
                bottom - top,
            ];
            let difference = ours
                .iter()
                .zip(stored)
                .map(|(ours, stored)| (ours - stored).abs())
                .fold(0.0, f32::max);
            let cells = &mut stats.cells;
            cells.0 += 1;
            cells.1 += usize::from(page == cache.page);
            cells.2 += usize::from(difference <= 1.0);
            cells.3 = cells.3.max(difference);
        }
        let text_frame = cache.flags & (0x4 | 0x2000_0000) == 0;
        if text_frame
            && let (Some(first), Some(last)) = (cache.columns.first(), cache.columns.last())
        {
            stats.columns.0 += 1;
            stats.columns.1 = stats
                .columns
                .1
                .max((frame_box.interior.x - first.0 as f32).abs())
                .max((frame_box.interior.right() - last.1 as f32).abs());
        }
    }
    stats
}

/// A frame's box on whichever page holds it.
fn page_frame_box(layout: &DocumentLayout, frame: FrameId) -> Option<Bounds> {
    layout.pages.iter().find_map(|page| {
        page.frames
            .iter()
            .find(|frame_box| frame_box.frame == frame)
            .map(|frame_box| frame_box.bounds)
    })
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "—".into()
    } else {
        format!("{:.0}%", part as f64 * 100.0 / whole as f64)
    }
}

#[test]
#[ignore = "manual layout comparison with Ami Pro's caches over private files"]
fn layout_comparison_report() {
    let mut families = BTreeMap::new();
    for variable in ["AMIPRO_VM_LAB", "AMIPRO_CORPUS"] {
        let Some(directory) = std::env::var_os(variable) else {
            eprintln!("{variable} is not set; skipping");
            continue;
        };
        let mut files = Vec::new();
        sam_files(Path::new(&directory), &mut files);
        eprintln!("== {variable}: {} SAM files", files.len());
        eprintln!(
            "file | paragraphs | pages Ami Pro/ours | lines equal (±1) of n | page ends same of n (worst) | anchored n: |dy| |dx| |size| same page | text-frame columns n max | notes"
        );
        let (mut counted, mut same_count, mut within_one, mut within_tenth) = (0, 0, 0, 0);
        let mut totals = (0, 0, 0, 0, 0);
        let mut indents_differ = 0;
        let mut cell_totals = (0, 0, 0, 0.0f32);
        let mut table_totals = TableStats::default();
        let (mut height_sum, mut height_worst) = (0.0f64, 0.0f64);
        let mut by_mode = [(0usize, 0.0f64); 3];
        let mut by_fonts = [(0usize, 0.0f64); 4];
        let mut seen = BTreeSet::new();
        for path in &files {
            let Some(comparison) = compare(path, &mut families) else {
                eprintln!("{}: not imported", path.display());
                continue;
            };
            // The corpus repeats some files in several directories.
            if !seen.insert((comparison.name.clone(), comparison.paragraphs)) {
                continue;
            }
            if let Some(theirs) = comparison.oracle_pages
                && theirs > 0
            {
                let ours = comparison.our_pages as i64;
                let theirs = i64::from(theirs);
                counted += 1;
                same_count += usize::from(ours == theirs);
                within_one += usize::from((ours - theirs).abs() <= 1);
                within_tenth += usize::from((ours - theirs).abs() as f64 <= theirs as f64 * 0.1);
            }
            height_sum += comparison.heights.0;
            for (total, part) in by_mode.iter_mut().zip(comparison.by_mode) {
                total.0 += part.0;
                total.1 += part.1;
            }
            for (total, part) in by_fonts.iter_mut().zip(comparison.by_fonts) {
                total.0 += part.0;
                total.1 += part.1;
            }
            height_worst = height_worst.max(comparison.heights.1);
            indents_differ += comparison.indents_differ;
            let found = comparison.tables;
            table_totals.pages.0 += found.pages.0;
            table_totals.pages.1 += found.pages.1;
            table_totals.rows.0 += found.rows.0;
            table_totals.rows.1 += found.rows.1;
            table_totals.rows.2 += found.rows.2;
            table_totals.rows.3 += found.rows.3;
            table_totals.cached += found.cached;
            let cells = comparison.frames.cells;
            cell_totals.0 += cells.0;
            cell_totals.1 += cells.1;
            cell_totals.2 += cells.2;
            cell_totals.3 = cell_totals.3.max(cells.3);
            totals.0 += comparison.lines.0;
            totals.1 += comparison.lines.1;
            totals.2 += comparison.lines.2;
            totals.3 += comparison.page_ends.0;
            totals.4 += comparison.page_ends.1;
            let mut notes = Vec::new();
            if comparison.out_of_range > 0 {
                notes.push(format!("{} out of range", comparison.out_of_range));
            }
            if comparison.frames_mismatched {
                notes.push("frame count differs".into());
            }
            if comparison.indents_differ > 0 {
                notes.push(format!("{} with other indents", comparison.indents_differ));
            }
            if found.pages.0 > 0 {
                notes.push(format!(
                    "table page ends {} of {}",
                    found.pages.1, found.pages.0
                ));
            }
            if found.rows.0 > found.rows.1 {
                notes.push(format!(
                    "table rows {} of {} (mean {:.0})",
                    found.rows.1,
                    found.rows.0,
                    found.rows.3 / found.rows.0 as f64
                ));
            }
            eprintln!(
                "{} | {} | {}/{} | {} ({}) of {} | {} of {} ({:+}) | {}: {:.0} {:.0} {:.0} {} | {} {:.0} | {}",
                comparison.name,
                comparison.paragraphs,
                comparison
                    .oracle_pages
                    .map_or("—".into(), |pages| pages.to_string()),
                comparison.our_pages,
                comparison.lines.1,
                comparison.lines.2,
                comparison.lines.0,
                comparison.page_ends.1,
                comparison.page_ends.0,
                comparison.page_ends.2,
                comparison.frames.anchored.0,
                comparison.frames.anchored.1,
                comparison.frames.anchored.2,
                comparison.frames.anchored.3,
                comparison.frames.anchored.4,
                comparison.frames.columns.0,
                comparison.frames.columns.1,
                notes.join("; "),
            );
        }
        eprintln!(
            "summary {variable}: page counts equal {} ({same_count} of {counted}), within one {}, within 10% {}; paragraph line counts equal {} ({} of {}), within one {}; paragraph height difference where line counts agree: mean {:.1}%, worst {:.1}% (single spacing: {} paragraphs, mean {:.1}%; other line spacing: {}, mean {:.1}%; styles from a missing STY: {}, mean {:.1}%); page ends on the same page {} ({} of {})",
            percent(same_count, counted),
            percent(within_one, counted),
            percent(within_tenth, counted),
            percent(totals.1, totals.0),
            totals.1,
            totals.0,
            percent(totals.2, totals.0),
            height_sum * 100.0 / totals.1.max(1) as f64,
            height_worst * 100.0,
            by_mode[0].0,
            by_mode[0].1 * 100.0 / by_mode[0].0.max(1) as f64,
            by_mode[1].0,
            by_mode[1].1 * 100.0 / by_mode[1].0.max(1) as f64,
            by_mode[2].0,
            by_mode[2].1 * 100.0 / by_mode[2].0.max(1) as f64,
            percent(totals.4, totals.3),
            totals.4,
            totals.3,
        );
        eprintln!(
            "summary {variable}: paragraphs whose imported indents differ from their style's: {indents_differ}"
        );
        eprintln!(
            "summary {variable}: frames in table cells: {} compared, {} on the stored page, {} where the stored rectangle puts them in their table, largest difference {:.0}",
            cell_totals.0, cell_totals.1, cell_totals.2, cell_totals.3
        );
        eprintln!(
            "summary {variable}: tables: page ends in a table with the same rows {} ({} of {}); rows of Automatic tables at Ami Pro's stored height {} ({} of {}; {} of them by the row cache), within 15 twips {}, mean difference {:.1}",
            percent(table_totals.pages.1, table_totals.pages.0),
            table_totals.pages.1,
            table_totals.pages.0,
            percent(table_totals.rows.1, table_totals.rows.0),
            table_totals.rows.1,
            table_totals.rows.0,
            table_totals.cached,
            percent(table_totals.rows.2, table_totals.rows.0),
            table_totals.rows.3 / table_totals.rows.0.max(1) as f64
        );
        let [one, one_uncalibrated, mixed, mixed_uncalibrated] =
            by_fonts.map(|(count, sum)| (count, sum * 100.0 / count.max(1) as f64));
        eprintln!(
            "summary {variable}: single-spaced height difference by fonts: one calibrated font {} paragraphs, mean {:.1}%; one other font {}, {:.1}%; mixed, all calibrated {}, {:.1}%; mixed with others {}, {:.1}%",
            one.0,
            one.1,
            one_uncalibrated.0,
            one_uncalibrated.1,
            mixed.0,
            mixed.1,
            mixed_uncalibrated.0,
            mixed_uncalibrated.1,
        );
    }
    eprintln!(
        "font families → substitutes: paragraphs compared, line counts equal, mean height difference where equal"
    );
    for (family, stats) in families {
        eprintln!(
            "  {family} → {}: {}, {} ({}), {:.1}%",
            stats.substitute,
            stats.compared,
            stats.equal,
            percent(stats.equal, stats.compared),
            stats.heights * 100.0 / stats.equal.max(1) as f64
        );
    }
}

/// Prints each table's rows whose height differs from Ami Pro's stored one,
/// with their cells' paragraphs.
fn table_detail(document: &Document, oracle: &LayoutOracle, layout: &DocumentLayout) {
    for (index, end) in oracle.pages.iter().enumerate() {
        let Some((first, count)) = end.table_rows else {
            continue;
        };
        let Some(table) = end
            .paragraph
            .checked_sub(1)
            .and_then(|p| table_in(document, p))
        else {
            continue;
        };
        let ours: Vec<_> = table_parts(layout, index, table)
            .iter()
            .map(|part| (part.rows.clone(), part.heading.clone(), part.cut))
            .collect();
        eprintln!(
            "  page {} ends in table {table:?}: Ami Pro rows {first}+{count}; ours {ours:?}",
            index + 1
        );
    }
    for (cache, frame_id) in oracle.frames.iter().zip(document.frame_stack()) {
        if cache.table_rows.is_empty() {
            continue;
        }
        let Some(amipro_document::FrameContent::Table(table)) =
            document.frame(*frame_id).map(|frame| &frame.content)
        else {
            continue;
        };
        let mut ours: BTreeMap<usize, (u32, f32, bool)> = BTreeMap::new();
        for page in 0..layout.pages.len() {
            for part in table_parts(layout, page, *frame_id) {
                for (offset, row) in part.rows.clone().enumerate() {
                    let split =
                        (offset == 0 && part.continued()) || (part.cut && row + 1 == part.rows.end);
                    if let Some(height) = part.heights.get(offset) {
                        ours.insert(row, (page as u32 + 1, *height, split));
                    }
                }
            }
        }
        eprintln!(
            "  table {frame_id:?}: automatic {} columns {:?} options {:?}",
            cache.automatic, table.columns, table.row_options
        );
        let columns = table.columns.len().max(1);
        for (row, stored) in cache.table_rows.iter().enumerate() {
            let Some((page, height, split)) = ours.get(&row) else {
                eprintln!("    row {row}: stored {stored}, not laid out");
                continue;
            };
            if (height - *stored as f32).abs() < 1.0 {
                continue;
            }
            eprintln!(
                "    row {row} page {page}: stored {stored}, ours {height}{}",
                if *split { " (split)" } else { "" }
            );
            for column in 0..columns {
                let Some(cell) = table.cells.get(row * columns + column) else {
                    continue;
                };
                let amipro_document::CellContent::Text(flow) = cell.content;
                let Some(flow) = document.flow(flow) else {
                    continue;
                };
                for paragraph in &flow.paragraphs {
                    let style = &document.styles()[&paragraph.style];
                    eprintln!(
                        "      c{column} {:?} {} {}pt spacing {:?} line {:?} format {:?}: {:?}",
                        style.name,
                        style.font_family,
                        style.font_size_points,
                        (
                            style.spacing.above,
                            style.spacing.below,
                            style.spacing.add_at_break
                        ),
                        style.spacing.line,
                        paragraph.format,
                        paragraph.text.chars().take(60).collect::<String>()
                    );
                }
            }
        }
    }
}

/// Prints one file's oracle next to our layout.
fn detail(document: &Document, oracle: &LayoutOracle, layout: &DocumentLayout) {
    table_detail(document, oracle, layout);
    if std::env::var("AMIPRO_COMPARE_DETAIL_TABLES").is_ok() {
        return;
    }
    for style in &oracle.styles {
        eprintln!("  style {style:?}");
    }
    for cache in &oracle.paragraphs {
        eprintln!("  cache {cache:?}");
    }
    for end in &oracle.pages {
        eprintln!("  page end {end:?}");
    }
    for (index, frame) in oracle.frames.iter().enumerate() {
        let ours = document
            .frame_stack()
            .get(index)
            .and_then(|id| document.frame(*id));
        eprintln!(
            "  oracle frame {index}: {frame:?}; ours {:?}",
            ours.map(|frame| (frame.id, frame.placement, &frame.style.interior.margins))
        );
    }
    for page in &layout.pages {
        eprintln!(
            "  our page {} ({}×{}), columns {:?}",
            page.number, page.width, page.height, page.columns
        );
        for frame in &page.frames {
            eprintln!(
                "    frame {:?} {:?} bounds {:?} interior {:?}",
                frame.frame, frame.layer, frame.bounds, frame.interior
            );
        }
        for line in &page.lines {
            eprintln!(
                "    line {:?} p{} {:?} x {} top {} height {} width {}/{}",
                line.container,
                line.paragraph_index,
                line.bytes,
                line.x,
                line.top,
                line.height,
                line.width,
                line.slot_width
            );
        }
    }
}

/// The text of each line of main-flow paragraph `index`, trailing blanks
/// left out.
fn line_texts(
    document: &Document,
    layout: &amipro_layout::DocumentLayout,
    index: usize,
) -> Vec<String> {
    let paragraph = &document.paragraphs()[index];
    layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| {
            line.container == amipro_layout::Container::Main && line.paragraph_index == index
        })
        .map(|line| paragraph.text[line.bytes.clone()].trim_end().to_owned())
        .collect()
}

/// VM fixtures FLB0 and FLB1 (1st line bold off and on, Ami Pro 3.1,
/// `screens/flb_after.png`): the first line of the long paragraphs ends
/// where the bold text fits, "quickly," and "can be" (FLB0: "quickly, and"),
/// and the later lines break as regular text.
#[test]
#[ignore = "needs the VM folder (AMIPRO_VM_LAB)"]
fn first_line_bold_breaks_where_ami_pro_breaks_it() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lines = |name: &str, index: usize| {
        let path = Path::new(&lab).join("OUT").join(name);
        let imported =
            import_sam_path_with_style_directories(&path, &[]).expect("the fixture reads");
        let mut pages = SessionPages::default();
        pages.update(&imported.document);
        line_texts(&imported.document, pages.layout(), index)
    };
    let off = lines("FLB0.SAM", 0);
    assert!(off[0].ends_with("jump quickly, and"), "FLB0: {off:?}");
    let on = lines("FLB1.SAM", 0);
    assert!(on[0].ends_with("jump quickly,"), "{on:?}");
    assert!(on[1].starts_with("and every good boy"), "{on:?}");
    assert!(on[1].ends_with("runs over"), "{on:?}");
    assert_eq!(on[2], "several lines of text.", "{on:?}");
    assert_eq!(on.len(), 3, "{on:?}");
    let short = lines("FLB1.SAM", 1);
    assert_eq!(short, ["Short second paragraph."]);
    let third = lines("FLB1.SAM", 2);
    assert!(
        third[0].ends_with("second and third lines can be"),
        "{third:?}"
    );
    assert!(
        third[1].starts_with("compared with the first line"),
        "{third:?}"
    );
    assert_eq!(third.len(), 2, "{third:?}");
}
