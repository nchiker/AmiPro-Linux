//! Comparing a document with itself written as a SAM and read back (Phase
//! 4): the corpus audit's measure, and the loss check that Save As SAM runs
//! first (decision B).
//!
//! A document is described as facts: a place (the main text's paragraph 12,
//! frame 3, layout 0's right header, a style) and an aspect (text, runs,
//! placement, definition), each with a value. Places name things by their
//! order and names, never by ID, so two imports of one file describe alike,
//! and so do a document and its written copy when nothing was lost.

use std::collections::{BTreeMap, HashMap};

use amipro_document::{
    CellContent, CharacterRun, Document, Flow, FlowId, Frame, FrameContent, FrameId, GroupId,
    InlineObject, LayoutId, OBJECT_REPLACEMENT_CHARACTER, ObjectId, Paragraph, PictureSource,
    SideFrames, StyleId, StyleOrigin,
};

use crate::sam::{SamOptions, sam_document};
use crate::{ImportError, SheetOptions, import_sam, style_sheet};

/// One thing a document says.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fact {
    pub place: String,
    pub aspect: String,
    pub value: String,
}

/// How a fact of the original fares in the copy.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Change {
    /// Both say it, differently.
    Changed,
    /// Only the original says it.
    Missing,
    /// Only the copy says it.
    Added,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Difference {
    pub place: String,
    pub aspect: String,
    pub change: Change,
}

/// The facts of `document`, in a fixed order: the document's own settings
/// and kept sections, the styles by name, the page layouts with their
/// frames, the page frames back to front, then the main text.
pub fn describe(document: &Document) -> Vec<Fact> {
    let mut describer = Describer::new(document);
    describer.document();
    describer.facts
}

/// What `copy` says differently from `original`, in the original's order,
/// then what only the copy says.
pub fn differences(original: &Document, copy: &Document) -> Vec<Difference> {
    compare(&describe(original), &describe(copy))
}

fn compare(before: &[Fact], after: &[Fact]) -> Vec<Difference> {
    let mut remaining: HashMap<(&str, &str), &str> = after
        .iter()
        .map(|fact| {
            (
                (fact.place.as_str(), fact.aspect.as_str()),
                fact.value.as_str(),
            )
        })
        .collect();
    let mut found = Vec::new();
    for fact in before {
        let key = (fact.place.as_str(), fact.aspect.as_str());
        let change = match remaining.remove(&key) {
            Some(value) if value == fact.value => continue,
            Some(_) => Change::Changed,
            None => Change::Missing,
        };
        found.push(Difference {
            place: fact.place.clone(),
            aspect: fact.aspect.clone(),
            change,
        });
    }
    for fact in after {
        if remaining.contains_key(&(fact.place.as_str(), fact.aspect.as_str())) {
            found.push(Difference {
                place: fact.place.clone(),
                aspect: fact.aspect.clone(),
                change: Change::Added,
            });
        }
    }
    found
}

/// The document as Ami Pro would read it back from the SAM the writer
/// makes. A document that uses a style sheet is read with a sheet written
/// from its own styles and page layout, so that the sheet's part is the
/// document's.
pub fn read_back(document: &Document, options: &SamOptions) -> Result<Document, ImportError> {
    let sam = sam_document(document, options);
    // A document whose sheet was not found is read back as it was read,
    // without one: the sheet Ami Pro finds is the real one, not the
    // defaults that stood for it here.
    let missing = document
        .legacy()
        .get(amipro_document::STYLE_SHEET_MISSING_KEY)
        .is_some();
    // The sheet stands for the real one, so it carries what the document
    // takes from it, the header and footer text included: with contents,
    // since a sheet marks every layout frame as its own and leaves their
    // text out without (`11c8:14e0`).
    let sheet = document.style_sheet().filter(|_| !missing).map(|_| {
        style_sheet(
            document,
            &SheetOptions {
                contents: true,
                description: String::new(),
                saved_at: options.saved_at,
            },
        )
    });
    Ok(import_sam(&sam, sheet.as_deref(), document.title.clone())?.document)
}

/// What a document holds that the SAM writer cannot write exactly
/// (decision B), by kind.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LossKind {
    /// Characters Windows-1252 has no code for: written as `?`.
    Characters,
    /// Paragraphs, their text, breaks, and anchors.
    Text,
    CharacterFormatting,
    ParagraphFormatting,
    Styles,
    /// Style names longer than Ami Pro's 13 characters, shortened (IM4).
    StyleNames,
    PageLayouts,
    Frames,
    Tables,
    /// Table lines in a colour other than black: Ami Pro's are black (TD8),
    /// so Save As writes them black.
    TableLineColours,
    Pictures,
    /// Pictures' descriptions (alt text): Ami Pro has none (P7).
    PictureDescriptions,
    /// Pictures in formats Ami Pro cannot store, which Save As writes as
    /// Windows bitmaps (P8).
    ConvertedPictures,
    /// Image adjustments of a bitmap other than a gray-scale TIFF: Save As
    /// bakes them into the bitmap (OD8); the native file keeps them editable.
    BakedAdjustments,
    /// Pictures linked to files Ami Pro cannot reach (a Linux path), which
    /// Save As writes with a copy of the file (P2).
    LinkedPictures,
    DocumentSettings,
    /// A language Ami Pro has no number for: written as the nearest (SP6).
    Language,
    /// Merge conditions: Ami Pro does not store them (MG6).
    MergeConditions,
    /// Formulas with a function Ami Pro lacks (`@avg`, `@min`, `@max`,
    /// `@count`): the text is written, Ami Pro cannot understand it.
    FormulaFunctions,
}

