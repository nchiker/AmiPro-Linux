//! Tools › TOC, Index…: the table of contents and the index as Ami Pro
//! makes them (`docs/research/TOC_INDEX_NOTES.md`, `docs/Design/UI_SPEC.md`
//! §36). The entries are collected from the document with the help of a
//! [`ContentsHost`], which knows the pages and list numbers the layout
//! shows; the commands then put the TOC's page table into the document
//! (decisions TX2, TX6) as one undo step.

use std::collections::BTreeMap;

use amipro_document::contents::{
    Leader, TOC_LEVELS, TocLevel, TocMark, is_toc_table, mark_toc_table,
};
use amipro_document::{
    Alignment, BreakKind, CellLeader, CharacterFormat, CharacterRun, Document, FlowId,
    FrameContent, FrameId, Indents, InlineObject, ParagraphStyle, Placement, PowerMark, StyleId,
    StyleOrigin, power_kind,
};

use crate::tables::{NewTable, TableTarget, insert_table};
use crate::{Applied, EditError, TextPosition};

/// What the layout knows that the TOC needs.
pub trait ContentsHost {
    /// The page that `byte` of paragraph `paragraph` of `flow` is on, as
    /// that page's page-number object shows it (`1588:1951`): its leading
    /// text and number style, the TOC break's restart; for text in an
    /// anchored frame, the page of its anchor.
    fn page_text(&self, flow: FlowId, paragraph: usize, byte: usize) -> Option<String> {
        self.page(flow, paragraph, byte).map(|(_, text)| text)
    }
    /// The same page's index among the document's pages (0 for the first),
    /// with its text as shown.
    fn page(&self, flow: FlowId, paragraph: usize, byte: usize) -> Option<(usize, String)>;
    /// The paragraph's list number as shown (Bullets and numbers), if any.
    fn number_text(&self, flow: FlowId, paragraph: usize) -> Option<String>;
}

/// One collected TOC entry.
#[derive(Clone, Debug, PartialEq)]
pub struct TocEntry {
    /// 1 to 9.
    pub level: u8,
    pub text: String,
    /// The text's bold, italic, underlines, and the like (fonts, sizes,
    /// and colours dropped, `1588:1083`).
    pub runs: Vec<CharacterRun>,
    /// The page as shown, if the layout placed the entry.
    pub page: Option<String>,
    /// Where the entry comes from (a heading, or a TOC mark).
    pub source: TextPosition,
}

/// TOC Options' first state (`1450:0624`): each style whose outline level
/// no other style took yet at that level, with page numbers right aligned
/// after dots; the rest None.
pub fn default_toc_levels(document: &Document) -> Vec<TocLevel> {
    let mut by_level: BTreeMap<u8, &ParagraphStyle> = BTreeMap::new();
    for style in document.styles().values() {
        let level = style.bullets.level;
        if (1..=TOC_LEVELS as u8).contains(&level) {
            by_level.entry(level).or_insert(style);
        }
    }
    by_level
        .values()
        .map(|style| TocLevel {
            style: style.name.clone(),
            page_number: true,
            right_align: true,
            leader: Leader::Dots,
        })
        .collect()
}

/// The document's TOC levels: its `[toc]`, else TOC Options' first state.
pub fn toc_levels(document: &Document) -> Vec<TocLevel> {
    match &document.contents().toc {
        Some(toc) => toc.levels(),
        None => default_toc_levels(document),
    }
}

/// The options of an entry's level. A TOC mark of a level no style has
/// gets its page right aligned without a leader when the document has TOC
/// levels (TI1T2's level 3), and after its text without a separator when it
/// has none (TI3T: "FirstPage 1") (TQ1).
fn level_options(levels: &[TocLevel], level: u8) -> TocLevel {
    levels.get(level as usize - 1).cloned().unwrap_or(TocLevel {
        style: String::new(),
        page_number: true,
        right_align: !levels.is_empty(),
        leader: Leader::None,
    })
}

