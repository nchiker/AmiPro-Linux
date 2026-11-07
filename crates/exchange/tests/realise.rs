//! `realise` as the export's builder run backwards: a document built into
//! an exchange document, realised, and built again gives the same exchange
//! document (IM.3's gate).

use std::path::{Path, PathBuf};

use amipro_document::{Document, PictureSource, StyleId};
use amipro_exchange::build::{BuildOptions, PictureSink, SunkPicture, build};
use amipro_exchange::import::{Findings, StyleOption};
use amipro_exchange::realise::{RealiseOptions, realise};
use amipro_exchange::{Block, ExchangeDocument, Image, ImageKind, Inline, Section, ShapeContent};
use amipro_layout::{FixedShaper, ShapeCache, layout_document};

/// Pictures passed through as their bytes, each covering its frame.
struct Bytes;

impl PictureSink for Bytes {
    fn picture(
        &mut self,
        document: &Document,
        picture: &amipro_document::Picture,
    ) -> Option<SunkPicture> {
        let PictureSource::Embedded(object) = picture.source else {
            return None;
        };
        let bytes = document.objects().get(&object)?.bytes.clone();
        let kind = match picture
            .extension
            .trim_start_matches('.')
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => ImageKind::Png,
            "jpg" | "jpeg" => ImageKind::Jpeg,
            "gif" => ImageKind::Gif,
            "wmf" => ImageKind::Wmf,
            "svg" => ImageKind::Svg,
            _ => ImageKind::Bmp,
        };
        Some(SunkPicture {
            image: Image {
                bytes,
                kind,
                fallback: None,
                pixels: None,
            },
            natural: None,
        })
    }
}

fn built(document: &Document) -> ExchangeDocument {
    let mut cache = ShapeCache::default();
    let layout = layout_document(document, &mut FixedShaper, &mut cache);
    let options = BuildOptions {
        layout: Some(&layout),
        title: "Fixture".into(),
        ..BuildOptions::default()
    };
    build(document, &options, &mut Bytes).document
}

fn realised(exchange: &ExchangeDocument) -> (Document, Findings) {
    let base = Document::default();
    let options = RealiseOptions {
        styles: StyleOption::Convert,
        base: &base,
        inserting: false,
        into_text: false,
        title: "Fixture".into(),
        now: 1_792_635_771,
    };
    let mut findings = Findings::default();
    let realised = realise(exchange, &options, &mut FixedShaper, &mut findings);
    (realised.document, findings)
}

/// Shape ids renumbered in order, so that two builds compare.
fn normalise(mut exchange: ExchangeDocument) -> ExchangeDocument {
    fn renumber(blocks: &mut [Block], next: &mut u32) {
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => {
                    // Text split by something that shows nothing, joined.
                    let mut joined: Vec<Inline> = Vec::new();
                    for inline in std::mem::take(&mut paragraph.inlines) {
                        if let (
                            Some(Inline::Text { text, props }),
                            Inline::Text {
                                text: more,
                                props: same,
                            },
                        ) = (joined.last_mut(), &inline)
                            && props == same
                        {
                            text.push_str(more);
                            continue;
                        }
                        joined.push(inline);
                    }
                    paragraph.inlines = joined;
                    for inline in &mut paragraph.inlines {
                        if let Inline::Shape(shape) = inline {
                            if shape.name.starts_with("Frame ") {
                                shape.name = "Frame".into();
                            }
                            shape.id = *next;
                            *next += 1;
                            shape.z = 0;
                            if let ShapeContent::Text { blocks: inner, .. } = &mut shape.content {
                                renumber(inner, next);
                            }
                        }
                    }
                }
                Block::Table(table) => {
                    for row in &mut table.rows {
                        for cell in &mut row.cells {
                            renumber(&mut cell.blocks, next);
                        }
                    }
                }
            }
        }
    }
    // The style sheet a document named comes back only with the sheet.
    exchange.properties.template.clear();
    // A run's font named as its style's after the export renamed both
    // (EC4: Tms Rmn and TimesNewRomanPS are both Times New Roman).
    let fonts: Vec<Option<String>> = exchange
        .styles
        .iter()
        .map(|style| style.character.font.clone())
        .collect();
    fn same_fonts(blocks: &mut [Block], fonts: &[Option<String>]) {
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => {
                    let font = fonts.get(paragraph.style).cloned().flatten();
                    for inline in &mut paragraph.inlines {
                        if let Inline::Text { props, .. } | Inline::Tab { props } = inline
                            && props.font.is_some()
                            && props.font == font
                        {
                            props.font = None;
                        }
                        if let Inline::Shape(shape) = inline
                            && let ShapeContent::Text { blocks, .. } = &mut shape.content
                        {
                            same_fonts(blocks, fonts);
                        }
                    }
                }
                Block::Table(table) => {
                    for row in &mut table.rows {
                        for cell in &mut row.cells {
                            same_fonts(&mut cell.blocks, fonts);
                        }
                    }
                }
            }
        }
    }
    for section in &mut exchange.sections {
        same_fonts(&mut section.blocks, &fonts);
    }
    let mut next = 1;
    for section in &mut exchange.sections {
        renumber(&mut section.blocks, &mut next);
        for set in [&mut section.headers, &mut section.footers] {
            for part in [&mut set.default, &mut set.even, &mut set.first]
                .into_iter()
                .flatten()
            {
                renumber(part, &mut next);
            }
        }
    }
    exchange
}

