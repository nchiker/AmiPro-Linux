//! Writing frame records (`[frm]` on a page, `[lyfrm]` in a layout) and
//! their parts as Ami Pro 3.1's frame writer does (decompile `11c8:2a0f`):
//! fields 1–13 (16 for anchored frames), `[chint]`/`[ehint]` and other kept
//! parts, `[frmmac]`, `[frmname]`, `[frmlay]`, then the content: text,
//! a table (`1490:0000`), or a picture (`11c8:2e29`). Values the model does
//! not hold come from what the importer kept (`build.rs`) while they still
//! say what the model says.

use std::collections::{BTreeMap, BTreeSet};

use amipro_document::{
    CellContent, CellSpan, Document, FlowId, Frame, FrameContent, FrameId, LayoutId, Picture,
    PictureSource, Placement, Rect, Table, TableRows, TableSpacing, Twips, Wrap,
};

use crate::body::Body;
use crate::sheet::{
    Extent, FRAME_DEFAULTS, Sequence, frame_columns, frame_side_mask, layout_block, unsigned,
};
use crate::writer::Lines;

/// Frame flag bits (`SAM_PAGE_FRAME_SPEC.md` §4.3).
pub(crate) const TABLE: u32 = 0x4;
pub(crate) const LAYOUT_FRAME: u32 = 0x8;
pub(crate) const OPAQUE: u32 = 0x40;
pub(crate) const WRAP_AROUND: u32 = 0x80;
pub(crate) const HAS_TEXT: u32 = 0x200;
const NEVER_WRITTEN: u32 = 0x400;
pub(crate) const NO_WRAP_BESIDE: u32 = 0x20000;
const ANCHORED: u32 = 0x80000;
const GROUP_MEMBER: u32 = 0x80_0000;
const GROUP_FIRST: u32 = 0x100_0000;
const GROUP_LAST: u32 = 0x200_0000;
pub(crate) const PICTURE: u32 = 0x2000_0000;
pub(crate) use crate::build::IN_CELL;
/// A picture's bits that a layout frame from the style sheet loses in a
/// save (`11c8:2a0f`, with 0x8).
const SHEET_PICTURE_BITS: u32 = 0x8000 | 0x6000_0000;

/// What every frame record of one file is written with.
pub(crate) struct Context<'a> {
    pub document: &'a Document,
    /// Whether contents are written as Ami Pro's Save writes them; Save as
    /// a Style Sheet without contents leaves out pictures.
    pub contents: bool,
    /// Writing a style sheet: Ami Pro marks every frame it writes as the
    /// sheet's (0x8, and 0x80 on the layout number; `11c8:14e0`).
    pub sheet: bool,
    /// Whether frames marked as the sheet's (0x8) keep their text and
    /// pictures: `11c8:2a0f`'s fourth argument, set by Save as a Style
    /// Sheet with contents, clear in a document, whose sheet has them.
    pub sheet_frames_text: bool,
    pub layout_names: &'a BTreeMap<LayoutId, String>,
    pub anchors: &'a BTreeMap<FrameId, usize>,
}

static NO_LAYOUT_NAMES: BTreeMap<LayoutId, String> = BTreeMap::new();
static NO_ANCHORS: BTreeMap<FrameId, usize> = BTreeMap::new();

impl<'a> Context<'a> {
    /// Save as a Style Sheet without contents (`sheet.rs`).
    pub(crate) fn without_contents(document: &'a Document) -> Self {
        Self {
            document,
            contents: false,
            sheet: true,
            sheet_frames_text: false,
            layout_names: &NO_LAYOUT_NAMES,
            anchors: &NO_ANCHORS,
        }
    }

    /// One layout on its own, as a document stores it but without its
    /// pictures (`layout_record`, the layout fingerprint).
    pub(crate) fn layout_only(document: &'a Document) -> Self {
        Self {
            sheet: false,
            ..Self::without_contents(document)
        }
    }

    fn body(&self) -> Body<'_> {
        Body::new(self.document, self.anchors, self.layout_names)
    }
}

/// A frame record's place and the values its placement gives.
pub(crate) struct Record {
    /// 0 for a page frame (`[frm]`), 1 for a layout frame (`[lyfrm]`).
    pub depth: usize,
    pub rect: Rect,
    /// The flags the placement and the content give, without the kept
    /// ones.
    pub flags: u32,
    pub field1: String,
    /// Fields 14, 15, and 16 of an anchored frame.
    pub anchored: Option<[String; 3]>,
}

/// The kept flag bits the model does not interpret.
pub(crate) fn kept_flags(frame: &Frame) -> u32 {
    frame
        .legacy
        .get("frame.flags.unknown")
        .and_then(|value| u32::from_str_radix(value.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0)
}

/// The flags of a frame's look and content.
pub(crate) fn style_and_content_flags(document: &Document, frame: &Frame) -> u32 {
    let mut flags = if frame.style.opaque { OPAQUE } else { 0 };
    // Ami Pro draws a frame's lines only with this bit (its reader sets the
    // sides to none without it).
    if frame.style.border.sides != amipro_document::Sides::NONE {
        flags |= crate::build::LINES_ON;
    }
    flags |= match &frame.content {
        FrameContent::Text(_) if without_content(document, frame) => 0,
        FrameContent::Text(_) => HAS_TEXT,
        FrameContent::Table(_) => TABLE,
        FrameContent::Picture(_) => PICTURE,
        // A picture whose bytes the file did not have, kept as read.
        FrameContent::Legacy(kept) if kept.get("isd").is_some() => PICTURE,
        FrameContent::Legacy(_) => 0,
    };
    flags
}

/// Writes a frame record, its parts, and its content.
pub(crate) fn write_frame(
    out: &mut Lines,
    context: &Context<'_>,
    frame: &Frame,
    record: Record,
    sequence: &mut Sequence,
) {
    let style = &frame.style;
    let legacy = &frame.legacy;
    let kept_number = |key: &str| {
        legacy
            .get(key)
            .and_then(|value| value.trim().parse::<i64>().ok())
    };
    let kept = kept_flags(frame);
    let mut flags = record.flags | kept;
    if context.sheet {
        flags |= LAYOUT_FRAME;
    }
    // The writer clears 0x400 (`11c8:2a0f`); a frame that is the style
    // sheet's (0x8) loses its text and its picture, with the picture's
    // bits, unless the call keeps them (a sheet with contents).
    flags &= !NEVER_WRITTEN;
    let from_sheet = flags & LAYOUT_FRAME != 0 && !context.sheet_frames_text;
    if from_sheet || !context.contents {
        flags &= !SHEET_PICTURE_BITS;
    }
    let depth = record.depth;
    let values = depth + 1;
    out.line(
        depth,
        if depth == 0 {
            b"[frm]".as_slice()
        } else {
            b"[lyfrm]".as_slice()
        },
    );
    out.line(values, record.field1.trim().as_bytes());
    out.number(values, flags.into());
    let rect = record.rect;
    for value in [rect.left, rect.top, rect.right, rect.bottom] {
        out.number(values, unsigned(value.into()));
    }
    out.number(
        values,
        frame_side_mask(style.border.sides, kept_number("frame.field7")),
    );
    out.line(
        values,
        legacy.get("frame.field8").unwrap_or("1").trim().as_bytes(),
    );
    out.number(
        values,
        kept_number("frame.line-position").unwrap_or(style.border.position as i64),
    );
    let shadow = style.shadow;
    let corners = kept_number("frame.corners").unwrap_or(style.corners.into());
    out.line(
        values,
        format!(
            "{} {} {} {} {} {} {}",
            style.border.style,
            shadow.left,
            shadow.top,
            shadow.right,
            shadow.bottom,
            corners,
            shadow.colour
        )
        .as_bytes(),
    );
    out.number(values, style.border.colour.into());
    out.number(values, style.background.into());
    let number = sequence.of(frame);
    out.line(values, number.trim().as_bytes());
    let number = number.trim().to_owned();
    if let Some(fields) = &record.anchored {
        for field in fields {
            out.line(values, field.trim().as_bytes());
        }
    }
    if let Some(extra) = legacy.get("frame.extra") {
        for value in extra.split('\n') {
            out.line(values, value.trim().as_bytes());
        }
    }
    // The parts, in Ami Pro's order or the order the file had when it had
    // parts the model does not know.
    let usual = ["frmmac", "frmname", "frmlay", "content"];
    let kept_parts: Option<Vec<String>> = legacy.get("frame.parts").map(|parts| {
        parts
            .split_whitespace()
            .map(|part| match part.to_ascii_lowercase().as_str() {
                "txt" | "tbl" | "isd" => "content".to_owned(),
                _ => part.to_owned(),
            })
            .collect()
    });
    let order: Vec<String> = kept_parts.unwrap_or_else(|| usual.map(String::from).to_vec());
    let mut written_content = false;
    for part in &order {
        match part.to_ascii_lowercase().as_str() {
            "frmmac" => {
                if let Some(name) = &style.macro_name {
                    out.line(1, b"[frmmac]");
                    out.text(2, name);
                }
            }
            "frmname" => {
                if let Some(name) = &style.name {
                    out.line(1, b"[frmname]");
                    out.text(2, name);
                }
            }
            "frmlay" => {
                if let Some(raw) = legacy.get("section [frmlay]") {
                    out.line(1, b"[frmlay]");
                    for value in raw.split('\n') {
                        out.line(2, value.as_bytes());
                    }
                    continue;
                }
                out.line(1, b"[frmlay]");
                let derived = frame_columns(rect, style);
                let edges = crate::sheet::written_edges(
                    &style.interior,
                    &derived,
                    legacy.get("frmlay.edges"),
                );
                layout_block(
                    out,
                    Extent {
                        first: i64::from(rect.bottom) + kept_number("frmlay.1").unwrap_or(0),
                        width: rect.width().into(),
                        top: i64::from(rect.top) + i64::from(style.interior.margins.top),
                    },
                    &style.interior,
                    None,
                    &edges,
                    legacy,
                    "frmlay",
                    if depth == 0 {
                        crate::sheet::PAGE_FRAME_DEFAULTS
                    } else {
                        FRAME_DEFAULTS
                    },
                );
            }
            "content" => {
                if !written_content {
                    write_content(out, context, frame, from_sheet, &number);
                    written_content = true;
                }
            }
            _ => {
                let key = format!("section [{part}]");
                if let Some(raw) = legacy
                    .values
                    .iter()
                    .find(|value| value.key.eq_ignore_ascii_case(&key))
                    .map(|value| value.value.as_str())
                {
                    out.line(1, format!("[{part}]").as_bytes());
                    for value in crate::sheet::section_values(raw) {
                        out.line(2, value.as_bytes());
                    }
                }
            }
        }
    }
    if !written_content {
        write_content(out, context, frame, from_sheet, &number);
    }
}

/// A frame's content: its text, table, or picture.
fn write_content(
    out: &mut Lines,
    context: &Context<'_>,
    frame: &Frame,
    from_sheet: bool,
    number: &str,
) {
    let pictures = context.contents && !from_sheet;
    match &frame.content {
        FrameContent::Text(_) if without_content(context.document, frame) => {}
        FrameContent::Text(flow) => {
            out.line(1, b"[txt]");
            // A frame from the style sheet has its text there (`11c8:2a0f`),
            // and one never typed in has none.
            if !from_sheet && !never_typed_in(context.document, frame, *flow) {
                context.body().flow(&mut out.0, *flow);
            }
            out.line(0, b">");
        }
        // Tables are written whatever the frame's marks (`11c8:2a0f` →
        // `1490:0000`).
        FrameContent::Table(table) => write_table(out, context, table),
        FrameContent::Picture(picture) if pictures => {
            if picture.kept_is_current() && picture.kept_adjust_is_current() {
                write_kept_isd(out, &picture.legacy);
            } else {
                write_isd(out, picture, number);
            }
        }
        FrameContent::Legacy(kept) if pictures => write_kept_isd(out, kept),
        _ => {}
    }
}

/// A picture's description (`[isd]`), as read.
fn write_kept_isd(out: &mut Lines, kept: &amipro_document::LegacyData) {
    if let Some(values) = kept.get("isd") {
        out.line(1, b"[isd]");
        for value in values.split('\n') {
            out.line(2, value.as_bytes());
        }
    }
}

/// The values of a picture's `[isd]` record that the model does not type
/// (`11c8:2e29`): the third (the stored copy's name, the linked path, or
/// empty for a pasted picture), the fifth (the dialog's unit), the
/// driver's saved state with the driver's type, and the second state block.
/// `None` when nothing was kept or the values do not read.
pub(crate) fn kept_isd_parts(picture: &Picture) -> Option<KeptIsd> {
    let values: Vec<&str> = picture.legacy.get("isd")?.split('\n').collect();
    picture.kept_scaling()?;
    let count: usize = values.get(11)?.trim().parse().ok()?;
    let driver = 12usize.checked_add(count)?;
    if !values.get(driver)?.starts_with('.') {
        // An older version's record: its state is not Ami Pro 3.1's.
        return None;
    }
    Some(KeptIsd {
        stored: values.get(2)?.trim().to_owned(),
        unit: values.get(4)?.trim().to_owned(),
        state: values[11..=driver]
            .iter()
            .map(|value| value.trim().to_owned())
            .collect(),
        second: values
            .get(driver + 3..)
            .unwrap_or_default()
            .iter()
            .map(|value| value.trim().to_owned())
            .collect(),
    })
}

/// What a picture with no kept record is written with: its copy's name or
/// its path, unit 1 (inches), an empty driver state with its type, and an
/// empty second block (PIR1, PIR3).
pub(crate) fn new_isd_parts(picture: &Picture) -> KeptIsd {
    KeptIsd {
        stored: match &picture.source {
            PictureSource::Linked(path) => path.clone(),
            PictureSource::Embedded(_) => ".X".into(),
        },
        unit: "1".into(),
        state: if picture.is_gray_tiff() {
            tgf_state(&picture.adjust)
        } else {
            vec!["0".into(), picture.extension.to_ascii_lowercase()]
        },
        second: vec!["0".into()],
    }
}

/// The driver state of a `.tgf` frame: the count, the words, the type.
fn tgf_state(adjust: &amipro_document::ImageAdjust) -> Vec<String> {
    let mut state = vec![amipro_document::TGF_STATE_WORDS.to_string()];
    state.extend(adjust.tgf_words().iter().map(u16::to_string));
    state.push(".tgf".into());
    state
}

/// See [`kept_isd_parts`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeptIsd {
    pub stored: String,
    pub unit: String,
    /// The count, the state's words, and the driver's type.
    pub state: Vec<String>,
    /// The second block's count and words; empty in older records.
    pub second: Vec<String>,
}