/// Collects the TOC's entries in document order (`1588:0239`): the main
/// text and the text frames anchored in it (With para above and Flow with
/// text), a paragraph's TOC marks if it has any, else the paragraph when
/// its style is a level's, with its list number. Empty paragraphs are
/// skipped.
pub fn collect_toc(
    document: &Document,
    levels: &[TocLevel],
    host: &dyn ContentsHost,
) -> Vec<TocEntry> {
    let mut entries = Vec::new();
    let main = document.main_flow_id();
    collect_flow(document, main, levels, host, &mut entries, 0);
    entries
}

fn collect_flow(
    document: &Document,
    flow: FlowId,
    levels: &[TocLevel],
    host: &dyn ContentsHost,
    entries: &mut Vec<TocEntry>,
    depth: usize,
) {
    let Some(text_flow) = document.flow(flow) else {
        return;
    };
    for (index, paragraph) in text_flow.paragraphs.iter().enumerate() {
        let marks: Vec<(usize, TocMark)> = paragraph
            .marks
            .iter()
            .filter_map(|mark| {
                let power = PowerMark::parse(&mark.raw)?;
                (!power.end && power.kind == power_kind::TOC)
                    .then(|| TocMark::parse(&power.code).map(|toc| (mark.byte, toc)))
                    .flatten()
            })
            .collect();
        if !marks.is_empty() {
            for (byte, mark) in marks {
                let source = TextPosition {
                    flow,
                    paragraph: index,
                    byte,
                };
                entries.push(TocEntry {
                    level: mark.level,
                    text: mark.text,
                    runs: Vec::new(),
                    page: host.page_text(flow, index, byte),
                    source,
                });
            }
        } else if let Some(style) = document.styles().get(&paragraph.style)
            && let Some(level) = levels
                .iter()
                .position(|level| level.style.eq_ignore_ascii_case(&style.name))
        {
            let (mut text, mut runs) = entry_text(paragraph);
            if let Some(number) = host.number_text(flow, index) {
                let prefix = if number.ends_with(' ') {
                    number
                } else {
                    format!("{number} ")
                };
                for run in &mut runs {
                    run.start += prefix.len();
                    run.end += prefix.len();
                }
                text.insert_str(0, &prefix);
            }
            if !text.trim().is_empty() {
                entries.push(TocEntry {
                    level: level as u8 + 1,
                    text,
                    runs,
                    page: host.page_text(flow, index, 0),
                    source: TextPosition {
                        flow,
                        paragraph: index,
                        byte: 0,
                    },
                });
            }
        }
        // The text frames anchored in this paragraph follow it.
        if depth < 4 {
            for object in &paragraph.objects {
                if let InlineObject::FrameAnchor(frame) = object
                    && let Some(frame) = document.frame(*frame)
                    && matches!(
                        frame.placement,
                        Placement::BelowParagraph { .. } | Placement::Inline { .. }
                    )
                    && let FrameContent::Text(inner) = frame.content
                {
                    collect_flow(document, inner, levels, host, entries, depth + 1);
                }
            }
        }
    }
}

/// A paragraph's text for the TOC: without its objects, line breaks as
/// spaces; its attribute runs kept, fonts, sizes, and colours dropped.
fn entry_text(paragraph: &amipro_document::Paragraph) -> (String, Vec<CharacterRun>) {
    let mut text = String::with_capacity(paragraph.text.len());
    // New byte offset for each old one.
    let mut map = vec![0usize; paragraph.text.len() + 1];
    for (at, character) in paragraph.text.char_indices() {
        for slot in &mut map[at..at + character.len_utf8()] {
            *slot = text.len();
        }
        match character {
            '\u{FFFC}' | '\u{00AD}' => {}
            '\u{2028}' | '\t' => text.push(' '),
            other => text.push(other),
        }
    }
    map[paragraph.text.len()] = text.len();
    let runs = paragraph
        .runs
        .iter()
        .filter_map(|run| {
            let format = CharacterFormat {
                font_family: None,
                font_size_points: None,
                colour: None,
                ..run.format.clone()
            };
            let (start, end) = (
                map[run.start.min(paragraph.text.len())],
                map[run.end.min(paragraph.text.len())],
            );
            (start < end && format != CharacterFormat::PLAIN).then_some(CharacterRun {
                start,
                end,
                format,
            })
        })
        .collect();
    (text, runs)
}