/// One kind of loss and how many paragraphs, frames, styles, or settings
/// it touches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Loss {
    pub kind: LossKind,
    pub count: usize,
}

impl Loss {
    /// The loss as the Save As dialog lists it.
    pub fn message(&self) -> String {
        let count = self.count;
        let plural = |one: &str, many: &str| {
            if count == 1 {
                format!("1 {one}")
            } else {
                format!("{count} {many}")
            }
        };
        match self.kind {
            LossKind::Characters => format!(
                "{} that Ami Pro cannot store, written as ?",
                plural("character", "characters")
            ),
            LossKind::Text => format!("Text in {}", plural("paragraph", "paragraphs")),
            LossKind::CharacterFormatting => format!(
                "Character formatting in {}",
                plural("paragraph", "paragraphs")
            ),
            LossKind::ParagraphFormatting => format!(
                "Paragraph formatting in {}",
                plural("paragraph", "paragraphs")
            ),
            LossKind::Styles => plural("style", "styles"),
            LossKind::StyleNames => format!(
                "{} longer than Ami Pro's 13 characters, shortened",
                plural("style name", "style names")
            ),
            LossKind::PageLayouts => plural("page layout", "page layouts"),
            LossKind::Frames => plural("frame", "frames"),
            LossKind::Tables => plural("table", "tables"),
            LossKind::TableLineColours => format!(
                "Coloured lines in {}, written black (Ami Pro's table lines are black)",
                plural("table cell", "table cells")
            ),
            LossKind::Pictures => plural("picture", "pictures"),
            LossKind::PictureDescriptions => format!(
                "{}, which Ami Pro cannot store",
                plural("picture description", "picture descriptions")
            ),
            LossKind::ConvertedPictures => format!(
                "{} in a format Ami Pro cannot read, written as {} (reduced above 300 dpi)",
                plural("picture", "pictures"),
                if count == 1 {
                    "a Windows bitmap"
                } else {
                    "Windows bitmaps"
                }
            ),
            LossKind::BakedAdjustments => format!(
                "{} with image adjustments, which are baked into the bitmap (the editable settings stay in .amipro files)",
                plural("picture", "pictures")
            ),
            LossKind::LinkedPictures => format!(
                "{} linked to a file Ami Pro cannot reach, written with {}",
                plural("picture", "pictures"),
                if count == 1 {
                    "a copy of the file"
                } else {
                    "copies of the files"
                }
            ),
            LossKind::DocumentSettings => plural("document setting", "document settings"),
            LossKind::Language => {
                "The document's language, which Ami Pro has no number for (written as the nearest)"
                    .to_string()
            }
            LossKind::MergeConditions => format!(
                "{}, which Ami Pro does not store",
                plural("merge condition", "merge conditions")
            ),
            LossKind::FormulaFunctions => format!(
                "{} with a function Ami Pro lacks (@avg, @min, @max, @count), which it cannot understand",
                plural("formula", "formulas")
            ),
        }
    }
}

/// Sections Ami Pro 3.1 writes in every document, which files from older
/// versions lack: a copy that has them has lost nothing.
const ALWAYS_WRITTEN: [&str; 11] = [
    "section [revisions]",
    "section [prn]",
    "section [port]",
    "section [lang]",
    "section [fldnames]",
    "section [desc]",
    "section [fopts]",
    "section [lnopts]",
    "section [docopts]",
    "section [GramStyle]",
    "section [l1]",
];

/// Whether a difference is what Ami Pro 3.1's writer does to every file
/// it saves: sections an older file lacks, which it writes with their
/// defaults.
pub fn is_writer_addition(difference: &Difference) -> bool {
    difference.place == "document"
        && difference.change == Change::Added
        && ALWAYS_WRITTEN.contains(&difference.aspect.as_str())
}

