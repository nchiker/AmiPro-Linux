//! Native format v3 (`docs/Design/NATIVE_FORMAT_V3.md`): a strict,
//! line-oriented, tab-separated encoding of the whole model, with embedded
//! payloads as base64 chunks.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::{
    Alignment, BLACK, BreakKind, Breaks, Bullets, Caps, CellContent, CellFormat, CellLeader,
    CellLines, CellMargins, CellSpan, CharacterAttributes, CharacterFormat, CharacterRun, Colour,
    ColumnEdges, ColumnLine, DateField, DateSource, DocInfoField, Document, DocumentError,
    DocumentInfo, EmbeddedObject, FaceKey, Flow, FlowId, Frame, FrameContent, FrameId, FrameStyle,
    GroupId, HeaderFooterKind, Indents, InlineObject, LayoutBox, LayoutId, LegacyData, LegacyMark,
    LineHeights, LinePosition, LineSpacing, Margins, NumberFormat, ObjectId, Offset, PageGeometry,
    PageLayout, PageNumberField, PageSide, PageSides, Paragraph, ParagraphCache, ParagraphFormat,
    ParagraphId, ParagraphRules, ParagraphStyle, Picture, PictureScaling, PictureSizing,
    PictureSource, Placement, Rect, RepeatPages, RuleLength, RuleSide, Shadow, SideFrames, Sides,
    Size, Spacing, Statistics, StyleId, StyleKept, StyleOrigin, StyleSheetLink, TabKind, TabLeader,
    TabStop, Table, TableCell, TableLayout, TableRows, TableSpacing, Twips, WHITE, Wrap,
    geometry::Border,
};

pub const MAGIC_V3: &str = "AMIPRO-LINUX/3";

/// The size a picture read from a file written before the scaling record
/// gets when it keeps no `[isd]` values either.
const UNSCALED: Size = Size {
    width: 1_440,
    height: 1_440,
};
const BLOB_CHUNK: usize = 76;

// ---------------------------------------------------------------- encoding

/// Doc Info, when it is not empty: `docinfo` with the numbers and the
/// texts, then `docfield` for each named or filled field.
fn encode_info(out: &mut Output, info: &DocumentInfo) {
    if *info == DocumentInfo::default() {
        return;
    }
    let optional = |value: Option<u32>| value.map_or_else(String::new, |value| value.to_string());
    let statistics = &info.statistics;
    out.record(&[
        "docinfo",
        &optional(info.created),
        &optional(info.saved),
        &info.revisions.to_string(),
        &info.editing_minutes.to_string(),
        &statistics.pages.to_string(),
        &statistics.words.to_string(),
        &statistics.characters.to_string(),
        &statistics.size.to_string(),
        &statistics.size_with_links.to_string(),
        &info.flags.to_string(),
        &escape(&info.description),
        &escape(&info.keywords),
        &escape(&info.annotation_lock),
    ]);
    for (index, (name, value)) in info.field_names.iter().zip(&info.fields).enumerate() {
        if !name.is_empty() || !value.is_empty() {
            out.record(&[
                "docfield",
                &(index + 1).to_string(),
                &escape(name),
                &escape(value),
            ]);
        }
    }
}

pub(crate) fn encode(document: &Document) -> String {
    let mut out = Output(format!("{MAGIC_V3}\n"));
    out.record(&["title", &escape(&document.title)]);
    out.record(&[
        "ids",
        &document.next_id.to_string(),
        &document.main_flow.0.to_string(),
    ]);
    if document.hyphenation_zone != crate::DEFAULT_HYPHENATION_ZONE {
        out.record(&["hyphenzone", &document.hyphenation_zone.to_string()]);
    }
    encode_info(&mut out, &document.info);
    // The language and grammar set, when they are not a new document's
    // (the proofing run, 2026-10-26).
    if document.language != crate::Language::default() {
        let language = &document.language;
        let mut fields = vec!["language".to_string(), language.number.to_string()];
        if let Some(locale) = &language.locale {
            fields.push(escape(locale));
        }
        let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
        out.record(&fields);
    }
    if !document.grammar_set.is_empty() {
        out.record(&["grammarset", &escape(&document.grammar_set)]);
    }
    // The TOC levels and the TOC, Index options, as Ami Pro's sections
    // keep them (2026-11-03).
    for (name, values) in [
        (
            "toc",
            document.contents.toc.as_ref().map(|toc| toc.values()),
        ),
        (
            "master",
            document
                .contents
                .master
                .as_ref()
                .map(|master| master.values()),
        ),
        (
            "sequence",
            document
                .contents
                .sequence
                .as_ref()
                .map(|sequence| sequence.values()),
        ),
    ] {
        if let Some(values) = values {
            let mut fields = vec![name.to_string()];
            fields.extend(values.iter().map(|value| escape(value)));
            out.record_owned(&fields);
        }
    }
    // The merge's `[recfile]` and its conditions (the merge run, MG6).
    if let Some(file) = &document.merge.record_file {
        let mut fields = vec!["recfile".to_string()];
        fields.extend(file.values().iter().map(|value| escape(value)));
        out.record_owned(&fields);
    }
    for condition in &document.merge.conditions {
        out.record(&[
            "mergecondition",
            &escape(&condition.field),
            condition.operator.symbol(),
            &escape(&condition.value),
            match condition.join {
                crate::merge::Join::And => "and",
                crate::merge::Join::Or => "or",
            },
        ]);
    }
    if document.merge.with_conditions {
        out.record(&["withconditions"]);
    }
    // Footnote Options, when they are not Ami Pro's defaults (2026-10-26).
    if *document.footnote_options != crate::FootnoteOptions::default() {
        let options = &document.footnote_options;
        out.record(&[
            "footnotes",
            &options.flags.to_string(),
            &options.start.to_string(),
            &options.length.to_string(),
            &options.indent.to_string(),
            &escape(&options.continued_on),
            &escape(&options.continued_from),
        ]);
    }
    // Line Numbering, when not Ami Pro's default (2026-11-05).
    if *document.line_numbering != crate::LineNumbering::default() {
        let numbering = &document.line_numbering;
        out.record(&[
            "linenumbering",
            &numbering.flags.to_string(),
            &escape(&numbering.style),
            &numbering.interval.to_string(),
        ]);
    }
    if let Some(link) = &document.style_sheet {
        out.record(&[
            "stylesheet",
            &escape(&link.name),
            bool_field(link.layout_from_sheet),
        ]);
    }
    for style in document.styles.values() {
        let mut fields = vec![
            "style".to_owned(),
            style.id.0.to_string(),
            escape(&style.name),
            style.parent.map_or(0, |id| id.0).to_string(),
            escape(&style.font_family),
            style.font_size_points.to_string(),
            bool_field(style.bold).to_owned(),
            bool_field(style.italic).to_owned(),
            (style.spacing.below as f32 / 20.0).to_string(),
        ];
        // Indents and the rest of the spacing only when they differ from the
        // defaults, so that files without them read as before (2026-10-12).
        let indents = style.indents;
        let spacing = style.spacing;
        let breaks = style.breaks;
        let alignment = style.alignment;
        if !indents.is_zero()
            || spacing != Spacing::below(spacing.below)
            || breaks != Breaks::default()
            || alignment != Alignment::Left
        {
            fields.extend(
                [indents.all, indents.first, indents.rest, indents.right].map(|v| v.to_string()),
            );
            let (kind, height) = line_spacing_fields(spacing.line);
            fields.extend([
                kind.to_owned(),
                height.to_string(),
                spacing.above.to_string(),
                bool_field(spacing.add_at_break).to_owned(),
                spacing.tightness.to_string(),
                breaks.0.to_string(),
            ]);
            if alignment != Alignment::Left {
                fields.push(alignment_field(alignment).to_owned());
            }
        }
        out.record_owned(&fields);
        encode_style_attributes(&mut out, style.attributes);
        encode_style_rules(&mut out, style.rules);
        if let Some(next) = style.next {
            out.record(&["snext", &next.0.to_string()]);
        }
        if !style.tabs.is_empty() {
            let mut fields = vec!["stabs".to_owned()];
            fields.extend(style.tabs.iter().map(encode_tab_stop));
            out.record_owned(&fields);
        }
        if style.hanging {
            out.record(&["shanging"]);
        }
        if style.hyphenation {
            out.record(&["shyphenation"]);
        }
        if style.bullets != Bullets::default() {
            let bullets = &style.bullets;
            let [v1, v4, v5, v6, v9, v10] = bullets.kept;
            out.record(&[
                "sbullets",
                &escape(&bullets.text),
                &bullets.level.to_string(),
                &bullets.attributes.to_string(),
                &bullets.flags.to_string(),
                &v1.to_string(),
                &v4.to_string(),
                &v5.to_string(),
                &v6.to_string(),
                &v9.to_string(),
                &v10.to_string(),
            ]);
        }
        match style.origin {
            StyleOrigin::Document => {}
            StyleOrigin::Sheet => out.record(&["sorigin", "sheet"]),
            StyleOrigin::Missing => out.record(&["sorigin", "missing"]),
        }
        if style.both_sides || style.kept != StyleKept::default() {
            let kept = &style.kept;
            let mut fields = vec![
                "skept".to_owned(),
                bool_field(style.both_sides).to_owned(),
                kept.function_key.to_string(),
                kept.face_bits.to_string(),
                kept.units[0].to_string(),
                kept.units[1].to_string(),
                kept.units[2].to_string(),
                kept.add_in.to_string(),
                kept.single_height.to_string(),
            ];
            fields.extend(
                kept.number_format
                    .to_values()
                    .iter()
                    .map(|value| escape(value)),
            );
            fields.push(bool_field(kept.next_name.is_some()).to_owned());
            fields.push(escape(kept.next_name.as_deref().unwrap_or("")));
            fields.push(kept.alignment_bits.to_string());
            out.record_owned(&fields);
        }
        if style.first_line_bold {
            out.record(&["sfirstbold"]);
        }
    }
    for (key, height) in document.line_heights.iter() {
        out.record(&[
            "lineheight",
            &escape(&key.family),
            &key.size.to_string(),
            bool_field(key.bold),
            bool_field(key.italic),
            &height.to_string(),
        ]);
    }
    for (family, kind) in document.face_kinds.iter() {
        out.record(&["facekind", &escape(family), &kind.to_string()]);
    }
    for value in &document.legacy.values {
        out.record(&["doclegacy", &escape(&value.key), &escape(&value.value)]);
    }
    let referenced: BTreeSet<ObjectId> = document
        .frames
        .values()
        .flat_map(|frame| frame.objects())
        .collect();
    for (id, object) in document.objects.iter() {
        if !referenced.contains(id) {
            continue;
        }
        out.record(&["object", &id.0.to_string(), &object.bytes.len().to_string()]);
        let encoded = base64_encode(&object.bytes);
        for chunk in encoded.as_bytes().chunks(BLOB_CHUNK) {
            out.record(&["blob", std::str::from_utf8(chunk).expect("base64 is ASCII")]);
        }
    }
    for layout in document.layouts.iter() {
        encode_layout(&mut out, layout);
    }
    encode_start_layout(&mut out, document);
    for frame in document.frames.values() {
        encode_frame(&mut out, frame);
    }
    let mut stack = vec!["stack".to_owned()];
    stack.extend(document.frame_stack.iter().map(|id| id.0.to_string()));
    out.record_owned(&stack);
    for (id, flow) in document.flows.iter() {
        out.record(&["flow", &id.0.to_string()]);
        for paragraph in &flow.paragraphs {
            encode_paragraph(&mut out, paragraph);
        }
    }
    out.0
}

struct Output(String);

impl Output {
    fn record(&mut self, fields: &[&str]) {
        self.0.push_str(&fields.join("\t"));
        self.0.push('\n');
    }

    fn record_owned(&mut self, fields: &[String]) {
        self.0.push_str(&fields.join("\t"));
        self.0.push('\n');
    }
}

fn bool_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn optional_bool_field(value: Option<bool>) -> &'static str {
    match value {
        None => "-",
        Some(false) => "0",
        Some(true) => "1",
    }
}

fn side_field(side: PageSide) -> &'static str {
    match side {
        PageSide::Right => "right",
        PageSide::Left => "left",
    }
}

fn encode_layout(out: &mut Output, layout: &PageLayout) {
    out.record(&[
        "layout",
        &layout.id.0.to_string(),
        &escape(&layout.name),
        &layout.paper.to_string(),
        bool_field(layout.landscape),
        bool_field(layout.mirror),
        bool_field(layout.header_from_second_page),
        bool_field(layout.footer_from_second_page),
    ]);
    for (side, page) in [
        ("right", Some(&layout.right)),
        ("left", layout.left.as_ref()),
    ] {
        let Some(page) = page else { continue };
        out.record(&[
            "page",
            side,
            &page.width.to_string(),
            &page.height.to_string(),
        ]);
        encode_box(out, &page.layout);
        if let Some(border) = page.border {
            let mut fields = vec!["pageborder".to_owned()];
            fields.extend(border_fields(border));
            out.record_owned(&fields);
        }
    }
    let frame_or_zero = |id: Option<FrameId>| id.map_or(0, |id| id.0).to_string();
    out.record(&[
        "slots",
        &frame_or_zero(layout.header.right),
        &frame_or_zero(layout.header.left),
        &frame_or_zero(layout.footer.right),
        &frame_or_zero(layout.footer.left),
    ]);
    let mut repeating = vec!["repeating".to_owned()];
    repeating.extend(layout.repeating.iter().map(|id| id.0.to_string()));
    out.record_owned(&repeating);
    for value in &layout.legacy.values {
        out.record(&["layoutlegacy", &escape(&value.key), &escape(&value.value)]);
    }
}