/// Where two exchange documents first differ, as short descriptions.
fn differences(a: &ExchangeDocument, b: &ExchangeDocument) -> Vec<String> {
    let mut out = Vec::new();
    if a.styles.len() != b.styles.len() {
        let names = |styles: &[amipro_exchange::Style]| {
            styles
                .iter()
                .map(|style| style.name.clone())
                .collect::<Vec<_>>()
        };
        out.push(format!(
            "styles {} against {}: {:?}\n    against {:?}",
            a.styles.len(),
            b.styles.len(),
            names(&a.styles),
            names(&b.styles)
        ));
    }
    for (index, (x, y)) in a.styles.iter().zip(&b.styles).enumerate() {
        if x != y {
            out.push(format!(
                "style {index} {:?}: {}",
                x.name,
                fields_differing(&format!("{x:?}"), &format!("{y:?}"))
            ));
        }
    }
    if a.sections.len() != b.sections.len() {
        out.push(format!(
            "sections {} against {}",
            a.sections.len(),
            b.sections.len()
        ));
    }
    for (index, (x, y)) in a.sections.iter().zip(&b.sections).enumerate() {
        section_differences(index, x, y, &mut out);
    }
    if a.lists != b.lists {
        out.push(format!(
            "lists: {}",
            fields_differing(&format!("{:?}", a.lists), &format!("{:?}", b.lists))
        ));
    }
    if a.properties != b.properties {
        out.push(format!(
            "properties {:?}\n  against {:?}",
            a.properties, b.properties
        ));
    }
    if std::env::var_os("AMIPRO_REALISE_FONTS").is_some() && a.fonts != b.fonts {
        out.push(format!("fonts {:?}\n  against {:?}", a.fonts, b.fonts));
    }
    if a.settings != b.settings {
        out.push(format!(
            "settings {:?}\n  against {:?}",
            a.settings, b.settings
        ));
    }
    out
}

fn section_differences(index: usize, x: &Section, y: &Section, out: &mut Vec<String>) {
    if x.page != y.page {
        out.push(format!(
            "section {index} page: {}",
            fields_differing(&format!("{:?}", x.page), &format!("{:?}", y.page))
        ));
    }
    if x.headers != y.headers {
        out.push(format!(
            "section {index} headers: {}",
            fields_differing(&format!("{:?}", x.headers), &format!("{:?}", y.headers))
        ));
    }
    if x.footers != y.footers {
        out.push(format!(
            "section {index} footers: {}",
            fields_differing(&format!("{:?}", x.footers), &format!("{:?}", y.footers))
        ));
    }
    if x.blocks.len() != y.blocks.len() {
        out.push(format!(
            "section {index} blocks {} against {}",
            x.blocks.len(),
            y.blocks.len()
        ));
    }
    for (number, (a, b)) in x.blocks.iter().zip(&y.blocks).enumerate() {
        if a != b {
            out.push(format!(
                "section {index} block {number}: {}",
                fields_differing(&format!("{a:?}"), &format!("{b:?}"))
            ));
            break;
        }
    }
}

