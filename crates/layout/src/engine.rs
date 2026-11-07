//! Pagination, frame placement, headers and footers, frame interiors, and the
//! display list (`ARCHITECTURE.md`, "Layout engine"; D5). The rules come from
//! the VM fixtures in `docs/research/SAM_FORMAT_NOTES.md`; each test names
//! the fixture it follows.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

use amipro_document::{
    Alignment, BreakKind, Breaks, CellContent, CharacterRun, Colour, ColumnLine, Displayed,
    Document, FieldContext, FlowId, Frame, FrameContent, FrameId, HeaderFooterKind, Indents,
    InlineObject, LayoutBox, LayoutId, LineHeights, LinePosition, OBJECT_REPLACEMENT_CHARACTER,
    PageGeometry, PageLayout, PageSide, PageSides, Paragraph, ParagraphId, ParagraphRules,
    ParagraphStyle, Placement, Rect, RepeatPages, RuleLength, Shadow, StyleId, TabLeader, Twips,
    Wrap, rule_strokes,
};

use crate::breaking::{next_line, visual_order};
use crate::bullets::{BulletBox, ParagraphNumber, bullet_box, paragraph_numbers_carried};
use crate::shaper::{ShapeRequest, ShapedParagraph, ShapedWord, Shaper};
use crate::tabs::{size_tabs, tab_stops};

mod footnotes;
use footnotes::Footnotes;

/// A rectangle in twips from the page's top-left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Bounds {
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    fn from_rect(rect: Rect) -> Self {
        Self {
            x: rect.left as f32,
            y: rect.top as f32,
            width: rect.width() as f32,
            height: rect.height() as f32,
        }
    }

    /// The rectangle without `margins` (top, right, bottom, left) and an
    /// equal `inset` on every side.
    fn inside(self, top: f32, right: f32, bottom: f32, left: f32, inset: f32) -> Self {
        Self {
            x: self.x + left + inset,
            y: self.y + top + inset,
            width: (self.width - left - right - 2.0 * inset).max(0.0),
            height: (self.height - top - bottom - 2.0 * inset).max(0.0),
        }
    }
}

/// Which text a line belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Container {
    Main,
    /// A frame's text, a header or footer frame included.
    Frame(FrameId),
    /// A floating header or footer's text.
    Floating(FlowId),
    /// A table cell's text.
    Cell {
        frame: FrameId,
        cell: usize,
    },
    /// A footnote's text, in a page's footnote area or after the text as
    /// an endnote.
    Footnote(FlowId),
}

/// A word placed on a line.
#[derive(Clone, Debug, PartialEq)]
pub struct WordBox {
    pub bytes: Range<usize>,
    pub x: f32,
    pub width: f32,
    pub object: Option<usize>,
    /// Text shown instead of the source text (a page-number field).
    pub text: Option<String>,
    /// The shown text is drawn red (a negative number under a Red format).
    pub red: bool,
    /// A tab, with the leader its stop draws in its gap.
    pub tab: Option<TabLeader>,
}

/// One line of text: the flow, paragraph, and byte range it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct LineBox {
    pub container: Container,
    pub flow: FlowId,
    pub paragraph: ParagraphId,
    pub paragraph_index: usize,
    pub bytes: Range<usize>,
    /// The left edge of the slot the line was set in.
    pub x: f32,
    pub top: f32,
    pub baseline: f32,
    pub height: f32,
    /// The width of the slot.
    pub slot_width: f32,
    /// The width of the content.
    pub width: f32,
    /// Words in visual order, with absolute x positions.
    pub words: Vec<WordBox>,
    pub hyphen: bool,
    /// Justified: its spaces are wider than the font's, so its words are
    /// drawn one by one at their positions.
    pub spaced: bool,
    /// The paragraph rules drawn with this line: the rule above on a
    /// paragraph's first line, the rule below on its last.
    pub rules: Vec<RuleBox>,
    /// What a paragraph's first line draws before its text (Bullets &
    /// numbers), at an absolute x.
    pub bullet: Option<BulletBox>,
}

/// A paragraph rule (Modify Style › Lines) drawn with its line: from the
/// line's top (above) or ending at its bottom (below), `x` to `x + width`,
/// in Ami Pro's line style `line` (`rule_strokes`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuleBox {
    pub below: bool,
    pub x: f32,
    pub width: f32,
    pub line: u8,
    pub colour: Colour,
}

impl RuleBox {
    /// The rule's thickness, strokes and gaps together.
    pub fn thickness(&self) -> f32 {
        rule_strokes(self.line).iter().sum::<i32>() as f32
    }

    /// The top of the rule in `line`.
    pub fn top(&self, line: &LineBox) -> f32 {
        if self.below {
            line.top + line.height - self.thickness()
        } else {
            line.top
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layer {
    /// Repeating, header, and footer frames: below every page frame (V14).
    Layout,
    /// Where placed and anchored frames: the anchored ones below the
    /// others, each kind in stack order ([`page_frame_z`]).
    Page,
}

/// Where a page frame is drawn among the page's frames: anchored frames
/// below Where placed ones, whatever their places in the stack (VM
/// fixtures FC7G, FC8), each kind back to front in stack order.
pub(crate) fn page_frame_z(document: &Document, frame: FrameId) -> usize {
    let stack = document.frame_stack();
    let index = stack.iter().position(|id| *id == frame).unwrap_or(0);
    let anchored = document
        .frame(frame)
        .is_some_and(|frame| frame.placement.is_anchored());
    if anchored { index } else { stack.len() + index }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrameBoxKind {
    Text,
    /// A table, or the part of one on this column or page: each cell's box
    /// (empty for cells of other parts), and which rows the part holds.
    Table {
        cells: Vec<Bounds>,
        part: TablePart,
    },
    Picture,
    Legacy,
}

/// The rows of a table one of its frame boxes holds. Ami Pro continues a
/// page table on the next column or page (`TABLE_PAGES_NOTES.md`): each
/// part starts with the table's top inset, a part after the first with
/// its heading rows, and with Rows span pages a row may continue from one
/// part to the next. A table that is not split has one part with all its
/// rows; an empty `rows` (a table not yet laid out) means the same.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TablePart {
    /// The table's rows in this part, in order.
    pub rows: Range<usize>,
    /// The heading rows drawn first, in a part after the first.
    pub heading: Option<Range<usize>>,
    /// Each of `rows`' height here: its text's, or for a row split by
    /// Rows span pages the height of its text in this part.
    pub heights: Vec<f32>,
    /// When the first row continues from the previous part: each column's
    /// text height shown there (Rows span pages, TP5).
    pub shown_before: Vec<f32>,
    /// The last row continues in the next part.
    pub cut: bool,
}

impl TablePart {
    /// Whether the part's first row continues from an earlier part.
    pub fn continued(&self) -> bool {
        !self.shown_before.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameBox {
    pub frame: FrameId,
    pub bounds: Bounds,
    /// Where the frame's content goes: inside its margins and corner inset.
    pub interior: Bounds,
    /// The line between a text frame's columns (Frame Properties › Line
    /// between columns), drawn with the frame (VM fixture C04).
    pub strokes: Vec<PageStroke>,
    pub layer: Layer,
    pub kind: FrameBoxKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PageBox {
    /// 1-based. Right pages are odd.
    pub number: u32,
    /// The number page-number fields count on this page: `number`, less
    /// the pages before Ami Pro's TOC break (`InlineObject::Break::restart`).
    pub shown: u32,
    pub layout: LayoutId,
    pub side: PageSide,
    pub width: f32,
    pub height: f32,
    /// The main text's columns.
    pub columns: Vec<Bounds>,
    /// Back to front: layout frames, then page frames in stack order.
    pub frames: Vec<FrameBox>,
    pub lines: Vec<LineBox>,
    /// The lines the page layout draws: around the page and between its
    /// columns (Modify Page Layout › Lines).
    pub strokes: Vec<PageStroke>,
    /// The line above the page's footnotes: from `x`, `width` long, at `y`
    /// (drawn one device pixel thick, `1178:09f8`).
    pub footnote_separator: Option<Bounds>,
}

/// A stroke of a page layout's lines, a filled rectangle in twips from the
/// page's top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageStroke {
    pub bounds: Bounds,
    pub colour: Colour,
    /// A stroke of the line between columns, which with Column balance is
    /// only as tall as the balanced columns.
    pub between_columns: bool,
}

/// The lines around a page and between its columns. Where Ami Pro draws
/// them was measured on its screen (PL.6, probes AP1–AP5 with two-inch
/// margins; `PAGE_LAYOUT_NOTES.md`, "Lines"): Outside puts the line's outer
/// edge on the page's edge, Inside its inner edge on the margin, and Close
/// to outside, Middle, and Close to inside centre it a quarter, a half, and
/// three quarters of the margin in from the edge; each side by its own
/// margin. The line between columns is centred in each gutter, the
/// columns' height. A line style's strokes run from its outer edge inward.
fn page_strokes(geometry: &PageGeometry, columns: &[Bounds]) -> Vec<PageStroke> {
    let strokes_of = line_strokes;
    let mut out = Vec::new();
    if let Some(border) = geometry.border
        && !border.sides.is_none()
    {
        let (parts, thickness) = strokes_of(border.style);
        let start = |margin: Twips| {
            let margin = margin as f32;
            match border.position {
                LinePosition::Outside => 0.0,
                LinePosition::Inside => margin - thickness,
                LinePosition::CloseToOutside => margin * 0.25 - thickness / 2.0,
                LinePosition::Middle => margin * 0.5 - thickness / 2.0,
                LinePosition::CloseToInside => margin * 0.75 - thickness / 2.0,
            }
            .max(0.0)
        };
        let margins = geometry.layout.margins;
        let (width, height) = (geometry.width as f32, geometry.height as f32);
        let left = start(margins.left);
        let top = start(margins.top);
        let right = width - start(margins.right);
        let bottom = height - start(margins.bottom);
        let sides = border.sides;
        for (offset, size) in parts {
            let mut push = |x: f32, y: f32, w: f32, h: f32| {
                out.push(PageStroke {
                    bounds: Bounds {
                        x,
                        y,
                        width: w,
                        height: h,
                    },
                    colour: border.colour,
                    between_columns: false,
                });
            };
            if sides.top {
                push(left, top + offset, right - left, size);
            }
            if sides.bottom {
                push(left, bottom - offset - size, right - left, size);
            }
            if sides.left {
                push(left + offset, top, size, bottom - top);
            }
            if sides.right {
                push(right - offset - size, top, size, bottom - top);
            }
        }
    }
    if let Some(line) = geometry.layout.line_between {
        out.extend(between_column_strokes(columns, line));
    }
    out
}

/// A line style's strokes: each one's offset from the line's outer edge and
/// its thickness, and the line's whole thickness.
fn line_strokes(style: u16) -> (Vec<(f32, f32)>, f32) {
    let mut offset = 0.0;
    let mut strokes = Vec::new();
    for (index, part) in rule_strokes(u8::try_from(style).unwrap_or(0))
        .iter()
        .enumerate()
    {
        if index % 2 == 0 {
            strokes.push((offset, *part as f32));
        }
        offset += *part as f32;
    }
    (strokes, offset)
}

/// The line between columns, centred in each gutter and as tall as the
/// columns, text or not (pages: AP1–AP5; frames: C04).
fn between_column_strokes(columns: &[Bounds], line: ColumnLine) -> Vec<PageStroke> {
    let (parts, thickness) = line_strokes(line.style);
    let mut out = Vec::new();
    for pair in columns.windows(2) {
        let centre = (pair[0].right() + pair[1].x) / 2.0;
        let top = pair[0].y.min(pair[1].y);
        let bottom = (pair[0].y + pair[0].height).max(pair[1].y + pair[1].height);
        for (offset, size) in &parts {
            out.push(PageStroke {
                bounds: Bounds {
                    x: centre - thickness / 2.0 + offset,
                    y: top,
                    width: *size,
                    height: bottom - top,
                },
                colour: line.colour,
                between_columns: true,
            });
        }
    }
    out
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentLayout {
    /// Shared, so that incremental layout reuses unchanged pages.
    pub pages: Vec<Arc<PageBox>>,
    /// Page › Line Numbering's numbers, one list per page; empty when
    /// numbering is off ([`crate::line_numbers`]).
    pub line_numbers: Vec<Vec<crate::LineNumberBox>>,
}

impl DocumentLayout {
    /// The page whose main text shows `byte` of main-flow paragraph
    /// `paragraph`, if laid out.
    pub fn page_of(&self, paragraph: usize, byte: usize) -> Option<u32> {
        self.pages
            .iter()
            .find(|page| {
                page.lines.iter().any(|line| {
                    line.container == Container::Main
                        && line.paragraph_index == paragraph
                        && line.bytes.start <= byte
                        && byte <= line.bytes.end
                })
            })
            .map(|page| page.number)
    }
}

/// The ISO 639-1 code a document is hyphenated in: its language's, English
/// for Ami Pro's 0 (none) and as before for every document Ami Pro wrote in
/// American (decision SP6).
pub fn hyphenation_language(document: &amipro_document::Document) -> String {
    document
        .language()
        .locale()
        .and_then(|locale| locale.split(['_', '-']).next())
        .filter(|code| code.len() == 2)
        .unwrap_or("en")
        .to_ascii_lowercase()
}

/// Shaped paragraphs, kept between layouts (D5). An entry is reused while the
/// paragraph is the same shared value, or has the same text, runs, and end
/// format, in the same style.
#[derive(Default)]
pub struct ShapeCache {
    entries: HashMap<ParagraphId, CacheEntry>,
    /// Numeric table cells shaped as their style shows them (TD.5).
    numbers: HashMap<ParagraphId, NumberEntry>,
    /// The line heights the entries were shaped with.
    line_heights: LineHeights,
    /// The language they were hyphenated in (`en` when empty).
    language: String,
    pub hits: usize,
    pub misses: usize,
}

struct NumberEntry {
    paragraph: Arc<Paragraph>,
    style: ParagraphStyle,
    shown: Displayed,
    shaped: Arc<ShapedParagraph>,
}

struct CacheEntry {
    paragraph: Arc<Paragraph>,
    style: ParagraphStyle,
    shaped: Arc<ShapedParagraph>,
}

impl ShapeCache {
    /// Forgets every entry when the document's line heights differ from the
    /// ones they were shaped with.
    pub fn use_line_heights(&mut self, line_heights: &LineHeights) {
        if self.line_heights != *line_heights {
            self.entries.clear();
            self.numbers.clear();
            self.line_heights = line_heights.clone();
        }
    }

    /// Forgets every entry when the document's language differs from the
    /// one they were hyphenated in.
    pub fn use_language(&mut self, language: &str) {
        if self.language != language {
            self.entries.clear();
            self.numbers.clear();
            self.language = language.to_string();
        }
    }

    pub fn shaped(
        &mut self,
        shaper: &mut dyn Shaper,
        paragraph: &Arc<Paragraph>,
        style: &ParagraphStyle,
    ) -> Arc<ShapedParagraph> {
        if let Some(entry) = self.entries.get(&paragraph.id)
            && entry.style == *style
            && (Arc::ptr_eq(&entry.paragraph, paragraph)
                || (entry.paragraph.text == paragraph.text
                    && entry.paragraph.runs == paragraph.runs
                    && entry.paragraph.end_format == paragraph.end_format))
        {
            self.hits += 1;
            return Arc::clone(&entry.shaped);
        }
        self.misses += 1;
        let shaped = Arc::new(shaper.shape(ShapeRequest {
            text: &paragraph.text,
            style,
            runs: &paragraph.runs,
            end_format: &paragraph.end_format,
            line_heights: &self.line_heights,
            language: if self.language.is_empty() {
                "en"
            } else {
                &self.language
            },
        }));
        self.entries.insert(
            paragraph.id,
            CacheEntry {
                paragraph: Arc::clone(paragraph),
                style: style.clone(),
                shaped: Arc::clone(&shaped),
            },
        );
        shaped
    }

    /// A numeric table cell's paragraph shaped as the style shows its
    /// number: one unbreakable word over all the stored bytes, as wide as
    /// the shown text (the page surface draws that text for the word).
    pub fn shaped_number(
        &mut self,
        shaper: &mut dyn Shaper,
        paragraph: &Arc<Paragraph>,
        style: &ParagraphStyle,
        shown: &Displayed,
    ) -> Arc<ShapedParagraph> {
        if let Some(entry) = self.numbers.get(&paragraph.id)
            && entry.style == *style
            && entry.shown == *shown
            && (Arc::ptr_eq(&entry.paragraph, paragraph)
                || (entry.paragraph.text == paragraph.text
                    && entry.paragraph.runs == paragraph.runs
                    && entry.paragraph.end_format == paragraph.end_format))
        {
            return Arc::clone(&entry.shaped);
        }
        // The first run's format over all the shown text, when the runs
        // are one that covers the stored text.
        let runs = match paragraph.runs.as_slice() {
            [run] if run.start == 0 && run.end == paragraph.text.len() => vec![CharacterRun {
                start: 0,
                end: shown.text.len(),
                format: run.format.clone(),
            }],
            _ => Vec::new(),
        };
        let mut shaped = shaper.shape(ShapeRequest {
            text: &shown.text,
            style,
            runs: &runs,
            end_format: &paragraph.end_format,
            line_heights: &self.line_heights,
            language: "en",
        });
        let blank = shaped.words.is_empty();
        let word = ShapedWord {
            bytes: 0..paragraph.text.len(),
            width: shaped.words.iter().map(|word| word.width).sum(),
            ascent: shaped
                .words
                .iter()
                .map(|word| word.ascent)
                .fold(0.0, f32::max),
            descent: shaped
                .words
                .iter()
                .map(|word| word.descent)
                .fold(0.0, f32::max),
            blank,
            level: 0,
            forced_break: false,
            soft_hyphen: false,
            hyphenation: false,
            object: None,
            tab: None,
        };
        shaped.words = vec![word];
        let shaped = Arc::new(shaped);
        self.numbers.insert(
            paragraph.id,
            NumberEntry {
                paragraph: Arc::clone(paragraph),
                style: style.clone(),
                shown: shown.clone(),
                shaped: Arc::clone(&shaped),
            },
        );
        shaped
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The least width indents leave for text (0.10 in), when they are wider
/// than the column (a narrow frame in a style with large indents).
const MIN_INDENTED: f32 = 144.0;

/// `column` less a paragraph's indents for its first line or the others:
/// `all` plus `first` or `rest` from the left, `right` from the right
/// (Modify Style › Alignment; VM fixtures I1–I3).
/// The one word of a line that is not blank, when it is a frame's anchor.
fn lone_frame<'a>(words: &'a [ShapedWord], objects: &[InlineObject]) -> Option<&'a ShapedWord> {
    let mut shown = words
        .iter()
        .filter(|word| !word.blank || word.object.is_some());
    let word = shown.next()?;
    let anchor = word
        .object
        .and_then(|object| objects.get(object))
        .is_some_and(|object| matches!(object, InlineObject::FrameAnchor(_)));
    (anchor && shown.next().is_none()).then_some(word)
}

fn indented(column: Bounds, indents: Indents, first: bool) -> Bounds {
    let left = (indents.left(first).max(0) as f32).min(column.width);
    let right = (indents.right.max(0) as f32).min(column.width - left);
    let width = column.width - left - right;
    let least = MIN_INDENTED.min(column.width);
    if width >= least {
        Bounds {
            x: column.x + left,
            width,
            ..column
        }
    } else {
        Bounds {
            x: (column.x + left).min(column.right() - least),
            width: least,
            ..column
        }
    }
}
/// The interior margins of a floating header or footer without a layout
/// header frame (the Header/Footer panel defaults).
const FLOATING_MARGINS: (f32, f32) = (1_440.0, 1_440.0);

/// A frame's box at `bounds`: its text area inside its margins and corner
/// inset, its kind, and the line between a text frame's columns.
fn frame_box(frame: &Frame, bounds: Bounds, layer: Layer) -> FrameBox {
    let style = &frame.style;
    // Inside the margins and, with rounded corners, their inset, as
    // Ami Pro works the text area out (`FrameStyle::text_insets`).
    let insets = style.text_insets(bounds.width.round() as i32, bounds.height.round() as i32);
    let interior = bounds.inside(
        insets.top as f32,
        insets.right as f32,
        insets.bottom as f32,
        insets.left as f32,
        0.0,
    );
    let kind = match &frame.content {
        FrameContent::Text(_) => FrameBoxKind::Text,
        FrameContent::Table(_) => FrameBoxKind::Table {
            cells: Vec::new(),
            part: TablePart::default(),
        },
        FrameContent::Picture(_) => FrameBoxKind::Picture,
        FrameContent::Legacy(_) => FrameBoxKind::Legacy,
    };
    // A text frame's line between columns (C04: the text area's full
    // height, text or not).
    let strokes = match (&kind, style.interior.line_between) {
        (FrameBoxKind::Text, Some(line)) if style.interior.columns > 1 => {
            between_column_strokes(&columns(interior, &style.interior), line)
        }
        _ => Vec::new(),
    };
    FrameBox {
        frame: frame.id,
        bounds,
        interior,
        strokes,
        layer,
        kind,
    }
}

/// Lays out the whole document, its dates and Doc Info entries shown for
/// no clock or file ([`FieldContext::default`]).
pub fn layout_document(
    document: &Document,
    shaper: &mut dyn Shaper,
    cache: &mut ShapeCache,
) -> DocumentLayout {
    layout_document_with(document, shaper, cache, &FieldContext::default())
}

/// Lays out the whole document with its dates and Doc Info entries shown
/// for `context`.
pub fn layout_document_with(
    document: &Document,
    shaper: &mut dyn Shaper,
    cache: &mut ShapeCache,
    context: &FieldContext,
) -> DocumentLayout {
    let mut engine = Engine::new(document, shaper, cache, context);
    engine.main_phase();
    let all = 0..engine.pages.len();
    engine.headers_and_footers(all.clone());
    engine.frame_interiors(all);
    let mut layout = DocumentLayout {
        pages: engine.pages,
        line_numbers: Vec::new(),
    };
    layout.line_numbers = crate::line_numbers(document, &layout, shaper);
    layout
}

/// Where the main text starts on a page: the first main line's paragraph and
/// word, and the state that decides the rest of the page. Incremental layout
/// restarts from one and stops when a page starts as it did before (D5).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PageStart {
    pub(crate) paragraph_index: usize,
    pub(crate) paragraph: ParagraphId,
    pub(crate) word: usize,
    pub(crate) byte: usize,
    pub(crate) layout: LayoutId,
    pub(crate) next_layout: Option<LayoutId>,
    pub(crate) cursor: Cursor,
    /// The page frames already on the page.
    pub(crate) frames: Vec<(usize, FrameBox)>,
    /// Whether the paragraph keeps its place even if a frame after it does
    /// not fit (V12).
    pub(crate) keeps_place: bool,
    pub(crate) floating: usize,
    pub(crate) run_starts: usize,
    pub(crate) number_restarts: usize,
    /// Nothing but Where placed frames came before the first line, so a
    /// layout can restart here on a new page.
    pub(crate) fresh: bool,
}

/// Where `main_text` begins.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MainStart {
    pub(crate) paragraph: usize,
    pub(crate) word: usize,
    pub(crate) next_layout: Option<LayoutId>,
}

/// What a resumed layout compares with to stop early.
pub(crate) struct Convergence<'p> {
    pub(crate) starts: &'p [Option<PageStart>],
    pub(crate) floating: &'p [Floating],
    pub(crate) run_starts: &'p [u32],
    pub(crate) number_restarts: &'p [u32],
    /// From this new paragraph index on, the main text is unchanged.
    pub(crate) unchanged_from: usize,
    /// New index minus old index for unchanged paragraphs.
    pub(crate) delta: isize,
    /// Tables whose cells changed: a page that starts with a part of one
    /// is not the same as before.
    pub(crate) tables: &'p [FrameId],
}

/// A floating header or footer: the page of its mark and the object.
pub(crate) type Floating = (u32, HeaderFooterKind, PageSides, FlowId);

pub(crate) struct Engine<'a> {
    document: &'a Document,
    shaper: &'a mut dyn Shaper,
    cache: &'a mut ShapeCache,
    /// What dates and Doc Info entries show.
    context: &'a FieldContext,
    pub(crate) pages: Vec<Arc<PageBox>>,
    /// The page frames on each page, with their stack position, before
    /// sorting into the page's frame list.
    pub(crate) page_layouts: Vec<Vec<(usize, FrameBox)>>,
    /// Where the main text starts on each page.
    pub(crate) starts: Vec<Option<PageStart>>,
    pub(crate) floating: Vec<Floating>,
    /// The page numbers on which a layout run begins.
    pub(crate) run_starts: Vec<u32>,
    /// The page numbers after which page numbers start again: the pages
    /// after Ami Pro's TOC break (`InlineObject::Break::restart`).
    pub(crate) number_restarts: Vec<u32>,
    /// The page index at which a resumed layout matched the previous one.
    pub(crate) converged: Option<usize>,
    /// Whether the paragraph being set keeps its place if a frame after it
    /// does not fit (V12).
    keeps_place: bool,
    /// Each table's grid, measured once per layout: placing a table frame
    /// and setting its cells both need it.
    table_grids: HashMap<FrameId, TableGrid>,
    /// The cells' flows whose formula reads something that is not a number
    /// and whose result is empty: Ami Pro draws REF there (TD.7).
    ref_flows: HashSet<FlowId>,
    /// Each flow's paragraph numbers (Bullets & numbers), counted once.
    numbers: HashMap<FlowId, Arc<Vec<ParagraphNumber>>>,
    /// A page or column break after the last paragraph, made before the
    /// next one (Modify Style › Breaks; B2, B4).
    pending_break: Option<BreakKind>,
    /// Where the paragraph being set placed its first line: page and
    /// column, for paragraphs kept with the next one.
    first_line_at: Option<(usize, usize)>,
    /// Where the last table placed its first part: page, column, and top.
    first_part_at: Option<(usize, usize, f32)>,
    /// The main text's footnotes and each page's area, when it has any.
    footnotes: Option<Footnotes>,
    /// The text each footnote mark shows, by footnote (or table footnote
    /// number) and shown number, for the word boxes.
    mark_texts: HashMap<(Option<FlowId>, u32), String>,
}

/// The state before a paragraph, to set it again elsewhere (V12, break
/// rules).
#[derive(Clone)]
struct Snapshot {
    cursor: Cursor,
    layout: LayoutId,
    next_layout: Option<LayoutId>,
    pages: usize,
    /// Lines and page frames on each page from the cursor's page on.
    counts: Vec<(usize, usize)>,
    floating: usize,
    /// The cursor's page had no main text yet.
    start_open: bool,
    pending_break: Option<BreakKind>,
    footnotes: Option<Footnotes>,
}

/// Where `Engine::strip_slot` puts the next line.
enum StripSlot {
    /// In this part of the column: its left and width.
    Part(f32, f32),
    /// The cursor moved to the next part or below the band: place again.
    Moved,
    /// No frames split the column here.
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    Fits,
    /// A With para above frame did not fit below the paragraph (V12).
    DoesNotFit,
    /// A resumed layout reached a page that starts as before.
    Converged,
}

/// Where the main text is being set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cursor {
    page: usize,
    column: usize,
    y: f32,
    has_content: bool,
    /// The parts of the column beside Wrap around frames being filled.
    strips: Option<Strips>,
}

/// The parts of a column that Wrap around page frames leave beside them
/// (VM WB1–WB3): the text fills the first part down the frames' band, then
/// the next from the band's top, and goes on across the column below the
/// band, after the lowest line the parts hold. Even a narrow part takes
/// text.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Strips {
    parts: [(f32, f32); 4],
    count: u8,
    index: u8,
    /// Where the band starts: the top of its first line.
    top: f32,
    /// Where it ends: the frames' bottoms with their shadows.
    bottom: f32,
    /// The lowest line bottom the parts hold so far.
    reached: f32,
}