/// What the SAM writer would not write exactly, by kind; empty when the
/// document reads back as it is.
pub fn sam_losses(document: &Document, options: &SamOptions) -> Result<Vec<Loss>, ImportError> {
    let copy = read_back(document, options)?;
    // Names too long for Ami Pro are shortened as they are written, and
    // listed once as that (IM4).
    let shortened = crate::writer::shortened_style_names(document);
    let before = describe(shortened.as_ref().unwrap_or(document));
    let after = describe(&copy);
    let values: HashMap<(&str, &str), &str> = before
        .iter()
        .map(|fact| {
            (
                (fact.place.as_str(), fact.aspect.as_str()),
                fact.value.as_str(),
            )
        })
        .collect();
    let mut units: BTreeMap<LossKind, std::collections::BTreeSet<String>> = BTreeMap::new();
    let mut characters = 0;
    for fact in &before {
        if fact.aspect == "text" {
            characters += fact.value.chars().filter(|c| !encodable(*c)).count();
        }
    }
    if characters > 0 {
        units.entry(LossKind::Characters).or_default();
    }
    let conditions = document.merge().conditions.len();
    if conditions > 0 {
        units.entry(LossKind::MergeConditions).or_default();
    }
    let long_names = crate::writer::long_style_names(document);
    if long_names > 0 {
        units.entry(LossKind::StyleNames).or_default();
    }
    // What Save As changes in pictures before writing them (P8, P2).
    for frame in document.frames() {
        let FrameContent::Picture(picture) = &frame.content else {
            continue;
        };
        let unit = format!("frame {}", frame.id.raw());
        if !picture.ami_pro_stores() {
            units
                .entry(LossKind::ConvertedPictures)
                .or_default()
                .insert(unit.clone());
        }
        if let PictureSource::Linked(path) = &picture.source
            && !ami_pro_path(path)
        {
            units
                .entry(LossKind::LinkedPictures)
                .or_default()
                .insert(unit);
        }
    }
    // Formulas with the native-only functions (TN4).
    for frame in document.frames() {
        let FrameContent::Table(table) = &frame.content else {
            continue;
        };
        let dims = amipro_document::formula::Dims {
            columns: table.columns.len().max(1),
            rows: table.rows.len().max(1),
        };
        let extras = amipro_document::formula::Options {
            extras: true,
            ..Default::default()
        };
        for (index, cell) in table.cells.iter().enumerate() {
            if let Some(text) = &cell.formula
                && amipro_document::formula::Formula::compile(text, dims, extras)
                    .is_ok_and(|formula| formula.uses_extras)
            {
                units
                    .entry(LossKind::FormulaFunctions)
                    .or_default()
                    .insert(format!("frame {} cell {index}", frame.id.raw()));
            }
        }
    }
    for difference in compare(&before, &after) {
        let key = (difference.place.as_str(), difference.aspect.as_str());
        // Text with characters Windows-1252 lacks is counted once, above.
        if difference.aspect == "text"
            && values
                .get(&key)
                .is_some_and(|text| text.chars().any(|c| !encodable(c)))
        {
            continue;
        }
        if let Some((kind, unit)) = classify(&difference) {
            units.entry(kind).or_default().insert(unit);
        }
    }
    Ok(units
        .into_iter()
        .map(|(kind, units)| Loss {
            kind,
            count: match kind {
                LossKind::Characters => characters,
                LossKind::StyleNames => long_names,
                LossKind::MergeConditions => conditions,
                _ => units.len(),
            },
        })
        .collect())
}

/// Whether Ami Pro can store a character of text.
fn encodable(character: char) -> bool {
    matches!(character, '\u{2028}' | OBJECT_REPLACEMENT_CHARACTER)
        || character == '?'
        || crate::writer::windows_1252(character) != b'?'
}

/// The kind of loss a difference is, and what it touches; `None` for what
/// is not a loss: values derived from what is written (the fonts' printer
/// line heights and kinds) and sections Ami Pro 3.1 always adds.
fn classify(difference: &Difference) -> Option<(LossKind, String)> {
    let place = difference.place.as_str();
    let aspect = difference.aspect.as_str();
    if place == "document" {
        if matches!(aspect, "line heights" | "face kinds") || is_writer_addition(difference) {
            return None;
        }
        if aspect == "page frames" {
            return Some((LossKind::Frames, aspect.into()));
        }
        if aspect == "language" {
            return Some((LossKind::Language, aspect.into()));
        }
        return Some((LossKind::DocumentSettings, aspect.into()));
    }
    if let Some(name) = place.strip_prefix("style ") {
        return Some((LossKind::Styles, name.to_lowercase()));
    }
    let mut words = place.split(' ');
    let first = words.next().unwrap_or_default();
    let second = words.next().unwrap_or_default();
    match first {
        "layout" => Some((LossKind::PageLayouts, second.into())),
        "frame" => {
            let frame = format!("frame {second}");
            if aspect == "cell line colour" {
                Some((LossKind::TableLineColours, place.to_owned()))
            } else if place.contains(" cell ") || aspect == "table" {
                Some((LossKind::Tables, frame))
            } else if aspect == "picture description" {
                Some((LossKind::PictureDescriptions, frame))
            } else if aspect == "picture adjustments" {
                Some((LossKind::BakedAdjustments, frame))
            } else if aspect.starts_with("picture") || aspect == "rendering" {
                Some((LossKind::Pictures, frame))
            } else {
                Some((LossKind::Frames, frame))
            }
        }
        _ => {
            // The main text; a floating header's paragraphs count with
            // their own.
            let kind = match aspect {
                "runs" | "end format" => LossKind::CharacterFormatting,
                "format" | "marks" | "style" => LossKind::ParagraphFormatting,
                _ => LossKind::Text,
            };
            Some((kind, place.into()))
        }
    }
}

