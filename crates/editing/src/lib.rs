//! Typed editing commands applied to immutable document roots, and undo
//! history over those roots (`docs/research/DECISIONS.md`, Phase 2.2, D2 and
//! D6).
//!
//! A command never mutates its input. [`apply`] returns a new root that shares
//! every untouched flow and paragraph with the old one, plus a report of what
//! changed. [`History`] keeps previous roots, so undo and redo swap roots and
//! no command needs a hand-written inverse.

use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;

use amipro_document::{
    Alignment, BreakKind, Caps, CharacterFormat, CharacterRun, Colour, Document, FlowId, FrameId,
    FrameStyle, GroupId, HeaderFooterKind, InlineObject, LayoutId, LegacyMark, MarkPlace,
    OBJECT_REPLACEMENT_CHARACTER, Offset, PageNumberField, PageSides, Paragraph, ParagraphFormat,
    ParagraphId, ParagraphStyle, PictureScaling, Placement, PowerMark, REPLACEMENT_CHARACTER, Rect,
    StyleId, TableRows, Twips,
};

pub mod contents;
pub mod field_language;
pub mod fields;
pub mod find;
pub mod frame_creation;
mod frames;
pub mod insert_document;
pub mod layout_setup;
pub mod lists;
pub mod master;
pub mod merge;
pub mod outline;
mod pictures;
pub mod proofing;
pub mod references;
mod rulers;
pub mod sections;
pub mod sort;
mod styles;
pub mod tables;

pub use frames::{BandBoxes, LayoutSettings, NewFrameContent};
pub use pictures::{NewPicture, NewPictureSource, PictureTarget};
pub mod drawing;
pub use rulers::IndentChange;
pub use styles::style_name_problem;

/// The error type of every command: a short description of why it was
/// rejected.
pub type EditError = &'static str;

/// A position in one flow: a paragraph index and a UTF-8 boundary in that
/// paragraph's text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextPosition {
    pub flow: FlowId,
    pub paragraph: usize,
    pub byte: usize,
}

/// A main-flow position, as the one-flow editors use it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cursor {
    pub paragraph: usize,
    pub byte: usize,
}

impl Cursor {
    pub fn in_flow(self, flow: FlowId) -> TextPosition {
        TextPosition {
            flow,
            paragraph: self.paragraph,
            byte: self.byte,
        }
    }
}

impl From<TextPosition> for Cursor {
    fn from(position: TextPosition) -> Self {
        Self {
            paragraph: position.paragraph,
            byte: position.byte,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Replaces the text between two positions of one flow. A `\n` in `text`
    /// separates paragraphs; CR and CRLF count as `\n`. New paragraphs take
    /// the style of the paragraph at `start`.
    ReplaceText {
        start: TextPosition,
        end: TextPosition,
        text: String,
    },
    /// Sets the style of paragraphs `first..=last` of a flow.
    SetParagraphStyle {
        flow: FlowId,
        first: usize,
        last: usize,
        style: StyleId,
    },
    /// Format › Bullets and Numbering (`lists`): paragraphs `first..=last`
    /// of a flow take the document's list style of that kind, made from Ami
    /// Pro's default when the document has none, or Body Text for `None`.
    SetList {
        flow: FlowId,
        first: usize,
        last: usize,
        list: Option<lists::ListKind>,
    },
    /// Format › Promote and Demote (`outline`): paragraphs `first..=last`
    /// of a flow take the style one outline level up or down, nested list
    /// styles and Ami Pro's Outline# made as needed.
    ChangeLevel {
        flow: FlowId,
        first: usize,
        last: usize,
        change: outline::LevelChange,
    },
    /// Sets the alignment of paragraphs `first..=last` of a flow over their
    /// styles' (Ami Pro's Text › Alignment); `None` returns them to their
    /// styles' alignment.
    SetParagraphAlignment {
        flow: FlowId,
        first: usize,
        last: usize,
        alignment: Option<Alignment>,
    },
    /// Sets the own tab ruler of paragraphs `first..=last` of a flow, as
    /// Ami Pro's current ruler and Page › Ruler do (`1170:052d`): every
    /// paragraph takes the same `tabs`, kept by position, at most 22; an
    /// empty list is a ruler without stops (Clear Tabs); `None` removes
    /// their own rulers, so the style's or the box's stops apply.
    SetParagraphTabs {
        flow: FlowId,
        first: usize,
        last: usize,
        tabs: Option<Vec<amipro_document::TabStop>>,
    },
    /// Sets indents of paragraphs `first..=last` of a flow over their
    /// styles' (the ruler's indent markers; `12e8:01da`): each paragraph
    /// keeps the values `indents` leaves `None` from its own indents, or
    /// its style's when it has none.
    SetParagraphIndents {
        flow: FlowId,
        first: usize,
        last: usize,
        indents: IndentChange,
    },
    /// Returns the indents of paragraphs `first..=last` to their styles'
    /// (Ami Pro's Indention › Revert to style; Paragraph Properties, CM3).
    RevertParagraphIndents {
        flow: FlowId,
        first: usize,
        last: usize,
    },
    /// Sets the line spacing of paragraphs `first..=last` over their
    /// styles' (`<:S…>`, Ami Pro's Text › Spacing…); `None` returns them to
    /// their styles' (Revert to style).
    SetParagraphLineSpacing {
        flow: FlowId,
        first: usize,
        last: usize,
        spacing: Option<amipro_document::LineSpacing>,
    },
    /// Applies direct character formatting between two positions of one flow.
    FormatCharacters {
        start: TextPosition,
        end: TextPosition,
        format: CharacterFormat,
    },
    /// Switches one character attribute on or off between two positions of
    /// one flow, as the Format menu does. On sets it on the text; off sets
    /// it off over a style that has it and otherwise returns the text to its
    /// style, so that no run says what the style already says.
    SetCharacterAttribute {
        start: TextPosition,
        end: TextPosition,
        attribute: CharacterAttribute,
        on: bool,
    },
    /// Sets the face, size (points), and colour between two positions of
    /// one flow, as Ami Pro's Text › Font writes all three: `None` is the
    /// style's (Revert to style).
    SetFont {
        start: TextPosition,
        end: TextPosition,
        family: Option<String>,
        size: Option<f32>,
        colour: Option<Colour>,
    },
    /// Ami Pro's Text › Normal: removes the direct character formatting
    /// between two positions of one flow and the direct paragraph formatting
    /// (indents, spacing, alignment) of the paragraphs they touch, which
    /// return to their styles (VM fixture C1); their own rulers stay
    /// (`12e0:132b`). An empty range clears only its paragraph's formatting.
    ClearDirectFormatting {
        start: TextPosition,
        end: TextPosition,
    },
    /// Deletes the character before `at` (`forward: false`) or after it, or
    /// joins paragraphs at a paragraph boundary: a single Backspace or
    /// Delete. Deleting an object's character deletes the object and what it
    /// owns.
    DeleteCharacter {
        at: TextPosition,
        forward: bool,
    },
    /// Inserts a page or column break at a main-flow position.
    InsertBreak {
        at: TextPosition,
        kind: BreakKind,
        centre: bool,
        layout: Option<LayoutId>,
    },
    /// Inserts a page-number field at a position of any flow (usually a
    /// header or footer), as Ami Pro's Page Numbering does.
    InsertPageNumber {
        at: TextPosition,
        field: PageNumberField,
    },
    /// Inserts one of Ami Pro's bullets (`BULLET_CODES`) into the text at a
    /// position of any flow, as Ami Pro's Edit › Insert › Bullet… does: one
    /// character that takes the formatting where it stands.
    InsertBullet {
        at: TextPosition,
        code: char,
    },
    /// Changes the page-number field whose character is at `at`.
    SetPageNumber {
        at: TextPosition,
        field: PageNumberField,
    },
    /// Edit › Insert › Note at a place in any text (`UI_SPEC.md` §34).
    InsertNote {
        at: TextPosition,
        note: amipro_document::NoteInfo,
        text: String,
    },
    /// Changes the note at `at`: its fields, and its plain text when given.
    SetNote {
        at: TextPosition,
        note: amipro_document::NoteInfo,
        text: Option<String>,
    },
    /// Removes the note at `at`, or every note (Remove All Notes).
    RemoveNotes {
        at: Option<TextPosition>,
    },
    /// Edit › Bookmarks › Add.
    AddBookmark {
        name: String,
        target: references::NewBookmark,
    },
    /// Edit › Bookmarks › Remove (`to` none) or Rename.
    ChangeBookmark {
        name: String,
        to: Option<String>,
    },
    /// Tools › Footnotes › Insert footnote in the main text.
    InsertFootnote {
        at: TextPosition,
    },
    /// Footnote Options.
    SetFootnoteOptions(amipro_document::FootnoteOptions),
    /// Page › Line Numbering (`[lnopts]`), with the dialog's limits.
    SetLineNumbering(amipro_document::LineNumbering),
    /// Outline › Move Up and Move Down (OM3): the section of paragraph
    /// `paragraph` of `flow` over the one before or after it
    /// (`sections`).
    MoveSection {
        flow: FlowId,
        paragraph: usize,
        down: bool,
    },
    /// The Outline navigator's drop: the section of paragraph `paragraph`
    /// to stand before paragraph `to` (`sections::move_section_to`).
    MoveSectionTo {
        flow: FlowId,
        paragraph: usize,
        to: usize,
    },
    /// Tools › Sort… on text (TD.8): paragraphs `first` to `last` of `flow`
    /// in records, by the settings' keys (`sort`).
    SortParagraphs {
        flow: FlowId,
        first: usize,
        last: usize,
        settings: sort::SortSettings,
    },
    /// Tools › Language…: the document's language (`[lang]`).
    SetLanguage(amipro_document::Language),
    /// The grammar and style set a grammar check used (`[GramStyle]`).
    SetGrammarSet(String),
    /// Tools › TOC, Index… › Generate Contents: the TOC's page table at the
    /// document's start (replacing one there), at a position, or as an
    /// output document's text (`contents.rs`; TX2).
    PutToc(Box<contents::TocRequest>),
    /// Tools › TOC, Index… › Generate Index: the index's paragraphs as an
    /// output document's text or at the document's end (TX3).
    PutIndex(Box<contents::IndexRequest>),
    /// Index and TOC entries put into or taken out of the text.
    PutMarks(contents::MarkChanges),
    /// TOC Options' levels and the TOC, Index options (`[toc]`,
    /// `[master]`).
    SetContents(amipro_document::contents::ContentsSettings),
    /// The merge's data file, label values, and conditions (`[recfile]`;
    /// the merge run, MG1, MG6, MG7).
    SetMerge(Box<amipro_document::merge::MergeSettings>),
    /// Inserts a floating header or footer, with a new flow holding one empty
    /// paragraph in the style at `at`, at a main-flow position.
    InsertFloatingHeaderFooter {
        at: TextPosition,
        kind: HeaderFooterKind,
        pages: PageSides,
    },
    /// Creates a frame. Where placed and anchored frames go to the front of
    /// the frame stack, and anchored ones get their anchor at `anchor`.
    /// Repeating frames go to the front of `layout`'s list (the document's
    /// layout by default). Header and footer frames belong to layouts.
    CreateFrame {
        placement: Placement,
        style: FrameStyle,
        content: NewFrameContent,
        anchor: Option<TextPosition>,
        layout: Option<LayoutId>,
    },
    /// Deletes a frame, its whole group, its anchor, and what it owns.
    DeleteFrame {
        frame: FrameId,
    },
    /// Moves a frame and its group. A With para above frame moves only
    /// horizontally, and a Flow with text frame only vertically.
    MoveFrame {
        frame: FrameId,
        dx: Twips,
        dy: Twips,
    },
    /// Moves a Where placed frame and its group to another page.
    SetFramePage {
        frame: FrameId,
        page: u32,
    },
    /// Sets a frame's rectangle, or an anchored frame's size from it.
    ResizeFrame {
        frame: FrameId,
        rect: Rect,
    },
    /// Sets a frame's appearance, interior, name, and macro.
    SetFrameStyle {
        frame: FrameId,
        style: FrameStyle,
    },
    /// Changes a frame's placement. Becoming anchored inserts the anchor at
    /// `anchor`; becoming Where placed removes it; repeating frames move to
    /// `layout` (by default their current one or the document's).
    ChangePlacement {
        frame: FrameId,
        placement: Placement,
        anchor: Option<TextPosition>,
        layout: Option<LayoutId>,
    },
    /// Brings a frame and its group to the front of its kind: the
    /// anchored frames, the Where placed frames (drawn above the anchored
    /// ones, FC8), or a layout's repeating frames.
    BringToFront {
        frame: FrameId,
    },
    /// Sends a frame and its group to the back of its kind.
    SendToBack {
        frame: FrameId,
    },
    /// Groups two or more Where placed frames on one page.
    Group {
        frames: Vec<FrameId>,
    },
    Ungroup {
        group: GroupId,
    },
    /// Ami Pro's Import Picture into an empty frame or an empty table cell
    /// (`UI_SPEC.md` §25); a picture in a cell is a frame of `cell_style`.
    InsertPicture {
        target: PictureTarget,
        picture: NewPicture,
        cell_style: FrameStyle,
    },
    /// Replace Picture: a new picture in the frame, keeping its scaling and
    /// place in the frame.
    ReplacePicture {
        frame: FrameId,
        picture: NewPicture,
    },
    /// How a table's rows take their height and continue across pages:
    /// Automatic, Rows span pages, heading rows, and page breaks in rows
    /// (`UI_SPEC.md` §26).
    SetTableRows {
        frame: FrameId,
        rows: TableRows,
    },
    /// Makes a table at a position in the main text or in an empty text
    /// frame (Insert › Table…, `UI_SPEC.md` §32).
    InsertTable {
        target: tables::TableTarget,
        table: tables::NewTable,
    },
    /// One change to a table: rows and columns, sizes, joins, headings,
    /// cells' lines, fills, leaders, and protection, and the rest of the
    /// Table menu (`UI_SPEC.md` §32).
    EditTable {
        frame: FrameId,
        edit: tables::TableEdit,
    },
    /// Table › Edit Formula…'s OK (`UI_SPEC.md` §41, notes §10.5, §11.3): the
    /// formula text as typed goes to the cell, its result is worked out and
    /// stored, and the cells that read it follow, in one step. A text Ami Pro
    /// cannot understand is refused with [`tables::FORMULA_REFUSED`] and
    /// nothing is stored. `None` takes the formula away: the cell is empty
    /// and plain, as after Del (TDF1).
    SetCellFormula {
        frame: FrameId,
        row: usize,
        column: usize,
        formula: Option<String>,
    },
    /// Table › Quick Add › Row (`along_row`, sums the cells to the left) or
    /// Column (sums the cells above): `@sum(a..b)` into the cell, as Edit
    /// Formula would (notes §10.4).
    QuickAdd {
        frame: FrameId,
        row: usize,
        column: usize,
        along_row: bool,
    },
    /// Every formula's result of the table worked out again (`changed` empty),
    /// or those that read the `changed` cells (row, column): the trigger when
    /// the caret leaves a cell the user typed in, and the Recalculate Table
    /// command for the doubtful case. Refused with
    /// [`tables::NOTHING_RECALCULATED`] when no result changes.
    RecalculateTable {
        frame: FrameId,
        changed: Vec<(usize, usize)>,
    },
    /// Convert Text to Table: the main text's paragraphs `first..=last`,
    /// a row each, a column per tab-separated part, the columns filling
    /// `width` (`UI_SPEC.md` §32, TD7).
    TextToTable {
        first: usize,
        last: usize,
        width: Twips,
        row_height: Twips,
    },
    /// Convert Table to Text: a table in the text becomes paragraphs, a row
    /// each, its cells separated by tabs.
    TableToText {
        frame: FrameId,
    },
    /// Graphics Scaling.
    SetPictureScaling {
        frame: FrameId,
        scaling: PictureScaling,
    },
    /// Picture mode: where the picture is in its frame.
    MovePictureInFrame {
        frame: FrameId,
        pan: Offset,
    },
    SetPictureDescription {
        frame: FrameId,
        description: String,
    },
    /// Image Processing (DO.5, OD8): the picture's adjustments. With
    /// `gray_tiff` the picture (a gray-scale TIFF) takes Ami Pro's own form,
    /// the `.tgf` driver, which keeps the adjustments in `[isd]`.
    SetImageAdjust {
        frame: FrameId,
        adjust: amipro_document::ImageAdjust,
        gray_tiff: bool,
    },
    /// Equations (DO.9): the frame's equation holds this stored text
    /// (`amipro_pictures::equation::Stored::bytes`), drawn at `natural`
    /// twips with `baseline` as its ascent; with `fit_frame` the frame takes
    /// the natural size.
    SetEquation {
        frame: FrameId,
        bytes: std::sync::Arc<[u8]>,
        natural: amipro_document::Size,
        baseline: i32,
        fit_frame: bool,
    },
    /// Draw mode: the frame's drawing holds these objects now (one step;
    /// `UI_SPEC.md` §33).
    SetDrawing {
        frame: FrameId,
        drawing: std::sync::Arc<amipro_pictures::sdw::Drawing>,
    },
    /// Inserts a copy of a layout, with fresh IDs, and a page break at `at`
    /// that switches to it from the next page.
    InsertLayout {
        at: TextPosition,
        from: LayoutId,
    },
    /// Changes a layout's settings. Left pages get copies of the right
    /// pages' header and footer; removing them deletes the left frames.
    ModifyLayout {
        layout: LayoutId,
        settings: LayoutSettings,
    },
    /// Removes an inserted layout that no page break names, with its frames.
    RemoveLayout {
        layout: LayoutId,
    },
    /// Ami Pro's Create Style: a new style named `name` with `based_on`'s
    /// settings, stored in the document.
    CreateStyle {
        name: String,
        based_on: StyleId,
    },
    /// Replaces the definition of the style with `style`'s ID; a style
    /// changed in the document is stored there.
    ModifyStyle {
        style: ParagraphStyle,
    },
    RenameStyle {
        style: StyleId,
        name: String,
    },
    /// Style Management's Remove (VM fixture ST1): the paragraphs keep the
    /// name and format as Body Text, or the style takes the style sheet's
    /// version again when one is given.
    DeleteStyle {
        style: StyleId,
        sheet_version: Option<ParagraphStyle>,
    },
    /// Use Another Style Sheet (VM fixture ST1).
    UseStyleSheet {
        sheet: amipro_document::StyleSheet,
    },
    /// What Save as a Style Sheet does to the document (VM fixtures SD1,
    /// SD2, SS2): with `name`, the document uses the sheet it just wrote,
    /// every style and the page layout being the sheet's; without (an
    /// untitled document), it has no style sheet and every style is stored
    /// in it.
    SavedAsStyleSheet {
        name: Option<String>,
    },
    /// Several commands as one step, applied in order, each to what the one
    /// before left: Replace All (`find`), whose replacements go from the
    /// last to the first so that each position is still where it was found.
    /// The caret is the last command's.
    Several(Vec<Command>),
    /// Insert › File… and Paste from other programs: a document read from a
    /// file put at `at`, its Where placed frames from page `first_page` on
    /// (`insert_document`).
    InsertDocument {
        at: TextPosition,
        source: InsertedDocument,
        first_page: u32,
    },
    /// Inserts a power field kept as marks: its result between its start
    /// and end marks, or its mark alone (`fields::insert_power_field`).
    InsertPowerField {
        at: TextPosition,
        mark: PowerMark,
        result: String,
    },
    /// Deletes the text from `start` to `end` and every power field wholly
    /// in it, its marks at the ends too (`fields::apply_delete_fields`):
    /// Backspace after a field, Delete before one, a selection holding
    /// fields; no field is left behind without its text, invisible.
    DeleteFields {
        start: TextPosition,
        end: TextPosition,
    },
    /// Puts `result` between a field's marks, which stay around it.
    SetFieldResult {
        flow: FlowId,
        start: MarkPlace,
        end: MarkPlace,
        result: String,
    },
    /// Rewrites a field's marks for `mark`: its code, kind, and flags.
    SetFieldMarks {
        flow: FlowId,
        start: MarkPlace,
        end: Option<MarkPlace>,
        mark: PowerMark,
    },
    /// Inserts a date, the time, a Doc Info entry, or a Seq field.
    InsertFieldObject {
        at: TextPosition,
        object: InlineObject,
    },
    /// Sets a date, Doc Info entry, or Seq field in place.
    SetFieldObject {
        at: TextPosition,
        object: InlineObject,
    },
    /// Sets the document's Doc Info (Document Properties, `UI_SPEC.md`
    /// §28).
    SetDocumentInfo(Box<amipro_document::DocumentInfo>),
    /// View › Show Power Fields on: each power field's result set aside and
    /// its code shown in red in its place, to be edited
    /// (`fields::swap_codes`).
    ShowPowerFields,
    /// View › Show Power Fields off: the codes, edited or not, read back and
    /// the results put back.
    HidePowerFields,
}

impl Command {
    pub fn insert(at: TextPosition, text: impl Into<String>) -> Self {
        Self::ReplaceText {
            start: at,
            end: at,
            text: text.into(),
        }
    }

    pub fn delete(start: TextPosition, end: TextPosition) -> Self {
        Self::ReplaceText {
            start,
            end,
            text: String::new(),
        }
    }

    pub fn split_paragraph(at: TextPosition) -> Self {
        Self::insert(at, "\n")
    }

    /// Joins paragraph `paragraph` of `flow` to its predecessor, or `None` for
    /// the first paragraph.
    pub fn join_with_previous(document: &Document, flow: FlowId, paragraph: usize) -> Option<Self> {
        let previous = paragraph.checked_sub(1)?;
        let previous_end = document.flow(flow)?.paragraph(previous)?.text.len();
        Some(Self::delete(
            TextPosition {
                flow,
                paragraph: previous,
                byte: previous_end,
            },
            TextPosition {
                flow,
                paragraph,
                byte: 0,
            },
        ))
    }
}

/// A document a command inserts, compared by identity.
#[derive(Clone, Debug)]
pub struct InsertedDocument(pub Arc<Document>);

impl PartialEq for InsertedDocument {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// What a command changed. Layout uses it as its dirty set.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChangeReport {
    /// Flows whose paragraphs changed.
    pub flows: BTreeSet<FlowId>,
    /// Existing paragraphs whose text, style, runs, objects, or marks changed.
    pub changed: Vec<ParagraphId>,
    pub inserted: Vec<ParagraphId>,
    pub removed: Vec<ParagraphId>,
    /// Flows created for new owners, and flows deleted with their owners.
    pub added_flows: Vec<FlowId>,
    pub removed_flows: Vec<FlowId>,
    /// Frames created or changed (placement, style, group, or z-order).
    pub frames: BTreeSet<FrameId>,
    pub removed_frames: Vec<FrameId>,
    /// Layouts created or changed.
    pub layouts: BTreeSet<LayoutId>,
    pub removed_layouts: Vec<LayoutId>,
    /// Style definitions, or the style sheet the document uses, changed:
    /// every paragraph may look different.
    pub styles: bool,
    /// The Doc Info changed.
    pub info: bool,
    /// Power fields' codes began or stopped showing.
    pub codes: bool,
}

impl ChangeReport {
    pub fn is_empty(&self) -> bool {
        !self.styles
            && !self.info
            && !self.codes
            && self.flows.is_empty()
            && self.added_flows.is_empty()
            && self.removed_flows.is_empty()
            && self.frames.is_empty()
            && self.removed_frames.is_empty()
            && self.layouts.is_empty()
            && self.removed_layouts.is_empty()
    }

    /// Adds another report's changes to this one.
    pub fn merge(&mut self, other: ChangeReport) {
        self.flows.extend(other.flows);
        self.changed.extend(other.changed);
        self.inserted.extend(other.inserted);
        self.removed.extend(other.removed);
        self.added_flows.extend(other.added_flows);
        self.removed_flows.extend(other.removed_flows);
        self.frames.extend(other.frames);
        self.removed_frames.extend(other.removed_frames);
        self.layouts.extend(other.layouts);
        self.removed_layouts.extend(other.removed_layouts);
        self.styles |= other.styles;
        self.info |= other.info;
        self.codes |= other.codes;
    }
}

/// The result of applying a command.
#[derive(Clone, Debug)]
pub struct Applied {
    pub document: Document,
    /// Where the caret goes after a text command.
    pub caret: Option<TextPosition>,
    pub report: ChangeReport,
}

/// Applies `command` to `document`, returning the new root. `document` is
/// unchanged; the new root shares everything the command did not touch.
pub fn apply(document: &Document, command: &Command) -> Result<Applied, EditError> {
    let mut applied = apply_command(document, command)?;
    drop_checked_flags(document, &mut applied);
    Ok(applied)
}

/// Ami Pro's spell check flags a paragraph it passed clean with `<:s>` and
/// skips it in later checks; its text edits clear the flag
/// (`docs/research/PROOFING_NOTES.md`, "Breaking text into words"; VM
/// PSP4B). A paragraph whose text a command changed, or a new one, loses
/// the mark, so Ami Pro checks it again.
fn drop_checked_flags(before: &Document, applied: &mut Applied) {
    const CHECKED: &str = "<:s>";
    let report = &applied.report;
    if report.changed.is_empty() && report.inserted.is_empty() {
        return;
    }
    let touched: BTreeSet<ParagraphId> = report
        .changed
        .iter()
        .chain(&report.inserted)
        .copied()
        .collect();
    let old_text = |flow: FlowId, id: ParagraphId| {
        before
            .flow(flow)
            .and_then(|flow| flow.paragraphs.iter().find(|paragraph| paragraph.id == id))
            .map(|paragraph| paragraph.text.as_str())
    };
    for &flow in &report.flows {
        let Some(current) = applied.document.flow(flow) else {
            continue;
        };
        let stale: Vec<usize> = current
            .paragraphs
            .iter()
            .enumerate()
            .filter(|(_, paragraph)| {
                touched.contains(&paragraph.id)
                    && paragraph.marks.iter().any(|mark| mark.raw == CHECKED)
                    && old_text(flow, paragraph.id) != Some(paragraph.text.as_str())
            })
            .map(|(index, _)| index)
            .collect();
        if stale.is_empty() {
            continue;
        }
        let Some(flow) = applied.document.flow_mut(flow) else {
            continue;
        };
        for index in stale {
            if let Some(paragraph) = flow.paragraphs.get_mut(index) {
                Arc::make_mut(paragraph)
                    .marks
                    .retain(|mark| mark.raw != CHECKED);
            }
        }
    }
}

fn apply_command(document: &Document, command: &Command) -> Result<Applied, EditError> {
    match command {
        Command::ReplaceText { start, end, text } => replace_text(document, *start, *end, text),
        Command::SetParagraphStyle {
            flow,
            first,
            last,
            style,
        } => set_paragraph_style(document, *flow, *first, *last, *style),
        Command::SetList {
            flow,
            first,
            last,
            list,
        } => lists::set_list(document, *flow, *first, *last, *list),
        Command::ChangeLevel {
            flow,
            first,
            last,
            change,
        } => outline::change_level(document, *flow, *first, *last, *change),
        Command::SetParagraphAlignment {
            flow,
            first,
            last,
            alignment,
        } => set_paragraph_alignment(document, *flow, *first, *last, *alignment),
        Command::SetParagraphTabs {
            flow,
            first,
            last,
            tabs,
        } => rulers::set_paragraph_tabs(document, *flow, *first, *last, tabs.as_deref()),
        Command::SetParagraphIndents {
            flow,
            first,
            last,
            indents,
        } => rulers::set_paragraph_indents(document, *flow, *first, *last, *indents),
        Command::RevertParagraphIndents { flow, first, last } => {
            rulers::revert_paragraph_indents(document, *flow, *first, *last)
        }
        Command::SetParagraphLineSpacing {
            flow,
            first,
            last,
            spacing,
        } => rulers::set_paragraph_line_spacing(document, *flow, *first, *last, *spacing),
        Command::FormatCharacters { start, end, format } => {
            if format.is_empty() {
                return Err("character format change is empty");
            }
            modify_characters(document, *start, *end, |_, run| run.overlay(format))
        }
        Command::SetCharacterAttribute {
            start,
            end,
            attribute,
            on,
        } => modify_characters(document, *start, *end, |style, run| {
            attribute.set(run, style, *on);
        }),
        Command::SetFont {
            start,
            end,
            family,
            size,
            colour,
        } => {
            if family
                .as_ref()
                .is_some_and(|family| family.trim().is_empty())
                || size.is_some_and(|size| !size.is_finite() || size <= 0.0)
            {
                return Err("invalid font");
            }
            modify_characters(document, *start, *end, |_, run| {
                run.font_family.clone_from(family);
                run.font_size_points = *size;
                run.colour = *colour;
            })
        }
        Command::ClearDirectFormatting { start, end } => {
            clear_direct_formatting(document, *start, *end)
        }
        Command::DeleteCharacter { at, forward } => delete_character(document, *at, *forward),
        Command::InsertBreak {
            at,
            kind,
            centre,
            layout,
        } => {
            if *kind == BreakKind::Column && (*centre || layout.is_some()) {
                return Err("a column break neither centres nor switches layouts");
            }
            if let Some(layout) = layout
                && !document
                    .layouts()
                    .iter()
                    .any(|candidate| candidate.id == *layout)
            {
                return Err("layout does not exist");
            }
            let object = InlineObject::Break {
                kind: *kind,
                centre: *centre,
                layout: *layout,
                restart: false,
            };
            insert_object(document.clone(), *at, object, None)
        }
        Command::InsertPageNumber { at, field } => {
            check_page_number(field)?;
            replace_range(
                document.clone(),
                *at,
                *at,
                &OBJECT_REPLACEMENT_CHARACTER.to_string(),
                vec![InlineObject::PageNumber(field.clone())],
                None,
            )
        }
        Command::InsertBullet { at, code } => {
            if !amipro_document::is_bullet_code(*code) {
                return Err("not one of Ami Pro's bullets");
            }
            replace_range(
                document.clone(),
                *at,
                *at,
                &OBJECT_REPLACEMENT_CHARACTER.to_string(),
                vec![InlineObject::Bullet(*code)],
                None,
            )
        }
        Command::SetPageNumber { at, field } => set_page_number(document, *at, field),
        Command::InsertNote { at, note, text } => {
            references::insert_note(document, *at, note, text)
        }
        Command::SetNote { at, note, text } => {
            references::set_note(document, *at, note, text.as_deref())
        }
        Command::RemoveNotes { at } => references::remove_notes(document, *at),
        Command::AddBookmark { name, target } => references::add_bookmark(document, name, target),
        Command::ChangeBookmark { name, to } => {
            references::change_bookmark(document, name, to.as_deref())
        }
        Command::InsertFootnote { at } => references::insert_footnote(document, *at),
        Command::SetFootnoteOptions(options) => references::set_footnote_options(document, options),
        Command::SetLineNumbering(numbering) => {
            layout_setup::set_line_numbering(document, numbering)
        }
        Command::MoveSection {
            flow,
            paragraph,
            down,
        } => sections::move_section(document, *flow, *paragraph, *down),
        Command::MoveSectionTo {
            flow,
            paragraph,
            to,
        } => sections::move_section_to(document, *flow, *paragraph, *to),
        Command::SortParagraphs {
            flow,
            first,
            last,
            settings,
        } => sort::sort_paragraphs(document, *flow, *first, *last, settings),
        Command::SetLanguage(language) => proofing::set_language(document, language),
        Command::SetGrammarSet(name) => proofing::set_grammar_set(document, name),
        Command::PutToc(request) => contents::put_toc(document, request),
        Command::PutIndex(request) => contents::put_index(document, request),
        Command::PutMarks(changes) => contents::put_marks(document, changes),
        Command::SetContents(settings) => contents::set_contents(document, settings),
        Command::SetMerge(settings) => merge::set_merge(document, settings),
        Command::InsertPowerField { at, mark, result } => {
            fields::apply_insert_power_field(document, *at, mark, result)
        }
        Command::SetFieldResult {
            flow,
            start,
            end,
            result,
        } => fields::apply_set_field_result(document, *flow, *start, *end, result),
        Command::SetFieldMarks {
            flow,
            start,
            end,
            mark,
        } => fields::apply_set_field_marks(document, *flow, *start, *end, mark),
        Command::DeleteFields { start, end } => fields::apply_delete_fields(document, *start, *end),
        Command::InsertFieldObject { at, object } => {
            if !fields::is_field_object(object) {
                return Err("not a field");
            }
            replace_range(
                document.clone(),
                *at,
                *at,
                &OBJECT_REPLACEMENT_CHARACTER.to_string(),
                vec![object.clone()],
                None,
            )
        }
        Command::SetFieldObject { at, object } => {
            fields::apply_set_field_object(document, *at, object)
        }
        Command::ShowPowerFields => Ok(fields::swap_codes(document, true).0),
        Command::HidePowerFields => Ok(fields::swap_codes(document, false).0),
        Command::SetDocumentInfo(info) => {
            let mut next = document.clone();
            let changed = next.info() != &**info;
            *next.info_mut() = (**info).clone();
            Ok(Applied {
                document: next,
                caret: None,
                report: ChangeReport {
                    info: changed,
                    ..ChangeReport::default()
                },
            })
        }
        Command::InsertFloatingHeaderFooter { at, kind, pages } => {
            let style = document
                .flow(at.flow)
                .and_then(|flow| flow.paragraph(at.paragraph))
                .ok_or("paragraph out of bounds")?
                .style;
            let mut next = document.clone();
            let flow = next.create_flow(style);
            let object = InlineObject::FloatingHeaderFooter {
                kind: *kind,
                pages: *pages,
                flow,
            };
            insert_object(next, *at, object, Some(flow))
        }
        Command::CreateFrame {
            placement,
            style,
            content,
            anchor,
            layout,
        } => frames::create_frame(document, *placement, style, content, *anchor, *layout),
        Command::DeleteFrame { frame } => frames::delete_frame(document, *frame),
        Command::MoveFrame { frame, dx, dy } => frames::move_frame(document, *frame, *dx, *dy),
        Command::SetFramePage { frame, page } => frames::set_frame_page(document, *frame, *page),
        Command::ResizeFrame { frame, rect } => frames::resize_frame(document, *frame, *rect),
        Command::SetFrameStyle { frame, style } => frames::set_frame_style(document, *frame, style),
        Command::ChangePlacement {
            frame,
            placement,
            anchor,
            layout,
        } => frames::change_placement(document, *frame, *placement, *anchor, *layout),
        Command::BringToFront { frame } => frames::restack(document, *frame, true),
        Command::SendToBack { frame } => frames::restack(document, *frame, false),
        Command::Group { frames: members } => frames::group(document, members),
        Command::Ungroup { group } => frames::ungroup(document, *group),
        Command::InsertPicture {
            target,
            picture,
            cell_style,
        } => pictures::insert(document, *target, picture, cell_style),
        Command::ReplacePicture { frame, picture } => pictures::replace(document, *frame, picture),
        Command::SetTableRows { frame, rows } => frames::set_table_rows(document, *frame, rows),
        Command::InsertTable { target, table } => tables::insert_table(document, *target, table),
        Command::EditTable { frame, edit } => tables::edit_table(document, *frame, edit),
        Command::SetCellFormula {
            frame,
            row,
            column,
            formula,
        } => tables::set_cell_formula(document, *frame, *row, *column, formula.as_deref()),
        Command::QuickAdd {
            frame,
            row,
            column,
            along_row,
        } => tables::quick_add(document, *frame, *row, *column, *along_row),
        Command::RecalculateTable { frame, changed } => {
            tables::recalculate_table(document, *frame, changed)
        }
        Command::TextToTable {
            first,
            last,
            width,
            row_height,
        } => tables::text_to_table(document, *first, *last, *width, *row_height),
        Command::TableToText { frame } => tables::table_to_text(document, *frame),
        Command::SetPictureScaling { frame, scaling } => {
            pictures::set_scaling(document, *frame, *scaling)
        }
        Command::MovePictureInFrame { frame, pan } => {
            pictures::move_in_frame(document, *frame, *pan)
        }
        Command::SetDrawing { frame, drawing } => drawing::set_drawing(document, *frame, drawing),
        Command::SetEquation {
            frame,
            bytes,
            natural,
            baseline,
            fit_frame,
        } => pictures::set_equation(document, *frame, bytes, *natural, *baseline, *fit_frame),
        Command::SetPictureDescription { frame, description } => {
            pictures::set_description(document, *frame, description)
        }
        Command::SetImageAdjust {
            frame,
            adjust,
            gray_tiff,
        } => pictures::set_adjust(document, *frame, *adjust, *gray_tiff),
        Command::InsertLayout { at, from } => frames::insert_layout(document, *at, *from),
        Command::ModifyLayout { layout, settings } => {
            frames::modify_layout(document, *layout, settings)
        }
        Command::RemoveLayout { layout } => frames::remove_layout(document, *layout),
        Command::CreateStyle { name, based_on } => styles::create_style(document, name, *based_on),
        Command::ModifyStyle { style } => styles::modify_style(document, style),
        Command::RenameStyle { style, name } => styles::rename_style(document, *style, name),
        Command::DeleteStyle {
            style,
            sheet_version,
        } => styles::delete_style(document, *style, sheet_version.as_ref()),
        Command::UseStyleSheet { sheet } => styles::use_style_sheet(document, sheet),
        Command::SavedAsStyleSheet { name } => {
            styles::saved_as_style_sheet(document, name.as_deref())
        }
        Command::InsertDocument {
            at,
            source,
            first_page,
        } => insert_document::insert_document(document, *at, &source.0, *first_page),
        Command::Several(commands) => {
            let mut applied = Applied {
                document: document.clone(),
                caret: None,
                report: ChangeReport::default(),
            };
            for command in commands {
                let next = match command {
                    // The caret so far moves with the text the codes swap.
                    Command::ShowPowerFields | Command::HidePowerFields => {
                        let show = matches!(command, Command::ShowPowerFields);
                        let (mut next, swaps) = fields::swap_codes(&applied.document, show);
                        next.caret = applied.caret.map(|caret| swaps.map(caret));
                        next
                    }
                    _ => apply(&applied.document, command)?,
                };
                applied.document = next.document;
                applied.caret = next.caret.or(applied.caret);
                applied.report.merge(next.report);
            }
            Ok(applied)
        }
    }
}

fn ordered(start: TextPosition, end: TextPosition) -> (TextPosition, TextPosition) {
    if (start.paragraph, start.byte) <= (end.paragraph, end.byte) {
        (start, end)
    } else {
        (end, start)
    }
}

fn replace_text(
    document: &Document,
    start: TextPosition,
    end: TextPosition,
    text: &str,
) -> Result<Applied, EditError> {
    // Text from input or the clipboard never creates an object.
    let replacement = normalize_line_breaks(text).replace(
        OBJECT_REPLACEMENT_CHARACTER,
        &REPLACEMENT_CHARACTER.to_string(),
    );
    replace_range(document.clone(), start, end, &replacement, Vec::new(), None)
}

fn normalize_line_breaks(text: &str) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.to_owned()
    }
}