/// Where Generate puts a TOC (TX2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TocPlace {
    /// At the start of the document, before a page break after which page
    /// numbers start again (Ami Pro's TOC with no output file); a TOC
    /// already there is replaced.
    Start,
    /// At a position in the main text (TX2's improvement).
    At(TextPosition),
    /// A new document's whole text (Ami Pro's output file on `_TOC.STY`):
    /// the table alone, without the TOC flag or a break.
    Output,
}

/// Generate Contents: the entries as Ami Pro's two-column page table.
#[derive(Clone, Debug, PartialEq)]
pub struct TocRequest {
    pub place: TocPlace,
    pub levels: Vec<TocLevel>,
    pub entries: Vec<TocEntry>,
    /// The width the table fills (the main text's column), in twips.
    pub width: i32,
}

/// The page column's width: Ami Pro's TOC tables keep 914 twips for the
/// pages (TI1T2, TI2TOC: 8274 and 914 in a 9360 column).
const PAGE_COLUMN: i32 = 914;

/// The document's generated TOC table, if it has one.
pub fn toc_table(document: &Document) -> Option<FrameId> {
    document
        .frames()
        .find(|frame| is_toc_table(frame))
        .map(|frame| frame.id)
}

/// The styles TOCn and TOCPGn (`1588:0a1f`): made when missing, TOCn as a
/// copy of the level n+1 style (Body Text for level 9) indented by level,
/// TOCPGn the same right aligned without indents (TI1T2).
fn toc_styles(next: &mut Document, levels: &[TocLevel]) -> Vec<(StyleId, StyleId)> {
    let body = next.body_text_style();
    let style_of = |next: &Document, name: &str| {
        next.styles()
            .values()
            .find(|style| style.name.eq_ignore_ascii_case(name))
            .map(|style| style.id)
    };
    let mut ids = Vec::with_capacity(TOC_LEVELS);
    for level in 1..=TOC_LEVELS {
        let base = levels
            .get(level)
            .and_then(|next_level| style_of(next, &next_level.style))
            .unwrap_or(body);
        let mut pair = [StyleId(0); 2];
        for (slot, page) in [(0, false), (1, true)] {
            let name = if page {
                format!("TOCPG{level}")
            } else {
                format!("TOC{level}")
            };
            pair[slot] = match style_of(next, &name) {
                Some(id) => id,
                None => {
                    let id = StyleId(next.styles().keys().next_back().map_or(1, |id| id.0 + 1));
                    let mut style = next.styles()[&base].clone();
                    style.id = id;
                    style.name = name;
                    style.parent = None;
                    style.next = None;
                    style.origin = StyleOrigin::Document;
                    style.kept.function_key = 0;
                    if page {
                        style.alignment = Alignment::Right;
                        style.indents = Indents::default();
                    } else {
                        style.alignment = Alignment::Left;
                        style.indents = Indents {
                            all: 0,
                            first: (level as i32 - 1) * 360,
                            rest: level as i32 * 360,
                            right: 0,
                        };
                    }
                    next.styles_mut().insert(id, style);
                    id
                }
            };
        }
        ids.push((pair[0], pair[1]));
    }
    ids
}

fn cell_leader(leader: Leader) -> CellLeader {
    match leader {
        Leader::Dots => CellLeader::Dots,
        Leader::Hyphens => CellLeader::Dashes,
        Leader::Line => CellLeader::Line,
        Leader::None | Leader::Comma => CellLeader::None,
    }
}