/// The layout the first page uses, after every layout, when it is not the
/// document's own (Phase 4.5).
fn encode_start_layout(out: &mut Output, document: &Document) {
    if let Some(start) = document.start_layout {
        out.record(&["startlayout", &start.0.to_string()]);
    }
}

fn encode_box(out: &mut Output, layout: &LayoutBox) {
    let margins = layout.margins;
    let (line_style, line_colour) = layout.line_between.map_or_else(
        || ("-".to_owned(), "-".to_owned()),
        |line| (line.style.to_string(), line.colour.to_string()),
    );
    out.record(&[
        "box",
        &margins.top.to_string(),
        &margins.right.to_string(),
        &margins.bottom.to_string(),
        &margins.left.to_string(),
        &layout.columns.to_string(),
        &layout.gutter.to_string(),
        bool_field(layout.balance),
        &line_style,
        &line_colour,
    ]);
    if let Some(edges) = &layout.column_edges {
        let mut fields = vec!["edges".to_owned()];
        for edges in edges {
            fields.push(edges.left.to_string());
            fields.push(edges.right.to_string());
        }
        out.record_owned(&fields);
    }
    for (column, tabs) in layout.tabs.iter().enumerate() {
        let mut fields = vec!["tabs".to_owned(), column.to_string()];
        fields.extend(tabs.iter().map(encode_tab_stop));
        out.record_owned(&fields);
    }
}

fn border_fields(border: Border) -> Vec<String> {
    let sides: String = [
        (border.sides.left, 'L'),
        (border.sides.right, 'R'),
        (border.sides.top, 'T'),
        (border.sides.bottom, 'B'),
    ]
    .iter()
    .filter(|(set, _)| *set)
    .map(|(_, letter)| *letter)
    .collect();
    vec![
        if sides.is_empty() { "-".into() } else { sides },
        border.style.to_string(),
        border.colour.to_string(),
        (border.position as u8).to_string(),
    ]
}

fn rect_fields(rect: Rect) -> [String; 4] {
    [rect.left, rect.top, rect.right, rect.bottom].map(|value| value.to_string())
}

fn encode_frame(out: &mut Output, frame: &Frame) {
    out.record(&[
        "frame",
        &frame.id.0.to_string(),
        &frame.group.map_or(0, |id| id.0).to_string(),
    ]);
    let mut placement = vec!["placement".to_owned()];
    match frame.placement {
        Placement::Fixed { page, rect } => {
            placement.extend(["fixed".to_owned(), page.to_string()]);
            placement.extend(rect_fields(rect));
        }
        Placement::BelowParagraph { x_offset, size } => placement.extend([
            "below".to_owned(),
            x_offset.to_string(),
            size.width.to_string(),
            size.height.to_string(),
        ]),
        Placement::Inline { y_offset, size } => placement.extend([
            "inline".to_owned(),
            y_offset.to_string(),
            size.width.to_string(),
            size.height.to_string(),
        ]),
        Placement::Repeating { pages, rect } => {
            placement.extend([
                "repeating".to_owned(),
                match pages {
                    RepeatPages::All => "all",
                    RepeatPages::RightLeft => "rightleft",
                }
                .to_owned(),
            ]);
            placement.extend(rect_fields(rect));
        }
        Placement::Header { side } => {
            placement.extend(["header".to_owned(), side_field(side).to_owned()]);
        }
        Placement::Footer { side } => {
            placement.extend(["footer".to_owned(), side_field(side).to_owned()]);
        }
        Placement::InCell => placement.push("incell".to_owned()),
    }
    out.record_owned(&placement);
    let style = &frame.style;
    out.record(&[
        "look",
        match style.wrap {
            Wrap::Around => "around",
            Wrap::NoWrapAround => "none",
            Wrap::NoWrapBeside => "beside",
        },
        bool_field(style.opaque),
        &style.corners.to_string(),
        &style.background.to_string(),
    ]);
    let mut border = vec!["border".to_owned()];
    border.extend(border_fields(style.border));
    out.record_owned(&border);
    let shadow = style.shadow;
    out.record_owned(&[
        "shadow".to_owned(),
        shadow.left.to_string(),
        shadow.top.to_string(),
        shadow.right.to_string(),
        shadow.bottom.to_string(),
        shadow.colour.to_string(),
    ]);
    encode_box(out, &style.interior);
    if let Some(name) = &style.name {
        out.record(&["name", &escape(name)]);
    }
    if let Some(name) = &style.macro_name {
        out.record(&["macro", &escape(name)]);
    }
    match &frame.content {
        FrameContent::Text(flow) => out.record(&["content", "text", &flow.0.to_string()]),
        FrameContent::Table(table) => {
            let mut fields = vec![
                "content".to_owned(),
                "table".to_owned(),
                table.columns.len().to_string(),
                table.rows.len().to_string(),
                table.cell_margins.horizontal.to_string(),
                table.cell_margins.vertical.to_string(),
            ];
            let spacing = &table.spacing;
            if (spacing.top, spacing.left) != (TableSpacing::INSET, TableSpacing::INSET) {
                fields.push(spacing.top.to_string());
                fields.push(spacing.left.to_string());
            }
            out.record(&fields.iter().map(String::as_str).collect::<Vec<_>>());
            // A row or column with a gutter of its own (2026-10-18) has it
            // after its size.
            for (tracks, own, kind) in [
                (&table.columns, &spacing.column_gutters, "column"),
                (&table.rows, &spacing.row_gutters, "row"),
            ] {
                for (index, size) in tracks.iter().enumerate() {
                    match own.get(&index) {
                        Some(gutter) => out.record(&[kind, &size.to_string(), &gutter.to_string()]),
                        None => out.record(&[kind, &size.to_string()]),
                    }
                }
            }
            // How the rows take their height and continue (2026-10-18, the
            // tables across pages run), when not a new table's.
            let options = &table.row_options;
            if *options != TableRows::default() {
                let list = |rows: &std::collections::BTreeSet<usize>| {
                    if rows.is_empty() {
                        "-".to_owned()
                    } else {
                        rows.iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(",")
                    }
                };
                out.record_owned(&[
                    "tablerows".to_owned(),
                    u8::from(options.automatic).to_string(),
                    u8::from(options.span_pages).to_string(),
                    list(&options.headings),
                    list(&options.page_breaks),
                ]);
            }
            // Modify Table Layout's defaults and options and the rows and
            // columns with a size of their own (2026-10-24).
            let layout = &table.layout;
            let list = |items: &BTreeSet<usize>| {
                if items.is_empty() {
                    "-".to_owned()
                } else {
                    items
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                }
            };
            out.record_owned(&[
                "tablelayout".to_owned(),
                layout.row_height.to_string(),
                layout.column_width.to_string(),
                u8::from(layout.center).to_string(),
                u8::from(layout.honor_protection).to_string(),
                u8::from(layout.fixed_sizes).to_string(),
                list(&layout.own_heights),
                list(&layout.own_widths),
            ]);
            // Ami Pro's row heights as read (2026-10-20, the row cache).
            for (row, fingerprint) in &table.read_rows {
                out.record(&["readrow", &row.to_string(), &format!("{fingerprint:016x}")]);
            }
            for cell in &table.cells {
                let CellContent::Text(flow) = cell.content;
                let mut fields = vec![
                    "cell".to_owned(),
                    flow.0.to_string(),
                    cell.span.rows.to_string(),
                    cell.span.columns.to_string(),
                ];
                if let Some(frame) = cell.frame {
                    fields.push(frame.0.to_string());
                }
                out.record_owned(&fields);
                // A cell's lines, fill, leader, and protection (2026-10-24).
                let format = &cell.format;
                if !format.is_default() {
                    out.record_owned(&[
                        "cellformat".to_owned(),
                        format.lines.word().to_string(),
                        format!("{:06x}", format.line_colour),
                        format
                            .fill
                            .map_or_else(|| "-".to_owned(), |fill| format!("{fill:06x}")),
                        (format.leader.bits() >> 14).to_string(),
                        u8::from(format.protected).to_string(),
                    ]);
                }
                if let Some(formula) = &cell.formula {
                    out.record(&["cellformula", &escape(formula)]);
                }
                for value in &cell.legacy.values {
                    out.record(&["celllegacy", &escape(&value.key), &escape(&value.value)]);
                }
            }
            for value in &table.legacy.values {
                out.record(&["tablelegacy", &escape(&value.key), &escape(&value.value)]);
            }
        }
        FrameContent::Picture(picture) => {
            let (kind, source) = match &picture.source {
                PictureSource::Embedded(object) => ("embedded", object.0.to_string()),
                PictureSource::Linked(path) => ("linked", escape(path)),
            };
            out.record(&[
                "content",
                "picture",
                kind,
                &source,
                &escape(&picture.extension),
                &picture.rendering.map_or(0, |id| id.0).to_string(),
            ]);
            let scaling = &picture.scaling;
            out.record_owned(&[
                "picturescaling".to_owned(),
                match scaling.sizing {
                    PictureSizing::Original => "original",
                    PictureSizing::FitInFrame => "fit",
                    PictureSizing::Percentage => "percentage",
                    PictureSizing::Custom => "custom",
                }
                .to_owned(),
                u8::from(scaling.keep_aspect).to_string(),
                scaling.percent.to_string(),
                scaling.size.width.to_string(),
                scaling.size.height.to_string(),
                scaling.rotation.to_string(),
                picture.pan.x.to_string(),
                picture.pan.y.to_string(),
            ]);
            if !picture.description.is_empty() {
                out.record(&["picturedescription", &escape(&picture.description)]);
            }
            if picture.adjust != crate::ImageAdjust::default() {
                let adjust = &picture.adjust;
                out.record_owned(&[
                    "pictureadjust".to_owned(),
                    adjust.brightness.to_string(),
                    adjust.contrast.to_string(),
                    adjust.edge.to_string(),
                    adjust.smoothing.to_string(),
                    u8::from(adjust.invert).to_string(),
                    (adjust.halftone as u8).to_string(),
                ]);
            }
            for value in &picture.legacy.values {
                out.record(&["picturelegacy", &escape(&value.key), &escape(&value.value)]);
            }
        }
        FrameContent::Legacy(legacy) => {
            out.record(&["content", "legacy"]);
            for value in &legacy.values {
                out.record(&["contentlegacy", &escape(&value.key), &escape(&value.value)]);
            }
        }
    }
    for value in &frame.legacy.values {
        out.record(&["framelegacy", &escape(&value.key), &escape(&value.value)]);
    }
}

fn alignment_field(alignment: Alignment) -> &'static str {
    match alignment {
        Alignment::Left => "left",
        Alignment::Right => "right",
        Alignment::Centre => "centre",
        Alignment::Justify => "justify",
    }
}

fn decode_alignment(decoder: &Decoder<'_>, index: usize) -> Result<Alignment, DocumentError> {
    Ok(match decoder.fields[index] {
        "left" => Alignment::Left,
        "right" => Alignment::Right,
        "centre" => Alignment::Centre,
        "justify" => Alignment::Justify,
        _ => return Err(decoder.error("unknown alignment")),
    })
}

fn line_spacing_fields(line: LineSpacing) -> (&'static str, Twips) {
    match line {
        LineSpacing::Single => ("single", 0),
        LineSpacing::OneAndAHalf(height) => ("half", height),
        LineSpacing::Double(height) => ("double", height),
        LineSpacing::Custom(height) => ("custom", height),
    }
}

/// The character attributes `sformat` and `rformat` records name, in the
/// order they are written: eight on/off attributes, then `caps`, `colour`,
/// and `overstrike`.
const ATTRIBUTE_FLAGS: [&str; 8] = [
    "underline",
    "wordunderline",
    "doubleunderline",
    "strikethrough",
    "superscript",
    "subscript",
    "protected",
    "nohyphenation",
];

fn caps_field(caps: Caps) -> &'static str {
    match caps {
        Caps::Upper => "upper",
        Caps::Lower => "lower",
        Caps::Initial => "initial",
        Caps::Small => "small",
        Caps::Normal => "normal",
    }
}

fn attribute_flags(attributes: &CharacterAttributes) -> [bool; 8] {
    [
        attributes.underline,
        attributes.word_underline,
        attributes.double_underline,
        attributes.strikethrough,
        attributes.superscript,
        attributes.subscript,
        attributes.protected,
        attributes.no_hyphenation,
    ]
}

fn attribute_flags_mut(attributes: &mut CharacterAttributes) -> [&mut bool; 8] {
    [
        &mut attributes.underline,
        &mut attributes.word_underline,
        &mut attributes.double_underline,
        &mut attributes.strikethrough,
        &mut attributes.superscript,
        &mut attributes.subscript,
        &mut attributes.protected,
        &mut attributes.no_hyphenation,
    ]
}

