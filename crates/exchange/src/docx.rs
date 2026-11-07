//! The Word Document filter (ECMA-376, Office Open XML; EX1): the exchange
//! document as a zip of WordprocessingML parts. Frames are DrawingML text
//! boxes and pictures, anchored where Ami Pro places them (EX3); lists are
//! `numbering.xml` lists, fields Word's own (EX8); Ami Pro's style names
//! are kept (EX9); Doc Info is the package's properties (EX10).

use std::fmt::Write as _;

use crate::xml::escape;
use crate::zip::{self, Deflate, Entry};
use crate::{
    Alignment, Anchor, Block, Cell, CharProps, ExchangeDocument, Field, FieldKind, FontFamily,
    HorizontalFrom, Inline, LineKind, LineSpacing, NumberFormat, Paragraph, ParagraphProps,
    PlacedPicture, Row, RowHeight, RuleLine, Script, Section, Shape, ShapeContent, ShapeWrap, Tab,
    TabKind, TabLeader, Table, Twips, Underline, VerticalFrom, VerticalMerge, hex,
};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const WP: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const PIC: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";
const WPS: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingShape";
const MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const W14: &str = "http://schemas.microsoft.com/office/word/2010/wordml";
const ASVG: &str = "http://schemas.microsoft.com/office/drawing/2016/SVG/main";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const IMAGE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

/// English Metric Units in a twip.
const EMU: i64 = 635;

/// The document as a DOCX package; `deflate` compresses its parts.
pub fn write(document: &ExchangeDocument, deflate: Deflate) -> Vec<u8> {
    zip::write(&parts(document), deflate)
}

/// The package's parts, in the order they are zipped.
pub fn parts(document: &ExchangeDocument) -> Vec<Entry> {
    let mut package = Package::new(document);
    let body = package.body();
    let mut entries = vec![
        Entry::new("[Content_Types].xml", package.content_types()),
        Entry::new("_rels/.rels", package.root_rels()),
        Entry::new("word/document.xml", body.xml),
        Entry::new(
            "word/_rels/document.xml.rels",
            package.document_rels(&body.rels),
        ),
        Entry::new("word/styles.xml", package.styles()),
        Entry::new("word/numbering.xml", package.numbering()),
        Entry::new("word/settings.xml", package.settings()),
        Entry::new("word/fontTable.xml", package.fonts()),
        Entry::new("docProps/core.xml", package.core()),
        Entry::new("docProps/app.xml", package.app()),
        Entry::new("docProps/custom.xml", package.custom()),
    ];
    // Footnotes, endnotes, and comments (Ami Pro's notes), when there are.
    for (name, notes, part) in [
        (
            "footnotes",
            package.notes_xml(false),
            std::mem::take(&mut package.footnote_part),
        ),
        (
            "endnotes",
            package.notes_xml(true),
            std::mem::take(&mut package.endnote_part),
        ),
        (
            "comments",
            package.comments_xml(),
            std::mem::take(&mut package.comment_part),
        ),
    ] {
        let Some(notes) = notes else {
            continue;
        };
        if !part.rels.is_empty() {
            entries.push(Entry::new(
                format!("word/_rels/{name}.xml.rels"),
                rels_xml(&part.rels),
            ));
        }
        entries.push(Entry::new(format!("word/{name}.xml"), notes));
    }
    for (name, part) in std::mem::take(&mut package.headers) {
        if !part.rels.is_empty() {
            entries.push(Entry::new(
                format!("word/_rels/{name}.rels"),
                rels_xml(&part.rels),
            ));
        }
        entries.push(Entry::new(format!("word/{name}"), part.xml));
    }
    for (index, image) in document.images.iter().enumerate() {
        entries.push(Entry::new(
            format!("word/media/image{}.{}", index + 1, image.kind.extension()),
            image.bytes.to_vec(),
        ));
        if let Some(fallback) = &image.fallback {
            entries.push(Entry::new(
                format!("word/media/image{}.png", index + 1),
                fallback.to_vec(),
            ));
        }
    }
    entries
}

/// A part's XML and its relationships (id, type, target).
#[derive(Default)]
struct Part {
    xml: String,
    rels: Vec<(String, String, String)>,
}

impl Part {
    /// The relationship id of image `index` in this part, added once.
    fn image(&mut self, document: &ExchangeDocument, index: usize, fallback: bool) -> String {
        let image = &document.images[index];
        let extension = if fallback {
            "png"
        } else {
            image.kind.extension()
        };
        let target = format!("media/image{}.{extension}", index + 1);
        if let Some((id, ..)) = self
            .rels
            .iter()
            .find(|(_, _, existing)| *existing == target)
        {
            return id.clone();
        }
        let id = format!("rIdImage{}", self.rels.len() + 1);
        self.rels.push((id.clone(), IMAGE.into(), target));
        id
    }
}

struct Package<'a> {
    document: &'a ExchangeDocument,
    style_ids: Vec<String>,
    /// Header and footer parts by file name.
    headers: Vec<(String, Part)>,
    /// The next `docPr` id.
    drawing_id: u32,
    /// Each footnote's, endnote's, and comment's XML, and their parts'
    /// relationships (the notes run).
    footnotes: Vec<String>,
    endnotes: Vec<String>,
    comments: Vec<String>,
    footnote_part: Part,
    endnote_part: Part,
    comment_part: Part,
    /// Each bookmark's id, by name.
    bookmarks: Vec<String>,
}

impl<'a> Package<'a> {
    fn new(document: &'a ExchangeDocument) -> Self {
        Self {
            document,
            style_ids: style_ids(document),
            headers: Vec::new(),
            drawing_id: 1,
            footnotes: Vec::new(),
            endnotes: Vec::new(),
            comments: Vec::new(),
            footnote_part: Part::default(),
            endnote_part: Part::default(),
            comment_part: Part::default(),
            bookmarks: Vec::new(),
        }
    }

    // ---- The body ----

    fn body(&mut self) -> Part {
        let mut part = Part::default();
        let mut xml = String::new();
        let _ = write!(
            xml,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="{W}" xmlns:r="{R}" xmlns:wp="{WP}" xmlns:a="{A}" xmlns:pic="{PIC}" xmlns:wps="{WPS}" xmlns:mc="{MC}" xmlns:w14="{W14}" mc:Ignorable="w14"><w:body>"#
        );
        let sections = &self.document.sections;
        for (index, section) in sections.iter().enumerate() {
            let properties = self.section_properties(section, &mut part);
            let last = index + 1 == sections.len();
            let blocks = &section.blocks;
            for (at, block) in blocks.iter().enumerate() {
                let closing = !last && at + 1 == blocks.len();
                match block {
                    Block::Paragraph(paragraph) => {
                        let section = closing.then_some(properties.as_str());
                        self.paragraph(paragraph, section, &mut part, &mut xml);
                    }
                    Block::Table(table) => {
                        self.table(table, &mut part, &mut xml);
                        if closing {
                            let _ = write!(xml, "<w:p><w:pPr>{properties}</w:pPr></w:p>");
                        }
                    }
                }
            }
            if last {
                if matches!(blocks.last(), Some(Block::Table(_))) {
                    xml.push_str("<w:p/>");
                }
                xml.push_str(&properties);
            }
        }
        xml.push_str("</w:body></w:document>");
        part.xml = xml;
        part
    }