/// A picture's `[isd]` record from the model (`11c8:2e29`), for a picture
/// whose kept values no longer say its scaling or pan, or that has none: the
/// kept parts the model does not type are written as read, so a metafile
/// keeps the driver's record of its size; a new picture has an empty
/// driver state, which Ami Pro fills in when it opens the file (PIR1,
/// PIR3).
fn write_isd(out: &mut Lines, picture: &Picture, number: &str) {
    let mut kept = kept_isd_parts(picture).unwrap_or_else(|| new_isd_parts(picture));
    if picture.is_gray_tiff() {
        // Ami Pro re-applies the effects from this state when it opens the
        // file (IMV1).
        kept.state = tgf_state(&picture.adjust);
    }
    let name = format!(".X{number}");
    let stored = match &picture.source {
        PictureSource::Linked(path) => path.clone(),
        PictureSource::Embedded(_) if kept.stored.is_empty() => String::new(),
        PictureSource::Embedded(_) => name.clone(),
    };
    let scaling = &picture.scaling;
    let mut values = vec![
        name,
        stored_extension(picture),
        stored,
        scaling.flags().to_string(),
        kept.unit.clone(),
        "0".into(),
        "0".into(),
        unsigned(scaling.size.width.into()).to_string(),
        unsigned((-scaling.size.height).into()).to_string(),
        scaling.percent.to_string(),
        scaling.rotation.to_string(),
    ];
    values.extend(kept.state.iter().cloned());
    values.push(unsigned(picture.pan.x.into()).to_string());
    values.push(unsigned((-picture.pan.y).into()).to_string());
    if kept.second.is_empty() {
        values.push("0".into());
    } else {
        values.extend(kept.second);
    }
    out.line(1, b"[isd]");
    for value in values {
        out.line(2, value.as_bytes());
    }
}

/// The picture type Ami Pro's files carry for a picture. A chart the app
/// makes keeps `.sct` in memory as its marker, but Ami Pro 3.1 finds the
/// library for a chart frame only under `.sdw` (CHV1: `.sct` gives "Cannot
/// find a library for frame"); its own files carry `.sdw` as the type and
/// `.sct` as the driver after the saved state.
pub(crate) fn stored_extension(picture: &Picture) -> String {
    if picture.extension.eq_ignore_ascii_case(".sct") {
        ".sdw".into()
    } else {
        picture.extension.clone()
    }
}

/// Whether a frame had no content part at all (drawings stored as `[mf]`)
/// and still holds nothing: it is written without one.
fn without_content(document: &Document, frame: &Frame) -> bool {
    frame.legacy.get("frame.content") == Some("none")
        && matches!(frame.content, FrameContent::Text(flow) if empty_flow(document, flow))
}

/// A flow that is one empty paragraph with nothing on it.
pub(crate) fn empty_flow(document: &Document, flow: FlowId) -> bool {
    document.flow(flow).is_some_and(|flow| {
        matches!(&flow.paragraphs[..], [paragraph]
            if paragraph.text.is_empty()
                && paragraph.marks.is_empty()
                && paragraph.format == Default::default()
                && paragraph.end_format == Default::default())
    })
}

/// Whether a frame's text is still what a `[txt]` without paragraphs gave:
/// one empty paragraph with nothing on it. Ami Pro writes such a frame's
/// text as no paragraphs.
pub(crate) fn never_typed_in(document: &Document, frame: &Frame, flow: FlowId) -> bool {
    frame.legacy.get("txt") == Some("none") && empty_flow(document, flow)
}

// ----- Tables -----

/// The length of a body's prefix that a calculated cell's formula body
/// repeats: the style tag (`@Name@`, a name that is no function call) and an
/// alignment tag (`<+A>`).
pub(crate) fn prefix_of(body: &str) -> usize {
    let mut end = 0;
    if body.starts_with('@')
        && let Some(close) = body[1..].find('@')
        && !body[1..1 + close].contains(['(', ')', '<', '>', '+', '-', '*', '/', ':', '='])
    {
        end = close + 2;
    }
    let rest = &body.as_bytes()[end..];
    if rest.len() >= 4 && rest.starts_with(b"<+") && rest[3] == b'>' {
        end += 4;
    }
    end
}

/// A calculated cell's formula from its record and bodies: F3 bit `0x40`,
/// the second body being the first's prefix and then the formula's text
/// (`TABLE_DATA_NOTES.md` section 2). The bodies the model does not read as
/// that stay where they are, as the cell's other bodies. Returns the
/// remaining bodies, the formula, and its prefix when it is not the first
/// body's.
pub(crate) fn split_formula(
    record: &str,
    first: Option<&String>,
    extra: &[String],
) -> (Vec<String>, Option<String>) {
    let calculated = numbers(record).get(2).is_some_and(|f3| f3 & 0x40 != 0);
    let (Some(first), Some(second)) = (first, extra.first()) else {
        return (extra.to_vec(), None);
    };
    if !calculated {
        return (extra.to_vec(), None);
    }
    let expected = &first[..prefix_of(first)];
    // An untagged paragraph that starts with `@` writes it twice, so that
    // the text is not taken for a style tag: `<+A>@@sum(A1..A4)`.
    let formula = second.strip_prefix(expected).and_then(|rest| {
        let rest = if expected.starts_with('@') {
            (prefix_of(second) == expected.len()).then_some(rest)?
        } else if rest.starts_with("@@") {
            &rest[1..]
        } else if prefix_of(second) == expected.len() && !rest.starts_with('@') {
            rest
        } else {
            return None;
        };
        (!rest.is_empty()
            && rest.is_ascii()
            && !rest.contains(['<', '>', '\n', '\r'])
            && !rest.starts_with(' '))
        .then_some(rest)
    });
    match formula {
        Some(formula) => (extra[1..].to_vec(), Some(formula.to_owned())),
        None => (extra.to_vec(), None),
    }
}

/// Numbers as Ami Pro writes a table's records: each after a space.
pub(crate) fn spaced(values: &[i64]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| format!(" {value}").into_bytes())
        .collect()
}

fn numbers(text: &str) -> Vec<i64> {
    text.split_whitespace()
        .map(|value| value.parse().unwrap_or(0))
        .collect()
}

/// Whether a cell holds one right-aligned paragraph that reads as a number
/// under its style.
fn right_aligned_number(document: &Document, flow: FlowId) -> bool {
    let Some([paragraph]) = document.flow(flow).map(|flow| flow.paragraphs.as_slice()) else {
        return false;
    };
    paragraph.format.alignment == Some(amipro_document::Alignment::Right)
        && document
            .styles()
            .get(&paragraph.style)
            .is_some_and(|style| style.kept.number_format.scan(&paragraph.text).is_some())
}