/// A run's attributes beyond its `run` record, which holds underline.
fn format_flags_mut(format: &mut CharacterFormat) -> [&mut Option<bool>; 8] {
    [
        &mut format.underline,
        &mut format.word_underline,
        &mut format.double_underline,
        &mut format.strikethrough,
        &mut format.superscript,
        &mut format.subscript,
        &mut format.protected,
        &mut format.no_hyphenation,
    ]
}

/// One `sformat` record per attribute a style sets (2026-10-12).
/// A tab stop as `K:POSITION`, with `:h`, `:d`, or `:l` for a leader of
/// hyphens, dots, or a line (2026-10-12).
fn encode_tab_stop(tab: &TabStop) -> String {
    let kind = match tab.kind {
        TabKind::Left => 'L',
        TabKind::Centre => 'C',
        TabKind::Right => 'R',
        TabKind::Decimal => 'D',
    };
    let leader = match tab.leader {
        TabLeader::None => "",
        TabLeader::Hyphens => ":h",
        TabLeader::Dots => ":d",
        TabLeader::Line => ":l",
    };
    format!("{kind}:{}{leader}", tab.position)
}

/// A style's paragraph rules as Ami Pro's ten `[line]` values, only when
/// they are not the default (2026-10-12).
fn encode_style_rules(out: &mut Output, rules: ParagraphRules) {
    if rules == ParagraphRules::default() {
        return;
    }
    out.record(&[
        "sline",
        &rules.flags().to_string(),
        &rules.other_length.to_string(),
        &rules.unknown[0].to_string(),
        &rules.colour.to_string(),
        &rules.unknown[1].to_string(),
        &rules.above.line.to_string(),
        &rules.below.line.to_string(),
        &rules.above.space.to_string(),
        &rules.below.space.to_string(),
        &rules.unit.to_string(),
    ]);
}

fn encode_style_attributes(out: &mut Output, attributes: CharacterAttributes) {
    for (name, set) in ATTRIBUTE_FLAGS.iter().zip(attribute_flags(&attributes)) {
        if set {
            out.record(&["sformat", name, "1"]);
        }
    }
    if let Some(caps) = attributes.caps {
        out.record(&["sformat", "caps", caps_field(caps)]);
    }
    if attributes.colour != BLACK {
        out.record(&["sformat", "colour", &attributes.colour.to_string()]);
    }
    if let Some(overstrike) = attributes.overstrike {
        out.record(&["sformat", "overstrike", &escape(&overstrike.to_string())]);
    }
}

/// One `rformat` record per attribute a run sets beyond its `run` record
/// (2026-10-12).
fn encode_run_attributes(out: &mut Output, format: &CharacterFormat) {
    let flags = [
        format.word_underline,
        format.double_underline,
        format.strikethrough,
        format.superscript,
        format.subscript,
        format.protected,
        format.no_hyphenation,
    ];
    for (name, value) in ATTRIBUTE_FLAGS[1..].iter().zip(flags) {
        if let Some(value) = value {
            out.record(&["rformat", name, bool_field(value)]);
        }
    }
    if let Some(caps) = format.caps {
        out.record(&["rformat", "caps", caps_field(caps)]);
    }
    if let Some(colour) = format.colour {
        out.record(&["rformat", "colour", &colour.to_string()]);
    }
    if let Some(overstrike) = format.overstrike {
        out.record(&["rformat", "overstrike", &escape(&overstrike.to_string())]);
    }
}

/// A `run` or `endformat` record's format fields: family present, family,
/// size or `-`, bold, italic, underline.
fn format_fields(format: &CharacterFormat) -> [String; 6] {
    [
        bool_field(format.font_family.is_some()).to_owned(),
        escape(format.font_family.as_deref().unwrap_or("")),
        format
            .font_size_points
            .map_or_else(|| "-".to_owned(), |size| size.to_string()),
        optional_bool_field(format.bold).to_owned(),
        optional_bool_field(format.italic).to_owned(),
        optional_bool_field(format.underline).to_owned(),
    ]
}

fn encode_paragraph(out: &mut Output, paragraph: &Paragraph) {
    out.record(&[
        "paragraph",
        &paragraph.id.0.to_string(),
        &paragraph.style.0.to_string(),
        &escape(&paragraph.text),
    ]);
    // Direct paragraph formatting, one record per property (2026-10-12).
    if let Some(indents) = paragraph.format.indents {
        out.record(&[
            "pformat",
            "indents",
            &indents.all.to_string(),
            &indents.first.to_string(),
            &indents.rest.to_string(),
            &indents.right.to_string(),
        ]);
    }
    if let Some(line) = paragraph.format.line_spacing {
        let (kind, height) = line_spacing_fields(line);
        out.record(&["pformat", "line", kind, &height.to_string()]);
    }
    if let Some(alignment) = paragraph.format.alignment {
        out.record(&["pformat", "align", alignment_field(alignment)]);
    }
    if let Some(tabs) = &paragraph.format.tabs {
        let mut fields = vec!["pformat".to_owned(), "tabs".to_owned()];
        fields.extend(tabs.iter().map(encode_tab_stop));
        out.record_owned(&fields);
    }
    for run in &paragraph.runs {
        let mut fields = vec!["run".to_owned(), run.start.to_string(), run.end.to_string()];
        fields.extend(format_fields(&run.format));
        out.record(&fields.iter().map(String::as_str).collect::<Vec<_>>());
        encode_run_attributes(out, &run.format);
    }
    // The paragraph's end format, only when it has one (2026-10-12).
    if !paragraph.end_format.is_empty() {
        let mut fields = vec!["endformat".to_owned()];
        fields.extend(format_fields(&paragraph.end_format));
        out.record(&fields.iter().map(String::as_str).collect::<Vec<_>>());
        encode_run_attributes(out, &paragraph.end_format);
    }
    for object in &paragraph.objects {
        match object {
            InlineObject::Break {
                kind,
                centre,
                layout,
                restart,
            } => {
                let layout = layout.map_or(0, |id| id.0).to_string();
                let mut fields = vec![
                    "inline",
                    "break",
                    match kind {
                        BreakKind::Page => "page",
                        BreakKind::Column => "column",
                    },
                    bool_field(*centre),
                    &layout,
                ];
                // The TOC break's restart (2026-11-03), only when set.
                if *restart {
                    fields.push("restart");
                }
                out.record(&fields);
            }
            InlineObject::FloatingHeaderFooter { kind, pages, flow } => out.record(&[
                "inline",
                "floating",
                match kind {
                    HeaderFooterKind::Header => "header",
                    HeaderFooterKind::Footer => "footer",
                },
                match pages {
                    PageSides::All => "all",
                    PageSides::Right => "right",
                    PageSides::Left => "left",
                },
                &flow.0.to_string(),
            ]),
            InlineObject::FrameAnchor(frame) => {
                out.record(&["inline", "anchor", &frame.0.to_string()]);
            }
            InlineObject::PageNumber(field) => out.record(&[
                "inline",
                "pagenumber",
                &escape(&field.style.to_string()),
                &field.start_page.to_string(),
                &field.start_number.to_string(),
                &escape(&field.leading_text),
            ]),
            InlineObject::Bullet(code) => {
                out.record(&["inline", "bullet", &(*code as u32).to_string()]);
            }
            InlineObject::Date(field) => out.record(&[
                "inline",
                "date",
                match field.source {
                    DateSource::Today => "today",
                    DateSource::LastSaved => "saved",
                    DateSource::Created => "created",
                    DateSource::Time => "time",
                },
                &field.format.to_string(),
            ]),
            InlineObject::DocInfo(field) => out.record(&[
                "inline",
                "docinfo",
                &field.code.to_string(),
                &field.format.to_string(),
            ]),
            InlineObject::Seq(field) => out.record(&[
                "inline",
                "seq",
                &field.value.to_string(),
                &escape(&field.code),
            ]),
            InlineObject::Note { note, flow } => out.record(&[
                "inline",
                "note",
                &flow.0.to_string(),
                &note.time.to_string(),
                &note
                    .initials
                    .as_deref()
                    .map_or_else(String::new, |initials| format!("={}", escape(initials))),
                &note
                    .colour
                    .map_or_else(String::new, |colour| colour.to_string()),
                &note
                    .number
                    .map_or_else(String::new, |number| number.to_string()),
            ]),
            InlineObject::Footnote { flow } => {
                out.record(&["inline", "footnote", &flow.0.to_string()]);
            }
            InlineObject::TableFootnote { number, row } => out.record(&[
                "inline",
                "tablefootnote",
                &number.to_string(),
                &row.to_string(),
            ]),
        }
    }
    for mark in &paragraph.marks {
        out.record(&["mark", &mark.byte.to_string(), &escape(&mark.raw)]);
    }
    // Ami Pro's layout cache, as read (Phase 4.9).
    if let Some(cache) = paragraph.cache {
        out.record(&[
            "pcache",
            &cache.height.to_string(),
            &cache.width.to_string(),
            &format!("{:016x}", cache.fingerprint),
        ]);
    }
}

// ---------------------------------------------------------------- decoding

/// An `sline` record: the ten `[line]` values (`encode_style_rules`).
fn decode_rules(decoder: &Decoder<'_>) -> Result<ParagraphRules, DocumentError> {
    let flags: u16 = decoder.number(1)?;
    let colour: Colour = decoder.number(4)?;
    if colour > crate::WHITE {
        return Err(decoder.error("a rule colour is a COLORREF, 0x00BBGGRR"));
    }
    let side = |on: bool, line: usize, space: usize| -> Result<RuleSide, DocumentError> {
        Ok(RuleSide {
            on,
            line: decoder.number(line)?,
            space: decoder.number(space)?,
        })
    };
    Ok(ParagraphRules {
        above: side(flags & 1 != 0, 6, 8)?,
        below: side(flags & 2 != 0, 7, 9)?,
        length: if flags & 4 != 0 {
            RuleLength::Text
        } else if flags & 8 != 0 {
            RuleLength::Margins
        } else {
            RuleLength::Other
        },
        other_length: decoder.number(2)?,
        colour,
        other_flags: flags & !0xF,
        unknown: [decoder.number(3)?, decoder.number(5)?],
        unit: decoder.number(10)?,
    })
}

/// The format fields of a `run` or `endformat` record from field `first`
/// on (`format_fields`).
fn decode_format_fields(
    decoder: &Decoder<'_>,
    first: usize,
) -> Result<CharacterFormat, DocumentError> {
    let has_family = decoder.flag(first)?;
    let family = decoder.text(first + 1)?;
    let size = match decoder.fields[first + 2] {
        "-" => None,
        _ => Some(decoder.number(first + 2)?),
    };
    Ok(CharacterFormat {
        font_family: has_family.then_some(family),
        font_size_points: size,
        bold: decoder.optional_flag(first + 3)?,
        italic: decoder.optional_flag(first + 4)?,
        underline: decoder.optional_flag(first + 5)?,
        ..CharacterFormat::default()
    })
}

/// The sections of a v3 file, in their required order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Section {
    Title,
    Ids,
    StyleSheet,
    Styles,
    LineHeights,
    FaceKinds,
    DocLegacy,
    Objects,
    Layouts,
    Frames,
    Stack,
    Flows,
}

struct LayoutBuilder {
    layout: PageLayout,
    right: Option<PageBuilder>,
    left: Option<PageBuilder>,
    /// 0 pages, 1 right page done, 2 left page done, 3 slots, 4 repeating.
    stage: u8,
}

struct PageBuilder {
    width: i32,
    height: i32,
    layout: Option<BoxBuilder>,
    border: Option<Border>,
}

struct BoxBuilder {
    layout: LayoutBox,
    tabs_seen: usize,
}

struct FrameBuilder {
    id: FrameId,
    group: Option<GroupId>,
    placement: Option<Placement>,
    style: FrameStyle,
    interior: Option<BoxBuilder>,
    content: Option<FrameContent>,
    legacy: LegacyData,
    /// The last frame record seen, by its position in the frame order.
    stage: u8,
    /// The column and row counts a table content record declared.
    table_shape: (usize, usize),
    /// Whether a table's layout record was read; files written before it
    /// existed infer the defaults from the sizes.
    table_laid_out: bool,
    /// Whether a picture's scaling record was read; files written before it
    /// existed take the scaling from the kept `[isd]` values.
    picture_scaled: bool,
}

#[derive(Default)]
struct ParagraphBuilder {
    paragraph: Option<Paragraph>,
    /// 0 paragraph formats, 1 runs, 2 the end format, 3 inline objects, 4
    /// marks.
    stage: u8,
}

struct Decoder<'a> {
    line: usize,
    fields: Vec<&'a str>,
}

impl<'a> Decoder<'a> {
    fn error(&self, message: &str) -> DocumentError {
        DocumentError::Parse {
            line: self.line,
            message: message.into(),
        }
    }

    fn expect_len(&self, len: usize) -> Result<(), DocumentError> {
        if self.fields.len() == len {
            Ok(())
        } else {
            Err(self.error("wrong number of fields"))
        }
    }

    fn text(&self, index: usize) -> Result<String, DocumentError> {
        unescape(self.fields[index]).map_err(|message| self.error(&message))
    }

    fn number<T: std::str::FromStr>(&self, index: usize) -> Result<T, DocumentError> {
        self.fields[index]
            .parse()
            .map_err(|_| self.error("malformed number"))
    }