fn layout_of(document: &Document, id: LayoutId) -> &PageLayout {
    document
        .layout(id)
        .unwrap_or_else(|| document.document_layout())
}

fn geometry(layout: &PageLayout, number: u32) -> &PageGeometry {
    match (&layout.left, number % 2 == 0) {
        (Some(left), true) => left,
        _ => &layout.right,
    }
}

/// The columns inside `area`: the box's uneven ones from its left edge
/// (VM fixtures RU2, RU9), else equal ones with the gutter between them.
fn columns(area: Bounds, layout: &LayoutBox) -> Vec<Bounds> {
    if let Some(edges) = &layout.column_edges {
        return edges
            .iter()
            .map(|edges| Bounds {
                x: area.x + edges.left as f32,
                y: area.y,
                width: edges.width().max(1) as f32,
                height: area.height,
            })
            .collect();
    }
    let count = f32::from(layout.columns.max(1));
    let gutter = layout.gutter as f32;
    let width = ((area.width - gutter * (count - 1.0)) / count).max(0.0);
    (0..layout.columns.max(1))
        .map(|index| Bounds {
            x: area.x + f32::from(index) * (width + gutter),
            y: area.y,
            width,
            height: area.height,
        })
        .collect()
}

/// Whether a With para above frame takes its height in the text: text runs
/// through a No wrap around frame, but a table always pushes the text after
/// it down (MERCURY.SAM).
fn reserves_height(frame: &Frame) -> bool {
    frame.style.wrap != Wrap::NoWrapAround || matches!(frame.content, FrameContent::Table(_))
}

/// Each column's and row's cell size, cell margins included.
#[derive(Clone)]
struct TableGrid {
    /// Each column's text width and the gutter after it.
    columns: Vec<(f32, f32)>,
    /// Each row's text height, grown to its cells' text, and the gutter
    /// after it.
    rows: Vec<(f32, f32)>,
    /// The insets above the first row and left of the first column.
    top: f32,
    left: f32,
    /// How far each row's text may run below its text area: a row that
    /// keeps Ami Pro's stored height while as read (the row cache), whose
    /// text measures taller here than in Ami Pro by less than half its
    /// gutter, lets its text run into the gutter above the line under it.
    overflow: Vec<f32>,
}

/// Where tracks `range` lie along one side of a table grid: the text's
/// start from the table's edge (the inset plus each earlier track and its
/// gutter, `CELL_FRAME_NOTES.md`), the text's length (the tracks and the
/// gutters between them), and the cell's box, from the middle of the gutter
/// before it (the table's edge for the first track) to the middle of the
/// gutter after it.
fn track_span(tracks: &[(f32, f32)], inset: f32, range: Range<usize>) -> (f32, f32, f32, f32) {
    let start: f32 = inset
        + tracks[..range.start]
            .iter()
            .map(|(size, gutter)| size + gutter)
            .sum::<f32>();
    let inner = &tracks[range.clone()];
    let length: f32 = inner.iter().map(|(size, _)| size).sum::<f32>()
        + inner[..inner.len().saturating_sub(1)]
            .iter()
            .map(|(_, gutter)| gutter)
            .sum::<f32>();
    let before = if range.start == 0 {
        start
    } else {
        tracks[range.start - 1].1 / 2.0
    };
    let after = inner.last().map_or(0.0, |(_, gutter)| gutter / 2.0);
    (start, length, start - before, before + length + after)
}

impl TableGrid {
    /// The text area and the box of a cell over `rows` and `columns`, from
    /// the grid's top-left.
    fn cell(&self, rows: Range<usize>, columns: Range<usize>) -> (Bounds, Bounds) {
        let (x, width, box_x, box_width) = track_span(&self.columns, self.left, columns);
        let (y, height, box_y, box_height) = track_span(&self.rows, self.top, rows);
        (
            Bounds {
                x,
                y,
                width,
                height,
            },
            Bounds {
                x: box_x,
                y: box_y,
                width: box_width,
                height: box_height,
            },
        )
    }

    /// The height of the rows: the top inset and every row and its gutter.
    fn height(&self) -> f32 {
        self.top
            + self
                .rows
                .iter()
                .map(|(size, gutter)| size + gutter)
                .sum::<f32>()
    }
}

/// A line's ascent and descent: the largest of its words', or the
/// paragraph's own for a line with nothing measurable.
fn line_metrics(words: &[ShapedWord], shaped: &ShapedParagraph) -> (f32, f32) {
    let measured = words
        .iter()
        .filter(|word| word.ascent > 0.0 || word.descent > 0.0);
    let ascent = measured.clone().map(|word| word.ascent).fold(0.0, f32::max);
    let descent = measured.map(|word| word.descent).fold(0.0, f32::max);
    if ascent == 0.0 && descent == 0.0 {
        (shaped.ascent, shaped.descent)
    } else {
        (ascent, descent)
    }
}