/// Puts the TOC's page table where the request says (one undo step).
pub(crate) fn put_toc(document: &Document, request: &TocRequest) -> Result<Applied, EditError> {
    if request.entries.is_empty() {
        return Err("the document has no TOC entries");
    }
    let main = document.main_flow_id();
    let start = TextPosition {
        flow: main,
        paragraph: 0,
        byte: 0,
    };
    // Where the table's anchor goes, after the old table (if any) is
    // removed and the break made.
    let (mut applied, at) = match request.place {
        TocPlace::Start | TocPlace::Output => match toc_table(document) {
            Some(old) if request.place == TocPlace::Start => {
                let at = crate::frames::anchor_position(document, old)
                    .ok_or("the TOC's table has no anchor")?;
                (crate::frames::delete_frame(document, old)?, at)
            }
            _ if request.place == TocPlace::Start => {
                let layout = document.document_layout().id;
                let applied = crate::replace_range(
                    document.clone(),
                    start,
                    start,
                    "\u{FFFC}\n",
                    vec![InlineObject::Break {
                        kind: BreakKind::Page,
                        centre: false,
                        layout: Some(layout),
                        restart: true,
                    }],
                    None,
                )?;
                (applied, start)
            }
            _ => (
                Applied {
                    document: document.clone(),
                    caret: None,
                    report: Default::default(),
                },
                start,
            ),
        },
        TocPlace::At(position) => (
            Applied {
                document: document.clone(),
                caret: None,
                report: Default::default(),
            },
            position,
        ),
    };
    let mut next = applied.document.clone();
    if request.place == TocPlace::Start && toc_table(document).is_none() {
        // The break's paragraph is untagged: Body Text (TI1T2).
        let body = next.body_text_style();
        if let Some(paragraph) = next.main_flow_mut().paragraph_mut(0) {
            paragraph.style = body;
            paragraph.runs.clear();
        }
    }
    let pairs = toc_styles(&mut next, &request.levels);
    let width = request.width.max(PAGE_COLUMN * 2);
    let mut spec = NewTable::fitting(2, request.entries.len(), width, 240);
    let first = width - PAGE_COLUMN - 2 * spec.cell_margins.horizontal;
    spec.column_width = first.max(1);
    let table = insert_table(&next, TableTarget::Text(at), &spec)?;
    let mut next = table.document;
    applied.report.merge(table.report);
    applied.report.styles = true;
    // The frame made (the last one in the stack).
    let frame = *next.frame_stack().last().ok_or("no table was made")?;
    {
        let slot = next.frame_mut(frame).ok_or("no table was made")?;
        if request.place != TocPlace::Output {
            mark_toc_table(slot);
        }
        if let FrameContent::Table(table) = &mut slot.content {
            table.columns = vec![spec.column_width, PAGE_COLUMN];
            // The page numbers' column has its own width (the file says
            // so; reading it back would otherwise differ).
            if PAGE_COLUMN != table.layout.column_width {
                table.layout.own_widths.insert(1);
            }
        }
    }
    let cells: Vec<(FlowId, FlowId)> = match &next.frame(frame).ok_or("no table")?.content {
        FrameContent::Table(table) => table
            .cells
            .chunks(2)
            .map(|row| {
                let amipro_document::CellContent::Text(left) = row[0].content;
                let amipro_document::CellContent::Text(right) = row[1].content;
                (left, right)
            })
            .collect(),
        _ => return Err("no table was made"),
    };
    for (row, entry) in request.entries.iter().enumerate() {
        let options = level_options(&request.levels, entry.level);
        let (text_style, page_style) = pairs[entry.level as usize - 1];
        let page = entry.page.clone().unwrap_or_default();
        let (left_text, right_text, leader) = if !options.page_number {
            (entry.text.clone(), " ".to_string(), CellLeader::None)
        } else if !options.right_align {
            let separator = match options.leader {
                Leader::Comma => ", ",
                _ if options.style.is_empty() => "",
                _ => "  ",
            };
            (
                format!("{}{separator}{page}", entry.text),
                " ".to_string(),
                CellLeader::None,
            )
        } else {
            // The page as shown (TQ2: Ami Pro writes a number with two
            // decimals, `1.00`, and reads either as the number).
            (entry.text.clone(), page, cell_leader(options.leader))
        };
        let (left, right) = cells[row];
        for (flow, text, style, runs) in [
            (left, left_text, text_style, entry.runs.clone()),
            (right, right_text, page_style, Vec::new()),
        ] {
            let paragraph = next
                .flow_mut(flow)
                .and_then(|flow| flow.paragraph_mut(0))
                .ok_or("a TOC cell has no paragraph")?;
            paragraph.text = text;
            paragraph.style = style;
            paragraph.runs = runs;
        }
        if let Some(slot) = next.frame_mut(frame)
            && let FrameContent::Table(table) = &mut slot.content
        {
            table.cells[row * 2].format.leader = leader;
            table.cells[row * 2 + 1].format.leader = leader;
        }
    }
    applied.document = next;
    applied.report.frames.insert(frame);
    applied.caret = Some(at);
    Ok(applied)
}