    fn id(&self, index: usize) -> Result<u64, DocumentError> {
        let id: u64 = self.number(index)?;
        if id == 0 {
            Err(self.error("IDs are positive"))
        } else {
            Ok(id)
        }
    }

    fn optional_id(&self, index: usize) -> Result<Option<u64>, DocumentError> {
        let id: u64 = self.number(index)?;
        Ok((id != 0).then_some(id))
    }

    fn flag(&self, index: usize) -> Result<bool, DocumentError> {
        match self.fields[index] {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(self.error("malformed boolean")),
        }
    }

    fn optional_flag(&self, index: usize) -> Result<Option<bool>, DocumentError> {
        match self.fields[index] {
            "-" => Ok(None),
            "0" => Ok(Some(false)),
            "1" => Ok(Some(true)),
            _ => Err(self.error("malformed optional boolean")),
        }
    }

    fn rect(&self, from: usize) -> Result<Rect, DocumentError> {
        Ok(Rect::new(
            self.number(from)?,
            self.number(from + 1)?,
            self.number(from + 2)?,
            self.number(from + 3)?,
        ))
    }

    fn side(&self, index: usize) -> Result<PageSide, DocumentError> {
        match self.fields[index] {
            "right" => Ok(PageSide::Right),
            "left" => Ok(PageSide::Left),
            _ => Err(self.error("malformed page side")),
        }
    }

    fn border(&self, from: usize) -> Result<Border, DocumentError> {
        let sides_field = self.fields[from];
        let mut sides = Sides::NONE;
        if sides_field != "-" {
            for letter in sides_field.chars() {
                let side = match letter {
                    'L' => &mut sides.left,
                    'R' => &mut sides.right,
                    'T' => &mut sides.top,
                    'B' => &mut sides.bottom,
                    _ => return Err(self.error("malformed border sides")),
                };
                if *side {
                    return Err(self.error("malformed border sides"));
                }
                *side = true;
            }
            if sides.is_none() {
                return Err(self.error("malformed border sides"));
            }
        }
        let position = match self.number::<u8>(from + 3)? {
            1 => LinePosition::Middle,
            2 => LinePosition::Inside,
            3 => LinePosition::Outside,
            4 => LinePosition::CloseToInside,
            5 => LinePosition::CloseToOutside,
            _ => return Err(self.error("line position must be 1 to 5")),
        };
        Ok(Border {
            sides,
            style: self.number(from + 1)?,
            colour: self.number(from + 2)?,
            position,
        })
    }

    fn layout_box(&self) -> Result<BoxBuilder, DocumentError> {
        self.expect_len(10)?;
        let line_between = match (self.fields[8], self.fields[9]) {
            ("-", "-") => None,
            _ => Some(ColumnLine {
                style: self.number(8)?,
                colour: self.number(9)?,
            }),
        };
        let columns: u8 = self.number(5)?;
        Ok(BoxBuilder {
            layout: LayoutBox {
                margins: Margins {
                    top: self.number(1)?,
                    right: self.number(2)?,
                    bottom: self.number(3)?,
                    left: self.number(4)?,
                },
                columns,
                gutter: self.number(6)?,
                column_edges: None,
                tabs: Vec::new(),
                balance: self.flag(7)?,
                line_between,
            },
            tabs_seen: 0,
        })
    }

    /// Uneven columns' edges, after the box and before its tabs, at most
    /// once (2026-10-17).
    fn edges(&self, builder: &mut BoxBuilder) -> Result<(), DocumentError> {
        if builder.tabs_seen != 0 || builder.layout.column_edges.is_some() {
            return Err(self.error("edges follow their box, before its tabs, once"));
        }
        self.expect_len(1 + 2 * usize::from(builder.layout.columns))?;
        let edges = (0..usize::from(builder.layout.columns))
            .map(|column| {
                Ok(ColumnEdges {
                    left: self.number(1 + 2 * column)?,
                    right: self.number(2 + 2 * column)?,
                })
            })
            .collect::<Result<Vec<_>, DocumentError>>()?;
        builder.layout.column_edges = Some(edges);
        Ok(())
    }

    fn tabs(&self, builder: &mut BoxBuilder) -> Result<(), DocumentError> {
        if self.fields.len() < 2 {
            return Err(self.error("wrong number of fields"));
        }
        let column: usize = self.number(1)?;
        if column != builder.tabs_seen || column >= usize::from(builder.layout.columns) {
            return Err(self.error("tab lists must follow their columns in order"));
        }
        let tabs = self.fields[2..]
            .iter()
            .map(|stop| self.tab_stop(stop))
            .collect::<Result<Vec<_>, _>>()?;
        builder.layout.tabs.push(tabs);
        builder.tabs_seen += 1;
        Ok(())
    }

    /// A tab stop as `encode_tab_stop` writes it.
    fn tab_stop(&self, stop: &str) -> Result<TabStop, DocumentError> {
        let mut parts = stop.split(':');
        let (Some(kind), Some(position)) = (parts.next(), parts.next()) else {
            return Err(self.error("malformed tab stop"));
        };
        let kind = match kind {
            "L" => TabKind::Left,
            "C" => TabKind::Centre,
            "R" => TabKind::Right,
            "D" => TabKind::Decimal,
            _ => return Err(self.error("malformed tab kind")),
        };
        let position = position
            .parse()
            .map_err(|_| self.error("malformed tab position"))?;
        let leader = match (parts.next(), parts.next()) {
            (None, _) => TabLeader::None,
            (Some("h"), None) => TabLeader::Hyphens,
            (Some("d"), None) => TabLeader::Dots,
            (Some("l"), None) => TabLeader::Line,
            _ => return Err(self.error("malformed tab leader")),
        };
        Ok(TabStop {
            kind,
            position,
            leader,
        })
    }

    fn legacy(&self, data: &mut LegacyData) -> Result<(), DocumentError> {
        self.expect_len(3)?;
        data.push(self.text(1)?, self.text(2)?);
        Ok(())
    }
}

fn finish_box(builder: Option<BoxBuilder>, line: usize) -> Result<LayoutBox, DocumentError> {
    let builder = builder.ok_or(DocumentError::Parse {
        line,
        message: "a page or frame needs its box".into(),
    })?;
    if builder.tabs_seen != usize::from(builder.layout.columns) {
        return Err(DocumentError::Parse {
            line,
            message: "a box needs one tab list per column".into(),
        });
    }
    Ok(builder.layout)
}

fn finish_page(
    builder: Option<PageBuilder>,
    line: usize,
) -> Result<Option<PageGeometry>, DocumentError> {
    builder
        .map(|page| {
            Ok(PageGeometry {
                width: page.width,
                height: page.height,
                layout: finish_box(page.layout, line)?,
                border: page.border,
            })
        })
        .transpose()
}