/// A table (`[tbl]` … `[tble]`; `SAM_PAGE_FRAME_SPEC.md` §4.7): its header,
/// the row and column records for the sizes that differ from the header's,
/// each group closed by `[e]`, then each cell's record and text.
fn write_table(out: &mut Lines, context: &Context<'_>, table: &Table) {
    out.line(1, b"[tbl]");
    write_table_head(out, table);
    out.line(2, b"[data]");
    let cell_frames = cell_frame_numbers(context.document);
    // Which formula results are REF (F3 bit 0x2000): worked out for the
    // write only; reading never calculates (Ami Pro keeps stored results).
    let outcomes = if table.cells.iter().any(|cell| cell.formula.is_some()) {
        let (dims, inputs) = amipro_document::formula::table_inputs(context.document, table);
        let options = amipro_document::formula::Options {
            extras: true,
            ..Default::default()
        };
        amipro_document::formula::calculate(dims, &inputs, options, &|_, value| value)
    } else {
        Vec::new()
    };
    for (index, cell) in table.cells.iter().enumerate() {
        // A cell the file left out (never typed in) stays out while it is
        // empty, holds no frame, and has no lines, fill, leader, or
        // protection of its own (a grid drawn before it is typed in).
        let CellContent::Text(flow) = cell.content;
        if cell.legacy.is_empty()
            && cell.formula.is_none()
            && cell.span == CellSpan::SINGLE
            && cell.frame.is_none()
            && cell.format.is_default()
            && empty_flow(context.document, flow)
        {
            continue;
        }
        let mut record = cell_record(context.document, table, cell, index, &cell_frames);
        if outcomes
            .get(index)
            .copied()
            .flatten()
            .is_some_and(|outcome| outcome.is_not_a_number())
        {
            record[2] |= amipro_document::formula::NOT_A_NUMBER_BIT;
        }
        out.line(3, &spaced(&record));
        if record[7] != 0 {
            let first = out.0.len();
            if cell.formula.is_some() && empty_flow(context.document, flow) {
                // An empty result (divide by zero, a loop, REF) is the style
                // tag and `<+A>` alone, never a bare `@Style@` line, which
                // reads back as two paragraphs.
                let name = context
                    .document
                    .flow(flow)
                    .and_then(|flow| context.document.styles().get(&flow.paragraphs[0].style))
                    .map_or(crate::build::TABLE_TEXT, |style| style.name.as_str());
                if !name.eq_ignore_ascii_case(crate::build::TABLE_TEXT) {
                    out.0.extend(format!("@{name}@").bytes());
                }
                out.0.extend(b"<+A>\r\n\r\n");
            } else {
                context
                    .body()
                    .flow_untagged(&mut out.0, flow, crate::build::TABLE_TEXT);
            }
            out.line(0, b">");
            // A calculated cell's second body: the result's style tag and
            // alignment, then the formula as typed.
            if let Some(formula) = &cell.formula {
                let prefix = match cell.legacy.get("cell.formula.prefix") {
                    Some(kept) => kept.to_owned(),
                    None => {
                        let text: String = out.0[first..].iter().map(|b| char::from(*b)).collect();
                        text[..prefix_of(&text)].to_owned()
                    }
                };
                out.0.extend(prefix.bytes());
                if !prefix.starts_with('@') && formula.starts_with('@') {
                    out.0.push(b'@');
                }
                out.0.extend(formula.chars().map(|c| {
                    if c.is_ascii() {
                        c as u8
                    } else {
                        amipro_document::windows_1252(c)
                    }
                }));
                out.0.extend(b"\r\n\r\n");
                out.line(0, b">");
            }
        }
        for body in cell
            .legacy
            .values
            .iter()
            .filter(|value| value.key == "cell.body")
        {
            for paragraph in body.value.split("\n\n") {
                out.0.extend(paragraph.as_bytes());
                out.0.extend(b"\r\n\r\n");
            }
            out.line(0, b">");
        }
    }
    out.line(2, b"[e]");
    out.line(1, b"[tble]");
}

/// The top-left cell (row, column) of the joined block covering cell
/// `index`.
fn covering_block(table: &Table, index: usize) -> Option<(usize, usize)> {
    let columns = table.columns.len().max(1);
    let (row, column) = (index / columns, index % columns);
    table.cells.iter().enumerate().find_map(|(owner, cell)| {
        let (top, left) = (owner / columns, owner % columns);
        (!cell.span.is_covered()
            && (top..top + usize::from(cell.span.rows)).contains(&row)
            && (left..left + usize::from(cell.span.columns)).contains(&column))
        .then_some((top, left))
    })
}

/// A table's header values and its `[h]` and `[w]` records (`1490:0000`).
fn write_table_head(out: &mut Lines, table: &Table) {
    let kept = &table.legacy;
    let rows = table.rows.len();
    let columns = table.columns.len();
    let mut header = kept
        .get("tbl.header")
        .map(numbers)
        .unwrap_or_else(|| vec![0, 0, 0, 86, 1_440, 86, 1, 43, 43]);
    // Older headers have seven values; the six the model sets are there.
    header.resize(header.len().max(6), 0);
    header[0] = rows as i64;
    header[1] = columns as i64;
    header[3] = table.cell_margins.vertical.into();
    header[5] = table.cell_margins.horizontal.into();
    // Values 8 and 9, the insets, which a header without them reads as 43.
    let spacing = &table.spacing;
    if header.len() >= 9
        || (spacing.top, spacing.left) != (TableSpacing::INSET, TableSpacing::INSET)
    {
        header.resize(header.len().max(9), 0);
        header[7] = spacing.top.into();
        header[8] = spacing.left.into();
    }
    // Value 7: Automatic (1) and Rows span pages (0x100), from the model;
    // a header without it reads as a new table's.
    let options = &table.row_options;
    if header.len() >= 7 || *options != TableRows::default() {
        header.resize(header.len().max(7), 0);
        header[6] = (header[6] & !0x101)
            | i64::from(options.automatic)
            | if options.span_pages { 0x100 } else { 0 };
    }
    // Values 3 and 5, the default row height and column width, and value
    // 7's options (0x4 Center, 0x80 Honor protection, 0x400 Disable mouse
    // sizing; VM TE13, TE12, TE15) from the model.
    let layout = &table.layout;
    header[2] = layout.row_height.into();
    header[4] = layout.column_width.into();
    if header.len() >= 7 || layout.center || layout.honor_protection || layout.fixed_sizes {
        header.resize(header.len().max(7), 0);
        header[6] = (header[6] & !0x484)
            | if layout.center { 0x4 } else { 0 }
            | if layout.honor_protection { 0x80 } else { 0 }
            | if layout.fixed_sizes { 0x400 } else { 0 };
    }
    let default_height = header[2];
    let default_width = header[4];
    // The rows and columns a joined block of more than one row or column
    // crosses carry flag 1 (`1458:041b`; VM TE8).
    let mut joined_rows = BTreeSet::new();
    let mut joined_columns = BTreeSet::new();
    for (index, cell) in table.cells.iter().enumerate() {
        let (row, column) = (index / columns.max(1), index % columns.max(1));
        if cell.span.rows > 1 {
            joined_rows.extend(row..row + usize::from(cell.span.rows));
        }
        if cell.span.columns > 1 {
            joined_columns.extend(column..column + usize::from(cell.span.columns));
        }
    }
    out.line(2, &spaced(&header));
    // A record is index, size, gutter, flags, 0 (and for rows two more, as
    // Ami Pro writes them: `1428:046b` reads five values or fails); the
    // gutter is the row's or column's own with flag 4.
    for (records, sizes, default, gutters, own_sizes, joined, section) in [
        (
            kept.get("tbl.h"),
            &table.rows,
            default_height,
            &spacing.row_gutters,
            &layout.own_heights,
            &joined_rows,
            "[h]",
        ),
        (
            kept.get("tbl.w"),
            &table.columns,
            default_width,
            &spacing.column_gutters,
            &layout.own_widths,
            &joined_columns,
            "[w]",
        ),
    ] {
        let default_gutter = if section == "[h]" {
            table.cell_margins.vertical
        } else {
            table.cell_margins.horizontal
        };
        let mut by_index: BTreeMap<usize, Vec<i64>> = records
            .map(|records| {
                records
                    .split('\n')
                    .map(numbers)
                    .filter_map(|values| Some((usize::try_from(*values.first()?).ok()?, values)))
                    .collect()
            })
            .unwrap_or_default();
        for (index, size) in sizes.iter().enumerate() {
            let size = i64::from(*size);
            let own = gutters.get(&index).copied();
            // A row's heading (0x10) and page break (0x20) flags.
            let marks = if section == "[h]" {
                (if options.headings.contains(&index) {
                    0x10
                } else {
                    0
                }) | if options.page_breaks.contains(&index) {
                    0x20
                } else {
                    0
                }
            } else {
                0
            };
            // Flag 2, a size of its own (TE5). Flag 1, inside a joined block
            // (TE8), Ami Pro's reader sets for itself (`1458:041b`): it goes
            // only into records written here, so records read keep theirs.
            let own_size = own_sizes.contains(&index) || size != default;
            // An Automatic table's rows Ami Pro measured, so every row it
            // writes has the flag (TE2–TE7).
            let measured = own_size || (section == "[h]" && options.automatic);
            let in_block = joined.contains(&index);
            match by_index.get_mut(&index) {
                Some(values) if values.len() > 1 => {
                    values[1] = size;
                    if values.len() >= 4 {
                        if let Some(gutter) = own {
                            values[2] = gutter.into();
                            values[3] |= 4;
                        } else {
                            values[3] &= !4;
                        }
                        if section == "[h]" {
                            values[3] = (values[3] & !0x30) | marks;
                        }
                        // A record read keeps flag 2 as Ami Pro wrote it;
                        // a size of its own in the model sets it.
                        if own_sizes.contains(&index) {
                            values[3] |= 2;
                        }
                    }
                }
                _ if own_size || own.is_some() || marks != 0 => {
                    // Index, size, gutter, flags, 0; a row also the number of
                    // cells it starts (TE3, TE8; read past, `1428:046b`) and 0.
                    let mut values = vec![
                        index as i64,
                        size,
                        own.unwrap_or(default_gutter).into(),
                        marks,
                        0,
                    ];
                    if section == "[h]" {
                        let starts = (0..columns)
                            .filter(|column| {
                                table
                                    .cells
                                    .get(index * columns + column)
                                    .is_some_and(|cell| !cell.span.is_covered())
                            })
                            .count();
                        values.extend([starts as i64, 0]);
                    }
                    if own.is_some() {
                        values[3] |= 4;
                    }
                    values[3] |= if measured { 2 } else { 0 } | if in_block { 1 } else { 0 };
                    by_index.insert(index, values);
                }
                _ => {}
            }
        }
        // Records beyond the grid (a column since deleted) stay as read.
        if by_index.is_empty() {
            continue;
        }
        out.line(2, section.as_bytes());
        for values in by_index.values() {
            out.line(3, &spaced(values));
        }
        out.line(2, b"[e]");
    }
}