/// One index mark found in the document, with its page.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexMark {
    pub entry: amipro_document::contents::IndexEntry,
    /// The page's index and its text as shown, if the layout placed it.
    pub page: Option<(usize, String)>,
    pub at: TextPosition,
}

/// Every index mark (kind 8) in the main text and in text frames, in page
/// order and then in text order, as Ami Pro's scan visits them
/// (`1378:0000`).
pub fn collect_index(document: &Document, host: &dyn ContentsHost) -> Vec<IndexMark> {
    let mut flows = vec![document.main_flow_id()];
    flows.extend(document.frames().filter_map(|frame| match frame.content {
        FrameContent::Text(flow) => Some(flow),
        _ => None,
    }));
    let mut marks = Vec::new();
    for (order, flow) in flows.into_iter().enumerate() {
        let Some(text_flow) = document.flow(flow) else {
            continue;
        };
        for (index, paragraph) in text_flow.paragraphs.iter().enumerate() {
            for mark in &paragraph.marks {
                let Some(power) = PowerMark::parse(&mark.raw) else {
                    continue;
                };
                if power.end || power.kind != power_kind::INDEX {
                    continue;
                }
                let Some(entry) = amipro_document::contents::IndexEntry::parse(&power.code) else {
                    continue;
                };
                let at = TextPosition {
                    flow,
                    paragraph: index,
                    byte: mark.byte,
                };
                marks.push((
                    order,
                    IndexMark {
                        entry,
                        page: host.page(flow, index, mark.byte),
                        at,
                    },
                ));
            }
        }
    }
    // By page; within a page the main text first, each flow in text order.
    marks.sort_by_key(|(order, mark)| {
        (
            mark.page.as_ref().map_or(usize::MAX, |(page, _)| *page),
            *order,
            mark.at.paragraph,
            mark.at.byte,
        )
    });
    marks.into_iter().map(|(_, mark)| mark).collect()
}

/// Ami Pro's sort weight of a Windows-1252 byte (`SORTTAB`, `1020:2020`):
/// punctuation first, then spaces, digits, and letters without regard to
/// case or accents; Æ, Ø, Å after z.
pub fn sort_weight(byte: u8) -> u8 {
    match byte {
        0x00..=0x1E => 0,
        0x1F | 0xFF => 0x7F,
        b' ' | 0xA0 => 2,
        b'0'..=b'9' => 3 + (byte - b'0'),
        b'a'..=b'z' => 13 + (byte - b'a'),
        b'A'..=b'Z' => 13 + (byte - b'A'),
        0xC0..=0xC4 | 0xE0..=0xE4 => 13,
        0xC7 | 0xE7 => 15,
        0xD0 => 16,
        0xC8..=0xCB | 0xE8..=0xEB => 17,
        0xCC..=0xCF | 0xEC..=0xEF => 21,
        0xD1 | 0xF1 => 26,
        0xD2..=0xD6 | 0xF2..=0xF6 => 27,
        0xDE | 0xFE => 14,
        0xDF => 31,
        0xD9..=0xDC | 0xF9..=0xFC => 33,
        0xDD | 0xFD => 37,
        0xC6 | 0xE6 => 0x27,
        0xD8 | 0xF8 => 0x28,
        0xC5 | 0xE5 => 0x29,
        _ => 1,
    }
}

/// Compares two entries as Ami Pro's index does: by weights, then byte for
/// byte, so "Apple" comes before "apple" and both stay.
pub fn index_order(left: &str, right: &str) -> std::cmp::Ordering {
    let bytes =
        |text: &str| -> Vec<u8> { text.chars().map(amipro_document::windows_1252).collect() };
    let (left, right) = (bytes(left), bytes(right));
    let weights = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| sort_weight(*byte))
            .collect::<Vec<_>>()
    };
    weights(&left)
        .cmp(&weights(&right))
        .then_with(|| left.cmp(&right))
}