/// The first place two debug texts part, with a little of each.
fn fields_differing(a: &str, b: &str) -> String {
    let at = a
        .char_indices()
        .zip(b.chars())
        .find(|((_, x), y)| x != y)
        .map_or(a.len().min(b.len()), |((index, _), _)| index);
    // The field the texts part in: the last `name:` before the place.
    let field = a[..at]
        .rmatch_indices(": ")
        .find_map(|(colon, _)| {
            let name: String = a[..colon]
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            (!name.is_empty() && name.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
                .then_some(name)
        })
        .unwrap_or_default();
    let start = a[..at].rfind(", ").map_or(0, |comma| comma + 2);
    let start = a[..start].rfind(", ").map_or(start, |comma| comma + 2);
    let take = |text: &str| {
        text.get(start..)
            .unwrap_or("")
            .chars()
            .take(160)
            .collect::<String>()
    };
    format!("[{field}] {}\n    against {}", take(a), take(b))
}

/// What a format cannot hold, taken out of both sides of a comparison
/// (the export's loss list and its notes, `EXPORT_NOTES.md`).
fn within_format(exchange: ExchangeDocument, format: &str) -> ExchangeDocument {
    if format == "odt" {
        return within_odt(exchange);
    }
    within_word(exchange, format)
}

/// What OpenDocument Text cannot hold, as the reader gives it back.
fn within_odt(mut exchange: ExchangeDocument) -> ExchangeDocument {
    fn inlines(inlines: &mut Vec<Inline>) {
        // Inserted bullets are characters in their font.
        let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
        for inline in inlines.drain(..) {
            let inline = match inline {
                Inline::Symbol {
                    character,
                    font,
                    mut props,
                } => {
                    if font.is_some() {
                        props.font = font;
                    }
                    Inline::Text {
                        text: character.to_string(),
                        props,
                    }
                }
                other => other,
            };
            if let (
                Some(Inline::Text {
                    text: last,
                    props: same,
                }),
                Inline::Text { text, props },
            ) = (out.last_mut(), &inline)
                && same == props
            {
                last.push_str(text);
                continue;
            }
            out.push(inline);
        }
        *inlines = out;
        for inline in inlines.iter_mut() {
            if let Inline::Shape(shape) = inline {
                if let Some(line) = &mut shape.line {
                    line.kind = double(line.kind, &mut line.width);
                }
                // No format lowers a frame below the baseline.
                if let amipro_exchange::Anchor::Inline { drop } = &mut shape.anchor {
                    *drop = 0;
                }
                match &mut shape.content {
                    ShapeContent::Text { blocks: inner, .. } => limit(inner),
                    // A picture's frame has square corners.
                    ShapeContent::Picture(_) => shape.corner_radius = 0,
                    ShapeContent::Empty => {}
                }
            }
        }
    }
    fn line(line: &mut amipro_exchange::RuleLine) {
        line.kind = double(line.kind, &mut line.width);
    }
    /// One line or two: Ami Pro's double lines, thin or thick.
    fn double(kind: amipro_exchange::LineKind, width: &mut i32) -> amipro_exchange::LineKind {
        if kind == amipro_exchange::LineKind::Single {
            return kind;
        }
        *width = if *width <= 80 { 60 } else { 100 };
        amipro_exchange::LineKind::Double
    }
    /// What XML holds: no control characters.
    fn xml_text(text: &mut String) {
        text.retain(|character| matches!(character, '\t' | '\n') || !character.is_control());
    }
    fn limit(blocks: &mut [Block]) {
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => {
                    for side in [
                        &mut paragraph.props.border_top,
                        &mut paragraph.props.border_bottom,
                    ] {
                        if let Some(Some(rule)) = side {
                            line(rule);
                        }
                    }
                    inlines(&mut paragraph.inlines);
                    // A paragraph with text has no look of its mark's own.
                    if paragraph.inlines.iter().any(|inline| {
                        matches!(
                            inline,
                            Inline::Text { .. } | Inline::Tab { .. } | Inline::Field(_)
                        )
                    }) {
                        paragraph.mark = Default::default();
                    }
                }
                Block::Table(table) => {
                    // Heading rows only at the top.
                    let mut top = true;
                    for row in &mut table.rows {
                        top &= row.heading;
                        row.heading = top;
                        for cell in &mut row.cells {
                            for border in cell.borders.iter_mut().flatten() {
                                line(border);
                            }
                            limit(&mut cell.blocks);
                        }
                    }
                }
            }
        }
    }
    let properties = &mut exchange.properties;
    for text in [
        &mut properties.title,
        &mut properties.description,
        &mut properties.keywords,
    ] {
        xml_text(text);
    }
    for (name, value) in &mut properties.custom {
        xml_text(name);
        xml_text(value);
    }
    properties.custom.retain(|(_, value)| !value.is_empty());
    for style in &mut exchange.styles {
        xml_text(&mut style.name);
        // No tabs and the default ones are written alike.
        if style.paragraph.tabs.as_ref().is_some_and(Vec::is_empty) {
            style.paragraph.tabs = Some(
                (1..=12)
                    .map(|stop| amipro_exchange::Tab {
                        position: 720 * stop,
                        kind: amipro_exchange::TabKind::Left,
                        leader: amipro_exchange::TabLeader::None,
                    })
                    .collect(),
            );
        }
        for side in [
            &mut style.paragraph.border_top,
            &mut style.paragraph.border_bottom,
        ] {
            if let Some(Some(rule)) = side {
                line(rule);
            }
        }
    }
    // Every level counts again after a higher one.
    for list in &mut exchange.lists {
        for level in &mut list.levels {
            level.restart_after_higher = true;
        }
    }
    for section in &mut exchange.sections {
        limit(&mut section.blocks);
        for set in [&mut section.headers, &mut section.footers] {
            for part in [&mut set.default, &mut set.even, &mut set.first]
                .into_iter()
                .flatten()
            {
                limit(part);
            }
        }
        if let Some(border) = &mut section.page.border {
            line(&mut border.line);
            // The border at the margins.
            border.distance = 0;
        }
        // A floating header or footer starts a page style of its own, which
        // starts a page: a blank one is none.
        for set in [&mut section.headers, &mut section.footers] {
            for part in [&mut set.default, &mut set.even, &mut set.first] {
                let blank = part.as_ref().is_some_and(|blocks| {
                    blocks.iter().all(|block| {
                        matches!(block, Block::Paragraph(paragraph) if paragraph.inlines.is_empty())
                    })
                });
                if blank {
                    *part = None;
                }
            }
        }
        let page = &mut section.page;
        // The header and footer inside the margins.
        if !section.headers.is_empty() {
            page.margin_top = page.margin_top.max(page.header_distance);
        }
        if !section.footers.is_empty() {
            page.margin_bottom = page.margin_bottom.max(page.footer_distance);
        }
        // A page without a header has no header distance in ODF.
        if section.headers.is_empty() {
            page.header_distance = page.margin_top / 2;
        }
        if section.footers.is_empty() {
            page.footer_distance = page.margin_bottom / 2;
        }
        // One column has no gap.
        if page.columns.count <= 1 {
            page.columns.gap = 720;
            page.columns.line_between = false;
        }
    }
    exchange
}