/// A cell's record (`1490:07c0`): the values read, with the cell's place,
/// its join bits, and whether text follows (the eighth value) from the
/// model.
pub(crate) fn cell_record(
    document: &Document,
    table: &Table,
    cell: &amipro_document::TableCell,
    index: usize,
    cell_frames: &BTreeMap<FrameId, usize>,
) -> Vec<i64> {
    let columns = table.columns.len();
    let CellContent::Text(flow) = cell.content;
    let (row, column) = (index / columns.max(1), index % columns.max(1));
    let mut record = cell
        .legacy
        .get("cell.record")
        .map(numbers)
        .unwrap_or_else(|| vec![0; 12]);
    record.resize(record.len().max(5), 0);
    // F3 bit 0x10: a number aligned right (VM TDT1–TDT4 store it with the
    // `<+A>` code when the caret leaves a typed number).
    // A calculated cell with an empty result (divide by zero, a loop, REF)
    // is written as `<+A>` alone and stored with the bit as well (VM TDF1:
    // 0x50).
    if right_aligned_number(document, flow)
        || (cell.formula.is_some()
            && document.flow(flow).is_some_and(
                |flow| matches!(&flow.paragraphs[..], [paragraph] if paragraph.text.is_empty()),
            ))
    {
        record[2] |= 0x10;
    }
    record[0] = row as i64;
    record[1] = column as i64;
    // Field 3's join bits (`build.rs`): 0x100 with 0x80 starts a block
    // of field 4 rows by field 5 columns; 0x80 alone is covered, fields
    // 4 and 5 then its offset from the block's first cell.
    let joined = 0x100 | 0x80;
    record[2] &= !joined;
    // F3 bit 0x40: a formula follows the result as a second body. A body
    // the model does not read as a formula keeps the bit as read.
    record[2] &= !0x40;
    if cell.formula.is_some() {
        // Set by the writer when the result is REF.
        record[2] &= !amipro_document::formula::NOT_A_NUMBER_BIT;
    }
    if cell.formula.is_some()
        || cell.legacy.get("cell.body").is_some()
            && numbers(cell.legacy.get("cell.record").unwrap_or_default())
                .get(2)
                .is_some_and(|f3| f3 & 0x40 != 0)
        || cell.legacy.get("cell.formula.missing").is_some()
    {
        record[2] |= 0x40;
    }
    if cell.span.is_covered() {
        record[2] |= 0x80;
        // Fields 4 and 5: the offset from the block's top-left cell, which
        // Ami Pro's reader follows (VM TE8).
        if let Some((top, left)) = covering_block(table, index) {
            record[3] = (row - top) as i64;
            record[4] = (column - left) as i64;
        }
    } else if cell.span != CellSpan::SINGLE {
        record[2] |= joined;
        record[3] = cell.span.rows.into();
        record[4] = cell.span.columns.into();
    }
    // The eighth value says whether text follows: Ami Pro writes none for
    // a cell never typed in, and a covered cell, and refuses a file whose
    // text disagrees with it. A cell that now holds text, or anything but
    // one empty Table Text paragraph, writes its text.
    let default_text = document.flow(flow).is_some_and(|text| {
        empty_flow(document, flow)
            && document
                .styles()
                .get(&text.paragraphs[0].style)
                .is_some_and(|style| style.name.eq_ignore_ascii_case(crate::build::TABLE_TEXT))
    });
    record.resize(record.len().max(8), 0);
    if !default_text || cell.formula.is_some() {
        record[7] = 1;
    }
    // The cell's look (`docs/research/TABLE_EDIT_NOTES.md`): the leader and
    // the fill's bit in F3, the fill's place in Ami Pro's fill palette in F6
    // (`1490:0d0a`, from F10, which an unfilled cell keeps), the lines in
    // F7, protection in F9, the fill's colour in F10.
    let format = &cell.format;
    if !format.is_default() || record.len() >= 10 {
        record.resize(record.len().max(12), 0);
        record[2] = (record[2] & !0xC200)
            | i64::from(format.leader.bits())
            | if format.fill.is_some() { 0x200 } else { 0 };
        if let Some(fill) = format.fill {
            record[9] = fill.into();
        }
        record[5] = fill_palette_index(record[9]);
        record[6] = format.lines.word().into();
        record[8] = i64::from(format.protected);
    }
    // Values 11 and 12: whether the cell holds a frame, and which, by its
    // place in Ami Pro's list (`1490:07c0`); a cell whose frame was not
    // found keeps what it said.
    match cell.frame.and_then(|frame| cell_frames.get(&frame)) {
        Some(number) => {
            record.resize(record.len().max(12), 0);
            record[10] = 1;
            record[11] = *number as i64;
        }
        None if cell.legacy.get("cell.frame.unresolved").is_none() && record.len() >= 12 => {
            record[10] = 0;
            record[11] = 0;
        }
        None => {}
    }
    record
}

/// A fill colour's place in Ami Pro's old fill palette, the cell record's
/// F6 (`1490:0d0a`): black 0, the greys 1–4, blue, red, green, yellow, and
/// 11 for any other colour.
fn fill_palette_index(colour: i64) -> i64 {
    match colour & 0xFF_FFFF {
        0 => 0,
        0x3F_3F3F => 1,
        0x7F_7F7F => 2,
        0xBF_BFBF => 3,
        0xF7_F7F7 => 4,
        0xFF_0000 => 5,
        0x00_00FF => 6,
        0x00_FF00 => 8,
        0x00_FFFF => 9,
        _ => 11,
    }
}

/// Drops the kept values a frame read back with that the writer writes the
/// same without (the rule of 4.5's layouts, for anchored frames and
/// tables): a frame made in the app, written and read again, says what it
/// said, while values Ami Pro wrote otherwise stay.
pub(crate) fn drop_derived_values(document: &mut Document) {
    const ANCHORED_KEYS: [&str; 3] = ["frame.anchored.rect", "frame.field15", "frame.field16"];
    const TABLE_KEYS: [&str; 3] = ["tbl.header", "tbl.h", "tbl.w"];
    let without = |legacy: &amipro_document::LegacyData, keys: &[&str]| {
        let mut bare = legacy.clone();
        bare.values
            .retain(|value| !keys.contains(&value.key.as_str()));
        bare
    };
    let anchors = anchor_numbers(document);
    let cell_frames = cell_frame_numbers(document);
    let mut changes = Vec::new();
    {
        let context = Context {
            document,
            contents: true,
            sheet: false,
            sheet_frames_text: false,
            layout_names: &NO_LAYOUT_NAMES,
            anchors: &anchors,
        };
        for &id in document.frame_stack() {
            let Some(frame) = document.frame(id) else {
                continue;
            };
            let mut changed = frame.clone();
            if frame.placement.is_anchored() {
                let bare = Frame {
                    legacy: without(&frame.legacy, &ANCHORED_KEYS),
                    ..frame.clone()
                };
                let (kept, derived) = (
                    page_frame_record(&context, frame, 0),
                    page_frame_record(&context, &bare, 0),
                );
                if kept.rect == derived.rect && kept.anchored == derived.anchored {
                    changed.legacy = bare.legacy;
                }
            }
            if let FrameContent::Table(table) = &mut changed.content {
                let bare = Table {
                    legacy: without(&table.legacy, &TABLE_KEYS),
                    ..table.clone()
                };
                let head = |table: &Table| {
                    let mut out = Lines::default();
                    write_table_head(&mut out, table);
                    out.0
                };
                if head(&*table) == head(&bare) {
                    table.legacy = bare.legacy;
                }
                let shape = table.clone();
                for (index, cell) in table.cells.iter_mut().enumerate() {
                    let CellContent::Text(flow) = cell.content;
                    // An empty cell's record is what keeps it in the file,
                    // as it is for a cell whose frame is deleted later
                    // (Ami Pro writes its record then, TF7).
                    if cell.frame.is_some()
                        || (cell.span == CellSpan::SINGLE && empty_flow(document, flow))
                    {
                        continue;
                    }
                    let bare = amipro_document::TableCell {
                        legacy: without(&cell.legacy, &["cell.record"]),
                        ..cell.clone()
                    };
                    if cell_record(document, &shape, cell, index, &cell_frames)
                        == cell_record(document, &shape, &bare, index, &cell_frames)
                    {
                        cell.legacy = bare.legacy;
                    }
                }
            }
            if changed != *frame {
                changes.push(changed);
            }
        }
    }
    for frame in changes {
        if let Some(slot) = document.frame_mut(frame.id) {
            *slot = frame;
        }
    }
}

// ----- Page frames -----

/// Each anchored page frame's number in the main text's marks: its place
/// among the anchored frames in the stack, tables included (Z05, V20).
pub(crate) fn anchor_numbers(document: &Document) -> BTreeMap<FrameId, usize> {
    document
        .frame_stack()
        .iter()
        .filter(|id| {
            document
                .frame(**id)
                .is_some_and(|frame| frame.placement.is_anchored() || without_mark(frame))
        })
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect()
}

/// The page frames in the order Ami Pro keeps and writes them
/// (`11c8:3ba0`): its lists, those that are not anchored first, then the
/// anchored ones (records without a mark included), each back to front
/// (FC6N, FC7; every file of both corpora but one). Anchored frames are
/// drawn below the others whatever this order (FC8).
pub(crate) fn page_frames_in_file_order(document: &Document) -> Vec<FrameId> {
    let mut order = cell_frame_list(document);
    let (others, anchored): (Vec<FrameId>, Vec<FrameId>) = document
        .frame_stack()
        .iter()
        .filter(|id| !order.contains(id))
        .partition(|id| {
            document
                .frame(**id)
                .is_none_or(|frame| !frame.placement.is_anchored() && !without_mark(frame))
        });
    order.extend(others);
    order.extend(anchored);
    order
}

/// Ami Pro's list of frames in table cells, which it writes first
/// (`11c8:3ba0`): the stack's frames in cells, and Where placed frames read
/// with the bit that no cell named, in stack order, which is the file's
/// (the newest first: `14d0:0000` puts a frame at the list's front).
pub(crate) fn cell_frame_list(document: &Document) -> Vec<FrameId> {
    document
        .frame_stack()
        .iter()
        .copied()
        .filter(|id| {
            document.frame(*id).is_some_and(|frame| {
                frame.placement.is_in_cell()
                    || (matches!(frame.placement, Placement::Fixed { .. })
                        && kept_flags(frame) & IN_CELL != 0)
            })
        })
        .collect()
}

/// Each frame in a cell's place in Ami Pro's list, as cell records name it
/// (value 12, `1490:0708`).
pub(crate) fn cell_frame_numbers(document: &Document) -> BTreeMap<FrameId, usize> {
    cell_frame_list(document)
        .into_iter()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect()
}