/// Generate Index's options (TX5's ranges are the app's).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IndexOptions {
    /// Include alphabetical separators (`[master]` flag 1).
    pub separators: bool,
    /// Join pages that follow one another: `3–5` (TX5, off by default).
    pub ranges: bool,
}

/// One paragraph of a generated index, in `_INDEX.STY`'s styles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndexLine {
    Separator(String),
    Primary(String),
    Secondary(String),
}

impl IndexLine {
    /// The paragraph style Ami Pro gives it.
    pub fn style(&self) -> &'static str {
        match self {
            Self::Separator(_) => "Separator",
            Self::Primary(_) => "Primary",
            Self::Secondary(_) => "Secondary",
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Self::Separator(text) | Self::Primary(text) | Self::Secondary(text) => text,
        }
    }
}

/// A primary entry: its text, its own pages and Other, its secondaries.
type Primary = (String, Heading, Vec<(String, Heading)>);

#[derive(Default)]
struct Heading {
    pages: Vec<String>,
    others: Vec<String>,
}

impl Heading {
    fn add(&mut self, entry: &amipro_document::contents::IndexEntry, page: Option<&str>) {
        if entry.page_number
            && let Some(page) = page
            && self.pages.last().map(String::as_str) != Some(page)
        {
            self.pages.push(page.to_string());
        }
        if !entry.other.is_empty() && !self.others.contains(&entry.other) {
            self.others.push(entry.other.clone());
        }
    }

    fn text(&self, heading: &str, ranges: bool) -> String {
        let mut text = heading.to_string();
        for page in joined_pages(&self.pages, ranges) {
            text.push_str(", ");
            text.push_str(&page);
        }
        for other in &self.others {
            text.push_str(other);
        }
        text
    }
}

/// Pages, with runs of consecutive numbers joined when asked (TX5).
fn joined_pages(pages: &[String], ranges: bool) -> Vec<String> {
    if !ranges {
        return pages.to_vec();
    }
    let mut out: Vec<String> = Vec::new();
    let mut run: Option<(u32, u32)> = None;
    let flush = |run: &mut Option<(u32, u32)>, out: &mut Vec<String>| {
        if let Some((first, last)) = run.take() {
            out.push(if first == last {
                first.to_string()
            } else {
                format!("{first}\u{2013}{last}")
            });
        }
    };
    for page in pages {
        match (page.parse::<u32>(), run) {
            (Ok(number), Some((first, last))) if number == last + 1 => run = Some((first, number)),
            (Ok(number), _) => {
                flush(&mut run, &mut out);
                run = Some((number, number));
            }
            (Err(_), _) => {
                flush(&mut run, &mut out);
                out.push(page.clone());
            }
        }
    }
    flush(&mut run, &mut out);
    out
}

/// The index's paragraphs (`1378:05ec`, `1378:0a25`, `1378:0b0e`): entries
/// sorted by Ami Pro's weights, equal primaries and secondaries merged,
/// each one's pages after ", " in page order without repeats, its Other
/// text after them; a separator before each new first letter (each digit
/// its own) when asked.
pub fn build_index(marks: &[IndexMark], options: IndexOptions) -> Vec<IndexLine> {
    let mut primaries: Vec<Primary> = Vec::new();
    for mark in marks {
        let entry = mark.entry.trimmed();
        if entry.primary.is_empty() {
            continue;
        }
        let page = mark.page.as_ref().map(|(_, text)| text.as_str());
        let at = match primaries
            .iter()
            .position(|(name, ..)| *name == entry.primary)
        {
            Some(at) => at,
            None => {
                primaries.push((entry.primary.clone(), Heading::default(), Vec::new()));
                primaries.len() - 1
            }
        };
        let (_, heading, secondaries) = &mut primaries[at];
        if entry.secondary.is_empty() {
            heading.add(&entry, page);
        } else {
            let at = match secondaries
                .iter()
                .position(|(name, _)| *name == entry.secondary)
            {
                Some(at) => at,
                None => {
                    secondaries.push((entry.secondary.clone(), Heading::default()));
                    secondaries.len() - 1
                }
            };
            secondaries[at].1.add(&entry, page);
        }
    }
    primaries.sort_by(|left, right| index_order(&left.0, &right.0));
    let mut lines = Vec::new();
    let mut group: Option<u8> = None;
    for (name, heading, mut secondaries) in primaries {
        if options.separators {
            let first = name
                .chars()
                .next()
                .map(amipro_document::windows_1252)
                .unwrap_or(0);
            let weight = sort_weight(first);
            if group != Some(weight) {
                group = Some(weight);
                lines.push(IndexLine::Separator(separator_text(
                    name.chars().next().unwrap_or(' '),
                )));
            }
        }
        lines.push(IndexLine::Primary(heading.text(&name, options.ranges)));
        secondaries.sort_by(|left, right| index_order(&left.0, &right.0));
        for (secondary, heading) in secondaries {
            lines.push(IndexLine::Secondary(
                heading.text(&secondary, options.ranges),
            ));
        }
    }
    lines
}

