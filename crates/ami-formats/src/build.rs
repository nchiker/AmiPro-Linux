//! Maps a SAM file's structure and text onto the document model:
//! page layouts, frames, groups, tables, pictures, and the main text's inline
//! objects (`docs/research/SAM_PAGE_FRAME_SPEC.md`). Values the model does not
//! interpret stay with their object as legacy data; caches are dropped.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use amipro_document::{
    BreakKind, CellContent, CellFormat, CellMargins, CellSpan, ColumnLine, Document, FaceKey, Flow,
    FlowId, Frame, FrameContent, FrameId, FrameStyle, GroupId, InlineObject, LayoutBox, LegacyData,
    LineHeights, LinePosition, LineSpacing, Margins, PICTURE_MAX, PageGeometry, PageLayout,
    PageSide, Paragraph, Picture, PictureSizing, PictureSource, Placement, Rect, RepeatPages,
    Shadow, Sides, Size, StyleId, Table, TableCell, TableLayout, TableRows, TableSpacing, Twips,
    Wrap,
};

use crate::structure::{EmbeddedEntry, RawContent, RawFrame, RawLayout, SamStructure, Slot};
use crate::text::{ParseContext, ParsedParagraph, PendingObject, TextReport, parse_paragraph};

/// Frame flag bits (`SAM_PAGE_FRAME_SPEC.md` §4.3).
const TABLE: u32 = 0x4;
/// The paragraph style of table cell text (user's guide).
pub(crate) const TABLE_TEXT: &str = "Table Text";
/// Cell record field 3: the top-left cell of joined cells, which also has
/// [`COVERED_CELL`]; fields 4 and 5 are then its rows and columns.
const JOINED_CELL: i64 = 0x100;
/// Cell record field 3: a cell inside joined cells; fields 4 and 5 are then
/// its offset from the top-left cell (corpus: 27 joined, 37 covered, all
/// consistent).
const COVERED_CELL: i64 = 0x80;
const OPAQUE: u32 = 0x40;
const WRAP_AROUND: u32 = 0x80;
const REPEATING: u32 = 0x100;
const HEADER: u32 = 0x800;
const FOOTER: u32 = 0x1000;
const RIGHT_LEFT: u32 = 0x2000;
const NO_WRAP_BESIDE: u32 = 0x20000;
const ANCHORED: u32 = 0x80000;
const GROUP_MEMBER: u32 = 0x80_0000;
const GROUP_FIRST: u32 = 0x100_0000;
const GROUP_LAST: u32 = 0x200_0000;
const PICTURE: u32 = 0x2000_0000;
/// A frame in a table cell (`1248:0347`, `CELL_FRAME_NOTES.md`): kept as an
/// unknown bit until a cell names the frame (`cells_hold_frames`).
pub(crate) const IN_CELL: u32 = 0x20;
/// A frame that holds text: the writers derive it (2,989 of 2,989 frame
/// records in the corpora have it exactly when they hold text).
const HAS_TEXT: u32 = 0x200;
/// Lines on: the frame's sides (field 7) count only with this bit
/// (decompiled reader; `SAM_FORMAT_NOTES.md`).
pub(crate) const LINES_ON: u32 = 0x10000;
const KNOWN_FLAGS: u32 = TABLE
    | HAS_TEXT
    | LINES_ON
    | OPAQUE
    | WRAP_AROUND
    | REPEATING
    | HEADER
    | FOOTER
    | RIGHT_LEFT
    | NO_WRAP_BESIDE
    | ANCHORED
    | GROUP_MEMBER
    | GROUP_FIRST
    | GROUP_LAST
    | PICTURE;

/// Layout number bits (§3.1).
const LANDSCAPE: u32 = 0x100;
const ONE_LAYOUT: u32 = 0x200;
const MIRROR: u32 = 0x400;
const HEADER_SECOND: u32 = 0x800;
const FOOTER_SECOND: u32 = 0x1000;

/// Bounds against garbled tables.
const MAX_TABLE_ROWS: usize = 32_768;
const MAX_TABLE_COLUMNS: usize = 256;
const MAX_TABLE_CELLS: usize = 262_144;

/// Counts the corpus audit and the import report use.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StructureStats {
    pub layouts: usize,
    pub frames: usize,
    pub anchors_resolved: usize,
    pub anchors_unresolved: usize,
    /// Anchored frame records with no mark in the main text; they become
    /// Where placed frames.
    pub anchored_without_mark: usize,
    /// Frame flag bits the model does not interpret, OR-ed together.
    pub unknown_frame_flags: u32,
    /// Pages whose stored column edges differ from uniform columns derived
    /// from the margins and the first gutter.
    pub nonuniform_page_columns: usize,
    pub legacy_marks: usize,
    pub dropped_caches: usize,
    /// Fonts whose printer line height the caches gave.
    pub calibrated_line_heights: usize,
    pub embedded_objects: usize,
    pub unbalanced_controls: usize,
}

pub(crate) struct Built {
    pub document: Document,
    pub stats: StructureStats,
    pub warnings: Vec<String>,
    pub unmapped_controls: BTreeSet<String>,
    /// The pitch and family bytes the text's font changes gave each face.
    pub face_kinds: crate::text::FaceKindVotes,
}

/// A table cell's record, parsed text, and further raw bodies.
/// A cell's record, parsed text, further bodies, and formula.
type PreparedCell = (String, Vec<ParsedParagraph>, Vec<String>, Option<String>);

/// A frame record with its text parsed.
struct Prepared<'a> {
    raw: &'a RawFrame,
    text: Option<Vec<ParsedParagraph>>,
    cells: Vec<PreparedCell>,
}

/// Parses the frame's text and cells (never main text).
fn prepare<'a>(raw: &'a RawFrame, report: &mut TextReport) -> Prepared<'a> {
    let mut outside = ParseContext::outside_main();
    let mut parse_all = |paragraphs: &[String]| -> Vec<ParsedParagraph> {
        paragraphs
            .iter()
            .map(|paragraph| parse_paragraph(paragraph, report, &mut outside))
            .collect()
    };
    match &raw.content {
        RawContent::Text(paragraphs) => Prepared {
            raw,
            text: Some(parse_all(paragraphs)),
            cells: Vec::new(),
        },
        RawContent::Table(table) => {
            let cells = table
                .cells
                .iter()
                .map(|cell| {
                    let (extra, formula) = crate::frames::split_formula(
                        &cell.record,
                        cell.paragraphs.first(),
                        &cell.extra,
                    );
                    (
                        cell.record.clone(),
                        parse_all(&cell.paragraphs),
                        extra,
                        formula,
                    )
                })
                .collect();
            Prepared {
                raw,
                text: None,
                cells,
            }
        }
        RawContent::Picture(_) | RawContent::None | RawContent::Unreadable(_) => Prepared {
            raw,
            text: None,
            cells: Vec::new(),
        },
    }
}

/// Every style name that `paragraphs` use, nested floating headers, notes,
/// and footnotes included.
fn collect_style_names(paragraphs: &[ParsedParagraph], names: &mut Vec<String>) {
    for paragraph in paragraphs {
        names.push(
            paragraph
                .style
                .clone()
                .unwrap_or_else(|| crate::BODY_TEXT.to_owned()),
        );
        for object in &paragraph.objects {
            if let PendingObject::Floating { paragraphs, .. }
            | PendingObject::Note { paragraphs, .. }
            | PendingObject::Footnote { paragraphs } = object
            {
                collect_style_names(paragraphs, names);
            }
        }
    }
}

struct Builder<'a> {
    document: Document,
    style_ids: &'a BTreeMap<String, StyleId>,
    stats: StructureStats,
    warnings: Vec<String>,
    embedded: BTreeMap<u32, EmbeddedEntry>,
    file: &'a [u8],
    /// Single-spaced paragraphs without objects whose height Ami Pro
    /// cached: the fonts their lines are measured by, and the height
    /// without the space of the style's paragraph rules.
    line_height_samples: Vec<(BTreeSet<FaceKey>, i32)>,
    /// Each style's fingerprint, for the paragraph caches.
    style_prints: BTreeMap<StyleId, u64>,
}