fn within_word(mut exchange: ExchangeDocument, format: &str) -> ExchangeDocument {
    let rtf = format == "rtf";
    fn line(line: &mut amipro_exchange::RuleLine, rtf: bool) {
        line.width = if rtf {
            // RTF: thick single lines are halved and doubled back.
            if line.kind == amipro_exchange::LineKind::Single && line.width > 75 {
                (line.width / 2).clamp(1, 75) * 2
            } else {
                line.width.clamp(1, 75)
            }
        } else {
            // DOCX: widths in eighths of a point, spacing in points.
            let eighths = (line.width * 8 / 20).clamp(2, 96);
            line.space = line.space / 20 * 20;
            ((eighths * 20 + 4) / 8).max(1)
        };
    }
    fn props(props: &mut amipro_exchange::CharProps) {
        // Half points.
        if let Some(size) = &mut props.size {
            *size = (*size + 5) / 10 * 10;
        }
    }
    fn default_tabs() -> Vec<amipro_exchange::Tab> {
        (1..=12)
            .map(|stop| amipro_exchange::Tab {
                position: 720 * stop,
                kind: amipro_exchange::TabKind::Left,
                leader: amipro_exchange::TabLeader::None,
            })
            .collect()
    }
    fn paragraph_props(props: &mut amipro_exchange::ParagraphProps, rtf: bool) {
        for side in [&mut props.border_top, &mut props.border_bottom] {
            if let Some(Some(rule)) = side {
                line(rule, rtf);
            }
        }
        // No tabs and the default ones are written alike.
        if props.tabs.as_ref().is_some_and(Vec::is_empty) {
            props.tabs = Some(default_tabs());
        }
    }
    fn limit(blocks: &mut [Block], rtf: bool) {
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => {
                    paragraph_props(&mut paragraph.props, rtf);
                    props(&mut paragraph.mark);
                    for inline in &mut paragraph.inlines {
                        match inline {
                            Inline::Text { props: p, .. } | Inline::Tab { props: p } => props(p),
                            Inline::Field(field) => props(&mut field.props),
                            Inline::Shape(shape) => limit_shape(shape, rtf),
                            _ => {}
                        }
                    }
                }
                Block::Table(table) => {
                    for row in &mut table.rows {
                        for cell in &mut row.cells {
                            for border in cell.borders.iter_mut().flatten() {
                                line(border, rtf);
                                border.space = 0;
                            }
                            limit(&mut cell.blocks, rtf);
                        }
                    }
                }
            }
        }
    }
    fn limit_shape(shape: &mut amipro_exchange::Shape, rtf: bool) {
        // A whole outline only.
        if let Some(line) = &mut shape.line {
            line.sides = [true; 4];
        }
        let (width, height) = (shape.width, shape.height);
        match &mut shape.content {
            ShapeContent::Text {
                blocks: inner,
                columns,
                ..
            } => {
                if rtf {
                    // Text boxes: square corners (EC11), one column.
                    shape.corner_radius = 0;
                    *columns = 1;
                }
                limit(inner, rtf);
            }
            ShapeContent::Picture(picture) => {
                if rtf && !matches!(shape.anchor, amipro_exchange::Anchor::Inline { .. }) {
                    // A shape's picture fills it.
                    picture.left = 0;
                    picture.top = 0;
                    picture.width = width;
                    picture.height = height;
                }
                // A picture filling its frame is the picture, without
                // the frame's look (EC7).
                let fills = picture.left == 0
                    && picture.top == 0
                    && picture.width == width
                    && picture.height == height;
                if fills || matches!(shape.anchor, amipro_exchange::Anchor::Inline { .. }) && rtf {
                    if rtf {
                        shape.name = "Frame".into();
                    }
                    shape.line = None;
                    shape.fill = None;
                    shape.shadow = None;
                    shape.corner_radius = 0;
                }
            }
            ShapeContent::Empty => {}
        }
        if let amipro_exchange::Anchor::Inline { drop } = &mut shape.anchor {
            // No format lowers a frame below the baseline.
            *drop = 0;
        }
        let below_paragraph = matches!(
            shape.anchor,
            amipro_exchange::Anchor::Floating {
                vertical: (amipro_exchange::VerticalFrom::Paragraph, _),
                ..
            }
        );
        if rtf
            && (below_paragraph || matches!(shape.anchor, amipro_exchange::Anchor::Inline { .. }))
            && !matches!(shape.content, ShapeContent::Picture(_))
        {
            // RTF: no frame in the line but a picture.
            shape.anchor = amipro_exchange::Anchor::Floating {
                horizontal: (amipro_exchange::HorizontalFrom::Column, 0),
                vertical: (amipro_exchange::VerticalFrom::Paragraph, 0),
                wrap: amipro_exchange::ShapeWrap::TopAndBottom,
                behind: false,
            };
        }
    }
    for style in &mut exchange.styles {
        props(&mut style.character);
        paragraph_props(&mut style.paragraph, rtf);
        // No style breaks to a column; names as Word spells its own.
        style.paragraph.column_break_before = Some(false);
        style.name = style.name.to_lowercase();
    }
    for list in &mut exchange.lists {
        if let Some(first) = list.levels.first_mut() {
            first.restart_after_higher = false;
        }
    }
    for section in &mut exchange.sections {
        limit(&mut section.blocks, rtf);
        let columns = &mut section.page.columns;
        if columns.count <= 1 {
            columns.line_between = false;
        }
        if columns.widths.is_some() {
            columns.gap = 0;
        }
        if let Some(border) = &mut section.page.border {
            line(&mut border.line, rtf);
            border.distance = (border.distance / 20).clamp(0, 31) * 20;
        }
        for set in [&mut section.headers, &mut section.footers] {
            for part in [&mut set.default, &mut set.even, &mut set.first]
                .into_iter()
                .flatten()
            {
                limit(part, rtf);
            }
        }
    }
    exchange
}