pub(crate) fn decode(input: &str) -> Result<Document, DocumentError> {
    let mut lines = input.lines().enumerate();
    match lines.next() {
        Some((_, MAGIC_V3)) => {}
        _ => {
            return Err(DocumentError::Parse {
                line: 1,
                message: "invalid native document signature".into(),
            });
        }
    }

    let mut section = Section::Title;
    let mut title = None;
    let mut ids: Option<(u64, FlowId)> = None;
    let mut styles = BTreeMap::new();
    // The style or run that `sformat` or `rformat` records extend, and the
    // attributes they have set so far (bits in `ATTRIBUTE_FLAGS` order, then
    // caps and colour).
    let mut last_style: Option<StyleId> = None;
    let mut attributes_seen = 0u16;
    // A style's children in order: sformat (0), sline (1), snext (2),
    // stabs (3), shanging (4), shyphenation (5), sbullets (6), sorigin (7),
    // skept (8).
    let mut style_stage = 0u8;
    let mut line_heights = LineHeights::default();
    let mut face_kinds = BTreeMap::new();
    let mut style_sheet = None;
    let mut hyphenation_zone = None;
    let mut language: Option<crate::Language> = None;
    let mut grammar_set: Option<String> = None;
    let mut contents = crate::contents::ContentsSettings::default();
    let mut merge = crate::merge::MergeSettings::default();
    let mut footnote_options: Option<crate::FootnoteOptions> = None;
    let mut line_numbering: Option<crate::LineNumbering> = None;
    let mut info: Option<DocumentInfo> = None;
    let mut start_layout: Option<LayoutId> = None;
    let mut legacy = LegacyData::default();
    let mut objects: Vec<(ObjectId, usize, String)> = Vec::new();
    let mut layouts: Vec<LayoutBuilder> = Vec::new();
    let mut frames: Vec<FrameBuilder> = Vec::new();
    let mut stack: Option<Vec<FrameId>> = None;
    let mut flows: Vec<(FlowId, Vec<Paragraph>)> = Vec::new();
    let mut paragraph = ParagraphBuilder::default();
    let mut last_line = 1;

    for (index, line) in lines {
        let decoder = Decoder {
            line: index + 1,
            fields: line.split('\t').collect(),
        };
        last_line = index + 1;
        let record = decoder.fields[0];
        let record_section = match record {
            "title" => Section::Title,
            "ids" => Section::Ids,
            "stylesheet" | "hyphenzone" | "docinfo" | "docfield" | "footnotes"
            | "linenumbering" | "language" | "grammarset" | "toc" | "master" | "sequence"
            | "recfile" | "mergecondition" | "withconditions" => Section::StyleSheet,
            "style" | "sformat" | "sline" | "snext" | "stabs" | "shanging" | "shyphenation"
            | "sbullets" | "sorigin" | "skept" | "sfirstbold" => Section::Styles,
            "lineheight" => Section::LineHeights,
            "facekind" => Section::FaceKinds,
            "doclegacy" => Section::DocLegacy,
            "object" | "blob" => Section::Objects,
            "layout" | "page" | "pageborder" | "slots" | "repeating" | "layoutlegacy"
            | "startlayout" => Section::Layouts,
            "box" | "edges" | "tabs" if section <= Section::Layouts => Section::Layouts,
            "frame" | "placement" | "look" | "border" | "shadow" | "box" | "edges" | "tabs"
            | "name" | "macro" | "content" | "column" | "row" | "tablerows" | "tablelayout"
            | "readrow" | "cell" | "cellformat" | "cellformula" | "celllegacy" | "tablelegacy"
            | "picturescaling" | "picturedescription" | "pictureadjust" | "picturelegacy"
            | "contentlegacy" | "framelegacy" => Section::Frames,
            "stack" => Section::Stack,
            "flow" | "paragraph" | "pformat" | "run" | "endformat" | "rformat" | "inline"
            | "pcache" | "mark" => Section::Flows,
            _ => return Err(decoder.error("unknown record")),
        };
        if record_section < section {
            return Err(decoder.error("record out of order"));
        }
        let entering = record_section > section;
        section = record_section;
        let singleton_taken = match record {
            "title" => title.is_some(),
            "ids" => ids.is_some(),
            "stack" => stack.is_some(),
            _ => false,
        };
        if singleton_taken {
            return Err(decoder.error("record may appear only once"));
        }
        if entering && section > Section::Title && title.is_none() {
            return Err(decoder.error("the title comes first"));
        }
        if entering && section > Section::Ids && ids.is_none() {
            return Err(decoder.error("the ids record comes second"));
        }

        match record {
            "title" => {
                decoder.expect_len(2)?;
                title = Some(decoder.text(1)?);
            }
            "ids" => {
                decoder.expect_len(3)?;
                ids = Some((decoder.id(1)?, FlowId(decoder.id(2)?)));
            }
            "style" => {
                // Nine fields; thirteen with the indents; eighteen with the
                // indents and the spacing; nineteen with the break rules too;
                // twenty with an alignment other than left (2026-10-12).
                let length = decoder.fields.len();
                if ![9, 13, 18, 19, 20].contains(&length) {
                    return Err(decoder.error("a style record has 9, 13, 18, 19, or 20 fields"));
                }
                let indents = if length >= 13 {
                    Indents {
                        all: decoder.number(9)?,
                        first: decoder.number(10)?,
                        rest: decoder.number(11)?,
                        right: decoder.number(12)?,
                    }
                } else {
                    Indents::default()
                };
                let below = crate::points_to_twips(decoder.number(8)?);
                let breaks = if length >= 19 {
                    Breaks(decoder.number(18)?)
                } else {
                    Breaks::default()
                };
                let alignment = if length == 20 {
                    decode_alignment(&decoder, 19)?
                } else {
                    Alignment::Left
                };
                let spacing = if length >= 18 {
                    let height = decoder.number(14)?;
                    Spacing {
                        line: decode_line_spacing(&decoder, 13, height)?,
                        above: decoder.number(15)?,
                        below,
                        add_at_break: decoder.flag(16)?,
                        tightness: decoder.number(17)?,
                    }
                } else {
                    Spacing::below(below)
                };
                let id = StyleId(decoder.id(1)?);
                let style = ParagraphStyle {
                    id,
                    name: decoder.text(2)?,
                    parent: decoder.optional_id(3)?.map(StyleId),
                    font_family: decoder.text(4)?,
                    font_size_points: decoder.number(5)?,
                    bold: decoder.flag(6)?,
                    italic: decoder.flag(7)?,
                    spacing,
                    indents,
                    breaks,
                    alignment,
                    attributes: CharacterAttributes::default(),
                    rules: ParagraphRules::default(),
                    next: None,
                    tabs: Vec::new(),
                    hanging: false,
                    hyphenation: false,
                    bullets: Bullets::default(),
                    origin: StyleOrigin::Document,
                    both_sides: false,
                    first_line_bold: false,
                    kept: StyleKept::default(),
                };
                if styles.insert(id, style).is_some() {
                    return Err(decoder.error("duplicate style ID"));
                }
                last_style = Some(id);
                attributes_seen = 0;
                style_stage = 0;
            }
            "sline" => {
                decoder.expect_len(11)?;
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("sline without a style"))?;
                if style_stage >= 1 {
                    return Err(decoder.error("a style has one sline record, before snext"));
                }
                style_stage = 1;
                style.rules = decode_rules(&decoder)?;
            }
            "stabs" => {
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("stabs without a style"))?;
                if style_stage >= 3 || decoder.fields.len() < 2 {
                    return Err(decoder.error("a style has one stabs record with stops"));
                }
                style_stage = 3;
                style.tabs = decoder.fields[1..]
                    .iter()
                    .map(|stop| decoder.tab_stop(stop))
                    .collect::<Result<_, _>>()?;
            }
            "sbullets" => {
                decoder.expect_len(11)?;
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("sbullets without a style"))?;
                if style_stage >= 6 {
                    return Err(decoder.error("a style has one sbullets record, last"));
                }
                style_stage = 6;
                style.bullets = Bullets {
                    text: decoder.text(1)?,
                    level: decoder.number(2)?,
                    attributes: decoder.number(3)?,
                    flags: decoder.number(4)?,
                    kept: [
                        decoder.number(5)?,
                        decoder.number(6)?,
                        decoder.number(7)?,
                        decoder.number(8)?,
                        decoder.number(9)?,
                        decoder.number(10)?,
                    ],
                };
            }
            "sorigin" => {
                decoder.expect_len(2)?;
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("sorigin without a style"))?;
                if style_stage >= 7 {
                    return Err(decoder.error("a style has one sorigin record, last"));
                }
                style_stage = 7;
                style.origin = match decoder.fields[1] {
                    "sheet" => StyleOrigin::Sheet,
                    "missing" => StyleOrigin::Missing,
                    _ => return Err(decoder.error("unknown style origin")),
                };
            }
            "skept" => {
                decoder.expect_len(18)?;
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("skept without a style"))?;
                if style_stage >= 8 {
                    return Err(decoder.error("a style has one skept record, last"));
                }
                style_stage = 8;
                style.both_sides = decoder.flag(1)?;
                style.kept = StyleKept {
                    function_key: decoder.number(2)?,
                    face_bits: decoder.number(3)?,
                    units: [decoder.number(4)?, decoder.number(5)?, decoder.number(6)?],
                    add_in: decoder.number(7)?,
                    single_height: decoder.number(8)?,
                    number_format: NumberFormat::from_values(&[
                        decoder.text(9)?,
                        decoder.text(10)?,
                        decoder.text(11)?,
                        decoder.text(12)?,
                        decoder.text(13)?,
                        decoder.text(14)?,
                    ])
                    .ok_or_else(|| decoder.error("a number format has numbers"))?,
                    next_name: decoder.flag(15)?.then(|| decoder.text(16)).transpose()?,
                    alignment_bits: decoder.number(17)?,
                };
            }
            "sfirstbold" => {
                decoder.expect_len(1)?;
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("sfirstbold without a style"))?;
                if style_stage >= 9 {
                    return Err(decoder.error("a style has one sfirstbold record, last"));
                }
                style_stage = 9;
                style.first_line_bold = true;
            }
            "docinfo" => {
                decoder.expect_len(14)?;
                if info.is_some() {
                    return Err(decoder.error("one docinfo record"));
                }
                let optional = |index: usize| -> Result<Option<u32>, DocumentError> {
                    if decoder.fields[index].is_empty() {
                        Ok(None)
                    } else {
                        decoder.number(index).map(Some)
                    }
                };
                info = Some(DocumentInfo {
                    created: optional(1)?,
                    saved: optional(2)?,
                    revisions: decoder.number(3)?,
                    editing_minutes: decoder.number(4)?,
                    statistics: Statistics {
                        pages: decoder.number(5)?,
                        words: decoder.number(6)?,
                        characters: decoder.number(7)?,
                        size: decoder.number(8)?,
                        size_with_links: decoder.number(9)?,
                    },
                    flags: decoder.number(10)?,
                    description: decoder.text(11)?,
                    keywords: decoder.text(12)?,
                    annotation_lock: decoder.text(13)?,
                    ..DocumentInfo::default()
                });
            }
            "docfield" => {
                decoder.expect_len(4)?;
                let info = info
                    .as_mut()
                    .ok_or_else(|| decoder.error("docfield without docinfo"))?;
                let index: usize = decoder.number(1)?;
                if !(1..=8).contains(&index) {
                    return Err(decoder.error("Doc Info fields are 1 to 8"));
                }
                info.field_names[index - 1] = decoder.text(2)?;
                info.fields[index - 1] = decoder.text(3)?;
            }
            "footnotes" => {
                decoder.expect_len(7)?;
                if footnote_options.is_some() {
                    return Err(decoder.error("one footnotes record"));
                }
                footnote_options = Some(crate::FootnoteOptions {
                    flags: decoder.number(1)?,
                    start: decoder.number(2)?,
                    length: decoder.number(3)?,
                    indent: decoder.number(4)?,
                    continued_on: decoder.text(5)?,
                    continued_from: decoder.text(6)?,
                });
            }
            "linenumbering" => {
                decoder.expect_len(4)?;
                if line_numbering.is_some() {
                    return Err(decoder.error("one linenumbering record"));
                }
                line_numbering = Some(crate::LineNumbering {
                    flags: decoder.number(1)?,
                    style: decoder.text(2)?,
                    interval: decoder.number(3)?,
                });
            }
            "language" => {
                if language.is_some() {
                    return Err(decoder.error("one language record"));
                }
                let locale = match decoder.fields.len() {
                    2 => None,
                    3 => Some(decoder.text(2)?),
                    _ => return Err(decoder.error("language takes a number and a locale")),
                };
                language = Some(crate::Language {
                    number: decoder.number(1)?,
                    locale,
                });
            }
            "recfile" => {
                let values = (1..decoder.fields.len())
                    .map(|index| decoder.text(index))
                    .collect::<Result<Vec<_>, _>>()?;
                if merge.record_file.is_some() {
                    return Err(decoder.error("one recfile record"));
                }
                merge.record_file = Some(
                    crate::merge::RecordFile::from_values(&values)
                        .ok_or_else(|| decoder.error("recfile is Ami Pro's [recfile]"))?,
                );
            }
            "mergecondition" => {
                decoder.expect_len(5)?;
                merge.conditions.push(crate::merge::Condition {
                    field: decoder.text(1)?,
                    operator: crate::merge::Operator::from_symbol(decoder.fields[2])
                        .ok_or_else(|| decoder.error("a condition's operator"))?,
                    value: decoder.text(3)?,
                    join: match decoder.fields[4] {
                        "and" => crate::merge::Join::And,
                        "or" => crate::merge::Join::Or,
                        _ => return Err(decoder.error("a condition joins by and or or")),
                    },
                });
            }
            "withconditions" => {
                decoder.expect_len(1)?;
                merge.with_conditions = true;
            }
            "toc" | "master" | "sequence" => {
                let values = (1..decoder.fields.len())
                    .map(|index| decoder.text(index))
                    .collect::<Result<Vec<_>, _>>()?;
                if record == "toc" {
                    if contents.toc.is_some() {
                        return Err(decoder.error("one toc record"));
                    }
                    contents.toc = Some(
                        crate::contents::TocSettings::from_values(&values)
                            .ok_or_else(|| decoder.error("toc is Ami Pro's [toc]"))?,
                    );
                } else if record == "sequence" {
                    if contents.sequence.is_some() {
                        return Err(decoder.error("one sequence record"));
                    }
                    contents.sequence = Some(
                        crate::contents::Sequence::from_values(&values)
                            .ok_or_else(|| decoder.error("sequence is Ami Pro's [sequence]"))?,
                    );
                } else {
                    if contents.master.is_some() {
                        return Err(decoder.error("one master record"));
                    }
                    contents.master = Some(
                        crate::contents::MasterSettings::from_values(&values)
                            .ok_or_else(|| decoder.error("master is Ami Pro's [master]"))?,
                    );
                }
            }
            "grammarset" => {
                decoder.expect_len(2)?;
                if grammar_set.is_some() {
                    return Err(decoder.error("one grammarset record"));
                }
                grammar_set = Some(decoder.text(1)?);
            }
            "hyphenzone" => {
                decoder.expect_len(2)?;
                if hyphenation_zone.is_some() {
                    return Err(decoder.error("one hyphenzone record"));
                }
                hyphenation_zone = Some(decoder.number(1)?);
            }
            "shyphenation" => {
                decoder.expect_len(1)?;
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("shyphenation without a style"))?;
                if style_stage >= 5 {
                    return Err(
                        decoder.error("a style has one shyphenation record, after shanging")
                    );
                }
                style_stage = 5;
                style.hyphenation = true;
            }
            "stylesheet" => {
                decoder.expect_len(3)?;
                if style_sheet.is_some() {
                    return Err(decoder.error("one stylesheet record"));
                }
                style_sheet = Some(StyleSheetLink {
                    name: decoder.text(1)?,
                    layout_from_sheet: decoder.flag(2)?,
                });
            }
            "shanging" => {
                decoder.expect_len(1)?;
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("shanging without a style"))?;
                if style_stage >= 4 {
                    return Err(decoder.error("a style has one shanging record, before sbullets"));
                }
                style_stage = 4;
                style.hanging = true;
            }
            "snext" => {
                decoder.expect_len(2)?;
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("snext without a style"))?;
                if style_stage >= 2 {
                    return Err(decoder.error("a style has one snext record"));
                }
                style_stage = 2;
                style.next = Some(StyleId(decoder.id(1)?));
            }
            "sformat" => {
                if style_stage > 0 {
                    return Err(decoder.error("sformat records come before sline and snext"));
                }
                let style = last_style
                    .and_then(|id| styles.get_mut(&id))
                    .ok_or_else(|| decoder.error("sformat without a style"))?;
                let attributes = &mut style.attributes;
                match decode_attribute(&decoder, &mut attributes_seen)? {
                    Attribute::Flag(index, value) => {
                        *attribute_flags_mut(attributes)[index] = value
                    }
                    Attribute::Caps(caps) => attributes.caps = Some(caps),
                    Attribute::Colour(colour) => attributes.colour = colour,
                    Attribute::Overstrike(overstrike) => attributes.overstrike = Some(overstrike),
                }
            }
            "lineheight" => {
                decoder.expect_len(6)?;
                let key = FaceKey {
                    family: decoder.text(1)?,
                    size: decoder.number(2)?,
                    bold: decoder.flag(3)?,
                    italic: decoder.flag(4)?,
                };
                if line_heights.get(&key).is_some() {
                    return Err(decoder.error("duplicate line height"));
                }
                line_heights.insert(key, decoder.number(5)?);
            }
            "facekind" => {
                decoder.expect_len(3)?;
                let family = decoder.text(1)?;
                let kind = decoder.number(2)?;
                if face_kinds.insert(family, kind).is_some() {
                    return Err(decoder.error("duplicate face kind"));
                }
            }
            "doclegacy" => decoder.legacy(&mut legacy)?,
            "object" => {
                decoder.expect_len(3)?;
                objects.push((ObjectId(decoder.id(1)?), decoder.number(2)?, String::new()));
            }
            "blob" => {
                decoder.expect_len(2)?;
                let (_, _, data) = objects
                    .last_mut()
                    .ok_or_else(|| decoder.error("blob without an object"))?;
                data.push_str(decoder.fields[1]);
            }
            "layout" => {
                decoder.expect_len(8)?;
                if start_layout.is_some() {
                    return Err(decoder.error("startlayout follows every layout"));
                }
                let id = LayoutId(decoder.id(1)?);
                let mut layout = PageLayout::letter(id, decoder.text(2)?);
                layout.paper = decoder.number(3)?;
                layout.landscape = decoder.flag(4)?;
                layout.mirror = decoder.flag(5)?;
                layout.header_from_second_page = decoder.flag(6)?;
                layout.footer_from_second_page = decoder.flag(7)?;
                layouts.push(LayoutBuilder {
                    layout,
                    right: None,
                    left: None,
                    stage: 0,
                });
            }
            "page" => {
                decoder.expect_len(4)?;
                let builder = layouts
                    .last_mut()
                    .ok_or_else(|| decoder.error("page without a layout"))?;
                let page = PageBuilder {
                    width: decoder.number(2)?,
                    height: decoder.number(3)?,
                    layout: None,
                    border: None,
                };
                match (decoder.side(1)?, builder.stage) {
                    (PageSide::Right, 0) => {
                        builder.right = Some(page);
                        builder.stage = 1;
                    }
                    (PageSide::Left, 1) => {
                        builder.left = Some(page);
                        builder.stage = 2;
                    }
                    _ => return Err(decoder.error("pages come right, then left, once each")),
                }
            }
            "box" | "edges" | "tabs" | "pageborder" if section == Section::Layouts => {
                let builder = layouts
                    .last_mut()
                    .ok_or_else(|| decoder.error("page record without a layout"))?;
                let page = match builder.stage {
                    1 => builder.right.as_mut(),
                    2 => builder.left.as_mut(),
                    _ => None,
                }
                .ok_or_else(|| decoder.error("page record without a page"))?;
                match record {
                    "box" if page.layout.is_none() => page.layout = Some(decoder.layout_box()?),
                    "edges" if page.border.is_none() => decoder.edges(
                        page.layout
                            .as_mut()
                            .ok_or_else(|| decoder.error("edges without a box"))?,
                    )?,
                    "tabs" if page.border.is_none() => decoder.tabs(
                        page.layout
                            .as_mut()
                            .ok_or_else(|| decoder.error("tabs without a box"))?,
                    )?,
                    "pageborder" if page.layout.is_some() && page.border.is_none() => {
                        decoder.expect_len(5)?;
                        page.border = Some(decoder.border(1)?);
                    }
                    _ => return Err(decoder.error("page record out of order")),
                }
            }
            "slots" => {
                decoder.expect_len(5)?;
                let builder = layouts
                    .last_mut()
                    .ok_or_else(|| decoder.error("slots without a layout"))?;
                if !(1..=2).contains(&builder.stage) {
                    return Err(decoder.error("slots follow the pages, once"));
                }
                let frame = |index| decoder.optional_id(index).map(|id| id.map(FrameId));
                builder.layout.header = SideFrames {
                    right: frame(1)?,
                    left: frame(2)?,
                };
                builder.layout.footer = SideFrames {
                    right: frame(3)?,
                    left: frame(4)?,
                };
                builder.stage = 3;
            }
            "repeating" => {
                let builder = layouts
                    .last_mut()
                    .ok_or_else(|| decoder.error("repeating without a layout"))?;
                if builder.stage != 3 {
                    return Err(decoder.error("repeating follows the slots, once"));
                }
                builder.layout.repeating = (1..decoder.fields.len())
                    .map(|index| decoder.id(index).map(FrameId))
                    .collect::<Result<_, _>>()?;
                builder.stage = 4;
            }
            "startlayout" => {
                decoder.expect_len(2)?;
                if start_layout.is_some() {
                    return Err(decoder.error("one startlayout record"));
                }
                start_layout = Some(LayoutId(decoder.id(1)?));
            }
            "layoutlegacy" => {
                if start_layout.is_some() {
                    return Err(decoder.error("startlayout follows every layout"));
                }
                let builder = layouts
                    .last_mut()
                    .ok_or_else(|| decoder.error("legacy without a layout"))?;
                if builder.stage != 4 {
                    return Err(decoder.error("layout legacy data comes last"));
                }
                decoder.legacy(&mut builder.layout.legacy)?;
            }
            "frame" => {
                decoder.expect_len(3)?;
                frames.push(FrameBuilder {
                    id: FrameId(decoder.id(1)?),
                    group: decoder.optional_id(2)?.map(GroupId),
                    placement: None,
                    style: FrameStyle::default(),
                    interior: None,
                    content: None,
                    legacy: LegacyData::default(),
                    stage: 0,
                    table_shape: (0, 0),
                    table_laid_out: false,
                    picture_scaled: false,
                });
            }
            _ if section == Section::Frames => {
                let builder = frames
                    .last_mut()
                    .ok_or_else(|| decoder.error("frame record without a frame"))?;
                decode_frame_record(&decoder, record, builder)?;
            }
            "stack" => {
                stack = Some(
                    (1..decoder.fields.len())
                        .map(|index| decoder.id(index).map(FrameId))
                        .collect::<Result<_, _>>()?,
                );
            }
            "flow" => {
                decoder.expect_len(2)?;
                if let Some(done) = paragraph.paragraph.take() {
                    flows
                        .last_mut()
                        .expect("a paragraph belongs to a flow")
                        .1
                        .push(done);
                }
                flows.push((FlowId(decoder.id(1)?), Vec::new()));
            }
            "paragraph" => {
                decoder.expect_len(4)?;
                let (_, paragraphs) = flows
                    .last_mut()
                    .ok_or_else(|| decoder.error("paragraph without a flow"))?;
                if let Some(done) = paragraph.paragraph.take() {
                    paragraphs.push(done);
                }
                paragraph = ParagraphBuilder {
                    paragraph: Some(Paragraph {
                        format: ParagraphFormat::default(),
                        id: ParagraphId(decoder.id(1)?),
                        style: StyleId(decoder.id(2)?),
                        text: decoder.text(3)?,
                        runs: Vec::new(),
                        objects: Vec::new(),
                        marks: Vec::new(),
                        end_format: CharacterFormat::default(),
                        cache: None,
                    }),
                    stage: 0,
                };
            }
            "pformat" | "run" | "endformat" | "rformat" | "inline" | "mark" | "pcache" => {
                let stage = match record {
                    "pformat" => 0,
                    "run" => 1,
                    // Attributes of the most recent run or end format.
                    "rformat" => paragraph.stage.max(1),
                    "endformat" => 2,
                    "inline" => 3,
                    "mark" => 4,
                    _ => 5,
                };
                if record == "pcache" && paragraph.stage == 5 {
                    return Err(decoder.error("a paragraph has one pcache record, last"));
                }
                if stage < paragraph.stage || (record == "endformat" && paragraph.stage == 2) {
                    return Err(decoder.error(
                        "paragraph formats, runs, the end format, objects, and marks come in that order",
                    ));
                }
                if record == "rformat" && stage > 2 {
                    return Err(decoder.error("rformat without a run or end format"));
                }
                paragraph.stage = stage;
                let target = paragraph
                    .paragraph
                    .as_mut()
                    .ok_or_else(|| decoder.error("paragraph record without a paragraph"))?;
                match record {
                    "pformat" => decode_paragraph_format(&decoder, &mut target.format)?,
                    "run" => {
                        decoder.expect_len(9)?;
                        target.runs.push(CharacterRun {
                            start: decoder.number(1)?,
                            end: decoder.number(2)?,
                            format: decode_format_fields(&decoder, 3)?,
                        });
                        attributes_seen = 0;
                    }
                    "endformat" => {
                        decoder.expect_len(7)?;
                        target.end_format = decode_format_fields(&decoder, 1)?;
                        attributes_seen = 0;
                    }
                    "rformat" => {
                        let format = if paragraph.stage == 2 {
                            &mut target.end_format
                        } else {
                            &mut target
                                .runs
                                .last_mut()
                                .ok_or_else(|| decoder.error("rformat without a run"))?
                                .format
                        };
                        match decode_attribute(&decoder, &mut attributes_seen)? {
                            // Underline belongs to the run record.
                            Attribute::Flag(0, _) => {
                                return Err(decoder.error("a run's underline is in its run record"));
                            }
                            Attribute::Flag(index, value) => {
                                *format_flags_mut(format)[index] = Some(value);
                            }
                            Attribute::Caps(caps) => format.caps = Some(caps),
                            Attribute::Colour(colour) => format.colour = Some(colour),
                            Attribute::Overstrike(overstrike) => {
                                format.overstrike = Some(overstrike)
                            }
                        }
                    }
                    "inline" => target.objects.push(decode_inline(&decoder)?),
                    "pcache" => {
                        decoder.expect_len(4)?;
                        target.cache = Some(ParagraphCache {
                            height: decoder.number(1)?,
                            width: decoder.number(2)?,
                            fingerprint: u64::from_str_radix(decoder.fields[3], 16).map_err(
                                |_| decoder.error("a pcache fingerprint is hexadecimal"),
                            )?,
                        });
                    }
                    _ => {
                        decoder.expect_len(3)?;
                        target.marks.push(LegacyMark {
                            byte: decoder.number(1)?,
                            raw: decoder.text(2)?,
                        });
                    }
                }
            }
            _ => return Err(decoder.error("record out of place")),
        }
    }
    if let Some(done) = paragraph.paragraph.take() {
        flows
            .last_mut()
            .expect("a paragraph belongs to a flow")
            .1
            .push(done);
    }

    let end = |message: &str| DocumentError::Parse {
        line: last_line,
        message: message.into(),
    };
    let title = title.ok_or_else(|| end("missing title"))?;
    let (next_id, main_flow) = ids.ok_or_else(|| end("missing ids"))?;
    let frame_stack = stack.ok_or_else(|| end("missing stack"))?;

    let mut decoded_objects = BTreeMap::new();
    for (id, length, data) in objects {
        let bytes = base64_decode(&data).ok_or_else(|| end("malformed base64 payload"))?;
        if bytes.len() != length {
            return Err(end("payload length does not match its object record"));
        }
        if decoded_objects
            .insert(
                id,
                EmbeddedObject {
                    bytes: Arc::from(bytes),
                },
            )
            .is_some()
        {
            return Err(end("duplicate object ID"));
        }
    }

    let mut decoded_layouts = Vec::with_capacity(layouts.len());
    for builder in layouts {
        if builder.stage < 4 {
            return Err(end("a layout needs its pages, slots, and repeating list"));
        }
        let mut layout = builder.layout;
        layout.right = finish_page(builder.right, last_line)?
            .ok_or_else(|| end("a layout needs its right page"))?;
        layout.left = finish_page(builder.left, last_line)?;
        decoded_layouts.push(Arc::new(layout));
    }

    let mut decoded_frames = BTreeMap::new();
    for builder in frames {
        let (Some(placement), Some(mut content)) = (builder.placement, builder.content) else {
            return Err(end("a frame needs its placement and content"));
        };
        if let FrameContent::Picture(picture) = &mut content
            && !builder.picture_scaled
            && let Some((scaling, pan)) = picture.kept_scaling()
        {
            picture.scaling = scaling;
            picture.pan = pan;
        }
        if builder.stage < 5 {
            return Err(end("a frame needs its look, border, shadow, and box"));
        }
        if let FrameContent::Table(table) = &content
            && (builder.table_shape != (table.columns.len(), table.rows.len())
                || table.cells.len() != table.rows.len() * table.columns.len())
        {
            return Err(end("a table needs all its columns, rows, and cells"));
        }
        if let FrameContent::Table(table) = &mut content
            && !builder.table_laid_out
        {
            table.layout = TableLayout::inferred(&table.rows, &table.columns);
        }
        let mut style = builder.style;
        style.interior = finish_box(builder.interior, last_line)?;
        let frame = Frame {
            id: builder.id,
            placement,
            style,
            group: builder.group,
            content,
            legacy: builder.legacy,
        };
        if decoded_frames.insert(frame.id, Arc::new(frame)).is_some() {
            return Err(end("duplicate frame ID"));
        }
    }

    let mut decoded_flows = BTreeMap::new();
    for (id, paragraphs) in flows {
        let flow = Flow {
            paragraphs: paragraphs.into_iter().map(Arc::new).collect(),
        };
        if decoded_flows.insert(id, Arc::new(flow)).is_some() {
            return Err(end("duplicate flow ID"));
        }
    }

    let document = Document {
        title,
        styles: Arc::new(styles),
        flows: Arc::new(decoded_flows),
        main_flow,
        layouts: Arc::new(decoded_layouts),
        frames: Arc::new(decoded_frames),
        frame_stack: Arc::new(frame_stack),
        objects: Arc::new(decoded_objects),
        legacy: Arc::new(legacy),
        line_heights: Arc::new(line_heights),
        face_kinds: Arc::new(face_kinds),
        style_sheet,
        hyphenation_zone: hyphenation_zone.unwrap_or(crate::DEFAULT_HYPHENATION_ZONE),
        start_layout,
        info: Arc::new(info.unwrap_or_default()),
        footnote_options: Arc::new(footnote_options.unwrap_or_default()),
        line_numbering: Arc::new(line_numbering.unwrap_or_default()),
        language: language.unwrap_or_default(),
        grammar_set: grammar_set.unwrap_or_default(),
        contents: Arc::new(contents),
        merge: Arc::new(merge),
        codes: None,
        next_id,
    };
    document.validate()?;
    Ok(document)
}