/// A line's ascent and descent with its style's line spacing: single lines
/// as their fonts give them; others the height Ami Pro stored (or 1.5 or 2
/// times the single line when none was stored, LS1), the extra split equally
/// above and below the text (VM fixtures S1–S3). A line that holds a Flow
/// with text frame keeps at least its own height.
pub(crate) fn spaced_line(
    style: &ParagraphStyle,
    words: &[ShapedWord],
    ascent: f32,
    descent: f32,
) -> (f32, f32) {
    let single = ascent + descent;
    let Some(height) = style.spacing.line.height_for(single) else {
        return (ascent, descent);
    };
    if words
        .iter()
        .any(|word| word.object.is_some() && word.width > 0.0)
        && height < single
    {
        return (ascent, descent);
    }
    // The baseline moves by half the difference, rounded down, and never
    // above the top of the line (decompile).
    let shift = if height >= single {
        ((height - single) / 2.0).floor()
    } else {
        -((single - height) / 2.0).floor()
    };
    let ascent = (ascent + shift).max(0.0);
    (ascent, height - ascent)
}

/// The bullet box of a paragraph's first line, whose slot starts at `x`
/// (the first-line indent), and how far the line's text moves right for
/// it: the box stands at the left indent, `first` before `x`, or with Right
/// align ends a space before `x` when it is narrower than that
/// (`1108:4107`, with the paragraph's own indents when it has them), and
/// the text follows it unless the first-line indent leaves more room.
fn first_line_bullet(
    bullet: Option<&BulletBox>,
    start: usize,
    x: f32,
    indents: Indents,
) -> (Option<BulletBox>, f32) {
    match bullet.filter(|_| start == 0) {
        Some(bullet) => {
            let first = indents.first as f32;
            let room = first - (bullet.width + bullet.space);
            let shift = if bullet.right_align && room > 0.0 {
                room
            } else {
                0.0
            };
            (
                Some(BulletBox {
                    x: x - first + shift,
                    ..bullet.clone()
                }),
                (bullet.width - first).max(0.0),
            )
        }
        None => (None, 0.0),
    }
}

/// A line's ascent and descent with its paragraph's rules: a rule above
/// takes its space at the top of the first line and one below at the
/// bottom of the last, outside the line spacing (decompile `10d0:0d82`;
/// VM fixtures NX1, NX2).
fn ruled_line(
    rules: &ParagraphRules,
    first: bool,
    last: bool,
    (ascent, descent): (f32, f32),
) -> (f32, f32) {
    let (above, below) = rules.space();
    (
        ascent + if first { above as f32 } else { 0.0 },
        descent + if last { below as f32 } else { 0.0 },
    )
}

impl<'a> Engine<'a> {
    pub(crate) fn new(
        document: &'a Document,
        shaper: &'a mut dyn Shaper,
        cache: &'a mut ShapeCache,
        context: &'a FieldContext,
    ) -> Self {
        cache.use_line_heights(document.line_heights());
        cache.use_language(&hyphenation_language(document));
        Self {
            document,
            shaper,
            cache,
            context,
            pages: Vec::new(),
            page_layouts: Vec::new(),
            starts: Vec::new(),
            floating: Vec::new(),
            run_starts: Vec::new(),
            number_restarts: Vec::new(),
            converged: None,
            keeps_place: false,
            pending_break: None,
            first_line_at: None,
            first_part_at: None,
            table_grids: HashMap::new(),
            ref_flows: HashSet::new(),
            numbers: HashMap::new(),
            footnotes: Footnotes::of(document),
            mark_texts: HashMap::new(),
        }
    }

    /// The main text from the start, the page-count rule, and page frames:
    /// everything before headers, footers, and frame interiors.
    pub(crate) fn main_phase(&mut self) {
        let first = self.new_page(self.document.start_layout().id);
        self.main_text(first, MainStart::default(), None);
        self.footnotes_after_text();
        self.footnote_areas();
        self.page_count_rule();
    }

    fn page_mut(&mut self, page: usize) -> &mut PageBox {
        Arc::make_mut(&mut self.pages[page])
    }

    fn style(&self, id: StyleId) -> ParagraphStyle {
        self.document.styles().get(&id).cloned().unwrap_or_else(|| {
            self.document
                .styles()
                .values()
                .next()
                .cloned()
                .expect("a style")
        })
    }

    /// The room a line needs left before a word for it to be hyphenated:
    /// the document's zone in the paragraph font's average characters.
    fn hyphenation_zone(&self, shaped: &ShapedParagraph) -> f32 {
        f32::from(self.document.hyphenation_zone()) * shaped.average_width
    }

    /// What paragraph `index` of `flow` draws before its first line, if its
    /// style has bullets or numbers.
    fn bullet(&mut self, flow: FlowId, index: usize, style: &ParagraphStyle) -> Option<BulletBox> {
        if style.bullets.is_empty() {
            return None;
        }
        let number = if style.bullets.numbered() {
            let document = self.document;
            let flow_id = flow;
            let numbers = self.numbers.entry(flow).or_insert_with(|| {
                Arc::new(document.flow(flow).map_or_else(Vec::new, |flow| {
                    let carried = (flow_id == document.main_flow_id())
                        .then(|| document.contents().sequence.as_ref())
                        .flatten();
                    paragraph_numbers_carried(&flow.paragraphs, document.styles(), carried)
                }))
            });
            numbers
                .get(index)
                .copied()
                .unwrap_or(ParagraphNumber::own(1))
        } else {
            ParagraphNumber::own(0)
        };
        bullet_box(&style.bullets, number, style, self.shaper)
    }

    /// A paragraph's shaped words and its style with the paragraph's own
    /// formatting over it.
    fn shaped(&mut self, paragraph: &Arc<Paragraph>) -> (Arc<ShapedParagraph>, ParagraphStyle) {
        let mut style = self.style(paragraph.style);
        paragraph.format.apply(&mut style);
        (self.cache.shaped(self.shaper, paragraph, &style), style)
    }

    /// [`Engine::shaped`] for a paragraph of a table cell: a cell holding
    /// one number is shaped as its style shows it (TD.5), and the text it
    /// shows comes with it.
    fn shaped_in_cell(
        &mut self,
        flow: FlowId,
        paragraph: &Arc<Paragraph>,
        alone: bool,
    ) -> (Arc<ShapedParagraph>, ParagraphStyle, Option<Displayed>) {
        let mut style = self.style(paragraph.style);
        paragraph.format.apply(&mut style);
        let shown = (alone
            && paragraph.objects.is_empty()
            && paragraph.marks.is_empty()
            && !paragraph.text.is_empty())
        .then(|| style.kept.number_format.display(&paragraph.text))
        .flatten()
        .or_else(|| {
            (alone && paragraph.text.is_empty() && self.ref_flows.contains(&flow)).then(|| {
                Displayed {
                    text: "REF".into(),
                    red: false,
                }
            })
        });
        let shaped = match &shown {
            Some(shown) => self
                .cache
                .shaped_number(self.shaper, paragraph, &style, shown),
            None => self.cache.shaped(self.shaper, paragraph, &style),
        };
        (shaped, style, shown)
    }

    /// Adds page `number` with `layout`, and the Where placed frames that
    /// belong on it.
    pub(crate) fn new_page(&mut self, layout: LayoutId) -> usize {
        let number = self.pages.len() as u32 + 1;
        let page_layout = layout_of(self.document, layout);
        let geometry = geometry(page_layout, number);
        let margins = geometry.layout.margins;
        let area = Bounds {
            x: margins.left as f32,
            y: margins.top as f32,
            width: (geometry.width - margins.left - margins.right) as f32,
            height: (geometry.height - margins.top - margins.bottom) as f32,
        };
        if self
            .pages
            .last()
            .is_none_or(|previous| previous.layout != layout)
        {
            self.run_starts.push(number);
        }
        let page_columns = columns(area, &geometry.layout);
        let shown = self.shown_number(number);
        self.pages.push(Arc::new(PageBox {
            number,
            shown,
            layout: page_layout.id,
            side: if number % 2 == 1 {
                PageSide::Right
            } else {
                PageSide::Left
            },
            width: geometry.width as f32,
            height: geometry.height as f32,
            strokes: page_strokes(geometry, &page_columns),
            columns: page_columns,
            frames: Vec::new(),
            lines: Vec::new(),
            footnote_separator: None,
        }));
        self.page_layouts.push(Vec::new());
        self.starts.push(None);
        let index = self.pages.len() - 1;
        // F08, F09: repeating frames on every page of their layout (Repeat
        // right/left mirrors the frame on left pages; inferred). They go below
        // every page frame (V14) and text wraps around them.
        let width = geometry.width as f32;
        let side = self.pages[index].side;
        for frame_id in &page_layout.repeating {
            let Some(frame) = self.document.frame(*frame_id) else {
                continue;
            };
            let Placement::Repeating { pages, rect } = frame.placement else {
                continue;
            };
            let mut bounds = Bounds::from_rect(rect);
            if pages == RepeatPages::RightLeft && side == PageSide::Left {
                bounds.x = width - bounds.right();
            }
            self.place_frame(index, 0, frame, bounds, Layer::Layout);
        }
        for frame_id in self.document.frame_stack() {
            if let Some(frame) = self.document.frame(*frame_id)
                && let Placement::Fixed { page, rect } = frame.placement
                && page == number
            {
                let bounds = Bounds::from_rect(rect);
                let z = page_frame_z(self.document, *frame_id);
                self.place_frame(index, z, frame, bounds, Layer::Page);
            }
        }
        self.footnote_page(index);
        index
    }

    fn place_frame(&mut self, page: usize, z: usize, frame: &Frame, bounds: Bounds, layer: Layer) {
        let frame_box = frame_box(frame, bounds, layer);
        match layer {
            Layer::Page => self.page_layouts[page].push((z, frame_box)),
            Layer::Layout => self.page_mut(page).frames.push(frame_box),
        }
    }

    /// [`Engine::exclusions`] with each frame's shadow.
    fn exclusions_with_shadows(&self, page: usize) -> Vec<(Bounds, Wrap, Shadow)> {
        let mut found = Vec::new();
        for frame_box in self.pages[page].frames.iter().chain(
            self.page_layouts[page]
                .iter()
                .map(|(_, frame_box)| frame_box),
        ) {
            if let Some(frame) = self.document.frame(frame_box.frame)
                && !matches!(
                    frame.placement,
                    Placement::Header { .. } | Placement::Footer { .. }
                )
                && frame.style.wrap != Wrap::NoWrapAround
            {
                found.push((frame_box.bounds, frame.style.wrap, frame.style.shadow));
            }
        }
        found
    }

    /// Frames on `page` that text wraps around, each with its shadow, and
    /// their wrap mode.
    fn wrapped_with_shadows(&self, page: usize) -> Vec<(Bounds, Wrap)> {
        self.exclusions_with_shadows(page)
            .into_iter()
            .map(|(bounds, wrap, shadow)| {
                let (left, top) = (shadow.left.max(0) as f32, shadow.top.max(0) as f32);
                let grown = Bounds {
                    x: bounds.x - left,
                    y: bounds.y - top,
                    width: bounds.width + left + shadow.right.max(0) as f32,
                    height: bounds.height + top + shadow.bottom.max(0) as f32,
                };
                (grown, wrap)
            })
            .collect()
    }

    /// Frames on `page` that text wraps around, with their wrap mode.
    fn exclusions(&self, page: usize) -> Vec<(Bounds, Wrap)> {
        let mut found = Vec::new();
        for frame_box in self.pages[page].frames.iter().chain(
            self.page_layouts[page]
                .iter()
                .map(|(_, frame_box)| frame_box),
        ) {
            if let Some(frame) = self.document.frame(frame_box.frame)
                && !matches!(
                    frame.placement,
                    Placement::Header { .. } | Placement::Footer { .. }
                )
                && frame.style.wrap != Wrap::NoWrapAround
            {
                found.push((frame_box.bounds, frame.style.wrap));
            }
        }
        found
    }

    /// The slot for a line in `column` over the band `top..bottom`: the widest
    /// part not covered by a wrapping frame, or, when a frame blocks the band,
    /// the y to retry from.
    /// The part of the column beside Wrap around frames where the next
    /// line goes (`Strips`): within the parts being filled, the next part
    /// from the band's top when this one is full, below the band after the
    /// last; parts started when frames split the column at the line's band.
    fn strip_slot(
        &self,
        cursor: &mut Cursor,
        bounds: Bounds,
        top: f32,
        estimate: f32,
    ) -> StripSlot {
        if let Some(strips) = cursor.strips.as_mut() {
            if top < strips.bottom {
                let (left, right) = strips.parts[usize::from(strips.index)];
                return StripSlot::Part(left, (right - left).max(1.0));
            }
            if strips.index + 1 < strips.count {
                strips.index += 1;
                cursor.y = strips.top;
            } else {
                cursor.y = strips.reached.max(cursor.y);
                cursor.strips = None;
            }
            return StripSlot::Moved;
        }
        let Some((parts, bottom)) = self.splitting_frames(cursor.page, bounds, top, top + estimate)
        else {
            return StripSlot::None;
        };
        let mut strips = Strips {
            parts: [(0.0, 0.0); 4],
            count: parts.len().min(4) as u8,
            index: 0,
            top,
            bottom,
            reached: top,
        };
        for (slot, part) in strips.parts.iter_mut().zip(&parts) {
            *slot = *part;
        }
        let (left, right) = strips.parts[0];
        cursor.strips = Some(strips);
        StripSlot::Part(left, (right - left).max(1.0))
    }

    /// When Wrap around page frames split `column` over the band
    /// `top..bottom` into two or more parts: the parts, left to right, each
    /// frame's right shadow taken out, and where the frames end with their
    /// bottom shadows (VM WB1: the right part's text starts past the
    /// shadow, and a line starting above the shadow's bottom is beside the
    /// frame).
    fn splitting_frames(
        &self,
        page: usize,
        column: Bounds,
        top: f32,
        bottom: f32,
    ) -> Option<(Vec<(f32, f32)>, f32)> {
        let mut parts = vec![(column.x, column.right())];
        let mut end: f32 = top;
        for (frame_box, wrap, shadow) in self.exclusions_with_shadows(page) {
            if wrap != Wrap::Around {
                continue;
            }
            let right = frame_box.right() + shadow.right.max(0) as f32;
            let lowest = frame_box.bottom() + shadow.bottom.max(0) as f32;
            if lowest <= top
                || frame_box.y >= bottom
                || right <= column.x
                || frame_box.x >= column.right()
            {
                continue;
            }
            end = end.max(lowest);
            parts = parts
                .into_iter()
                .flat_map(|(left, part_right)| {
                    [
                        (left, part_right.min(frame_box.x)),
                        (left.max(right), part_right),
                    ]
                })
                .filter(|(left, right)| right > left)
                .collect();
        }
        (parts.len() >= 2).then_some((parts, end))
    }

    /// The parts of `column` over the band `top..bottom` that no Wrap around
    /// page frame covers, left to right; `None` when a No wrap beside frame
    /// takes the band.
    fn free_parts(
        &self,
        page: usize,
        column: Bounds,
        top: f32,
        bottom: f32,
    ) -> Option<Vec<(f32, f32)>> {
        let mut parts = vec![(column.x, column.right())];
        for (frame_box, wrap) in self.exclusions(page) {
            if frame_box.bottom() <= top
                || frame_box.y >= bottom
                || frame_box.right() <= column.x
                || frame_box.x >= column.right()
            {
                continue;
            }
            match wrap {
                Wrap::NoWrapBeside => return None,
                Wrap::Around => {
                    parts = parts
                        .into_iter()
                        .flat_map(|(left, right)| {
                            [
                                (left, right.min(frame_box.x)),
                                (left.max(frame_box.right()), right),
                            ]
                        })
                        .filter(|(left, right)| right > left)
                        .collect();
                }
                Wrap::NoWrapAround => {}
            }
        }
        (!parts.is_empty()).then_some(parts)
    }