/// Whether a linked picture's path is one Ami Pro can follow: a DOS path,
/// as Ami Pro wrote it (`L:\\IN\\PIC.BMP`), not a Linux one.
pub fn ami_pro_path(path: &str) -> bool {
    !path.starts_with('/') && !path.contains("://")
}

/// A 64-bit FNV-1a hash: stable across builds and platforms.
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

struct Describer<'a> {
    document: &'a Document,
    facts: Vec<Fact>,
    /// Page frames by their place in Ami Pro's order (its lists: only the
    /// order within a kind shows, FC8).
    frames: HashMap<FrameId, usize>,
    layouts: HashMap<LayoutId, usize>,
    /// Groups by the order their first member appears in the stack.
    groups: HashMap<GroupId, usize>,
    styles: BTreeMap<StyleId, String>,
}

impl<'a> Describer<'a> {
    fn new(document: &'a Document) -> Self {
        let order = crate::frames::page_frames_in_file_order(document);
        let frames = order
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect();
        let layouts = document
            .layouts()
            .iter()
            .enumerate()
            .map(|(index, layout)| (layout.id, index))
            .collect();
        let mut groups = HashMap::new();
        for id in &order {
            if let Some(group) = document.frame(*id).and_then(|frame| frame.group) {
                let next = groups.len();
                groups.entry(group).or_insert(next);
            }
        }
        let styles = document
            .styles()
            .iter()
            .map(|(id, style)| (*id, style.name.clone()))
            .collect();
        Self {
            document,
            facts: Vec::new(),
            frames,
            layouts,
            groups,
            styles,
        }
    }

    fn fact(&mut self, place: &str, aspect: &str, value: String) {
        self.facts.push(Fact {
            place: place.to_owned(),
            aspect: aspect.to_owned(),
            value,
        });
    }