fn decode_frame_record(
    decoder: &Decoder<'_>,
    record: &str,
    builder: &mut FrameBuilder,
) -> Result<(), DocumentError> {
    // Stages: 1 placement, 2 look, 3 border, 4 shadow, 5 box, 6 tabs,
    // 7 name, 8 macro, 9 content, 10 columns, 11 rows, 12 row options,
    // 13 table layout, 14 cached rows, 15 cells, 16 table legacy, 17 content
    // legacy, 18 frame legacy.
    let stage = match record {
        "placement" => 1,
        "look" => 2,
        "border" => 3,
        "shadow" => 4,
        "box" => 5,
        "edges" | "tabs" => 6,
        "name" => 7,
        "macro" => 8,
        "content" => 9,
        // A picture's records take the places of a table's.
        "column" | "picturescaling" => 10,
        "row" | "picturedescription" => 11,
        "tablerows" => 12,
        "tablelayout" => 13,
        "readrow" => 14,
        "cell" | "cellformat" | "cellformula" | "celllegacy" => 15,
        "tablelegacy" => 16,
        "pictureadjust" | "picturelegacy" | "contentlegacy" => 17,
        _ => 18,
    };
    let repeatable = matches!(
        record,
        "tabs"
            | "column"
            | "row"
            | "readrow"
            | "cell"
            | "cellformat"
            | "cellformula"
            | "celllegacy"
            | "tablelegacy"
            | "pictureadjust"
            | "picturelegacy"
            | "contentlegacy"
            | "framelegacy"
    );
    if stage < builder.stage || (stage == builder.stage && !repeatable) {
        return Err(decoder.error("frame record out of order"));
    }
    if stage > 5 && builder.stage < 5 && record != "tabs" {
        return Err(decoder.error("a frame needs its look, border, shadow, and box first"));
    }
    if (2..=5).contains(&stage) && builder.stage != stage - 1 {
        return Err(decoder.error("frame records out of order"));
    }
    builder.stage = stage;
    let (declared_columns, declared_rows) = builder.table_shape;
    match record {
        "placement" => {
            let kind = decoder.fields.get(1).copied().unwrap_or("");
            builder.placement = Some(match kind {
                "fixed" => {
                    decoder.expect_len(7)?;
                    Placement::Fixed {
                        page: decoder.number(2)?,
                        rect: decoder.rect(3)?,
                    }
                }
                "below" | "inline" => {
                    decoder.expect_len(5)?;
                    let offset = decoder.number(2)?;
                    let size = Size {
                        width: decoder.number(3)?,
                        height: decoder.number(4)?,
                    };
                    if kind == "below" {
                        Placement::BelowParagraph {
                            x_offset: offset,
                            size,
                        }
                    } else {
                        Placement::Inline {
                            y_offset: offset,
                            size,
                        }
                    }
                }
                "repeating" => {
                    decoder.expect_len(7)?;
                    let pages = match decoder.fields[2] {
                        "all" => RepeatPages::All,
                        "rightleft" => RepeatPages::RightLeft,
                        _ => return Err(decoder.error("malformed repeat pages")),
                    };
                    Placement::Repeating {
                        pages,
                        rect: decoder.rect(3)?,
                    }
                }
                "header" | "footer" => {
                    decoder.expect_len(3)?;
                    let side = decoder.side(2)?;
                    if kind == "header" {
                        Placement::Header { side }
                    } else {
                        Placement::Footer { side }
                    }
                }
                "incell" => {
                    decoder.expect_len(2)?;
                    Placement::InCell
                }
                _ => return Err(decoder.error("malformed placement")),
            });
        }
        "look" => {
            decoder.expect_len(5)?;
            builder.style.wrap = match decoder.fields[1] {
                "around" => Wrap::Around,
                "none" => Wrap::NoWrapAround,
                "beside" => Wrap::NoWrapBeside,
                _ => return Err(decoder.error("malformed wrap")),
            };
            builder.style.opaque = decoder.flag(2)?;
            builder.style.corners = decoder.number(3)?;
            builder.style.background = decoder.number(4)?;
        }
        "border" => {
            decoder.expect_len(5)?;
            builder.style.border = decoder.border(1)?;
        }
        "shadow" => {
            decoder.expect_len(6)?;
            builder.style.shadow = Shadow {
                left: decoder.number(1)?,
                top: decoder.number(2)?,
                right: decoder.number(3)?,
                bottom: decoder.number(4)?,
                colour: decoder.number(5)?,
            };
        }
        "box" => builder.interior = Some(decoder.layout_box()?),
        "edges" => decoder.edges(
            builder
                .interior
                .as_mut()
                .ok_or_else(|| decoder.error("edges without a box"))?,
        )?,
        "tabs" => decoder.tabs(
            builder
                .interior
                .as_mut()
                .ok_or_else(|| decoder.error("tabs without a box"))?,
        )?,
        "name" => {
            decoder.expect_len(2)?;
            builder.style.name = Some(decoder.text(1)?);
        }
        "macro" => {
            decoder.expect_len(2)?;
            builder.style.macro_name = Some(decoder.text(1)?);
        }
        "content" => {
            let kind = decoder.fields.get(1).copied().unwrap_or("");
            builder.content = Some(match kind {
                "text" => {
                    decoder.expect_len(3)?;
                    FrameContent::Text(FlowId(decoder.id(2)?))
                }
                "table" => {
                    // Files written before cell margins (2026-10-12) omit
                    // them and get the defaults; the insets follow them
                    // when they are not Ami Pro's 43 (2026-10-18).
                    let cell_margins = match decoder.fields.len() {
                        4 => CellMargins::default(),
                        6 | 8 => CellMargins {
                            horizontal: decoder.number(4)?,
                            vertical: decoder.number(5)?,
                        },
                        _ => return Err(decoder.error("a table has 2, 4, or 6 fields")),
                    };
                    let mut spacing = TableSpacing::default();
                    if decoder.fields.len() == 8 {
                        spacing.top = decoder.number(6)?;
                        spacing.left = decoder.number(7)?;
                    }
                    let columns: usize = decoder.number(2)?;
                    let rows: usize = decoder.number(3)?;
                    if columns == 0 || rows == 0 {
                        return Err(decoder.error("a table needs at least one column and row"));
                    }
                    builder.table_shape = (columns, rows);
                    FrameContent::Table(Table {
                        columns: Vec::new(),
                        rows: Vec::new(),
                        cell_margins,
                        spacing,
                        row_options: TableRows::default(),
                        layout: TableLayout::default(),
                        read_rows: Default::default(),
                        cells: Vec::new(),
                        legacy: LegacyData::default(),
                    })
                }
                "picture" => {
                    decoder.expect_len(6)?;
                    let source = match decoder.fields[2] {
                        "embedded" => PictureSource::Embedded(ObjectId(decoder.id(3)?)),
                        "linked" => PictureSource::Linked(decoder.text(3)?),
                        _ => return Err(decoder.error("malformed picture source")),
                    };
                    let mut picture = Picture::new(source, decoder.text(4)?, UNSCALED);
                    picture.rendering = decoder.optional_id(5)?.map(ObjectId);
                    FrameContent::Picture(picture)
                }
                "legacy" => {
                    decoder.expect_len(2)?;
                    FrameContent::Legacy(LegacyData::default())
                }
                _ => return Err(decoder.error("malformed content")),
            });
        }
        "column" => {
            let gutter = own_gutter(decoder)?;
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("column without a table"));
            };
            if table.columns.len() >= declared_columns {
                return Err(decoder.error("more columns than the table declares"));
            }
            if let Some(gutter) = gutter {
                table
                    .spacing
                    .column_gutters
                    .insert(table.columns.len(), gutter);
            }
            table.columns.push(decoder.number(1)?);
        }
        "row" => {
            let gutter = own_gutter(decoder)?;
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("row without a table"));
            };
            if table.columns.len() != declared_columns || table.rows.len() >= declared_rows {
                return Err(decoder.error("rows follow all columns, as many as declared"));
            }
            if let Some(gutter) = gutter {
                table.spacing.row_gutters.insert(table.rows.len(), gutter);
            }
            table.rows.push(decoder.number(1)?);
        }
        "tablerows" => {
            decoder.expect_len(5)?;
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("row options without a table"));
            };
            if table.rows.len() != declared_rows {
                return Err(decoder.error("row options follow all rows"));
            }
            let flag = |index: usize| match decoder.fields[index] {
                "0" => Ok(false),
                "1" => Ok(true),
                _ => Err(decoder.error("a row option is 0 or 1")),
            };
            let rows = |index: usize| -> Result<std::collections::BTreeSet<usize>, DocumentError> {
                let field = decoder.fields[index];
                if field == "-" {
                    return Ok(std::collections::BTreeSet::new());
                }
                let mut rows = std::collections::BTreeSet::new();
                for value in field.split(',') {
                    let row: usize = value
                        .parse()
                        .map_err(|_| decoder.error("malformed row list"))?;
                    if row >= declared_rows || !rows.insert(row) {
                        return Err(decoder.error("a row list names each of the table's rows once"));
                    }
                }
                Ok(rows)
            };
            table.row_options = TableRows {
                automatic: flag(1)?,
                span_pages: flag(2)?,
                headings: rows(3)?,
                page_breaks: rows(4)?,
            };
        }
        "tablelayout" => {
            decoder.expect_len(8)?;
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("a table layout without a table"));
            };
            if table.rows.len() != declared_rows {
                return Err(decoder.error("the table layout follows all rows"));
            }
            let flag = |index: usize| match decoder.fields[index] {
                "0" => Ok(false),
                "1" => Ok(true),
                _ => Err(decoder.error("a table option is 0 or 1")),
            };
            let list = |index: usize, count: usize| -> Result<BTreeSet<usize>, DocumentError> {
                let field = decoder.fields[index];
                if field == "-" {
                    return Ok(BTreeSet::new());
                }
                let mut items = BTreeSet::new();
                for value in field.split(',') {
                    let item: usize = value
                        .parse()
                        .map_err(|_| decoder.error("malformed row or column list"))?;
                    if item >= count || !items.insert(item) {
                        return Err(decoder.error("a list names each row or column once"));
                    }
                }
                Ok(items)
            };
            table.layout = TableLayout {
                row_height: decoder.number(1)?,
                column_width: decoder.number(2)?,
                center: flag(3)?,
                honor_protection: flag(4)?,
                fixed_sizes: flag(5)?,
                own_heights: list(6, declared_rows)?,
                own_widths: list(7, declared_columns)?,
            };
            builder.table_laid_out = true;
        }
        "cellformat" => {
            decoder.expect_len(6)?;
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("a cell format without a table"));
            };
            let colour = |index: usize| {
                Colour::from_str_radix(decoder.fields[index], 16)
                    .ok()
                    .filter(|colour| *colour <= WHITE)
                    .ok_or_else(|| decoder.error("a colour is six hexadecimal digits"))
            };
            let leader = match decoder.fields[4] {
                "0" => CellLeader::None,
                "1" => CellLeader::Dots,
                "2" => CellLeader::Dashes,
                "3" => CellLeader::Line,
                _ => return Err(decoder.error("a leader is 0 to 3")),
            };
            let format = CellFormat {
                lines: CellLines::from_word(decoder.number(1)?),
                line_colour: colour(2)?,
                fill: if decoder.fields[3] == "-" {
                    None
                } else {
                    Some(colour(3)?)
                },
                leader,
                protected: match decoder.fields[5] {
                    "0" => false,
                    "1" => true,
                    _ => return Err(decoder.error("protection is 0 or 1")),
                },
            };
            let cell = table
                .cells
                .last_mut()
                .ok_or_else(|| decoder.error("a cell format without a cell"))?;
            cell.format = format;
        }
        "readrow" => {
            decoder.expect_len(3)?;
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("a cached row height without a table"));
            };
            if table.rows.len() != declared_rows {
                return Err(decoder.error("cached row heights follow all rows"));
            }
            let row: usize = decoder.number(1)?;
            if row >= declared_rows
                || table
                    .read_rows
                    .last_key_value()
                    .is_some_and(|(last, _)| *last >= row)
            {
                return Err(decoder.error("cached rows name the table's rows once, in order"));
            }
            let fingerprint = u64::from_str_radix(decoder.fields[2], 16)
                .map_err(|_| decoder.error("a row fingerprint is hexadecimal"))?;
            table.read_rows.insert(row, fingerprint);
        }
        "cell" => {
            // Files written before joined cells (2026-10-12) omit the span;
            // a cell holding a frame (2026-10-18) names it after the span.
            let span = match decoder.fields.len() {
                2 => CellSpan::SINGLE,
                4 | 5 => CellSpan {
                    rows: decoder.number(2)?,
                    columns: decoder.number(3)?,
                },
                _ => return Err(decoder.error("a cell has 1, 3, or 4 fields")),
            };
            let frame = if decoder.fields.len() == 5 {
                Some(FrameId(decoder.id(4)?))
            } else {
                None
            };
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("cell without a table"));
            };
            if table.rows.len() != declared_rows
                || table.cells.len() >= declared_columns * declared_rows
            {
                return Err(decoder.error("cells follow all rows, as many as declared"));
            }
            table.cells.push(TableCell {
                content: CellContent::Text(FlowId(decoder.id(1)?)),
                span,
                frame,
                format: CellFormat::default(),
                formula: None,
                legacy: LegacyData::default(),
            });
        }
        "cellformula" => {
            decoder.expect_len(2)?;
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("a cell formula without a table"));
            };
            let cell = table
                .cells
                .last_mut()
                .ok_or_else(|| decoder.error("a cell formula without a cell"))?;
            if cell.formula.is_some() {
                return Err(decoder.error("a cell has one formula"));
            }
            let formula = decoder.text(1)?;
            if !crate::formula_text_is_valid(&formula) {
                return Err(decoder.error("a formula is one line of text"));
            }
            cell.formula = Some(formula);
        }
        "celllegacy" => {
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("cell legacy data without a table"));
            };
            let cell = table
                .cells
                .last_mut()
                .ok_or_else(|| decoder.error("cell legacy data without a cell"))?;
            decoder.legacy(&mut cell.legacy)?;
        }
        "tablelegacy" => {
            let Some(FrameContent::Table(table)) = &mut builder.content else {
                return Err(decoder.error("table legacy data without a table"));
            };
            decoder.legacy(&mut table.legacy)?;
        }
        "picturescaling" => {
            decoder.expect_len(9)?;
            let Some(FrameContent::Picture(picture)) = &mut builder.content else {
                return Err(decoder.error("picture scaling without a picture"));
            };
            picture.scaling = PictureScaling {
                sizing: match decoder.fields[1] {
                    "original" => PictureSizing::Original,
                    "fit" => PictureSizing::FitInFrame,
                    "percentage" => PictureSizing::Percentage,
                    "custom" => PictureSizing::Custom,
                    _ => return Err(decoder.error("malformed picture sizing")),
                },
                keep_aspect: decoder.flag(2)?,
                percent: decoder.number(3)?,
                size: Size {
                    width: decoder.number(4)?,
                    height: decoder.number(5)?,
                },
                rotation: decoder.number(6)?,
            };
            picture.pan = Offset {
                x: decoder.number(7)?,
                y: decoder.number(8)?,
            };
            builder.picture_scaled = true;
        }
        "picturedescription" => {
            decoder.expect_len(2)?;
            let Some(FrameContent::Picture(picture)) = &mut builder.content else {
                return Err(decoder.error("picture description without a picture"));
            };
            picture.description = decoder.text(1)?;
        }
        "pictureadjust" => {
            decoder.expect_len(7)?;
            let Some(FrameContent::Picture(picture)) = &mut builder.content else {
                return Err(decoder.error("picture adjustments without a picture"));
            };
            let code: u8 = decoder.number(6)?;
            let adjust = crate::ImageAdjust {
                brightness: decoder.number(1)?,
                contrast: decoder.number(2)?,
                edge: decoder.number(3)?,
                smoothing: decoder.number(4)?,
                invert: decoder.flag(5)?,
                halftone: crate::Halftone::from_code(code)
                    .ok_or_else(|| decoder.error("a halftone mode from 1 to 5"))?,
            };
            if adjust != adjust.clamped() {
                return Err(decoder.error("adjustments from 0 to 100"));
            }
            picture.adjust = adjust;
        }
        "picturelegacy" => {
            let Some(FrameContent::Picture(picture)) = &mut builder.content else {
                return Err(decoder.error("picture legacy data without a picture"));
            };
            decoder.legacy(&mut picture.legacy)?;
        }
        "contentlegacy" => {
            let Some(FrameContent::Legacy(legacy)) = &mut builder.content else {
                return Err(decoder.error("content legacy data without legacy content"));
            };
            decoder.legacy(legacy)?;
        }
        "framelegacy" => decoder.legacy(&mut builder.legacy)?,
        _ => return Err(decoder.error("unknown frame record")),
    }
    Ok(())
}