    ///
    /// A part beside a frame takes the line when `need` (the line's first
    /// word) fits in it; an empty line fits in any part (VM FG1–FG3: gaps of
    /// 20, 200, and 400 twips hold an empty line, "a" from 200, and a
    /// 4-letter word only from 400). A frame's shadow is part of what text
    /// wraps around (the 20-twip gap beside `_REPORT2.STY`'s Deep-shadowed
    /// frame).
    fn slot(
        &self,
        page: usize,
        column: Bounds,
        top: f32,
        bottom: f32,
        need: f32,
    ) -> Result<(f32, f32), f32> {
        let mut intervals = vec![(column.x, column.right())];
        let mut blocked_until: Option<f32> = None;
        let wrapped = self.wrapped_with_shadows(page);
        for &(frame_box, wrap) in &wrapped {
            if frame_box.bottom() <= top
                || frame_box.y >= bottom
                || frame_box.right() <= column.x
                || frame_box.x >= column.right()
            {
                continue;
            }
            match wrap {
                Wrap::NoWrapBeside => {
                    blocked_until = Some(
                        blocked_until.map_or(frame_box.bottom(), |y| y.max(frame_box.bottom())),
                    );
                }
                Wrap::Around => {
                    intervals = intervals
                        .into_iter()
                        .flat_map(|(left, right)| {
                            [
                                (left, right.min(frame_box.x)),
                                (left.max(frame_box.right()), right),
                            ]
                        })
                        .filter(|(left, right)| right > left)
                        .collect();
                    if intervals.is_empty() {
                        blocked_until = Some(
                            blocked_until.map_or(frame_box.bottom(), |y| y.min(frame_box.bottom())),
                        );
                    }
                }
                Wrap::NoWrapAround => {}
            }
        }
        if let Some(y) = blocked_until {
            return Err(y);
        }
        let widest = intervals
            .into_iter()
            .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)));
        match widest {
            Some((left, right)) if right - left >= need.min(column.width) => {
                Ok((left, right - left))
            }
            _ => {
                // Too narrow: move below the lowest frame in the band.
                let below = wrapped
                    .iter()
                    .filter(|(frame_box, _)| frame_box.y < bottom && frame_box.bottom() > top)
                    .map(|(frame_box, _)| frame_box.bottom())
                    .fold(top + 1.0, f32::max);
                Err(below)
            }
        }
    }

    /// Moves to the next column, or to a new page.
    fn advance_column(
        &mut self,
        cursor: &mut Cursor,
        layout: &mut LayoutId,
        next_layout: &mut Option<LayoutId>,
    ) {
        if cursor.column + 1 < self.pages[cursor.page].columns.len() {
            cursor.column += 1;
            cursor.y = self.pages[cursor.page].columns[cursor.column].y;
            cursor.has_content = false;
            cursor.strips = None;
        } else {
            self.advance_page(cursor, layout, next_layout);
        }
    }

    fn advance_page(
        &mut self,
        cursor: &mut Cursor,
        layout: &mut LayoutId,
        next_layout: &mut Option<LayoutId>,
    ) {
        // The text before a layout switch ends its layout's run.
        if next_layout.is_some_and(|next| next != *layout) {
            self.balance(cursor.page);
        }
        if let Some(next) = next_layout.take() {
            *layout = next;
        }
        cursor.page = if cursor.page + 1 < self.pages.len() {
            cursor.page + 1
        } else {
            self.new_page(*layout)
        };
        cursor.column = 0;
        cursor.y = self.pages[cursor.page].columns[0].y;
        cursor.has_content = false;
        cursor.strips = None;
    }

    /// The number page-number fields count on page `number`: the pages
    /// after the last TOC break before it are counted from 1 (TI3T).
    ///
    /// A member of a master document adds the pages of the members before
    /// it, its `[sequence]`'s page base, to every number it shows
    /// (`1308:0026`; VM MD2, MD3).
    pub(crate) fn shown_number(&self, number: u32) -> u32 {
        let base = self
            .number_restarts
            .iter()
            .copied()
            .filter(|start| *start <= number)
            .max()
            .map_or(0, |start| start - 1);
        let carried = self
            .document
            .contents()
            .sequence
            .as_ref()
            .map_or(0, |sequence| sequence.page_base);
        number - base + carried
    }

    /// The text a page-number field shows on `page`: nothing before its
    /// start page, else its leading text and the number in its style
    /// (`1308:0026`, `1308:0249`; the bullets' roman numerals and letters).
    fn field_text(&self, field: &amipro_document::PageNumberField, page: u32) -> String {
        let Some(number) = field.number_on(self.shown_number(page)) else {
            return String::new();
        };
        let code = match field.style {
            'I' => '\u{1B}',
            'i' => '\u{1C}',
            'A' => '\u{1D}',
            'a' => '\u{1E}',
            _ => '\u{1A}',
        };
        let number = crate::bullets::number_text(code, number).unwrap_or_default();
        format!("{}{number}", field.leading_text)
    }

    /// The text an object shows on `page`: a page number's, a date's, or a
    /// Doc Info entry's (`10d0:2d0e`); `None` for objects that show none.
    fn object_text(&self, object: &InlineObject, page: u32) -> Option<String> {
        match object {
            InlineObject::PageNumber(field) => Some(self.field_text(field, page)),
            InlineObject::Date(field) => Some(self.context.date_text(field, self.document.info())),
            InlineObject::DocInfo(field) => Some(self.context.doc_info_text(field, self.document)),
            InlineObject::Seq(field) => Some(field.text()),
            InlineObject::Footnote { flow } => {
                let ordinal = *self.footnotes.as_ref()?.ordinals.get(flow)?;
                let number = self.footnote_number(ordinal, page.saturating_sub(1) as usize);
                self.mark_texts.get(&(Some(*flow), number)).cloned()
            }
            InlineObject::TableFootnote { number, .. } => {
                self.mark_texts.get(&(None, u32::from(*number))).cloned()
            }
            _ => None,
        }
    }

    /// The words of a paragraph with each object's width and height set for
    /// where it is: Flow with text frames are boxes, fields show their text,
    /// and other objects take no space.
    fn words_for(
        &mut self,
        paragraph: &Paragraph,
        shaped: &ShapedParagraph,
        style: &ParagraphStyle,
        column: Bounds,
        page: u32,
    ) -> Vec<ShapedWord> {
        let mut words = shaped.words.clone();
        for word in &mut words {
            let Some(object) = word.object.and_then(|index| paragraph.objects.get(index)) else {
                continue;
            };
            match object {
                InlineObject::FrameAnchor(frame) => {
                    if let Some((Placement::Inline { size, y_offset }, shadow)) = self
                        .document
                        .frame(*frame)
                        .map(|frame| (frame.placement, frame.style.shadow))
                    {
                        // Clamped to the column with its right shadow (V13;
                        // VM FA1, FA2: an 8296-twip frame is 3942 wide in a
                        // 4114 column with the Deep shadow, 4114 without).
                        // The right shadow takes space after it (Z05: two
                        // frames with the Normal shadow stand 100 twips
                        // apart).
                        word.width = ((size.width + shadow.right.max(0)) as f32).min(column.width);
                        let frame = self.document.frame(*frame).expect("found above");
                        // A table stands on a line of its own, however
                        // narrow (TABLE_PAGES_NOTES.md; VM TE20, ATEC): the
                        // text before it ends its line, the text after it
                        // starts the next.
                        if matches!(frame.content, FrameContent::Table(_)) {
                            word.width = column.width;
                        }
                        // A line moves to the next column whole, so a Flow
                        // with text table may take a column.
                        let height = self
                            .anchored_height(frame, size.height as f32, column.height)
                            .min(column.height);
                        // A frame dragged down hangs `y_offset` below the
                        // baseline, and the line's descent holds that part
                        // (F18–F20); dragged up, it stands above the text.
                        // The bottom shadow is in the descent too: the frame
                        // stands at its line's top and the next line starts
                        // below the shadow (VM FA1: 408 + 172; FA2: 408).
                        let below = (y_offset as f32).min(height);
                        word.ascent = (height - below).min(column.height);
                        word.descent = below.max(0.0) + shadow.bottom.max(0) as f32;
                    } else {
                        word.width = 0.0;
                    }
                }
                // A page number, a date, or a Doc Info entry takes the text
                // formatting where it stands, its line the height of its font
                // (VM fixtures PF1, FD1), as one piece (`10d0:1f9e`).
                InlineObject::PageNumber(_)
                | InlineObject::Date(_)
                | InlineObject::DocInfo(_)
                | InlineObject::Seq(_) => {
                    let text = self.object_text(object, page).unwrap_or_default();
                    let format = paragraph.format_at(word.bytes.start);
                    let extent = self.shaper.measure_text(
                        &text,
                        style,
                        format,
                        self.document.line_heights(),
                    );
                    word.width = extent.width;
                    if extent.ascent + extent.descent > 0.0 {
                        word.ascent = extent.ascent;
                        word.descent = extent.descent;
                    } else {
                        word.ascent = shaped.ascent;
                        word.descent = shaped.descent;
                    }
                }
                // An inserted bullet is as wide as the style bullet with its
                // code, in the font where it stands, and gives its line
                // that font's height (`10d0:3b66`; VM fixture BL1).
                InlineObject::Bullet(code) => {
                    let format = paragraph.format_at(word.bytes.start);
                    let size = format.font_size_points.unwrap_or(style.font_size_points);
                    word.width = crate::bullets::inline_bullet_width(*code, size);
                    let extent =
                        self.shaper
                            .measure_text("", style, format, self.document.line_heights());
                    if extent.ascent + extent.descent > 0.0 {
                        word.ascent = extent.ascent;
                        word.descent = extent.descent;
                    } else {
                        word.ascent = shaped.ascent;
                        word.descent = shaped.descent;
                    }
                }
                // A footnote's reference is its number as the Footnote
                // style's numbering shows it (superscript by default), as
                // wide as that (`1308:0ebd`; VM RNF1, RNF6).
                InlineObject::Footnote { .. } | InlineObject::TableFootnote { .. } => {
                    let (key, mark) = match object {
                        InlineObject::Footnote { flow } => {
                            let ordinal = self
                                .footnotes
                                .as_ref()
                                .and_then(|footnotes| footnotes.ordinals.get(flow))
                                .copied()
                                .unwrap_or(1);
                            let number =
                                self.footnote_number(ordinal, page.saturating_sub(1) as usize);
                            ((Some(*flow), number), self.footnote_mark(*flow, number))
                        }
                        InlineObject::TableFootnote { number, .. } => (
                            (None, u32::from(*number)),
                            self.table_footnote_mark(u32::from(*number)),
                        ),
                        _ => unreachable!("matched above"),
                    };
                    match mark {
                        Some(mark) => {
                            word.width = mark.width;
                            self.mark_texts.insert(key, Self::mark_text(&mark));
                        }
                        None => {
                            let text = key.1.to_string();
                            let format = paragraph.format_at(word.bytes.start);
                            word.width = self
                                .shaper
                                .measure_text(&text, style, format, self.document.line_heights())
                                .width;
                            self.mark_texts.insert(key, text);
                        }
                    }
                    word.ascent = shaped.ascent;
                    word.descent = shaped.descent;
                }
                // A note's mark is drawn over its neighbours and takes no
                // width (`1108:35a6`; VM RNN3).
                InlineObject::Break { .. }
                | InlineObject::FloatingHeaderFooter { .. }
                | InlineObject::Note { .. } => {
                    word.width = 0.0;
                }
            }
        }
        words
    }

    /// The main text from `start` on page `first`: pages, columns, breaks,
    /// layout switching, and anchored frames. With `convergence`, it stops
    /// at the first page that starts as it did in the previous layout.
    pub(crate) fn main_text(
        &mut self,
        first: usize,
        start: MainStart,
        convergence: Option<&Convergence<'_>>,
    ) {
        let document = self.document;
        let main = document.main_flow_id();
        let mut layout = self.pages[first].layout;
        let mut next_layout = start.next_layout;
        let mut cursor = Cursor {
            page: first,
            column: 0,
            y: self.pages[first].columns[0].y,
            has_content: false,
            strips: None,
        };
        self.pending_break = None;
        let paragraphs = document.paragraphs();
        // Paragraphs kept together (keep with next or previous, B6, B7): the
        // first of the run and the state before it, and the run already
        // moved to a later column, which is not moved again.
        let mut chain: Option<(usize, Snapshot)> = None;
        let mut moved_chain: Option<usize> = None;
        let mut index = start.paragraph;
        while index < paragraphs.len() {
            let paragraph = &paragraphs[index];
            let first_word = if index == start.paragraph {
                start.word
            } else {
                0
            };
            let snapshot = Snapshot {
                cursor,
                layout,
                next_layout,
                pages: self.pages.len(),
                counts: self.line_counts(cursor.page),
                floating: self.floating.len(),
                start_open: self.starts[cursor.page].is_none(),
                pending_break: self.pending_break,
                footnotes: self.footnotes.clone(),
            };
            let breaks = |id: StyleId| {
                document
                    .styles()
                    .get(&id)
                    .map_or_else(Breaks::default, |style| style.breaks)
            };
            let joined = index > start.paragraph
                && first_word == 0
                && (breaks(paragraphs[index - 1].style).keep_with_next()
                    || breaks(paragraph.style).keep_with_previous());
            if !joined {
                chain = Some((index, snapshot.clone()));
            }
            let mut retried = false;
            loop {
                self.keeps_place = retried || !snapshot.cursor.has_content;
                // A paragraph kept with the one before may still move that
                // one onto its page, so the layout never stops at its start.
                let outcome = self.main_paragraph(
                    index,
                    paragraph,
                    first_word,
                    main,
                    &mut cursor,
                    &mut layout,
                    &mut next_layout,
                    if joined { None } else { convergence },
                );
                match outcome {
                    Outcome::Converged => return,
                    Outcome::Fits => break,
                    Outcome::DoesNotFit if self.keeps_place => break,
                    Outcome::DoesNotFit => {}
                }
                // V12: a With para above frame that does not fit takes its
                // paragraph to the next column or page; so does a paragraph
                // that may not break within (B5).
                retried = true;
                self.rollback(&snapshot);
                cursor = snapshot.cursor;
                layout = snapshot.layout;
                next_layout = snapshot.next_layout;
                self.pending_break = snapshot.pending_break;
                self.advance_column(&mut cursor, &mut layout, &mut next_layout);
            }
            // Keep with next: a paragraph that went to a later column than
            // the one its kept predecessor ended in takes the whole run with
            // it, once, unless the run already began a column.
            if joined
                && let Some((chain_index, chain_snapshot)) = &chain
                && moved_chain != Some(*chain_index)
                && chain_snapshot.cursor.has_content
                && self
                    .first_line_at
                    .is_some_and(|at| at != (snapshot.cursor.page, snapshot.cursor.column))
            {
                let (chain_index, chain_snapshot) = chain.take().expect("checked above");
                self.rollback(&chain_snapshot);
                cursor = chain_snapshot.cursor;
                layout = chain_snapshot.layout;
                next_layout = chain_snapshot.next_layout;
                self.pending_break = chain_snapshot.pending_break;
                self.advance_column(&mut cursor, &mut layout, &mut next_layout);
                moved_chain = Some(chain_index);
                index = chain_index;
                continue;
            }
            index += 1;
        }
        // A break after the last paragraph still starts a column or page:
        // Ami Pro's page table lists the empty page (B2, B4, V22).
        if let Some(kind) = self.pending_break.take()
            && cursor.has_content
        {
            match kind {
                BreakKind::Page => self.advance_page(&mut cursor, &mut layout, &mut next_layout),
                BreakKind::Column => {
                    self.advance_column(&mut cursor, &mut layout, &mut next_layout)
                }
            }
        }
        self.balance(cursor.page);
    }

    /// Records where the main text starts on `page`, and reports whether the
    /// previous layout's page started the same way with the same text after.
    fn start_page(
        &mut self,
        page: usize,
        start: PageStart,
        convergence: Option<&Convergence<'_>>,
    ) -> bool {
        let converged = convergence.is_some_and(|convergence| {
            start.paragraph_index >= convergence.unchanged_from
                && convergence
                    .starts
                    .get(page)
                    .and_then(Option::as_ref)
                    .is_some_and(|old| {
                        old.paragraph_index as isize + convergence.delta
                            == start.paragraph_index as isize
                            && PageStart {
                                paragraph_index: start.paragraph_index,
                                ..old.clone()
                            } == start
                            && convergence.floating[..old.floating] == self.floating[..]
                            && convergence.run_starts[..old.run_starts] == self.run_starts[..]
                            && convergence.number_restarts[..old.number_restarts]
                                == self.number_restarts[..]
                    })
                && !start
                    .frames
                    .iter()
                    .any(|(_, frame_box)| convergence.tables.contains(&frame_box.frame))
        });
        self.starts[page] = Some(start);
        converged
    }

    fn line_counts(&self, from: usize) -> Vec<(usize, usize)> {
        self.pages[from..]
            .iter()
            .zip(&self.page_layouts[from..])
            .map(|(page, frames)| (page.lines.len(), frames.len()))
            .collect()
    }

    fn rollback(&mut self, snapshot: &Snapshot) {
        self.pages.truncate(snapshot.pages);
        self.page_layouts.truncate(snapshot.pages);
        self.starts.truncate(snapshot.pages);
        self.run_starts
            .retain(|start| *start as usize <= snapshot.pages);
        self.number_restarts
            .retain(|start| *start as usize <= snapshot.pages);
        let from = snapshot.cursor.page;
        for (offset, (lines, frames)) in snapshot.counts.iter().enumerate() {
            let page = from + offset;
            if self
                .pages
                .get(page)
                .is_some_and(|target| target.lines.len() > *lines)
            {
                self.page_mut(page).lines.truncate(*lines);
            }
            if let Some(target) = self.page_layouts.get_mut(page) {
                target.truncate(*frames);
            }
        }
        if snapshot.start_open {
            self.starts[from] = None;
        }
        self.floating.truncate(snapshot.floating);
        self.footnotes.clone_from(&snapshot.footnotes);
    }

    /// Sets one main-text paragraph from word `first_word`.
    #[allow(clippy::too_many_arguments)]
    fn main_paragraph(
        &mut self,
        index: usize,
        paragraph: &Arc<Paragraph>,
        first_word: usize,
        main: FlowId,
        cursor: &mut Cursor,
        layout: &mut LayoutId,
        next_layout: &mut Option<LayoutId>,
        convergence: Option<&Convergence<'_>>,
    ) -> Outcome {
        let (shaped, style) = self.shaped(paragraph);
        let bullet = self.bullet(main, index, &style);
        let anchors: Vec<(usize, FrameId)> = paragraph
            .object_offsets()
            .zip(&paragraph.objects)
            .filter_map(|(byte, object)| match object {
                InlineObject::FrameAnchor(frame)
                    if matches!(
                        self.document.frame(*frame).map(|frame| frame.placement),
                        Some(Placement::BelowParagraph { .. })
                    ) =>
                {
                    Some((byte, *frame))
                }
                _ => None,
            })
            .collect();
        // A paragraph that holds only With para above anchors takes no
        // height: its frames stand where its line would be (MERCURY.SAM).
        let anchors_only = !paragraph.text.is_empty()
            && paragraph
                .text
                .chars()
                .all(|character| character == OBJECT_REPLACEMENT_CHARACTER)
            && anchors.len() == paragraph.objects.len();
        // Breaks after the last paragraph and before this one (B1–B4); none
        // at the top of a column.
        self.first_line_at = None;
        if first_word == 0 {
            if let Some(kind) = self.pending_break.take()
                && cursor.has_content
            {
                match kind {
                    BreakKind::Page => self.advance_page(cursor, layout, next_layout),
                    BreakKind::Column => self.advance_column(cursor, layout, next_layout),
                }
            }
            if cursor.has_content && style.breaks.page_before() {
                self.advance_page(cursor, layout, next_layout);
            } else if cursor.has_content && style.breaks.column_before() {
                self.advance_column(cursor, layout, next_layout);
            }
        }
        // D7 (refined): a With para above frame whose anchor starts the
        // paragraph sits before it; one whose anchor follows text sits after.
        let mut table_start = None;
        if first_word == 0 {
            for (_, frame) in anchors.iter().filter(|(byte, _)| *byte == 0) {
                self.first_part_at = None;
                self.place_below_frame(*frame, cursor, layout, next_layout);
                table_start = table_start.or(self.first_part_at.take());
            }
        }
        // An anchors-only paragraph's empty line stays where its table began
        // when the table continues on a later column or page: Ami Pro's page
        // table ends that page on it (TP1, TP2).
        let redirect = table_start.filter(|(page, column, _)| {
            anchors_only && (*page, *column) != (cursor.page, cursor.column)
        });
        let mut start = first_word;
        let mut first_line = true;
        let mut column_break = false;
        let mut words_key = None;
        let mut words: Vec<ShapedWord> = Vec::new();
        loop {
            let column = self.column_on(cursor.page, cursor.column);
            let page_number = self.pages[cursor.page].number;
            // Objects take sizes that depend on the column and page.
            if words_key != Some((cursor.page, cursor.column)) {
                words = self.words_for(paragraph, &shaped, &style, column, page_number);
                words_key = Some((cursor.page, cursor.column));
            }
            if words.is_empty() && !first_line {
                break;
            }
            // Space above the first line: not at the top of a column or page
            // unless the style adds it always (S4, S5). It is an offset, not
            // a move of the cursor, so a page start replays it the same way.
            let above = if start == 0 && (cursor.has_content || style.spacing.add_at_break) {
                style.spacing.above as f32
            } else {
                0.0
            };
            let top = cursor.y + above;
            let estimate = shaped.ascent + shaped.descent;
            let bounds = indented(column, style.indents, start == 0);
            let slot = match self.strip_slot(cursor, bounds, top, estimate) {
                StripSlot::Part(x, width) => Ok((x, width)),
                StripSlot::Moved => continue,
                StripSlot::None => {
                    // The line's first word, with any blanks before it.
                    let need =
                        words[start..]
                            .iter()
                            .position(|word| !word.blank)
                            .map_or(0.0, |first| {
                                words[start..=start + first]
                                    .iter()
                                    .map(|word| word.width)
                                    .sum()
                            });
                    self.slot(cursor.page, bounds, top, top + estimate, need)
                }
            };
            let (x, width) = match slot {
                Ok(slot) => slot,
                Err(retry) => {
                    if retry + estimate > column.bottom() {
                        self.advance_column(cursor, layout, next_layout);
                    } else {
                        cursor.y = retry;
                    }
                    continue;
                }
            };
            // The bullet or number stands at the left indent and the first
            // line's text after it, or at the first-line indent if that is
            // further (decompile `1200:12de`, `10d0:0d82`).
            let (line_bullet, shift) = first_line_bullet(bullet.as_ref(), start, x, style.indents);
            let (x, width) = (x + shift, (width - shift).max(1.0));
            // Tabs reach this column's stops, or the style's own.
            if words[start..].iter().any(|word| word.tab.is_some()) {
                let page = &self.pages[cursor.page];
                let geometry = geometry(layout_of(self.document, page.layout), page.number);
                let column_tabs = geometry.layout.tab_stops();
                let stops = tab_stops(&style, style.indents, column_tabs, column.width);
                size_tabs(
                    &mut words,
                    start,
                    x - column.x,
                    width,
                    &stops,
                    &paragraph.text,
                );
            }
            let objects = &paragraph.objects;
            let breaks_after = |word: usize| {
                words[word]
                    .object
                    .and_then(|object| objects.get(object))
                    .is_some_and(|object| matches!(object, InlineObject::Break { .. }))
            };
            let zone = self.hyphenation_zone(&shaped);
            let line = next_line(
                &words,
                start,
                width,
                shaped.hyphen_width,
                zone,
                breaks_after,
            );
            let (ascent, descent) = line_metrics(&words[start..line.end], &shaped);
            let (ascent, descent) = ruled_line(
                &style.rules,
                start == 0,
                line.end >= words.len(),
                spaced_line(&style, &words[start..line.end], ascent, descent),
            );
            // An anchors-only line keeps its baseline for the caret.
            let height = if anchors_only { 0.0 } else { ascent + descent };
            // With Add in Always, the last line takes its paragraph's space
            // below with it: both must fit in the column (decompile).
            let below = if line.end >= words.len() && style.spacing.add_at_break {
                style.spacing.below as f32
            } else {
                0.0
            };
            // A line that holds only a table: Ami Pro's page tables are
            // Flow with text frames on a line of their own, and continue on
            // later columns and pages (`TABLE_PAGES_NOTES.md`).
            let table_line = if redirect.is_none() {
                self.lone_table(paragraph, &words[start..line.end])
            } else {
                None
            };
            if redirect.is_none()
                && table_line.is_none()
                && top + height + below > column.bottom()
                && cursor.has_content
            {
                // No break within: the paragraph goes to the next column
                // whole (B5), unless it began this one.
                if !first_line && !style.breaks.within() && !self.keeps_place {
                    return Outcome::DoesNotFit;
                }
                self.advance_column(cursor, layout, next_layout);
                continue;
            }
            // The footnotes the line refers to go into the page's area right
            // after it; when not even the separator and a first line fit
            // below it, the line goes to the next page (RNF72).
            if redirect.is_none() && table_line.is_none() {
                let flows: Vec<FlowId> = words[start..line.end]
                    .iter()
                    .filter_map(
                        |word| match word.object.and_then(|object| objects.get(object)) {
                            Some(InlineObject::Footnote { flow }) => Some(*flow),
                            _ => None,
                        },
                    )
                    .collect();
                if !flows.is_empty()
                    && !self.fit_footnotes(cursor.page, &flows, top + height, !cursor.has_content)
                {
                    self.advance_column(cursor, layout, next_layout);
                    continue;
                }
            }
            if start == 0 && first_line {
                self.first_line_at = Some(
                    redirect.map_or((cursor.page, cursor.column), |(page, column, _)| {
                        (page, column)
                    }),
                );
            }
            if redirect.is_none() && self.starts[cursor.page].is_none() {
                let frames = self.page_layouts[cursor.page].clone();
                let fresh = frames.iter().all(|(_, frame_box)| {
                    self.document
                        .frame(frame_box.frame)
                        .is_some_and(|frame| matches!(frame.placement, Placement::Fixed { .. }))
                });
                let start_record = PageStart {
                    paragraph_index: index,
                    paragraph: paragraph.id,
                    word: start,
                    byte: words.get(start).map_or(0, |word| word.bytes.start),
                    layout: *layout,
                    next_layout: *next_layout,
                    cursor: *cursor,
                    frames,
                    keeps_place: self.keeps_place,
                    floating: self.floating.len(),
                    run_starts: self.run_starts.len(),
                    number_restarts: self.number_restarts.len(),
                    fresh,
                };
                if self.start_page(cursor.page, start_record, convergence) {
                    // The rest is as before: drop this page and stop.
                    self.pages.truncate(cursor.page);
                    self.page_layouts.truncate(cursor.page);
                    self.starts.truncate(cursor.page);
                    self.run_starts
                        .retain(|number| (*number as usize) <= cursor.page);
                    self.number_restarts
                        .retain(|number| (*number as usize) <= cursor.page);
                    self.converged = Some(cursor.page);
                    return Outcome::Converged;
                }
            }
            if let Some(table_frame) = table_line {
                let frame = self
                    .document
                    .frame(table_frame)
                    .cloned()
                    .expect("lone_table found it");
                let (FrameContent::Table(table), Placement::Inline { size, .. }) =
                    (&frame.content, frame.placement)
                else {
                    unreachable!("lone_table checks both");
                };
                let table = table.clone();
                let table_width = (size.width as f32).min(column.width);
                // Center table on page puts it in the middle of its column
                // (value 7 `0x4`, `1478:128a`; VM TE13).
                let centred = table.layout.center;
                let offset = match style.alignment {
                    _ if centred => {
                        ((column.width - table_width) / 2.0).floor().max(0.0) - (x - column.x)
                    }
                    Alignment::Centre => ((width - table_width) / 2.0).max(0.0),
                    Alignment::Right => (width - table_width).max(0.0),
                    Alignment::Left | Alignment::Justify => 0.0,
                };
                let before = (cursor.page, cursor.column, top);
                cursor.y = top;
                let fits = self.place_table(
                    &frame,
                    &table,
                    (x + offset - column.x).round() as Twips,
                    table_width.round() as Twips,
                    cursor,
                    layout,
                    next_layout,
                );
                // A table whose first row does not fit takes its paragraph to
                // the next column, as a With para above frame does (TP11).
                if !fits && !self.keeps_place {
                    return Outcome::DoesNotFit;
                }
                // The table's line, empty, where its first part stands: Ami
                // Pro's page table ends that page on it (TP1).
                let (page, column_index, part_top) = self.first_part_at.unwrap_or(before);
                let part_column = self.pages[page].columns[column_index];
                self.push_line(
                    page,
                    LineSpec {
                        shown: None,
                        container: Container::Main,
                        flow: main,
                        paragraph,
                        paragraph_index: index,
                        words: &words,
                        range: start..line.end,
                        x: x - column.x + part_column.x,
                        top: part_top,
                        ascent: 0.0,
                        height: 0.0,
                        slot_width: width,
                        width: 0.0,
                        hyphen: false,
                        page_number: self.pages[page].number,
                        alignment: style.alignment,
                        first: start == 0,
                        last: line.end >= words.len(),
                        rules: &ParagraphRules::default(),
                        column: part_column,
                        bullet: None,
                    },
                );
                cursor.has_content = true;
                first_line = false;
                if line.end >= words.len() {
                    break;
                }
                start = line.end;
                continue;
            }
            // A frame too wide for its line moves left until it ends, with
            // its shadow, at the line's right edge, but not past the
            // column's left (VM FA1, FA3: 3800 twips in a 3394 line after a
            // 720 indent stands 142 twips in).
            let x = match lone_frame(&words[start..line.end], objects) {
                Some(word) if line.overflow => (x + width - word.width).max(column.x).min(x),
                _ => x,
            };
            let (line_page, line_top, line_column) = match redirect {
                Some((page, column_index, y)) => (page, y, self.pages[page].columns[column_index]),
                None => (cursor.page, top, column),
            };
            self.push_line(
                line_page,
                LineSpec {
                    shown: None,
                    container: Container::Main,
                    flow: main,
                    paragraph,
                    paragraph_index: index,
                    words: &words,
                    range: start..line.end,
                    x,
                    top: line_top,
                    ascent,
                    height,
                    slot_width: width,
                    width: line.width,
                    hyphen: line.hyphen,
                    page_number,
                    alignment: style.alignment,
                    first: start == 0,
                    last: line.end >= words.len(),
                    rules: &style.rules,
                    column: line_column,
                    bullet: line_bullet,
                },
            );
            self.inline_frames(
                line_page,
                paragraph,
                &words,
                start..line.end,
                line_top + ascent,
            );
            if redirect.is_none() {
                cursor.y = top + height;
                if let Some(strips) = cursor.strips.as_mut() {
                    strips.reached = strips.reached.max(cursor.y);
                }
            }
            cursor.has_content = true;
            first_line = false;
            // Objects ending the line: breaks act now; floating headers and
            // footers apply from the next page.
            for word in &words[start..line.end] {
                match word.object.and_then(|object| paragraph.objects.get(object)) {
                    Some(InlineObject::FloatingHeaderFooter { kind, pages, flow }) => {
                        self.floating.push((page_number, *kind, *pages, *flow));
                    }
                    Some(InlineObject::Break {
                        kind: BreakKind::Page,
                        centre,
                        layout: switch,
                        restart,
                    }) => {
                        // A page break balances the columns it ends, as
                        // the end of the text does (SETUP.SAM in Ami Pro,
                        // PL.11).
                        self.balance(cursor.page);
                        if *centre {
                            self.centre(cursor.page);
                        }
                        if let Some(switch) = switch {
                            *next_layout = Some(*switch);
                        }
                        if *restart {
                            let next = self.pages[cursor.page].number + 1;
                            if !self.number_restarts.contains(&next) {
                                self.number_restarts.push(next);
                            }
                        }
                        self.advance_page(cursor, layout, next_layout);
                        let number = self.pages[cursor.page].number;
                        let shown = self.shown_number(number);
                        if self.pages[cursor.page].shown != shown {
                            self.page_mut(cursor.page).shown = shown;
                        }
                    }
                    // A column break ends the column after its paragraph,
                    // wherever it stands in it (decompile; V22).
                    Some(InlineObject::Break {
                        kind: BreakKind::Column,
                        ..
                    }) => column_break = true,
                    _ => {}
                }
            }
            if line.end >= words.len() {
                break;
            }
            start = line.end;
        }
        // A frame anchored after text sits below the text, before the
        // paragraph's space after (MERCURY.SAM).
        let mut fits = true;
        for (_, frame) in anchors.iter().filter(|(byte, _)| *byte > 0) {
            fits &= self.place_below_frame(*frame, cursor, layout, next_layout);
        }
        // Space below a paragraph never starts a column or page.
        if cursor.has_content {
            cursor.y += style.spacing.below as f32;
        }
        if style.breaks.page_after() {
            self.pending_break = Some(BreakKind::Page);
        } else if style.breaks.column_after() || column_break {
            self.pending_break = Some(BreakKind::Column);
        }
        if fits {
            Outcome::Fits
        } else {
            Outcome::DoesNotFit
        }
    }

    /// Places a With para above frame at the cursor, clamped to the column
    /// (V12). Returns false if it does not fit in the rest of the column.
    fn place_below_frame(
        &mut self,
        frame_id: FrameId,
        cursor: &mut Cursor,
        layout: &mut LayoutId,
        next_layout: &mut Option<LayoutId>,
    ) -> bool {
        let Some(frame) = self.document.frame(frame_id) else {
            return true;
        };
        let Placement::BelowParagraph { x_offset, size } = frame.placement else {
            return true;
        };
        if let FrameContent::Table(table) = &frame.content {
            let (frame, table) = (frame.clone(), table.clone());
            let column_width = self.pages[cursor.page].columns[cursor.column].width;
            let x_offset = if table.layout.center {
                ((column_width - size.width as f32) / 2.0).max(0.0) as Twips
            } else {
                x_offset
            };
            return self.place_table(
                &frame,
                &table,
                x_offset,
                size.width,
                cursor,
                layout,
                next_layout,
            );
        }
        let mut column = self.column_on(cursor.page, cursor.column);
        let room = column.bottom() - cursor.y;
        let height = self
            .anchored_height(frame, size.height as f32, room)
            .min(column.height);
        let mut fits = true;
        if cursor.y + height > column.bottom() {
            fits = !cursor.has_content;
            if cursor.has_content {
                self.advance_column(cursor, layout, next_layout);
                column = self.column_on(cursor.page, cursor.column);
            }
        }
        // Beside a Wrap around page frame it takes the free part of the
        // column at its height, narrowed so that it and its right shadow end
        // there; a No wrap around frame does not narrow it (VM FC7, FC7E,
        // FC7G).
        let shadow = frame.style.shadow.right.max(0) as f32;
        let wanted = column.x + x_offset as f32;
        let (left, right) = self
            .free_parts(cursor.page, column, cursor.y, cursor.y + height)
            .filter(|parts| parts.as_slice() != [(column.x, column.right())])
            .and_then(|parts| {
                // The part where the frame's left edge would be, else the
                // nearest one.
                parts.into_iter().min_by(|a, b| {
                    let distance = |(left, right): (f32, f32)| {
                        if wanted < left {
                            left - wanted
                        } else if wanted >= right {
                            wanted - right
                        } else {
                            0.0
                        }
                    };
                    distance(*a).total_cmp(&distance(*b))
                })
            })
            .map_or((column.x, column.right()), |(left, right)| {
                (left, right - shadow)
            });
        let width = (size.width as f32)
            .min(column.width)
            .min((right - left).max(1.0));
        let x = (column.x + x_offset as f32).clamp(left, (right - width).max(left));
        let x = x.min((column.right() - width).max(column.x));
        let z = page_frame_z(self.document, frame_id);
        let bounds = Bounds {
            x,
            y: cursor.y,
            width,
            height,
        };
        let frame = frame.clone();
        self.place_frame(cursor.page, z, &frame, bounds, Layer::Page);
        // Text runs through a No wrap around frame, so the text after it
        // starts beside it; a table always takes its height (MERCURY.SAM).
        if reserves_height(&frame) {
            cursor.y += height;
            cursor.has_content = true;
        }
        fits
    }

    /// Places an anchored table at the cursor, continuing on later columns
    /// and pages as Ami Pro does (`TABLE_PAGES_NOTES.md`): each part from
    /// its column's top with the table's top inset (and, after the first,
    /// its heading rows and one more inset), holding the whole rows whose
    /// text and gutter fit; with Rows span pages a row that does not fit
    /// splits after its last line that does (TP5), and a row that does not
    /// fit even an empty column is cut to it (TP4); a page break on a row
    /// starts the next page with it (TP6). Returns false when not even the
    /// first row fits below text already in the column (V12, TP10, TP11).
    #[allow(clippy::too_many_arguments)]
    fn place_table(
        &mut self,
        frame: &Frame,
        table: &amipro_document::Table,
        x_offset: Twips,
        width: Twips,
        cursor: &mut Cursor,
        layout: &mut LayoutId,
        next_layout: &mut Option<LayoutId>,
    ) -> bool {
        let grid = self.table_grid(frame.id, table);
        let margins = frame.style.interior.margins;
        let framing = (margins.top + margins.bottom) as f32;
        let options = &table.row_options;
        let count = grid.rows.len();
        let z = page_frame_z(self.document, frame.id);
        let mut fits = true;
        let mut first_part = true;
        let mut first_row = 0;
        let mut shown_before: Vec<f32> = Vec::new();
        self.first_part_at = None;
        while first_row < count {
            let column = self.column_on(cursor.page, cursor.column);
            let room = column.bottom() - cursor.y;
            let heading = if first_part {
                None
            } else {
                options.heading_before(first_row)
            };
            let heading_height: f32 = heading
                .clone()
                .into_iter()
                .flatten()
                .map(|row| grid.rows[row].0 + grid.rows[row].1)
                .sum();
            // A repeated heading costs one more inset (TP2D, TP2E).
            let mut used = framing
                + grid.top
                + heading_height
                + if heading.is_some() { grid.top } else { 0.0 };
            let mut heights = Vec::new();
            let mut end = first_row;
            let mut cut = false;
            let mut next_shown = Vec::new();
            while end < count {
                if end > first_row && options.page_breaks.contains(&end) {
                    break;
                }
                let unit = self.row_unit(table, end);
                let gutter: f32 = (end..unit).map(|row| grid.rows[row].1).sum();
                let continuing = end == first_row && !shown_before.is_empty();
                let full: f32 = if continuing {
                    let lines = self.row_lines(table, &grid, end);
                    lines
                        .iter()
                        .map(|(column, _, total)| {
                            let before = shown_before.get(*column).copied().unwrap_or(0.0);
                            (total - before).max(0.0)
                        })
                        .fold(0.0, f32::max)
                } else {
                    (end..unit).map(|row| grid.rows[row].0).sum::<f32>()
                        + (end..unit.saturating_sub(1))
                            .map(|row| grid.rows[row].1)
                            .sum::<f32>()
                };
                if used + full + gutter <= room + 0.5 {
                    used += full + gutter;
                    heights.extend(
                        (end..unit).map(|row| if continuing { full } else { grid.rows[row].0 }),
                    );
                    end = unit;
                    continue;
                }
                let alone = end == first_row && !cursor.has_content;
                let avail = room - used - gutter;
                if options.span_pages && unit == end + 1 {
                    // Rows span pages: each cell shows its lines that fit.
                    let lines = self.row_lines(table, &grid, end);
                    let before = |column: usize| {
                        if continuing {
                            shown_before.get(column).copied().unwrap_or(0.0)
                        } else {
                            0.0
                        }
                    };
                    let shown: Vec<(usize, f32)> = lines
                        .iter()
                        .map(|(column, bottoms, _)| {
                            let start = before(*column);
                            let fitting = bottoms
                                .iter()
                                .filter(|bottom| **bottom > start + 0.5)
                                .take_while(|bottom| **bottom - start <= avail + 0.5)
                                .last()
                                .map_or(0.0, |bottom| bottom - start);
                            (*column, fitting)
                        })
                        .collect();
                    let tallest = shown.iter().map(|(_, height)| *height).fold(0.0, f32::max);
                    if tallest > 0.0 || alone {
                        let columns = grid.columns.len();
                        let mut after = vec![0.0; columns];
                        for (column, height) in &shown {
                            after[*column] = before(*column) + height;
                        }
                        heights.push(tallest);
                        used += tallest + gutter;
                        // A row that showed all its text here is done.
                        cut = lines
                            .iter()
                            .any(|(column, _, total)| *total > after[*column] + 0.5)
                            && tallest > 0.0;
                        next_shown = after;
                        end += 1;
                    }
                } else if alone {
                    // Taller than an empty column: cut to it, the rest of its
                    // text hidden (TP4).
                    let lines = self.row_lines(table, &grid, end);
                    let tallest = lines
                        .iter()
                        .map(|(_, bottoms, _)| {
                            bottoms
                                .iter()
                                .take_while(|bottom| **bottom <= avail + 0.5)
                                .last()
                                .copied()
                                .unwrap_or(0.0)
                        })
                        .fold(0.0, f32::max)
                        .max((table.rows[end] as f32).min(avail.max(0.0)));
                    heights.push(tallest);
                    heights.extend((end + 1..unit).map(|row| grid.rows[row].0));
                    used += tallest + gutter;
                    end = unit;
                }
                break;
            }
            if end == first_row {
                // Not even the first row fits: the next column, and, with
                // text above it, the paragraph too (V12).
                if first_part && cursor.has_content {
                    fits = false;
                }
                self.advance_column(cursor, layout, next_layout);
                continue;
            }
            let column = self.column_on(cursor.page, cursor.column);
            let width = (width as f32).min(column.width);
            let x = column.x + (x_offset as f32).clamp(0.0, (column.width - width).max(0.0));
            let bounds = Bounds {
                x,
                y: cursor.y,
                width,
                height: used,
            };
            let mut frame_box = frame_box(frame, bounds, Layer::Page);
            frame_box.kind = FrameBoxKind::Table {
                cells: Vec::new(),
                part: TablePart {
                    rows: first_row..end,
                    heading,
                    heights,
                    shown_before: std::mem::take(&mut shown_before),
                    cut,
                },
            };
            if first_part {
                self.first_part_at = Some((cursor.page, cursor.column, cursor.y));
            }
            self.page_layouts[cursor.page].push((z, frame_box));
            first_part = false;
            if end >= count && !cut {
                cursor.y += used;
                cursor.has_content = true;
                break;
            }
            // The rest on the next column or page.
            first_row = if cut { end - 1 } else { end };
            shown_before = if cut { next_shown } else { Vec::new() };
            if !cut && options.page_breaks.contains(&first_row) {
                self.advance_page(cursor, layout, next_layout);
            } else {
                self.advance_column(cursor, layout, next_layout);
            }
        }
        fits
    }

    /// The table a line holds when it holds nothing else: one Flow with
    /// text table anchor among blanks.
    fn lone_table(&self, paragraph: &Paragraph, words: &[ShapedWord]) -> Option<FrameId> {
        let mut found = None;
        for word in words {
            match word.object.and_then(|object| paragraph.objects.get(object)) {
                Some(InlineObject::FrameAnchor(frame))
                    if found.is_none()
                        && self.document.frame(*frame).is_some_and(|frame| {
                            matches!(frame.placement, Placement::Inline { .. })
                                && matches!(frame.content, FrameContent::Table(_))
                        }) =>
                {
                    found = Some(*frame);
                }
                None if word.blank => {}
                _ => return None,
            }
        }
        found
    }

    /// The last row (exclusive) of the rows that must stay together with
    /// `row`: those its cells over several rows cover.
    fn row_unit(&self, table: &amipro_document::Table, row: usize) -> usize {
        let columns = table.columns.len().max(1);
        let mut end = row + 1;
        let mut current = row;
        while current < end {
            for column in 0..columns {
                if let Some(cell) = table.cells.get(current * columns + column)
                    && !cell.span.is_covered()
                {
                    end = end.max(current + usize::from(cell.span.rows.max(1)));
                }
            }
            current += 1;
        }
        end.min(table.rows.len())
    }

    /// The text of each cell that starts in `row`: its first column, where
    /// each line ends from the text's top, and the height it needs.
    fn row_lines(
        &mut self,
        table: &amipro_document::Table,
        grid: &TableGrid,
        row: usize,
    ) -> Vec<(usize, Vec<f32>, f32)> {
        let count = table.columns.len();
        let mut found = Vec::new();
        for column in 0..count {
            let Some(cell) = table.cells.get(row * count + column) else {
                continue;
            };
            if cell.span.is_covered() {
                continue;
            }
            let last_column = (column + usize::from(cell.span.columns)).min(count);
            let (area, _) = grid.cell(row..row + 1, column..last_column);
            let CellContent::Text(flow) = cell.content;
            let empty = self.document.flow(flow).is_some_and(|text| {
                text.paragraphs.len() == 1 && text.paragraphs[0].text.is_empty()
            });
            let (bottoms, total) = if empty {
                (Vec::new(), 0.0)
            } else {
                self.flow_lines(flow, area.width.max(0.0))
            };
            found.push((column, bottoms, total));
        }
        found
    }

    /// Flow with text frames on a placed line: boxes standing on the
    /// baseline (X02, V13).
    fn inline_frames(
        &mut self,
        page: usize,
        paragraph: &Paragraph,
        words: &[ShapedWord],
        range: Range<usize>,
        baseline: f32,
    ) {
        // Where the line just pushed put each object, alignment included.
        let placed: Vec<(usize, f32)> = self.pages[page]
            .lines
            .last()
            .map(|line| {
                line.words
                    .iter()
                    .filter_map(|word| word.object.map(|object| (object, word.x)))
                    .collect()
            })
            .unwrap_or_default();
        for word in &words[range] {
            let Some(offset) = word.object.and_then(|object| {
                placed
                    .iter()
                    .find(|(placed, _)| *placed == object)
                    .map(|(_, x)| *x)
            }) else {
                continue;
            };
            if let Some(InlineObject::FrameAnchor(frame_id)) =
                word.object.and_then(|object| paragraph.objects.get(object))
                && let Some(frame) = self.document.frame(*frame_id)
                && let Placement::Inline { size, y_offset } = frame.placement
            {
                let z = page_frame_z(self.document, *frame_id);
                let shadow = frame.style.shadow;
                // Above the baseline by its ascent; its height is the ascent
                // and descent, less its bottom shadow and any gap left when
                // it was raised; its width leaves room for its right shadow.
                let bounds = Bounds {
                    x: offset,
                    y: baseline - word.ascent,
                    width: (size.width as f32)
                        .min(word.width - shadow.right.max(0) as f32)
                        .max(1.0),
                    height: word.ascent + word.descent - shadow.bottom.max(0) as f32
                        + (y_offset as f32).min(0.0),
                };
                let frame = frame.clone();
                self.place_frame(page, z, &frame, bounds, Layer::Page);
            }
        }
    }

    fn push_line(&mut self, page: usize, spec: LineSpec<'_>) {
        let order = visual_order(spec.words, spec.range.clone(), 0);
        let spread = Spread::of(&spec);
        let mut x = spec.x + spread.shift;
        let start_x = x;
        // Where the line's text stands: its words that are not blank.
        let mut extent: Option<(f32, f32)> = None;
        let mut boxes = Vec::with_capacity(order.len());
        for index in order {
            let word = &spec.words[index];
            let width = word.width + spread.extra(index, word);
            if !word.blank || word.object.is_some() {
                let (left, right) = extent.unwrap_or((x, x + width));
                extent = Some((left.min(x), right.max(x + width)));
            }
            let text = word
                .object
                .and_then(|object| spec.paragraph.objects.get(object))
                .and_then(|object| self.object_text(object, spec.page_number));
            let (text, red) = match (text, spec.shown) {
                (None, Some(shown)) if word.object.is_none() && word.tab.is_none() => {
                    (Some(shown.text.clone()), shown.red)
                }
                (text, _) => (text, false),
            };
            boxes.push(WordBox {
                bytes: word.bytes.clone(),
                x,
                width,
                object: word.object,
                text,
                red,
                tab: word.tab,
            });
            x += width;
        }
        let bytes = match (
            spec.words.get(spec.range.start),
            spec.range
                .end
                .checked_sub(1)
                .and_then(|last| spec.words.get(last)),
        ) {
            (Some(first), Some(last)) if spec.range.end > spec.range.start => {
                first.bytes.start..last.bytes.end
            }
            _ => 0..0,
        };
        let rules = line_rules(&spec, extent.unwrap_or((start_x, start_x)));
        self.page_mut(page).lines.push(LineBox {
            container: spec.container,
            flow: spec.flow,
            paragraph: spec.paragraph.id,
            paragraph_index: spec.paragraph_index,
            bytes,
            x: spec.x,
            top: spec.top,
            baseline: spec.top + spec.ascent,
            height: spec.height,
            slot_width: spec.slot_width,
            width: spec.width,
            words: boxes,
            hyphen: spec.hyphen,
            spaced: spread.per_space > 0.0,
            rules,
            bullet: spec.bullet,
        });
    }

    /// V23: a centring break centres the main text above it on its page.
    fn centre(&mut self, page: usize) {
        let column = self.pages[page].columns[0];
        let lines: Vec<usize> = (0..self.pages[page].lines.len())
            .filter(|index| self.pages[page].lines[*index].container == Container::Main)
            .collect();
        let (Some(first), Some(last)) = (lines.first(), lines.last()) else {
            return;
        };
        let used = self.pages[page].lines[*last].top + self.pages[page].lines[*last].height
            - self.pages[page].lines[*first].top;
        let shift = ((column.height - used) / 2.0).max(0.0);
        for index in lines {
            let line = &mut self.page_mut(page).lines[index];
            line.top += shift;
            line.baseline += shift;
        }
    }

    /// G06: with column balance, the last page of a layout's run (before a
    /// layout switch, or at the end of the text) spreads its main-text lines
    /// evenly over the columns (inferred: which pages balance is not yet
    /// observed).
    fn balance(&mut self, page: usize) {
        let layout = layout_of(self.document, self.pages[page].layout);
        let geometry = geometry(layout, self.pages[page].number);
        let columns = self.pages[page].columns.clone();
        if !geometry.layout.balance || columns.len() < 2 {
            return;
        }
        let mut lines: Vec<usize> = (0..self.pages[page].lines.len())
            .filter(|index| self.pages[page].lines[*index].container == Container::Main)
            .collect();
        lines.sort_by(|a, b| {
            let (a, b) = (&self.pages[page].lines[*a], &self.pages[page].lines[*b]);
            let column_of = |line: &LineBox| {
                columns
                    .iter()
                    .position(|column| line.x < column.right())
                    .unwrap_or(0)
            };
            (column_of(a), a.top)
                .partial_cmp(&(column_of(b), b.top))
                .unwrap()
        });
        let total: f32 = lines
            .iter()
            .map(|index| self.pages[page].lines[*index].height)
            .sum();
        let target = total / columns.len() as f32;
        let mut column = 0;
        let mut used = 0.0;
        let mut tallest: f32 = 0.0;
        for index in lines {
            let height = self.pages[page].lines[index].height;
            if used > 0.0 && used + height > target + 0.5 && column + 1 < columns.len() {
                column += 1;
                used = 0.0;
            }
            let line = &mut self.page_mut(page).lines[index];
            let dx = columns[column].x - line.x;
            line.x += dx;
            for word in &mut line.words {
                word.x += dx;
            }
            let top = columns[column].y + used;
            line.baseline += top - line.top;
            line.top = top;
            used += height;
            tallest = tallest.max(used);
        }
        // The line between balanced columns is only as tall as they are
        // (SETUP.SAM in Ami Pro, PL.11).
        for stroke in &mut self.page_mut(page).strokes {
            if stroke.between_columns {
                stroke.bounds.height = tallest;
            }
        }
    }

    /// V11, X06: the document has as many pages as its text needs or as the
    /// highest Where placed frame's page, whichever is more. Added pages use
    /// the layout in force at the end of the text.
    pub(crate) fn page_count_rule(&mut self) {
        let highest = self
            .document
            .frames()
            .filter_map(|frame| match frame.placement {
                Placement::Fixed { page, .. } => Some(page),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let layout = self
            .pages
            .last()
            .map_or(self.document.start_layout().id, |page| page.layout);
        while (self.pages.len() as u32) < highest {
            self.new_page(layout);
        }
        // Page frames, anchored ones first (FC8), each kind in stack order.
        for (page, frames) in self.pages.iter_mut().zip(&mut self.page_layouts) {
            if frames.is_empty() {
                continue;
            }
            frames.sort_by_key(|(z, _)| *z);
            Arc::make_mut(page)
                .frames
                .extend(frames.drain(..).map(|(_, frame_box)| frame_box));
        }
    }

    /// The header or footer for each page: a floating one from the page after
    /// its mark (G22, V17, V18), or the layout's, which may begin on the
    /// second page of the layout's run (G19, G23). Their text wraps around
    /// page frames (V14).
    pub(crate) fn headers_and_footers(&mut self, pages: Range<usize>) {
        for page in pages {
            let number = self.pages[page].number;
            let side = self.pages[page].side;
            let layout = layout_of(self.document, self.pages[page].layout).clone();
            let geometry = geometry(&layout, number).clone();
            let run_start = self
                .run_starts
                .iter()
                .copied()
                .filter(|start| *start <= number)
                .max()
                .unwrap_or(1);
            for kind in [HeaderFooterKind::Header, HeaderFooterKind::Footer] {
                let slots = match kind {
                    HeaderFooterKind::Header => layout.header,
                    HeaderFooterKind::Footer => layout.footer,
                };
                let layout_frame = match (side, slots.left) {
                    (PageSide::Left, Some(left)) if layout.left.is_some() => Some(left),
                    _ => slots.right,
                };
                let from_second = match kind {
                    HeaderFooterKind::Header => layout.header_from_second_page,
                    HeaderFooterKind::Footer => layout.footer_from_second_page,
                };
                let floating = self
                    .floating
                    .iter()
                    .rev()
                    .find(|(mark_page, floating_kind, pages, _)| {
                        *mark_page < number
                            && *floating_kind == kind
                            && match pages {
                                PageSides::All => true,
                                PageSides::Right => side == PageSide::Right,
                                PageSides::Left => side == PageSide::Left,
                            }
                    })
                    .map(|(_, _, _, flow)| *flow);
                let band = match kind {
                    HeaderFooterKind::Header => Bounds {
                        x: 0.0,
                        y: 0.0,
                        width: geometry.width as f32,
                        height: geometry.layout.margins.top as f32,
                    },
                    HeaderFooterKind::Footer => Bounds {
                        x: 0.0,
                        y: (geometry.height - geometry.layout.margins.bottom) as f32,
                        width: geometry.width as f32,
                        height: geometry.layout.margins.bottom as f32,
                    },
                };
                let frame = layout_frame.and_then(|id| self.document.frame(id)).cloned();
                if let Some(frame) = &frame {
                    self.place_frame(page, 0, frame, band, Layer::Layout);
                    // Keep layout frames below page frames.
                    let placed = self.page_mut(page).frames.pop().expect("just placed");
                    let first_page_frame = self.pages[page]
                        .frames
                        .iter()
                        .position(|frame_box| frame_box.layer == Layer::Page)
                        .unwrap_or(self.pages[page].frames.len());
                    self.page_mut(page).frames.insert(first_page_frame, placed);
                }
                let interior = match &frame {
                    Some(frame) => {
                        let margins = frame.style.interior.margins;
                        band.inside(
                            margins.top as f32,
                            margins.right as f32,
                            margins.bottom as f32,
                            margins.left as f32,
                            0.0,
                        )
                    }
                    None => band.inside(0.0, FLOATING_MARGINS.1, 0.0, FLOATING_MARGINS.0, 0.0),
                };
                let (flow, container) = match (floating, &frame) {
                    (Some(flow), _) => (flow, Container::Floating(flow)),
                    (None, Some(frame)) if !(from_second && number == run_start) => {
                        match frame.content {
                            FrameContent::Text(flow) => (flow, Container::Frame(frame.id)),
                            _ => continue,
                        }
                    }
                    _ => continue,
                };
                let layout_box = frame.as_ref().map_or_else(
                    || LayoutBox::single_column(amipro_document::Margins::uniform(0)),
                    |frame| frame.style.interior.clone(),
                );
                self.flow_in_area(page, flow, container, interior, &layout_box, true);
            }
        }
    }

    /// Text frames, tables, and pictures: their content inside their interior.
    /// Text that does not fit stays hidden (user's guide).
    pub(crate) fn frame_interiors(&mut self, pages: Range<usize>) {
        for page in pages {
            let mut index = 0;
            while index < self.pages[page].frames.len() {
                let frame_box = self.pages[page].frames[index].clone();
                index += 1;
                let Some(frame) = self.document.frame(frame_box.frame).cloned() else {
                    continue;
                };
                if matches!(
                    frame.placement,
                    Placement::Header { .. } | Placement::Footer { .. }
                ) {
                    continue;
                }
                match &frame.content {
                    // A frame in a cell holds a picture, never text (Ami Pro
                    // clears the text bit, `14d0:0000`).
                    FrameContent::Text(_) if frame.placement.is_in_cell() => {}
                    FrameContent::Text(flow) => {
                        self.flow_in_area(
                            page,
                            *flow,
                            Container::Frame(frame.id),
                            frame_box.interior,
                            &frame.style.interior,
                            false,
                        );
                    }
                    FrameContent::Table(table) => {
                        let part = match &frame_box.kind {
                            FrameBoxKind::Table { part, .. } => part.clone(),
                            _ => TablePart::default(),
                        };
                        let (cells, held) =
                            self.table(page, &frame, table, frame_box.interior, &part);
                        let frames = &mut self.page_mut(page).frames;
                        if let FrameBoxKind::Table { cells: slot, .. } = &mut frames[index - 1].kind
                        {
                            *slot = cells;
                        }
                        // The frames in its cells, right above it (§24),
                        // laid out next.
                        frames.splice(index..index, held);
                    }
                    FrameContent::Picture(_) | FrameContent::Legacy(_) => {}
                }
            }
        }
    }

    /// The height of an anchored frame. A table's frame is as tall as its
    /// rows need when they fit in `room`, so text after it never runs under
    /// it. A longer table continues on later pages in Ami Pro (its stored
    /// rectangle is one page's part: 80 of 220 corpus tables); tables are not
    /// split yet, so it keeps `stored`, as other frames do.
    fn anchored_height(&mut self, frame: &Frame, stored: f32, room: f32) -> f32 {
        let FrameContent::Table(table) = &frame.content else {
            return stored;
        };
        let margins = frame.style.interior.margins;
        let natural =
            self.table_grid(frame.id, table).height() + (margins.top + margins.bottom) as f32;
        if natural > stored && natural <= room {
            natural
        } else {
            stored
        }
    }

    /// The table's grid on Ami Pro's rules: each column's text width and
    /// gutter, each row's least text height and gutter, the insets; a row
    /// grows to fit its cells' text, and a joined cell's text takes all of
    /// its columns and rows with the gutters between them.
    /// Notes the cells of `table` that draw REF: a formula whose result is
    /// empty because it read a cell that is not a number (VM TDF1). Only
    /// tables with an empty formula result are calculated.
    fn note_refs(&mut self, table: &amipro_document::Table) {
        use amipro_document::formula;
        let document = self.document;
        let empty = |cell: &amipro_document::TableCell| {
            let CellContent::Text(flow) = cell.content;
            cell.formula.is_some()
                && document
                    .flow(flow)
                    .is_some_and(|text| text.paragraphs.iter().all(|p| p.text.is_empty()))
        };
        if !table.cells.iter().any(empty) {
            return;
        }
        let (dims, inputs) = formula::table_inputs(document, table);
        let options = formula::Options {
            extras: true,
            decimal: None,
        };
        let outcomes = formula::calculate(dims, &inputs, options, &|_, value| value);
        for (cell, outcome) in table.cells.iter().zip(&outcomes) {
            if empty(cell) && matches!(outcome, Some(formula::Outcome::Ref)) {
                let CellContent::Text(flow) = cell.content;
                self.ref_flows.insert(flow);
            }
        }
    }

    fn table_grid(&mut self, frame: FrameId, table: &amipro_document::Table) -> TableGrid {
        if let Some(grid) = self.table_grids.get(&frame) {
            return grid.clone();
        }
        self.note_refs(table);
        let columns: Vec<(f32, f32)> = table
            .columns
            .iter()
            .enumerate()
            .map(|(index, width)| (*width as f32, table.column_gutter(index) as f32))
            .collect();
        let mut grid = TableGrid {
            columns,
            rows: table
                .rows
                .iter()
                .enumerate()
                .map(|(index, height)| (*height as f32, table.row_gutter(index) as f32))
                .collect(),
            top: table.spacing.top as f32,
            left: table.spacing.left as f32,
            overflow: vec![0.0; table.rows.len()],
        };
        let count = table.columns.len();
        // Ami Pro's stored height is its own measure of a row's text (VM
        // RC1–RC3), so a row as read keeps it (user decision 2026-10-19)
        // while its text measures here at most half its gutter taller: our
        // substitute fonts' lines are a few twips taller than Ami Pro's
        // printer's (`TABLE_PAGES_PROGRESS.md`, TP.6). A taller measure means
        // a stale height, which Ami Pro would not keep either (RC1).
        let mut as_read: Vec<Option<bool>> = vec![None; table.rows.len()];
        // Without Automatic a row keeps its size and its text is cut (TP2E).
        let grows = if table.row_options.automatic {
            [false, true].as_slice()
        } else {
            [].as_slice()
        };
        // An Automatic row Ami Pro measured and the user has since changed
        // is as tall as its text, shrinking too, an empty cell taking a line
        // (VM RC2, RC3: what Ami Pro does when it measures a row). A row
        // made here grows from its one-line default and returns to it.
        let fits: Vec<bool> = (0..table.rows.len())
            .map(|row| {
                table.row_options.automatic
                    && table.read_rows.get(&row).is_some_and(|read| {
                        *read != self.document.table_row_fingerprint(table, row)
                    })
            })
            .collect();
        for (row, fit) in fits.iter().enumerate() {
            if *fit {
                grid.rows[row].0 = 0.0;
                as_read[row] = Some(false);
            }
        }
        // Cells within one row first, then cells over several rows, which
        // add what they still need to their last row.
        for &several_rows in grows {
            for (index, cell) in table.cells.iter().enumerate() {
                let span = cell.span;
                if span.is_covered() || (span.rows > 1) != several_rows {
                    continue;
                }
                let (row, column) = (index / count, index % count);
                let last_row = (row + usize::from(span.rows)).min(grid.rows.len());
                let last_column = (column + usize::from(span.columns)).min(count);
                let (area, _) = grid.cell(row..last_row, column..last_column);
                let CellContent::Text(flow) = cell.content;
                // An empty cell needs no height: Ami Pro keeps a row of
                // empty cells at its stored height, even 0 (15 such rows in
                // the user's corpus).
                let empty = self.document.flow(flow).is_some_and(|text| {
                    text.paragraphs.len() == 1 && text.paragraphs[0].text.is_empty()
                });
                let needed = if empty && !fits[last_row - 1] {
                    0.0
                } else {
                    self.flow_height(flow, area.width.max(0.0))
                };
                if needed > area.height {
                    let row = last_row - 1;
                    let over = needed - area.height;
                    let trusted = over <= grid.rows[row].1 / 2.0
                        && *as_read[row].get_or_insert_with(|| {
                            table.read_rows.get(&row).is_some_and(|read| {
                                *read == self.document.table_row_fingerprint(table, row)
                            })
                        });
                    if trusted {
                        grid.overflow[row] = grid.overflow[row].max(over);
                    } else {
                        grid.rows[row].0 += over;
                    }
                }
            }
        }
        self.table_grids.insert(frame, grid.clone());
        grid
    }

    /// Sets a table's cells in `interior`: each cell's text in its text
    /// area. Returns each cell's box (a covered cell takes the box of the
    /// cell joined over it; cells outside `part` none), and the boxes of the
    /// frames its cells hold, each over its cell's text area
    /// (`CELL_FRAME_NOTES.md`; T2: whatever size the file stored). A part
    /// lays out its heading rows, then its rows at their heights there; a
    /// row continued from an earlier part shows the text after what that
    /// part showed (TP5). A table not split shows the rows that fit its
    /// frame (TP8).
    fn table(
        &mut self,
        page: usize,
        frame: &Frame,
        table: &amipro_document::Table,
        interior: Bounds,
        part: &TablePart,
    ) -> (Vec<Bounds>, Vec<FrameBox>) {
        let grid = self.table_grid(frame.id, table);
        let count = table.columns.len();
        let whole = part.rows.is_empty();
        // The part's tracks, heading rows first, and where each table row is
        // among them.
        let mut tracks = Vec::new();
        let mut place: HashMap<usize, usize> = HashMap::new();
        let mut own_rows = 0..grid.rows.len();
        if !whole {
            for row in part.heading.clone().into_iter().flatten() {
                place.insert(row, tracks.len());
                tracks.push(grid.rows[row]);
            }
            for (offset, row) in part.rows.clone().enumerate() {
                place.insert(row, tracks.len());
                let height = part
                    .heights
                    .get(offset)
                    .copied()
                    .unwrap_or(grid.rows[row].0);
                tracks.push((height, grid.rows[row].1));
            }
            own_rows = part.rows.clone();
        } else {
            for (row, track) in grid.rows.iter().enumerate() {
                place.insert(row, tracks.len());
                tracks.push(*track);
            }
        }
        let part_grid = TableGrid {
            columns: grid.columns.clone(),
            rows: tracks,
            top: grid.top,
            left: grid.left,
            overflow: Vec::new(),
        };
        let mut cells = vec![Bounds::default(); table.cells.len()];
        let mut held = Vec::new();
        for (index, cell) in table.cells.iter().enumerate() {
            let span = cell.span;
            if span.is_covered() {
                continue;
            }
            let (row, column) = (index / count, index % count);
            let Some(&at) = place.get(&row) else {
                continue;
            };
            let last_row = (row + usize::from(span.rows))
                .min(grid.rows.len())
                .max(row + 1);
            let rows_here = (row..last_row)
                .filter(|row| place.contains_key(row))
                .count();
            let last_column = (column + usize::from(span.columns)).min(count);
            let (area, cell_box) = part_grid.cell(at..at + rows_here.max(1), column..last_column);
            let at_page = |bounds: Bounds| Bounds {
                x: interior.x + bounds.x,
                y: interior.y + bounds.y,
                ..bounds
            };
            let mut area = at_page(area);
            let mut bounds = at_page(cell_box);
            // A row keeping Ami Pro's stored height lets its text run into
            // its gutter where it measures taller here (the row cache), when
            // the part shows the row whole.
            let mut overflow = match place.get(&(last_row - 1)) {
                Some(&track) if part_grid.rows[track].0 >= grid.rows[last_row - 1].0 - 0.5 => {
                    grid.overflow[last_row - 1]
                }
                _ => 0.0,
            };
            if whole {
                // A frame table shows the rows its frame holds (TP8).
                if area.y >= interior.bottom() - 0.5 {
                    continue;
                }
                if area.bottom() > interior.bottom() {
                    overflow = 0.0;
                }
                area.height = area.height.min(interior.bottom() - area.y);
                bounds.height = bounds.height.min(interior.bottom() - bounds.y);
            }
            for covered_row in row..last_row {
                for covered_column in column..last_column {
                    cells[covered_row * count + covered_column] = bounds;
                }
            }
            let CellContent::Text(flow) = cell.content;
            let container = Container::Cell {
                frame: frame.id,
                cell: index,
            };
            // A row continued from the last part: its text from where that
            // part's ended, the lines shown there left out.
            let before = if row == part.rows.start && part.continued() {
                part.shown_before.get(column).copied().unwrap_or(0.0)
            } else {
                0.0
            };
            let lines_before = self.pages[page].lines.len();
            self.flow_in_area(
                page,
                flow,
                container,
                Bounds {
                    y: area.y - before,
                    height: area.height + before + overflow,
                    ..area
                },
                &LayoutBox::single_column(amipro_document::Margins::uniform(0)),
                false,
            );
            if before > 0.0 {
                let top = area.y - 0.5;
                let lines = &mut self.page_mut(page).lines;
                let mut kept = lines_before;
                for index in lines_before..lines.len() {
                    if lines[index].top >= top {
                        lines.swap(kept, index);
                        kept += 1;
                    }
                }
                lines.truncate(kept);
            }
            if own_rows.contains(&row)
                && let Some(inner) = cell.frame.and_then(|id| self.document.frame(id))
            {
                held.push(frame_box(inner, area, Layer::Page));
            }
        }
        (cells, held)
    }

    /// The height a flow's text needs at `width`.
    fn flow_height(&mut self, flow: FlowId, width: f32) -> f32 {
        self.flow_lines(flow, width).1
    }

    /// Where each line of a flow's text ends at `width`, from its top, and
    /// the height the text needs.
    fn flow_lines(&mut self, flow: FlowId, width: f32) -> (Vec<f32>, f32) {
        let Some(paragraphs) = self.document.flow(flow).map(|flow| flow.paragraphs.clone()) else {
            return (Vec::new(), 0.0);
        };
        let mut bottoms = Vec::new();
        let mut height = 0.0;
        let alone = paragraphs.len() == 1;
        for (index, paragraph) in paragraphs.iter().enumerate() {
            let (shaped, style, _) = self.shaped_in_cell(flow, paragraph, alone);
            let mut words = shaped.words.clone();
            let stops = tab_stops(&style, style.indents, &[], width);
            let bullet = self.bullet(flow, index, &style);
            let mut start = 0;
            loop {
                let area = Bounds {
                    x: 0.0,
                    y: 0.0,
                    width,
                    height: 0.0,
                };
                let slot = indented(area, style.indents, start == 0);
                let (_, shift) = first_line_bullet(bullet.as_ref(), start, slot.x, style.indents);
                let line_width = (slot.width - shift).max(1.0);
                let slot = Bounds {
                    x: slot.x + shift,
                    ..slot
                };
                size_tabs(
                    &mut words,
                    start,
                    slot.x,
                    line_width,
                    &stops,
                    &paragraph.text,
                );
                let zone = self.hyphenation_zone(&shaped);
                let line = next_line(&words, start, line_width, shaped.hyphen_width, zone, |_| {
                    false
                });
                let (ascent, descent) = line_metrics(&words[start..line.end], &shaped);
                let (ascent, descent) = ruled_line(
                    &style.rules,
                    start == 0,
                    line.end >= words.len(),
                    spaced_line(&style, &words[start..line.end], ascent, descent),
                );
                if start == 0 && (height > 0.0 || style.spacing.add_at_break) {
                    height += style.spacing.above as f32;
                }
                height += ascent + descent;
                bottoms.push(height);
                if line.end >= words.len() {
                    break;
                }
                start = line.end;
            }
            height += style.spacing.below as f32;
        }
        (bottoms, height)
    }

    /// Sets a flow's text in `area`'s columns, top to bottom. Lines that do
    /// not fit are not shown. With `wrap`, the text wraps around page frames
    /// (header and footer text, V14).
    fn flow_in_area(
        &mut self,
        page: usize,
        flow: FlowId,
        container: Container,
        area: Bounds,
        layout_box: &LayoutBox,
        wrap: bool,
    ) {
        let Some(paragraphs) = self.document.flow(flow).map(|flow| flow.paragraphs.clone()) else {
            return;
        };
        let columns = columns(area, layout_box);
        let page_number = self.pages[page].number;
        let mut column = 0;
        let mut y = columns[0].y;
        let in_cell = matches!(container, Container::Cell { .. });
        let alone = paragraphs.len() == 1;
        'paragraphs: for (index, paragraph) in paragraphs.iter().enumerate() {
            let (shaped, style, shown) = if in_cell {
                self.shaped_in_cell(flow, paragraph, alone)
            } else {
                let (shaped, style) = self.shaped(paragraph);
                (shaped, style, None)
            };
            let mut words =
                self.words_for(paragraph, &shaped, &style, columns[column], page_number);
            let has_tabs = words.iter().any(|word| word.tab.is_some());
            let bullet = self.bullet(flow, index, &style);
            let mut start = 0;
            let mut above_done = false;
            loop {
                let bounds = indented(columns[column], style.indents, start == 0);
                let estimate = shaped.ascent + shaped.descent;
                let (x, width) = if wrap {
                    match self.slot_among_page_frames(page, bounds, y, y + estimate) {
                        Some(slot) => slot,
                        None => (bounds.x, 0.0),
                    }
                } else {
                    (bounds.x, bounds.width)
                };
                let (line_bullet, shift) =
                    first_line_bullet(bullet.as_ref(), start, x, style.indents);
                let (x, width) = (x + shift, (width - shift).max(1.0));
                if has_tabs {
                    let column_tabs = layout_box.tab_stops();
                    let stops =
                        tab_stops(&style, style.indents, column_tabs, columns[column].width);
                    size_tabs(
                        &mut words,
                        start,
                        x - columns[column].x,
                        width.max(1.0),
                        &stops,
                        &paragraph.text,
                    );
                }
                let zone = self.hyphenation_zone(&shaped);
                let line = next_line(
                    &words,
                    start,
                    width.max(1.0),
                    shaped.hyphen_width,
                    zone,
                    |_| false,
                );
                let (ascent, descent) = line_metrics(&words[start..line.end], &shaped);
                let (ascent, descent) = ruled_line(
                    &style.rules,
                    start == 0,
                    line.end >= words.len(),
                    spaced_line(&style, &words[start..line.end], ascent, descent),
                );
                let height = ascent + descent;
                // Space above, except at the top of the area unless the style
                // adds it always (S4, S5).
                if start == 0
                    && (y > columns[column].y + 0.5 || style.spacing.add_at_break)
                    && !above_done
                {
                    y += style.spacing.above as f32;
                    above_done = true;
                    continue;
                }
                // With Add in Always, a paragraph's last line and its space
                // below must fit together, as on pages (VM fixtures FS4,
                // FS5: a line with 288 twips below needs 600 to 650 twips
                // of interior, one without 300 to 320).
                let below = if line.end >= words.len() && style.spacing.add_at_break {
                    style.spacing.below as f32
                } else {
                    0.0
                };
                if y + height + below > bounds.bottom() + 0.5 {
                    if column + 1 < columns.len() {
                        column += 1;
                        y = columns[column].y;
                        continue;
                    }
                    break 'paragraphs;
                }
                self.push_line(
                    page,
                    LineSpec {
                        shown: shown.as_ref(),
                        container,
                        flow,
                        paragraph,
                        paragraph_index: index,
                        words: &words,
                        range: start..line.end,
                        x,
                        top: y,
                        ascent,
                        height,
                        slot_width: width,
                        width: line.width,
                        hyphen: line.hyphen,
                        page_number,
                        // REF stands at the cell's left edge (VM TDF1).
                        alignment: if paragraph.text.is_empty() && shown.is_some() {
                            Alignment::Left
                        } else {
                            style.alignment
                        },
                        first: start == 0,
                        last: line.end >= words.len(),
                        rules: &style.rules,
                        column: columns[column],
                        bullet: line_bullet,
                    },
                );
                y += height;
                if line.end >= words.len() {
                    break;
                }
                start = line.end;
            }
            y += style.spacing.below as f32;
        }
    }

    /// The slot for header or footer text beside page frames (V14).
    fn slot_among_page_frames(
        &self,
        page: usize,
        column: Bounds,
        top: f32,
        bottom: f32,
    ) -> Option<(f32, f32)> {
        let mut intervals = vec![(column.x, column.right())];
        for frame_box in &self.pages[page].frames {
            if frame_box.layer != Layer::Page {
                continue;
            }
            let Some(frame) = self.document.frame(frame_box.frame) else {
                continue;
            };
            let bounds = frame_box.bounds;
            if frame.style.wrap == Wrap::NoWrapAround
                || bounds.bottom() <= top
                || bounds.y >= bottom
                || bounds.right() <= column.x
                || bounds.x >= column.right()
            {
                continue;
            }
            if frame.style.wrap == Wrap::NoWrapBeside {
                return None;
            }
            intervals = intervals
                .into_iter()
                .flat_map(|(left, right)| {
                    [
                        (left, right.min(bounds.x)),
                        (left.max(bounds.right()), right),
                    ]
                })
                .filter(|(left, right)| right > left)
                .collect();
        }
        intervals
            .into_iter()
            .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
            .map(|(left, right)| (left, right - left))
    }
}