/// The page frames, in Ami Pro's order (`11c8:3ba0`).
pub(crate) fn write_page_frames(out: &mut Lines, context: &Context<'_>, sequence: &mut Sequence) {
    let document = context.document;
    let stack = page_frames_in_file_order(document);
    let group_of = |index: usize| {
        stack
            .get(index)
            .and_then(|id| document.frame(*id))
            .and_then(|frame| frame.group)
    };
    for (index, id) in stack.iter().enumerate() {
        let Some(frame) = document.frame(*id) else {
            continue;
        };
        if let FrameContent::Legacy(kept) = &frame.content
            && let Some(raw) = kept.get("section [frm]")
        {
            // A record the importer could not read, written as it was.
            out.line(0, b"[frm]");
            for line in raw.split('\n') {
                out.0.extend(line.as_bytes());
                out.0.extend(b"\r\n");
            }
            continue;
        }
        let mut flags = style_and_content_flags(document, frame);
        if let Some(group) = frame.group {
            flags |= GROUP_MEMBER;
            if index == 0 || group_of(index - 1) != Some(group) {
                flags |= GROUP_FIRST;
            }
            if group_of(index + 1) != Some(group) {
                flags |= GROUP_LAST;
            }
        }
        let record = page_frame_record(context, frame, flags);
        write_frame(out, context, frame, record, sequence);
    }
}

fn page_frame_record(context: &Context<'_>, frame: &Frame, mut flags: u32) -> Record {
    let legacy = &frame.legacy;
    let field1 = || legacy.get("frame.field1").unwrap_or("1").to_owned();
    let wrap_flags = |wrap: Wrap| match wrap {
        Wrap::Around => WRAP_AROUND,
        Wrap::NoWrapBeside => NO_WRAP_BESIDE,
        Wrap::NoWrapAround => 0,
    };
    match frame.placement {
        // An anchored record without a mark in the text, which the
        // importer placed where its rectangle was: written as read, an
        // anchored frame that keeps its place in the anchor numbering.
        Placement::Fixed { page, rect } if without_mark(frame) => {
            flags |= ANCHORED | wrap_flags(frame.style.wrap);
            let field16 = legacy.get("frame.field16").map_or_else(
                || format!("{} {}", rect.width(), rect.height()),
                str::to_owned,
            );
            Record {
                depth: 0,
                rect,
                flags,
                field1: page.to_string(),
                anchored: Some([
                    anchor_field(context, frame),
                    legacy.get("frame.field15").unwrap_or("0").to_owned(),
                    field16,
                ]),
            }
        }
        Placement::Fixed { page, rect } => {
            flags |= wrap_flags(frame.style.wrap);
            Record {
                depth: 0,
                rect,
                flags,
                field1: page.to_string(),
                anchored: None,
            }
        }
        Placement::BelowParagraph { x_offset, size } => {
            flags |= ANCHORED;
            if frame.style.wrap == Wrap::NoWrapBeside {
                flags |= NO_WRAP_BESIDE;
            }
            let table = matches!(frame.content, FrameContent::Table(_));
            let rect = anchored_rect(context, frame, size.width, size.height, x_offset);
            // Field 15 is the offset from the margin, but a stale value for
            // tables (MERCURY.SAM); field 16's height is stale.
            let field15 = if table {
                legacy.get("frame.field15").unwrap_or("0").to_owned()
            } else {
                x_offset.to_string()
            };
            // The importer takes field 16's width when it has one (`build.rs`).
            let field16 = kept_if_same(legacy.get("frame.field16"), |kept| {
                table
                    || kept
                        .first()
                        .is_some_and(|width| *width <= 0 || *width == i64::from(size.width))
            })
            .unwrap_or_else(|| {
                if table {
                    "0 65535".to_owned()
                } else {
                    format!("{} {}", size.width, size.height)
                }
            });
            Record {
                depth: 0,
                rect,
                flags,
                field1: field1(),
                anchored: Some([anchor_field(context, frame), field15, field16]),
            }
        }
        Placement::Inline { y_offset, size } => {
            flags |= ANCHORED | WRAP_AROUND;
            let table = matches!(frame.content, FrameContent::Table(_));
            let rect = anchored_rect(context, frame, size.width, size.height, 0);
            // Field 15 is where layout put the frame (a cache), field 16
            // the width and the distance from the top to the baseline.
            let baseline = size.height - y_offset;
            // The importer takes field 16's width when it has one, and its
            // baseline distance when it is positive (else the frame stands
            // on the baseline), `build.rs`.
            let field16 = kept_if_same(legacy.get("frame.field16"), |kept| {
                table
                    || match kept[..] {
                        [width, distance, ..] => {
                            (width <= 0 || width == i64::from(size.width))
                                && if distance > 0 {
                                    distance == i64::from(baseline)
                                } else {
                                    y_offset == 0
                                }
                        }
                        _ => false,
                    }
            })
            .unwrap_or_else(|| format!("{} {}", size.width, baseline));
            Record {
                depth: 0,
                rect,
                flags,
                field1: field1(),
                anchored: Some([
                    anchor_field(context, frame),
                    legacy.get("frame.field15").unwrap_or("0").to_owned(),
                    field16,
                ]),
            }
        }
        // Layout frames are written with their layout (`sheet.rs`).
        Placement::Repeating { rect, .. } => Record {
            depth: 0,
            rect,
            flags,
            field1: field1(),
            anchored: None,
        },
        Placement::Header { .. } | Placement::Footer { .. } => Record {
            depth: 0,
            rect: Rect::new(0, 0, 1, 1),
            flags,
            field1: field1(),
            anchored: None,
        },
        Placement::InCell => {
            flags |= IN_CELL | wrap_flags(frame.style.wrap);
            let (page, rect) = cell_frame_place(context.document, frame);
            Record {
                depth: 0,
                rect,
                flags,
                field1: page,
                anchored: None,
            }
        }
    }
}

/// A frame in a cell's page and rectangle: as read while its size is still
/// its cell's text area, else that size at the corner read (Ami Pro takes
/// the page and corner from the cell when it lays the table out, and keeps
/// the size it reads, TF1Y; T2: the frame is its cell's size); a frame not
/// read from a file at its cell's corner, from where its table was read or
/// stands.
fn cell_frame_place(document: &Document, frame: &Frame) -> (String, Rect) {
    let holder = document.cell_holding(frame.id).and_then(|(table, cell)| {
        let table_frame = document.frame(table)?;
        let FrameContent::Table(content) = &table_frame.content else {
            return None;
        };
        Some((table_frame, content.cell_text_area(cell)?))
    });
    let area = holder.map(|(_, area)| area);
    let kept: Vec<i64> = frame
        .legacy
        .get("frame.cell.rect")
        .map(numbers)
        .unwrap_or_default();
    if let [page, left, top, right, bottom] = kept[..] {
        let (left, top) = (left as Twips, top as Twips);
        let stored = Rect::new(left, top, right as Twips, bottom as Twips);
        let rect = match area {
            Some(area) if (stored.width(), stored.height()) != (area.width(), area.height()) => {
                Rect::new(left, top, left + area.width(), top + area.height())
            }
            _ => stored,
        };
        return (page.to_string(), rect);
    }
    let Some((table_frame, area)) = holder else {
        return ("1".to_owned(), Rect::new(0, 0, 1, 1));
    };
    let (page, corner) = match table_frame.placement {
        Placement::Fixed { page, rect } => (page.to_string(), (rect.left, rect.top)),
        _ => {
            let kept: Vec<i64> = table_frame
                .legacy
                .get("frame.anchored.rect")
                .map(numbers)
                .unwrap_or_default();
            let corner = match kept[..] {
                [left, top, ..] => (left as Twips, top as Twips),
                _ => (document.document_layout().right.layout.margins.left, 0),
            };
            let page = table_frame.legacy.get("frame.field1").unwrap_or("1");
            (page.to_owned(), corner)
        }
    };
    let margins = table_frame.style.interior.margins;
    let (left, top) = (
        corner.0 + margins.left + area.left,
        corner.1 + margins.top + area.top,
    );
    (
        page,
        Rect::new(left, top, left + area.width(), top + area.height()),
    )
}

/// The size the importer reads from a stored rectangle of `width` by
/// `height` and the kept field 16 (`build.rs`): a table's is the
/// rectangle; otherwise field 16's width when it has one, and the
/// rectangle's height unless the rectangle is degenerate.
fn read_size(frame: &Frame, width: i64, height: i64) -> (i64, i64) {
    // An empty or inverted rectangle is read one twip wide or tall.
    let (width, height) = (width.max(1), height.max(1));
    if matches!(frame.content, FrameContent::Table(_)) && width > 0 && height > 0 {
        return (width, height);
    }
    let field16 = frame
        .legacy
        .get("frame.field16")
        .map(numbers)
        .unwrap_or_default();
    let width = field16
        .first()
        .copied()
        .filter(|width| *width > 0)
        .unwrap_or(width);
    let height = match frame.placement {
        Placement::BelowParagraph { .. } if height <= 1 => field16
            .get(1)
            .copied()
            .filter(|height| *height > 0)
            .unwrap_or(height),
        _ => height,
    };
    (width, height)
}

/// Whether a frame is an anchored record that had no mark in the text.
fn without_mark(frame: &Frame) -> bool {
    frame.legacy.get("frame.anchored-without-mark").is_some()
}

/// A kept value, when `same` says it still holds.
fn kept_if_same(kept: Option<&str>, same: impl Fn(&[i64]) -> bool) -> Option<String> {
    let kept = kept?;
    same(&numbers(kept)).then(|| kept.to_owned())
}

fn anchor_field(context: &Context<'_>, frame: &Frame) -> String {
    context
        .anchors
        .get(&frame.id)
        .copied()
        .unwrap_or(0)
        .to_string()
}

/// An anchored frame's stored rectangle: Ami Pro places the frame from its
/// anchor and uses only the size (X02), so the rectangle read is written
/// back while its size is the frame's, and keeps its corner otherwise; a
/// new frame stands at the margin plus its offset.
fn anchored_rect(
    context: &Context<'_>,
    frame: &Frame,
    width: Twips,
    height: Twips,
    x_offset: Twips,
) -> Rect {
    let kept: Vec<i64> = frame
        .legacy
        .get("frame.anchored.rect")
        .map(numbers)
        .unwrap_or_default();
    if let [left, top, right, bottom] = kept[..] {
        let (left, top) = (left as Twips, top as Twips);
        if read_size(frame, right - i64::from(left), bottom - i64::from(top))
            == (i64::from(width), i64::from(height))
        {
            return Rect::new(left, top, right as Twips, bottom as Twips);
        }
        return Rect::new(left, top, left + width, top + height);
    }
    let margin = context.document.document_layout().right.layout.margins.left;
    let left = margin + x_offset;
    Rect::new(left, 0, left + width, height)
}

#[cfg(test)]
mod tests {
    use amipro_document::FrameContent;

    use crate::roundtrip::{Difference, differences, is_writer_addition, read_back};
    use crate::{SamOptions, import_sam, sam_document};

    const OPTIONS: SamOptions = SamOptions::at(1_791_958_088);

    /// A frame record: fields 1–13 (and 14–16 when anchored), its layout,
    /// and its content.
    fn record(fields: &[&str], content: &str) -> String {
        let mut out = String::from("[frm]\r\n");
        for field in fields {
            out.push_str(&format!("\t{field}\r\n"));
        }
        out.push_str(
            "\t[frmlay]\r\n\t\t4168\r\n\t\t2880\r\n\t\t1\r\n\t\t144\r\n\t\t144\r\n\t\t1\r\n\t\t2152\r\n\t\t144\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\t\t3959\r\n\t\t6437\r\n\t\t0\r\n",
        );
        out.push_str(content);
        out
    }