    fn style_name(&self, id: StyleId) -> String {
        self.styles
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("unknown style {}", id.0))
    }

    fn document(&mut self) {
        let document = self.document;
        self.fact(
            "document",
            "style sheet",
            format!("{:?}", document.style_sheet()),
        );
        self.fact(
            "document",
            "hyphenation zone",
            document.hyphenation_zone().to_string(),
        );
        // The kinds of the faces the document uses: a face named only by a
        // font change that nothing follows serves nothing.
        let used = used_faces(document);
        let kinds: BTreeMap<&String, &u8> = document
            .face_kinds()
            .iter()
            .filter(|(family, _)| used.contains(family.as_str()))
            .collect();
        self.fact("document", "face kinds", format!("{kinds:?}"));
        self.fact(
            "document",
            "line heights",
            format!("{:?}", document.line_heights()),
        );
        let start = document
            .start_layout_id()
            .map(|id| self.layouts.get(&id).copied());
        self.fact("document", "start layout", format!("{start:?}"));
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for kept in &document.legacy().values {
            // Kept for the writer's own use, or written from typed data
            // (`[sty]`), as a constant (`[ver]`, `[charset]`), or from the
            // pictures (`[files]`): not the document's content.
            if [
                "style order",
                crate::sam::SHEET_LAYOUT_KEY,
                amipro_document::STYLE_SHEET_MISSING_KEY,
                "section [ver]",
                "section [sty]",
                "section [charset]",
                "section [files]",
                // The language and grammar set are the document's, below.
                crate::info::LANGUAGE,
                crate::info::GRAMMAR_SET,
            ]
            .iter()
            .any(|key| kept.key.eq_ignore_ascii_case(key))
            {
                continue;
            }
            // A section taken into the model leaves an empty place to keep
            // the order of the sections around it; the model's value is
            // described with the document's (`contents`, `record file`).
            if kept.value.is_empty()
                && [
                    crate::info::TOC,
                    crate::info::MASTER,
                    crate::info::SEQUENCE,
                    crate::info::RECFILE,
                ]
                .iter()
                .any(|key| kept.key.eq_ignore_ascii_case(key))
            {
                continue;
            }
            // Line Numbering is the document's, described below.
            if kept.key.eq_ignore_ascii_case(crate::info::LINE_NUMBERING) {
                continue;
            }
            let count = seen.entry(kept.key.to_lowercase()).or_default();
            *count += 1;
            let aspect = if *count == 1 {
                kept.key.clone()
            } else {
                format!("{} #{count}", kept.key)
            };
            // Doc Info is the document's (`crate::info`), the kept sections
            // only its places: described as Ami Pro writes it.
            let value = if kept.key.eq_ignore_ascii_case(crate::info::DESC) {
                let values = crate::info::description_values(document.info(), 0);
                without_trailing_defaults(&without_save_bookkeeping(&values.join("\n")))
            } else if kept.key.eq_ignore_ascii_case(crate::info::FIELD_NAMES) {
                crate::info::field_name_values(document.info()).join("\n")
            } else if kept.key.eq_ignore_ascii_case(crate::info::FOOTNOTE_OPTIONS) {
                format!("{:?}", document.footnote_options())
            } else {
                kept.value.clone()
            };
            self.fact("document", &aspect, value);
        }
        self.fact("document", "language", format!("{:?}", document.language()));
        self.fact(
            "document",
            "line numbering",
            format!("{:?}", document.line_numbering()),
        );
        self.fact(
            "document",
            "grammar set",
            document.grammar_set().to_string(),
        );
        self.fact("document", "contents", format!("{:?}", document.contents()));
        // The merge's record file; its conditions are listed apart, as SAM
        // cannot hold them (MG6).
        self.fact(
            "document",
            "record file",
            format!("{:?}", document.merge().record_file),
        );
        self.styles();
        for (index, layout) in document.layouts().iter().enumerate() {
            let place = format!("layout {index}");
            let mut copy = (**layout).clone();
            copy.id = LayoutId::from_raw(0);
            let frames = std::mem::take(&mut copy.repeating);
            let header = std::mem::take(&mut copy.header);
            let footer = std::mem::take(&mut copy.footer);
            // The gutter of one column or of uneven columns is what Ami Pro
            // works out from the columns, not a setting.
            copy.right.layout.settle_gutter();
            if let Some(left) = &mut copy.left {
                left.layout.settle_gutter();
            }
            self.fact(&place, "definition", format!("{copy:?}"));
            for (slot, frames) in [("header", header), ("footer", footer)] {
                let SideFrames { right, left } = frames;
                for (side, frame) in [("right", right), ("left", left)] {
                    match frame.and_then(|id| document.frame(id)) {
                        Some(frame) => self.frame(&format!("{place} {slot} {side}"), frame),
                        None => self.fact(&place, &format!("{slot} {side}"), "none".into()),
                    }
                }
            }
            self.fact(&place, "repeating frames", frames.len().to_string());
            for (number, id) in frames.iter().enumerate() {
                if let Some(frame) = document.frame(*id) {
                    self.frame(&format!("{place} repeating {number}"), frame);
                }
            }
        }
        self.fact(
            "document",
            "page frames",
            document.frame_stack().len().to_string(),
        );
        for (index, id) in crate::frames::page_frames_in_file_order(document)
            .iter()
            .enumerate()
        {
            if let Some(frame) = document.frame(*id) {
                self.frame(&format!("frame {index}"), frame);
            }
        }
        self.flow("main", document.main_flow());
    }

    fn styles(&mut self) {
        let mut styles: Vec<_> = self.document.styles().values().collect();
        styles.sort_by_key(|style| style.name.to_lowercase());
        for style in styles {
            let place = format!("style {}", style.name);
            let mut copy = style.clone();
            copy.id = StyleId(0);
            // Ami Pro's styles have no parent: a style is based on another
            // only when it is made (Create Style).
            copy.parent = None;
            // Without Add in Always, the record says When not at break
            // whatever else was kept (`writer.rs`).
            if !copy.spacing.add_at_break {
                copy.kept.add_in = 32;
            }
            let next = copy.next.take().map(|id| self.style_name(id));
            let origin = std::mem::replace(&mut copy.origin, StyleOrigin::Document);
            // A name defined nowhere has no definition to keep: Ami Pro
            // formats its paragraphs as Body Text (VM fixture ST1), and
            // nothing about it is written.
            if origin == StyleOrigin::Missing {
                self.fact(&place, "definition", "defined nowhere".to_owned());
            } else {
                self.fact(&place, "definition", format!("{copy:?} next {next:?}"));
            }
            self.fact(&place, "origin", format!("{origin:?}"));
        }
    }

    fn frame(&mut self, place: &str, frame: &Frame) {
        self.fact(place, "placement", format!("{:?}", frame.placement));
        let mut style = frame.style.clone();
        style.interior.settle_gutter();
        self.fact(place, "style", format!("{style:?}"));
        let group = frame.group.map(|group| self.groups.get(&group).copied());
        self.fact(place, "group", format!("{group:?}"));
        // The frame's sequence number (field 13) is bookkeeping, as IDs
        // are: the writer gives a new frame the next free one.
        let mut kept = frame.legacy.clone();
        kept.values.retain(|value| value.key != "frame.field13");
        // A frame in a cell takes its page and place from the cell when Ami
        // Pro lays the table out (TF1Y): the rectangle read is a cache.
        if frame.placement.is_in_cell() {
            kept.values.retain(|value| value.key != "frame.cell.rect");
        }
        // "No paragraphs in `[txt]`" holds only while the text is empty
        // (a new frame, FC1); once typed in, the writer writes the text.
        if let FrameContent::Text(flow) = frame.content
            && !crate::frames::empty_flow(self.document, flow)
        {
            kept.values.retain(|value| value.key != "txt");
        }
        self.fact(place, "kept", format!("{kept:?}"));
        match &frame.content {
            FrameContent::Text(flow) => {
                self.fact(place, "content", "text".into());
                if let Some(flow) = self.document.flow(*flow) {
                    self.flow(&format!("{place} text"), flow);
                }
            }
            FrameContent::Table(table) => {
                self.fact(place, "content", "table".into());
                self.fact(
                    place,
                    "table",
                    format!(
                        "columns {:?} rows {:?} {:?} {:?} {:?} {:?} {:?}",
                        table.columns,
                        table.rows,
                        table.cell_margins,
                        table.spacing,
                        table.row_options,
                        // An Automatic table's rows have no height of their
                        // own: they fit their text.
                        amipro_document::TableLayout {
                            own_heights: if table.row_options.automatic {
                                Default::default()
                            } else {
                                table.layout.own_heights.clone()
                            },
                            ..table.layout.clone()
                        },
                        table.legacy
                    ),
                );
                let columns = table.columns.len().max(1);
                let order = crate::frames::page_frames_in_file_order(self.document);
                let numbers = crate::frames::cell_frame_numbers(self.document);
                for (index, cell) in table.cells.iter().enumerate() {
                    let cell_place =
                        format!("{place} cell {},{}", index / columns, index % columns);
                    // The frame it holds, by its place among the page frames.
                    let holds = cell
                        .frame
                        .map(|frame| order.iter().position(|id| *id == frame));
                    // The record as the writer writes it: whether text follows
                    // and the frame it names come from the model.
                    let mut kept = cell.legacy.clone();
                    // A kept record only keeps an empty cell in the file; a
                    // cell the writer writes anyway (text, a frame, joined,
                    // its own lines or fill) is the same with or without.
                    let amipro_document::CellContent::Text(flow) = cell.content;
                    if !(cell.span == amipro_document::CellSpan::SINGLE
                        && cell.frame.is_none()
                        && cell.format.is_default()
                        && crate::frames::empty_flow(self.document, flow))
                    {
                        kept.values.retain(|value| value.key != "cell.record");
                    }
                    if let Some(value) = kept
                        .values
                        .iter_mut()
                        .find(|value| value.key == "cell.record")
                    {
                        let record =
                            crate::frames::cell_record(self.document, table, cell, index, &numbers);
                        value.value =
                            String::from_utf8_lossy(&crate::frames::spaced(&record)).into_owned();
                    }
                    // The lines' colour on its own, for its own loss (TD8).
                    let format = amipro_document::CellFormat {
                        line_colour: amipro_document::BLACK,
                        ..cell.format
                    };
                    self.fact(
                        &cell_place,
                        "cell",
                        format!(
                            "{:?} holds {holds:?} {format:?} {:?} {kept:?}",
                            cell.span, cell.formula
                        ),
                    );
                    if !cell.format.lines.is_none()
                        && cell.format.line_colour != amipro_document::BLACK
                    {
                        self.fact(
                            &cell_place,
                            "cell line colour",
                            format!("{:06x}", cell.format.line_colour),
                        );
                    }
                    let CellContent::Text(flow) = cell.content;
                    if let Some(flow) = self.document.flow(flow) {
                        self.flow(&cell_place, flow);
                    }
                }
            }
            FrameContent::Picture(picture) => {
                self.fact(place, "content", "picture".into());
                let source = match &picture.source {
                    PictureSource::Embedded(object) => format!("embedded {}", self.bytes(*object)),
                    PictureSource::Linked(path) => format!("linked {path:?}"),
                };
                self.fact(place, "picture", source);
                self.fact(
                    place,
                    "picture extension",
                    crate::frames::stored_extension(picture),
                );
                let scaling = picture.scaling;
                // A stored size counts only for Custom; the other modes'
                // is recomputed whenever the picture is drawn (`1168:2dd2`).
                let size = (scaling.sizing == amipro_document::PictureSizing::Custom)
                    .then_some(scaling.size);
                self.fact(
                    place,
                    "picture scaling",
                    format!(
                        "{:?} aspect {} {}% {size:?} {}°",
                        scaling.sizing, scaling.keep_aspect, scaling.percent, scaling.rotation
                    ),
                );
                self.fact(place, "picture pan", format!("{:?}", picture.pan));
                self.fact(place, "picture description", picture.description.clone());
                self.fact(
                    place,
                    "picture adjustments",
                    format!("{:?}", picture.adjust),
                );
                // Ami Pro's screen image is a cache of the picture as last
                // drawn: it counts while the record it goes with is current.
                let rendering = picture
                    .rendering
                    .filter(|_| picture.kept_is_current())
                    .map(|object| self.bytes(object));
                self.fact(place, "rendering", format!("{rendering:?}"));
                // What the model does not type; the stored copy's name
                // follows the frame's number.
                let mut kept = crate::frames::kept_isd_parts(picture)
                    .unwrap_or_else(|| crate::frames::new_isd_parts(picture));
                if kept.stored.to_ascii_uppercase().starts_with(".X") {
                    kept.stored = ".X".into();
                }
                if kept.second.is_empty() {
                    kept.second = vec!["0".into()];
                }
                let others: Vec<_> = picture
                    .legacy
                    .values
                    .iter()
                    .filter(|value| value.key != "isd")
                    .collect();
                self.fact(place, "picture kept", format!("{kept:?} {others:?}"));
            }
            FrameContent::Legacy(kept) => {
                self.fact(place, "content", format!("kept {kept:?}"));
            }
        }
    }

    /// An object's bytes by length and hash.
    fn bytes(&self, object: ObjectId) -> String {
        match self.document.objects().get(&object) {
            Some(object) => format!("{} bytes {:016x}", object.bytes.len(), fnv1a(&object.bytes)),
            None => "missing bytes".into(),
        }
    }

    fn flow(&mut self, place: &str, flow: &Flow) {
        self.fact(place, "paragraphs", flow.paragraphs.len().to_string());
        for (index, paragraph) in flow.paragraphs.iter().enumerate() {
            self.paragraph(&format!("{place} paragraph {index}"), paragraph);
        }
    }

    fn paragraph(&mut self, place: &str, paragraph: &Paragraph) {
        let style = self.style_name(paragraph.style);
        self.fact(place, "style", style);
        self.fact(place, "text", paragraph.text.clone());
        self.fact(place, "format", format!("{:?}", paragraph.format));
        self.fact(
            place,
            "runs",
            format!("{:?}", canonical_runs(&paragraph.runs)),
        );
        self.fact(place, "marks", format!("{:?}", paragraph.marks));
        self.fact(place, "end format", format!("{:?}", paragraph.end_format));
        // Ami Pro's cache as written back: only a valid one is.
        let valid = paragraph.cache.filter(|cache| {
            self.document
                .styles()
                .get(&paragraph.style)
                .is_some_and(|style| paragraph.content_fingerprint(style) == cache.fingerprint)
        });
        self.fact(
            place,
            "cache",
            format!("{:?}", valid.map(|cache| (cache.height, cache.width))),
        );
        for (index, object) in paragraph.objects.iter().enumerate() {
            let aspect = format!("object {index}");
            let value = match object {
                InlineObject::Break {
                    kind,
                    centre,
                    layout,
                    ..
                } => format!(
                    "{kind:?} break centre {centre} layout {:?}",
                    layout.map(|id| self.layouts.get(&id).copied())
                ),
                InlineObject::FloatingHeaderFooter { kind, pages, flow } => {
                    self.floating(&format!("{place} floating {index}"), *flow);
                    format!("floating {kind:?} {pages:?}")
                }
                InlineObject::FrameAnchor(frame) => {
                    format!("anchor of frame {:?}", self.frames.get(frame))
                }
                InlineObject::PageNumber(field) => format!("{field:?}"),
                InlineObject::Bullet(code) => format!("bullet {code:?}"),
                InlineObject::Date(field) => format!("{field:?}"),
                InlineObject::DocInfo(field) => format!("{field:?}"),
                InlineObject::Seq(field) => format!("{field:?}"),
                InlineObject::Note { note, flow } => {
                    self.floating(&format!("{place} note {index}"), *flow);
                    format!("{note:?}")
                }
                InlineObject::Footnote { flow } => {
                    self.floating(&format!("{place} footnote {index}"), *flow);
                    "footnote".to_owned()
                }
                InlineObject::TableFootnote { number, row } => {
                    format!("table footnote {number} row {row}")
                }
            };
            self.fact(place, &aspect, value);
        }
    }

    fn floating(&mut self, place: &str, flow: FlowId) {
        if let Some(flow) = self.document.flow(flow) {
            self.flow(place, flow);
        }
    }
}