/// A `column` or `row` record's own gutter, after its size (2026-10-18).
fn own_gutter(decoder: &Decoder<'_>) -> Result<Option<Twips>, DocumentError> {
    match decoder.fields.len() {
        2 => Ok(None),
        3 => Ok(Some(decoder.number(2)?)),
        _ => Err(decoder.error("a column or row has a size and an optional gutter")),
    }
}

fn decode_line_spacing(
    decoder: &Decoder<'_>,
    index: usize,
    height: Twips,
) -> Result<LineSpacing, DocumentError> {
    Ok(match decoder.fields[index] {
        "single" => LineSpacing::Single,
        "half" => LineSpacing::OneAndAHalf(height),
        "double" => LineSpacing::Double(height),
        "custom" => LineSpacing::Custom(height),
        _ => return Err(decoder.error("unknown line spacing")),
    })
}

/// One `pformat` record: `indents ALL FIRST REST RIGHT` or `line KIND HEIGHT`.
fn decode_paragraph_format(
    decoder: &Decoder<'_>,
    format: &mut ParagraphFormat,
) -> Result<(), DocumentError> {
    match decoder.fields.get(1).copied() {
        Some("indents") => {
            decoder.expect_len(6)?;
            format.indents = Some(Indents {
                all: decoder.number(2)?,
                first: decoder.number(3)?,
                rest: decoder.number(4)?,
                right: decoder.number(5)?,
            });
        }
        Some("line") => {
            decoder.expect_len(4)?;
            format.line_spacing = Some(decode_line_spacing(decoder, 2, decoder.number(3)?)?);
        }
        Some("align") => {
            decoder.expect_len(3)?;
            format.alignment = Some(decode_alignment(decoder, 2)?);
        }
        // The paragraph's tab ruler, possibly without stops (2026-10-12).
        Some("tabs") => {
            format.tabs = Some(
                decoder.fields[2..]
                    .iter()
                    .map(|stop| decoder.tab_stop(stop))
                    .collect::<Result<_, _>>()?,
            );
        }
        _ => return Err(decoder.error("unknown paragraph format")),
    }
    Ok(())
}