/// The paragraph rules a line draws (decompile `1108:15c3`, `1288:0000`;
/// VM fixtures NX1, NX2): Margins spans the column whatever the indents;
/// Text the line's text; Other its length from the text's start, ending at
/// its end on a right-aligned line, or centred on a centred one; all within
/// the column. `left..right` is the line's text, or where an empty line's
/// text would start.
fn line_rules(spec: &LineSpec<'_>, (left, right): (f32, f32)) -> Vec<RuleBox> {
    let rules = spec.rules;
    let sides = [
        (spec.first && rules.above.on, rules.above, false),
        (spec.last && rules.below.on, rules.below, true),
    ];
    if !sides.iter().any(|(drawn, _, _)| *drawn) {
        return Vec::new();
    }
    let column = spec.column;
    let (left, right) = match rules.length {
        RuleLength::Margins => (column.x, column.right()),
        RuleLength::Text => (left, right),
        RuleLength::Other => {
            let length = rules.other_length.max(0) as f32;
            match spec.alignment {
                Alignment::Right => (right - length, right),
                Alignment::Centre => {
                    let middle = (left + right) / 2.0;
                    (middle - length / 2.0, middle + length / 2.0)
                }
                Alignment::Left | Alignment::Justify => (left, left + length),
            }
        }
    };
    let (left, right) = (left.max(column.x), right.min(column.right()));
    sides
        .into_iter()
        .filter(|(drawn, side, _)| *drawn && !rule_strokes(side.line).is_empty())
        .filter(|_| right > left)
        .map(|(_, side, below)| RuleBox {
            below,
            x: left,
            width: right - left,
            line: side.line,
            colour: rules.colour,
        })
        .collect()
}