/// A separator's letter: upper case, an accented letter its base letter;
/// Æ, Ø, Å as they are (`1378:0be9`).
fn separator_text(first: char) -> String {
    let byte = amipro_document::windows_1252(first);
    let weight = sort_weight(byte);
    if byte >= 0x80 && (13..=38).contains(&weight) {
        return char::from(b'A' + (weight - 13)).to_string();
    }
    first.to_uppercase().collect()
}

/// Where Generate puts an index (TX3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexPlace {
    /// A new document's whole text (Ami Pro's output file on `_INDEX.STY`).
    Output,
    /// After a page break at the end of the document (TX3's option).
    End,
}

/// Generate Index: the paragraphs and the styles they need.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexRequest {
    pub place: IndexPlace,
    pub lines: Vec<IndexLine>,
    /// Primary, Secondary, and Separator as `_INDEX.STY` defines them,
    /// added when the document lacks them (else made from Body Text).
    pub styles: Vec<ParagraphStyle>,
}

/// Puts an index's paragraphs where the request says (one undo step).
pub(crate) fn put_index(document: &Document, request: &IndexRequest) -> Result<Applied, EditError> {
    if request.lines.is_empty() {
        return Err("the document has no index entries");
    }
    let mut next = document.clone();
    let mut report = crate::ChangeReport {
        styles: true,
        ..Default::default()
    };
    let main = next.main_flow_id();
    report.flows.insert(main);
    let mut style_ids: BTreeMap<&'static str, StyleId> = BTreeMap::new();
    for name in ["Primary", "Secondary", "Separator"] {
        let found = next
            .styles()
            .values()
            .find(|style| style.name.eq_ignore_ascii_case(name))
            .map(|style| style.id);
        let id = match found {
            Some(id) => id,
            None => {
                let id = StyleId(next.styles().keys().next_back().map_or(1, |id| id.0 + 1));
                let mut style = request
                    .styles
                    .iter()
                    .find(|style| style.name.eq_ignore_ascii_case(name))
                    .cloned()
                    .unwrap_or_else(|| next.styles()[&next.body_text_style()].clone());
                style.id = id;
                style.name = name.to_string();
                style.parent = None;
                style.next = None;
                style.origin = StyleOrigin::Document;
                next.styles_mut().insert(id, style);
                id
            }
        };
        style_ids.insert(name, id);
    }
    let removed: Vec<amipro_document::ParagraphId> = match request.place {
        IndexPlace::Output => next.main_flow().paragraphs.iter().map(|p| p.id).collect(),
        IndexPlace::End => Vec::new(),
    };
    match request.place {
        IndexPlace::Output => next.main_flow_mut().paragraphs.clear(),
        IndexPlace::End => {
            let last = next.main_flow().paragraphs.len() - 1;
            let paragraph = next
                .main_flow_mut()
                .paragraph_mut(last)
                .ok_or("the document has no text")?;
            paragraph.text.push('\u{FFFC}');
            paragraph.objects.push(InlineObject::Break {
                kind: BreakKind::Page,
                centre: false,
                layout: None,
                restart: false,
            });
            report.changed.push(paragraph.id);
        }
    }
    report.removed = removed;
    for line in &request.lines {
        let paragraph = next.push_paragraph(line.text(), style_ids[line.style()]);
        report.inserted.push(paragraph.id);
    }
    let caret = TextPosition {
        flow: main,
        paragraph: match request.place {
            IndexPlace::Output => 0,
            IndexPlace::End => next.main_flow().paragraphs.len() - request.lines.len(),
        },
        byte: 0,
    };
    Ok(Applied {
        document: next,
        caret: Some(caret),
        report,
    })
}