/// The span that changed between two texts, as the lengths of their common
/// prefix and of their common suffix after it, both on UTF-8 boundaries of
/// both texts. Comparison runs at memory speed, so editors can call this per
/// keystroke on long documents.
pub fn changed_span(old: &str, new: &str) -> (usize, usize) {
    let (old_bytes, new_bytes) = (old.as_bytes(), new.as_bytes());
    let mut prefix = common_prefix_len(old_bytes, new_bytes);
    while !(old.is_char_boundary(prefix) && new.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let mut suffix = common_suffix_len(&old_bytes[prefix..], &new_bytes[prefix..]);
    while !(old.is_char_boundary(old.len() - suffix) && new.is_char_boundary(new.len() - suffix)) {
        suffix -= 1;
    }
    (prefix, suffix)
}

const COMPARE_CHUNK: usize = 64;

fn common_prefix_len(left: &[u8], right: &[u8]) -> usize {
    let limit = left.len().min(right.len());
    let mut length = 0;
    while length + COMPARE_CHUNK <= limit
        && left[length..length + COMPARE_CHUNK] == right[length..length + COMPARE_CHUNK]
    {
        length += COMPARE_CHUNK;
    }
    while length < limit && left[length] == right[length] {
        length += 1;
    }
    length
}

fn common_suffix_len(left: &[u8], right: &[u8]) -> usize {
    let limit = left.len().min(right.len());
    let (left_end, right_end) = (left.len(), right.len());
    let mut length = 0;
    while length + COMPARE_CHUNK <= limit
        && left[left_end - length - COMPARE_CHUNK..left_end - length]
            == right[right_end - length - COMPARE_CHUNK..right_end - length]
    {
        length += COMPARE_CHUNK;
    }
    while length < limit && left[left_end - 1 - length] == right[right_end - 1 - length] {
        length += 1;
    }
    length
}

/// Replaces `start..end` of one flow in `next` with `replacement`, which holds
/// one U+FFFC per entry of `inserted_objects`.
///
/// Objects whose U+FFFC is in the range are deleted with everything they own.
/// Objects, runs, and legacy marks outside the range keep their characters.
/// A legacy mark strictly inside the range is deleted; one at either end of
/// it ends up after the inserted text, as a run start would.
pub(crate) fn replace_range(
    mut next: Document,
    start: TextPosition,
    end: TextPosition,
    replacement: &str,
    inserted_objects: Vec<InlineObject>,
    detach: Option<FrameId>,
) -> Result<Applied, EditError> {
    if start.flow != end.flow {
        return Err("a text range must stay in one flow");
    }
    let (start, end) = ordered(start, end);
    let flow_paragraphs = next
        .flow(start.flow)
        .ok_or("flow does not exist")?
        .paragraphs
        .clone();
    let start_paragraph = Arc::clone(
        flow_paragraphs
            .get(start.paragraph)
            .ok_or("paragraph out of bounds")?,
    );
    let end_paragraph = Arc::clone(
        flow_paragraphs
            .get(end.paragraph)
            .ok_or("paragraph out of bounds")?,
    );
    if start.byte > start_paragraph.text.len()
        || end.byte > end_paragraph.text.len()
        || !start_paragraph.text.is_char_boundary(start.byte)
        || !end_paragraph.text.is_char_boundary(end.byte)
    {
        return Err("replacement is not a UTF-8-safe range");
    }
    if replacement.matches(OBJECT_REPLACEMENT_CHARACTER).count() != inserted_objects.len() {
        return Err("inserted text and inserted objects disagree");
    }
    if start == end && replacement.is_empty() {
        return Ok(Applied {
            document: next,
            caret: Some(start),
            report: ChangeReport::default(),
        });
    }

    let same_paragraph = start.paragraph == end.paragraph;
    let mut report = ChangeReport::default();

    // Objects before the range, inside it, and after it.
    let mut prefix_objects = Vec::new();
    let mut removed_objects = Vec::new();
    for (offset, object) in start_paragraph
        .object_offsets()
        .zip(&start_paragraph.objects)
    {
        if offset < start.byte {
            prefix_objects.push(object.clone());
        } else if !same_paragraph || offset < end.byte {
            removed_objects.push(object.clone());
        }
    }
    for paragraph in &flow_paragraphs[start.paragraph + 1..end.paragraph.max(start.paragraph + 1)] {
        removed_objects.extend(paragraph.objects.iter().cloned());
    }
    let mut suffix_objects = Vec::new();
    for (offset, object) in end_paragraph.object_offsets().zip(&end_paragraph.objects) {
        if offset >= end.byte {
            suffix_objects.push(object.clone());
        } else if !same_paragraph {
            removed_objects.push(object.clone());
        }
    }

    // Legacy marks before the range, and those that end up after the
    // inserted text (offsets relative to its end). The end marks of power
    // fields that close where the range starts stay before it: text typed
    // right after a field goes after it, as Ami Pro's caret stands after a
    // field it has put in (decision MC11).
    let closing = start_paragraph
        .marks
        .iter()
        .skip_while(|mark| mark.byte < start.byte)
        .take_while(|mark| {
            // (While codes show, an end mark holds what its field set
            // aside, and showing them back takes typed text out itself.)
            mark.byte == start.byte
                && !mark.raw.contains('\0')
                && amipro_document::PowerMark::parse(&mark.raw).is_some_and(|power| power.end)
        })
        .count();
    let before = start_paragraph
        .marks
        .iter()
        .filter(|mark| mark.byte < start.byte)
        .count();
    let prefix_marks: Vec<LegacyMark> = start_paragraph.marks[..before + closing].to_vec();
    let mut after_marks: Vec<LegacyMark> = start_paragraph
        .marks
        .iter()
        .enumerate()
        .filter(|(index, mark)| {
            !same_paragraph && mark.byte == start.byte && *index >= before + closing
        })
        .map(|(_, mark)| LegacyMark {
            byte: 0,
            raw: mark.raw.clone(),
        })
        .collect();
    after_marks.extend(
        end_paragraph
            .marks
            .iter()
            .enumerate()
            .filter(|(index, mark)| {
                mark.byte >= end.byte
                    || (same_paragraph && mark.byte == start.byte && *index >= before + closing)
            })
            .filter(|(index, _)| !same_paragraph || *index >= before + closing)
            .map(|(_, mark)| LegacyMark {
                byte: mark.byte.saturating_sub(end.byte),
                raw: mark.raw.clone(),
            }),
    );

    let prefix = &start_paragraph.text[..start.byte];
    let suffix = &end_paragraph.text[end.byte..];
    let lines = replacement.split('\n').collect::<Vec<_>>();
    let mut line_objects = inserted_objects.into_iter();
    let mut objects_for = |line: &str| -> Vec<InlineObject> {
        line_objects
            .by_ref()
            .take(line.matches(OBJECT_REPLACEMENT_CHARACTER).count())
            .collect()
    };
    let shifted = |marks: &[LegacyMark], by: usize| -> Vec<LegacyMark> {
        marks
            .iter()
            .map(|mark| LegacyMark {
                byte: mark.byte + by,
                raw: mark.raw.clone(),
            })
            .collect()
    };
    let mut after = Vec::with_capacity(lines.len());
    // Text inserted into an empty paragraph lands after its format changes
    // and takes its end format.
    let typed_into_empty = |runs: &mut Vec<CharacterRun>, length: usize| {
        if start_paragraph.text.is_empty() && length > 0 && !start_paragraph.end_format.is_empty() {
            runs.insert(
                0,
                CharacterRun {
                    start: 0,
                    end: length,
                    format: start_paragraph.end_format.clone(),
                },
            );
            merge_adjacent_runs(runs);
        }
    };

    if lines.len() == 1 {
        let inserted = lines[0];
        let mut runs = if same_paragraph {
            // Text typed or pasted inside a paragraph extends the run it lands
            // in or at the end of.
            runs_after_insert(
                &runs_after_delete(&start_paragraph.runs, start.byte, end.byte),
                start.byte,
                inserted.len(),
            )
        } else {
            let mut runs = runs_for_slice(&start_paragraph.runs, 0, start.byte, 0);
            runs.extend(runs_for_slice(
                &end_paragraph.runs,
                end.byte,
                end_paragraph.text.len(),
                prefix.len() + inserted.len(),
            ));
            merge_adjacent_runs(&mut runs);
            runs
        };
        typed_into_empty(&mut runs, inserted.len());
        let mut objects = prefix_objects;
        objects.extend(objects_for(inserted));
        objects.extend(suffix_objects);
        let mut marks = prefix_marks;
        marks.extend(shifted(&after_marks, prefix.len() + inserted.len()));
        after.push(Paragraph {
            id: start_paragraph.id,
            text: format!("{prefix}{inserted}{suffix}"),
            style: start_paragraph.style,
            format: start_paragraph.format.clone(),
            runs,
            objects,
            marks,
            // The paragraph keeps the end of the range's last paragraph.
            end_format: end_paragraph.end_format.clone(),
            // Ami Pro's cache stays with what it was read for: written back
            // only while the paragraph is unchanged (its fingerprint).
            cache: start_paragraph.cache,
        });
    } else {
        let mut objects = prefix_objects;
        objects.extend(objects_for(lines[0]));
        let mut runs = runs_for_slice(&start_paragraph.runs, 0, start.byte, 0);
        typed_into_empty(&mut runs, lines[0].len());
        // A paragraph end inserted with the text takes the format typing at
        // the caret would take (VM fixture EN1: Enter after a bold word, or
        // before one, gives a bold paragraph end).
        let inserted_end = caret_format(&start_paragraph, start.byte).clone();
        after.push(Paragraph {
            id: start_paragraph.id,
            text: format!("{prefix}{}", lines[0]),
            style: start_paragraph.style,
            format: start_paragraph.format.clone(),
            runs,
            objects,
            marks: prefix_marks,
            end_format: inserted_end.clone(),
            cache: start_paragraph.cache,
        });
        for middle in &lines[1..lines.len() - 1] {
            let mut paragraph = next.new_paragraph(*middle, start_paragraph.style);
            paragraph.format = start_paragraph.format.clone();
            paragraph.objects = objects_for(middle);
            paragraph.end_format = inserted_end.clone();
            after.push(paragraph);
        }
        let last = lines[lines.len() - 1];
        // Enter at a paragraph's end gives the new, empty paragraph the
        // style's next style while Next style is on; in the middle or at the
        // start both parts keep the style (VM fixture NX1).
        let next_style = next
            .styles()
            .get(&start_paragraph.style)
            .filter(|style| style.breaks.next_style() && last.is_empty() && suffix.is_empty())
            .map(|style| style.next.unwrap_or(style.id));
        let mut last_paragraph = next.new_paragraph(
            format!("{last}{suffix}"),
            next_style.unwrap_or(start_paragraph.style),
        );
        if next_style.is_none() {
            last_paragraph.format = start_paragraph.format.clone();
        }
        last_paragraph.runs = runs_for_slice(
            &end_paragraph.runs,
            end.byte,
            end_paragraph.text.len(),
            last.len(),
        );
        last_paragraph.objects = objects_for(last);
        last_paragraph.objects.extend(suffix_objects);
        last_paragraph.marks = shifted(&after_marks, last.len());
        // The last paragraph keeps the range's paragraph end; empty, it is
        // where typing continues (EN1).
        last_paragraph.end_format = if last_paragraph.text.is_empty() {
            inserted_end
        } else {
            end_paragraph.end_format.clone()
        };
        after.push(last_paragraph);
    }

    let caret = TextPosition {
        flow: start.flow,
        paragraph: start.paragraph + after.len() - 1,
        byte: if after.len() == 1 {
            prefix.len() + lines[0].len()
        } else {
            lines.last().map_or(0, |line| line.len())
        },
    };
    report.flows.insert(start.flow);
    report.changed.push(start_paragraph.id);
    report
        .inserted
        .extend(after[1..].iter().map(|paragraph| paragraph.id));
    report.removed.extend(
        flow_paragraphs[start.paragraph + 1..=end.paragraph]
            .iter()
            .map(|paragraph| paragraph.id),
    );

    let mut paragraphs = flow_paragraphs;
    paragraphs.splice(
        start.paragraph..=end.paragraph,
        after.into_iter().map(Arc::new),
    );
    next.replace_flow_paragraphs(start.flow, paragraphs)
        .map_err(|_| "flow does not exist")?;
    for object in &removed_objects {
        if object.anchored_frame().is_none() || object.anchored_frame() != detach {
            remove_owned(&mut next, object, &mut report);
        }
    }
    Ok(Applied {
        document: next,
        caret: Some(caret),
        report,
    })
}

/// Removes what a deleted object owns: a floating header's flow, or an
/// anchored frame and everything it owns.
fn remove_owned(document: &mut Document, object: &InlineObject, report: &mut ChangeReport) {
    if let Some(flow) = object.owned_flow()
        && document.remove_flow(flow).is_some()
    {
        report.removed_flows.push(flow);
    }
    if let Some(frame) = object.anchored_frame() {
        frames::delete_frame_tree(document, frame, report);
    }
}

/// Inserts one inline object at `at` in the main flow.
fn insert_object(
    next: Document,
    at: TextPosition,
    object: InlineObject,
    created_flow: Option<FlowId>,
) -> Result<Applied, EditError> {
    if at.flow != next.main_flow_id() {
        return Err("inline objects occur only in the main flow");
    }
    let mut applied = replace_range(
        next,
        at,
        at,
        &OBJECT_REPLACEMENT_CHARACTER.to_string(),
        vec![object],
        None,
    )?;
    applied.report.added_flows.extend(created_flow);
    Ok(applied)
}

/// What Ami Pro's Page Numbering can set (`1140:0000`): one of its five
/// styles, start page and start number up to 9999, and at most 30
/// characters of leading text on one line.
fn check_page_number(field: &PageNumberField) -> Result<(), EditError> {
    if !PageNumberField::STYLES.contains(&field.style) {
        return Err("page numbers are 1, I, i, A, or a");
    }
    if field.start_page > 9_999 || field.start_number > 9_999 {
        return Err("a start page or number is at most 9999");
    }
    if field.leading_text.chars().count() > 30 {
        return Err("leading text is at most 30 characters");
    }
    if field
        .leading_text
        .contains(['\n', '\r', '\u{2028}', OBJECT_REPLACEMENT_CHARACTER])
    {
        return Err("leading text is one line of plain text");
    }
    Ok(())
}

fn set_page_number(
    document: &Document,
    at: TextPosition,
    field: &PageNumberField,
) -> Result<Applied, EditError> {
    check_page_number(field)?;
    let paragraph = document
        .flow(at.flow)
        .and_then(|flow| flow.paragraph(at.paragraph))
        .ok_or("paragraph out of bounds")?;
    let index = paragraph
        .object_offsets()
        .position(|offset| offset == at.byte)
        .filter(|index| matches!(paragraph.objects[*index], InlineObject::PageNumber(_)))
        .ok_or("there is no page number there")?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let paragraph = next
        .flow_mut(at.flow)
        .and_then(|flow| flow.paragraph_mut(at.paragraph))
        .ok_or("paragraph out of bounds")?;
    let replacement = InlineObject::PageNumber(field.clone());
    if paragraph.objects[index] != replacement {
        paragraph.objects[index] = replacement;
        report.changed.push(paragraph.id);
        report.flows.insert(at.flow);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

fn delete_character(
    document: &Document,
    at: TextPosition,
    forward: bool,
) -> Result<Applied, EditError> {
    let flow = document.flow(at.flow).ok_or("flow does not exist")?;
    let paragraph = flow
        .paragraph(at.paragraph)
        .ok_or("paragraph out of bounds")?;
    let text = paragraph.text.as_str();
    if at.byte > text.len() || !text.is_char_boundary(at.byte) {
        return Err("cursor is not a UTF-8 boundary");
    }
    let unchanged = || {
        Ok(Applied {
            document: document.clone(),
            caret: Some(at),
            report: ChangeReport::default(),
        })
    };
    let (start, end) = if forward {
        if let Some(character) = text[at.byte..].chars().next() {
            let end = TextPosition {
                byte: at.byte + character.len_utf8(),
                ..at
            };
            (at, end)
        } else if at.paragraph + 1 < flow.len() {
            let end = TextPosition {
                paragraph: at.paragraph + 1,
                byte: 0,
                ..at
            };
            (at, end)
        } else {
            return unchanged();
        }
    } else if let Some((byte, _)) = text[..at.byte].char_indices().next_back() {
        (TextPosition { byte, ..at }, at)
    } else if let Some(previous) = at.paragraph.checked_sub(1) {
        let start = TextPosition {
            paragraph: previous,
            byte: flow.paragraphs[previous].text.len(),
            ..at
        };
        (start, at)
    } else {
        return unchanged();
    };
    // A single Delete or Backspace onto a frame anchor does nothing (V05);
    // deleting a selection that holds it deletes the frame.
    let removed = flow
        .paragraph(start.paragraph)
        .filter(|_| start.paragraph == end.paragraph)
        .and_then(|paragraph| paragraph.object_at(start.byte));
    if removed.is_some_and(|object| object.anchored_frame().is_some()) {
        return unchanged();
    }
    replace_range(document.clone(), start, end, "", Vec::new(), None)
}

/// The main-flow position of a byte offset in `Document::plain_text()`, where
/// each paragraph boundary is one `\n` byte.
pub fn main_position_at_offset(document: &Document, offset: usize) -> Option<TextPosition> {
    let mut paragraph_start = 0;
    for (paragraph, source) in document.paragraphs().iter().enumerate() {
        let paragraph_end = paragraph_start + source.text.len();
        if offset <= paragraph_end {
            let byte = offset - paragraph_start;
            return source.text.is_char_boundary(byte).then_some(TextPosition {
                flow: document.main_flow_id(),
                paragraph,
                byte,
            });
        }
        paragraph_start = paragraph_end + 1;
    }
    None
}

/// The command that turns the main flow's plain text into `text`, for an
/// editor that edits that projection (one line per paragraph), or `None` if
/// nothing changed.
///
/// The edit is one replacement of the changed span. `caret`, the byte offset
/// in `text` where the edit left the caret, decides where an ambiguous edit
/// happened: typing "o" after "o", or pressing Enter at the end of one
/// paragraph rather than the start of the next. The replacement goes through
/// [`Command::ReplaceText`], so the object rule applies and no object is
/// invented.
pub fn plain_text_edit(document: &Document, text: &str, caret: Option<usize>) -> Option<Command> {
    let text = normalize_line_breaks(text);
    let old = document.plain_text();
    if old == text {
        return None;
    }
    let (old_bytes, new_bytes) = (old.as_bytes(), text.as_bytes());
    let common = old_bytes.len().min(new_bytes.len());
    let longest_prefix = common_prefix_len(old_bytes, new_bytes);
    let longest_suffix = common_suffix_len(old_bytes, new_bytes);
    let mut prefix = longest_prefix.min(common);
    let mut suffix = longest_suffix.min(common - prefix);
    if let Some(caret) = caret.filter(|caret| *caret <= new_bytes.len()) {
        // Prefer the alignment whose changed span in `text` ends at the caret.
        let total = prefix + suffix;
        let candidate = new_bytes.len() - caret;
        if candidate <= longest_suffix && candidate <= total && total - candidate <= longest_prefix
        {
            suffix = candidate;
            prefix = total - candidate;
        }
    }
    while !(old.is_char_boundary(prefix) && text.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    while !(old.is_char_boundary(old.len() - suffix) && text.is_char_boundary(text.len() - suffix))
    {
        suffix -= 1;
    }
    Some(Command::ReplaceText {
        start: main_position_at_offset(document, prefix)?,
        end: main_position_at_offset(document, old.len() - suffix)?,
        text: text[prefix..text.len() - suffix].to_owned(),
    })
}

fn set_paragraph_style(
    document: &Document,
    flow_id: FlowId,
    first: usize,
    last: usize,
    style: StyleId,
) -> Result<Applied, EditError> {
    if !document.styles().contains_key(&style) {
        return Err("style does not exist");
    }
    let flow = document.flow(flow_id).ok_or("flow does not exist")?;
    if first > last || last >= flow.len() {
        return Err("paragraph out of bounds");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for index in first..=last {
        if flow.paragraphs[index].style == style {
            continue;
        }
        let paragraph = next
            .flow_mut(flow_id)
            .and_then(|flow| flow.paragraph_mut(index))
            .ok_or("paragraph out of bounds")?;
        paragraph.style = style;
        report.changed.push(paragraph.id);
    }
    if !report.changed.is_empty() {
        report.flows.insert(flow_id);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

fn set_paragraph_alignment(
    document: &Document,
    flow_id: FlowId,
    first: usize,
    last: usize,
    alignment: Option<Alignment>,
) -> Result<Applied, EditError> {
    let flow = document.flow(flow_id).ok_or("flow does not exist")?;
    if first > last || last >= flow.len() {
        return Err("paragraph out of bounds");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for index in first..=last {
        if flow.paragraphs[index].format.alignment == alignment {
            continue;
        }
        let paragraph = next
            .flow_mut(flow_id)
            .and_then(|flow| flow.paragraph_mut(index))
            .ok_or("paragraph out of bounds")?;
        paragraph.format.alignment = alignment;
        report.changed.push(paragraph.id);
    }
    if !report.changed.is_empty() {
        report.flows.insert(flow_id);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// One character attribute the Format menu switches on and off.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterAttribute {
    Bold,
    Italic,
    Underline,
    WordUnderline,
    DoubleUnderline,
    Strikethrough,
    Superscript,
    Subscript,
    /// One of the caps; `Caps::Normal` is not a menu attribute.
    Caps(Caps),
    /// Edit › Mark Text's Protected Text and No Hyphenation.
    Protected,
    NoHyphenation,
    /// Special Effects' Overstrike with this character: on when any
    /// overstrike shows; off removes it whatever its character.
    Overstrike(char),
}

impl CharacterAttribute {
    /// Whether text with `format` in `style` shows the attribute.
    pub fn is_on(self, format: &CharacterFormat, style: &ParagraphStyle) -> bool {
        let attributes = format.attributes(style);
        match self {
            Self::Bold => format.bold.unwrap_or(style.bold),
            Self::Italic => format.italic.unwrap_or(style.italic),
            Self::Underline => attributes.underline,
            Self::WordUnderline => attributes.word_underline,
            Self::DoubleUnderline => attributes.double_underline,
            Self::Strikethrough => attributes.strikethrough,
            Self::Superscript => attributes.superscript,
            Self::Subscript => attributes.subscript,
            Self::Caps(caps) => attributes.caps == Some(caps),
            Self::Protected => attributes.protected,
            Self::NoHyphenation => attributes.no_hyphenation,
            Self::Overstrike(_) => attributes.overstrike.is_some(),
        }
    }

    /// Whether choosing `other` replaces a choice of this attribute: the
    /// same attribute, or any two overstrikes.
    pub fn replaces(self, other: Self) -> bool {
        self == other || matches!((self, other), (Self::Overstrike(_), Self::Overstrike(_)))
    }

    /// Switches the attribute on or off in `format` over `style`: on sets
    /// it; off sets it off where the style has it and otherwise clears it.
    pub fn set(self, format: &mut CharacterFormat, style: &ParagraphStyle, on: bool) {
        let base = style.attributes;
        let (field, styled) = match self {
            Self::Bold => (&mut format.bold, style.bold),
            Self::Italic => (&mut format.italic, style.italic),
            Self::Underline => (&mut format.underline, base.underline),
            Self::WordUnderline => (&mut format.word_underline, base.word_underline),
            Self::DoubleUnderline => (&mut format.double_underline, base.double_underline),
            Self::Strikethrough => (&mut format.strikethrough, base.strikethrough),
            Self::Superscript => (&mut format.superscript, base.superscript),
            Self::Subscript => (&mut format.subscript, base.subscript),
            Self::Protected => (&mut format.protected, base.protected),
            Self::NoHyphenation => (&mut format.no_hyphenation, base.no_hyphenation),
            Self::Overstrike(character) => {
                format.overstrike = on.then_some(character);
                return;
            }
            Self::Caps(caps) => {
                format.caps = if on {
                    Some(caps)
                } else if base.caps == Some(caps) {
                    Some(Caps::Normal)
                } else {
                    None
                };
                return;
            }
        };
        *field = if on {
            Some(true)
        } else if styled {
            Some(false)
        } else {
            None
        };
    }
}

/// Changes the direct character formatting between two positions of one
/// flow: `change` sees each stretch's format with its paragraph's style.
fn modify_characters(
    document: &Document,
    start: TextPosition,
    end: TextPosition,
    change: impl Fn(&ParagraphStyle, &mut CharacterFormat),
) -> Result<Applied, EditError> {
    if start.flow != end.flow {
        return Err("a text range must stay in one flow");
    }
    let (start, end) = ordered(start, end);
    if start == end {
        return Err("character format range is empty");
    }
    let flow_id = start.flow;
    let flow = document.flow(flow_id).ok_or("flow does not exist")?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let mut ranges = 0;
    for index in start.paragraph..=end.paragraph {
        let source = flow.paragraph(index).ok_or("paragraph out of bounds")?;
        let range_start = if index == start.paragraph {
            start.byte
        } else {
            0
        };
        let range_end = if index == end.paragraph {
            end.byte
        } else {
            source.text.len()
        };
        // A paragraph the range covers whole has its end format changed
        // with its text, so that an empty one counts too.
        let whole = range_start == 0 && range_end == source.text.len();
        if range_start == range_end && !whole {
            continue;
        }
        ranges += 1;
        let style = document
            .styles()
            .get(&source.style)
            .ok_or("missing style")?;
        let mut formatted = source.clone();
        if range_start < range_end {
            formatted
                .modify_character_format(range_start, range_end, |format| change(style, format))
                .map_err(|_| "invalid character format range")?;
        }
        if whole {
            change(style, &mut formatted.end_format);
            if !formatted.end_format.has_valid_font() {
                return Err("character format change has invalid font properties");
            }
        }
        if formatted.runs != source.runs || formatted.end_format != source.end_format {
            report.changed.push(source.id);
            let target = next
                .flow_mut(flow_id)
                .and_then(|flow| flow.paragraph_mut(index))
                .ok_or("paragraph out of bounds")?;
            target.runs = formatted.runs;
            target.end_format = formatted.end_format;
        }
    }
    if ranges == 0 {
        return Err("character format range contains no text");
    }
    if !report.changed.is_empty() {
        report.flows.insert(flow_id);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

fn clear_direct_formatting(
    document: &Document,
    start: TextPosition,
    end: TextPosition,
) -> Result<Applied, EditError> {
    if start.flow != end.flow {
        return Err("a text range must stay in one flow");
    }
    let (start, end) = ordered(start, end);
    let flow_id = start.flow;
    let flow = document.flow(flow_id).ok_or("flow does not exist")?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for index in start.paragraph..=end.paragraph {
        let source = flow.paragraph(index).ok_or("paragraph out of bounds")?;
        let range_start = if index == start.paragraph {
            start.byte
        } else {
            0
        };
        let range_end = if index == end.paragraph {
            end.byte
        } else {
            source.text.len()
        };
        let mut cleared = source.clone();
        // Normal keeps the paragraph's own ruler (`12e0:132b`, `12e8:01da`).
        cleared.format = ParagraphFormat {
            tabs: source.format.tabs.clone(),
            ..ParagraphFormat::default()
        };
        if range_start < range_end {
            cleared
                .modify_character_format(range_start, range_end, |format| {
                    *format = CharacterFormat::default();
                })
                .map_err(|_| "invalid character format range")?;
        }
        // The end format of a paragraph covered whole, an empty one included.
        if range_start == 0 && range_end == source.text.len() {
            cleared.end_format = CharacterFormat::default();
        }
        if cleared.runs != source.runs
            || cleared.format != source.format
            || cleared.end_format != source.end_format
        {
            report.changed.push(source.id);
            let target = next
                .flow_mut(flow_id)
                .and_then(|flow| flow.paragraph_mut(index))
                .ok_or("paragraph out of bounds")?;
            target.runs = cleared.runs;
            target.format = cleared.format;
            target.end_format = cleared.end_format;
        }
    }
    if !report.changed.is_empty() {
        report.flows.insert(flow_id);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// A document root with the selection that went with it.
#[derive(Clone, Debug)]
pub struct Snapshot<S> {
    pub document: Document,
    pub selection: S,
}

/// What [`History::execute`] reports about an applied command.
#[derive(Clone, Debug)]
pub struct Executed {
    pub caret: Option<TextPosition>,
    pub report: ChangeReport,
}

/// Undo and redo over document roots, each with its selection.
///
/// Roots share unchanged parts, so an entry costs only what its edit touched.
/// The history also keeps the highest ID counter it has seen and raises the
/// current root to it before an edit, so IDs from undone edits are never
/// handed out again.
#[derive(Clone, Debug)]
pub struct History<S> {
    undo: VecDeque<Snapshot<S>>,
    redo: Vec<Snapshot<S>>,
    limit: usize,
    id_floor: u64,
    /// Counts every change to the history: a step made, amended, undone,
    /// redone, or the history cleared (see [`History::changes`]).
    changes: u64,
}

impl<S> Default for History<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> History<S> {
    /// The number of undo steps kept, as the app has always kept.
    pub const DEFAULT_LIMIT: usize = 100;

    pub fn new() -> Self {
        Self::with_limit(Self::DEFAULT_LIMIT)
    }

    pub fn with_limit(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            limit: limit.max(1),
            id_floor: 0,
            changes: 0,
        }
    }

    /// The number of undo steps kept.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Keeps `limit` undo steps from now on (at least one); with fewer than
    /// it holds, the oldest go (Preferences' undo levels, CM6).
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        if self.undo.len() > self.limit {
            while self.undo.len() > self.limit {
                self.undo.pop_front();
            }
            self.changes += 1;
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.changes += 1;
    }

    /// A count that grows with every change to the history (a step made,
    /// amended, undone, or redone, or the history cleared), unlike
    /// [`History::undo_len`], which stays put at the limit: a caller that
    /// joins later edits to a step (a run of typing) checks that nothing
    /// came between.
    pub fn changes(&self) -> u64 {
        self.changes
    }

    /// Records `current` and `selection` as the state before an edit that the
    /// caller makes directly, then readies `current` for it.
    pub fn record(&mut self, current: &mut Document, selection: S) {
        self.observe(current);
        self.push_undo(Snapshot {
            document: current.clone(),
            selection,
        });
        self.redo.clear();
        current.raise_id_counter(self.id_floor);
    }

    /// Applies `command` to `current`. When it changes anything, the previous
    /// root and `selection` become the next undo step and `current` becomes
    /// the new root. A command that changes nothing is not recorded.
    pub fn execute(
        &mut self,
        current: &mut Document,
        selection: S,
        command: &Command,
    ) -> Result<Executed, EditError> {
        self.observe(current);
        let mut base = current.clone();
        base.raise_id_counter(self.id_floor);
        let applied = apply(&base, command)?;
        if !applied.report.is_empty() {
            let previous = std::mem::replace(current, applied.document);
            self.push_undo(Snapshot {
                document: previous,
                selection,
            });
            self.redo.clear();
        }
        Ok(Executed {
            caret: applied.caret,
            report: applied.report,
        })
    }

    /// Applies `command` to `current` as part of the last undo step, so that
    /// undo restores the root from before that step: for a gesture or a run
    /// of key presses that issues several commands. Without an undo step it
    /// records one with `selection`, like [`History::execute`].
    pub fn amend(
        &mut self,
        current: &mut Document,
        selection: S,
        command: &Command,
    ) -> Result<Executed, EditError> {
        if self.undo.is_empty() {
            return self.execute(current, selection, command);
        }
        self.observe(current);
        let mut base = current.clone();
        base.raise_id_counter(self.id_floor);
        let applied = apply(&base, command)?;
        if !applied.report.is_empty() {
            *current = applied.document;
            self.redo.clear();
            self.changes += 1;
        }
        Ok(Executed {
            caret: applied.caret,
            report: applied.report,
        })
    }

    /// Restores the previous root into `current` and returns its selection.
    /// `current` and `selection` become the next redo step.
    pub fn undo(&mut self, current: &mut Document, selection: S) -> Option<S> {
        let snapshot = self.undo.pop_back()?;
        self.observe(current);
        self.changes += 1;
        self.redo.push(Snapshot {
            document: std::mem::replace(current, snapshot.document),
            selection,
        });
        Some(snapshot.selection)
    }

    /// Restores the next root into `current` and returns its selection.
    pub fn redo(&mut self, current: &mut Document, selection: S) -> Option<S> {
        let snapshot = self.redo.pop()?;
        self.observe(current);
        self.push_undo(Snapshot {
            document: std::mem::replace(current, snapshot.document),
            selection,
        });
        Some(snapshot.selection)
    }

    fn observe(&mut self, document: &Document) {
        self.id_floor = self.id_floor.max(document.next_id());
    }

    fn push_undo(&mut self, snapshot: Snapshot<S>) {
        self.changes += 1;
        self.undo.push_back(snapshot);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }
}

/// A main-flow editor: a document, a caret, and a history, operated through
/// commands. Tests and the rich-editor prototype's action mapping use it.
#[derive(Clone, Debug)]
pub struct Editor {
    document: Document,
    cursor: Cursor,
    history: History<Cursor>,
}

impl Editor {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            cursor: Cursor {
                paragraph: 0,
                byte: 0,
            },
            history: History::new(),
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    fn at(&self, cursor: Cursor) -> TextPosition {
        cursor.in_flow(self.document.main_flow_id())
    }

    pub fn set_cursor(&mut self, cursor: Cursor) -> Result<(), EditError> {
        let text = self
            .document
            .paragraphs()
            .get(cursor.paragraph)
            .ok_or("paragraph out of bounds")?
            .text
            .as_str();
        if cursor.byte > text.len() || !text.is_char_boundary(cursor.byte) {
            return Err("cursor is not a UTF-8 boundary");
        }
        self.cursor = cursor;
        Ok(())
    }

    /// Executes any command through the history. A text command moves the
    /// caret to the position it reports.
    pub fn execute(&mut self, command: &Command) -> Result<ChangeReport, EditError> {
        let executed = self
            .history
            .execute(&mut self.document, self.cursor, command)?;
        if let Some(caret) = executed
            .caret
            .filter(|caret| caret.flow == self.document.main_flow_id())
        {
            self.cursor = caret.into();
        }
        Ok(executed.report)
    }

    pub fn insert(&mut self, text: impl Into<String>) -> Result<(), EditError> {
        let text = text.into();
        if text.contains(['\r', '\n']) {
            return Err("paragraph insertion cannot contain a line break");
        }
        self.execute(&Command::insert(self.at(self.cursor), text))
            .map(drop)
    }

    /// Deletes the character before the cursor, or joins the paragraph to its
    /// predecessor at its start. Returns whether anything changed.
    pub fn delete_backward(&mut self) -> Result<bool, EditError> {
        let report = self.execute(&Command::DeleteCharacter {
            at: self.at(self.cursor),
            forward: false,
        })?;
        Ok(!report.is_empty())
    }

    /// Splits the current paragraph at the cursor.
    pub fn split_paragraph(&mut self) -> Result<(), EditError> {
        self.replace_document_range(self.cursor, self.cursor, "\n")
    }

    /// Joins the current paragraph to its predecessor when at its start.
    pub fn join_with_previous(&mut self) -> Result<bool, EditError> {
        if self.cursor.byte != 0 {
            return Ok(false);
        }
        let Some(command) = Command::join_with_previous(
            &self.document,
            self.document.main_flow_id(),
            self.cursor.paragraph,
        ) else {
            return Ok(false);
        };
        self.execute(&command)?;
        Ok(true)
    }

    /// Replaces a UTF-8-safe range spanning one or more paragraphs atomically.
    /// Newlines in `replacement` create paragraphs which inherit the starting
    /// paragraph style; unaffected character runs retain exact byte coverage.
    pub fn replace_document_range(
        &mut self,
        start: Cursor,
        end: Cursor,
        replacement: impl Into<String>,
    ) -> Result<(), EditError> {
        self.execute(&Command::ReplaceText {
            start: self.at(start),
            end: self.at(end),
            text: replacement.into(),
        })
        .map(drop)
    }

    /// Replaces a UTF-8-safe range in one paragraph as one undoable command.
    pub fn replace_range(
        &mut self,
        start: Cursor,
        end: Cursor,
        replacement: impl Into<String>,
    ) -> Result<(), EditError> {
        if start.paragraph != end.paragraph {
            return Err("replacement cannot cross paragraphs");
        }
        let replacement = replacement.into();
        if replacement.contains(['\r', '\n']) {
            return Err("paragraph replacement cannot contain a line break");
        }
        self.replace_document_range(start, end, replacement)
    }

    pub fn apply_style(&mut self, style: StyleId) -> Result<(), EditError> {
        let paragraph = self.cursor.paragraph;
        self.execute(&Command::SetParagraphStyle {
            flow: self.document.main_flow_id(),
            first: paragraph,
            last: paragraph,
            style,
        })
        .map(drop)
    }

    pub fn apply_character_format(
        &mut self,
        paragraph: usize,
        start: usize,
        end: usize,
        format: CharacterFormat,
    ) -> Result<(), EditError> {
        self.apply_character_format_range(
            Cursor {
                paragraph,
                byte: start,
            },
            Cursor {
                paragraph,
                byte: end,
            },
            format,
        )
    }

    pub fn apply_character_format_range(
        &mut self,
        start: Cursor,
        end: Cursor,
        format: CharacterFormat,
    ) -> Result<(), EditError> {
        self.execute(&Command::FormatCharacters {
            start: self.at(start),
            end: self.at(end),
            format,
        })
        .map(drop)
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        Ok(match self.history.undo(&mut self.document, self.cursor) {
            Some(cursor) => {
                self.cursor = cursor;
                true
            }
            None => false,
        })
    }

    pub fn redo(&mut self) -> Result<bool, EditError> {
        Ok(match self.history.redo(&mut self.document, self.cursor) {
            Some(cursor) => {
                self.cursor = cursor;
                true
            }
            None => false,
        })
    }
}

/// Runs after inserting `length` bytes at `at`. Text typed at either end
/// of a run joins it, as in Ami Pro (VM fixture C1: typing before and after
/// a bold word types bold); where one run ends and another starts, the
/// earlier one takes the text.
/// The format text typed at `byte` takes, as [`runs_after_insert`] gives
/// it (VM fixture C1): the run that ends there, else the run it starts or
/// falls in, else none; in an empty paragraph, its end format.
pub(crate) fn caret_format(paragraph: &Paragraph, byte: usize) -> &CharacterFormat {
    if paragraph.text.is_empty() {
        return &paragraph.end_format;
    }
    paragraph
        .runs
        .iter()
        .find(|run| run.start < byte && run.end == byte)
        .or_else(|| {
            paragraph
                .runs
                .iter()
                .find(|run| run.start <= byte && byte < run.end)
        })
        .map_or(&CharacterFormat::PLAIN, |run| &run.format)
}

fn runs_after_insert(runs: &[CharacterRun], at: usize, length: usize) -> Vec<CharacterRun> {
    let ending = runs.iter().position(|run| run.start < at && run.end == at);
    let ends_here = ending.is_some();
    // Text typed at either edge of protected text stays outside it (VM
    // fixture PT1): after it, with the rest of its format; before it, with
    // the format before it, which no run gives.
    let outside = ending
        .or_else(|| runs.iter().position(|run| run.start == at))
        .filter(|&index| runs[index].format.protected == Some(true));
    let mut result = runs.to_vec();
    for (index, run) in result.iter_mut().enumerate() {
        if at < run.start || (at == run.start && ends_here) {
            run.start += length;
            run.end += length;
        } else if Some(index) == outside {
            if run.start == at {
                run.start += length;
                run.end += length;
            }
        } else if at <= run.end {
            run.end += length;
        }
    }
    if let Some(index) = outside.filter(|_| ends_here) {
        let mut format = runs[index].format.clone();
        format.protected = None;
        if !format.is_empty() {
            result.push(CharacterRun {
                start: at,
                end: at + length,
                format,
            });
            result.sort_by_key(|run| run.start);
        }
    }
    result
}

fn runs_after_delete(runs: &[CharacterRun], start: usize, end: usize) -> Vec<CharacterRun> {
    let length = end - start;
    runs.iter()
        .filter_map(|run| {
            if run.end <= start {
                Some(run.clone())
            } else if run.start >= end {
                Some(CharacterRun {
                    start: run.start - length,
                    end: run.end - length,
                    format: run.format.clone(),
                })
            } else {
                let kept_before = start.saturating_sub(run.start);
                let kept_after = run.end.saturating_sub(end);
                let kept = kept_before + kept_after;
                (kept > 0).then(|| CharacterRun {
                    start: run.start.min(start),
                    end: run.start.min(start) + kept,
                    format: run.format.clone(),
                })
            }
        })
        .collect()
}

fn runs_for_slice(
    runs: &[CharacterRun],
    start: usize,
    end: usize,
    destination_start: usize,
) -> Vec<CharacterRun> {
    runs.iter()
        .filter_map(|run| {
            let clipped_start = run.start.max(start);
            let clipped_end = run.end.min(end);
            (clipped_start < clipped_end).then(|| CharacterRun {
                start: destination_start + clipped_start - start,
                end: destination_start + clipped_end - start,
                format: run.format.clone(),
            })
        })
        .collect()
}

fn merge_adjacent_runs(runs: &mut Vec<CharacterRun>) {
    let mut merged: Vec<CharacterRun> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if let Some(previous) = merged
            .last_mut()
            .filter(|previous| previous.end == run.start && previous.format == run.format)
        {
            previous.end = run.end;
        } else {
            merged.push(run);
        }
    }
    *runs = merged;
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{CharacterFormat, CharacterRun};

    fn bold() -> CharacterFormat {
        CharacterFormat {
            bold: Some(true),
            ..CharacterFormat::default()
        }
    }

    fn main_at(document: &Document, paragraph: usize, byte: usize) -> TextPosition {
        TextPosition {
            flow: document.main_flow_id(),
            paragraph,
            byte,
        }
    }

    #[test]
    fn unicode_edit_undo_redo_is_exact() {
        let mut editor = Editor::new(Document::default());
        editor.insert("A🙂").unwrap();
        assert!(editor.delete_backward().unwrap());
        assert_eq!(editor.document().paragraphs()[0].text, "A");
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document().paragraphs()[0].text, "A🙂");
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document().paragraphs()[0].text, "");
        assert!(editor.redo().unwrap());
        assert_eq!(editor.document().paragraphs()[0].text, "A🙂");
    }

    #[test]
    fn selection_replacement_is_atomic_utf8_safe_and_preserves_outer_formatting() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "bold 🙂 tail".into();
        document.paragraph_mut(0).runs = vec![CharacterRun {
            start: 0,
            end: 4,
            format: bold(),
        }];
        let mut editor = Editor::new(document.clone());

        editor
            .replace_range(
                Cursor {
                    paragraph: 0,
                    byte: 5,
                },
                Cursor {
                    paragraph: 0,
                    byte: 9,
                },
                "café",
            )
            .unwrap();

        assert_eq!(editor.document().paragraphs()[0].text, "bold café tail");
        assert_eq!(
            editor.document().paragraphs()[0].runs,
            document.paragraphs()[0].runs
        );
        assert_eq!(editor.cursor().byte, 10);
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document(), &document);
        assert!(editor.redo().unwrap());
        assert_eq!(editor.document().paragraphs()[0].text, "bold café tail");
    }

    #[test]
    fn replacement_rejects_invalid_boundaries_and_cross_paragraph_ranges() {
        let mut editor = Editor::new(Document::with_paragraphs([
            ("🙂", StyleId(1)),
            ("second", StyleId(1)),
        ]));

        assert_eq!(
            editor.replace_range(
                Cursor {
                    paragraph: 0,
                    byte: 1,
                },
                Cursor {
                    paragraph: 0,
                    byte: 4,
                },
                "x",
            ),
            Err("replacement is not a UTF-8-safe range")
        );
        assert_eq!(
            editor.replace_range(
                Cursor {
                    paragraph: 0,
                    byte: 0,
                },
                Cursor {
                    paragraph: 1,
                    byte: 0,
                },
                "x",
            ),
            Err("replacement cannot cross paragraphs")
        );
        assert!(!editor.can_undo(), "rejected commands are not recorded");
    }

    #[test]
    fn paragraph_style_change_is_reversible() {
        let mut editor = Editor::new(Document::default());
        editor.apply_style(StyleId(2)).unwrap();
        assert_eq!(editor.document().paragraphs()[0].style, StyleId(2));
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document().paragraphs()[0].style, StyleId(1));
        assert!(editor.redo().unwrap());
        assert_eq!(editor.document().paragraphs()[0].style, StyleId(2));
        assert_eq!(editor.apply_style(StyleId(99)), Err("style does not exist"));
    }

    #[test]
    fn text_edits_preserve_and_restore_character_runs() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "one bold end".into();
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 4,
            end: 8,
            format: bold(),
        });
        let original = document.clone();
        let mut editor = Editor::new(document);

        editor
            .set_cursor(Cursor {
                paragraph: 0,
                byte: 6,
            })
            .unwrap();
        editor.insert("🙂").unwrap();
        assert_eq!(
            (
                editor.document().paragraphs()[0].runs[0].start,
                editor.document().paragraphs()[0].runs[0].end
            ),
            (4, 12)
        );
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document(), &original);
        assert!(editor.redo().unwrap());
        editor.delete_backward().unwrap();
        assert_eq!(editor.document().plain_text(), original.plain_text());
        assert_eq!(
            editor.document().paragraphs()[0].runs,
            original.paragraphs()[0].runs
        );
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document().paragraphs()[0].text, "one bo🙂ld end");
        assert!(editor.document().validate().is_ok());
    }

    /// The runs of the main flow's paragraph `index` as (text, format).
    fn runs_of(document: &Document, index: usize) -> Vec<(String, CharacterFormat)> {
        let paragraph = &document.paragraphs()[index];
        paragraph
            .runs
            .iter()
            .map(|run| {
                (
                    paragraph.text[run.start..run.end].to_owned(),
                    run.format.clone(),
                )
            })
            .collect()
    }

    #[test]
    fn text_typed_at_the_edges_of_protected_text_stays_outside_it_pt1() {
        // VM fixture PT1: "X" typed after a protected word is not protected
        // but keeps the word's other formatting; "Y" typed before it is
        // plain; text typed inside it is protected.
        let protected = CharacterFormat {
            protected: Some(true),
            bold: Some(true),
            ..CharacterFormat::default()
        };
        let mut document = Document::with_paragraphs([("ab cd ef", StyleId(1))]);
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 3,
            end: 5,
            format: protected.clone(),
        });
        let typed = |document: &Document, byte: usize, text: &str| {
            let at = main_at(document, 0, byte);
            apply(
                document,
                &Command::ReplaceText {
                    start: at,
                    end: at,
                    text: text.into(),
                },
            )
            .unwrap()
            .document
        };
        let document = typed(&document, 5, "X");
        let document = typed(&document, 3, "Y");
        let document = typed(&document, 5, "Z");
        assert_eq!(document.paragraphs()[0].text, "ab YcZdX ef");
        assert_eq!(
            runs_of(&document, 0),
            [("cZd".to_owned(), protected), ("X".to_owned(), bold()),]
        );
        // The attributes switch on and off; any overstrike replaces another.
        let style = &document.styles()[&StyleId(1)];
        let mut format = CharacterFormat::default();
        CharacterAttribute::Protected.set(&mut format, style, true);
        CharacterAttribute::Overstrike('/').set(&mut format, style, true);
        assert!(CharacterAttribute::Protected.is_on(&format, style));
        assert!(CharacterAttribute::Overstrike('x').is_on(&format, style));
        assert_eq!(format.overstrike, Some('/'));
        CharacterAttribute::Overstrike('x').set(&mut format, style, false);
        CharacterAttribute::Protected.set(&mut format, style, false);
        assert!(format.is_empty());
        assert!(CharacterAttribute::Overstrike('/').replaces(CharacterAttribute::Overstrike('=')));
        assert!(!CharacterAttribute::Protected.replaces(CharacterAttribute::NoHyphenation));
    }

    #[test]
    fn typing_at_either_end_of_a_run_joins_it_c1() {
        // VM fixture C1: "QQ" typed before a bold word and "WW" after it
        // are bold; where two runs meet, the earlier one takes the text.
        let mut document = Document::with_paragraphs([("plain bold plain", StyleId(1))]);
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 6,
            end: 10,
            format: bold(),
        });
        let typed = |document: &Document, byte: usize, text: &str| {
            let at = main_at(document, 0, byte);
            apply(
                document,
                &Command::ReplaceText {
                    start: at,
                    end: at,
                    text: text.into(),
                },
            )
            .unwrap()
            .document
        };
        let document = typed(&document, 6, "QQ");
        let document = typed(&document, 12, "WW");
        assert_eq!(runs_of(&document, 0), [("QQboldWW".to_owned(), bold())]);
        let document = typed(&document, 5, "x");
        assert_eq!(document.paragraphs()[0].text, "plainx QQboldWW plain");
        assert_eq!(runs_of(&document, 0)[0].0, "QQboldWW");
        let italic = CharacterFormat {
            italic: Some(true),
            ..CharacterFormat::default()
        };
        let mut meeting = Document::with_paragraphs([("abcd", StyleId(1))]);
        meeting.paragraph_mut(0).runs = vec![
            CharacterRun {
                start: 0,
                end: 2,
                format: bold(),
            },
            CharacterRun {
                start: 2,
                end: 4,
                format: italic.clone(),
            },
        ];
        let meeting = typed(&meeting, 2, "X");
        assert_eq!(
            runs_of(&meeting, 0),
            [("abX".to_owned(), bold()), ("cd".to_owned(), italic)]
        );
    }

    #[test]
    fn end_formats_follow_typing_splits_joins_and_formatting() {
        let small = CharacterFormat {
            font_size_points: Some(10.0),
            ..CharacterFormat::default()
        };
        let mut document = Document::with_paragraphs([("", StyleId(1)), ("next", StyleId(1))]);
        document.paragraph_mut(0).end_format = small.clone();
        let run = |document: &Document, command: Command| apply(document, &command).unwrap();
        let replace = |document: &Document, start: TextPosition, end: TextPosition, text: &str| {
            run(
                document,
                Command::ReplaceText {
                    start,
                    end,
                    text: text.into(),
                },
            )
            .document
        };

        // Text typed into an empty paragraph lands after its format changes.
        let at = main_at(&document, 0, 0);
        let typed = replace(&document, at, at, "ab");
        assert_eq!(runs_of(&typed, 0), [("ab".to_owned(), small.clone())]);
        assert_eq!(typed.paragraphs()[0].end_format, small);
        // Typing into a paragraph with text keeps the C1 rule.
        let end = main_at(&typed, 1, 4);
        let typed_next = replace(&typed, end, end, "!");
        assert!(typed_next.paragraphs()[1].runs.is_empty());

        // VM fixture EN1: a paragraph end that Enter inserts takes the
        // format typing at the caret would take.
        let bold_text = |text: &str, start: usize, end: usize| {
            let mut document = Document::with_paragraphs([(text, StyleId(1))]);
            document.paragraph_mut(0).runs.push(CharacterRun {
                start,
                end,
                format: bold(),
            });
            document
        };
        let enter = |document: &Document, byte: usize| {
            let at = main_at(document, 0, byte);
            replace(document, at, at, "\n")
        };
        // "aa <+!>bold", Enter, "x": x is bold (Ami Pro wrote `<+!>x`).
        let after_bold = enter(&bold_text("aa bold", 3, 7), 7);
        assert_eq!(after_bold.paragraphs()[1].end_format, bold());
        let at = main_at(&after_bold, 1, 0);
        let x = replace(&after_bold, at, at, "x");
        assert_eq!(runs_of(&x, 1), [("x".to_owned(), bold())]);
        // Enter after plain text: plain.
        let after_plain = enter(&bold_text("ee bold", 0, 2), 7);
        assert!(after_plain.paragraphs()[1].end_format.is_empty());
        // Enter before a bold word: the empty paragraph above is bold (Ami
        // Pro wrote `<+!>`), and the word keeps its own end.
        let before_bold = enter(&bold_text("zz", 0, 2), 0);
        assert_eq!(before_bold.paragraphs()[0].end_format, bold());
        assert_eq!(runs_of(&before_bold, 1), [("zz".to_owned(), bold())]);
        assert!(before_bold.paragraphs()[1].end_format.is_empty());

        // In an empty paragraph the caret's format is its end format.
        let at_end = main_at(&typed, 0, 2);
        let split = replace(&typed, at_end, at_end, "\n");
        assert_eq!(split.paragraphs()[1].text, "");
        assert_eq!(split.paragraphs()[0].end_format, small);
        assert_eq!(split.paragraphs()[1].end_format, small);
        let at_start = main_at(&typed, 0, 0);
        let split = replace(&typed, at_start, at_start, "\n");
        assert_eq!(split.paragraphs()[0].text, "");
        assert_eq!(split.paragraphs()[0].end_format, small);
        assert_eq!(runs_of(&split, 1), [("ab".to_owned(), small.clone())]);

        // A join keeps the later paragraph's end, and undo restores it.
        let joined = run(
            &typed,
            Command::DeleteCharacter {
                at: main_at(&typed, 1, 0),
                forward: false,
            },
        );
        assert_eq!(joined.document.paragraphs()[0].text, "abnext");
        assert!(joined.document.paragraphs()[0].end_format.is_empty());

        // Formatting a range that covers an empty paragraph whole changes
        // its end format; one that does not leaves it.
        let both = run(
            &document,
            Command::SetCharacterAttribute {
                start: main_at(&document, 0, 0),
                end: main_at(&document, 1, 2),
                attribute: CharacterAttribute::Bold,
                on: true,
            },
        )
        .document;
        assert_eq!(both.paragraphs()[0].end_format.bold, Some(true));
        assert_eq!(both.paragraphs()[0].end_format.font_size_points, Some(10.0));
        assert!(both.paragraphs()[1].end_format.is_empty());
        let whole = run(
            &document,
            Command::SetFont {
                start: main_at(&document, 1, 0),
                end: main_at(&document, 1, 4),
                family: Some("Arial".into()),
                size: None,
                colour: None,
            },
        )
        .document;
        assert_eq!(
            whole.paragraphs()[1].end_format.font_family.as_deref(),
            Some("Arial")
        );
        let only_empty = run(
            &document,
            Command::SetCharacterAttribute {
                start: main_at(&document, 0, 0),
                end: main_at(&document, 1, 0),
                attribute: CharacterAttribute::Italic,
                on: true,
            },
        );
        assert_eq!(
            only_empty.document.paragraphs()[0].end_format.italic,
            Some(true)
        );

        // Normal at a caret in an empty paragraph clears its end format.
        let at = main_at(&document, 0, 0);
        let cleared = run(
            &document,
            Command::ClearDirectFormatting { start: at, end: at },
        );
        assert!(cleared.document.paragraphs()[0].end_format.is_empty());
        let mut history: History<()> = History::new();
        let mut current = document.clone();
        history
            .execute(
                &mut current,
                (),
                &Command::ClearDirectFormatting { start: at, end: at },
            )
            .unwrap();
        history.undo(&mut current, ());
        assert_eq!(current.paragraphs()[0].end_format, small);
    }

    #[test]
    fn enter_at_the_end_gives_the_next_style_nx1() {
        // VM fixture NX1: a heading with Next style on and Body Text next.
        let mut document = Document::with_paragraphs([("Heading", StyleId(2))]);
        let heading = document.styles_mut().get_mut(&StyleId(2)).unwrap();
        heading.breaks = amipro_document::Breaks(
            amipro_document::Breaks::WITHIN | amipro_document::Breaks::NEXT_STYLE,
        );
        heading.next = Some(StyleId(1));
        let enter = |document: &Document, byte: usize| {
            let at = main_at(document, 0, byte);
            apply(
                document,
                &Command::ReplaceText {
                    start: at,
                    end: at,
                    text: "\n".into(),
                },
            )
            .unwrap()
            .document
        };
        let styles = |document: &Document| {
            document
                .paragraphs()
                .iter()
                .map(|paragraph| paragraph.style)
                .collect::<Vec<_>>()
        };
        // At the end: Body Text; in the middle and at the start: Heading.
        assert_eq!(styles(&enter(&document, 7)), [StyleId(2), StyleId(1)]);
        assert_eq!(styles(&enter(&document, 3)), [StyleId(2), StyleId(2)]);
        assert_eq!(styles(&enter(&document, 0)), [StyleId(2), StyleId(2)]);
        // Next style on with no next style named keeps the style.
        document.styles_mut().get_mut(&StyleId(2)).unwrap().next = None;
        assert_eq!(styles(&enter(&document, 7)), [StyleId(2), StyleId(2)]);
        // Off: the style continues.
        let heading = document.styles_mut().get_mut(&StyleId(2)).unwrap();
        heading.next = Some(StyleId(1));
        heading.breaks = amipro_document::Breaks::default();
        assert_eq!(styles(&enter(&document, 7)), [StyleId(2), StyleId(2)]);
    }

    #[test]
    fn attributes_switch_on_and_off_over_their_styles() {
        let mut styles = amipro_document::default_styles();
        let heading = styles.get_mut(&StyleId(2)).unwrap();
        heading.attributes.caps = Some(Caps::Upper);
        heading.attributes.strikethrough = true;
        let mut document = Document::empty("Attributes", styles);
        document.push_paragraph("body text", StyleId(1));
        document.push_paragraph("heading text", StyleId(2));
        let set = |document: &Document, paragraph: usize, attribute, on| {
            let start = main_at(document, paragraph, 0);
            let end = main_at(document, paragraph, 4);
            apply(
                document,
                &Command::SetCharacterAttribute {
                    start,
                    end,
                    attribute,
                    on,
                },
            )
            .unwrap()
            .document
        };
        // On sets it; off over a style without it leaves no run.
        let on = set(&document, 0, CharacterAttribute::WordUnderline, true);
        let style = &on.styles()[&StyleId(1)];
        let run = &on.paragraphs()[0].runs[0];
        assert_eq!((run.start, run.end), (0, 4));
        assert!(CharacterAttribute::WordUnderline.is_on(&run.format, style));
        let off = set(&on, 0, CharacterAttribute::WordUnderline, false);
        assert!(off.paragraphs()[0].runs.is_empty());
        // Off over a style with it switches it off on the text.
        let heading = set(&document, 1, CharacterAttribute::Strikethrough, false);
        assert_eq!(
            heading.paragraphs()[1].runs[0].format.strikethrough,
            Some(false)
        );
        let lowered = set(&document, 1, CharacterAttribute::Caps(Caps::Upper), false);
        assert_eq!(
            lowered.paragraphs()[1].runs[0].format.caps,
            Some(Caps::Normal)
        );
        let raised = set(&lowered, 1, CharacterAttribute::Caps(Caps::Upper), true);
        assert_eq!(
            raised.paragraphs()[1].runs[0].format.caps,
            Some(Caps::Upper)
        );
        // Another caps over a caps style; off returns to the style's.
        let small = set(&document, 1, CharacterAttribute::Caps(Caps::Small), true);
        let style = &small.styles()[&StyleId(2)];
        let format = &small.paragraphs()[1].runs[0].format;
        assert!(CharacterAttribute::Caps(Caps::Small).is_on(format, style));
        assert!(!CharacterAttribute::Caps(Caps::Upper).is_on(format, style));
        let back = set(&small, 1, CharacterAttribute::Caps(Caps::Small), false);
        assert!(back.paragraphs()[1].runs.is_empty());
        // Bold over a bold style switches it off, as before.
        let mut bold_style = Document::with_paragraphs([("Heading", StyleId(2))]);
        bold_style = set(&bold_style, 0, CharacterAttribute::Bold, false);
        assert_eq!(bold_style.paragraphs()[0].runs[0].format.bold, Some(false));
    }

    #[test]
    fn fonts_set_all_three_and_revert_to_the_style() {
        let document = Document::with_paragraphs([("some text here", StyleId(1))]);
        let font = |document: &Document, family: Option<&str>, size, colour| {
            apply(
                document,
                &Command::SetFont {
                    start: main_at(document, 0, 5),
                    end: main_at(document, 0, 9),
                    family: family.map(str::to_owned),
                    size,
                    colour,
                },
            )
        };
        let set = font(&document, Some("Arial"), Some(18.0), Some(0x0000_00FF))
            .unwrap()
            .document;
        let run = &set.paragraphs()[0].runs[0];
        assert_eq!((run.start, run.end), (5, 9));
        assert_eq!(
            (
                run.format.font_family.as_deref(),
                run.format.font_size_points,
                run.format.colour
            ),
            (Some("Arial"), Some(18.0), Some(0x0000_00FF))
        );
        let reverted = font(&set, None, None, None).unwrap().document;
        assert!(reverted.paragraphs()[0].runs.is_empty());
        assert!(font(&document, Some(" "), None, None).is_err());
        assert!(font(&document, None, Some(0.0), None).is_err());
        assert!(font(&document, None, None, Some(0x0100_0000)).is_err());
    }

    #[test]
    fn normal_clears_character_and_paragraph_formatting_c1() {
        // VM fixture C1: Text › Normal over a bold word in a centred
        // paragraph left both the word and the paragraph as their style.
        let mut document = Document::with_paragraphs([
            ("first bold words", StyleId(1)),
            ("second line", StyleId(1)),
        ]);
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 6,
            end: 10,
            format: bold(),
        });
        document.paragraph_mut(0).format.alignment = Some(Alignment::Centre);
        document.paragraph_mut(1).format.alignment = Some(Alignment::Right);
        let cleared = apply(
            &document,
            &Command::ClearDirectFormatting {
                start: main_at(&document, 0, 8),
                end: main_at(&document, 0, 12),
            },
        )
        .unwrap()
        .document;
        assert_eq!(runs_of(&cleared, 0), [("bo".to_owned(), bold())]);
        assert_eq!(cleared.paragraphs()[0].format, ParagraphFormat::default());
        assert_eq!(
            cleared.paragraphs()[1].format.alignment,
            Some(Alignment::Right),
            "an untouched paragraph keeps its format"
        );
        let caret = main_at(&document, 1, 3);
        let caret_only = apply(
            &document,
            &Command::ClearDirectFormatting {
                start: caret,
                end: caret,
            },
        )
        .unwrap();
        assert_eq!(caret_only.report.changed, [document.paragraphs()[1].id]);
        assert_eq!(caret_only.document.paragraphs()[0].runs.len(), 1);
    }

    #[test]
    fn character_format_change_is_reversible() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "A🙂B".into();
        let mut editor = Editor::new(document);

        editor.apply_character_format(0, 1, 5, bold()).unwrap();
        assert_eq!(
            (
                editor.document().paragraphs()[0].runs[0].start,
                editor.document().paragraphs()[0].runs[0].end
            ),
            (1, 5)
        );
        assert!(editor.undo().unwrap());
        assert!(editor.document().paragraphs()[0].runs.is_empty());
        assert!(editor.redo().unwrap());
        assert_eq!(
            editor.document().paragraphs()[0].runs[0].format.bold,
            Some(true)
        );
    }

    #[test]
    fn cross_paragraph_format_is_one_reversible_command() {
        let document = Document::with_paragraphs([("first", StyleId(1)), ("second", StyleId(1))]);
        let original = document.clone();
        let mut editor = Editor::new(document);

        editor
            .apply_character_format_range(
                Cursor {
                    paragraph: 0,
                    byte: 2,
                },
                Cursor {
                    paragraph: 1,
                    byte: 3,
                },
                CharacterFormat {
                    italic: Some(true),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();

        assert_eq!(
            (
                editor.document().paragraphs()[0].runs[0].start,
                editor.document().paragraphs()[0].runs[0].end
            ),
            (2, 5)
        );
        assert_eq!(
            (
                editor.document().paragraphs()[1].runs[0].start,
                editor.document().paragraphs()[1].runs[0].end
            ),
            (0, 3)
        );
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document(), &original);
        assert!(editor.redo().unwrap());
        assert_eq!(
            editor.document().paragraphs()[1].runs[0].format.italic,
            Some(true)
        );
    }

    #[test]
    fn paragraph_split_preserves_runs_and_is_exactly_reversible() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "bold tail".into();
        document.paragraph_mut(0).style = StyleId(2);
        document.paragraph_mut(0).format.line_spacing =
            Some(amipro_document::LineSpacing::Custom(432));
        document.paragraph_mut(0).runs = vec![CharacterRun {
            start: 0,
            end: 9,
            format: bold(),
        }];
        let original = document.clone();
        let mut editor = Editor::new(document);
        editor
            .set_cursor(Cursor {
                paragraph: 0,
                byte: 5,
            })
            .unwrap();

        editor.split_paragraph().unwrap();

        let paragraphs = editor.document().paragraphs();
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].text, "bold ");
        assert_eq!(paragraphs[1].text, "tail");
        assert_eq!(paragraphs[1].style, StyleId(2));
        assert_eq!(
            paragraphs[1].format, paragraphs[0].format,
            "both halves keep the paragraph's own formatting, like its style"
        );
        assert_eq!(paragraphs[0].id, original.paragraphs()[0].id);
        assert_ne!(paragraphs[1].id, paragraphs[0].id);
        assert_eq!(
            (paragraphs[0].runs[0].start, paragraphs[0].runs[0].end),
            (0, 5)
        );
        assert_eq!(
            (paragraphs[1].runs[0].start, paragraphs[1].runs[0].end),
            (0, 4)
        );
        assert!(editor.document().validate().is_ok());
        assert_eq!(
            editor.cursor(),
            Cursor {
                paragraph: 1,
                byte: 0
            }
        );
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document(), &original);
        assert!(editor.redo().unwrap());
        assert_eq!(editor.document().paragraphs()[1].text, "tail");
    }

    #[test]
    fn backspace_at_paragraph_start_joins_and_preserves_both_run_ranges() {
        let mut document = Document::with_paragraphs([("one", StyleId(1)), ("two", StyleId(2))]);
        document.paragraph_mut(0).runs = vec![CharacterRun {
            start: 0,
            end: 3,
            format: bold(),
        }];
        document.paragraph_mut(1).runs = vec![CharacterRun {
            start: 0,
            end: 3,
            format: CharacterFormat {
                italic: Some(true),
                ..CharacterFormat::default()
            },
        }];
        let original = document.clone();
        let mut editor = Editor::new(document);
        editor
            .set_cursor(Cursor {
                paragraph: 1,
                byte: 0,
            })
            .unwrap();

        assert!(editor.delete_backward().unwrap());

        assert_eq!(editor.document().paragraphs().len(), 1);
        assert_eq!(editor.document().paragraphs()[0].text, "onetwo");
        assert_eq!(editor.document().paragraphs()[0].style, StyleId(1));
        assert_eq!(
            editor.document().paragraphs()[0]
                .runs
                .iter()
                .map(|run| (run.start, run.end))
                .collect::<Vec<_>>(),
            [(0, 3), (3, 6)]
        );
        assert_eq!(editor.cursor().byte, 3);
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document(), &original);
        assert!(editor.redo().unwrap());
        assert_eq!(editor.document().paragraphs()[0].text, "onetwo");
    }

    #[test]
    fn cross_paragraph_unicode_replacement_is_one_undo_step() {
        let document = Document::with_paragraphs([
            ("A🙂 first", StyleId(1)),
            ("second", StyleId(1)),
            ("third éZ", StyleId(1)),
        ]);
        let original = document.clone();
        let mut editor = Editor::new(document);
        editor
            .set_cursor(Cursor {
                paragraph: 2,
                byte: 2,
            })
            .unwrap();

        editor
            .replace_document_range(
                Cursor {
                    paragraph: 0,
                    byte: 1,
                },
                Cursor {
                    paragraph: 2,
                    byte: 8,
                },
                "new\ntext",
            )
            .unwrap();

        assert_eq!(editor.document().plain_text(), "Anew\ntextZ");
        assert_eq!(
            editor.cursor(),
            Cursor {
                paragraph: 1,
                byte: 4
            }
        );
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document(), &original);
        assert_eq!(
            editor.cursor(),
            Cursor {
                paragraph: 2,
                byte: 2
            },
            "undo restores the selection recorded with the root"
        );
        assert!(!editor.undo().unwrap());
        assert!(editor.redo().unwrap());
        assert_eq!(editor.document().plain_text(), "Anew\ntextZ");
    }

    #[test]
    fn commands_leave_their_input_unchanged_and_share_untouched_paragraphs() {
        let document = Document::with_paragraphs([
            ("one", StyleId(1)),
            ("two", StyleId(1)),
            ("three", StyleId(1)),
        ]);
        let snapshot = document.clone();

        let applied = apply(&document, &Command::insert(main_at(&document, 1, 3), "!")).unwrap();

        assert_eq!(document, snapshot);
        assert_eq!(applied.document.plain_text(), "one\ntwo!\nthree");
        for index in [0, 2] {
            assert!(Arc::ptr_eq(
                &document.paragraphs()[index],
                &applied.document.paragraphs()[index]
            ));
        }
        assert_eq!(applied.caret, Some(main_at(&document, 1, 4)));
        assert_eq!(
            applied.report,
            ChangeReport {
                flows: BTreeSet::from([document.main_flow_id()]),
                changed: vec![document.paragraphs()[1].id],
                ..ChangeReport::default()
            }
        );
    }

    #[test]
    fn replacement_reports_inserted_and_removed_paragraphs() {
        let document = Document::with_paragraphs([
            ("alpha", StyleId(1)),
            ("beta", StyleId(1)),
            ("gamma", StyleId(1)),
        ]);
        let ids: Vec<_> = document.paragraphs().iter().map(|p| p.id).collect();

        let joined = apply(
            &document,
            &Command::delete(main_at(&document, 0, 2), main_at(&document, 2, 2)),
        )
        .unwrap();
        assert_eq!(joined.document.plain_text(), "almma");
        assert_eq!(joined.report.changed, [ids[0]]);
        assert_eq!(joined.report.removed, [ids[1], ids[2]]);
        assert!(joined.report.inserted.is_empty());

        let split = apply(
            &document,
            &Command::insert(main_at(&document, 1, 2), "x\ny\nz"),
        )
        .unwrap();
        assert_eq!(split.document.plain_text(), "alpha\nbex\ny\nzta\ngamma");
        assert_eq!(split.report.inserted.len(), 2);
        assert!(split.report.inserted.iter().all(|id| !ids.contains(id)));
        assert!(split.document.validate().is_ok());
    }

    const OBJ: char = OBJECT_REPLACEMENT_CHARACTER;

    fn run_command(document: &Document, command: Command) -> Applied {
        let applied = apply(document, &command).unwrap();
        applied.document.validate().unwrap();
        applied
    }

    /// "Intro" / "a<break>b<header>c" / "Tail é🙂", with a page break that
    /// switches to the document layout and a floating header that owns a
    /// flow holding "Header text".
    fn document_with_objects() -> Document {
        let document = Document::with_paragraphs([
            ("Intro", StyleId(1)),
            ("abc", StyleId(2)),
            ("Tail é🙂", StyleId(1)),
        ]);
        let layout = document.document_layout().id;
        let document = run_command(
            &document,
            Command::InsertBreak {
                at: main_at(&document, 1, 1),
                kind: BreakKind::Page,
                centre: true,
                layout: Some(layout),
            },
        )
        .document;
        let applied = run_command(
            &document,
            Command::InsertFloatingHeaderFooter {
                at: main_at(&document, 1, 5),
                kind: HeaderFooterKind::Header,
                pages: PageSides::Right,
            },
        );
        let flow = applied.report.added_flows[0];
        let mut document = applied.document;
        document
            .flow_mut(flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "Header text".into();
        document.validate().unwrap();
        document
    }

    fn header_flow(document: &Document) -> FlowId {
        document
            .paragraphs()
            .iter()
            .flat_map(|paragraph| &paragraph.objects)
            .find_map(InlineObject::owned_flow)
            .expect("the fixture has a floating header")
    }

    #[test]
    fn inserting_objects_places_one_placeholder_per_object() {
        let document = document_with_objects();
        let paragraph = &document.paragraphs()[1];
        assert_eq!(paragraph.text, format!("a{OBJ}b{OBJ}c"));
        assert_eq!(paragraph.object_offsets().collect::<Vec<_>>(), [1, 5]);
        assert!(matches!(
            paragraph.object_at(1),
            Some(InlineObject::Break {
                kind: BreakKind::Page,
                centre: true,
                layout: Some(_),
                ..
            })
        ));
        assert!(matches!(
            paragraph.object_at(5),
            Some(InlineObject::FloatingHeaderFooter {
                kind: HeaderFooterKind::Header,
                pages: PageSides::Right,
                ..
            })
        ));
        let flow = header_flow(&document);
        assert_eq!(document.flow(flow).unwrap().plain_text(), "Header text");
        assert_eq!(
            document.flow(flow).unwrap().paragraphs[0].style,
            StyleId(2),
            "header text starts in the style at the insertion point"
        );
    }

    #[test]
    fn object_commands_are_restricted_to_valid_objects_in_the_main_flow() {
        let document = document_with_objects();
        let flow = header_flow(&document);
        let in_header = TextPosition {
            flow,
            paragraph: 0,
            byte: 0,
        };
        assert_eq!(
            apply(
                &document,
                &Command::InsertBreak {
                    at: in_header,
                    kind: BreakKind::Page,
                    centre: false,
                    layout: None,
                },
            )
            .err(),
            Some("inline objects occur only in the main flow")
        );
        assert_eq!(
            apply(
                &document,
                &Command::InsertBreak {
                    at: main_at(&document, 0, 0),
                    kind: BreakKind::Column,
                    centre: true,
                    layout: None,
                },
            )
            .err(),
            Some("a column break neither centres nor switches layouts")
        );
        assert_eq!(
            apply(
                &document,
                &Command::InsertBreak {
                    at: main_at(&document, 0, 0),
                    kind: BreakKind::Page,
                    centre: false,
                    layout: Some(LayoutId::from_raw(999)),
                },
            )
            .err(),
            Some("layout does not exist")
        );
    }

    #[test]
    fn range_deletion_deletes_objects_and_what_they_own_and_undo_restores_them() {
        let mut document = document_with_objects();
        let original = document.clone();
        let flow = header_flow(&document);
        let mut history = History::new();

        let executed = history
            .execute(
                &mut document,
                (),
                &Command::delete(main_at(&original, 0, 2), main_at(&original, 2, 2)),
            )
            .unwrap();

        document.validate().unwrap();
        assert_eq!(document.plain_text(), "Inil é🙂");
        assert!(document.paragraphs()[0].objects.is_empty());
        assert!(document.flow(flow).is_none());
        assert_eq!(executed.report.removed_flows, [flow]);
        history.undo(&mut document, ()).unwrap();
        assert_eq!(document, original);
        history.redo(&mut document, ()).unwrap();
        assert!(document.flow(flow).is_none());
    }

    #[test]
    fn deleting_text_beside_objects_keeps_them() {
        let document = document_with_objects();
        let flow = header_flow(&document);
        // Delete "b" between the break and the header.
        let applied = run_command(
            &document,
            Command::delete(main_at(&document, 1, 4), main_at(&document, 1, 5)),
        );
        let paragraph = &applied.document.paragraphs()[1];
        assert_eq!(paragraph.text, format!("a{OBJ}{OBJ}c"));
        assert_eq!(paragraph.objects, document.paragraphs()[1].objects);
        assert!(applied.document.flow(flow).is_some());
        assert!(applied.report.removed_flows.is_empty());
    }

    #[test]
    fn split_and_join_carry_objects_with_their_characters() {
        let document = document_with_objects();
        let objects = document.paragraphs()[1].objects.clone();
        // Split between the break and "b".
        let split = run_command(
            &document,
            Command::split_paragraph(main_at(&document, 1, 4)),
        );
        let paragraphs = split.document.paragraphs();
        assert_eq!(paragraphs[1].text, format!("a{OBJ}"));
        assert_eq!(paragraphs[1].objects, objects[..1]);
        assert_eq!(paragraphs[2].text, format!("b{OBJ}c"));
        assert_eq!(paragraphs[2].objects, objects[1..]);

        let joined = run_command(
            &split.document,
            Command::join_with_previous(&split.document, split.document.main_flow_id(), 2).unwrap(),
        );
        assert_eq!(
            joined.document.paragraphs()[1].text,
            format!("a{OBJ}b{OBJ}c")
        );
        assert_eq!(joined.document.paragraphs()[1].objects, objects);
    }

    #[test]
    fn input_and_paste_never_create_objects() {
        let document = document_with_objects();
        let applied = run_command(
            &document,
            Command::insert(main_at(&document, 0, 5), format!(" x{OBJ}y\nz{OBJ}")),
        );
        let paragraphs = applied.document.paragraphs();
        assert_eq!(
            paragraphs[0].text,
            format!("Intro x{REPLACEMENT_CHARACTER}y")
        );
        assert_eq!(paragraphs[1].text, format!("z{REPLACEMENT_CHARACTER}"));
        assert!(paragraphs[0].objects.is_empty() && paragraphs[1].objects.is_empty());
        assert_eq!(
            applied.document.paragraphs()[2].objects,
            document.paragraphs()[1].objects
        );
    }

    #[test]
    fn single_character_deletion_deletes_breaks_and_floating_headers_like_characters() {
        let document = document_with_objects();
        let flow = header_flow(&document);
        // Backspace just after the break.
        let applied = run_command(
            &document,
            Command::DeleteCharacter {
                at: main_at(&document, 1, 4),
                forward: false,
            },
        );
        assert_eq!(applied.document.paragraphs()[1].text, format!("ab{OBJ}c"));
        assert_eq!(applied.caret, Some(main_at(&document, 1, 1)));
        assert!(matches!(
            applied.document.paragraphs()[1].objects[..],
            [InlineObject::FloatingHeaderFooter { .. }]
        ));
        // Delete just before the floating header.
        let applied = run_command(
            &applied.document,
            Command::DeleteCharacter {
                at: main_at(&document, 1, 2),
                forward: true,
            },
        );
        assert_eq!(applied.document.paragraphs()[1].text, "abc");
        assert_eq!(applied.report.removed_flows, [flow]);
        assert!(applied.document.flow(flow).is_none());
    }

    #[test]
    fn single_character_deletion_steps_over_utf8_and_paragraph_boundaries() {
        let document = document_with_objects();
        let end = document.paragraphs()[2].text.len();
        let backspace = run_command(
            &document,
            Command::DeleteCharacter {
                at: main_at(&document, 2, end),
                forward: false,
            },
        );
        assert_eq!(backspace.document.paragraphs()[2].text, "Tail é");
        let join = run_command(
            &document,
            Command::DeleteCharacter {
                at: main_at(&document, 0, 5),
                forward: true,
            },
        );
        assert_eq!(
            join.document.paragraphs()[0].text,
            format!("Introa{OBJ}b{OBJ}c")
        );
        assert_eq!(join.document.paragraphs()[0].objects.len(), 2);
        for (paragraph, byte, forward) in [(0, 0, false), (2, end, true)] {
            let unchanged = apply(
                &document,
                &Command::DeleteCharacter {
                    at: main_at(&document, paragraph, byte),
                    forward,
                },
            )
            .unwrap();
            assert!(unchanged.report.is_empty());
        }
    }

    fn with_marks(marks: &[(usize, &str)]) -> Document {
        let mut document =
            Document::with_paragraphs([("one two", StyleId(1)), ("three", StyleId(1))]);
        document.paragraph_mut(0).marks = marks
            .iter()
            .map(|(byte, raw)| LegacyMark {
                byte: *byte,
                raw: (*raw).into(),
            })
            .collect();
        document.validate().unwrap();
        document
    }

    fn marks_of(document: &Document, paragraph: usize) -> Vec<(usize, String)> {
        document.paragraphs()[paragraph]
            .marks
            .iter()
            .map(|mark| (mark.byte, mark.raw.clone()))
            .collect()
    }

    #[test]
    fn legacy_marks_move_like_run_boundaries_and_die_with_their_text() {
        let document = with_marks(&[(0, "<:a>"), (3, "<:b>"), (5, "<:c>"), (7, "<:d>")]);

        let inserted = run_command(&document, Command::insert(main_at(&document, 0, 3), "XY"));
        assert_eq!(
            marks_of(&inserted.document, 0),
            [
                (0, "<:a>".into()),
                (5, "<:b>".into()),
                (7, "<:c>".into()),
                (9, "<:d>".into())
            ]
        );

        // Deleting bytes 2..6 removes the marks strictly inside: <:b> and <:c>.
        let deleted = run_command(
            &document,
            Command::delete(main_at(&document, 0, 2), main_at(&document, 0, 6)),
        );
        assert_eq!(deleted.document.paragraphs()[0].text, "ono");
        assert_eq!(
            marks_of(&deleted.document, 0),
            [(0, "<:a>".into()), (3, "<:d>".into())]
        );

        // A mark at either end of a deleted range survives.
        let ends = run_command(
            &document,
            Command::delete(main_at(&document, 0, 3), main_at(&document, 0, 5)),
        );
        assert_eq!(
            marks_of(&ends.document, 0),
            [
                (0, "<:a>".into()),
                (3, "<:b>".into()),
                (3, "<:c>".into()),
                (5, "<:d>".into())
            ]
        );

        let split = run_command(
            &document,
            Command::split_paragraph(main_at(&document, 0, 3)),
        );
        assert_eq!(marks_of(&split.document, 0), [(0, "<:a>".into())]);
        assert_eq!(
            marks_of(&split.document, 1),
            [(0, "<:b>".into()), (2, "<:c>".into()), (4, "<:d>".into())]
        );

        let joined = run_command(
            &split.document,
            Command::join_with_previous(&split.document, split.document.main_flow_id(), 1).unwrap(),
        );
        assert_eq!(marks_of(&joined.document, 0), marks_of(&document, 0));
    }

    #[test]
    fn plain_text_edits_keep_styles_runs_and_shared_paragraphs() {
        let mut document = Document::with_paragraphs([
            ("Heading", StyleId(2)),
            ("plain bold tail", StyleId(1)),
            ("Last", StyleId(1)),
        ]);
        document.paragraph_mut(1).runs.push(CharacterRun {
            start: 6,
            end: 10,
            format: bold(),
        });

        let command =
            plain_text_edit(&document, "Heading\nplain bo🙂ld tail\nLast", Some(20)).unwrap();
        let applied = run_command(&document, command);
        assert_eq!(applied.document.paragraphs()[1].runs[0].start, 6);
        assert_eq!(applied.document.paragraphs()[1].runs[0].end, 14);
        for index in [0, 2] {
            assert!(Arc::ptr_eq(
                &document.paragraphs()[index],
                &applied.document.paragraphs()[index]
            ));
        }

        let command = plain_text_edit(
            &document,
            "Heading revised\r\ncontinued\rplain bold tail\nLast",
            None,
        )
        .unwrap();
        let applied = run_command(&document, command);
        assert_eq!(
            applied.document.plain_text(),
            "Heading revised\ncontinued\nplain bold tail\nLast"
        );
        assert_eq!(
            applied
                .document
                .paragraphs()
                .iter()
                .map(|paragraph| paragraph.style)
                .collect::<Vec<_>>(),
            [StyleId(2), StyleId(2), StyleId(1), StyleId(1)]
        );
        assert!(plain_text_edit(&document, &document.plain_text(), None).is_none());
    }

    #[test]
    fn plain_text_edits_use_the_caret_to_place_ambiguous_edits() {
        let document = Document::with_paragraphs([("Heading", StyleId(2)), ("Body", StyleId(1))]);
        let ids: Vec<_> = document.paragraphs().iter().map(|p| p.id).collect();

        // Enter at the end of the heading: the caret is at the start of line 1.
        let enter_at_end = run_command(
            &document,
            plain_text_edit(&document, "Heading\n\nBody", Some(8)).unwrap(),
        );
        let paragraphs = enter_at_end.document.paragraphs();
        assert_eq!(paragraphs[0].id, ids[0]);
        assert_eq!(
            paragraphs[1].style,
            StyleId(2),
            "the new paragraph continues the heading"
        );
        assert!(Arc::ptr_eq(&paragraphs[2], &document.paragraphs()[1]));

        // Enter at the start of the body: the caret is at the start of line 2.
        let enter_at_start = run_command(
            &document,
            plain_text_edit(&document, "Heading\n\nBody", Some(9)).unwrap(),
        );
        let paragraphs = enter_at_start.document.paragraphs();
        assert!(Arc::ptr_eq(&paragraphs[0], &document.paragraphs()[0]));
        assert_eq!(paragraphs[1].style, StyleId(1));
        assert_eq!(paragraphs[2].style, StyleId(1));

        // Typing "o" after "Bo" rather than before it.
        let mut bold_document = document.clone();
        bold_document.paragraph_mut(1).runs.push(CharacterRun {
            start: 0,
            end: 2,
            format: bold(),
        });
        let typed = run_command(
            &bold_document,
            plain_text_edit(&bold_document, "Heading\nBoody", Some(11)).unwrap(),
        );
        assert_eq!(
            (
                typed.document.paragraphs()[1].runs[0].start,
                typed.document.paragraphs()[1].runs[0].end
            ),
            (0, 3),
            "text typed at the end of a run extends it"
        );
    }

    #[test]
    fn plain_text_edits_apply_the_object_rule_and_never_invent_objects() {
        let document = document_with_objects();
        let flow = header_flow(&document);
        let text = document.plain_text();

        // Backspace over the floating header's placeholder.
        let header_end = text.find('c').unwrap();
        let mut edited = text.clone();
        edited.replace_range(header_end - OBJ.len_utf8()..header_end, "");
        let applied = run_command(
            &document,
            plain_text_edit(&document, &edited, Some(header_end - OBJ.len_utf8())).unwrap(),
        );
        assert_eq!(applied.document.paragraphs()[1].objects.len(), 1);
        assert_eq!(applied.report.removed_flows, [flow]);

        // A pasted placeholder becomes U+FFFD, so the model differs from the
        // editor text and the editor must be brought back to the model.
        let pasted = format!("{text}{OBJ}");
        let applied = run_command(
            &document,
            plain_text_edit(&document, &pasted, Some(pasted.len())).unwrap(),
        );
        assert!(
            applied
                .document
                .plain_text()
                .ends_with(REPLACEMENT_CHARACTER)
        );
        assert_eq!(applied.document.paragraphs()[2].objects.len(), 0);
        assert_eq!(applied.document.paragraphs()[1].objects.len(), 2);
    }

    #[test]
    fn changed_span_finds_the_edit_on_utf8_boundaries_in_long_texts() {
        let long = "é🙂 paragraph text ".repeat(40);
        for (old, new) in [
            (format!("{long}abc{long}"), format!("{long}aXbc{long}")),
            (format!("{long}é{long}"), format!("{long}è{long}")),
            (format!("{long}🙂"), long.clone()),
            (long.clone(), format!("é{long}")),
            ("same".to_owned(), "same".to_owned()),
        ] {
            let (prefix, suffix) = changed_span(&old, &new);
            assert!(old.is_char_boundary(prefix) && new.is_char_boundary(prefix));
            assert!(prefix + suffix <= old.len().min(new.len()));
            assert_eq!(
                format!(
                    "{}{}{}",
                    &old[..prefix],
                    &new[prefix..new.len() - suffix],
                    &old[old.len() - suffix..]
                ),
                new
            );
            let unchanged_bytes = prefix + suffix;
            assert!(
                unchanged_bytes + 8 >= old.len().min(new.len()),
                "the span is minimal up to one character"
            );
        }
    }

    fn fixed(page: u32, left: Twips) -> Placement {
        Placement::Fixed {
            page,
            rect: Rect::new(left, 1_440, left + 2_880, 2_880),
        }
    }

    fn anchored_below() -> Placement {
        Placement::BelowParagraph {
            x_offset: 0,
            size: amipro_document::Size {
                width: 2_880,
                height: 1_440,
            },
        }
    }

    fn create(
        document: &Document,
        placement: Placement,
        anchor: Option<TextPosition>,
    ) -> (Document, FrameId) {
        let style = FrameStyle {
            wrap: if matches!(placement, Placement::BelowParagraph { .. }) {
                amipro_document::Wrap::NoWrapAround
            } else {
                amipro_document::Wrap::Around
            },
            ..FrameStyle::default()
        };
        let applied = run_command(
            document,
            Command::CreateFrame {
                placement,
                style,
                content: NewFrameContent::Text,
                anchor,
                layout: None,
            },
        );
        let frame = *applied.report.frames.iter().next().unwrap();
        (applied.document, frame)
    }

    fn frame_text_flow(document: &Document, frame: FrameId) -> FlowId {
        match document.frame(frame).unwrap().content {
            amipro_document::FrameContent::Text(flow) => flow,
            _ => panic!("text frame"),
        }
    }

    #[test]
    fn frames_are_created_in_their_layer_with_their_content() {
        let document = Document::from_plain_text("First\nSecond");
        let (document, page_frame) = create(&document, fixed(1, 1_440), None);
        assert_eq!(document.frame_stack(), [page_frame]);
        let flow = frame_text_flow(&document, page_frame);
        assert_eq!(document.flow(flow).unwrap().plain_text(), "");

        let at = main_at(&document, 0, 5);
        let (document, below) = create(&document, anchored_below(), Some(at));
        assert_eq!(document.frame_stack(), [page_frame, below]);
        assert_eq!(document.paragraphs()[0].text, "First\u{FFFC}");
        assert_eq!(
            document.paragraphs()[0].objects,
            [InlineObject::FrameAnchor(below)]
        );

        let table = run_command(
            &document,
            Command::CreateFrame {
                placement: fixed(1, 5_000),
                style: FrameStyle::default(),
                content: NewFrameContent::Table {
                    columns: vec![1_440; 3],
                    rows: vec![300; 2],
                },
                anchor: None,
                layout: None,
            },
        );
        assert_eq!(table.report.added_flows.len(), 6, "one flow per cell");

        let picture = run_command(
            &table.document,
            Command::CreateFrame {
                placement: Placement::Repeating {
                    pages: amipro_document::RepeatPages::RightLeft,
                    rect: Rect::new(0, 0, 1_440, 1_440),
                },
                style: FrameStyle::default(),
                content: NewFrameContent::EmbeddedPicture {
                    bytes: Arc::from(&b"BM synthetic picture"[..]),
                    extension: ".bmp".into(),
                },
                anchor: None,
                layout: None,
            },
        );
        let picture_frame = *picture.report.frames.iter().next().unwrap();
        assert_eq!(
            picture.document.document_layout().repeating,
            [picture_frame]
        );
        assert_eq!(picture.document.objects().len(), 1);

        for (placement, anchor, message) in [
            (
                Placement::Header {
                    side: amipro_document::PageSide::Right,
                },
                None,
                "header and footer frames belong to page layouts",
            ),
            (
                anchored_below(),
                None,
                "anchored frames, and only they, need an anchor position",
            ),
            (fixed(0, 0), None, "invalid frame placement"),
        ] {
            assert_eq!(
                apply(
                    &document,
                    &Command::CreateFrame {
                        placement,
                        style: FrameStyle {
                            wrap: amipro_document::Wrap::NoWrapAround,
                            ..FrameStyle::default()
                        },
                        content: NewFrameContent::Text,
                        anchor,
                        layout: None,
                    }
                )
                .err(),
                Some(message)
            );
        }
    }

    #[test]
    fn deleting_frames_follows_groups_anchors_and_the_single_delete_rule() {
        let document = Document::from_plain_text("Anchor paragraph\nNext");
        let (document, first) = create(&document, fixed(2, 1_440), None);
        let (document, second) = create(&document, fixed(2, 6_000), None);
        let document = run_command(
            &document,
            Command::Group {
                frames: vec![first, second],
            },
        )
        .document;
        let at = main_at(&document, 0, 6);
        let (document, below) = create(&document, anchored_below(), Some(at));
        let below_flow = frame_text_flow(&document, below);

        let group_deleted = run_command(&document, Command::DeleteFrame { frame: second });
        assert_eq!(
            group_deleted.report.removed_frames.len(),
            2,
            "the group goes together"
        );
        assert_eq!(group_deleted.document.frame_stack(), [below]);

        let anchored_deleted = run_command(&document, Command::DeleteFrame { frame: below });
        assert_eq!(
            anchored_deleted.document.paragraphs()[0].text,
            "Anchor paragraph"
        );
        assert!(anchored_deleted.document.flow(below_flow).is_none());

        // V05: a single Delete onto the anchor does nothing.
        let single = apply(
            &document,
            &Command::DeleteCharacter {
                at: main_at(&document, 0, 6),
                forward: true,
            },
        )
        .unwrap();
        assert!(single.report.is_empty());
        assert!(single.document.frame(below).is_some());

        // V06: deleting a selection that holds the anchor deletes the frame,
        // and one Undo restores both.
        let mut edited = document.clone();
        let mut history = History::new();
        history
            .execute(
                &mut edited,
                (),
                &Command::delete(main_at(&document, 0, 2), main_at(&document, 1, 1)),
            )
            .unwrap();
        edited.validate().unwrap();
        assert!(edited.frame(below).is_none());
        history.undo(&mut edited, ()).unwrap();
        assert_eq!(edited, document);

        let header = document.document_layout().header.right.unwrap();
        assert!(apply(&document, &Command::DeleteFrame { frame: header }).is_err());
    }

    /// T1 (`UI_SPEC.md` §24): Delete in a picture's cell removes the
    /// picture as one undo step; deleting its table removes it too; a frame
    /// in a cell is not moved, resized, restacked, or placed elsewhere.
    #[test]
    fn frames_in_cells_delete_with_undo_and_follow_their_table() {
        let document = Document::from_plain_text("Before\nAfter");
        let applied = run_command(
            &document,
            Command::CreateFrame {
                placement: anchored_below(),
                style: FrameStyle {
                    wrap: amipro_document::Wrap::NoWrapAround,
                    ..FrameStyle::default()
                },
                content: NewFrameContent::Table {
                    columns: vec![3_017, 3_017],
                    rows: vec![284],
                },
                anchor: Some(main_at(&document, 1, 0)),
                layout: None,
            },
        );
        let table = *applied.report.frames.iter().next().unwrap();
        let mut document = applied.document;
        let picture = document.allocate_frame_id();
        document.insert_frame(amipro_document::Frame {
            id: picture,
            placement: Placement::InCell,
            style: FrameStyle::default(),
            group: None,
            content: amipro_document::FrameContent::Legacy(Default::default()),
            legacy: Default::default(),
        });
        document.frame_stack_mut().insert(0, picture);
        let amipro_document::FrameContent::Table(cells) =
            &mut document.frame_mut(table).unwrap().content
        else {
            panic!("a table");
        };
        cells.cells[1].frame = Some(picture);
        document.validate().unwrap();

        let mut edited = document.clone();
        let mut history = History::new();
        let report = history
            .execute(&mut edited, (), &Command::DeleteFrame { frame: picture })
            .unwrap();
        assert!(
            report.report.frames.contains(&table),
            "its table is laid out again"
        );
        edited.validate().unwrap();
        assert!(edited.frame(picture).is_none());
        assert_eq!(edited.cell_holding(picture), None);
        history.undo(&mut edited, ()).unwrap();
        assert_eq!(edited, document);

        // Deleting the table's anchor with the text around it.
        let mut edited = document.clone();
        history
            .execute(
                &mut edited,
                (),
                &Command::delete(main_at(&document, 0, 2), main_at(&document, 1, 3)),
            )
            .unwrap();
        edited.validate().unwrap();
        assert!(edited.frame(table).is_none() && edited.frame(picture).is_none());
        history.undo(&mut edited, ()).unwrap();
        assert_eq!(edited, document);

        for refused in [
            Command::MoveFrame {
                frame: picture,
                dx: 100,
                dy: 0,
            },
            Command::ResizeFrame {
                frame: picture,
                rect: Rect::new(0, 0, 100, 100),
            },
            Command::BringToFront { frame: picture },
            Command::SendToBack { frame: picture },
            Command::ChangePlacement {
                frame: picture,
                placement: Placement::Fixed {
                    page: 1,
                    rect: Rect::new(0, 0, 100, 100),
                },
                anchor: None,
                layout: None,
            },
        ] {
            assert!(apply(&document, &refused).is_err(), "{refused:?}");
        }
    }

    #[test]
    fn frames_move_resize_and_change_style_by_their_placement() {
        let document = Document::from_plain_text("Text");
        let (document, first) = create(&document, fixed(1, 1_440), None);
        let (document, second) = create(&document, fixed(1, 6_000), None);
        let grouped = run_command(
            &document,
            Command::Group {
                frames: vec![first, second],
            },
        )
        .document;
        let moved = run_command(
            &grouped,
            Command::MoveFrame {
                frame: first,
                dx: 100,
                dy: -40,
            },
        );
        for (frame, left) in [(first, 1_540), (second, 6_100)] {
            assert!(matches!(
                moved.document.frame(frame).unwrap().placement,
                Placement::Fixed { rect, .. } if rect.left == left && rect.top == 1_400
            ));
        }
        let paged = run_command(
            &grouped,
            Command::SetFramePage {
                frame: second,
                page: 3,
            },
        );
        assert!(
            paged
                .document
                .frames()
                .filter(|f| f.group.is_some())
                .all(|f| matches!(f.placement, Placement::Fixed { page: 3, .. }))
        );

        let at = main_at(&document, 0, 4);
        let (anchored, below) = create(&document, anchored_below(), Some(at));
        assert_eq!(
            apply(
                &anchored,
                &Command::MoveFrame {
                    frame: below,
                    dx: 0,
                    dy: 10
                }
            )
            .err(),
            Some("a With para above frame moves only horizontally")
        );
        let resized = run_command(
            &anchored,
            Command::ResizeFrame {
                frame: below,
                rect: Rect::new(0, 0, 1_000, 500),
            },
        );
        assert!(matches!(
            resized.document.frame(below).unwrap().placement,
            Placement::BelowParagraph { size, .. } if size.width == 1_000 && size.height == 500
        ));

        let mut style = FrameStyle {
            corners: 18,
            wrap: amipro_document::Wrap::NoWrapBeside,
            ..FrameStyle::default()
        };
        let styled = run_command(
            &document,
            Command::SetFrameStyle {
                frame: first,
                style: style.clone(),
            },
        );
        assert_eq!(styled.document.frame(first).unwrap().style, style);
        style.wrap = amipro_document::Wrap::Around;
        assert!(
            apply(
                &anchored,
                &Command::SetFrameStyle {
                    frame: below,
                    style
                }
            )
            .is_err()
        );
    }

    #[test]
    fn placement_changes_move_frames_between_layers_and_anchors() {
        let document = Document::from_plain_text("Paragraph text");
        let (document, frame) = create(&document, fixed(1, 1_440), None);
        let flow = frame_text_flow(&document, frame);
        let inline = Placement::Inline {
            y_offset: 0,
            size: amipro_document::Size {
                width: 1_440,
                height: 720,
            },
        };

        let anchored = run_command(
            &document,
            Command::ChangePlacement {
                frame,
                placement: inline,
                anchor: Some(main_at(&document, 0, 9)),
                layout: None,
            },
        );
        assert_eq!(
            anchored.document.paragraphs()[0].text,
            "Paragraph\u{FFFC} text"
        );
        assert_eq!(
            anchored.document.frame(frame).unwrap().style.wrap,
            amipro_document::Wrap::Around
        );
        // F19: a Flow with text frame made taller keeps its top's height
        // above the baseline, so it hangs further below it.
        let taller = run_command(
            &anchored.document,
            Command::ResizeFrame {
                frame,
                rect: Rect::new(0, 0, 1_440, 1_000),
            },
        );
        assert_eq!(
            taller.document.frame(frame).unwrap().placement,
            Placement::Inline {
                y_offset: 280,
                size: amipro_document::Size {
                    width: 1_440,
                    height: 1_000
                }
            }
        );
        // F16: a Wrap around frame that becomes With para above takes No
        // wrap beside, as Ami Pro's Modify Frame Layout chooses.
        let below = run_command(
            &document,
            Command::ChangePlacement {
                frame,
                placement: Placement::BelowParagraph {
                    x_offset: 0,
                    size: amipro_document::Size {
                        width: 1_440,
                        height: 720,
                    },
                },
                anchor: Some(main_at(&document, 0, 9)),
                layout: None,
            },
        );
        assert_eq!(
            below.document.frame(frame).unwrap().style.wrap,
            amipro_document::Wrap::NoWrapBeside
        );

        let fixed_again = run_command(
            &anchored.document,
            Command::ChangePlacement {
                frame,
                placement: fixed(2, 1_440),
                anchor: None,
                layout: None,
            },
        );
        assert_eq!(fixed_again.document.paragraphs()[0].text, "Paragraph text");
        assert!(
            fixed_again.document.frame(frame).is_some(),
            "the frame survives"
        );
        assert!(fixed_again.document.flow(flow).is_some());

        let repeating = run_command(
            &document,
            Command::ChangePlacement {
                frame,
                placement: Placement::Repeating {
                    pages: amipro_document::RepeatPages::All,
                    rect: Rect::new(0, 0, 720, 720),
                },
                anchor: None,
                layout: None,
            },
        );
        assert!(repeating.document.frame_stack().is_empty());
        assert_eq!(repeating.document.document_layout().repeating, [frame]);
        let back = run_command(
            &repeating.document,
            Command::ChangePlacement {
                frame,
                placement: fixed(1, 0),
                anchor: None,
                layout: None,
            },
        );
        assert_eq!(back.document.frame_stack(), [frame]);
        assert!(back.document.document_layout().repeating.is_empty());
    }

    #[test]
    fn z_order_and_groups_stay_within_the_page_layer() {
        let document = Document::from_plain_text("Text");
        let (document, a) = create(&document, fixed(1, 0), None);
        let (document, b) = create(&document, fixed(1, 3_000), None);
        let (document, c) = create(&document, fixed(1, 6_000), None);
        let (document, other_page) = create(&document, fixed(2, 0), None);
        assert_eq!(document.frame_stack(), [a, b, c, other_page]);

        let front = run_command(&document, Command::BringToFront { frame: a });
        assert_eq!(front.document.frame_stack(), [b, c, other_page, a]);
        let back = run_command(&document, Command::SendToBack { frame: c });
        assert_eq!(back.document.frame_stack(), [c, a, b, other_page]);
        assert!(
            apply(&document, &Command::SendToBack { frame: a })
                .unwrap()
                .report
                .is_empty()
        );

        // Grouping a and c gathers them where the frontmost was.
        let grouped = run_command(&document, Command::Group { frames: vec![c, a] });
        assert_eq!(grouped.document.frame_stack(), [b, a, c, other_page]);
        let block = run_command(&grouped.document, Command::SendToBack { frame: c });
        assert_eq!(block.document.frame_stack(), [a, c, b, other_page]);
        let group = grouped.document.frame(a).unwrap().group.unwrap();
        let ungrouped = run_command(&grouped.document, Command::Ungroup { group });
        assert!(
            ungrouped
                .document
                .frames()
                .all(|frame| frame.group.is_none())
        );

        assert_eq!(
            apply(
                &document,
                &Command::Group {
                    frames: vec![a, other_page]
                }
            )
            .err(),
            Some("grouped frames must be on one page")
        );
        let header = document.document_layout().header.right.unwrap();
        assert!(apply(&document, &Command::BringToFront { frame: header }).is_err());
    }

    #[test]
    fn z_order_moves_a_frame_within_its_kind_fc8() {
        let document = Document::from_plain_text("One\nTwo\nThree");
        let main = document.main_flow_id();
        let at = |paragraph| TextPosition {
            flow: main,
            paragraph,
            byte: 0,
        };
        let (document, a1) = create(&document, anchored_below(), Some(at(0)));
        let (document, f1) = create(&document, fixed(1, 0), None);
        let (document, a2) = create(&document, anchored_below(), Some(at(1)));
        let (document, f2) = create(&document, fixed(1, 3_000), None);
        assert_eq!(document.frame_stack(), [a1, f1, a2, f2]);

        // The anchored frames trade their places; the Where placed ones
        // keep theirs.
        let front = run_command(&document, Command::BringToFront { frame: a1 });
        assert_eq!(front.document.frame_stack(), [a2, f1, a1, f2]);
        let back = run_command(&document, Command::SendToBack { frame: f2 });
        assert_eq!(back.document.frame_stack(), [a1, f2, a2, f1]);
        // Already the front of its kind: nothing to undo.
        for frame in [a2, f2] {
            assert!(
                apply(&document, &Command::BringToFront { frame })
                    .unwrap()
                    .report
                    .is_empty()
            );
        }
        assert!(
            apply(&document, &Command::SendToBack { frame: f1 })
                .unwrap()
                .report
                .is_empty()
        );
    }

    #[test]
    fn layouts_are_inserted_as_copies_modified_and_removed() {
        let document = Document::from_plain_text("Page one\nPage two");
        let standard = document.document_layout().clone();
        let inserted = run_command(
            &document,
            Command::InsertLayout {
                at: main_at(&document, 0, 8),
                from: standard.id,
            },
        );
        let layouts = inserted.document.layouts();
        assert_eq!(layouts.len(), 2);
        let copy = &layouts[1];
        assert_eq!(copy.name, "1");
        assert_ne!(copy.header.right, standard.header.right, "fresh frame IDs");
        assert_ne!(
            frame_text_flow(&inserted.document, copy.header.right.unwrap()),
            frame_text_flow(&document, standard.header.right.unwrap()),
            "fresh flow IDs"
        );
        assert_eq!(
            inserted.document.paragraphs()[0].objects,
            [InlineObject::Break {
                kind: BreakKind::Page,
                centre: false,
                layout: Some(copy.id),
                restart: false,
            }]
        );
        assert_eq!(
            apply(
                &inserted.document,
                &Command::RemoveLayout { layout: copy.id }
            )
            .err(),
            Some("a page break still switches to this layout")
        );
        let without_break = run_command(
            &inserted.document,
            Command::DeleteCharacter {
                at: main_at(&inserted.document, 0, 8),
                forward: true,
            },
        );
        let removed = run_command(
            &without_break.document,
            Command::RemoveLayout { layout: copy.id },
        );
        assert_eq!(removed.document.layouts().len(), 1);
        assert_eq!(removed.report.removed_frames.len(), 2);
        assert!(
            apply(
                &document,
                &Command::RemoveLayout {
                    layout: standard.id
                }
            )
            .is_err()
        );

        let mut settings = LayoutSettings {
            name: standard.name.clone(),
            paper: standard.paper,
            landscape: standard.landscape,
            right: standard.right.clone(),
            left: Some(standard.right.clone()),
            mirror: true,
            header_from_second_page: true,
            footer_from_second_page: false,
            header: BandBoxes::default(),
            footer: BandBoxes::default(),
        };
        let with_left = run_command(
            &document,
            Command::ModifyLayout {
                layout: standard.id,
                settings: settings.clone(),
            },
        );
        let layout = with_left.document.document_layout();
        assert!(layout.mirror && layout.header_from_second_page);
        let left_header = with_left
            .document
            .frame(layout.header.left.unwrap())
            .unwrap();
        assert_eq!(
            left_header.placement,
            Placement::Header {
                side: amipro_document::PageSide::Left
            }
        );
        settings.left = None;
        settings.mirror = false;
        let without_left = run_command(
            &with_left.document,
            Command::ModifyLayout {
                layout: standard.id,
                settings: settings.clone(),
            },
        );
        assert!(
            without_left
                .document
                .document_layout()
                .header
                .left
                .is_none()
        );
        assert_eq!(without_left.report.removed_frames.len(), 2);
        settings.mirror = true;
        assert!(
            apply(
                &document,
                &Command::ModifyLayout {
                    layout: standard.id,
                    settings
                }
            )
            .is_err()
        );
    }

    /// A small deterministic generator, so the property test needs no
    /// dependency and every failure reproduces from its seed.
    struct Xorshift(u64);

    impl Xorshift {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, bound: usize) -> usize {
            (self.next() % bound.max(1) as u64) as usize
        }
    }

    fn random_position(document: &Document, random: &mut Xorshift) -> TextPosition {
        let paragraph = random.below(document.paragraphs().len());
        let text = &document.paragraphs()[paragraph].text;
        let boundaries: Vec<usize> = text
            .char_indices()
            .map(|(byte, _)| byte)
            .chain([text.len()])
            .collect();
        main_at(
            document,
            paragraph,
            boundaries[random.below(boundaries.len())],
        )
    }

    const PAPERS_CODES: [u8; 6] = [1, 2, 3, 4, 5, 6];

    fn random_command(document: &Document, random: &mut Xorshift) -> Command {
        let frames: Vec<FrameId> = document.frames().map(|frame| frame.id).collect();
        let frame = frames
            .get(random.below(frames.len()))
            .copied()
            .unwrap_or(FrameId::from_raw(1));
        let page = 1 + random.below(3) as u32;
        let left = 200 * random.below(40) as Twips;
        match random.below(28) {
            0 | 1 => Command::insert(
                random_position(document, random),
                ["é", "x🙂", "\n", "ab"][random.below(4)],
            ),
            2 => Command::delete(
                random_position(document, random),
                random_position(document, random),
            ),
            3 => Command::DeleteCharacter {
                at: random_position(document, random),
                forward: random.below(2) == 0,
            },
            4 => Command::CreateFrame {
                placement: fixed(page, left),
                style: FrameStyle::default(),
                content: [
                    NewFrameContent::Text,
                    NewFrameContent::Table {
                        columns: vec![700, 700],
                        rows: vec![300],
                    },
                    NewFrameContent::EmbeddedPicture {
                        bytes: Arc::from(&b"picture"[..]),
                        extension: ".bmp".into(),
                    },
                ][random.below(3)]
                .clone(),
                anchor: None,
                layout: None,
            },
            5 => {
                let below = random.below(2) == 0;
                Command::CreateFrame {
                    placement: if below {
                        anchored_below()
                    } else {
                        Placement::Inline {
                            y_offset: 0,
                            size: amipro_document::Size {
                                width: 700,
                                height: 300,
                            },
                        }
                    },
                    style: FrameStyle {
                        wrap: if below {
                            amipro_document::Wrap::NoWrapAround
                        } else {
                            amipro_document::Wrap::Around
                        },
                        ..FrameStyle::default()
                    },
                    content: NewFrameContent::Text,
                    anchor: Some(random_position(document, random)),
                    layout: None,
                }
            }
            6 => Command::CreateFrame {
                placement: Placement::Repeating {
                    pages: amipro_document::RepeatPages::All,
                    rect: Rect::new(left, 0, left + 500, 500),
                },
                style: FrameStyle::default(),
                content: NewFrameContent::Text,
                anchor: None,
                layout: None,
            },
            7 => Command::DeleteFrame { frame },
            8 => Command::MoveFrame {
                frame,
                dx: 50,
                dy: 0,
            },
            9 => Command::SetFramePage { frame, page },
            10 => Command::ResizeFrame {
                frame,
                rect: Rect::new(left, 100, left + 900, 1_000),
            },
            11 => Command::ChangePlacement {
                frame,
                placement: [
                    fixed(page, left),
                    anchored_below(),
                    Placement::Repeating {
                        pages: amipro_document::RepeatPages::RightLeft,
                        rect: Rect::new(0, 0, 600, 600),
                    },
                ][random.below(3)],
                anchor: Some(random_position(document, random)),
                layout: None,
            },
            12 => Command::BringToFront { frame },
            13 => Command::SendToBack { frame },
            14 => {
                // Two ungrouped Where placed frames on one page, when there are.
                let mut by_page: std::collections::BTreeMap<u32, Vec<FrameId>> =
                    std::collections::BTreeMap::new();
                for candidate in document.frames().filter(|f| f.group.is_none()) {
                    if let Placement::Fixed { page, .. } = candidate.placement {
                        by_page.entry(page).or_default().push(candidate.id);
                    }
                }
                let pairs: Vec<Vec<FrameId>> = by_page
                    .into_values()
                    .filter(|members| members.len() >= 2)
                    .collect();
                match pairs.get(random.below(pairs.len())) {
                    Some(members) => Command::Group {
                        frames: members[..2].to_vec(),
                    },
                    None => Command::Group {
                        frames: vec![frame, frame],
                    },
                }
            }
            15 => match document.frames().find_map(|frame| frame.group) {
                Some(group) => Command::Ungroup { group },
                None => Command::SendToBack { frame },
            },
            16 => Command::InsertLayout {
                at: random_position(document, random),
                from: document.layouts()[random.below(document.layouts().len())].id,
            },
            17 => Command::InsertPageNumber {
                at: random_position(document, random),
                field: PageNumberField {
                    style: PageNumberField::STYLES[random.below(5)],
                    start_page: random.below(4) as u32,
                    start_number: random.below(4) as u32,
                    leading_text: "Page ".into(),
                },
            },
            18 => {
                use layout_setup::{Band, MarginSide, Pages};
                let layout = &document.layouts()[random.below(document.layouts().len())];
                let mut settings = LayoutSettings::of(document, layout.id).unwrap();
                let pages = settings.pages();
                // Refusals leave the settings as they were.
                let _ = match random.below(5) {
                    0 => settings.set_margin(pages, MarginSide::Top, 300 + left),
                    1 => settings.set_columns(pages, 1 + random.below(3) as u8),
                    2 => settings.set_mirror(Pages::Right, random.below(2) == 0),
                    3 => settings.set_band_margin(Band::Footer, pages, MarginSide::Left, left),
                    _ => settings.set_paper(PAPERS_CODES[random.below(PAPERS_CODES.len())]),
                };
                Command::ModifyLayout {
                    layout: layout.id,
                    settings,
                }
            }
            19 | 20 => {
                let flow = document.main_flow_id();
                let count = document.flow(flow).map_or(1, |flow| flow.len());
                let first = random.below(count);
                let last = first + random.below(count - first);
                if random.below(2) == 0 {
                    Command::SetParagraphTabs {
                        flow,
                        first,
                        last,
                        tabs: [
                            None,
                            Some(Vec::new()),
                            Some(vec![amipro_document::TabStop::new(
                                amipro_document::TabKind::Decimal,
                                left,
                            )]),
                        ][random.below(3)]
                        .clone(),
                    }
                } else {
                    Command::SetParagraphIndents {
                        flow,
                        first,
                        last,
                        indents: IndentChange {
                            first_line: Some(left),
                            right: (random.below(2) == 0).then_some(90),
                            ..IndentChange::default()
                        },
                    }
                }
            }
            21 => Command::InsertBullet {
                at: random_position(document, random),
                code: amipro_document::BULLET_CODES[random.below(17)],
            },
            22 => {
                let flow = document.main_flow_id();
                let count = document.flow(flow).map_or(1, |flow| flow.len());
                let first = random.below(count);
                let last = first + random.below(count - first);
                Command::SetList {
                    flow,
                    first,
                    last,
                    list: [
                        None,
                        Some(lists::ListKind::Bullets),
                        Some(lists::ListKind::Numbering),
                    ][random.below(3)],
                }
            }
            23 => {
                let flow = document.main_flow_id();
                let count = document.flow(flow).map_or(1, |flow| flow.len());
                let first = random.below(count);
                let last = first + random.below(count - first);
                Command::ChangeLevel {
                    flow,
                    first,
                    last,
                    change: [outline::LevelChange::Promote, outline::LevelChange::Demote]
                        [random.below(2)],
                }
            }
            24 | 25 => {
                // Pictures: into an empty frame or cell, replaced, scaled,
                // moved in their frames, described.
                let drawn = random.below(2) == 0;
                let picture = NewPicture {
                    source: NewPictureSource::Embedded(if drawn {
                        drawing::empty_drawing()
                    } else {
                        Arc::from(&b"png"[..])
                    }),
                    extension: if drawn { ".sdw" } else { ".png" }.into(),
                    size: amipro_document::Size {
                        width: 600,
                        height: 400,
                    },
                };
                let pictures: Vec<FrameId> = document
                    .frames()
                    .filter(|candidate| {
                        matches!(candidate.content, amipro_document::FrameContent::Picture(_))
                    })
                    .map(|candidate| candidate.id)
                    .collect();
                let frame = match random.below(pictures.len() + 1) {
                    index if index < pictures.len() => pictures[index],
                    _ => frame,
                };
                let cell = document
                    .frames()
                    .find_map(|candidate| match &candidate.content {
                        amipro_document::FrameContent::Table(table) => {
                            Some((candidate.id, table.cells.len()))
                        }
                        _ => None,
                    });
                match random.below(8) {
                    6 | 7 => {
                        // Draw mode: a rectangle added to a drawing.
                        let mut objects = drawing::drawing_of(document, frame).unwrap_or_default();
                        let at = 100 + left as i16;
                        drawing::add(
                            &mut objects,
                            vec![amipro_pictures::sdw::Object::rectangle(
                                amipro_pictures::sdw::Rect::new(at, 100, at + 500, 400),
                                Default::default(),
                                Default::default(),
                                false,
                            )],
                        );
                        Command::SetDrawing {
                            frame,
                            drawing: Arc::new(objects),
                        }
                    }
                    0 => Command::InsertPicture {
                        target: match cell {
                            Some((table, cells)) if random.below(2) == 0 => PictureTarget::Cell {
                                table,
                                cell: random.below(cells),
                            },
                            _ => PictureTarget::Frame(frame),
                        },
                        picture,
                        cell_style: FrameStyle::default(),
                    },
                    1 => Command::ReplacePicture { frame, picture },
                    2 => Command::SetPictureScaling {
                        frame,
                        scaling: amipro_document::PictureScaling {
                            sizing: [
                                amipro_document::PictureSizing::Original,
                                amipro_document::PictureSizing::Custom,
                                amipro_document::PictureSizing::Percentage,
                            ][random.below(3)],
                            keep_aspect: random.below(2) == 0,
                            percent: 50,
                            size: amipro_document::Size {
                                width: 300 + left,
                                height: 300,
                            },
                            rotation: 90 * random.below(4) as u16,
                        },
                    },
                    3 | 4 => Command::MovePictureInFrame {
                        frame,
                        pan: Offset { x: left, y: -72 },
                    },
                    _ => Command::SetPictureDescription {
                        frame,
                        description: ["", "A logo", "é 🙂"][random.below(3)].into(),
                    },
                }
            }
            _ => {
                let layout = &document.layouts()[random.below(document.layouts().len())];
                let toggle_left = layout.left.is_none();
                let mut right = layout.right.clone();
                right.layout.margins.left = 1_000 + left;
                Command::ModifyLayout {
                    layout: layout.id,
                    settings: LayoutSettings {
                        name: layout.name.clone(),
                        paper: layout.paper,
                        landscape: false,
                        right: right.clone(),
                        left: toggle_left.then_some(right),
                        mirror: false,
                        header_from_second_page: random.below(2) == 0,
                        footer_from_second_page: false,
                        header: BandBoxes::default(),
                        footer: BandBoxes::default(),
                    },
                }
            }
        }
    }

    /// Random command sequences over text, objects, frames, groups, and
    /// layouts: every intermediate root validates, undoing everything gives
    /// back the original root exactly, redoing gives back the final one, and
    /// the final one survives a native v3 save and load exactly.
    #[test]
    fn undo_and_redo_restore_roots_exactly_after_random_command_sequences() {
        let mut exercised = BTreeSet::new();
        for seed in 1..=40_u64 {
            let mut random = Xorshift(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let original = Document::from_plain_text("First é paragraph\nSecond 🙂 one\nThird");
            let mut document = original.clone();
            let mut history = History::with_limit(1_000);
            let mut applied = 0;
            for _ in 0..80 {
                let command = random_command(&document, &mut random);
                if let Ok(executed) = history.execute(&mut document, (), &command) {
                    if let Err(error) = document.validate() {
                        panic!("seed {seed}: {command:?} left an invalid document: {error}");
                    }
                    if !executed.report.is_empty() {
                        applied += 1;
                        let name = format!("{command:?}");
                        exercised.insert(
                            name[..name.find([' ', '{', '(']).unwrap_or(name.len())].to_owned(),
                        );
                    }
                }
            }
            assert!(
                applied > 10,
                "seed {seed}: too few commands applied ({applied})"
            );
            let last = document.clone();
            // Native v3 keeps every generated document exactly, once payloads
            // that no picture refers to any more are dropped, as saving does.
            let mut saved = last.clone();
            saved.drop_unreferenced_objects();
            let encoded = saved.encode_native().unwrap();
            assert_eq!(
                Document::decode_native(&encoded).unwrap(),
                saved,
                "seed {seed}: v3 round trip"
            );
            while history.undo(&mut document, ()).is_some() {}
            assert_eq!(document, original, "seed {seed}: undo all");
            while history.redo(&mut document, ()).is_some() {}
            assert_eq!(document, last, "seed {seed}: redo all");
        }
        for kind in [
            "ReplaceText",
            "SetParagraphTabs",
            "SetParagraphIndents",
            "DeleteCharacter",
            "CreateFrame",
            "DeleteFrame",
            "MoveFrame",
            "SetFramePage",
            "ResizeFrame",
            "ChangePlacement",
            "BringToFront",
            "SendToBack",
            "Group",
            "Ungroup",
            "InsertLayout",
            "ModifyLayout",
            "InsertPageNumber",
            "InsertBullet",
            "SetList",
            "ChangeLevel",
            "InsertPicture",
            "ReplacePicture",
            "SetPictureScaling",
            "MovePictureInFrame",
            "SetPictureDescription",
            "SetDrawing",
        ] {
            assert!(
                exercised.contains(kind),
                "{kind} never applied: {exercised:?}"
            );
        }
    }

    #[test]
    fn inserted_bullets_go_into_any_flow_in_the_formatting_where_they_stand() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Bold text".into();
        document.paragraph_mut(0).runs = vec![CharacterRun {
            start: 0,
            end: 4,
            format: bold(),
        }];
        let mut history = History::new();
        let at = main_at(&document, 0, 2);
        history
            .execute(
                &mut document,
                (),
                &Command::InsertBullet { at, code: '\u{11}' },
            )
            .unwrap();
        let paragraph = &document.paragraphs()[0];
        assert_eq!(paragraph.text, "Bo\u{FFFC}ld text");
        assert_eq!(paragraph.objects, [InlineObject::Bullet('\u{11}')]);
        assert_eq!(paragraph.runs[0].end, 4 + '\u{FFFC}'.len_utf8());
        // A header holds them too; only Ami Pro's 17 codes.
        let header = document.document_layout().header.right.unwrap();
        let amipro_document::FrameContent::Text(flow) = document.frame(header).unwrap().content
        else {
            unreachable!();
        };
        let at = TextPosition {
            flow,
            paragraph: 0,
            byte: 0,
        };
        for code in amipro_document::BULLET_CODES {
            history
                .execute(&mut document, (), &Command::InsertBullet { at, code })
                .unwrap();
        }
        assert_eq!(document.flow(flow).unwrap().paragraphs[0].objects.len(), 17);
        assert!(apply(&document, &Command::InsertBullet { at, code: '\u{9}' }).is_err());
        history.undo(&mut document, ());
        assert_eq!(document.flow(flow).unwrap().paragraphs[0].objects.len(), 16);
    }

    #[test]
    fn page_numbers_go_into_any_flow_and_change_in_place() {
        let mut document = Document::default();
        let header = document.document_layout().header.right.unwrap();
        let amipro_document::FrameContent::Text(flow) = document.frame(header).unwrap().content
        else {
            unreachable!();
        };
        let at = TextPosition {
            flow,
            paragraph: 0,
            byte: 0,
        };
        let field = PageNumberField {
            style: 'i',
            start_page: 2,
            start_number: 1,
            leading_text: "Page ".into(),
        };
        let mut history = History::new();
        history
            .execute(
                &mut document,
                (),
                &Command::InsertPageNumber {
                    at,
                    field: field.clone(),
                },
            )
            .unwrap();
        let objects =
            |document: &Document| document.flow(flow).unwrap().paragraphs[0].objects.clone();
        assert_eq!(
            objects(&document),
            [InlineObject::PageNumber(field.clone())]
        );
        let changed = PageNumberField {
            style: 'A',
            ..field.clone()
        };
        let executed = history
            .execute(
                &mut document,
                (),
                &Command::SetPageNumber {
                    at,
                    field: changed.clone(),
                },
            )
            .unwrap();
        assert!(executed.report.flows.contains(&flow));
        assert_eq!(objects(&document), [InlineObject::PageNumber(changed)]);
        history.undo(&mut document, ());
        assert_eq!(
            objects(&document),
            [InlineObject::PageNumber(field.clone())]
        );
        // Only what Ami Pro's dialog sets, and only on a field.
        for refused in [
            PageNumberField {
                style: 'x',
                ..field.clone()
            },
            PageNumberField {
                start_page: 10_000,
                ..field.clone()
            },
            PageNumberField {
                leading_text: "x".repeat(31),
                ..field.clone()
            },
        ] {
            assert!(apply(&document, &Command::InsertPageNumber { at, field: refused }).is_err());
        }
        let after = TextPosition { byte: 3, ..at };
        assert!(
            apply(
                &document,
                &Command::SetPageNumber {
                    at: after,
                    field: field.clone()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn paragraph_alignment_is_set_over_the_style_and_cleared_in_one_undo_step() {
        let mut document = Document::default();
        let at = main_at(&document, 0, 0);
        let mut history = History::new();
        history
            .execute(&mut document, (), &Command::insert(at, "one\ntwo\nthree"))
            .unwrap();
        let before = document.clone();
        let centre = Command::SetParagraphAlignment {
            flow: at.flow,
            first: 0,
            last: 1,
            alignment: Some(Alignment::Centre),
        };
        let executed = history.execute(&mut document, (), &centre).unwrap();
        assert_eq!(executed.report.changed.len(), 2);
        let alignments: Vec<_> = document
            .paragraphs()
            .iter()
            .map(|paragraph| paragraph.format.alignment)
            .collect();
        assert_eq!(
            alignments,
            [Some(Alignment::Centre), Some(Alignment::Centre), None]
        );
        // Untouched paragraphs stay shared with the previous root.
        assert!(Arc::ptr_eq(
            &before.flow(at.flow).unwrap().paragraphs[2],
            &document.flow(at.flow).unwrap().paragraphs[2]
        ));
        // Again: nothing changes and nothing is recorded.
        let undo_len = history.undo_len();
        let executed = history.execute(&mut document, (), &centre).unwrap();
        assert!(executed.report.is_empty());
        assert_eq!(history.undo_len(), undo_len);
        // Back to the style's.
        history
            .execute(
                &mut document,
                (),
                &Command::SetParagraphAlignment {
                    flow: at.flow,
                    first: 0,
                    last: 0,
                    alignment: None,
                },
            )
            .unwrap();
        assert_eq!(document.paragraphs()[0].format.alignment, None);
        history.undo(&mut document, ()).unwrap();
        assert_eq!(
            document.paragraphs()[0].format.alignment,
            Some(Alignment::Centre)
        );
        history.undo(&mut document, ()).unwrap();
        assert_eq!(document, before);
        assert_eq!(
            apply(
                &document,
                &Command::SetParagraphAlignment {
                    flow: at.flow,
                    first: 1,
                    last: 3,
                    alignment: None,
                },
            )
            .err(),
            Some("paragraph out of bounds")
        );
    }

    #[test]
    fn no_op_commands_report_nothing_and_are_not_recorded() {
        let mut document = Document::default();
        let mut history = History::new();
        let at = main_at(&document, 0, 0);

        let executed = history
            .execute(&mut document, (), &Command::insert(at, ""))
            .unwrap();
        assert!(executed.report.is_empty());
        let executed = history
            .execute(
                &mut document,
                (),
                &Command::SetParagraphStyle {
                    flow: at.flow,
                    first: 0,
                    last: 0,
                    style: StyleId(1),
                },
            )
            .unwrap();
        assert!(executed.report.is_empty());
        assert!(!history.can_undo());
    }

    #[test]
    fn text_ranges_must_stay_in_one_flow_and_flows_must_exist() {
        let document = Document::default();
        let other = FlowId::from_raw(999);
        let inside = main_at(&document, 0, 0);
        let outside = TextPosition {
            flow: other,
            paragraph: 0,
            byte: 0,
        };
        assert_eq!(
            apply(&document, &Command::delete(inside, outside)).err(),
            Some("a text range must stay in one flow")
        );
        assert_eq!(
            apply(&document, &Command::insert(outside, "x")).err(),
            Some("flow does not exist")
        );
    }

    #[test]
    fn history_swaps_roots_and_caps_undo_steps() {
        let mut document = Document::default();
        let mut history = History::with_limit(3);
        let flow = document.main_flow_id();
        for index in 0..5 {
            let at = TextPosition {
                flow,
                paragraph: 0,
                byte: index,
            };
            history
                .execute(&mut document, index, &Command::insert(at, "x"))
                .unwrap();
        }
        assert_eq!(document.plain_text(), "xxxxx");
        assert_eq!(history.undo_len(), 3);

        let mut selections = Vec::new();
        while let Some(selection) = history.undo(&mut document, 99) {
            selections.push(selection);
        }
        assert_eq!(selections, [4, 3, 2]);
        assert_eq!(document.plain_text(), "xx");
        assert_eq!(history.redo(&mut document, 0), Some(99));
        assert_eq!(document.plain_text(), "xxx");
    }

    #[test]
    fn a_lower_undo_limit_drops_the_oldest_steps_and_a_higher_one_keeps_more() {
        let mut document = Document::default();
        let mut history = History::new();
        let flow = document.main_flow_id();
        let insert = |history: &mut History<usize>, document: &mut Document, index: usize| {
            let at = TextPosition {
                flow,
                paragraph: 0,
                byte: index,
            };
            history
                .execute(document, index, &Command::insert(at, "x"))
                .unwrap();
        };
        for index in 0..4 {
            insert(&mut history, &mut document, index);
        }
        assert_eq!(history.undo_len(), 4);
        let changes = history.changes();
        history.set_limit(2);
        assert_eq!(history.limit(), 2);
        assert_eq!(history.undo_len(), 2);
        assert!(history.changes() > changes);
        while history.undo(&mut document, 0).is_some() {}
        assert_eq!(document.plain_text(), "xx");
        history.set_limit(0);
        assert_eq!(history.limit(), 1);
        history.set_limit(5);
        for index in 2..5 {
            insert(&mut history, &mut document, index);
        }
        assert_eq!(history.undo_len(), 3);
    }

    #[test]
    fn history_changes_grow_at_the_limit_too() {
        let mut document = Document::default();
        let mut history = History::with_limit(1);
        let flow = document.main_flow_id();
        let at = |byte| TextPosition {
            flow,
            paragraph: 0,
            byte,
        };
        let insert = |byte| Command::insert(at(byte), "x");
        history.execute(&mut document, 0, &insert(0)).unwrap();
        let full = history.changes();
        history.execute(&mut document, 0, &insert(1)).unwrap();
        assert_eq!(history.undo_len(), 1, "still at the limit");
        assert!(history.changes() > full, "a step at the limit is a change");
        let made = history.changes();
        history.amend(&mut document, 0, &insert(2)).unwrap();
        assert!(history.changes() > made, "an amendment is a change");
        let amended = history.changes();
        // A command that changes nothing changes nothing.
        history
            .execute(&mut document, 0, &Command::delete(at(1), at(1)))
            .unwrap();
        assert_eq!(history.changes(), amended);
        history.undo(&mut document, 0);
        let undone = history.changes();
        assert!(undone > amended);
        history.redo(&mut document, 0);
        assert!(history.changes() > undone);
    }

    #[test]
    fn amended_commands_undo_as_one_step() {
        let mut document = Document::from_plain_text("a");
        let original = document.clone();
        let mut history = History::new();
        let at = |byte| main_at(&document, 0, byte);
        let (first, second) = (Command::insert(at(1), "b"), Command::insert(at(2), "c"));
        history.execute(&mut document, 1, &first).unwrap();
        history.amend(&mut document, 2, &second).unwrap();
        assert_eq!(document.plain_text(), "abc");
        assert_eq!(history.undo_len(), 1);
        assert_eq!(history.undo(&mut document, 3), Some(1));
        assert_eq!(document, original);
        assert_eq!(history.redo(&mut document, 1), Some(3));
        assert_eq!(document.plain_text(), "abc");

        // With nothing to undo, amend records a step.
        let mut fresh = Document::from_plain_text("a");
        let mut history = History::new();
        history.amend(&mut fresh, 7, &first).unwrap();
        assert_eq!(history.undo_len(), 1);
        assert_eq!(history.undo(&mut fresh, 8), Some(7));
        assert_eq!(fresh.plain_text(), "a");
    }

    #[test]
    fn ids_from_undone_edits_are_not_reused() {
        let mut document = Document::default();
        let mut history = History::new();
        let at = main_at(&document, 0, 0);
        let executed = history
            .execute(&mut document, (), &Command::split_paragraph(at))
            .unwrap();
        let undone_id = executed.report.inserted[0];
        history.undo(&mut document, ()).unwrap();

        let executed = history
            .execute(&mut document, (), &Command::split_paragraph(at))
            .unwrap();
        assert_ne!(executed.report.inserted[0], undone_id);
        assert!(executed.report.inserted[0] > undone_id);
        assert!(document.validate().is_ok());

        let mut direct = Document::default();
        let mut history = History::new();
        history
            .execute(&mut direct, (), &Command::split_paragraph(at))
            .unwrap();
        history.undo(&mut direct, ()).unwrap();
        let clean = direct.clone();
        history.record(&mut direct, ());
        let paragraph = direct.new_paragraph("b", StyleId(1));
        assert!(
            paragraph.id > undone_id,
            "direct edits after record get fresh IDs"
        );
        direct.main_flow_mut().paragraphs.push(Arc::new(paragraph));
        history.undo(&mut direct, ()).unwrap();
        assert_eq!(direct, clean, "undo restores the exact recorded root");
    }

    /// Per-edit cost of a command plus its history entry on a real imported
    /// document. Run with:
    /// `AMIPRO_PERF_SAM=$PWD/Reference/SAM/README31.SAM cargo test --release
    /// -p amipro-editing command_history_performance_baseline -- --ignored --nocapture`
    #[test]
    #[ignore = "release-mode benchmark over a private SAM file"]
    fn command_history_performance_baseline() {
        use std::time::{Duration, Instant};

        let Some(path) = std::env::var_os("AMIPRO_PERF_SAM") else {
            println!("AMIPRO_PERF_SAM is not set; skipping");
            return;
        };
        let import = amipro_formats::import_sam_path(&path).unwrap();
        let mut document = import.document;
        let flow = document.main_flow_id();
        let paragraph = document.paragraphs().len() / 2;
        let base_len = document.paragraphs()[paragraph].text.len();
        let mut history = History::<Cursor>::new();
        const SAMPLES: usize = 2_000;

        let percentile = |samples: &mut Vec<Duration>, fraction: f64| {
            samples.sort_unstable();
            samples[((samples.len() - 1) as f64 * fraction).round() as usize]
        };
        let mut typing = Vec::with_capacity(SAMPLES);
        for index in 0..SAMPLES {
            let at = TextPosition {
                flow,
                paragraph,
                byte: base_len + index,
            };
            let started = Instant::now();
            history
                .execute(
                    &mut document,
                    Cursor {
                        paragraph,
                        byte: at.byte,
                    },
                    &Command::insert(at, "x"),
                )
                .unwrap();
            typing.push(started.elapsed());
        }
        let mut undo = Vec::with_capacity(History::<Cursor>::DEFAULT_LIMIT);
        let mut redo = Vec::with_capacity(History::<Cursor>::DEFAULT_LIMIT);
        let caret = Cursor { paragraph, byte: 0 };
        for _ in 0..History::<Cursor>::DEFAULT_LIMIT {
            let started = Instant::now();
            history.undo(&mut document, caret).unwrap();
            undo.push(started.elapsed());
        }
        for _ in 0..History::<Cursor>::DEFAULT_LIMIT {
            let started = Instant::now();
            history.redo(&mut document, caret).unwrap();
            redo.push(started.elapsed());
        }
        println!(
            "command+history baseline: {} paragraphs, {SAMPLES} insertions in paragraph {paragraph}",
            document.paragraphs().len()
        );
        for (name, samples) in [
            ("insert", &mut typing),
            ("undo", &mut undo),
            ("redo", &mut redo),
        ] {
            let median = percentile(samples, 0.5);
            let p95 = percentile(samples, 0.95);
            let max = *samples.last().unwrap();
            println!("{name}: median {median:?}, p95 {p95:?}, max {max:?}");
        }
    }
}