    /// A section's `w:sectPr`, its headers and footers written as parts.
    fn section_properties(&mut self, section: &Section, body: &mut Part) -> String {
        let mut xml = String::from("<w:sectPr>");
        for (kind, set, prefix) in [
            ("header", &section.headers, "header"),
            ("footer", &section.footers, "footer"),
        ] {
            for (slot, blocks) in [
                ("default", &set.default),
                ("even", &set.even),
                ("first", &set.first),
            ] {
                let Some(blocks) = blocks else {
                    continue;
                };
                let number = self.headers.len() + 1;
                let name = format!("{prefix}{number}.xml");
                let part = self.header_part(kind, blocks);
                self.headers.push((name.clone(), part));
                let id = format!("rId{kind}{number}");
                body.rels.push((id.clone(), format!("{REL}/{kind}"), name));
                let _ = write!(xml, r#"<w:{kind}Reference w:type="{slot}" r:id="{id}"/>"#);
            }
        }
        let page = &section.page;
        let _ = write!(
            xml,
            r#"<w:type w:val="{}"/><w:pgSz w:w="{}" w:h="{}"{}/><w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}" w:header="{}" w:footer="{}" w:gutter="0"/>"#,
            if section.new_page {
                "nextPage"
            } else {
                "continuous"
            },
            page.width,
            page.height,
            if page.landscape {
                r#" w:orient="landscape""#
            } else {
                ""
            },
            page.margin_top,
            page.margin_right,
            page.margin_bottom,
            page.margin_left,
            page.header_distance,
            page.footer_distance,
        );
        if let Some(border) = page.border {
            let side = |on: bool, name: &str| {
                if !on {
                    return String::new();
                }
                format!(
                    r#"<w:{name} w:val="{}" w:sz="{}" w:space="{}" w:color="{}"/>"#,
                    border_style(border.line.kind),
                    eighths(border.line.width),
                    (border.distance / 20).clamp(0, 31),
                    hex(border.line.colour)
                )
            };
            let _ = write!(
                xml,
                r#"<w:pgBorders w:offsetFrom="page">{}{}{}{}</w:pgBorders>"#,
                side(border.top, "top"),
                side(border.left, "left"),
                side(border.bottom, "bottom"),
                side(border.right, "right"),
            );
        }
        if let Some(numbers) = section.page_numbers {
            let _ = write!(
                xml,
                r#"<w:pgNumType w:fmt="{}"{}/>"#,
                number_format(numbers.format),
                numbers
                    .start
                    .map_or_else(String::new, |start| format!(r#" w:start="{start}""#))
            );
        }
        let columns = &page.columns;
        match &columns.widths {
            Some(widths) if columns.count > 1 => {
                let _ = write!(
                    xml,
                    r#"<w:cols w:num="{}" w:sep="{}" w:equalWidth="0">"#,
                    widths.len(),
                    u8::from(columns.line_between)
                );
                for (width, gap) in widths {
                    let _ = write!(xml, r#"<w:col w:w="{width}" w:space="{gap}"/>"#);
                }
                xml.push_str("</w:cols>");
            }
            _ => {
                let _ = write!(
                    xml,
                    r#"<w:cols w:num="{}" w:space="{}" w:sep="{}"/>"#,
                    columns.count.max(1),
                    columns.gap,
                    u8::from(columns.line_between && columns.count > 1)
                );
            }
        }
        if section.headers.first.is_some() || section.footers.first.is_some() {
            xml.push_str("<w:titlePg/>");
        }
        xml.push_str("</w:sectPr>");
        xml
    }

    fn header_part(&mut self, kind: &str, blocks: &[Block]) -> Part {
        let mut part = Part::default();
        let mut xml = String::new();
        let tag = if kind == "header" { "hdr" } else { "ftr" };
        let _ = write!(
            xml,
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:{tag} xmlns:w="{W}" xmlns:r="{R}" xmlns:wp="{WP}" xmlns:a="{A}" xmlns:pic="{PIC}" xmlns:wps="{WPS}" xmlns:mc="{MC}" xmlns:w14="{W14}" mc:Ignorable="w14">"#
        );
        self.blocks(blocks, &mut part, &mut xml);
        if blocks.is_empty() || matches!(blocks.last(), Some(Block::Table(_))) {
            xml.push_str("<w:p/>");
        }
        let _ = write!(xml, "</w:{tag}>");
        part.xml = xml;
        part
    }

    fn blocks(&mut self, blocks: &[Block], part: &mut Part, xml: &mut String) {
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => self.paragraph(paragraph, None, part, xml),
                Block::Table(table) => self.table(table, part, xml),
            }
        }
    }

    // ---- Paragraphs ----

    fn paragraph(
        &mut self,
        paragraph: &Paragraph,
        section: Option<&str>,
        part: &mut Part,
        xml: &mut String,
    ) {
        let document = self.document;
        let style = document.styles.get(paragraph.style);
        xml.push_str("<w:p><w:pPr>");
        let _ = write!(
            xml,
            r#"<w:pStyle w:val="{}"/>"#,
            self.style_ids
                .get(paragraph.style)
                .map_or("Normal", String::as_str)
        );
        let base_tabs = style.and_then(|style| style.paragraph.tabs.as_deref());
        let numbering = match paragraph.list {
            Some((Some(instance), level)) => Some((instance + 1, level)),
            Some((None, _)) => Some((0, 0)),
            None => None,
        };
        paragraph_properties(xml, &paragraph.props, numbering, base_tabs);
        let mark = char_properties(&paragraph.mark);
        if !mark.is_empty() {
            let _ = write!(xml, "<w:rPr>{mark}</w:rPr>");
        }
        if let Some(section) = section {
            xml.push_str(section);
        }
        xml.push_str("</w:pPr>");
        // A number no list can show, as text before the paragraph's.
        if matches!(paragraph.list, Some((None, _)))
            && let Some(number) = &paragraph.number_text
        {
            run(xml, &CharProps::default(), &format!("{number}\t"));
        }
        for inline in &paragraph.inlines {
            self.inline(inline, part, xml);
        }
        xml.push_str("</w:p>");
    }

    fn inline(&mut self, inline: &Inline, part: &mut Part, xml: &mut String) {
        match inline {
            Inline::Text { text, props } => run(xml, props, text),
            Inline::Tab { props } => {
                let _ = write!(xml, "<w:r>{}<w:tab/></w:r>", run_properties(props));
            }
            Inline::LineBreak => xml.push_str("<w:r><w:br/></w:r>"),
            Inline::PageBreak => xml.push_str(r#"<w:r><w:br w:type="page"/></w:r>"#),
            Inline::ColumnBreak => xml.push_str(r#"<w:r><w:br w:type="column"/></w:r>"#),
            Inline::Field(field) => self.field(field, xml),
            Inline::Symbol {
                character,
                font,
                props,
            } => {
                let mut props = props.clone();
                if font.is_some() {
                    props.font.clone_from(font);
                }
                run(xml, &props, &character.to_string());
            }
            Inline::Shape(shape) => self.shape(shape, part, xml),
            Inline::Footnote(footnote) => self.footnote(footnote, xml),
            Inline::Note(note) => self.comment(note, xml),
            Inline::Bookmark { name, end } => {
                let id = match self.bookmarks.iter().position(|known| known == name) {
                    Some(index) => index,
                    None => {
                        self.bookmarks.push(name.clone());
                        self.bookmarks.len() - 1
                    }
                };
                if *end {
                    let _ = write!(xml, r#"<w:bookmarkEnd w:id="{id}"/>"#);
                } else {
                    let _ = write!(
                        xml,
                        r#"<w:bookmarkStart w:id="{id}" w:name="{}"/>"#,
                        escape(name)
                    );
                }
            }
            Inline::Marker(_) => {}
        }
    }

    /// A footnote or endnote: its reference here, its text in its part,
    /// the first paragraph starting with the note's own mark. Ids 0 and 1
    /// are the separators.
    fn footnote(&mut self, footnote: &crate::Footnote, xml: &mut String) {
        let (kind, list_len) = if footnote.endnote {
            ("endnote", self.endnotes.len())
        } else {
            ("footnote", self.footnotes.len())
        };
        let id = list_len + 2;
        let mut part = std::mem::take(if footnote.endnote {
            &mut self.endnote_part
        } else {
            &mut self.footnote_part
        });
        let mut inner = String::new();
        self.blocks(&footnote.blocks, &mut part, &mut inner);
        if inner.is_empty() {
            inner.push_str("<w:p><w:pPr></w:pPr></w:p>");
        }
        let reference = format!(
            r#"<w:r><w:rPr><w:vertAlign w:val="superscript"/></w:rPr><w:{kind}Ref/></w:r>"#
        );
        if let Some(at) = inner.find("</w:pPr>") {
            inner.insert_str(at + "</w:pPr>".len(), &reference);
        }
        let body = format!(r#"<w:{kind} w:id="{id}">{inner}</w:{kind}>"#);
        if footnote.endnote {
            self.endnotes.push(body);
            self.endnote_part = part;
        } else {
            self.footnotes.push(body);
            self.footnote_part = part;
        }
        let mut props = footnote.props.clone();
        props.script = Some(crate::Script::Super);
        let _ = write!(
            xml,
            r#"<w:r>{}<w:{kind}Reference w:id="{id}"/></w:r>"#,
            run_properties(&props)
        );
    }

    /// A note as a comment at its place, with its author, initials, date,
    /// and text.
    fn comment(&mut self, note: &crate::Note, xml: &mut String) {
        let id = self.comments.len();
        let mut part = std::mem::take(&mut self.comment_part);
        let mut inner = String::new();
        self.blocks(&note.blocks, &mut part, &mut inner);
        if inner.is_empty() {
            inner.push_str("<w:p/>");
        }
        self.comment_part = part;
        let date = note
            .date
            .map(|date| format!(r#" w:date="{}""#, w3c_time(&date)))
            .unwrap_or_default();
        self.comments.push(format!(
            r#"<w:comment w:id="{id}" w:author="{}" w:initials="{}"{date}>{inner}</w:comment>"#,
            escape(&note.author),
            escape(&note.initials)
        ));
        let _ = write!(
            xml,
            r#"<w:commentRangeStart w:id="{id}"/><w:commentRangeEnd w:id="{id}"/><w:r><w:commentReference w:id="{id}"/></w:r>"#
        );
    }

    /// `footnotes.xml` or `endnotes.xml`, with Word's two separators.
    fn notes_xml(&self, endnotes: bool) -> Option<String> {
        let (kind, notes) = if endnotes {
            ("endnote", &self.endnotes)
        } else {
            ("footnote", &self.footnotes)
        };
        if notes.is_empty() {
            return None;
        }
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:{kind}s xmlns:w="{W}" xmlns:r="{R}" xmlns:wp="{WP}" xmlns:a="{A}" xmlns:pic="{PIC}" xmlns:wps="{WPS}" xmlns:mc="{MC}" xmlns:w14="{W14}" mc:Ignorable="w14"><w:{kind} w:type="separator" w:id="0"><w:p><w:r><w:separator/></w:r></w:p></w:{kind}><w:{kind} w:type="continuationSeparator" w:id="1"><w:p><w:r><w:continuationSeparator/></w:r></w:p></w:{kind}>"#
        );
        for note in notes {
            xml.push_str(note);
        }
        let _ = write!(xml, "</w:{kind}s>");
        Some(xml)
    }

    fn comments_xml(&self) -> Option<String> {
        if self.comments.is_empty() {
            return None;
        }
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:comments xmlns:w="{W}" xmlns:r="{R}" xmlns:wp="{WP}" xmlns:a="{A}" xmlns:pic="{PIC}" xmlns:wps="{WPS}" xmlns:mc="{MC}" xmlns:w14="{W14}" mc:Ignorable="w14">"#
        );
        for comment in &self.comments {
            xml.push_str(comment);
        }
        xml.push_str("</w:comments>");
        Some(xml)
    }

    fn field(&self, field: &Field, xml: &mut String) {
        let rpr = run_properties(&field.props);
        let instruction = crate::fields::instruction(&field.kind);
        let _ = write!(
            xml,
            r#"<w:r>{rpr}<w:fldChar w:fldCharType="begin"/></w:r><w:r>{rpr}<w:instrText xml:space="preserve"> {} </w:instrText></w:r>"#,
            escape(&instruction)
        );
        if !matches!(
            field.kind,
            FieldKind::IndexEntry { .. } | FieldKind::ContentsEntry { .. }
        ) {
            let _ = write!(
                xml,
                r#"<w:r>{rpr}<w:fldChar w:fldCharType="separate"/></w:r>"#
            );
            run(xml, &field.props, &field.result);
        }
        let _ = write!(xml, r#"<w:r>{rpr}<w:fldChar w:fldCharType="end"/></w:r>"#);
    }

    // ---- Tables ----

    fn table(&mut self, table: &Table, part: &mut Part, xml: &mut String) {
        // Word cannot break a page before a row: the table is split there,
        // with its heading rows again after a paragraph that breaks (and a
        // break before the first row is that paragraph before the table).
        let mut starts: Vec<usize> = table
            .rows
            .iter()
            .enumerate()
            .filter(|(index, row)| *index > 0 && row.page_break_before)
            .map(|(index, _)| index)
            .collect();
        starts.insert(0, 0);
        let headings: Vec<&Row> = table.rows.iter().filter(|row| row.heading).collect();
        for (at, start) in starts.iter().enumerate() {
            let end = starts.get(at + 1).copied().unwrap_or(table.rows.len());
            if start == &end {
                continue;
            }
            let mut rows: Vec<&Row> = Vec::new();
            if table.rows[*start].page_break_before {
                xml.push_str(
                    r#"<w:p><w:pPr><w:pageBreakBefore/><w:spacing w:before="0" w:after="0" w:line="20" w:lineRule="exact"/><w:rPr><w:sz w:val="2"/></w:rPr></w:pPr></w:p>"#,
                );
            }
            if *start > 0 {
                rows.extend(
                    headings
                        .iter()
                        .copied()
                        .filter(|row| !std::ptr::eq(*row, &table.rows[*start])),
                );
            }
            rows.extend(table.rows[*start..end].iter());
            self.table_rows(table, &rows, part, xml);
        }
    }

    fn table_rows(&mut self, table: &Table, rows: &[&Row], part: &mut Part, xml: &mut String) {
        let width: Twips = table.columns.iter().sum();
        let [top, right, bottom, left] = table.cell_margins;
        let _ = write!(
            xml,
            r#"<w:tbl><w:tblPr><w:tblW w:w="{width}" w:type="dxa"/><w:jc w:val="{}"/><w:tblInd w:w="{}" w:type="dxa"/><w:tblLayout w:type="fixed"/><w:tblCellMar><w:top w:w="{top}" w:type="dxa"/><w:left w:w="{left}" w:type="dxa"/><w:bottom w:w="{bottom}" w:type="dxa"/><w:right w:w="{right}" w:type="dxa"/></w:tblCellMar><w:tblLook w:val="0000"/></w:tblPr><w:tblGrid>"#,
            match table.align {
                Alignment::Centre => "center",
                Alignment::Right => "right",
                _ => "left",
            },
            table.indent
        );
        for column in &table.columns {
            let _ = write!(xml, r#"<w:gridCol w:w="{column}"/>"#);
        }
        xml.push_str("</w:tblGrid>");
        for row in rows {
            xml.push_str("<w:tr><w:trPr>");
            if row.keep_together {
                xml.push_str("<w:cantSplit/>");
            }
            let (height, rule) = match row.height {
                RowHeight::AtLeast(height) => (height, "atLeast"),
                RowHeight::Exact(height) => (height, "exact"),
            };
            let _ = write!(xml, r#"<w:trHeight w:val="{height}" w:hRule="{rule}"/>"#);
            if row.heading {
                xml.push_str("<w:tblHeader/>");
            }
            xml.push_str("</w:trPr>");
            let mut column = 0;
            for cell in &row.cells {
                let span = usize::from(cell.span.max(1));
                let width: Twips = table.columns.iter().skip(column).take(span).sum();
                column += span;
                self.cell(cell, width, part, xml);
            }
            xml.push_str("</w:tr>");
        }
        xml.push_str("</w:tbl>");
    }

    fn cell(&mut self, cell: &Cell, width: Twips, part: &mut Part, xml: &mut String) {
        let _ = write!(xml, r#"<w:tc><w:tcPr><w:tcW w:w="{width}" w:type="dxa"/>"#);
        if cell.span > 1 {
            let _ = write!(xml, r#"<w:gridSpan w:val="{}"/>"#, cell.span);
        }
        match cell.merge {
            VerticalMerge::Start => xml.push_str(r#"<w:vMerge w:val="restart"/>"#),
            VerticalMerge::Continue => xml.push_str("<w:vMerge/>"),
            VerticalMerge::None => {}
        }
        if cell.borders.iter().any(Option::is_some) {
            xml.push_str("<w:tcBorders>");
            for (side, name) in [(0, "top"), (3, "left"), (2, "bottom"), (1, "right")] {
                match cell.borders[side] {
                    Some(line) => {
                        let _ = write!(
                            xml,
                            r#"<w:{name} w:val="{}" w:sz="{}" w:space="0" w:color="{}"/>"#,
                            border_style(line.kind),
                            eighths(line.width),
                            hex(line.colour)
                        );
                    }
                    None => {
                        let _ = write!(xml, r#"<w:{name} w:val="nil"/>"#);
                    }
                }
            }
            xml.push_str("</w:tcBorders>");
        }
        if let Some(fill) = cell.shading {
            let _ = write!(
                xml,
                r#"<w:shd w:val="clear" w:color="auto" w:fill="{}"/>"#,
                hex(fill)
            );
        }
        xml.push_str("</w:tcPr>");
        self.blocks(&cell.blocks, part, xml);
        if cell.blocks.is_empty() || matches!(cell.blocks.last(), Some(Block::Table(_))) {
            xml.push_str("<w:p/>");
        }
        xml.push_str("</w:tc>");
    }

    // ---- Shapes ----

    fn shape(&mut self, shape: &Shape, part: &mut Part, xml: &mut String) {
        let id = self.drawing_id;
        self.drawing_id += 1;
        let (cx, cy) = (emu(shape.width), emu(shape.height));
        let name = escape(&shape.name);
        let description = match &shape.content {
            ShapeContent::Picture(picture) => escape(&picture.description),
            _ => String::new(),
        };
        let graphic = self.graphic(shape, id, part);
        xml.push_str("<w:r><w:drawing>");
        match shape.anchor {
            Anchor::Inline { .. } => {
                let _ = write!(
                    xml,
                    r#"<wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:docPr id="{id}" name="{name}" descr="{description}"/><wp:cNvGraphicFramePr/>{graphic}</wp:inline>"#
                );
            }
            Anchor::Floating {
                horizontal,
                vertical,
                wrap,
                behind,
            } => {
                let horizontal_from = match horizontal.0 {
                    HorizontalFrom::Page => "page",
                    HorizontalFrom::Margin => "margin",
                    HorizontalFrom::Column => "column",
                };
                let vertical_from = match vertical.0 {
                    VerticalFrom::Page => "page",
                    VerticalFrom::Margin => "margin",
                    VerticalFrom::Paragraph => "paragraph",
                };
                let wrap = match wrap {
                    ShapeWrap::Square => r#"<wp:wrapSquare wrapText="bothSides"/>"#,
                    ShapeWrap::TopAndBottom => "<wp:wrapTopAndBottom/>",
                    ShapeWrap::None => "<wp:wrapNone/>",
                };
                let (dx, dy) = shape.shadow.map_or((0, 0), |shadow| {
                    (emu(shadow.dx.max(0)), emu(shadow.dy.max(0)))
                });
                let _ = write!(
                    xml,
                    r#"<wp:anchor distT="0" distB="{dy}" distL="0" distR="{dx}" simplePos="0" relativeHeight="{}" behindDoc="{}" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="{horizontal_from}"><wp:posOffset>{}</wp:posOffset></wp:positionH><wp:positionV relativeFrom="{vertical_from}"><wp:posOffset>{}</wp:posOffset></wp:positionV><wp:extent cx="{cx}" cy="{cy}"/><wp:effectExtent l="0" t="0" r="{dx}" b="{dy}"/>{wrap}<wp:docPr id="{id}" name="{name}" descr="{description}"/><wp:cNvGraphicFramePr/>{graphic}</wp:anchor>"#,
                    (shape.z + 1) * 1024,
                    u8::from(behind),
                    emu(horizontal.1),
                    emu(vertical.1),
                );
            }
        }
        xml.push_str("</w:drawing></w:r>");
    }

    /// The shape's `a:graphic`: a picture, or a text box (an empty one for
    /// a frame without content).
    fn graphic(&mut self, shape: &Shape, id: u32, part: &mut Part) -> String {
        let (cx, cy) = (emu(shape.width), emu(shape.height));
        let outline = shape_outline(shape);
        match &shape.content {
            ShapeContent::Picture(picture) => {
                // The part of the picture the frame shows.
                let visible = visible_part(picture, shape.width, shape.height);
                let Some((left, top, right, bottom)) = visible else {
                    return self.empty_graphic(shape);
                };
                let crop = crop_rect(picture, left, top, right, bottom);
                if (left, top, right, bottom) == (0, 0, shape.width, shape.height) {
                    return self.picture_graphic(picture, id, cx, cy, &crop, &outline, part);
                }
                // A picture that does not fill its frame: a box with the
                // frame's look, the picture in it at its place (LibreOffice
                // does not take a crop that reaches past a picture).
                let inner_id = self.drawing_id;
                self.drawing_id += 1;
                let (width, height) = (emu(right - left), emu(bottom - top));
                let inner = self.picture_graphic(picture, inner_id, width, height, &crop, "", part);
                let content = format!(
                    r#"<w:p><w:pPr><w:spacing w:before="0" w:after="0" w:line="240" w:lineRule="auto"/><w:ind w:left="0" w:right="0" w:firstLine="0"/><w:jc w:val="left"/><w:rPr><w:sz w:val="2"/></w:rPr></w:pPr><w:r><w:rPr><w:sz w:val="2"/></w:rPr><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{width}" cy="{height}"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:docPr id="{inner_id}" name="{}" descr="{}"/><wp:cNvGraphicFramePr/>{inner}</wp:inline></w:drawing></w:r></w:p>"#,
                    escape(&shape.name),
                    escape(&picture.description),
                );
                self.text_box(
                    cx,
                    cy,
                    &outline,
                    &content,
                    [top, shape.width - right, shape.height - bottom, left],
                    "",
                )
            }
            ShapeContent::Text {
                blocks,
                insets,
                columns,
                column_gap,
            } => {
                let mut content = String::new();
                self.blocks(blocks, part, &mut content);
                if blocks.is_empty() || matches!(blocks.last(), Some(Block::Table(_))) {
                    content.push_str("<w:p/>");
                }
                let columns = if *columns > 1 {
                    format!(r#" numCol="{columns}" spcCol="{}""#, emu(*column_gap))
                } else {
                    String::new()
                };
                self.text_box(cx, cy, &outline, &content, *insets, &columns)
            }
            ShapeContent::Empty => self.empty_graphic(shape),
        }
    }

    /// A frame without content: its box alone.
    fn empty_graphic(&self, shape: &Shape) -> String {
        let (cx, cy) = (emu(shape.width), emu(shape.height));
        let outline = shape_outline(shape);
        format!(
            r#"<a:graphic><a:graphicData uri="{WPS}"><wps:wsp><wps:cNvSpPr/><wps:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>{outline}</wps:spPr><wps:bodyPr/></wps:wsp></a:graphicData></a:graphic>"#
        )
    }

    /// A text box `cx` × `cy` with the frame's look and `content`, its
    /// text `insets` (top, right, bottom, left) in from its edges.
    fn text_box(
        &self,
        cx: i64,
        cy: i64,
        outline: &str,
        content: &str,
        insets: [Twips; 4],
        columns: &str,
    ) -> String {
        let [top, right, bottom, left] = insets;
        format!(
            r#"<a:graphic><a:graphicData uri="{WPS}"><wps:wsp><wps:cNvSpPr txBox="1"/><wps:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>{outline}</wps:spPr><wps:txbx><w:txbxContent>{content}</w:txbxContent></wps:txbx><wps:bodyPr rot="0" vert="horz" wrap="square" lIns="{}" tIns="{}" rIns="{}" bIns="{}" anchor="t" anchorCtr="0"{columns}><a:noAutofit/></wps:bodyPr></wps:wsp></a:graphicData></a:graphic>"#,
            emu(left),
            emu(top),
            emu(right),
            emu(bottom),
        )
    }

    /// A picture's `a:graphic`, `cx` × `cy`, cropped by `crop`.
    #[allow(clippy::too_many_arguments)]
    fn picture_graphic(
        &self,
        picture: &PlacedPicture,
        id: u32,
        cx: i64,
        cy: i64,
        crop: &str,
        outline: &str,
        part: &mut Part,
    ) -> String {
        let image = &self.document.images[picture.image];
        let blip = if image.fallback.is_some() {
            let png = part.image(self.document, picture.image, true);
            let svg = part.image(self.document, picture.image, false);
            format!(
                r#"<a:blip r:embed="{png}"><a:extLst><a:ext uri="{{96DAC541-7B7A-43D3-8B79-37D633B846F1}}"><asvg:svgBlip xmlns:asvg="{ASVG}" r:embed="{svg}"/></a:ext></a:extLst></a:blip>"#
            )
        } else {
            let embed = part.image(self.document, picture.image, false);
            format!(r#"<a:blip r:embed="{embed}"/>"#)
        };
        let outline = if outline.is_empty() {
            r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:noFill/></a:ln>"#
        } else {
            outline
        };
        format!(
            r#"<a:graphic><a:graphicData uri="{PIC}"><pic:pic><pic:nvPicPr><pic:cNvPr id="{id}" name="Picture {id}" descr="{}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill>{blip}{crop}<a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>{outline}</pic:spPr></pic:pic></a:graphicData></a:graphic>"#,
            escape(&picture.description),
        )
    }

    // ---- The other parts ----

    fn content_types(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="jpeg" ContentType="image/jpeg"/><Default Extension="gif" ContentType="image/gif"/><Default Extension="bmp" ContentType="image/bmp"/><Default Extension="wmf" ContentType="image/x-wmf"/><Default Extension="emf" ContentType="image/x-emf"/><Default Extension="svg" ContentType="image/svg+xml"/>"#,
        );
        let main = "application/vnd.openxmlformats-officedocument.wordprocessingml";
        for (part, kind) in [
            ("/word/document.xml", format!("{main}.document.main+xml")),
            ("/word/styles.xml", format!("{main}.styles+xml")),
            ("/word/numbering.xml", format!("{main}.numbering+xml")),
            ("/word/settings.xml", format!("{main}.settings+xml")),
            ("/word/fontTable.xml", format!("{main}.fontTable+xml")),
            (
                "/docProps/core.xml",
                "application/vnd.openxmlformats-package.core-properties+xml".into(),
            ),
            (
                "/docProps/app.xml",
                "application/vnd.openxmlformats-officedocument.extended-properties+xml".into(),
            ),
            (
                "/docProps/custom.xml",
                "application/vnd.openxmlformats-officedocument.custom-properties+xml".into(),
            ),
        ] {
            let _ = write!(xml, r#"<Override PartName="{part}" ContentType="{kind}"/>"#);
        }
        for (kind, present) in [
            ("footnotes", !self.footnotes.is_empty()),
            ("endnotes", !self.endnotes.is_empty()),
            ("comments", !self.comments.is_empty()),
        ] {
            if present {
                let _ = write!(
                    xml,
                    r#"<Override PartName="/word/{kind}.xml" ContentType="{main}.{kind}+xml"/>"#
                );
            }
        }
        for (name, _) in &self.headers {
            let kind = if name.starts_with("header") {
                "header"
            } else {
                "footer"
            };
            let _ = write!(
                xml,
                r#"<Override PartName="/word/{name}" ContentType="{main}.{kind}+xml"/>"#
            );
        }
        xml.push_str("</Types>");
        xml
    }

    fn root_rels(&self) -> String {
        rels_xml(&[
            (
                "rId1".into(),
                format!("{REL}/officeDocument"),
                "word/document.xml".into(),
            ),
            (
                "rId2".into(),
                "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties".into(),
                "docProps/core.xml".into(),
            ),
            (
                "rId3".into(),
                format!("{REL}/extended-properties"),
                "docProps/app.xml".into(),
            ),
            (
                "rId4".into(),
                format!("{REL}/custom-properties"),
                "docProps/custom.xml".into(),
            ),
        ])
    }

    fn document_rels(&self, body: &[(String, String, String)]) -> String {
        let mut rels = vec![
            (
                "rIdStyles".into(),
                format!("{REL}/styles"),
                "styles.xml".into(),
            ),
            (
                "rIdNumbering".into(),
                format!("{REL}/numbering"),
                "numbering.xml".into(),
            ),
            (
                "rIdSettings".into(),
                format!("{REL}/settings"),
                "settings.xml".into(),
            ),
            (
                "rIdFonts".into(),
                format!("{REL}/fontTable"),
                "fontTable.xml".into(),
            ),
        ];
        for (kind, present) in [
            ("footnotes", !self.footnotes.is_empty()),
            ("endnotes", !self.endnotes.is_empty()),
            ("comments", !self.comments.is_empty()),
        ] {
            if present {
                rels.push((
                    format!("rId{kind}"),
                    format!("{REL}/{kind}"),
                    format!("{kind}.xml"),
                ));
            }
        }
        rels.extend(body.iter().cloned());
        rels_xml(&rels)
    }

    fn styles(&self) -> String {
        let document = self.document;
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="{W}">"#
        );
        let default_font = document
            .styles
            .first()
            .and_then(|style| style.character.font.clone())
            .unwrap_or_else(|| "Times New Roman".into());
        let _ = write!(
            xml,
            r#"<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="{0}" w:hAnsi="{0}" w:cs="{0}" w:eastAsia="{0}"/><w:sz w:val="24"/><w:szCs w:val="24"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>"#,
            escape(&default_font)
        );
        for (index, style) in document.styles.iter().enumerate() {
            let _ = write!(
                xml,
                r#"<w:style w:type="paragraph"{} w:styleId="{}"><w:name w:val="{}"/>"#,
                if index == 0 { r#" w:default="1""# } else { "" },
                self.style_ids[index],
                escape(&style.name)
            );
            if let Some(parent) = style.based_on.filter(|parent| *parent != index) {
                let _ = write!(xml, r#"<w:basedOn w:val="{}"/>"#, self.style_ids[parent]);
            }
            let next = style.next.unwrap_or(index);
            let _ = write!(
                xml,
                r#"<w:next w:val="{}"/><w:qFormat/>"#,
                self.style_ids[next]
            );
            xml.push_str("<w:pPr>");
            let numbering = style
                .list
                .map(|(list, level)| (document.list_instances.len() + list + 1, level));
            let mut props = style.paragraph.clone();
            let tabs = props.tabs.take();
            paragraph_properties_with_outline(
                &mut xml,
                &props,
                numbering,
                None,
                tabs.as_deref(),
                style.outline_level,
            );
            xml.push_str("</w:pPr>");
            let _ = write!(
                xml,
                "<w:rPr>{}</w:rPr></w:style>",
                char_properties(&style.character)
            );
        }
        xml.push_str(
            r#"<w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/><w:tblPr><w:tblInd w:w="0" w:type="dxa"/><w:tblCellMar><w:top w:w="0" w:type="dxa"/><w:left w:w="0" w:type="dxa"/><w:bottom w:w="0" w:type="dxa"/><w:right w:w="0" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style></w:styles>"#,
        );
        xml
    }

    fn numbering(&self) -> String {
        let document = self.document;
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering xmlns:w="{W}">"#
        );
        for (index, list) in document.lists.iter().enumerate() {
            let _ = write!(
                xml,
                r#"<w:abstractNum w:abstractNumId="{index}"><w:multiLevelType w:val="{}"/>"#,
                if list.levels.len() > 1 {
                    "multilevel"
                } else {
                    "singleLevel"
                }
            );
            for (level_index, level) in list.levels.iter().enumerate() {
                let _ = write!(
                    xml,
                    r#"<w:lvl w:ilvl="{level_index}"><w:start w:val="{}"/><w:numFmt w:val="{}"/>"#,
                    level.start,
                    number_format(level.format)
                );
                if level_index > 0 && !level.restart_after_higher {
                    xml.push_str(r#"<w:lvlRestart w:val="0"/>"#);
                }
                let _ = write!(
                    xml,
                    r#"<w:suff w:val="tab"/><w:lvlText w:val="{}"/><w:lvlJc w:val="{}"/><w:pPr><w:ind w:left="{}" w:hanging="{}"/></w:pPr>"#,
                    escape(&level.text),
                    if level.right_align { "right" } else { "left" },
                    level.indent_left,
                    level.hanging.max(0),
                );
                let mut props = level.props.clone();
                if level.font.is_some() {
                    props.font.clone_from(&level.font);
                }
                let rpr = char_properties(&props);
                if !rpr.is_empty() {
                    let _ = write!(xml, "<w:rPr>{rpr}</w:rPr>");
                }
                xml.push_str("</w:lvl>");
            }
            xml.push_str("</w:abstractNum>");
        }
        // Each instance starts every level where it says; then one
        // instance per list for the styles.
        for (index, instance) in document.list_instances.iter().enumerate() {
            let _ = write!(
                xml,
                r#"<w:num w:numId="{}"><w:abstractNumId w:val="{}"/>"#,
                index + 1,
                instance.list
            );
            let levels = document.lists[instance.list].levels.len().min(9);
            for (level, start) in instance.starts.iter().enumerate().take(levels) {
                let start = start.unwrap_or(document.lists[instance.list].levels[level].start);
                let _ = write!(
                    xml,
                    r#"<w:lvlOverride w:ilvl="{level}"><w:startOverride w:val="{start}"/></w:lvlOverride>"#
                );
            }
            xml.push_str("</w:num>");
        }
        for list in 0..document.lists.len() {
            let _ = write!(
                xml,
                r#"<w:num w:numId="{}"><w:abstractNumId w:val="{list}"/></w:num>"#,
                document.list_instances.len() + list + 1
            );
        }
        xml.push_str("</w:numbering>");
        xml
    }

    fn settings(&self) -> String {
        let settings = &self.document.settings;
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="{W}">"#
        );
        if settings.mirror_margins {
            xml.push_str("<w:mirrorMargins/>");
        }
        xml.push_str(r#"<w:defaultTabStop w:val="720"/>"#);
        if settings.hyphenation {
            xml.push_str("<w:autoHyphenation/>");
        }
        if settings.hyphenation_zone > 0 {
            let _ = write!(
                xml,
                r#"<w:hyphenationZone w:val="{}"/>"#,
                settings.hyphenation_zone
            );
        }
        if settings.even_and_odd_headers {
            xml.push_str("<w:evenAndOddHeaders/>");
        }
        xml.push_str(r#"<w:characterSpacingControl w:val="doNotCompress"/>"#);
        // Footnote Options: the starting number, numbering on each page.
        let start = settings.footnote_start.max(1);
        for (kind, present) in [
            ("footnote", !self.footnotes.is_empty()),
            ("endnote", !self.endnotes.is_empty()),
        ] {
            if !present {
                continue;
            }
            let _ = write!(xml, "<w:{kind}Pr>");
            if start != 1 {
                let _ = write!(xml, r#"<w:numStart w:val="{start}"/>"#);
            }
            if settings.footnotes_each_page && kind == "footnote" {
                xml.push_str(r#"<w:numRestart w:val="eachPage"/>"#);
            }
            let _ = write!(
                xml,
                r#"<w:{kind} w:id="0"/><w:{kind} w:id="1"/></w:{kind}Pr>"#
            );
        }
        xml.push_str(r#"<w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#);
        xml
    }

    fn fonts(&self) -> String {
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:fonts xmlns:w="{W}">"#
        );
        for font in &self.document.fonts {
            let family = match font.family {
                FontFamily::Roman => "roman",
                FontFamily::Swiss => "swiss",
                FontFamily::Modern => "modern",
                FontFamily::Script => "script",
                FontFamily::Decorative => "decorative",
                FontFamily::Unknown => "auto",
            };
            let _ = write!(
                xml,
                r#"<w:font w:name="{}"><w:family w:val="{family}"/><w:pitch w:val="{}"/></w:font>"#,
                escape(&font.name),
                if font.fixed { "fixed" } else { "variable" }
            );
        }
        xml.push_str("</w:fonts>");
        xml
    }

    fn core(&self) -> String {
        let properties = &self.document.properties;
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
        );
        if !properties.title.is_empty() {
            let _ = write!(xml, "<dc:title>{}</dc:title>", escape(&properties.title));
        }
        if !properties.description.is_empty() {
            let _ = write!(
                xml,
                "<dc:description>{}</dc:description>",
                escape(&properties.description)
            );
        }
        if !properties.keywords.is_empty() {
            let _ = write!(
                xml,
                "<cp:keywords>{}</cp:keywords>",
                escape(&properties.keywords)
            );
        }
        if properties.revisions > 0 {
            let _ = write!(xml, "<cp:revision>{}</cp:revision>", properties.revisions);
        }
        for (tag, time) in [
            ("dcterms:created", properties.created),
            ("dcterms:modified", properties.modified),
        ] {
            if let Some(time) = time {
                let _ = write!(
                    xml,
                    r#"<{tag} xsi:type="dcterms:W3CDTF">{}</{tag}>"#,
                    w3c_time(&time)
                );
            }
        }
        xml.push_str("</cp:coreProperties>");
        xml
    }

    fn app(&self) -> String {
        let properties = &self.document.properties;
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">"#,
        );
        if !properties.template.is_empty() {
            let _ = write!(xml, "<Template>{}</Template>", escape(&properties.template));
        }
        let _ = write!(
            xml,
            "<TotalTime>{}</TotalTime><Pages>{}</Pages><Words>{}</Words><Characters>{}</Characters><Application>AmiPro-Linux</Application></Properties>",
            properties.editing_minutes, properties.pages, properties.words, properties.characters
        );
        xml
    }

    fn custom(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">"#,
        );
        for (index, (name, value)) in self.document.properties.custom.iter().enumerate() {
            let _ = write!(
                xml,
                r#"<property fmtid="{{D5CDD505-2E9C-101B-9397-08002B2CF9AE}}" pid="{}" name="{}"><vt:lpwstr>{}</vt:lpwstr></property>"#,
                index + 2,
                escape(name),
                escape(value)
            );
        }
        xml.push_str("</Properties>");
        xml
    }
}

/// Style ids: each name's letters and digits, made unique.
fn style_ids(document: &ExchangeDocument) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for style in &document.styles {
        let mut id: String = style
            .name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        if id.is_empty() || id.starts_with(|first: char| first.is_ascii_digit()) {
            id = format!("Style{id}");
        }
        let base = id.clone();
        let mut number = 2;
        while ids
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&id))
        {
            id = format!("{base}{number}");
            number += 1;
        }
        ids.push(id);
    }
    ids
}

fn rels_xml(rels: &[(String, String, String)]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (id, kind, target) in rels {
        let _ = write!(
            xml,
            r#"<Relationship Id="{id}" Type="{kind}" Target="{}"/>"#,
            escape(target)
        );
    }
    xml.push_str("</Relationships>");
    xml
}

/// A paragraph's `w:pPr` children after its style: the formatting it
/// sets, its list, its tabs (the style's cleared where the paragraph's
/// replace them).
fn paragraph_properties(
    xml: &mut String,
    props: &ParagraphProps,
    numbering: Option<(usize, u8)>,
    base_tabs: Option<&[Tab]>,
) {
    paragraph_properties_with_outline(
        xml,
        props,
        numbering,
        base_tabs,
        props.tabs.as_deref(),
        None,
    );
}

fn paragraph_properties_with_outline(
    xml: &mut String,
    props: &ParagraphProps,
    numbering: Option<(usize, u8)>,
    base_tabs: Option<&[Tab]>,
    tabs: Option<&[Tab]>,
    outline: Option<u8>,
) {
    let flag = |xml: &mut String, value: Option<bool>, name: &str| match value {
        Some(true) => {
            let _ = write!(xml, "<w:{name}/>");
        }
        Some(false) => {
            let _ = write!(xml, r#"<w:{name} w:val="0"/>"#);
        }
        None => {}
    };
    flag(xml, props.keep_with_next, "keepNext");
    flag(xml, props.keep_together, "keepLines");
    flag(xml, props.page_break_before, "pageBreakBefore");
    if let Some((id, level)) = numbering {
        let _ = write!(
            xml,
            r#"<w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{id}"/></w:numPr>"#
        );
    }
    if props.border_top.is_some() || props.border_bottom.is_some() {
        let side = |line: &Option<Option<RuleLine>>, name: &str| match line {
            Some(Some(line)) => format!(
                r#"<w:{name} w:val="{}" w:sz="{}" w:space="{}" w:color="{}"/>"#,
                border_style(line.kind),
                eighths(line.width),
                (line.space / 20).clamp(0, 31),
                hex(line.colour)
            ),
            Some(None) => format!(r#"<w:{name} w:val="nil"/>"#),
            None => String::new(),
        };
        let top = side(&props.border_top, "top");
        let bottom = side(&props.border_bottom, "bottom");
        if !(top.is_empty() && bottom.is_empty()) {
            let _ = write!(xml, "<w:pBdr>{top}{bottom}</w:pBdr>");
        }
    }
    if let Some(tabs) = tabs {
        let mut stops = String::new();
        for cleared in base_tabs.unwrap_or_default() {
            if !tabs.iter().any(|tab| tab.position == cleared.position) {
                let _ = write!(
                    stops,
                    r#"<w:tab w:val="clear" w:pos="{}"/>"#,
                    cleared.position
                );
            }
        }
        for tab in tabs {
            let kind = match tab.kind {
                TabKind::Left => "left",
                TabKind::Centre => "center",
                TabKind::Right => "right",
                TabKind::Decimal => "decimal",
            };
            let leader = match tab.leader {
                TabLeader::None => "",
                TabLeader::Dots => r#" w:leader="dot""#,
                TabLeader::Hyphens => r#" w:leader="hyphen""#,
                TabLeader::Line => r#" w:leader="underscore""#,
            };
            let _ = write!(
                stops,
                r#"<w:tab w:val="{kind}"{leader} w:pos="{}"/>"#,
                tab.position
            );
        }
        if !stops.is_empty() {
            let _ = write!(xml, "<w:tabs>{stops}</w:tabs>");
        }
    }
    flag(xml, props.no_hyphenation, "suppressAutoHyphens");
    if props.space_before.is_some() || props.space_after.is_some() || props.line.is_some() {
        xml.push_str("<w:spacing");
        if let Some(before) = props.space_before {
            let _ = write!(xml, r#" w:before="{before}""#);
        }
        if let Some(after) = props.space_after {
            let _ = write!(xml, r#" w:after="{after}""#);
        }
        match props.line {
            Some(LineSpacing::Single) => xml.push_str(r#" w:line="240" w:lineRule="auto""#),
            Some(LineSpacing::Multiple(multiple)) => {
                let _ = write!(
                    xml,
                    r#" w:line="{}" w:lineRule="auto""#,
                    (240.0 * multiple).round() as i32
                );
            }
            Some(LineSpacing::Exact(height)) => {
                let _ = write!(xml, r#" w:line="{height}" w:lineRule="exact""#);
            }
            Some(LineSpacing::AtLeast(height)) => {
                let _ = write!(xml, r#" w:line="{height}" w:lineRule="atLeast""#);
            }
            None => {}
        }
        xml.push_str("/>");
    }
    if props.indent_left.is_some() || props.indent_right.is_some() || props.first_line.is_some() {
        xml.push_str("<w:ind");
        if let Some(left) = props.indent_left {
            let _ = write!(xml, r#" w:left="{left}""#);
        }
        if let Some(right) = props.indent_right {
            let _ = write!(xml, r#" w:right="{right}""#);
        }
        match props.first_line {
            Some(first) if first < 0 => {
                let _ = write!(xml, r#" w:hanging="{}""#, -first);
            }
            Some(first) => {
                let _ = write!(xml, r#" w:firstLine="{first}""#);
            }
            None => {}
        }
        xml.push_str("/>");
    }
    if let Some(alignment) = props.alignment {
        let value = match alignment {
            Alignment::Left => "left",
            Alignment::Right => "right",
            Alignment::Centre => "center",
            Alignment::Justify => "both",
        };
        let _ = write!(xml, r#"<w:jc w:val="{value}"/>"#);
    }
    if let Some(level) = outline {
        let _ = write!(
            xml,
            r#"<w:outlineLvl w:val="{}"/>"#,
            level.saturating_sub(1)
        );
    }
}

/// The `w:rPr` children for `props`, in the schema's order.
fn char_properties(props: &CharProps) -> String {
    let mut xml = String::new();
    if let Some(font) = &props.font {
        let font = escape(font);
        let _ = write!(
            xml,
            r#"<w:rFonts w:ascii="{font}" w:hAnsi="{font}" w:cs="{font}" w:eastAsia="{font}"/>"#
        );
    }
    let flag = |xml: &mut String, value: Option<bool>, name: &str| match value {
        Some(true) => {
            let _ = write!(xml, "<w:{name}/>");
        }
        Some(false) => {
            let _ = write!(xml, r#"<w:{name} w:val="0"/>"#);
        }
        None => {}
    };
    flag(&mut xml, props.bold, "b");
    flag(&mut xml, props.bold, "bCs");
    flag(&mut xml, props.italic, "i");
    flag(&mut xml, props.italic, "iCs");
    flag(&mut xml, props.caps, "caps");
    flag(&mut xml, props.small_caps, "smallCaps");
    flag(&mut xml, props.strike, "strike");
    if let Some(colour) = props.colour {
        let _ = write!(xml, r#"<w:color w:val="{}"/>"#, hex(colour));
    }
    if let Some(size) = props.size {
        // Half-points.
        let half = (size + 5) / 10;
        let _ = write!(xml, r#"<w:sz w:val="{half}"/><w:szCs w:val="{half}"/>"#);
    }
    if let Some(underline) = props.underline {
        let value = match underline {
            Underline::None => "none",
            Underline::Single => "single",
            Underline::Words => "words",
            Underline::Double => "double",
        };
        let _ = write!(xml, r#"<w:u w:val="{value}"/>"#);
    }
    if let Some(script) = props.script {
        let value = match script {
            Script::Normal => "baseline",
            Script::Super => "superscript",
            Script::Sub => "subscript",
        };
        let _ = write!(xml, r#"<w:vertAlign w:val="{value}"/>"#);
    }
    xml
}

fn run_properties(props: &CharProps) -> String {
    let inner = char_properties(props);
    if inner.is_empty() {
        String::new()
    } else {
        format!("<w:rPr>{inner}</w:rPr>")
    }
}

/// A run of text; soft hyphens are Word's own.
fn run(xml: &mut String, props: &CharProps, text: &str) {
    if text.is_empty() {
        return;
    }
    let rpr = run_properties(props);
    let _ = write!(xml, "<w:r>{rpr}");
    for (index, piece) in text.split('\u{AD}').enumerate() {
        if index > 0 {
            xml.push_str("<w:softHyphen/>");
        }
        if !piece.is_empty() {
            let _ = write!(xml, r#"<w:t xml:space="preserve">{}</w:t>"#, escape(piece));
        }
    }
    xml.push_str("</w:r>");
}

/// The frame's fill, line, corners, and shadow as DrawingML.
fn shape_outline(shape: &Shape) -> String {
    let mut xml = String::new();
    if shape.corner_radius > 0 {
        let shorter = shape.width.min(shape.height).max(1);
        let adjust = (i64::from(shape.corner_radius) * 100_000 / i64::from(shorter)).min(50_000);
        let _ = write!(
            xml,
            r#"<a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val {adjust}"/></a:avLst></a:prstGeom>"#
        );
    } else {
        xml.push_str(r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>"#);
    }
    match shape.fill {
        Some(colour) => {
            let _ = write!(
                xml,
                r#"<a:solidFill><a:srgbClr val="{}"/></a:solidFill>"#,
                hex(colour)
            );
        }
        None => xml.push_str("<a:noFill/>"),
    }
    match shape.line {
        Some(line) => {
            let compound = match line.kind {
                LineKind::Single => "sng",
                LineKind::Double => "dbl",
                LineKind::Triple => "tri",
                LineKind::ThickThin => "thickThin",
                LineKind::ThinThick => "thinThick",
            };
            let _ = write!(
                xml,
                r#"<a:ln w="{}" cmpd="{compound}"><a:solidFill><a:srgbClr val="{}"/></a:solidFill><a:miter lim="800000"/></a:ln>"#,
                emu(line.width),
                hex(line.colour)
            );
        }
        None => xml.push_str("<a:ln><a:noFill/></a:ln>"),
    }
    if let Some(shadow) = shape.shadow {
        let (dx, dy) = (f64::from(shadow.dx), f64::from(shadow.dy));
        let distance = (dx.hypot(dy) * EMU as f64).round() as i64;
        let mut degrees = dy.atan2(dx).to_degrees();
        if degrees < 0.0 {
            degrees += 360.0;
        }
        let _ = write!(
            xml,
            r#"<a:effectLst><a:outerShdw dist="{distance}" dir="{}" algn="tl" rotWithShape="0"><a:srgbClr val="{}"/></a:outerShdw></a:effectLst>"#,
            (degrees * 60_000.0).round() as i64,
            hex(shadow.colour)
        );
    }
    xml
}

/// The part of the frame (`width` × `height`) the picture covers: left,
/// top, right, bottom from the frame's corner, or `None` when it shows
/// nothing.
fn visible_part(
    picture: &PlacedPicture,
    width: Twips,
    height: Twips,
) -> Option<(Twips, Twips, Twips, Twips)> {
    let left = picture.left.max(0);
    let top = picture.top.max(0);
    let right = (picture.left + picture.width).min(width);
    let bottom = (picture.top + picture.height).min(height);
    (left < right && top < bottom).then_some((left, top, right, bottom))
}

/// What the frame cuts off the picture, as a source rectangle in the
/// picture's own hundred-thousandths.
fn crop_rect(
    picture: &PlacedPicture,
    left: Twips,
    top: Twips,
    right: Twips,
    bottom: Twips,
) -> String {
    let fraction = |cut: Twips, size: Twips| -> i64 {
        (i64::from(cut.max(0)) * 100_000 / i64::from(size.max(1))).min(100_000)
    };
    let l = fraction(left - picture.left, picture.width);
    let t = fraction(top - picture.top, picture.height);
    let r = fraction(picture.left + picture.width - right, picture.width);
    let b = fraction(picture.top + picture.height - bottom, picture.height);
    if l == 0 && t == 0 && r == 0 && b == 0 {
        return String::new();
    }
    format!(r#"<a:srcRect l="{l}" t="{t}" r="{r}" b="{b}"/>"#)
}

fn emu(twips: Twips) -> i64 {
    i64::from(twips) * EMU
}

/// A line's width in eighths of a point, as Word's borders take it.
fn eighths(width: Twips) -> i32 {
    (width * 8 / 20).clamp(2, 96)
}

fn border_style(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Single => "single",
        LineKind::Double => "double",
        LineKind::Triple => "triple",
        LineKind::ThickThin => "thickThinSmallGap",
        LineKind::ThinThick => "thinThickSmallGap",
    }
}

fn number_format(format: NumberFormat) -> &'static str {
    match format {
        NumberFormat::Decimal => "decimal",
        NumberFormat::UpperRoman => "upperRoman",
        NumberFormat::LowerRoman => "lowerRoman",
        NumberFormat::UpperLetter => "upperLetter",
        NumberFormat::LowerLetter => "lowerLetter",
        NumberFormat::Bullet => "bullet",
        NumberFormat::None => "none",
    }
}

/// A wall-clock time as W3C date and time; the clock is the document's,
/// written as UTC (EC2).
fn w3c_time(time: &amipro_document::WallClock) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        time.year, time.month, time.day, time.hour, time.minute, time.second
    )
}