/// Built, written as `format`, read back, realised, and built again.
fn format_round_trip(document: &Document, format: &str) -> Vec<String> {
    if format == "text" {
        return text_round_trip(document);
    }
    let first = within_format(normalise(built(document)), format);
    let inflate = |data: &[u8], size: usize| -> Option<Vec<u8>> {
        miniz_oxide::inflate::decompress_to_vec_with_limit(data, size).ok()
    };
    let deflate = |data: &[u8]| miniz_oxide::deflate::compress_to_vec(data, 6);
    let mut findings = Findings::default();
    let read = match format {
        "rtf" => {
            amipro_exchange::read_rtf::read_rtf(&amipro_exchange::rtf::write(&first), &mut findings)
        }
        "docx" => amipro_exchange::read_docx::read_docx(
            &amipro_exchange::docx::write(&first, Some(&deflate)),
            &inflate,
            &mut findings,
        ),
        "odt" => amipro_exchange::read_odt::read_odt(
            &amipro_exchange::odt::write(&first, Some(&deflate)),
            &inflate,
            &mut findings,
        ),
        _ => unreachable!(),
    };
    let read = match read {
        Ok(read) => read,
        Err(error) => return vec![format!("not read: {error}")],
    };
    let (again, _) = realised(&read);
    if let Err(error) = again.validate() {
        return vec![format!("the realised document is not valid: {error}")];
    }
    let second = within_format(normalise(built(&again)), format);
    differences(&first, &second)
}

