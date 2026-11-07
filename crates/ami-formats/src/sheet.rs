//! Writing Ami Pro style sheets (decision D3, Save as a Style Sheet): the
//! header sections, the styles, the standard page layout with its header,
//! footer, and repeating frames, and `[edoc]`, in the order and number
//! formats of Ami Pro 3.1's writer (decompile `11c8:14e0`, with the page
//! layout `11c8:236e`, its LAYOUT blocks `11c8:24d6`, and frame records
//! `11c8:2a0f`; VM fixtures SS1N, SS2N, SS2W). Values the model does not
//! hold come from what the importer kept (`build.rs`).
//!
//! A sheet is written without contents, as Ami Pro's dialog does with
//! "With contents" off: header and footer text, pictures, page frames, and
//! the main text are left out (SS2N against SS2W).

use std::collections::BTreeMap;

use amipro_document::{
    Border, Document, Frame, LayoutBox, LegacyData, PageGeometry, PageLayout, PageSide, Placement,
    Rect, RepeatPages, Sides, StyleOrigin, Twips, Wrap,
};

use crate::frames::{
    Context, NO_WRAP_BESIDE, Record, WRAP_AROUND, style_and_content_flags, write_frame,
};
use crate::writer::{Lines, windows_1252, write_style_record};

/// What Save as a Style Sheet asks for besides the document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SheetOptions {
    /// The description Ami Pro lists the sheet by (`[desc]` value 1).
    pub description: String,
    /// The save time in seconds since 1970, as Ami Pro's clock gives it
    /// (`[desc]` value 6).
    pub saved_at: u32,
    /// With contents (Ami Pro's default): the main text, the page frames,
    /// the header and footer text, and the pictures go into the sheet, and
    /// a new document made from it starts with them.
    pub contents: bool,
}

/// The document's styles and standard page layout as an Ami Pro style
/// sheet (`.STY`) without contents.
pub fn style_sheet(document: &Document, options: &SheetOptions) -> Vec<u8> {
    let shortened = crate::writer::shortened_style_names(document);
    let document = shortened.as_ref().unwrap_or(document);
    let mut out = Lines::default();
    header_sections(&mut out, document, options);
    // Ami Pro keeps styles in function-key order, styles without a key
    // last (every corpus file, 640 of 640). A name defined nowhere is not
    // a style Ami Pro has.
    let styles = document.styles();
    let mut ordered: Vec<_> = styles
        .values()
        .filter(|style| style.origin != StyleOrigin::Missing)
        .collect();
    ordered.sort_by_key(|style| {
        let key = style.kept.function_key;
        (key == 0, key, style.id)
    });
    for style in ordered {
        write_style_record(&mut out, style, styles);
    }
    // The standard layout only (`11c8:236e`), every frame marked as the
    // sheet's; with contents, then the page frames (each marked while it
    // is written), `[edoc]`, the main text, and the end with the pictures
    // of both (`11c8:14e0`, `11c8:3d76`).
    let mut sequence = Sequence::new(document);
    let layout = document.document_layout();
    let names: BTreeMap<_, _> = [(layout.id, layout.name.clone())].into();
    let anchors = if options.contents {
        crate::frames::anchor_numbers(document)
    } else {
        BTreeMap::new()
    };
    let context = Context {
        document,
        contents: options.contents,
        sheet: true,
        sheet_frames_text: options.contents,
        layout_names: &names,
        anchors: &anchors,
    };
    write_layout(&mut out, &context, layout, &layout.name, &mut sequence);
    out.line(0, b"[elay]");
    if options.contents {
        crate::frames::write_page_frames(&mut out, &context, &mut sequence);
    }
    out.line(0, b"[edoc]");
    if options.contents {
        crate::body::Body::new(document, &anchors, &names)
            .flow(&mut out.0, document.main_flow_id());
        let pictures = crate::sam::picture_frames(document, &[(layout, layout.name.clone())], true);
        crate::sam::write_end(&mut out, document, &pictures, &mut sequence);
    }
    out.0
}

/// The document's description (`[desc]` value 1), which Save as a Style
/// Sheet offers as the sheet's: empty when it has none.
pub fn document_description(document: &Document) -> String {
    document
        .legacy()
        .values
        .iter()
        .find(|value| value.key.eq_ignore_ascii_case("section [desc]"))
        .and_then(|value| section_values(&value.value).into_iter().next())
        .map(|value| crate::text::decode_name(&value))
        .unwrap_or_default()
}

/// One page layout as Ami Pro writes it, from `[lay]` to its last frame,
/// without contents.
pub fn layout_record(document: &Document, layout: &PageLayout) -> Vec<u8> {
    let mut out = Lines::default();
    write_layout(
        &mut out,
        &Context::layout_only(document),
        layout,
        &layout.name,
        &mut Sequence::new(document),
    );
    out.0
}