/// Marks to put into and take out of the text in one step: index and TOC
/// entries (Mark, Remove, Mark All, TOC Entry). A new mark goes after the
/// marks already at its place, as Ami Pro's caret stands after the last
/// one made (`1590:02a5`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkChanges {
    /// Marks to remove: a flow, a paragraph, and the mark's index in it.
    pub remove: Vec<(FlowId, usize, usize)>,
    /// Marks to add, as Ami Pro writes them (`<:X8,-32768;Index …>`).
    pub add: Vec<(TextPosition, String)>,
}

/// An index entry's mark as Mark Index Entry makes it in the main text.
pub fn index_mark(entry: &amipro_document::contents::IndexEntry) -> String {
    PowerMark {
        end: false,
        kind: power_kind::INDEX,
        value: amipro_document::power_flag::MAIN_TEXT,
        code: entry.code(),
    }
    .raw()
}

/// A TOC entry's mark as TOC Entry makes it.
pub fn toc_mark(mark: &TocMark) -> String {
    PowerMark {
        end: false,
        kind: power_kind::TOC,
        value: amipro_document::power_flag::MAIN_TEXT,
        code: mark.code(),
    }
    .raw()
}

pub(crate) fn put_marks(document: &Document, changes: &MarkChanges) -> Result<Applied, EditError> {
    if changes.remove.is_empty() && changes.add.is_empty() {
        return Err("no marks to change");
    }
    let mut next = document.clone();
    let mut report = crate::ChangeReport::default();
    let mut removals = changes.remove.clone();
    // Later marks first, so earlier indexes stay valid.
    removals.sort_by(|a, b| b.cmp(a));
    removals.dedup();
    for (flow, paragraph, mark) in removals {
        let paragraph = next
            .flow_mut(flow)
            .and_then(|flow| flow.paragraph_mut(paragraph))
            .ok_or("paragraph out of bounds")?;
        if mark >= paragraph.marks.len() {
            return Err("mark out of bounds");
        }
        paragraph.marks.remove(mark);
        report.flows.insert(flow);
        report.changed.push(paragraph.id);
    }
    for (at, raw) in &changes.add {
        let paragraph = next
            .flow_mut(at.flow)
            .and_then(|flow| flow.paragraph_mut(at.paragraph))
            .ok_or("paragraph out of bounds")?;
        if at.byte > paragraph.text.len() || !paragraph.text.is_char_boundary(at.byte) {
            return Err("position out of bounds");
        }
        let place = paragraph
            .marks
            .iter()
            .position(|mark| mark.byte > at.byte)
            .unwrap_or(paragraph.marks.len());
        paragraph.marks.insert(
            place,
            amipro_document::LegacyMark {
                byte: at.byte,
                raw: raw.clone(),
            },
        );
        report.flows.insert(at.flow);
        report.changed.push(paragraph.id);
    }
    report.changed.sort();
    report.changed.dedup();
    let caret = changes.add.last().map(|(at, _)| *at);
    Ok(Applied {
        document: next,
        caret,
        report,
    })
}

/// TOC Options and the TOC, Index options: the document's `[toc]` and
/// `[master]` as one step.
pub(crate) fn set_contents(
    document: &Document,
    settings: &amipro_document::contents::ContentsSettings,
) -> Result<Applied, EditError> {
    if document.contents() == settings {
        return Err("the contents settings are unchanged");
    }
    let mut next = document.clone();
    next.set_contents(settings.clone());
    Ok(Applied {
        document: next,
        caret: None,
        report: crate::ChangeReport {
            info: true,
            ..Default::default()
        },
    })
}