/// Every face the document's styles, runs, and paragraph ends name, in
/// lower case.
fn used_faces(document: &Document) -> std::collections::BTreeSet<String> {
    let mut faces: std::collections::BTreeSet<String> = document
        .styles()
        .values()
        .map(|style| style.font_family.to_lowercase())
        .collect();
    for (_, flow) in document.flows() {
        for paragraph in &flow.paragraphs {
            let formats = paragraph
                .runs
                .iter()
                .map(|run| &run.format)
                .chain(std::iter::once(&paragraph.end_format));
            faces.extend(formats.filter_map(|format| {
                format
                    .font_family
                    .as_ref()
                    .map(|family| family.to_lowercase())
            }));
        }
    }
    faces
}

/// Runs as their formatting reads: adjacent runs with one format are one,
/// and a run without formatting is none. (A file may change the font back
/// and forth between two characters.)
fn canonical_runs(runs: &[CharacterRun]) -> Vec<CharacterRun> {
    let mut canonical: Vec<CharacterRun> = Vec::with_capacity(runs.len());
    for run in runs {
        if run.format.is_empty() || run.start == run.end {
            continue;
        }
        if let Some(last) = canonical.last_mut()
            && last.end == run.start
            && last.format == run.format
        {
            last.end = run.end;
            continue;
        }
        canonical.push(run.clone());
    }
    canonical
}