// ----- Header sections -----

/// The sections before the styles, in the order Ami Pro 3.1 writes them
/// for a style sheet (`11c8:14e0`). Each comes from the document's kept
/// section when it has one, else Ami Pro's value for a new document.
fn header_sections(out: &mut Lines, document: &Document, options: &SheetOptions) {
    let kept = |name: &str| -> Option<Vec<String>> {
        let key = format!("section [{name}]");
        document
            .legacy()
            .values
            .iter()
            .find(|value| value.key.eq_ignore_ascii_case(&key))
            .map(|value| section_values(&value.value))
    };
    // Kept values are as the file had them, escapes included: written
    // back in Windows-1252.
    let section = |out: &mut Lines, name: &str, values: &[String]| {
        out.line(0, format!("[{name}]").as_bytes());
        for value in values {
            let bytes: Vec<u8> = value.chars().map(windows_1252).collect();
            out.line(1, &bytes);
        }
    };
    let strings =
        |values: &[&str]| -> Vec<String> { values.iter().map(|value| value.to_string()).collect() };
    section(out, "ver", &strings(&["4"]));
    // Linked pictures; a sheet without contents has none (`11c8:07dd`).
    let files = if options.contents {
        crate::sam::linked_files(document)
    } else {
        Vec::new()
    };
    section(out, "files", &files);
    section(
        out,
        "charset",
        &kept("charset").unwrap_or_else(|| strings(&["82", "ANSI (Windows, IBM CP 1252)"])),
    );
    section(
        out,
        "revisions",
        &kept("revisions").unwrap_or_else(|| strings(&["0"])),
    );
    for name in ["recfile", "toc", "docvars", "book"] {
        if let Some(values) = crate::info::contents_values(document, name, kept(name)) {
            section(out, name, &values);
        }
    }
    // The printer the document was set up for; a new document has the one
    // Ami Pro's default sheet names.
    section(
        out,
        "prn",
        &kept("prn").unwrap_or_else(|| strings(&["QMS-PS 810"])),
    );
    section(
        out,
        "port",
        &kept("port").unwrap_or_else(|| strings(&["LPT1:"])),
    );
    section(
        out,
        "lang",
        &crate::info::language_values(document, kept("lang")),
    );
    section(
        out,
        "fldnames",
        &crate::info::field_name_values(document.info()),
    );
    section(out, "desc", &description(document, options));
    section(
        out,
        "fopts",
        &crate::info::footnote_option_values(document.footnote_options()),
    );
    if let Some(values) = kept("newmac") {
        section(out, "newmac", &values);
    }
    section(out, "lnopts", &crate::info::line_numbering_values(document));
    let mut document_options = kept("docopts").unwrap_or_else(|| strings(&["5", "2"]));
    document_options.resize(2, "2".into());
    document_options[0] = document.hyphenation_zone().to_string();
    section(out, "docopts", &document_options);
    section(out, "GramStyle", &[document.grammar_set().to_string()]);
}

/// A kept section's values: its lines, each without its one leading tab.
pub(crate) fn section_values(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split('\n')
        .map(|line| line.strip_prefix('\t').unwrap_or(line).to_owned())
        .collect()
}

/// `[desc]` as Ami Pro writes it into a style sheet (SS1N, SS2N, SS2W;
/// `11c8:342b`): the document's Doc Info with the sheet's description,
/// the last save time as the creation time, the save time now, and no
/// minutes of editing.
fn description(document: &Document, options: &SheetOptions) -> Vec<String> {
    let now = options.saved_at.to_string();
    let mut values = crate::info::description_values(document.info(), options.saved_at);
    // Escaped as Ami Pro's record writer escapes it (`11c8:0bf5`).
    values[0] = crate::decode_windows_1252(&crate::escape_text(&options.description));
    values[7] = std::mem::replace(&mut values[5], now);
    values[8] = "0".into();
    values
}

// ----- Page layouts -----

/// Assigns frame sequence numbers (field 13): kept ones stay, as Ami Pro
/// keeps them (`11c8:07dd`), and frames without one take the next free.
pub(crate) struct Sequence {
    next: u32,
    /// The numbers given to frames that had none, so that a frame keeps
    /// its number wherever it is named (the `[Embedded]` directory).
    given: BTreeMap<amipro_document::FrameId, String>,
}

impl Sequence {
    pub(crate) fn new(document: &Document) -> Self {
        let highest = document
            .frames()
            .filter_map(|frame| {
                frame
                    .legacy
                    .get("frame.field13")?
                    .trim()
                    .parse::<u32>()
                    .ok()
            })
            .max()
            .unwrap_or(0);
        Self {
            next: highest + 1,
            given: BTreeMap::new(),
        }
    }