/// Plain text: the text of a document exported, read back (each line a
/// paragraph, IM9's default), realised, and exported again, equal line for
/// line.
fn text_round_trip(document: &Document) -> Vec<String> {
    let first = amipro_exchange::text::write(&built(document));
    let read = amipro_exchange::read_text::read_text(
        &first,
        &amipro_exchange::import::TextOptions::default(),
    );
    let (again, _) = realised(&read);
    if let Err(error) = again.validate() {
        return vec![format!("the realised document is not valid: {error}")];
    }
    let second = amipro_exchange::text::write(&built(&again));
    if first == second {
        return Vec::new();
    }
    let (a, b) = (
        String::from_utf8_lossy(&first),
        String::from_utf8_lossy(&second),
    );
    let (a, b): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    let line = a
        .iter()
        .zip(&b)
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    vec![format!(
        "text line {line} of {} against {}: [text] {:?}\n    against {:?}",
        a.len(),
        b.len(),
        a.get(line).unwrap_or(&""),
        b.get(line).unwrap_or(&"")
    )]
}

fn round_trip(document: &Document) -> Vec<String> {
    if let Ok(format) = std::env::var("AMIPRO_REALISE_FORMAT")
        && !format.is_empty()
    {
        return format_round_trip(document, &format);
    }
    let first = normalise(built(document));
    let (again, _) = realised(&first);
    if let Err(error) = again.validate() {
        return vec![format!("the realised document is not valid: {error}")];
    }
    let second = normalise(built(&again));
    differences(&first, &second)
}

#[test]
fn styles_runs_and_paragraphs_come_back() {
    let mut document = Document::with_paragraphs([
        ("Title", StyleId(2)),
        ("plain bold plain", StyleId(1)),
        ("Second", StyleId(3)),
    ]);
    document.paragraph_mut(1).runs = vec![amipro_document::CharacterRun {
        start: 6,
        end: 10,
        format: amipro_document::CharacterFormat {
            bold: Some(true),
            ..amipro_document::CharacterFormat::default()
        },
    }];
    assert_eq!(round_trip(&document), Vec::<String>::new());
}

fn sam_files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sam_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
        {
            out.push(path);
        }
    }
}