/// A section without the values Ami Pro 3.1 adds to an older file's: its
/// reader takes absent numbers as 0 and absent strings as empty, and its
/// writer writes them (`[desc]`'s 21 values, `[fopts]`'s two style names).
fn without_trailing_defaults(raw: &str) -> String {
    let values: Vec<&str> = raw.split('\n').collect();
    let kept = values
        .iter()
        .rposition(|value| !matches!(value.trim(), "" | "0"))
        .map_or(0, |last| last + 1);
    values[..kept].join("\n")
}

/// `[desc]` without the values every save changes: the save time and the
/// revisions (values 6 and 7; decompile `11c8:342b`, `1058:2732`), and the
/// minutes of editing and the statistics counted for the save (values 9 to
/// 14; the fields run).
fn without_save_bookkeeping(raw: &str) -> String {
    raw.split('\n')
        .enumerate()
        .filter(|(index, _)| !matches!(index, 5 | 6 | 8..=13))
        .map(|(_, value)| value)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_sam;

    const SAM: &str = "[ver]\r\n\t4\r\n[desc]\r\n\tA\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t700\r\n\t3\r\n\t600\r\n\t5\r\n\
        [tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
        [edoc]\r\nOne <+!>bold<-!>\r\n\r\nTwo\r\n\r\n>\r\n";

    #[test]
    fn a_document_whose_sheet_is_missing_lists_no_losses() {
        // A SAM naming a sheet that is not here and storing only an inserted
        // layout (a user's document opened without its style-sheet folder):
        // the loss check reads the written copy back as the document was
        // read, without a sheet, so nothing is listed.
        let mut made = amipro_document::Document::from_plain_text("Text");
        let id = made.document_layout().id;
        made.layout_mut(id).unwrap().name = "Brochure".into();
        made.set_style_sheet(Some(amipro_document::StyleSheetLink {
            name: "ELSEWHER.STY".into(),
            layout_from_sheet: false,
        }));
        let options = SamOptions::at(900);
        let written = sam_document(&made, &options);
        let text = String::from_utf8_lossy(&written);
        assert!(text.contains("[sty]\r\n\tELSEWHER.STY\r\n"), "{text}");
        assert!(text.contains("[lay]\r\n\t1\r\n"), "{text}");
        let imported = import_sam(&written, None, "Missing").unwrap().document;
        assert_eq!(
            imported
                .legacy()
                .get(amipro_document::STYLE_SHEET_MISSING_KEY),
            Some("ELSEWHER.STY")
        );
        assert_eq!(sam_losses(&imported, &options).unwrap(), []);
        // With the mark gone (as Use Another Style Sheet removes it), the
        // generated sheet's Standard layout would be taken as the document's.
        let mut marked_found = imported.clone();
        marked_found
            .legacy_mut()
            .values
            .retain(|value| value.key != amipro_document::STYLE_SHEET_MISSING_KEY);
        assert!(!sam_losses(&marked_found, &options).unwrap().is_empty());
    }

    #[test]
    fn two_imports_of_one_file_describe_alike() {
        let first = import_sam(SAM.as_bytes(), None, "A").unwrap().document;
        let second = import_sam(SAM.as_bytes(), None, "B").unwrap().document;
        assert_eq!(describe(&first), describe(&second));
        assert!(differences(&first, &second).is_empty());
    }

    #[test]
    fn differences_name_the_place_and_the_aspect() {
        let original = import_sam(SAM.as_bytes(), None, "A").unwrap().document;
        let changed = SAM
            .replace("Two\r\n\r\n", "Too\r\n\r\nThree\r\n\r\n")
            .replace("\t700\r\n\t3\r\n", "\t900\r\n\t4\r\n");
        let copy = import_sam(changed.as_bytes(), None, "A").unwrap().document;
        let found = differences(&original, &copy);
        let named: Vec<_> = found
            .iter()
            .map(|difference| {
                (
                    difference.place.as_str(),
                    difference.aspect.as_str(),
                    difference.change,
                )
            })
            .collect();
        // The save time and count are bookkeeping, not differences.
        assert_eq!(
            named,
            [
                ("main", "paragraphs", Change::Changed),
                ("main paragraph 1", "text", Change::Changed),
                ("main paragraph 2", "style", Change::Added),
                ("main paragraph 2", "text", Change::Added),
                ("main paragraph 2", "format", Change::Added),
                ("main paragraph 2", "runs", Change::Added),
                ("main paragraph 2", "marks", Change::Added),
                ("main paragraph 2", "end format", Change::Added),
                ("main paragraph 2", "cache", Change::Added),
            ]
        );
    }

    #[test]
    fn the_hash_is_fnv1a() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