/// Where a line's words go for its alignment: all moved by `shift`, and
/// each space between its first and last words widened by `per_space`
/// (justified lines), so that its box covers the gap.
#[derive(Clone, Copy, Default)]
struct Spread {
    shift: f32,
    per_space: f32,
    /// The logical indexes of the first and last words that are not blank.
    inner: (usize, usize),
}

impl Spread {
    fn of(spec: &LineSpec<'_>) -> Self {
        let free = (spec.slot_width - spec.width).max(0.0);
        match spec.alignment {
            Alignment::Left => Self::default(),
            Alignment::Right => Self {
                shift: free,
                ..Self::default()
            },
            Alignment::Centre => Self {
                shift: free / 2.0,
                ..Self::default()
            },
            Alignment::Justify => {
                // A paragraph's last line, and a line ended by a hard return,
                // stay left.
                let range = spec.range.clone();
                let words = &spec.words[range.clone()];
                if spec.last || words.last().is_some_and(|word| word.forced_break) || free <= 0.0 {
                    return Self::default();
                }
                // Only the text after the line's last tab is justified
                // (decompile `10d0:3ef4`).
                let after_tab = words
                    .iter()
                    .rposition(|word| word.tab.is_some())
                    .map_or(0, |tab| tab + 1);
                let (Some(first), Some(last)) = (
                    words[after_tab..]
                        .iter()
                        .position(|word| !word.blank)
                        .map(|first| first + after_tab),
                    words.iter().rposition(|word| !word.blank),
                ) else {
                    return Self::default();
                };
                let spaces = words[first..last].iter().filter(|word| word.blank).count();
                if spaces == 0 {
                    return Self::default();
                }
                Self {
                    shift: 0.0,
                    per_space: free / spaces as f32,
                    inner: (range.start + first, range.start + last),
                }
            }
        }
    }

    /// The width this spread adds to the word at logical `index`.
    fn extra(self, index: usize, word: &ShapedWord) -> f32 {
        if word.blank && index > self.inner.0 && index < self.inner.1 {
            self.per_space
        } else {
            0.0
        }
    }
}

struct LineSpec<'a> {
    /// A numeric table cell's text as its style shows it, instead of the
    /// source text (TD.5).
    shown: Option<&'a Displayed>,
    container: Container,
    flow: FlowId,
    paragraph: &'a Paragraph,
    paragraph_index: usize,
    words: &'a [ShapedWord],
    range: Range<usize>,
    x: f32,
    top: f32,
    ascent: f32,
    height: f32,
    slot_width: f32,
    width: f32,
    hyphen: bool,
    page_number: u32,
    alignment: Alignment,
    /// The paragraph's first and last lines.
    first: bool,
    last: bool,
    rules: &'a ParagraphRules,
    /// The column the line is set in.
    column: Bounds,
    bullet: Option<BulletBox>,
}