/// Every SAM under `AMIPRO_REALISE_DIR` (with `AMIPRO_STYLES`) built,
/// realised, and built again; aggregate counts, and with
/// `AMIPRO_REALISE_DETAIL=NAME` one file's differences.
#[test]
#[ignore = "manual: private files"]
fn realise_round_trip_report() {
    let Some(directory) = std::env::var_os("AMIPRO_REALISE_DIR") else {
        eprintln!("AMIPRO_REALISE_DIR is not set");
        return;
    };
    let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    let detail = std::env::var("AMIPRO_REALISE_DETAIL").ok();
    let mut files = Vec::new();
    sam_files(Path::new(&directory), &mut files);
    let (mut read, mut equal) = (0, 0);
    let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
    for path in &files {
        let Ok(imported) = amipro_formats::import_sam_path_with_style_directories(path, &styles)
        else {
            continue;
        };
        read += 1;
        let found = std::panic::catch_unwind(|| round_trip(&imported.document))
            .unwrap_or_else(|_| vec!["panic".into()]);
        if found.is_empty() {
            equal += 1;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if detail.as_deref() == Some("*") && !found.is_empty() {
            eprintln!("== {name}: {}", found[0]);
        }
        if detail.as_deref() == Some(name.as_str()) {
            for line in &found {
                eprintln!("{line}");
            }
        }
        for line in &found {
            let head = line.split('\n').next().unwrap_or("");
            let place: String = head
                .split([':', '"'])
                .next()
                .unwrap_or("")
                .split_whitespace()
                .filter(|word| !word.chars().all(|c| c.is_ascii_digit()))
                .collect::<Vec<_>>()
                .join(" ");
            let field = head
                .split_once('[')
                .and_then(|(_, rest)| rest.split_once(']'))
                .map_or("", |(field, _)| field);
            let kind = if head.starts_with("the realised document") {
                head.to_owned()
            } else {
                format!("{place} [{field}]")
            };
            *kinds.entry(kind).or_default() += 1;
        }
    }
    eprintln!("read {read}, the exchange document again equal {equal}");
    for (kind, count) in kinds {
        eprintln!("  {kind}: {count}");
    }
}

/// Notes, footnotes, and bookmarks (the notes run, NR3): built, written as
/// each format, read back, and realised as Ami Pro's notes, footnotes, and
/// bookmarks again.
#[test]
fn notes_footnotes_and_bookmarks_come_back_through_every_format() {
    use amipro_document::InlineObject;
    let sam = "[ver]\r\n\t4\r\n[edoc]\r\nOne <:Zmark>two<:Z~mark> three.<:N1793037315,LS,65535,1,1\r\nA note.\r\n\r\n>\r\n<:F\r\n@Footnote@The footnote.\r\n\r\n>\r\n\r\nAfter.\r\n\r\n>\r\n";
    let document = amipro_formats::import_sam(sam.as_bytes(), None, "Fixture")
        .unwrap()
        .document;
    for format in ["docx", "odt", "rtf"] {
        // RTF keeps a note's date to the minute (its DTTM) and counts at
        // least one revision: equal elsewhere.
        assert!(
            format == "rtf" || format_round_trip(&document, format).is_empty(),
            "{format}: {:?}",
            format_round_trip(&document, format)
        );
        let first = within_format(normalise(built(&document)), format);
        let inflate = |data: &[u8], size: usize| -> Option<Vec<u8>> {
            miniz_oxide::inflate::decompress_to_vec_with_limit(data, size).ok()
        };
        let deflate = |data: &[u8]| miniz_oxide::deflate::compress_to_vec(data, 6);
        let mut findings = Findings::default();
        let read = match format {
            "rtf" => amipro_exchange::read_rtf::read_rtf(
                &amipro_exchange::rtf::write(&first),
                &mut findings,
            ),
            "docx" => amipro_exchange::read_docx::read_docx(
                &amipro_exchange::docx::write(&first, Some(&deflate)),
                &inflate,
                &mut findings,
            ),
            _ => amipro_exchange::read_odt::read_odt(
                &amipro_exchange::odt::write(&first, Some(&deflate)),
                &inflate,
                &mut findings,
            ),
        }
        .unwrap();
        let (again, _) = realised(&read);
        let objects: Vec<&InlineObject> = again
            .paragraphs()
            .iter()
            .flat_map(|paragraph| &paragraph.objects)
            .collect();
        assert!(
            objects
                .iter()
                .any(|object| matches!(object, InlineObject::Footnote { .. })),
            "{format}: a footnote"
        );
        assert!(
            objects
                .iter()
                .any(|object| matches!(object, InlineObject::Note { .. })),
            "{format}: a note"
        );
        let names: Vec<String> = amipro_document::bookmarks::bookmarks(&again)
            .into_iter()
            .map(|bookmark| bookmark.name)
            .collect();
        assert_eq!(names, ["mark"], "{format}");
    }
}