/// A character attribute from an `sformat` or `rformat` record.
enum Attribute {
    /// An index into `ATTRIBUTE_FLAGS`, and the value.
    Flag(usize, bool),
    Caps(Caps),
    Colour(Colour),
    Overstrike(char),
}

/// Reads one `sformat` or `rformat` record: an attribute name and its
/// value. `seen` holds the attributes already set on the same style or run;
/// each may be set once.
fn decode_attribute(decoder: &Decoder<'_>, seen: &mut u16) -> Result<Attribute, DocumentError> {
    decoder.expect_len(3)?;
    let name = decoder.fields[1];
    let (bit, attribute) =
        if let Some(index) = ATTRIBUTE_FLAGS.iter().position(|flag| *flag == name) {
            (index, Attribute::Flag(index, decoder.flag(2)?))
        } else if name == "caps" {
            let caps = match decoder.fields[2] {
                "upper" => Caps::Upper,
                "lower" => Caps::Lower,
                "initial" => Caps::Initial,
                "small" => Caps::Small,
                "normal" => Caps::Normal,
                _ => return Err(decoder.error("unknown caps")),
            };
            (8, Attribute::Caps(caps))
        } else if name == "colour" {
            let colour: Colour = decoder.number(2)?;
            if colour > WHITE {
                return Err(decoder.error("a colour is 0x00BBGGRR"));
            }
            (9, Attribute::Colour(colour))
        } else if name == "overstrike" {
            let text = decoder.text(2)?;
            let mut characters = text.chars();
            let (Some(overstrike), None) = (characters.next(), characters.next()) else {
                return Err(decoder.error("an overstrike is one character"));
            };
            (10, Attribute::Overstrike(overstrike))
        } else {
            return Err(decoder.error("unknown character attribute"));
        };
    if *seen & (1 << bit) != 0 {
        return Err(decoder.error("a character attribute is set twice"));
    }
    *seen |= 1 << bit;
    Ok(attribute)
}

fn decode_inline(decoder: &Decoder<'_>) -> Result<InlineObject, DocumentError> {
    let kind = decoder.fields.get(1).copied().unwrap_or("");
    Ok(match kind {
        "break" => {
            if !(5..=6).contains(&decoder.fields.len())
                || decoder
                    .fields
                    .get(5)
                    .is_some_and(|field| *field != "restart")
            {
                return Err(decoder.error("malformed break"));
            }
            InlineObject::Break {
                kind: match decoder.fields[2] {
                    "page" => BreakKind::Page,
                    "column" => BreakKind::Column,
                    _ => return Err(decoder.error("malformed break kind")),
                },
                centre: decoder.flag(3)?,
                layout: decoder.optional_id(4)?.map(LayoutId),
                restart: decoder.fields.len() == 6,
            }
        }
        "floating" => {
            decoder.expect_len(5)?;
            InlineObject::FloatingHeaderFooter {
                kind: match decoder.fields[2] {
                    "header" => HeaderFooterKind::Header,
                    "footer" => HeaderFooterKind::Footer,
                    _ => return Err(decoder.error("malformed floating kind")),
                },
                pages: match decoder.fields[3] {
                    "all" => PageSides::All,
                    "right" => PageSides::Right,
                    "left" => PageSides::Left,
                    _ => return Err(decoder.error("malformed floating pages")),
                },
                flow: FlowId(decoder.id(4)?),
            }
        }
        "anchor" => {
            decoder.expect_len(3)?;
            InlineObject::FrameAnchor(FrameId(decoder.id(2)?))
        }
        "pagenumber" if decoder.fields.len() == 5 => {
            // Before 2026-10-13 the style character and the start page were
            // one number (`<:P10,…>` as style 10), and the start number was
            // the second (`PAGE_LAYOUT_NOTES.md`, "Page numbering").
            let joined: u64 = decoder.number(2)?;
            let digits = joined.to_string();
            let (style, page) = digits.split_at(1);
            InlineObject::PageNumber(PageNumberField {
                style: style.chars().next().unwrap_or('1'),
                start_page: if page.is_empty() {
                    0
                } else {
                    page.parse()
                        .map_err(|_| decoder.error("malformed page number"))?
                },
                start_number: decoder.number(3)?,
                leading_text: decoder.text(4)?,
            })
        }
        "pagenumber" => {
            decoder.expect_len(6)?;
            let style = decoder.text(2)?;
            let mut chars = style.chars();
            let (Some(style), None) = (chars.next(), chars.next()) else {
                return Err(decoder.error("a page-number style is one character"));
            };
            InlineObject::PageNumber(PageNumberField {
                style,
                start_page: decoder.number(3)?,
                start_number: decoder.number(4)?,
                leading_text: decoder.text(5)?,
            })
        }
        "bullet" => {
            decoder.expect_len(3)?;
            let code: u32 = decoder.number(2)?;
            match char::from_u32(code).filter(|code| crate::is_bullet_code(*code)) {
                Some(code) => InlineObject::Bullet(code),
                None => return Err(decoder.error("not one of Ami Pro's bullets")),
            }
        }
        "date" => {
            decoder.expect_len(4)?;
            InlineObject::Date(DateField {
                source: match decoder.fields[2] {
                    "today" => DateSource::Today,
                    "saved" => DateSource::LastSaved,
                    "created" => DateSource::Created,
                    "time" => DateSource::Time,
                    _ => return Err(decoder.error("malformed date source")),
                },
                format: decoder.number(3)?,
            })
        }
        "seq" => {
            decoder.expect_len(4)?;
            InlineObject::Seq(crate::SeqField {
                value: decoder.number(2)?,
                code: decoder.text(3)?,
            })
        }
        "note" => {
            decoder.expect_len(7)?;
            let optional = |index: usize| -> Result<Option<u32>, DocumentError> {
                if decoder.fields[index].is_empty() {
                    Ok(None)
                } else {
                    decoder.number(index).map(Some)
                }
            };
            let initials = match decoder.fields[4] {
                "" => None,
                _ => Some(
                    decoder
                        .text(4)?
                        .strip_prefix('=')
                        .ok_or_else(|| decoder.error("malformed note initials"))?
                        .to_owned(),
                ),
            };
            InlineObject::Note {
                flow: FlowId(decoder.id(2)?),
                note: crate::NoteInfo {
                    time: decoder.number(3)?,
                    initials,
                    colour: optional(5)?,
                    number: optional(6)?
                        .map(|number| {
                            u16::try_from(number).map_err(|_| decoder.error("a note number"))
                        })
                        .transpose()?,
                },
            }
        }
        "footnote" => {
            decoder.expect_len(3)?;
            InlineObject::Footnote {
                flow: FlowId(decoder.id(2)?),
            }
        }
        "tablefootnote" => {
            decoder.expect_len(4)?;
            InlineObject::TableFootnote {
                number: decoder.number(2)?,
                row: decoder.number(3)?,
            }
        }
        "docinfo" => {
            decoder.expect_len(4)?;
            InlineObject::DocInfo(DocInfoField {
                code: decoder.number(2)?,
                format: decoder.number(3)?,
            })
        }
        _ => return Err(decoder.error("malformed inline object")),
    })
}

// ------------------------------------------------------------ text helpers

pub(crate) fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(character),
        }
    }
    escaped
}

pub(crate) fn unescape(value: &str) -> Result<String, String> {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('r') => result.push('\r'),
            Some('\\') => result.push('\\'),
            _ => return Err("invalid escape sequence".into()),
        }
    }
    Ok(result)
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(crate) fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(char::from(
                    BASE64[(value >> (18 - 6 * index)) as usize & 63],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub(crate) fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if bytes.len() % 4 != 0 {
        return None;
    }
    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 || bytes[..bytes.len() - padding].contains(&b'=') {
        return None;
    }
    let value_of = |byte: u8| BASE64.iter().position(|candidate| *candidate == byte);
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let last = index + 1 == bytes.len() / 4;
        let mut value = 0u32;
        let mut digits = 0;
        for byte in chunk {
            if *byte == b'=' {
                if !last {
                    return None;
                }
                value <<= 6;
            } else {
                value = (value << 6) | value_of(*byte)? as u32;
                digits += 1;
            }
        }
        let produced = match digits {
            4 => 3,
            3 => 2,
            2 => 1,
            _ => return None,
        };
        let decoded = [(value >> 16) as u8, (value >> 8) as u8, value as u8];
        // Unused low bits of a padded group must be zero (canonical form).
        if decoded[produced..].iter().any(|byte| *byte != 0) {
            return None;
        }
        out.extend_from_slice(&decoded[..produced]);
    }
    Some(out)
}