pub(crate) fn build(
    title: String,
    mut styles: BTreeMap<StyleId, amipro_document::ParagraphStyle>,
    mut style_ids: BTreeMap<String, StyleId>,
    structure: &SamStructure,
    body: &[String],
    file: &[u8],
) -> Built {
    let mut report = TextReport::default();
    let anchored_records = structure
        .frames
        .iter()
        .filter(|frame| field(frame, 2).is_some_and(|flags: u32| flags & ANCHORED != 0))
        .count();
    let mut main = ParseContext {
        main: true,
        anchored_frames: anchored_records,
        used_anchors: BTreeSet::new(),
    };
    let body: Vec<ParsedParagraph> = body
        .iter()
        .map(|paragraph| parse_paragraph(paragraph, &mut report, &mut main))
        .collect();
    let top: Vec<Prepared<'_>> = structure
        .frames
        .iter()
        .map(|frame| prepare(frame, &mut report))
        .collect();
    let layouts: Vec<(&RawLayout, Vec<(Slot, Prepared<'_>)>)> = structure
        .layouts
        .iter()
        .map(|layout| {
            let frames = layout
                .frames
                .iter()
                .map(|(slot, frame)| (*slot, prepare(frame, &mut report)))
                .collect();
            (layout, frames)
        })
        .collect();

    // Styles are final before the document exists. Ami Pro generates
    // Table Text, from Body Text with no space above or below, when a table
    // is made, and puts every cell's text in it (user's guide). A file that
    // has Table Text keeps its own spacing; one whose cells name Table Text
    // without defining it has the generated one, as an untagged cell does
    // (Ami Pro writes cells untagged).
    let has_table = top
        .iter()
        .chain(
            layouts
                .iter()
                .flat_map(|(_, frames)| frames.iter().map(|(_, f)| f)),
        )
        .any(|prepared| matches!(prepared.raw.content, RawContent::Table(_)));
    if has_table && !style_ids.contains_key(&TABLE_TEXT.to_lowercase()) {
        let id = crate::resolve_style(&mut styles, &mut style_ids, TABLE_TEXT);
        if let Some(style) = styles.get_mut(&id) {
            style.spacing.above = 0;
            style.spacing.below = 0;
            // Made with the table, so stored in the document.
            style.origin = amipro_document::StyleOrigin::Document;
        }
    }
    let mut names = Vec::new();
    collect_style_names(&body, &mut names);
    for prepared in top.iter().chain(
        layouts
            .iter()
            .flat_map(|(_, frames)| frames.iter().map(|(_, f)| f)),
    ) {
        if let Some(text) = &prepared.text {
            collect_style_names(text, &mut names);
        }
        for (_, cell, _, _) in &prepared.cells {
            collect_style_names(cell, &mut names);
        }
    }
    if names.is_empty() {
        names.push(crate::BODY_TEXT.to_owned());
    }
    for name in &names {
        crate::resolve_style(&mut styles, &mut style_ids, name);
    }

    let mut builder = Builder {
        document: Document::empty(title, styles),
        style_ids: &style_ids,
        stats: StructureStats::default(),
        warnings: Vec::new(),
        embedded: BTreeMap::new(),
        file,
        line_height_samples: Vec::new(),
        style_prints: BTreeMap::new(),
    };
    match crate::structure::embedded_directory(file) {
        Ok(entries) => {
            for entry in entries {
                builder.embedded.insert(entry.sequence, entry);
            }
        }
        Err(message) => builder
            .warnings
            .push(format!("embedded objects: {message}")),
    }

    // Layouts replace the default one, in file order.
    if !layouts.is_empty() {
        let default = builder.document.document_layout().clone();
        for frame in default.frames() {
            if let Some(removed) = builder.document.remove_frame(frame) {
                for flow in removed.owned_flows() {
                    builder.document.remove_flow(flow);
                }
            }
        }
        builder.document.layouts_mut().clear();
        for (raw, frames) in &layouts {
            builder.layout(raw, frames);
        }
    }

    // Page frames, back to front; anchored ones in the shared anchor order.
    let mut anchored = Vec::new();
    let mut anchored_records = Vec::new();
    for (index, prepared) in top.iter().enumerate() {
        let id = builder.frame(prepared, None);
        builder.document.frame_stack_mut().push(id);
        if field(prepared.raw, 2).is_some_and(|flags: u32| flags & ANCHORED != 0) {
            anchored.push(id);
            anchored_records.push(index);
        }
    }
    builder.groups(&top);
    builder.cells_hold_frames();

    // The main text.
    let mut paragraphs = Vec::with_capacity(body.len().max(1));
    for parsed in &body {
        paragraphs.push(Arc::new(builder.paragraph(parsed, Some(&anchored))));
    }
    if paragraphs.is_empty() {
        let style = builder.style(None);
        paragraphs.push(Arc::new(builder.document.new_paragraph("", style)));
    }
    let main = builder.document.main_flow_id();
    builder
        .document
        .replace_flow_paragraphs(main, paragraphs)
        .expect("the main flow exists");

    // Anchored records without a mark become Where placed frames.
    let used: BTreeSet<usize> = main_context_used(&body);
    for (index, frame) in anchored.iter().enumerate() {
        if used.contains(&index) {
            continue;
        }
        builder.stats.anchored_without_mark += 1;
        let record = top[anchored_records[index]].raw;
        let page = field(record, 1).unwrap_or(1u32).max(1);
        let rect = frame_rect(record);
        if let Some(target) = builder.document.frame_mut(*frame) {
            target.placement = Placement::Fixed { page, rect };
            target.legacy.push("frame.anchored-without-mark", "1");
        }
        builder.warnings.push(format!(
            "anchored frame record {index} has no mark in the text; it is placed on page {page}"
        ));
    }

    // Style records and the end of the layouts are structure the styles
    // and layouts hold; every other section is kept as read, in order.
    for (name, raw) in &structure.sections {
        if name.eq_ignore_ascii_case("tag") || name.eq_ignore_ascii_case("elay") {
            continue;
        }
        builder
            .document
            .legacy_mut()
            .push(format!("section [{name}]"), raw.clone());
    }
    crate::info::take_doc_info(&mut builder.document);
    crate::info::take_footnote_options(&mut builder.document);
    crate::info::take_line_numbering(&mut builder.document);
    crate::info::take_proofing(&mut builder.document);
    crate::info::take_contents(&mut builder.document);

    let line_heights = calibrate_line_heights(&builder.line_height_samples);
    builder.stats.calibrated_line_heights = line_heights.len();
    builder.document.set_line_heights(line_heights);

    builder.stats.frames = builder.document.frames().count();
    builder.stats.layouts = builder.document.layouts().len();
    builder.stats.anchors_resolved = report.anchors_resolved;
    builder.stats.anchors_unresolved = report.anchors_unresolved;
    builder.stats.legacy_marks = report.legacy_marks;
    builder.stats.dropped_caches = report.dropped_caches;
    builder.stats.unbalanced_controls = report.unbalanced_controls;
    builder.stats.embedded_objects = builder.document.objects().len();
    let mut warnings = structure.warnings.clone();
    warnings.extend(builder.warnings);
    Built {
        document: builder.document,
        stats: builder.stats,
        warnings,
        unmapped_controls: report.unmapped,
        face_kinds: report.face_kinds,
    }
}

/// The printer line heights Ami Pro's paragraph caches give
/// (`DECISIONS.md`, "Calibrated line heights"). A line is as tall as the
/// tallest ascent plus external leading over its fonts plus the deepest
/// descent (decompile `10d0:359b`), so a one-line paragraph set in one
/// font, 0.85 to 1.6 times its size, votes for that font's height; each font
/// takes the height it was given most often, the smaller on a tie. Then a
/// one-line paragraph that mixes fonts all known but one, and is taller
/// than the known ones, gives the other its height, as fonts of one family
/// split their height alike; that repeats while it finds new fonts.
fn calibrate_line_heights(samples: &[(BTreeSet<FaceKey>, i32)]) -> LineHeights {
    let one_line = |fonts: &BTreeSet<FaceKey>, height: i32| {
        fonts
            .iter()
            .map(|key| key.size)
            .max()
            .filter(|size| *size > 0)
            .is_some_and(|size| (0.85..=1.6).contains(&(height as f32 / size as f32)))
    };
    let elect = |votes: BTreeMap<FaceKey, BTreeMap<i32, usize>>, heights: &mut LineHeights| {
        for (key, votes) in votes {
            if let Some((height, _)) = votes
                .into_iter()
                .max_by_key(|(height, count)| (*count, std::cmp::Reverse(*height)))
            {
                heights.insert(key, height);
            }
        }
    };
    let mut votes: BTreeMap<FaceKey, BTreeMap<i32, usize>> = BTreeMap::new();
    for (fonts, height) in samples {
        if fonts.len() == 1 && one_line(fonts, *height) {
            let key = fonts.first().expect("one font").clone();
            *votes.entry(key).or_default().entry(*height).or_default() += 1;
        }
    }
    let mut heights = LineHeights::default();
    elect(votes, &mut heights);
    loop {
        let mut votes: BTreeMap<FaceKey, BTreeMap<i32, usize>> = BTreeMap::new();
        for (fonts, height) in samples {
            if fonts.len() < 2 || !one_line(fonts, *height) {
                continue;
            }
            let mut unknown = fonts.iter().filter(|key| heights.get(key).is_none());
            let (Some(key), None) = (unknown.next(), unknown.next()) else {
                continue;
            };
            let known = fonts.iter().filter_map(|key| heights.get(key)).max();
            if known.is_some_and(|known| *height > known) {
                *votes
                    .entry(key.clone())
                    .or_default()
                    .entry(*height)
                    .or_default() += 1;
            }
        }
        if votes.is_empty() {
            return heights;
        }
        elect(votes, &mut heights);
    }
}

/// The anchor indexes the main text resolved.
fn main_context_used(body: &[ParsedParagraph]) -> BTreeSet<usize> {
    body.iter()
        .flat_map(|paragraph| &paragraph.objects)
        .filter_map(|object| match object {
            PendingObject::Anchor { index, .. } => Some(*index),
            _ => None,
        })
        .collect()
}

/// Field `number` (1-based) of a frame record.
fn field<T: std::str::FromStr>(frame: &RawFrame, number: usize) -> Option<T> {
    frame.values.get(number - 1)?.trim().parse().ok()
}

fn numbers(text: &str) -> Vec<i64> {
    text.split_whitespace()
        .map(|value| value.parse().unwrap_or(0))
        .collect()
}

/// The largest length a SAM value may give, about 230 m: far beyond any page,
/// and small enough that sums and differences of lengths never overflow.
const MAX_TWIPS: i64 = 1 << 24;

/// Ami Pro measures every Automatic row when it opens a file and stores its
/// measure (VM RC1–RC3), so each row's stored height is a cache, kept while
/// the row is as read (`Table::read_rows`): each row's fingerprint once the
/// import is complete.
pub(crate) fn remember_read_rows(document: &mut Document) {
    let tables: Vec<(FrameId, BTreeMap<usize, u64>)> = document
        .frames()
        .filter_map(|frame| match &frame.content {
            FrameContent::Table(table) if table.row_options.automatic => Some((
                frame.id,
                (0..table.rows.len())
                    .map(|row| (row, document.table_row_fingerprint(table, row)))
                    .collect(),
            )),
            _ => None,
        })
        .collect();
    for (id, rows) in tables {
        if let Some(frame) = document.frame_mut(id)
            && let FrameContent::Table(table) = &mut frame.content
        {
            table.read_rows = rows;
        }
    }
}

/// A cell record's span: `ROW COL FLAGS ROWS COLUMNS …`.
fn cell_span(record: &str) -> CellSpan {
    let values = numbers(record);
    let flags = values.get(2).copied().unwrap_or(0);
    let count = |index: usize| {
        values
            .get(index)
            .and_then(|value| u16::try_from(*value).ok())
            .filter(|value| *value > 0)
            .unwrap_or(1)
    };
    if flags & JOINED_CELL != 0 {
        CellSpan {
            rows: count(3),
            columns: count(4),
        }
    } else if flags & COVERED_CELL != 0 {
        CellSpan::COVERED
    } else {
        CellSpan::SINGLE
    }
}

fn twips(value: i64) -> Twips {
    value.clamp(-MAX_TWIPS, MAX_TWIPS) as Twips
}

fn frame_rect(frame: &RawFrame) -> Rect {
    let value = |number| twips(field::<i64>(frame, number).unwrap_or(0));
    let mut rect = Rect::new(value(3), value(4), value(5), value(6));
    // A degenerate rectangle cannot hold content; give it a minimal size.
    if rect.right <= rect.left {
        rect.right = rect.left.saturating_add(1);
    }
    if rect.bottom <= rect.top {
        rect.bottom = rect.top.saturating_add(1);
    }
    rect
}

/// A frame's sides as Ami Pro's reader takes them: none without the lines
/// bit (it sets field 7 to 0), and All where the bit is set and field 7 is
/// 0 or above 30 (older files).
pub(crate) fn frame_sides(mask: u32, lines_on: bool) -> Sides {
    match mask {
        _ if !lines_on => Sides::NONE,
        0 | 31.. => Sides::ALL,
        mask => sides(mask),
    }
}

/// Line sides: 0 none, 1 all, otherwise 2 left, 4 right, 8 top, 16 bottom.
pub(crate) fn sides(mask: u32) -> Sides {
    if mask == 1 {
        return Sides::ALL;
    }
    Sides {
        left: mask & 2 != 0,
        right: mask & 4 != 0,
        top: mask & 8 != 0,
        bottom: mask & 16 != 0,
    }
}

fn line_position(value: u32) -> Option<LinePosition> {
    Some(match value {
        1 => LinePosition::Middle,
        2 => LinePosition::Inside,
        3 => LinePosition::Outside,
        4 => LinePosition::CloseToInside,
        5 => LinePosition::CloseToOutside,
        _ => return None,
    })
}

/// A LAYOUT block (§3.2): page or frame interior. Returns the box, whether
/// its stored columns differ from uniform ones, and the values to keep.
struct LayoutValues {
    layout: LayoutBox,
    /// Page height or frame bottom edge (value 1) and width (value 2).
    extent: (Twips, Twips),
    page_border: Option<(u16, u32, u32, u32)>,
    nonuniform: bool,
    /// The stored column edges, absolute.
    edges: Vec<(i64, i64)>,
    legacy: Vec<(String, String)>,
}

fn layout_values(
    values: &[String],
    top: Twips,
    what: &str,
    defaults: [i64; 7],
) -> Option<LayoutValues> {
    // Clamped like every length, so the arithmetic below cannot overflow.
    let value = |index: usize| -> Option<i64> {
        values
            .get(index - 1)?
            .trim()
            .parse::<i64>()
            .ok()
            .map(|value| value.clamp(-MAX_TWIPS, MAX_TWIPS))
    };
    let height = value(1)?;
    let width = value(2)?;
    let flags = u32::try_from(value(9)?).ok()?;
    let columns = usize::try_from(value(15)?)
        .ok()
        .filter(|count| (1..=8).contains(count))?;
    let mut position = 15;
    let mut edges = Vec::with_capacity(columns);
    let mut tabs = Vec::with_capacity(columns);
    let mut legacy = Vec::new();
    for column in 0..columns {
        let left = value(position + 1)?;
        let right = value(position + 2)?;
        let count = usize::try_from(value(position + 3)?)
            .ok()
            .filter(|count| *count <= 64)?;
        position += 3;
        let mut stops = Vec::with_capacity(count);
        for _ in 0..count {
            let kind = value(position + 1)?;
            let at = value(position + 2)?;
            position += 2;
            let Some(stop) = crate::tab_stop(kind, at) else {
                legacy.push((format!("{what}.tab.{column}"), format!("{kind} {at}")));
                continue;
            };
            stops.push(stop);
        }
        edges.push((left, right));
        tabs.push(stops);
    }
    if values.len() > position {
        legacy.push((format!("{what}.extra"), values[position..].join(" ")));
    }
    // Kept for the writer (`sheet.rs`): values 3 and 6, value 9's bits
    // beyond balance, the line between columns, and the page border, and
    // values 10 to 14, which Ami Pro writes whether or not those lines are
    // on (stale in frames); each only when it is not what the writer writes
    // without it. The line's style (10) and the lines' colour (11) are the
    // writer's own while the line (or the border) is on, so they are kept
    // only while off: kept then, a line made in the app with another style
    // than the frames' default read back as a change (2026-10-18).
    for (index, default) in [3, 6, 10, 11, 12, 13, 14].into_iter().zip(defaults) {
        let written = match index {
            10 => flags & 2 != 0,
            11 => flags & 6 != 0,
            _ => false,
        };
        if !written
            && let Some(raw) = values.get(index - 1)
            && raw.trim().parse::<i64>().ok() != Some(default)
        {
            legacy.push((format!("{what}.{index}"), raw.clone()));
        }
    }
    if flags & !7 != 0 {
        legacy.push((format!("{what}.9"), (flags & !7).to_string()));
    }
    let left_margin = value(4)?;
    let right_margin = value(8)?;
    let gutter = edges
        .windows(2)
        .next()
        .map_or(240, |pair| pair[1].0 - pair[0].1);
    // Stored edges against uniform columns derived from the margins.
    let text_width = width - left_margin - right_margin;
    let column_count = columns as i64;
    let derived_width = (text_width - gutter * (column_count - 1)) / column_count;
    let nonuniform = edges.iter().enumerate().any(|(index, (left, right))| {
        let start = left_margin + index as i64 * (derived_width + gutter);
        (left - start).abs() > 2 || (right - (start + derived_width)).abs() > 2
    });
    let line_between = (flags & 2 != 0).then(|| ColumnLine {
        style: u16::try_from(value(10).unwrap_or(1)).unwrap_or(1),
        colour: u32::try_from(value(11).unwrap_or(0)).unwrap_or(0),
    });
    let page_border = (flags & 4 != 0).then(|| {
        (
            u16::try_from(value(12).unwrap_or(1)).unwrap_or(1),
            u32::try_from(value(11).unwrap_or(0)).unwrap_or(0),
            u32::try_from(value(13).unwrap_or(1)).unwrap_or(1),
            u32::try_from(value(14).unwrap_or(1)).unwrap_or(1),
        )
    });
    Some(LayoutValues {
        layout: LayoutBox {
            margins: Margins {
                top: twips((value(7)? - i64::from(top)).max(0)),
                right: twips(right_margin.max(0)),
                bottom: twips(value(5)?.max(0)),
                left: twips(left_margin.max(0)),
            },
            columns: columns as u8,
            gutter: twips(gutter.max(0)),
            column_edges: None,
            tabs,
            balance: flags & 1 != 0,
            line_between,
        },
        extent: (twips(height), twips(width)),
        page_border,
        nonuniform,
        edges,
        legacy,
    })
}

/// Stored column edges (absolute) as the model's uneven columns, from the
/// text area's left edge `origin` (the first derived edge), when their
/// widths or gaps differ by more than a twip and they lie in order within
/// the text area `text_width` wide; `None` for even columns, which keep
/// their exact edges as offsets instead (`RULER_NOTES.md`, "Unequal
/// columns in the corpora").
fn uneven_edges(
    stored: &[(i64, i64)],
    origin: i64,
    text_width: i64,
) -> Option<Vec<amipro_document::ColumnEdges>> {
    if stored.len() < 2 {
        return None;
    }
    let edges: Vec<amipro_document::ColumnEdges> = stored
        .iter()
        .map(|(left, right)| amipro_document::ColumnEdges {
            left: twips(left - origin),
            right: twips(right - origin),
        })
        .collect();
    let width = edges[0].width();
    let gap = edges[1].left - edges[0].right;
    let alike = edges.iter().all(|edges| (edges.width() - width).abs() <= 1)
        && edges
            .windows(2)
            .all(|pair| (pair[1].left - pair[0].right - gap).abs() <= 1);
    let mut previous = 0;
    let ordered = edges.iter().all(|edges| {
        let fits = edges.left >= previous && edges.right > edges.left;
        previous = edges.right;
        fits
    }) && i64::from(previous) <= text_width;
    (!alike && ordered).then_some(edges)
}

/// Strikethrough, the scripts, protected text, and no hyphenation end with
/// their off marks (`<-*>`). An off mark that ends the attribute where it was
/// on, in a style without it, returns to the style's own setting, so it
/// reads as the style's (`None`), as the app's commands leave it; an off
/// mark with nothing on before it (Ami Pro's files have them) stays off, so
/// that it is written back. Runs left saying nothing go.
fn ended_as_style(paragraph: &mut Paragraph, style: &amipro_document::CharacterAttributes) {
    use amipro_document::CharacterFormat;
    type Field = fn(&mut CharacterFormat) -> &mut Option<bool>;
    let fields: [(Field, bool); 5] = [
        (|format| &mut format.strikethrough, style.strikethrough),
        (|format| &mut format.superscript, style.superscript),
        (|format| &mut format.subscript, style.subscript),
        (|format| &mut format.protected, style.protected),
        (|format| &mut format.no_hyphenation, style.no_hyphenation),
    ];
    let length = paragraph.text.len();
    let mut runs = std::mem::take(&mut paragraph.runs);
    for (field, styled) in fields {
        if styled {
            continue;
        }
        // Whether the text just before is on, or off continuing an end.
        let mut ending = false;
        let mut at = 0;
        for run in &mut runs {
            if run.start != at {
                ending = false;
            }
            let value = field(&mut run.format);
            match *value {
                Some(true) => ending = true,
                Some(false) if ending => *value = None,
                _ => ending = false,
            }
            at = run.end;
        }
        let end = field(&mut paragraph.end_format);
        if *end == Some(false) && ending && at == length {
            *end = None;
        }
    }
    // Runs stay apart as read (Ami Pro's files have marks between runs
    // that say the same, which are written back).
    runs.retain(|run| !run.format.is_empty());
    paragraph.runs = runs;
}

impl Builder<'_> {
    fn style(&self, name: Option<&str>) -> StyleId {
        let name = name.unwrap_or(crate::BODY_TEXT).to_lowercase();
        self.style_ids
            .get(&name)
            .copied()
            .unwrap_or_else(|| self.document.body_text_style())
    }

    /// Builds a paragraph with a fresh ID. `anchored` is the main text's
    /// anchor targets; `None` outside the main text.
    fn paragraph(&mut self, parsed: &ParsedParagraph, anchored: Option<&[FrameId]>) -> Paragraph {
        let style = self.style(parsed.style.as_deref());
        self.paragraph_in(parsed, anchored, style)
    }

    /// A paragraph whose style, when it names none, is `untagged`.
    fn paragraph_in(
        &mut self,
        parsed: &ParsedParagraph,
        anchored: Option<&[FrameId]>,
        untagged: StyleId,
    ) -> Paragraph {
        let style = parsed
            .style
            .as_deref()
            .map_or(untagged, |name| self.style(Some(name)));
        let mut paragraph = self.document.new_paragraph(parsed.text.clone(), style);
        paragraph.format = parsed.format.clone();
        paragraph.runs = parsed.runs.clone();
        paragraph.marks = parsed.marks.clone();
        paragraph.end_format = parsed.end_format.clone();
        if let Some(style) = self.document.styles().get(&style) {
            ended_as_style(&mut paragraph, &style.attributes);
        }
        let cache = parsed.cache;
        if parsed.objects.is_empty() {
            self.sample_line_height(&paragraph, parsed.cached_height);
        }
        for object in &parsed.objects {
            let object = match object {
                PendingObject::Anchor { index, .. } => InlineObject::FrameAnchor(
                    *anchored
                        .and_then(|anchored| anchored.get(*index))
                        .expect("the parser resolves anchors against the record count"),
                ),
                PendingObject::PageBreak {
                    centre,
                    restart,
                    layout,
                    ..
                } => {
                    let layout = layout.as_ref().and_then(|name| {
                        let found = self
                            .document
                            .layouts()
                            .iter()
                            .find(|candidate| candidate.name.eq_ignore_ascii_case(name.trim()))
                            .map(|candidate| candidate.id);
                        if found.is_none() {
                            self.warnings
                                .push(format!("a page break names missing layout {name:?}"));
                        }
                        found
                    });
                    InlineObject::Break {
                        kind: BreakKind::Page,
                        centre: *centre,
                        layout,
                        restart: *restart,
                    }
                }
                PendingObject::ColumnBreak => InlineObject::Break {
                    kind: BreakKind::Column,
                    centre: false,
                    layout: None,
                    restart: false,
                },
                PendingObject::Floating {
                    kind,
                    pages,
                    paragraphs,
                } => {
                    let flow = self.flow(paragraphs);
                    InlineObject::FloatingHeaderFooter {
                        kind: *kind,
                        pages: *pages,
                        flow,
                    }
                }
                PendingObject::PageNumber(field) => InlineObject::PageNumber(field.clone()),
                PendingObject::Bullet(code) => InlineObject::Bullet(*code),
                PendingObject::Date(field) => InlineObject::Date(*field),
                PendingObject::DocInfo(field) => InlineObject::DocInfo(*field),
                PendingObject::Seq(field) => InlineObject::Seq(field.clone()),
                PendingObject::Note { note, paragraphs } => {
                    let flow = self.flow(paragraphs);
                    InlineObject::Note {
                        note: note.clone(),
                        flow,
                    }
                }
                PendingObject::Footnote { paragraphs } => InlineObject::Footnote {
                    flow: self.flow(paragraphs),
                },
                PendingObject::TableFootnote { number, row } => InlineObject::TableFootnote {
                    number: *number,
                    row: *row,
                },
            };
            paragraph.objects.push(object);
        }
        // Ami Pro's cache, kept with what the paragraph is now (`sam.rs`).
        if let Some((height, width)) = cache
            && let Some(style) = self.document.styles().get(&paragraph.style)
        {
            let print = *self
                .style_prints
                .entry(paragraph.style)
                .or_insert_with(|| style.fingerprint());
            paragraph.cache = Some(amipro_document::ParagraphCache {
                height,
                width,
                fingerprint: paragraph.content_fingerprint_with(print),
            });
        }
        paragraph
    }

    /// Ami Pro lays lines out with its printer's font metrics, which differ
    /// from printer to printer, and caches each paragraph's height. A
    /// single-spaced paragraph without objects (anchored frames and breaks
    /// change heights) is a sample for [`calibrate_line_heights`]. Legacy
    /// marks change no font (`<:s>`, the commonest, is a paragraph flag).
    fn sample_line_height(&mut self, paragraph: &Paragraph, cached: Option<i32>) {
        let Some(height) = cached.filter(|height| *height > 0) else {
            return;
        };
        if paragraph.text.contains('\u{2028}') {
            return;
        }
        let Some(style) = self.document.styles().get(&paragraph.style) else {
            return;
        };
        if paragraph.format.line_spacing.unwrap_or(style.spacing.line) != LineSpacing::Single {
            return;
        }
        // A rule above adds to the first line and one below to the last.
        let (above, below) = style.rules.space();
        let height = height - above - below;
        let fonts = paragraph.fonts(style);
        if !fonts.is_empty() && height > 0 {
            self.line_height_samples.push((fonts, height));
        }
    }

    /// A new flow with the given paragraphs (one empty Body Text paragraph if
    /// there are none).
    fn flow(&mut self, parsed: &[ParsedParagraph]) -> FlowId {
        let body_text = self.style(None);
        self.flow_in(parsed, body_text)
    }

    /// A new flow whose untagged paragraphs, and the empty paragraph of an
    /// empty flow, take `untagged`.
    fn flow_in(&mut self, parsed: &[ParsedParagraph], untagged: StyleId) -> FlowId {
        let mut paragraphs: Vec<Arc<Paragraph>> = parsed
            .iter()
            .map(|paragraph| Arc::new(self.paragraph_in(paragraph, None, untagged)))
            .collect();
        if paragraphs.is_empty() {
            paragraphs.push(Arc::new(self.document.new_paragraph("", untagged)));
        }
        let id = self.document.allocate_flow_id();
        self.document.insert_flow(id, Flow { paragraphs });
        id
    }

    fn layout(&mut self, raw: &RawLayout, frames: &[(Slot, Prepared<'_>)]) {
        let id = self.document.allocate_layout_id();
        let mut layout = PageLayout::letter(id, raw.name.trim());
        let number: u32 = raw.number.trim().parse().unwrap_or(0x201);
        // The paper code is the low seven bits; 0x80 marks a style sheet's
        // layout and is kept as read (`SAM_PAGE_FRAME_SPEC.md` §3.1).
        layout.paper = (number & 0x7F) as u8;
        layout.landscape = number & LANDSCAPE != 0;
        layout.mirror = number & MIRROR != 0;
        layout.header_from_second_page = number & HEADER_SECOND != 0;
        layout.footer_from_second_page = number & FOOTER_SECOND != 0;
        let unknown =
            number & !(0x7F | LANDSCAPE | ONE_LAYOUT | MIRROR | HEADER_SECOND | FOOTER_SECOND);
        if unknown != 0 {
            layout
                .legacy
                .push("lay.number.unknown", format!("{unknown:#x}"));
        }
        for (side, values) in [("rght", raw.right.as_ref()), ("lft", raw.left.as_ref())] {
            let Some(values) = values else { continue };
            match self.page_geometry(values, side) {
                Some((geometry, kept)) => {
                    if side == "rght" {
                        layout.right = geometry;
                    } else {
                        layout.left = Some(geometry);
                    }
                    for (key, value) in kept {
                        layout.legacy.push(key, value);
                    }
                }
                None => {
                    self.warnings.push(format!(
                        "layout {:?}: unreadable [{side}] kept as raw data",
                        raw.name
                    ));
                    layout
                        .legacy
                        .push(format!("section [{side}]"), values.join("\n"));
                }
            }
        }
        // Left header or footer frames without a [lft] page (corpus: one
        // inserted layout): the left pages take the right pages' geometry.
        if layout.left.is_none()
            && frames
                .iter()
                .any(|(slot, _)| matches!(slot, Slot::HeaderLeft | Slot::FooterLeft))
        {
            layout.left = Some(layout.right.clone());
            layout.legacy.push("lay.left-from-right", "1");
        }
        if layout.mirror && layout.left.is_none() {
            layout.mirror = false;
            layout.legacy.push("lay.mirror-without-left", "1");
        }
        for (name, value) in &raw.unknown {
            layout
                .legacy
                .push(format!("section [{name}]"), value.clone());
        }
        // The order of the layout's frames when it is not Ami Pro's usual
        // one (left header, right header, left footer, right footer, then
        // repeating frames): inserted layouts keep the order they were made
        // in. The writer follows it.
        let order: Vec<&str> = frames.iter().map(|(slot, _)| slot.name()).collect();
        let mut usual = order.clone();
        usual.sort_by_key(|name| Slot::NAMES.iter().position(|usual| usual == name));
        if order != usual {
            layout.legacy.push("lay.order", order.join(" "));
        }
        // Insert before its frames so that a frame's style can see it.
        self.document.layouts_mut().push(Arc::new(layout));
        for (slot, prepared) in frames {
            let side = match slot {
                Slot::HeaderLeft | Slot::FooterLeft => PageSide::Left,
                _ => PageSide::Right,
            };
            let placement = match slot {
                Slot::HeaderRight | Slot::HeaderLeft => Placement::Header { side },
                Slot::FooterRight | Slot::FooterLeft => Placement::Footer { side },
                Slot::Repeating => {
                    let flags: u32 = field(prepared.raw, 2).unwrap_or(0);
                    Placement::Repeating {
                        pages: if flags & RIGHT_LEFT != 0 {
                            RepeatPages::RightLeft
                        } else {
                            RepeatPages::All
                        },
                        rect: frame_rect(prepared.raw),
                    }
                }
            };
            let has_left = self
                .document
                .layout(id)
                .is_some_and(|layout| layout.left.is_some());
            let target = self.document.layout(id).map(|layout| match slot {
                Slot::HeaderRight => layout.header.right,
                Slot::HeaderLeft => layout.header.left,
                Slot::FooterRight => layout.footer.right,
                Slot::FooterLeft => layout.footer.left,
                Slot::Repeating => None,
            });
            if side == PageSide::Left && !has_left || target.flatten().is_some() {
                self.warnings.push(format!(
                    "layout {:?}: a header or footer frame has no slot and was kept as raw data",
                    raw.name
                ));
                if let Some(layout) = self.document.layout_mut(id) {
                    layout
                        .legacy
                        .push("section [lyfrm]", prepared.raw.values.join("\n"));
                }
                continue;
            }
            let frame = self.frame(prepared, Some(placement));
            let layout = self
                .document
                .layout_mut(id)
                .expect("the layout was just added");
            match slot {
                Slot::HeaderRight => layout.header.right = Some(frame),
                Slot::HeaderLeft => layout.header.left = Some(frame),
                Slot::FooterRight => layout.footer.right = Some(frame),
                Slot::FooterLeft => layout.footer.left = Some(frame),
                Slot::Repeating => layout.repeating.push(frame),
            }
        }
    }

    /// A page's geometry, with the values the model does not hold, keyed by
    /// `side`, for the writer.
    fn page_geometry(
        &mut self,
        values: &[String],
        side: &str,
    ) -> Option<(PageGeometry, Vec<(String, String)>)> {
        let parsed = layout_values(values, 0, side, crate::sheet::PAGE_DEFAULTS)?;
        if parsed.nonuniform {
            self.stats.nonuniform_page_columns += 1;
        }
        let border = parsed
            .page_border
            .and_then(|(style, colour, mask, position)| {
                Some(amipro_document::Border {
                    sides: sides(mask),
                    style,
                    colour,
                    position: line_position(position)?,
                })
            });
        let (height, width) = parsed.extent;
        let mut kept = parsed.legacy;
        let mut layout = parsed.layout;
        let derived = crate::sheet::page_columns(width, &layout);
        let text_width =
            i64::from(width) - i64::from(layout.margins.left) - i64::from(layout.margins.right);
        layout.column_edges = uneven_edges(&parsed.edges, derived[0].0, text_width);
        if layout.column_edges.is_none()
            && let Some(offsets) = crate::sheet::edge_offsets(&parsed.edges, &derived)
        {
            kept.push((format!("{side}.edges"), offsets));
        }
        let geometry = PageGeometry {
            width,
            height,
            layout,
            border,
        };
        let margins = geometry.layout.margins;
        if width <= 0
            || height <= 0
            || i64::from(margins.left) + i64::from(margins.right) >= i64::from(width)
            || i64::from(margins.top) + i64::from(margins.bottom) >= i64::from(height)
        {
            return None;
        }
        Some((geometry, kept))
    }

    /// `left` (absolute page twips) as an offset from the left edge of the
    /// document layout's column that it falls in, on page `page`'s side.
    fn column_offset(&self, left: Twips, page: u32) -> Twips {
        let layout = self.document.document_layout();
        let geometry = match (&layout.left, page % 2 == 0) {
            (Some(left_pages), true) => left_pages,
            _ => &layout.right,
        };
        let margins = geometry.layout.margins;
        let column = geometry
            .layout
            .edges(geometry.text_width())
            .iter()
            .map(|edges| i64::from(margins.left) + i64::from(edges.left))
            .take_while(|x| *x <= i64::from(left))
            .last()
            .unwrap_or(i64::from(margins.left));
        twips((i64::from(left) - column).max(0))
    }

    /// The margin band a header or footer frame covers on the layout being
    /// built (the last one added).
    fn margin_band(&self, placement: Placement) -> Option<Rect> {
        let layout = self.document.layouts().last()?;
        let (side, footer) = match placement {
            Placement::Header { side } => (side, false),
            Placement::Footer { side } => (side, true),
            _ => return None,
        };
        let geometry = match side {
            PageSide::Left => layout.left.as_ref().unwrap_or(&layout.right),
            PageSide::Right => &layout.right,
        };
        Some(crate::sheet::margin_band(geometry, footer))
    }

    /// Maps a frame record. `placement` is given for layout frames; page
    /// frames derive theirs from the flags.
    fn frame(&mut self, prepared: &Prepared<'_>, placement: Option<Placement>) -> FrameId {
        let raw = prepared.raw;
        // Top-level `[frm]` records come without a placement; header,
        // footer, and repeating frames with theirs.
        let page_frame = placement.is_none();
        let id = self.document.allocate_frame_id();
        let mut legacy = LegacyData::default();
        let flags: u32 = field(raw, 2).unwrap_or(0);
        let unknown = flags & !KNOWN_FLAGS;
        if unknown != 0 {
            self.stats.unknown_frame_flags |= unknown;
            legacy.push("frame.flags.unknown", format!("{unknown:#x}"));
        }
        let rect = frame_rect(raw);
        let anchored = flags & ANCHORED != 0;
        let anchored_size = || {
            // Field 16 is stale for table frames (`0 65535`, or another
            // frame's size): their size is the rectangle (2.5d, TB1, V20).
            if flags & TABLE != 0 && rect.width() > 0 && rect.height() > 0 {
                return Size {
                    width: rect.width(),
                    height: rect.height(),
                };
            }
            let size = raw
                .values
                .get(15)
                .map(|value| numbers(value))
                .unwrap_or_default();
            let width = size.first().copied().filter(|width| *width > 0);
            let height = size.get(1).copied().filter(|height| *height > 0);
            Size {
                width: width.map_or(rect.width(), twips),
                height: height.map_or(rect.height(), twips),
            }
        };
        // Field 15 is not an offset for table frames either: MERCURY.SAM
        // writes 20000 for a centred table, and corpus tables at the margin
        // write values up to 19456. Their position is the rectangle's left
        // edge, from the left of the column it starts in.
        let table_offset =
            (flags & TABLE != 0).then(|| self.column_offset(rect.left, field(raw, 1).unwrap_or(1)));
        let placement = placement.unwrap_or_else(|| {
            if anchored && flags & WRAP_AROUND != 0 {
                // For Flow with text, field 16's second value is the distance
                // from the frame's top to the baseline, not the height: a
                // frame dragged down hangs below the baseline (F18), and a
                // taller one keeps that distance (F19). The height is the
                // rectangle's.
                let size = anchored_size();
                if flags & TABLE == 0 && rect.height() > 0 {
                    Placement::Inline {
                        y_offset: rect.height() - size.height,
                        size: Size {
                            height: rect.height(),
                            ..size
                        },
                    }
                } else {
                    Placement::Inline { y_offset: 0, size }
                }
            } else if anchored {
                // Field 16's height is stale for With para above frames too:
                // corpus text frames store 0, 200, or 360 with rectangles 6,000
                // to 9,500 twips tall, and Ami Pro lays pictures out at the
                // rectangle's height when the two differ (VM, 2026-10-12). The
                // widths always agree. A degenerate rectangle, made one twip
                // tall, keeps field 16.
                let size = anchored_size();
                Placement::BelowParagraph {
                    x_offset: table_offset
                        .unwrap_or_else(|| twips(field::<i64>(raw, 15).unwrap_or(0))),
                    size: if rect.height() > 1 {
                        Size {
                            height: rect.height(),
                            ..size
                        }
                    } else {
                        size
                    },
                }
            } else {
                Placement::Fixed {
                    page: field(raw, 1).unwrap_or(1u32).max(1),
                    rect,
                }
            }
        });
        if !anchored && raw.values.len() > 13 {
            legacy.push("frame.extra", raw.values[13..].join("\n"));
        }
        if anchored {
            // Field 14 is the anchor index as written, recomputed from the
            // anchor order. The writer gives back the rest as read while it
            // still says the same (`frames.rs`): the stored rectangle, whose
            // place layout recomputes; field 15, a cache for Flow with text
            // and a stale value for tables; field 16, stale in parts.
            if raw.values.len() > 16 {
                legacy.push("frame.extra", raw.values[16..].join("\n"));
            }
            let stored = |number| twips(field::<i64>(raw, number).unwrap_or(0));
            legacy.push(
                "frame.anchored.rect",
                format!("{} {} {} {}", stored(3), stored(4), stored(5), stored(6)),
            );
            for (number, key) in [(15, "frame.field15"), (16, "frame.field16")] {
                if let Some(value) = raw.values.get(number - 1) {
                    legacy.push(key, value.clone());
                }
            }
        }
        // A frame in a cell: its stored page and rectangle, a cache that
        // Ami Pro recomputes from the cell when it lays the table out
        // (TF1Y), kept for the writer while the cell keeps its size.
        if flags & IN_CELL != 0 && !anchored {
            let stored = |number| field::<i64>(raw, number).unwrap_or(0);
            legacy.push(
                "frame.cell.rect",
                format!(
                    "{} {} {} {} {}",
                    stored(1),
                    stored(3),
                    stored(4),
                    stored(5),
                    stored(6)
                ),
            );
        }
        // The order of the frame's parts, when it has parts the model does
        // not know (`[chint]`, `[ehint]`, `[mf]`), so that the writer puts
        // them back where they were.
        if !raw.unknown.is_empty() {
            legacy.push("frame.parts", raw.parts.join(" "));
        }
        // Field 8 as read, when it is not the 1 the writers write.
        if let Some(value) = raw.values.get(7).filter(|value| value.trim() != "1") {
            legacy.push("frame.field8", value.clone());
        }
        // Field 1 is the page only for Where placed frames; field 13 is the
        // frame's sequence number, which Ami Pro assigns once and keeps
        // (decompile `11c8:07dd`), and which the embedded-object directory
        // names. Both are kept for the writer.
        if !matches!(placement, Placement::Fixed { .. })
            && let Some(value) = raw.values.first().filter(|value| value.trim() != "1")
        {
            legacy.push("frame.field1", value.clone());
        }
        if let Some(value) = raw.values.get(12) {
            legacy.push("frame.field13", value.clone());
        }
        // A header or footer frame's rectangle is its page's margin band;
        // where the stored one differs (2 frames in the user's corpus), the
        // difference is kept for the writer.
        let stored = |number| twips(field::<i64>(raw, number).unwrap_or(0));
        let stored = Rect::new(stored(3), stored(4), stored(5), stored(6));
        if let Some(band) = self.margin_band(placement)
            && band != stored
        {
            legacy.push(
                "frame.rect",
                format!(
                    "{} {} {} {}",
                    stored.left - band.left,
                    stored.top - band.top,
                    stored.right - band.right,
                    stored.bottom - band.bottom
                ),
            );
        }
        let wrap = match placement {
            Placement::Inline { .. } => Wrap::Around,
            Placement::BelowParagraph { .. } if flags & NO_WRAP_BESIDE != 0 => Wrap::NoWrapBeside,
            Placement::BelowParagraph { .. } => Wrap::NoWrapAround,
            _ if flags & WRAP_AROUND != 0 => Wrap::Around,
            _ if flags & NO_WRAP_BESIDE != 0 => Wrap::NoWrapBeside,
            _ => Wrap::NoWrapAround,
        };
        let line = raw
            .values
            .get(9)
            .map(|value| numbers(value))
            .unwrap_or_default();
        let line_value = |index: usize| line.get(index).copied().unwrap_or(0);
        let position_value: u32 = field(raw, 9).unwrap_or(1);
        let position = line_position(position_value).unwrap_or_else(|| {
            legacy.push("frame.line-position", position_value.to_string());
            LinePosition::Middle
        });
        // Field 7 as read where the writer would write another value for
        // the same sides (sides without the lines bit, or 0 with it).
        let side_field: u32 = field(raw, 7).unwrap_or(0);
        let shown = frame_sides(side_field, flags & LINES_ON != 0);
        if i64::from(side_field) != crate::sheet::frame_side_mask(shown, None) {
            legacy.push("frame.field7", side_field.to_string());
        }
        let corners = line_value(5);
        if !(0..=100).contains(&corners) {
            legacy.push("frame.corners", corners.to_string());
        }
        let top = rect.top;
        let mut style = FrameStyle {
            wrap,
            opaque: flags & OPAQUE != 0,
            border: amipro_document::Border {
                sides: frame_sides(field(raw, 7).unwrap_or(0), flags & LINES_ON != 0),
                style: u16::try_from(line_value(0)).unwrap_or(1),
                colour: field(raw, 11).unwrap_or(0),
                position,
            },
            shadow: Shadow {
                left: twips(line_value(1).max(0)),
                top: twips(line_value(2).max(0)),
                right: twips(line_value(3).max(0)),
                bottom: twips(line_value(4).max(0)),
                colour: u32::try_from(line_value(6)).unwrap_or(0),
            },
            corners: corners.clamp(0, 100) as u8,
            background: field(raw, 12).unwrap_or(amipro_document::WHITE),
            interior: FrameStyle::default().interior,
            name: raw.name.clone(),
            macro_name: raw.macro_name.clone(),
        };
        if !raw.interior.is_empty() {
            // A page frame's values 10 to 14 are not a header's (FC1).
            let defaults = if page_frame {
                crate::sheet::PAGE_FRAME_DEFAULTS
            } else {
                crate::sheet::FRAME_DEFAULTS
            };
            match layout_values(&raw.interior, top, "frmlay", defaults) {
                Some(parsed) => {
                    style.interior = parsed.layout;
                    let derived = crate::sheet::frame_columns(rect, &style);
                    let (start, end) = crate::sheet::frame_text_area(rect, &style);
                    let text_width = end - start;
                    style.interior.column_edges =
                        uneven_edges(&parsed.edges, derived[0].0, text_width);
                    for (key, value) in parsed.legacy {
                        legacy.push(key, value);
                    }
                    if style.interior.column_edges.is_none()
                        && let Some(offsets) = crate::sheet::edge_offsets(&parsed.edges, &derived)
                    {
                        legacy.push("frmlay.edges", offsets);
                    }
                    // The first value is the frame's bottom in every text and
                    // picture frame, but a table's carries over from earlier
                    // layouts (353 of 406 corpus tables differ, with no rule
                    // found): kept as its offset from the bottom, since Ami
                    // Pro gives the frame that room (MERCURY.SAM's table
                    // written with its bottom cost the page its picture,
                    // VM 2026-10-13).
                    // Measured from the stored bottom, which the writer
                    // writes, not the rectangle made usable above.
                    let bottom = i64::from(twips(field::<i64>(raw, 6).unwrap_or(0)));
                    if let Some(first) = raw
                        .interior
                        .first()
                        .and_then(|value| value.trim().parse::<i64>().ok())
                        && first != bottom
                    {
                        legacy.push("frmlay.1", (first - bottom).to_string());
                    }
                }
                None => legacy.push("section [frmlay]", raw.interior.join("\n")),
            }
        }
        for (name, value) in &raw.unknown {
            legacy.push(format!("section [{name}]"), value.clone());
        }
        let content = self.content(prepared, flags, &mut legacy);
        self.document.insert_frame(Frame {
            id,
            placement,
            style,
            group: None,
            content,
            legacy,
        });
        id
    }

    fn content(
        &mut self,
        prepared: &Prepared<'_>,
        flags: u32,
        legacy: &mut LegacyData,
    ) -> FrameContent {
        match &prepared.raw.content {
            RawContent::Text(_) => {
                let text = prepared.text.as_deref().unwrap_or_default();
                // A `[txt]` with no paragraph at all (a frame never typed in;
                // 1,289 of 2,989 frame texts in both corpora) against one
                // with an empty paragraph: the model's flow holds one either
                // way, so the writer is told.
                if text.is_empty() {
                    legacy.push("txt", "none");
                }
                FrameContent::Text(self.flow(text))
            }
            RawContent::Table(table) => match self.table(table, &prepared.cells) {
                Some(table) => FrameContent::Table(table),
                None => {
                    self.warnings
                        .push("a table has an unreadable shape and was kept as raw data".into());
                    let mut raw = LegacyData::default();
                    raw.push("section [tbl]", table.header.join("\n"));
                    FrameContent::Legacy(raw)
                }
            },
            RawContent::Picture(values) => self.picture(prepared.raw, values),
            RawContent::Unreadable(raw) => {
                let mut kept = LegacyData::default();
                kept.push("section [frm]", raw.clone());
                FrameContent::Legacy(kept)
            }
            RawContent::None => {
                if flags & (TABLE | PICTURE) != 0 {
                    self.warnings
                        .push("a table or picture frame has no content section".into());
                    legacy.push("frame.content", "missing");
                } else {
                    // No content part (a drawing stored as `[mf]`): the
                    // writer writes none while the frame stays empty.
                    legacy.push("frame.content", "none");
                }
                FrameContent::Text(self.flow(&[]))
            }
        }
    }

    fn table(
        &mut self,
        table: &crate::structure::RawTable,
        cells: &[PreparedCell],
    ) -> Option<Table> {
        self.warnings.extend(table.warnings.iter().cloned());
        let header = numbers(table.header.first()?);
        let rows = usize::try_from(*header.first()?).ok()?;
        let columns = usize::try_from(*header.get(1)?).ok()?;
        if rows == 0
            || columns == 0
            || rows > MAX_TABLE_ROWS
            || columns > MAX_TABLE_COLUMNS
            || rows * columns > MAX_TABLE_CELLS
        {
            return None;
        }
        let default_height = twips(header.get(2).copied().unwrap_or(0).max(0));
        let default_width = twips(header.get(4).copied().unwrap_or(1_440).max(0));
        // Header values 4 and 6: the table's gutters after each row and
        // each column (TB1, V20, README31, TF14).
        let defaults = CellMargins::default();
        let cell_margins = CellMargins {
            horizontal: header
                .get(5)
                .map_or(defaults.horizontal, |value| twips((*value).max(0))),
            vertical: header
                .get(3)
                .map_or(defaults.vertical, |value| twips((*value).max(0))),
        };
        let mut heights = vec![default_height; rows];
        let mut widths = vec![default_width; columns];
        // Header values 8 and 9, the insets above the first row and left of
        // the first column (43 when absent, `1428:0000`; VM TF14X).
        let inset = |index: usize| {
            header
                .get(index)
                .map_or(TableSpacing::INSET, |value| twips((*value).max(0)))
        };
        let mut spacing = TableSpacing {
            top: inset(7),
            left: inset(8),
            ..TableSpacing::default()
        };
        // Header value 7: bit 1 Automatic row height, bit 0x100 Rows span
        // pages (VM TP2E, TP5); a header without it is a new table's.
        let mut row_options = TableRows::default();
        if let Some(flags) = header.get(6) {
            row_options.automatic = flags & 1 != 0;
            row_options.span_pages = flags & 0x100 != 0;
        }
        // A row record's flag 0x10 marks a heading row and 0x20 a page break
        // before the row (`1428:046b`; VM TP2, TP6).
        for record in &table.rows {
            let values = numbers(record);
            if let (Some(index), Some(flags)) = (values.first(), values.get(3))
                && let Some(index) = usize::try_from(*index).ok().filter(|index| *index < rows)
            {
                if flags & 0x10 != 0 {
                    row_options.headings.insert(index);
                }
                if flags & 0x20 != 0 {
                    row_options.page_breaks.insert(index);
                }
            }
        }
        for (records, sizes, gutters) in [
            (&table.rows, &mut heights, &mut spacing.row_gutters),
            (&table.columns, &mut widths, &mut spacing.column_gutters),
        ] {
            for record in records {
                let values = numbers(record);
                if let (Some(index), Some(size)) = (values.first(), values.get(1))
                    && let Some(index) = usize::try_from(*index).ok()
                    && let Some(slot) = sizes.get_mut(index)
                {
                    *slot = twips((*size).max(0));
                    // A record's gutter (value 3) is the row's or column's
                    // own only with flag 4 (value 4), else the table's
                    // (`1428:046b`; READMEBP.SAM, VM BPS).
                    if let (Some(gutter), Some(flags)) = (values.get(2), values.get(3))
                        && flags & 4 != 0
                    {
                        gutters.insert(index, twips((*gutter).max(0)));
                    }
                }
            }
        }
        // Modify Table Layout's defaults (values 3 and 5) and options (value
        // 7: 0x4 Center, 0x80 Honor protection, 0x400 Disable mouse
        // sizing; VM TE13, TE12, TE15); a row or column with a size of its
        // own has flag 2 (TE5; a record without it keeps its size as read).
        let options = header.get(6).copied().unwrap_or(1);
        let mut layout = TableLayout {
            row_height: default_height,
            column_width: default_width,
            center: options & 0x4 != 0,
            honor_protection: options & 0x80 != 0,
            fixed_sizes: options & 0x400 != 0,
            ..TableLayout::default()
        };
        for (records, sizes, own) in [
            (&table.rows, &heights, &mut layout.own_heights),
            (&table.columns, &widths, &mut layout.own_widths),
        ] {
            for record in records {
                let values = numbers(record);
                if let (Some(index), Some(flags)) = (values.first(), values.get(3))
                    && let Ok(index) = usize::try_from(*index)
                    && index < sizes.len()
                    && flags & 2 != 0
                {
                    own.insert(index);
                }
            }
        }
        // An Automatic table's rows are as tall as their text: Ami Pro flags
        // every row it measured (TE2–TE7), and none has a height of its own.
        if options & 1 != 0 {
            layout.own_heights.clear();
        }
        let mut legacy = LegacyData::default();
        legacy.push("tbl.header", table.header.join("\n"));
        if !table.rows.is_empty() {
            legacy.push("tbl.h", table.rows.join("\n"));
        }
        if !table.columns.is_empty() {
            legacy.push("tbl.w", table.columns.join("\n"));
        }
        for (name, value) in &table.unknown {
            legacy.push(format!("section [{name}]"), value.clone());
        }
        let mut grid: Vec<Option<&PreparedCell>> = vec![None; rows * columns];
        for cell in cells {
            let record = &cell.0;
            let values = numbers(record);
            let position = match (values.first(), values.get(1)) {
                (Some(row), Some(column))
                    if (0..rows as i64).contains(row) && (0..columns as i64).contains(column) =>
                {
                    Some(*row as usize * columns + *column as usize)
                }
                _ => None,
            };
            match position {
                Some(position) if grid[position].is_none() => grid[position] = Some(cell),
                _ => {
                    self.warnings.push(format!(
                        "a table cell record {record:?} is outside the grid"
                    ));
                    legacy.push("tbl.cell.dropped", record.clone());
                }
            }
        }
        let spans: Vec<CellSpan> = grid
            .iter()
            .map(|cell| cell.map_or(CellSpan::SINGLE, |(record, _, _, _)| cell_span(record)))
            .collect();
        // The user's guide: Ami Pro assigns Table Text to all table cells.
        let table_text = self.style(Some(TABLE_TEXT));
        let cells: Vec<TableCell> = grid
            .into_iter()
            .zip(&spans)
            .map(|(cell, span)| {
                let mut legacy = LegacyData::default();
                let format = cell.map_or_else(CellFormat::default, |(record, _, _, _)| {
                    CellFormat::from_record(&numbers(record))
                });
                let mut formula = None;
                let paragraphs = match cell {
                    Some((record, paragraphs, extra, found)) => {
                        legacy.push("cell.record", record.clone());
                        for body in extra {
                            legacy.push("cell.body", body.clone());
                        }
                        formula = found.clone();
                        // Bit 0x40 with no formula body at all: kept as read.
                        if formula.is_none()
                            && extra.is_empty()
                            && numbers(record).get(2).is_some_and(|f3| f3 & 0x40 != 0)
                        {
                            legacy.push("cell.formula.missing", "1");
                        }
                        paragraphs.as_slice()
                    }
                    None => &[],
                };
                TableCell {
                    content: CellContent::Text(self.flow_in(paragraphs, table_text)),
                    span: *span,
                    frame: None,
                    format,
                    formula,
                    legacy,
                }
            })
            .collect();
        let mut table = Table {
            columns: widths,
            rows: heights,
            cell_margins,
            spacing,
            row_options,
            layout,
            read_rows: Default::default(),
            cells,
            legacy,
        };
        if table.validate_spans().is_err() {
            self.warnings
                .push("a table's joined cells do not fit its grid; kept as single cells".into());
            for cell in &mut table.cells {
                cell.span = CellSpan::SINGLE;
            }
        }
        Some(table)
    }

    fn picture(&mut self, raw: &RawFrame, values: &[String]) -> FrameContent {
        let mut legacy = LegacyData::default();
        legacy.push("isd", values.join("\n"));
        let extension = values.get(1).cloned().unwrap_or_default();
        let stored = values.get(2).cloned().unwrap_or_default();
        let sequence: Option<u32> = field(raw, 13);
        let entry = sequence
            .and_then(|sequence| self.embedded.get(&sequence))
            .cloned();
        let bytes = |range: Option<(usize, usize)>, file: &[u8]| {
            range.map(|(offset, length)| Arc::<[u8]>::from(&file[offset..offset + length]))
        };
        let rendering = entry
            .as_ref()
            .and_then(|entry| bytes(entry.rendering, self.file))
            .map(|data| self.document.add_object(data));
        let source = match entry
            .as_ref()
            .and_then(|entry| bytes(entry.original, self.file))
        {
            Some(data) => PictureSource::Embedded(self.document.add_object(data)),
            None => {
                let path = entry
                    .as_ref()
                    .and_then(|entry| entry.path.clone())
                    .or_else(|| (!stored.eq_ignore_ascii_case(".X3")).then_some(stored));
                match path {
                    Some(path) if !path.is_empty() => PictureSource::Linked(path),
                    _ => {
                        self.warnings
                            .push("a picture frame's bytes are missing; kept as raw data".into());
                        return FrameContent::Legacy(legacy);
                    }
                }
            }
        };
        let mut picture = Picture::new(
            source,
            extension,
            Size {
                width: 1_440,
                height: 1_440,
            },
        );
        picture.rendering = rendering;
        picture.legacy = legacy;
        picture.adjust = picture.kept_adjust().unwrap_or_default();
        // The scaling and pan Ami Pro stored (`11c8:2e29`); values it would
        // not have written are made the nearest it would.
        match picture.kept_scaling() {
            Some((scaling, pan)) => {
                picture.scaling = scaling;
                picture.pan = pan;
                let scaling = &mut picture.scaling;
                if scaling.percent == 0 || scaling.rotation >= 360 {
                    scaling.percent = scaling.percent.max(1);
                    scaling.rotation %= 360;
                    self.warnings
                        .push("a picture's percentage or rotation is out of range".into());
                }
                if scaling.sizing == PictureSizing::Custom {
                    let clamp = |value: i32| value.clamp(1, PICTURE_MAX);
                    if clamp(scaling.size.width) != scaling.size.width
                        || clamp(scaling.size.height) != scaling.size.height
                    {
                        scaling.size.width = clamp(scaling.size.width);
                        scaling.size.height = clamp(scaling.size.height);
                        self.warnings
                            .push("a picture's custom size is out of range".into());
                    }
                }
            }
            None => self
                .warnings
                .push("a picture's scaling does not read; it fits its frame".into()),
        }
        FrameContent::Picture(picture)
    }

    /// Links each cell that names a frame to it (`CELL_FRAME_NOTES.md`): a
    /// cell record's value 11 says the cell holds a frame and value 12 which,
    /// counting the frames with bit `0x20` in file order from 0 (Ami Pro's
    /// list of frames in cells, `1428:07ce`); such a frame is then in its
    /// cell. A frame no cell names, or one that cannot be in a cell (grouped,
    /// a table), stays where its record put it.
    fn cells_hold_frames(&mut self) {
        let listed: Vec<FrameId> = self
            .document
            .frame_stack()
            .iter()
            .copied()
            .filter(|id| {
                self.document
                    .frame(*id)
                    .is_some_and(|frame| frame.legacy.get("frame.cell.rect").is_some())
            })
            .collect();
        let tables: Vec<FrameId> = self
            .document
            .frames()
            .filter(|frame| matches!(frame.content, FrameContent::Table(_)))
            .map(|frame| frame.id)
            .collect();
        let mut claimed: BTreeSet<FrameId> = BTreeSet::new();
        for table_id in tables {
            let Some(FrameContent::Table(table)) = self
                .document
                .frame(table_id)
                .map(|frame| frame.content.clone())
            else {
                continue;
            };
            let mut cells = table.cells.clone();
            let mut any = false;
            for cell in &mut cells {
                let values = cell
                    .legacy
                    .get("cell.record")
                    .map(numbers)
                    .unwrap_or_default();
                let (Some(holds), Some(number)) = (values.get(10), values.get(11)) else {
                    continue;
                };
                if holds & 1 == 0 {
                    continue;
                }
                let frame = usize::try_from(*number)
                    .ok()
                    .and_then(|number| listed.get(number))
                    .copied()
                    .filter(|frame| !claimed.contains(frame) && !cell.span.is_covered())
                    .filter(|frame| {
                        self.document.frame(*frame).is_some_and(|frame| {
                            frame.group.is_none()
                                && !matches!(frame.content, FrameContent::Table(_))
                        })
                    });
                match frame {
                    Some(frame) => {
                        claimed.insert(frame);
                        cell.frame = Some(frame);
                        any = true;
                    }
                    None => {
                        self.warnings.push(format!(
                            "a table cell names frame {number} of the frames in cells, which is not there"
                        ));
                        cell.legacy.push("cell.frame.unresolved", "1");
                    }
                }
            }
            if let Some(target) = self.document.frame_mut(table_id)
                && let FrameContent::Table(content) = &mut target.content
                && (any || content.cells != cells)
            {
                content.cells = cells;
            }
        }
        for id in listed {
            let Some(frame) = self.document.frame_mut(id) else {
                continue;
            };
            if claimed.contains(&id) {
                frame.placement = Placement::InCell;
                // The writer gives the bit from the placement.
                let unknown = crate::frames::kept_flags(frame) & !IN_CELL;
                frame
                    .legacy
                    .values
                    .retain(|value| value.key != "frame.flags.unknown");
                if unknown != 0 {
                    frame
                        .legacy
                        .push("frame.flags.unknown", format!("{unknown:#x}"));
                }
            } else {
                frame
                    .legacy
                    .values
                    .retain(|value| value.key != "frame.cell.rect");
                self.warnings.push(
                    "a frame marked as in a table cell is in no cell; it stays on its page".into(),
                );
            }
        }
    }

    /// Groups: contiguous runs of page frames from a first to a last member
    /// (Z03, Z04). A run that breaks the model's rules is not grouped.
    fn groups(&mut self, top: &[Prepared<'_>]) {
        let mut current: Option<(GroupId, Vec<FrameId>)> = None;
        let stack: Vec<FrameId> = self.document.frame_stack().to_vec();
        let mut finished = Vec::new();
        for (prepared, id) in top.iter().zip(stack) {
            let flags: u32 = field(prepared.raw, 2).unwrap_or(0);
            if flags & GROUP_MEMBER == 0 {
                if let Some(open) = current.take() {
                    finished.push(open);
                }
                continue;
            }
            if flags & GROUP_FIRST != 0 || current.is_none() {
                if let Some(open) = current.take() {
                    finished.push(open);
                }
                current = Some((self.document.allocate_group_id(), Vec::new()));
            }
            if let Some((_, members)) = current.as_mut() {
                members.push(id);
            }
            if flags & GROUP_LAST != 0 {
                finished.extend(current.take());
            }
        }
        finished.extend(current);
        for (group, members) in finished {
            let pages: BTreeSet<u32> = members
                .iter()
                .filter_map(|member| match self.document.frame(*member)?.placement {
                    Placement::Fixed { page, .. } => Some(page),
                    _ => None,
                })
                .collect();
            if members.len() < 2 || pages.len() != 1 {
                self.warnings
                    .push("a frame group breaks the grouping rules and was not grouped".into());
                continue;
            }
            for member in members {
                if let Some(frame) = self.document.frame_mut(member) {
                    frame.group = Some(group);
                }
            }
        }
    }
}