    pub(crate) fn of(&mut self, frame: &Frame) -> String {
        // A frame numbered 0 gets the next free number, as Ami Pro gives
        // it (`11c8:07dd`; older files number every frame 0).
        if let Some(kept) = frame
            .legacy
            .get("frame.field13")
            .filter(|kept| kept.trim().parse::<u32>().is_ok_and(|number| number > 0))
        {
            return kept.to_owned();
        }
        if let Some(given) = self.given.get(&frame.id) {
            return given.clone();
        }
        let number = self.next.to_string();
        self.next += 1;
        self.given.insert(frame.id, number.clone());
        number
    }
}

/// Layout number bits (`SAM_PAGE_FRAME_SPEC.md` §3.1).
const LANDSCAPE: i64 = 0x100;
const ONE_LAYOUT: i64 = 0x200;
const MIRROR: i64 = 0x400;
const HEADER_SECOND: i64 = 0x800;
const FOOTER_SECOND: i64 = 0x1000;
/// The layout is the style sheet's: Save as a Style Sheet sets it on the
/// layout it writes (`11c8:14e0`).
const SHEET_LAYOUT: i64 = 0x80;

pub(crate) fn write_layout(
    out: &mut Lines,
    context: &Context<'_>,
    layout: &PageLayout,
    name: &str,
    sequence: &mut Sequence,
) {
    let document = context.document;
    let legacy = &layout.legacy;
    let unknown = legacy
        .get("lay.number.unknown")
        .and_then(|value| i64::from_str_radix(value.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);
    // Left pages the importer made from the right ones (left header or
    // footer frames without a `[lft]`) are written as they were read.
    let left = layout
        .left
        .as_ref()
        .filter(|_| legacy.get("lay.left-from-right").is_none());
    let number = i64::from(layout.paper)
        | if layout.landscape { LANDSCAPE } else { 0 }
        | if left.is_none() { ONE_LAYOUT } else { 0 }
        | if layout.mirror { MIRROR } else { 0 }
        | if layout.header_from_second_page {
            HEADER_SECOND
        } else {
            0
        }
        | if layout.footer_from_second_page {
            FOOTER_SECOND
        } else {
            0
        }
        | if context.sheet { SHEET_LAYOUT } else { 0 }
        | unknown;
    out.line(0, b"[lay]");
    out.text(1, name);
    out.number(1, number);
    out.line(1, b"[rght]");
    page_block(out, &layout.right, legacy, "rght");
    if let Some(left) = left {
        out.line(1, b"[lft]");
        page_block(out, left, legacy, "lft");
    }
    for (slot, id) in layout_frames(layout) {
        let Some(frame) = document.frame(id) else {
            continue;
        };
        out.line(1, format!("[{slot}]").as_bytes());
        let geometry = match frame.placement {
            Placement::Header {
                side: PageSide::Left,
            }
            | Placement::Footer {
                side: PageSide::Left,
            } => layout.left.as_ref().unwrap_or(&layout.right),
            _ => &layout.right,
        };
        write_layout_frame(out, context, frame, geometry, sequence);
    }
}

/// The layout's frames with the section that introduces each, in the kept
/// order when it names exactly these frames, else Ami Pro's usual one.
pub(crate) fn layout_frames(layout: &PageLayout) -> Vec<(&'static str, amipro_document::FrameId)> {
    let mut usual: Vec<(&'static str, _)> = [
        ("hlft", layout.header.left),
        ("hrght", layout.header.right),
        ("flft", layout.footer.left),
        ("frght", layout.footer.right),
    ]
    .into_iter()
    .filter_map(|(slot, frame)| Some((slot, frame?)))
    .collect();
    usual.extend(layout.repeating.iter().map(|frame| ("repfrm", *frame)));
    let Some(order) = layout.legacy.get("lay.order") else {
        return usual;
    };
    let mut remaining = usual.clone();
    let mut ordered = Vec::with_capacity(usual.len());
    for slot in order.split_whitespace() {
        let Some(index) = remaining.iter().position(|(name, _)| *name == slot) else {
            return usual;
        };
        ordered.push(remaining.remove(index));
    }
    if remaining.is_empty() { ordered } else { usual }
}

/// Flag bits of a layout frame's placement (`SAM_PAGE_FRAME_SPEC.md` §4.3).
const REPEATING: u32 = 0x100;
const HEADER: u32 = 0x800;
const FOOTER: u32 = 0x1000;
const RIGHT: u32 = 0x2000;

/// A header, footer, or repeating frame as a `[lyfrm]` record
/// (`11c8:2a0f`): its rectangle is the page's margin band (with the
/// offsets kept where the stored one differed) or its own.
fn write_layout_frame(
    out: &mut Lines,
    context: &Context<'_>,
    frame: &Frame,
    geometry: &PageGeometry,
    sequence: &mut Sequence,
) {
    let legacy = &frame.legacy;
    let band = |footer| {
        let band = margin_band(geometry, footer);
        let offsets: Vec<Twips> = legacy
            .get("frame.rect")
            .map(|offsets| {
                offsets
                    .split_whitespace()
                    .filter_map(|offset| offset.parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        match offsets[..] {
            [left, top, right, bottom] => Rect::new(
                band.left + left,
                band.top + top,
                band.right + right,
                band.bottom + bottom,
            ),
            _ => band,
        }
    };
    let (rect, mut flags) = match frame.placement {
        Placement::Header { side } => (
            band(false),
            HEADER | REPEATING | if side == PageSide::Right { RIGHT } else { 0 },
        ),
        Placement::Footer { side } => (
            band(true),
            FOOTER | REPEATING | if side == PageSide::Right { RIGHT } else { 0 },
        ),
        Placement::Repeating { pages, rect } => (
            rect,
            REPEATING
                | if pages == RepeatPages::RightLeft {
                    RIGHT
                } else {
                    0
                },
        ),
        // Page frames are not layout frames; not written here.
        _ => return,
    };
    flags |= match frame.style.wrap {
        Wrap::Around => WRAP_AROUND,
        Wrap::NoWrapBeside => NO_WRAP_BESIDE,
        Wrap::NoWrapAround => 0,
    };
    // A frame marked as the sheet's loses its picture's bits in
    // `write_frame`; tables are written whatever the marks (`11c8:2a0f`).
    flags |= style_and_content_flags(context.document, frame);
    write_frame(
        out,
        context,
        frame,
        Record {
            depth: 1,
            rect,
            flags,
            field1: legacy.get("frame.field1").unwrap_or("1").to_owned(),
            anchored: None,
        },
        sequence,
    );
}

/// The band a header (or footer) frame covers: the page's width, from the
/// top edge to the top margin (or from the bottom margin to the bottom).
pub(crate) fn margin_band(geometry: &PageGeometry, footer: bool) -> Rect {
    if footer {
        Rect::new(
            0,
            geometry.height - geometry.layout.margins.bottom,
            geometry.width,
            geometry.height,
        )
    } else {
        Rect::new(0, 0, geometry.width, geometry.layout.margins.top)
    }
}

/// Drops the page lines' kept values (`[rght]` and `[lft]` values 10 to
/// 14) that the writer would write anyway from the model, as
/// `frames::drop_derived_values` does for frames: a line that is on is the
/// model's, so the copy kept beside it says nothing, and a layout made or
/// changed in the app reads back with nothing kept (PL.11).
pub(crate) fn drop_derived_layout_values(document: &mut Document) {
    let mut changes = Vec::new();
    for layout in document.layouts() {
        let mut legacy = layout.legacy.clone();
        for (prefix, geometry) in [("rght", Some(&layout.right)), ("lft", layout.left.as_ref())] {
            let Some(geometry) = geometry else {
                continue;
            };
            let block = |legacy: &LegacyData| {
                let mut out = Lines::default();
                page_block(&mut out, geometry, legacy, prefix);
                out.0
            };
            for index in [10, 11, 12, 13, 14] {
                let key = format!("{prefix}.{index}");
                if legacy.get(&key).is_none() {
                    continue;
                }
                let mut without = legacy.clone();
                without.remove(&key);
                if block(&without) == block(&legacy) {
                    legacy = without;
                }
            }
        }
        if legacy != layout.legacy {
            changes.push((layout.id, legacy));
        }
    }
    for (id, legacy) in changes {
        if let Some(layout) = document.layout_mut(id) {
            layout.legacy = legacy;
        }
    }
}

/// A page's LAYOUT block (`[rght]`, `[lft]`).
fn page_block(out: &mut Lines, geometry: &PageGeometry, legacy: &LegacyData, prefix: &str) {
    let derived = page_columns(geometry.width, &geometry.layout);
    let edges = written_edges(
        &geometry.layout,
        &derived,
        legacy.get(&format!("{prefix}.edges")),
    );
    layout_block(
        out,
        Extent {
            first: geometry.height.into(),
            width: geometry.width.into(),
            top: geometry.layout.margins.top.into(),
        },
        &geometry.layout,
        geometry.border.as_ref(),
        &edges,
        legacy,
        prefix,
        PAGE_DEFAULTS,
    );
}

/// Values 1, 2, and 7 of a LAYOUT block: a page's height, width, and top
/// margin, or a frame's bottom edge, width, and the top of its text area.
pub(crate) struct Extent {
    pub first: i64,
    pub width: i64,
    pub top: i64,
}

/// Values 3, 6, and 10 to 14 when nothing was kept: what Ami Pro writes for
/// a page (`_DEFAULT.STY`, SS1N), for a header or footer frame, and for a
/// page frame it has just made (FC1, FC9; also the commonest in both
/// corpora: value 10 is 2 in 2,411 page frames and 1 in 68, values 12 to 14
/// `0 0 0` in 771, stale memory otherwise).
pub(crate) const PAGE_DEFAULTS: [i64; 7] = [1, 1, 1, 0, 1, 0, 2];
pub(crate) const FRAME_DEFAULTS: [i64; 7] = [1, 1, 1, 0, 1, 1, 0];
pub(crate) const PAGE_FRAME_DEFAULTS: [i64; 7] = [1, 1, 2, 0, 0, 0, 0];

/// A LAYOUT block (§3.2, `11c8:24d6`): fifteen values, then each column's
/// edges and tabs, two tab levels deep.
#[allow(clippy::too_many_arguments)]
pub(crate) fn layout_block(
    out: &mut Lines,
    extent: Extent,
    layout: &LayoutBox,
    border: Option<&Border>,
    edges: &[(i64, i64)],
    legacy: &LegacyData,
    prefix: &str,
    defaults: [i64; 7],
) {
    let kept = |index: usize, default: i64| {
        legacy
            .get(&format!("{prefix}.{index}"))
            .and_then(|value| value.trim().parse::<i64>().ok())
            .unwrap_or(default)
    };
    let margins = layout.margins;
    let flags = i64::from(layout.balance)
        | if layout.line_between.is_some() { 2 } else { 0 }
        | if border.is_some() { 4 } else { 0 }
        | kept(9, 0);
    let colour = border
        .map(|border| i64::from(border.colour))
        .or(layout.line_between.map(|line| i64::from(line.colour)));
    let values = [
        extent.first,
        extent.width,
        kept(3, defaults[0]),
        margins.left.into(),
        margins.bottom.into(),
        kept(6, defaults[1]),
        extent.top,
        margins.right.into(),
        flags,
        layout
            .line_between
            .map_or_else(|| kept(10, defaults[2]), |line| line.style.into()),
        colour.unwrap_or_else(|| kept(11, defaults[3])),
        border.map_or_else(|| kept(12, defaults[4]), |border| border.style.into()),
        border.map_or_else(
            || kept(13, defaults[5]),
            |border| {
                side_mask(
                    border.sides,
                    legacy
                        .get(&format!("{prefix}.13"))
                        .and_then(|v| v.trim().parse().ok()),
                )
            },
        ),
        border.map_or_else(|| kept(14, defaults[6]), |border| border.position as i64),
        layout.columns.into(),
    ];
    for value in values {
        out.number(2, unsigned(value));
    }
    for (column, (left, right)) in edges.iter().enumerate() {
        out.number(2, unsigned(*left));
        out.number(2, unsigned(*right));
        let tabs = layout.tabs.get(column).map_or(&[][..], Vec::as_slice);
        out.number(2, tabs.len() as i64);
        for stop in tabs {
            out.number(2, crate::writer::tab_type(stop).into());
            out.number(2, unsigned(stop.position.into()));
        }
    }
}

/// Ami Pro writes words with `%u`: a negative value appears as its 16-bit
/// two's complement.
pub(crate) fn unsigned(value: i64) -> i64 {
    if value < 0 {
        value.rem_euclid(0x1_0000)
    } else {
        value
    }
}

/// A side mask (§1): 1 for all sides, else 2 left, 4 right, 8 top, 16
/// bottom; a kept mask that means the same sides is written as it was.
/// A frame's field 7 for `sides`: the value read where it gives the same
/// sides (with the lines bit set exactly when there are sides), else the
/// mask.
pub(crate) fn frame_side_mask(sides: Sides, kept: Option<i64>) -> i64 {
    if let Some(kept) = kept
        && u32::try_from(kept)
            .is_ok_and(|mask| crate::build::frame_sides(mask, sides != Sides::NONE) == sides)
    {
        return kept;
    }
    side_mask(sides, None)
}

pub(crate) fn side_mask(sides: Sides, kept: Option<i64>) -> i64 {
    if let Some(kept) = kept
        && u32::try_from(kept).is_ok_and(|mask| crate::build::sides(mask) == sides)
    {
        return kept;
    }
    if sides == Sides::ALL {
        return 1;
    }
    [
        (sides.left, 2),
        (sides.right, 4),
        (sides.top, 8),
        (sides.bottom, 16),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, bit)| bit)
    .sum()
}

// ----- Column edges -----

/// A page's column edges (absolute), equal columns between the margins.
pub(crate) fn page_columns(width: Twips, layout: &LayoutBox) -> Vec<(i64, i64)> {
    uniform_columns(
        layout.margins.left.into(),
        i64::from(width) - i64::from(layout.margins.right),
        layout,
    )
}

/// A frame's column edges (absolute): equal columns between its margins,
/// further inset by r(1 − cos 45°) on each side with rounded corners, where
/// r = corners% × min(width, height) / 2 (F01, F03).
pub(crate) fn frame_columns(rect: Rect, style: &amipro_document::FrameStyle) -> Vec<(i64, i64)> {
    let (start, end) = frame_text_area(rect, style);
    uniform_columns(start, end, &style.interior)
}

/// A frame's text area across (absolute): inside its margins, further
/// inset with rounded corners, as Ami Pro works it out
/// (`FrameStyle::text_insets`; F01, F03, FC1–FC3).
pub(crate) fn frame_text_area(rect: Rect, style: &amipro_document::FrameStyle) -> (i64, i64) {
    let insets = style.text_insets(rect.width(), rect.height());
    (
        i64::from(rect.left) + i64::from(insets.left),
        i64::from(rect.right) - i64::from(insets.right),
    )
}

/// The edges a box writes (absolute): its uneven columns from the text
/// area's left `derived[0]`, or the derived even ones with any kept
/// offsets.
pub(crate) fn written_edges(
    layout: &LayoutBox,
    derived: &[(i64, i64)],
    offsets: Option<&str>,
) -> Vec<(i64, i64)> {
    match (&layout.column_edges, derived.first()) {
        (Some(edges), Some(&(origin, _))) => edges
            .iter()
            .map(|edges| {
                (
                    origin + i64::from(edges.left),
                    origin + i64::from(edges.right),
                )
            })
            .collect(),
        _ => apply_offsets(derived, offsets),
    }
}

fn uniform_columns(start: i64, end: i64, layout: &LayoutBox) -> Vec<(i64, i64)> {
    let count = i64::from(layout.columns.max(1));
    let gutter = i64::from(layout.gutter);
    let width = (end - start - gutter * (count - 1)) / count;
    (0..count)
        .map(|index| {
            let left = start + index * (width + gutter);
            (left, left + width)
        })
        .collect()
}

/// How stored column edges differ from the derived ones, edge by edge, or
/// `None` when they agree: kept so that the writer gives the stored edges
/// back, and moves them with the frame or page.
pub(crate) fn edge_offsets(stored: &[(i64, i64)], derived: &[(i64, i64)]) -> Option<String> {
    if stored == derived || stored.len() != derived.len() {
        return None;
    }
    Some(
        stored
            .iter()
            .zip(derived)
            .flat_map(|(stored, derived)| [stored.0 - derived.0, stored.1 - derived.1])
            .map(|offset| offset.to_string())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(crate) fn apply_offsets(derived: &[(i64, i64)], offsets: Option<&str>) -> Vec<(i64, i64)> {
    let offsets: Vec<i64> = offsets
        .map(|offsets| {
            offsets
                .split_whitespace()
                .filter_map(|offset| offset.parse().ok())
                .collect()
        })
        .unwrap_or_default();
    if offsets.len() != derived.len() * 2 {
        return derived.to_vec();
    }
    derived
        .iter()
        .zip(offsets.chunks(2))
        .map(|((left, right), offset)| (left + offset[0], right + offset[1]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{import_sam, read_style_sheet};
    use amipro_document::default_styles;

    const OPTIONS: SheetOptions = SheetOptions {
        contents: false,
        description: String::new(),
        saved_at: 1_791_958_088,
    };

    /// A page layout in Ami Pro's own form: two columns with a line between
    /// them and lines around the page, a header frame without 0x8 whose text
    /// is written, a footer frame with 0x8 whose text is not, and a
    /// repeating frame with rounded corners, a name, and odd column edges.
    const LAYOUT: &str = "[lay]\r\n\tStandard\r\n\t641\r\n\t[rght]\r\n\
        \t\t15840\r\n\t\t12240\r\n\t\t1\r\n\t\t1440\r\n\t\t1440\r\n\t\t1\r\n\t\t1440\r\n\t\t1440\r\n\t\t6\r\n\t\t3\r\n\t\t255\r\n\t\t2\r\n\t\t1\r\n\t\t2\r\n\t\t2\r\n\
        \t\t1440\r\n\t\t6000\r\n\t\t1\r\n\t\t1\r\n\t\t720\r\n\t\t6240\r\n\t\t10800\r\n\t\t0\r\n\
        \t[hrght]\r\n\t[lyfrm]\r\n\t\t1\r\n\t\t11200\r\n\t\t0\r\n\t\t0\r\n\t\t12240\r\n\t\t1440\r\n\t\t0\r\n\t\t1\r\n\t\t3\r\n\t\t1 0 0 0 0 0 0\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\
        \t[frmlay]\r\n\t\t1440\r\n\t\t12240\r\n\t\t1\r\n\t\t1440\r\n\t\t72\r\n\t\t1\r\n\t\t792\r\n\t\t1440\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\
        \t\t1440\r\n\t\t10800\r\n\t\t2\r\n\t\t2\r\n\t\t4680\r\n\t\t3\r\n\t\t9360\r\n\
        \t[txt]\r\n@Header@<+B>Page <:P10,1,>\r\n\r\n>\r\n\
        \t[frght]\r\n\t[lyfrm]\r\n\t\t1\r\n\t\t13256\r\n\t\t0\r\n\t\t14400\r\n\t\t12240\r\n\t\t15840\r\n\t\t0\r\n\t\t1\r\n\t\t3\r\n\t\t1 0 0 0 0 0 0\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\
        \t[frmlay]\r\n\t\t15840\r\n\t\t12240\r\n\t\t1\r\n\t\t1440\r\n\t\t792\r\n\t\t1\r\n\t\t14472\r\n\t\t1440\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\
        \t\t1440\r\n\t\t10800\r\n\t\t0\r\n\
        \t[txt]\r\n>\r\n\
        \t[repfrm]\r\n\t[lyfrm]\r\n\t\t1\r\n\t\t960\r\n\t\t1440\r\n\t\t2880\r\n\t\t4320\r\n\t\t5040\r\n\t\t1\r\n\t\t1\r\n\t\t1\r\n\t\t1 0 0 100 100 18 0\r\n\t\t255\r\n\t\t16777215\r\n\t\t3\r\n\
        \t[frmname]\r\n\t\tLogo\r\n\
        \t[frmlay]\r\n\t\t5040\r\n\t\t2880\r\n\t\t1\r\n\t\t144\r\n\t\t144\r\n\t\t1\r\n\t\t3024\r\n\t\t144\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t48\r\n\t\t42\r\n\t\t1368\r\n\t\t1\r\n\
        \t\t1641\r\n\t\t4119\r\n\t\t0\r\n\
        \t[txt]\r\n\r\n>\r\n";

    fn document_with_layout() -> Document {
        let sam = format!(
            "[ver]\r\n\t4\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n{LAYOUT}[elay]\r\n[edoc]\r\n>\r\n"
        );
        import_sam(sam.as_bytes(), None, "Layout").unwrap().document
    }

    #[test]
    fn a_page_layout_is_written_back_as_ami_pro_wrote_it() {
        let document = document_with_layout();
        let written =
            String::from_utf8(layout_record(&document, document.document_layout())).unwrap();
        for (number, (ours, theirs)) in written.split("\r\n").zip(LAYOUT.split("\r\n")).enumerate()
        {
            assert_eq!(ours, theirs, "line {number}");
        }
        assert_eq!(written, LAYOUT);
    }

    #[test]
    fn kept_column_edges_and_bands_move_with_their_frame_and_page() {
        let mut document = document_with_layout();
        let id = document.document_layout().repeating[0];
        // Move the repeating frame right by 100 twips: its odd edges move
        // with it.
        if let Some(frame) = document.frame_mut(id)
            && let Placement::Repeating { rect, .. } = &mut frame.placement
        {
            *rect = rect.offset(100, 0).unwrap();
        }
        let written =
            String::from_utf8(layout_record(&document, document.document_layout())).unwrap();
        assert!(written.contains("\t\t1741\r\n\t\t4219\r\n\t\t0\r\n\t[txt]"));
    }

    /// A document whose page has two columns stored at `edges`, as Ami
    /// Pro writes RU2's page layout.
    fn two_column_document(edges: [i64; 4]) -> Vec<u8> {
        let tabs: String = (1..=12)
            .map(|stop| format!("\t\t1\r\n\t\t{}\r\n", 720 * stop))
            .collect();
        format!(
            "[ver]\r\n\t4\r\n[lay]\r\n\tStandard\r\n\t513\r\n\t[rght]\r\n\t\t15840\r\n\t\t12240\r\n\t\t1\r\n\t\t1440\r\n\t\t1440\r\n\t\t1\r\n\t\t1440\r\n\t\t1440\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t2\r\n\t\t{}\r\n\t\t{}\r\n\t\t12\r\n{tabs}\t\t{}\r\n\t\t{}\r\n\t\t0\r\n[elay]\r\n[edoc]\r\nText\r\n\r\n>\r\n",
            edges[0], edges[1], edges[2], edges[3]
        )
        .into_bytes()
    }

    #[test]
    fn uneven_page_columns_are_model_data_and_written_back_ru2() {
        let sam = two_column_document([1440, 7050, 7290, 10800]);
        let document = import_sam(&sam, None, "RU2").unwrap().document;
        let layout = &document.document_layout().right.layout;
        let edges: Vec<_> = layout
            .column_edges
            .as_ref()
            .expect("uneven")
            .iter()
            .map(|edges| (edges.left, edges.right))
            .collect();
        assert_eq!(edges, [(0, 5_610), (5_850, 9_360)]);
        assert!(
            document
                .document_layout()
                .legacy
                .get("rght.edges")
                .is_none()
        );
        let written = String::from_utf8(crate::sam_document(
            &document,
            &crate::SamOptions::at(1_791_958_088),
        ))
        .unwrap();
        assert!(
            written.contains("\t\t2\r\n\t\t1440\r\n\t\t7050\r\n\t\t12\r\n"),
            "{written}"
        );
        assert!(
            written.contains("\t\t7290\r\n\t\t10800\r\n\t\t0\r\n"),
            "{written}"
        );
        // Even columns a few twips off the margin stay even, their stored
        // edges kept as offsets and written back.
        let near = import_sam(
            &two_column_document([1440, 5996, 6236, 10792]),
            None,
            "Near",
        )
        .unwrap()
        .document;
        assert_eq!(near.document_layout().right.layout.column_edges, None);
        assert_eq!(
            near.document_layout().legacy.get("rght.edges"),
            Some("0 -4 -4 -8")
        );
    }

    #[test]
    fn a_new_documents_sheet_has_ami_pros_sections_and_reads_back() {
        let document = Document::from_plain_text("");
        let sheet = style_sheet(
            &document,
            &SheetOptions {
                contents: false,
                description: "Mine <1>".into(),
                saved_at: 1_791_958_088,
            },
        );
        let text = String::from_utf8(sheet.clone()).unwrap();
        let sections: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with('[') && line != &"[tag]")
            .collect();
        assert_eq!(
            sections,
            [
                "[ver]",
                "[files]",
                "[charset]",
                "[revisions]",
                "[prn]",
                "[port]",
                "[lang]",
                "[fldnames]",
                "[desc]",
                "[fopts]",
                "[lnopts]",
                "[docopts]",
                "[GramStyle]",
                "[lay]",
                "[elay]",
                "[edoc]"
            ]
        );
        assert!(text.contains("[desc]\r\n\tMine <<1>\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t1791958088\r\n\t1\r\n\t1791958088\r\n\t0\r\n"));
        assert!(text.ends_with("[elay]\r\n[edoc]\r\n"));
        let read = read_style_sheet(&sheet, "new").unwrap();
        let mut names: Vec<_> = read.styles.iter().map(|style| style.name.clone()).collect();
        names.sort();
        let mut expected: Vec<_> = default_styles()
            .values()
            .map(|style| style.name.clone())
            .collect();
        expected.sort();
        assert_eq!(names, expected);
        assert_eq!(read.layout.unwrap().right, document.document_layout().right);
    }

    #[test]
    fn kept_sections_are_written_in_ami_pros_order_and_the_description_updated() {
        let sam = "[ver]\r\n\t4\r\n[sty]\r\n\t_default.sty\r\n[files]\r\n[charset]\r\n\t82\r\n\tANSI (Windows, IBM CP 1252)\r\n\
            [revisions]\r\n\t0\r\n[prn]\r\n\tPrinter caf<\\i>\r\n[port]\r\n\tLPT1:\r\n[lang]\r\n\t1\r\n\
            [desc]\r\n\tOld\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t700\r\n\t1\r\n\t600\r\n\t5\r\n\t1\r\n\t0\r\n\t0\r\n\t0\r\n\t0\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t1\r\n\
            [fopts]\r\n\t0\r\n\t1\r\n\t0\r\n\t0\r\n[lnopts]\r\n\t2\r\n\tBody Text\r\n\t1\r\n[docopts]\r\n\t3\r\n\t2\r\n\
            [tag]\r\n\tLast\r\n\t0\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
            [tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
            [edoc]\r\n@Ghost@Text\r\n\r\n>\r\n";
        let document = import_sam(sam.as_bytes(), None, "Kept").unwrap().document;
        let sheet = String::from_utf8(style_sheet(
            &document,
            &SheetOptions {
                contents: false,
                description: "New".into(),
                saved_at: 900,
            },
        ))
        .unwrap();
        assert!(!sheet.contains("[sty]"));
        assert!(sheet.contains("[prn]\r\n\tPrinter caf<\\i>\r\n"));
        // The revisions as the open document holds them: one more than its
        // file said (Ami Pro writes its count in memory, `2375`).
        assert!(sheet.contains(
            "[desc]\r\n\tNew\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t900\r\n\t2\r\n\t700\r\n\t0\r\n\t1\r\n"
        ));
        assert!(sheet.contains("[fopts]\r\n\t0\r\n\t1\r\n\t0\r\n\t0\r\n\t\r\n\t\r\n[lnopts]"));
        assert!(sheet.contains("[docopts]\r\n\t3\r\n\t2\r\n[GramStyle]\r\n\t\r\n"));
        // Function-key order, a style without a key last; a name defined
        // nowhere is not written.
        let body = sheet.find("\tBody Text\r\n\t2").unwrap();
        let last = sheet.find("\tLast\r\n\t0").unwrap();
        assert!(body < last);
        assert!(!sheet.contains("Ghost"));
        let _ = OPTIONS;
    }
}