    fn beyond_additions(original: &amipro_document::Document) -> Vec<Difference> {
        let copy = read_back(original, &OPTIONS).unwrap();
        differences(original, &copy)
            .into_iter()
            .filter(|difference| {
                !is_writer_addition(difference) && difference.aspect != "face kinds"
            })
            .collect()
    }

    #[test]
    fn page_frames_are_written_as_ami_pro_writes_them() {
        let text = "\t[txt]\r\nFrame text.\r\n\r\n>\r\n";
        let look = ["1", "1", "1", "1 0 0 100 100 18 0", "0", "16777215"];
        let fixed = |first: u32, page: &str, left: &str, number: &str| {
            let mut fields = vec![page, "", left, "2880", "6000", "5040"];
            fields.extend(look);
            fields.push(number);
            let flags = (0x2C0 | first).to_string();
            fields[1] = &flags;
            record(&fields, text)
        };
        let anchored = |flags: &str, number: &str, field15: &str, field16: &str| {
            let mut fields = vec!["1", flags, "3758", "2008", "6638", "4168"];
            fields.extend(look);
            fields.extend([number, "0", field15, field16]);
            record(&fields, text)
        };
        let table = "\t[tbl]\r\n\t\t 1 2 284 86 2000 86 1 43 43\r\n\t\t[data]\r\n\t\t\t 0 0 0 0 0 0 0 1 0 0 0 0\r\nA1\r\n\r\n>\r\n\t\t\t 0 1 0 0 0 0 0 1 0 0 0 0\r\n@Body Text@B1\r\n\r\n>\r\n\t\t[e]\r\n\t[tble]\r\n";
        let table_frame = {
            let mut fields = vec!["1", "1573060", "1440", "2292", "5755", "2724"];
            fields.extend(look);
            fields.extend(["7", "0", "0", "0 65535"]);
            record(&fields, table)
        };
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             {}{}{}{}{}{}\
             [edoc]\r\nOne<:A0>\r\n\r\nTwo <:A1>end<:t2>\r\n\r\n>\r\n",
            // Two grouped Where placed frames on page 2.
            fixed(0x180_0000, "2", "1440", "3"),
            fixed(0x280_0000, "2", "7000", "4"),
            // With para above, Flow with text, an anchored table, and an
            // anchored record without a mark, which keeps its number.
            anchored("590400", "5", "2318", "2880 2160"),
            anchored("590528", "6", "1790", "2880 0"),
            table_frame,
            anchored("590400", "8", "0", "2880 2160"),
        );
        let document = import_sam(sam.as_bytes(), None, "Frames").unwrap().document;
        assert_eq!(document.frame_stack().len(), 6);
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        // The group's first and last members, anchors in the shared order,
        // and the stored values given back.
        assert!(written.contains("[frm]\r\n\t2\r\n\t25166528\r\n\t1440\r\n"));
        assert!(written.contains("[frm]\r\n\t2\r\n\t41943744\r\n\t7000\r\n"));
        assert!(written.contains("One<:A0>\r\n\r\nTwo <:A1>end<:t2>\r\n"));
        assert!(written.contains("\t5\r\n\t0\r\n\t2318\r\n\t2880 2160\r\n"));
        assert!(written.contains("\t6\r\n\t1\r\n\t1790\r\n\t2880 0\r\n"));
        assert!(written.contains("\t8\r\n\t3\r\n\t0\r\n\t2880 2160\r\n"));
        assert!(written.contains("\t[tbl]\r\n\t\t 1 2 284 86 2000 86 1 43 43\r\n\t\t[data]\r\n"));
        assert!(
            written.contains("A1\r\n\r\n>\r\n\t\t\t 0 1 0 0 0 0 0 1 0 0 0 0\r\n@Body Text@B1\r\n")
        );
        assert_eq!(beyond_additions(&document), []);
    }

    #[test]
    fn a_frames_lines_count_only_with_the_lines_bit() {
        // F01's frame (0x102C0: lines on, All) and the same record without
        // 0x10000: Ami Pro's reader takes no sides then, whatever field 7
        // says, and the writer gives field 7 back as read.
        let sam = |flags: u32| {
            let fields = [
                "1",
                &flags.to_string(),
                "2160",
                "2880",
                "5040",
                "5040",
                "1",
                "1",
                "1",
                "1 0 0 100 100 18 0",
                "0",
                "16777215",
                "3",
            ]
            .map(str::to_owned);
            let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
            format!(
                "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n{}[edoc]\r\nText\r\n\r\n>\r\n",
                record(&fields, "\t[txt]\r\nFrame.\r\n\r\n>\r\n")
            )
        };
        for (flags, sides) in [
            (0x102C0, amipro_document::Sides::ALL),
            (0x2C0, amipro_document::Sides::NONE),
        ] {
            let document = import_sam(sam(flags).as_bytes(), None, "Lines")
                .unwrap()
                .document;
            let frame = document.frame(document.frame_stack()[0]).unwrap();
            assert_eq!(frame.style.border.sides, sides, "{flags:#x}");
            let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
            assert!(
                written.contains(&format!(
                    "[frm]\r\n\t1\r\n\t{flags}\r\n\t2160\r\n\t2880\r\n\t5040\r\n\t5040\r\n\t1\r\n"
                )),
                "{flags:#x}: {written}"
            );
            assert_eq!(beyond_additions(&document), []);
        }

        // A frame made with lines writes the bit; without lines, neither.
        let mut document = import_sam(sam(0x2C0).as_bytes(), None, "Lines")
            .unwrap()
            .document;
        let id = document.frame_stack()[0];
        let frame = document.frame_mut(id).unwrap();
        frame.legacy = Default::default();
        frame.style.border.sides = amipro_document::Sides::ALL;
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert!(written.contains("[frm]\r\n\t1\r\n\t66240\r\n"), "{written}");
        document.frame_mut(id).unwrap().style.border.sides = amipro_document::Sides::NONE;
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert!(
            written.contains(
                "[frm]\r\n\t1\r\n\t704\r\n\t2160\r\n\t2880\r\n\t5040\r\n\t5040\r\n\t0\r\n"
            ),
            "{written}"
        );
    }

    #[test]
    fn a_cell_writes_text_exactly_when_its_record_says_it_has_some() {
        // Ami Pro refuses a file whose cell record says no text (eighth
        // field 0) but has text after it (README31's empty cells).
        let table = "\t[tbl]\r\n\t\t 1 2 284 86 2000 86 1 43 43\r\n\t\t[data]\r\n\
            \t\t\t 0 0 0 0 0 0 0 0 0 0 1 2\r\n\
            \t\t\t 0 1 0 0 0 0 0 1 0 0 0 0\r\nB1\r\n\r\n>\r\n\
            \t\t[e]\r\n\t[tble]\r\n";
        let frame = format!(
            "[frm]\r\n\t1\r\n\t1573060\r\n\t1440\r\n\t2292\r\n\t5755\r\n\t2724\r\n\t1\r\n\t1\r\n\t1\r\n\t1 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t3\r\n\t0\r\n\t0\r\n\t0 65535\r\n\
             \t[frmlay]\r\n\t\t2860\r\n\t\t9353\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t2292\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t1\r\n\t\t1584\r\n\t\t10656\r\n\t\t0\r\n{table}"
        );
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             {frame}[edoc]\r\nText<:t0>\r\n\r\n>\r\n"
        );
        let cell_warning = |warnings: &[String]| {
            warnings
                .iter()
                .any(|warning| warning.starts_with("table cell record"))
        };
        let imported = import_sam(sam.as_bytes(), None, "Table").unwrap();
        assert!(!cell_warning(&imported.report.warnings));
        let mut document = imported.document;
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let start = written.find("\t[tbl]").unwrap();
        let end = written.find("\t[tble]").unwrap() + "\t[tble]\r\n".len();
        assert_eq!(&written[start..end], table);

        // Text typed into the empty cell: its record now says so.
        let id = document.frame_stack()[0];
        let amipro_document::FrameContent::Table(cells) = &document.frame(id).unwrap().content
        else {
            panic!("a table frame");
        };
        let amipro_document::CellContent::Text(flow) = cells.cells[0].content;
        document
            .flow_mut(flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "A1".into();
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert!(written.contains("\t\t\t 0 0 0 0 0 0 0 1 0 0 1 2\r\nA1\r\n\r\n>\r\n"));
        let copy = import_sam(written.as_bytes(), None, "Table").unwrap();
        assert!(!cell_warning(&copy.report.warnings));

        // A record that disagrees with its text is reported.
        let wrong = sam.replace(" 0 1 0 0 0 0 0 1 0 0 0 0", " 0 1 0 0 0 0 0 0 0 0 0 0");
        let report = import_sam(wrong.as_bytes(), None, "Table").unwrap().report;
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("has no text, but text follows")),
            "{:?}",
            report.warnings
        );
    }

    #[test]
    fn tables_keep_their_records_joins_omitted_cells_and_formulas() {
        // A 2 × 3 table: a row record and column records (one for a column
        // since deleted), cells 0,0 and 0,1 joined, cells 0,2 and 1,2 joined
        // with the covered one keeping its own text, cell 1,1 left out, and
        // a formula after cell 1,0.
        let table = "\t[tbl]\r\n\t\t 2 3 284 86 2000 86 5\r\n\
            \t\t[h]\r\n\t\t\t 1 500 86 6 0 3 0\r\n\t\t[e]\r\n\
            \t\t[w]\r\n\t\t\t 0 1500 126 7 0\r\n\t\t\t 3 900 86 3 0\r\n\t\t[e]\r\n\
            \t\t[data]\r\n\
            \t\t\t 0 0 384 1 2 0 0 1 0 0 0 0\r\nJoined\r\n\r\n>\r\n\
            \t\t\t 0 1 128 0 1 0 0 0 0 0 0 0\r\n\
            \t\t\t 0 2 384 2 1 0 0 1 0 0 0 0\r\n@Body Text@Tagged\r\n\r\n>\r\n\
            \t\t\t 1 0 0 0 0 0 0 1 0 0 0 0\r\n<+A>12\r\n\r\n>\r\n<+A>a1*2\r\n\r\n>\r\n\
            \t\t\t 1 2 128 1 0 0 0 1 0 0 0 0\r\n@Body Text@\r\n>\r\n\
            \t\t[e]\r\n\t[tble]\r\n";
        let frame = format!(
            "[frm]\r\n\t1\r\n\t1573060\r\n\t1440\r\n\t2292\r\n\t5755\r\n\t2724\r\n\t1\r\n\t1\r\n\t1\r\n\t1 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t3\r\n\t0\r\n\t0\r\n\t0 65535\r\n\
             \t[frmlay]\r\n\t\t2860\r\n\t\t9353\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t2292\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t1\r\n\t\t1584\r\n\t\t10656\r\n\t\t0\r\n{table}"
        );
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             {frame}[edoc]\r\nText<:t0>\r\n\r\n>\r\n"
        );
        let document = import_sam(sam.as_bytes(), None, "Table").unwrap().document;
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let start = written.find("\t[tbl]").unwrap();
        let end = written.find("\t[tble]").unwrap() + "\t[tble]\r\n".len();
        // A right-aligned number gets F3 bit 0x10 (TD.5, VM TDT1–TDT4).
        assert_eq!(
            &written[start..end],
            table.replace(" 1 0 0 0 0 0 0 1 0 0 0 0", " 1 0 16 0 0 0 0 1 0 0 0 0")
        );
        assert_eq!(beyond_additions(&document), []);
    }

    /// Ami Pro's grid (`CELL_FRAME_NOTES.md`): header values 8 and 9 are the
    /// insets, and a `[h]` or `[w]` record's gutter is its row's or
    /// column's own only with flag 4 (READMEBP's rows, VM BPS); both are
    /// written back as read, and a record the model needs is written in
    /// Ami Pro's full shape (its reader takes five values or fails).
    /// `TABLE_EDIT_NOTES.md`: a cell's leader, protection, lines, and fill,
    /// and the table's options and defaults, read from the records Ami Pro
    /// wrote (VM TE10, TE11, TE13, TE12, TE15, TF0X) and written as it writes
    /// them; Save As lists only a line colour (TD8).
    #[test]
    fn cell_formats_and_table_options_are_read_and_written_as_ami_pro_writes_them() {
        let table = "\t[tbl]\r\n\t\t 2 2 284 86 3017 86 1157 43 43\r\n\
            \t\t[data]\r\n\
            \t\t\t 0 0 16384 0 0 0 0 1 1 0 0 0\r\nA\r\n\r\n>\r\n\
            \t\t\t 0 1 512 0 0 0 4369 1 0 0 0 0\r\nB\r\n\r\n>\r\n\
            \t\t\t 1 0 0 0 0 6 0 1 0 255 0 0\r\nC\r\n\r\n>\r\n\
            \t\t[e]\r\n\t[tble]\r\n";
        let frame = format!(
            "[frm]\r\n\t1\r\n\t1573060\r\n\t1440\r\n\t2292\r\n\t7689\r\n\t3075\r\n\t1\r\n\t1\r\n\t1\r\n\t1 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t3\r\n\t0\r\n\t0\r\n\t0 65535\r\n\
             \t[frmlay]\r\n\t\t783\r\n\t\t6249\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t2292\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t1\r\n\t\t1584\r\n\t\t10656\r\n\t\t0\r\n{table}"
        );
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             {frame}[edoc]\r\nText<:t0>\r\n\r\n>\r\n"
        );
        let mut document = import_sam(sam.as_bytes(), None, "Table").unwrap().document;
        let id = document.frame_stack()[0];
        let FrameContent::Table(read) = &document.frame(id).unwrap().content else {
            panic!("a table");
        };
        // 1157 = 0x485: Automatic, Center (TE13), Honor protection (TE12),
        // Disable mouse sizing (TE15).
        let layout = &read.layout;
        assert!(layout.center && layout.honor_protection && layout.fixed_sizes);
        assert_eq!((layout.row_height, layout.column_width), (284, 3017));
        let formats: Vec<_> = read.cells.iter().map(|cell| cell.format).collect();
        assert_eq!(formats[0].leader, amipro_document::CellLeader::Dots);
        assert!(formats[0].protected);
        assert_eq!(formats[1].fill, Some(0), "black: the bit alone (TF0X)");
        assert_eq!(formats[1].lines.word(), 0x1111);
        assert_eq!(formats[2].fill, None, "a colour without the bit (TF0Y)");
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let start = written.find("\t[tbl]").unwrap();
        let end = written.find("\t[tble]").unwrap() + "\t[tble]\r\n".len();
        assert_eq!(&written[start..end], table);
        assert_eq!(beyond_additions(&document), []);

        // Changed in the model: no options, a yellow fill on A (palette 9,
        // TL4), dashes on B, C protected and its colour kept.
        let FrameContent::Table(edited) = &mut document.frame_mut(id).unwrap().content else {
            unreachable!()
        };
        edited.layout.center = false;
        edited.layout.honor_protection = false;
        edited.layout.fixed_sizes = false;
        edited.cells[0].format.fill = Some(0x00_FFFF);
        edited.cells[1].format.leader = amipro_document::CellLeader::Dashes;
        edited.cells[1].format.fill = None;
        edited.cells[2].format.protected = true;
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        for record in [
            "\t\t 2 2 284 86 3017 86 1 43 43\r\n",
            "\t\t\t 0 0 16896 0 0 9 0 1 1 65535 0 0\r\n",
            "\t\t\t 0 1 32768 0 0 0 4369 1 0 0 0 0\r\n",
            "\t\t\t 1 0 0 0 0 6 0 1 1 255 0 0\r\n",
        ] {
            assert!(written.contains(record), "{record:?} in {written}");
        }
        assert_eq!(beyond_additions(&document), []);

        // A line colour is the app's own: Save As writes black and lists it.
        let FrameContent::Table(coloured) = &mut document.frame_mut(id).unwrap().content else {
            unreachable!()
        };
        coloured.cells[1].format.line_colour = 0x00_00FF;
        let losses = crate::sam_losses(&document, &OPTIONS).unwrap();
        assert_eq!(
            losses
                .iter()
                .map(|loss| (loss.kind, loss.count))
                .collect::<Vec<_>>(),
            [(crate::LossKind::TableLineColours, 1)]
        );
    }

    #[test]
    fn tables_keep_their_insets_and_own_gutters() {
        let table = "\t[tbl]\r\n\t\t 2 2 284 230 2000 86 1 300 0\r\n\
            \t\t[h]\r\n\t\t\t 0 284 72 2 0 1 0\r\n\t\t\t 1 284 140 6 0 1 0\r\n\t\t[e]\r\n\
            \t\t[w]\r\n\t\t\t 0 2000 28 6 0\r\n\t\t[e]\r\n\
            \t\t[data]\r\n\
            \t\t\t 0 0 0 0 0 0 0 1 0 0 0 0\r\nA\r\n\r\n>\r\n\
            \t\t[e]\r\n\t[tble]\r\n";
        let frame = format!(
            "[frm]\r\n\t1\r\n\t1573060\r\n\t1440\r\n\t2292\r\n\t5755\r\n\t3456\r\n\t1\r\n\t1\r\n\t1\r\n\t1 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t3\r\n\t0\r\n\t0\r\n\t0 65535\r\n\
             \t[frmlay]\r\n\t\t3456\r\n\t\t4315\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t2292\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t1\r\n\t\t1584\r\n\t\t10656\r\n\t\t0\r\n{table}"
        );
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             {frame}[edoc]\r\nText<:t0>\r\n\r\n>\r\n"
        );
        let mut document = import_sam(sam.as_bytes(), None, "Table").unwrap().document;
        let id = document.frame_stack()[0];
        let FrameContent::Table(read) = &document.frame(id).unwrap().content else {
            panic!("a table");
        };
        assert_eq!((read.spacing.top, read.spacing.left), (300, 0));
        assert_eq!(read.row_gutter(0), 230, "72 without flag 4: the table's");
        assert_eq!(read.row_gutter(1), 140);
        assert_eq!(read.column_gutter(0), 28);
        assert_eq!(read.column_gutter(1), 86);
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let start = written.find("\t[tbl]").unwrap();
        let end = written.find("\t[tble]").unwrap() + "\t[tble]\r\n".len();
        assert_eq!(&written[start..end], table);
        assert_eq!(beyond_additions(&document), []);

        // A column given a gutter of its own and a row a new height get
        // records as Ami Pro writes them, and read back so.
        let FrameContent::Table(edited) = &mut document.frame_mut(id).unwrap().content else {
            unreachable!()
        };
        edited.spacing.column_gutters.insert(1, 50);
        edited.spacing.row_gutters.remove(&1);
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert!(
            written.contains("\t\t\t 1 284 140 2 0 1 0\r\n"),
            "{written}"
        );
        // A new column record with a gutter of its own has flag 4 alone, as
        // Ami Pro's Column/Row Size writes it (VM TE6).
        assert!(written.contains("\t\t\t 1 2000 50 4 0\r\n"), "{written}");
        let again = import_sam(written.as_bytes(), None, "Table")
            .unwrap()
            .document;
        let FrameContent::Table(back) = &again.frame(again.frame_stack()[0]).unwrap().content
        else {
            panic!("a table");
        };
        assert_eq!(back.spacing, edited_spacing(&document, id));
    }

    /// Header value 7's Automatic (1) and Rows span pages (0x100) and a row
    /// record's heading (0x10) and page break (0x20) flags (VM TP2, TP5,
    /// TP6, TP2E): read into the model, written back as read, and written
    /// as Ami Pro writes them when the model changes them.
    #[test]
    fn tables_keep_automatic_rows_span_pages_headings_and_page_breaks() {
        let table = "\t[tbl]\r\n\t\t 3 2 284 86 2000 86 257 43 43\r\n\
            \t\t[h]\r\n\t\t\t 0 284 86 18 0 2 0\r\n\t\t\t 2 284 86 34 0 2 0\r\n\t\t[e]\r\n\
            \t\t[data]\r\n\
            \t\t\t 0 0 0 0 0 0 0 1 0 0 0 0\r\nA\r\n\r\n>\r\n\
            \t\t[e]\r\n\t[tble]\r\n";
        let frame = format!(
            "[frm]\r\n\t1\r\n\t1573060\r\n\t1440\r\n\t2292\r\n\t5755\r\n\t3456\r\n\t1\r\n\t1\r\n\t1\r\n\t1 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t3\r\n\t0\r\n\t0\r\n\t0 65535\r\n\
             \t[frmlay]\r\n\t\t3456\r\n\t\t4315\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t2292\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t1\r\n\t\t1584\r\n\t\t10656\r\n\t\t0\r\n{table}"
        );
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             {frame}[edoc]\r\nText<:t0>\r\n\r\n>\r\n"
        );
        let mut document = import_sam(sam.as_bytes(), None, "Table").unwrap().document;
        let id = document.frame_stack()[0];
        let FrameContent::Table(read) = &document.frame(id).unwrap().content else {
            panic!("a table");
        };
        let options = &read.row_options;
        assert!(options.automatic && options.span_pages);
        assert_eq!(options.headings.iter().copied().collect::<Vec<_>>(), [0]);
        assert_eq!(options.page_breaks.iter().copied().collect::<Vec<_>>(), [2]);
        // Every row of an Automatic table is read as Ami Pro measured it
        // (the row cache), as it is now.
        assert_eq!(read.read_rows.len(), read.rows.len());
        for (row, print) in &read.read_rows {
            assert_eq!(*print, document.table_row_fingerprint(read, *row));
        }
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let start = written.find("\t[tbl]").unwrap();
        let end = written.find("\t[tble]").unwrap() + "\t[tble]\r\n".len();
        assert_eq!(&written[start..end], table);
        assert_eq!(beyond_additions(&document), []);

        // Changed in the model: Automatic off (0, TP2E), no Rows span pages,
        // row 0 no longer a heading, a page break on row 1 (a new record in
        // Ami Pro's shape) and none on row 2.
        let FrameContent::Table(edited) = &mut document.frame_mut(id).unwrap().content else {
            unreachable!()
        };
        edited.row_options.automatic = false;
        edited.row_options.span_pages = false;
        edited.row_options.headings.clear();
        edited.row_options.page_breaks = [1].into();
        let edited_options = edited.row_options.clone();
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert!(
            written.contains("\t\t 3 2 284 86 2000 86 0 43 43\r\n"),
            "{written}"
        );
        // A new row record counts the cells the row starts (TE3, TE8); a
        // row of a table without Automatic is not measured (no flag 2). The
        // other rows' records were Ami Pro's measures of an Automatic table,
        // derived at import, and are not needed now.
        assert!(
            written.contains("\t\t\t 1 284 86 32 0 2 0\r\n"),
            "{written}"
        );
        assert!(!written.contains("\t\t\t 0 284 "), "{written}");
        let again = import_sam(written.as_bytes(), None, "Table")
            .unwrap()
            .document;
        let FrameContent::Table(back) = &again.frame(again.frame_stack()[0]).unwrap().content
        else {
            panic!("a table");
        };
        assert_eq!(back.row_options, edited_options);
        // Without Automatic Ami Pro keeps each row's size: nothing to cache.
        assert!(back.read_rows.is_empty());
    }

    /// A picture frame in a table cell, as Ami Pro writes one (TF1: Import
    /// Picture with the caret in the cell, a linked picture).
    fn cell_frame(page: &str, rect: [i64; 4], sequence: u32) -> String {
        let [left, top, right, bottom] = rect;
        format!(
            "[frm]\r\n\t{page}\r\n\t536871136\r\n\t{left}\r\n\t{top}\r\n\t{right}\r\n\t{bottom}\r\n\t1\r\n\t1\r\n\t1\r\n\t0 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t{sequence}\r\n\
             \t[frmlay]\r\n\t\t{bottom}\r\n\t\t{width}\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t{top}\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t0\r\n\t\t0\r\n\t\t120\r\n\t\t1\r\n\t\t{left}\r\n\t\t{right}\r\n\t\t0\r\n\
             \t[isd]\r\n\t\t.X{sequence}\r\n\t\t.bmp\r\n\t\tL:\\IN\\PIC.BMP\r\n\t\t18\r\n",
            width = right - left
        )
    }

    /// A document with frames in cells before a 2 × 3 table of 3017-twip
    /// columns and 284-twip rows at 1440, 1724 (TF1's), whose cell records
    /// are `cells`.
    fn cells_document(frames: &[String], cells: &str) -> String {
        let table = format!(
            "[frm]\r\n\t1\r\n\t1573060\r\n\t1440\r\n\t1724\r\n\t10792\r\n\t2507\r\n\t1\r\n\t1\r\n\t1\r\n\t1 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t3\r\n\t0\r\n\t2560\r\n\t256 0\r\n\
             \t[frmlay]\r\n\t\t2292\r\n\t\t9352\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t1724\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t0\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t1584\r\n\t\t10656\r\n\t\t0\r\n\
             \t[tbl]\r\n\t\t 2 3 284 86 3017 86 1 43 43\r\n\t\t[data]\r\n{cells}\t\t[e]\r\n\t[tble]\r\n"
        );
        format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             {}{table}[edoc]\r\nBefore the table.\r\n\r\n<:t0>\r\n\r\n>\r\n",
            frames.concat()
        )
    }

    /// The frame records and the cell records of a written document.
    fn frame_and_cell_records(written: &str) -> (Vec<String>, Vec<String>) {
        let lines: Vec<&str> = written.split("\r\n").collect();
        let frames = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == "[frm]")
            .map(|(index, _)| {
                lines[index + 1..index + 7]
                    .iter()
                    .map(|line| line.trim())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let cells = lines
            .iter()
            .filter(|line| line.starts_with("\t\t\t "))
            .map(|line| line.trim().to_owned())
            .collect();
        (frames, cells)
    }

    /// `CELL_FRAME_NOTES.md`: a cell record's values 11 and 12 name the
    /// frame it holds, counting Ami Pro's frames in cells (flag `0x20`) in
    /// file order (`1428:07ce`); the frame is then in the cell, written
    /// first, with the bit and the cell's numbers given by the model.
    #[test]
    fn cells_hold_the_frames_their_records_name_tf1() {
        let frame = cell_frame("1", [4_586, 1_767, 7_603, 2_051], 4);
        let cells = "\t\t\t 0 1 0 0 0 0 0 0 0 0 1 0\r\n";
        let sam = cells_document(std::slice::from_ref(&frame), cells);
        let imported = import_sam(sam.as_bytes(), None, "TF1").unwrap();
        assert!(
            !imported
                .report
                .warnings
                .iter()
                .any(|warning| warning.contains("cell")),
            "{:?}",
            imported.report.warnings
        );
        let document = imported.document;
        let stack = document.frame_stack().to_vec();
        let picture = document.frame(stack[0]).unwrap();
        assert_eq!(picture.placement, amipro_document::Placement::InCell);
        assert_eq!(document.cell_holding(stack[0]), Some((stack[1], 1)));
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let start = written.find("[frm]").unwrap();
        assert!(
            written[start..].starts_with(&frame),
            "{}",
            &written[start..start + 400]
        );
        let (_, cells) = frame_and_cell_records(&written);
        assert_eq!(cells, ["0 1 0 0 0 0 0 0 0 0 1 0"]);
        assert_eq!(beyond_additions(&document), []);
    }

    /// README31's order: the newest frame first (`14d0:0000`), each cell
    /// naming its frame by place; deleting a frame in the app empties its
    /// cell and renumbers the cells after it.
    #[test]
    fn cell_frames_keep_ami_pros_order_and_numbers_readme31() {
        let frames = [
            cell_frame("1", [1_483, 2_097, 4_500, 2_381], 9),
            cell_frame("1", [4_586, 1_767, 7_603, 2_051], 8),
            cell_frame("1", [1_483, 1_767, 4_500, 2_051], 7),
        ];
        let cells = "\t\t\t 0 0 0 0 0 0 0 0 0 0 1 2\r\n\
            \t\t\t 0 1 0 0 0 0 0 0 0 0 1 1\r\n\
            \t\t\t 1 0 0 0 0 0 0 0 0 0 1 0\r\n";
        let sam = cells_document(&frames, cells);
        let mut document = import_sam(sam.as_bytes(), None, "Order").unwrap().document;
        let stack = document.frame_stack().to_vec();
        let table = stack[3];
        assert_eq!(document.cell_holding(stack[0]), Some((table, 3)));
        assert_eq!(document.cell_holding(stack[1]), Some((table, 1)));
        assert_eq!(document.cell_holding(stack[2]), Some((table, 0)));
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let (frame_records, cell_records) = frame_and_cell_records(&written);
        assert_eq!(frame_records.len(), 4);
        assert!(frame_records[0].contains("1483 2097"));
        assert_eq!(
            cell_records,
            [
                "0 0 0 0 0 0 0 0 0 0 1 2",
                "0 1 0 0 0 0 0 0 0 0 1 1",
                "1 0 0 0 0 0 0 0 0 0 1 0"
            ]
        );
        assert_eq!(beyond_additions(&document), []);

        // Delete the picture in cell (0,1).
        document.remove_frame(stack[1]).unwrap();
        document.validate().unwrap();
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let (frame_records, cell_records) = frame_and_cell_records(&written);
        assert_eq!(frame_records.len(), 3);
        assert_eq!(
            cell_records,
            [
                "0 0 0 0 0 0 0 0 0 0 1 1",
                "0 1 0 0 0 0 0 0 0 0 0 0",
                "1 0 0 0 0 0 0 0 0 0 1 0"
            ]
        );
        let again = import_sam(written.as_bytes(), None, "Order")
            .unwrap()
            .document;
        assert_eq!(again.frame_stack().len(), 3);
        assert_eq!(
            again.cell_holding(again.frame_stack()[1]),
            Some((again.frame_stack()[2], 0))
        );
    }

    /// T2: a frame Ami Pro never laid out (page 0, no height; ten in the
    /// user's corpus) is written at its cell's size, at the corner and page
    /// read; a frame with the bit that no cell names stays where its record
    /// put it, first in the file; a cell naming a frame that is not there
    /// keeps its numbers.
    #[test]
    fn never_sized_and_unnamed_cell_frames_tf1z() {
        let frame = cell_frame("0", [0, 0, 3_017, 0], 4);
        let cells = "\t\t\t 0 1 0 0 0 0 0 0 0 0 1 0\r\n";
        let document = import_sam(
            cells_document(std::slice::from_ref(&frame), cells).as_bytes(),
            None,
            "TF1Z",
        )
        .unwrap()
        .document;
        assert!(
            document
                .frame(document.frame_stack()[0])
                .unwrap()
                .placement
                .is_in_cell()
        );
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let (frame_records, _) = frame_and_cell_records(&written);
        assert_eq!(frame_records[0], "0 536871136 0 0 3017 284");
        assert_eq!(beyond_additions(&document), []);

        let unnamed = cells_document(
            std::slice::from_ref(&frame),
            "\t\t\t 0 1 0 0 0 0 0 1 0 0 0 0\r\nB1\r\n\r\n>\r\n\t\t\t 1 1 0 0 0 0 0 0 0 0 1 5\r\n",
        );
        let imported = import_sam(unnamed.as_bytes(), None, "Unnamed").unwrap();
        let document = imported.document;
        assert!(matches!(
            document.frame(document.frame_stack()[0]).unwrap().placement,
            amipro_document::Placement::Fixed { .. }
        ));
        assert!(
            imported
                .report
                .warnings
                .iter()
                .any(|w| w.contains("in no cell"))
        );
        assert!(
            imported
                .report
                .warnings
                .iter()
                .any(|w| w.contains("not there"))
        );
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let start = written.find("[frm]").unwrap();
        assert!(written[start..].starts_with("[frm]\r\n\t1\r\n\t536871136\r\n"));
        let (_, cells) = frame_and_cell_records(&written);
        assert_eq!(cells[1], "1 1 0 0 0 0 0 0 0 0 1 5");
        assert_eq!(beyond_additions(&document), []);
    }

    fn edited_spacing(
        document: &amipro_document::Document,
        id: amipro_document::FrameId,
    ) -> amipro_document::TableSpacing {
        let FrameContent::Table(table) = &document.frame(id).unwrap().content else {
            panic!("a table");
        };
        table.spacing.clone()
    }
}
