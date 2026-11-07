//! The OpenDocument Text filter (ISO/IEC 26300, ODF 1.3; EX1): the exchange
//! document as a zip of ODF parts. Each section is a master page with its
//! page layout, headers, and footers; frames are `draw:frame`s anchored
//! where Ami Pro places them (EX3); lists are `text:list`s continuing their
//! instance, fields ODF's own (EX8); Doc Info is `meta.xml` (EX10).

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::xml::escape;
use crate::zip::{self, Deflate, Entry};
use crate::{
    Alignment, Anchor, Block, CharProps, DateSource, ExchangeDocument, Field, FieldKind,
    FontFamily, HorizontalFrom, Inline, LineKind, LineSpacing, NumberFormat, Paragraph,
    ParagraphProps, RowHeight, Script, Section, Shape, ShapeContent, ShapeWrap, Tab, TabKind,
    TabLeader, Table, Twips, Underline, VerticalFrom, VerticalMerge, hex,
};

const NAMESPACES: &str = r#"xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" xmlns:number="urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0" office:version="1.3""#;

/// The document as an ODT package; `deflate` compresses its parts.
pub fn write(document: &ExchangeDocument, deflate: Deflate) -> Vec<u8> {
    zip::write(&parts(document), deflate)
}

/// The package's parts: `mimetype` first and stored, as ODF requires.
pub fn parts(document: &ExchangeDocument) -> Vec<Entry> {
    let mut writer = Writer::new(document);
    let content = writer.content();
    let styles = writer.styles();
    let mut entries = vec![
        Entry::stored("mimetype", "application/vnd.oasis.opendocument.text"),
        Entry::new("META-INF/manifest.xml", writer.manifest()),
        Entry::new("content.xml", content),
        Entry::new("styles.xml", styles),
        Entry::new("meta.xml", writer.meta()),
    ];
    for (index, image) in document.images.iter().enumerate() {
        entries.push(Entry::new(
            format!("Pictures/image{}.{}", index + 1, image.kind.extension()),
            image.bytes.to_vec(),
        ));
        if let Some(fallback) = &image.fallback {
            entries.push(Entry::new(
                format!("Pictures/image{}.png", index + 1),
                fallback.to_vec(),
            ));
        }
    }
    entries
}

struct Writer<'a> {
    document: &'a ExchangeDocument,
    style_names: Vec<String>,
    /// Automatic styles by family and properties: their names.
    automatic: HashMap<(char, String), String>,
    /// The automatic styles' XML, in the order they were made.
    automatic_xml: String,
    /// Each list instance's last `text:list` id, which the next continues.
    list_ids: HashMap<usize, String>,
    next_list: usize,
    next_frame: usize,
    next_table: usize,
    /// Master pages: (name, page layout name) for each section.
    masters: Vec<(String, String)>,
    /// The page layouts' and master pages' XML, for `styles.xml`.
    page_layouts: String,
    master_xml: String,
    /// Sequences used by Seq fields.
    sequences: Vec<String>,
    /// Writing master pages: their automatic styles go to `styles.xml`.
    in_master: bool,
    master_automatic: HashMap<(char, String), String>,
    master_automatic_xml: String,
    /// The footnotes and endnotes written so far.
    notes_written: usize,
}

impl<'a> Writer<'a> {
    fn new(document: &'a ExchangeDocument) -> Self {
        Self {
            document,
            style_names: style_names(document),
            automatic: HashMap::new(),
            automatic_xml: String::new(),
            list_ids: HashMap::new(),
            next_list: 1,
            next_frame: 1,
            next_table: 1,
            masters: Vec::new(),
            page_layouts: String::new(),
            master_xml: String::new(),
            sequences: Vec::new(),
            in_master: false,
            master_automatic: HashMap::new(),
            master_automatic_xml: String::new(),
            notes_written: 0,
        }
    }

    /// An automatic style of `family` (P paragraph, T text, F graphic, and
    /// table parts) with `properties`, made once.
    fn automatic(&mut self, family: char, kind: &str, properties: String) -> String {
        let key = (family, format!("{kind}|{properties}"));
        let (map, xml, prefix) = if self.in_master {
            (
                &mut self.master_automatic,
                &mut self.master_automatic_xml,
                "M",
            )
        } else {
            (&mut self.automatic, &mut self.automatic_xml, "")
        };
        if let Some(name) = map.get(&key) {
            return name.clone();
        }
        let name = format!("{prefix}{family}{}", map.len() + 1);
        let _ = write!(
            xml,
            r#"<style:style style:name="{name}" style:family="{kind}"{properties}</style:style>"#
        );
        map.insert(key, name.clone());
        name
    }

    /// The data style showing dates as Word's `picture` (`MMMM d, yyyy`),
    /// made once in the part being written.
    fn date_style(&mut self, picture: &str) -> String {
        let key = ('N', picture.to_owned());
        let (map, xml, prefix) = if self.in_master {
            (
                &mut self.master_automatic,
                &mut self.master_automatic_xml,
                "M",
            )
        } else {
            (&mut self.automatic, &mut self.automatic_xml, "")
        };
        if let Some(name) = map.get(&key) {
            return name.clone();
        }
        let name = format!("{prefix}N{}", map.len() + 1);
        let _ = write!(
            xml,
            r#"<number:date-style style:name="{name}">{}</number:date-style>"#,
            date_parts(picture)
        );
        map.insert(key, name.clone());
        name
    }

    // ---- content.xml ----

    fn content(&mut self) -> String {
        let mut body = String::new();
        let sections = &self.document.sections;
        for (index, section) in sections.iter().enumerate() {
            let master = self.master_page(section, index);
            let mut first = Some(master);
            for block in &section.blocks {
                self.block(block, &mut first, section, &mut body);
            }
            if first.is_some() {
                // A section without paragraphs still begins its pages.
                let style = self.paragraph_style(
                    &Paragraph::default(),
                    &ParagraphProps::default(),
                    first.take(),
                    None,
                    section,
                );
                let _ = write!(body, r#"<text:p text:style-name="{style}"/>"#);
            }
        }
        let mut sequences = String::new();
        if !self.sequences.is_empty() {
            sequences.push_str("<text:sequence-decls>");
            for name in &self.sequences {
                let _ = write!(
                    sequences,
                    r#"<text:sequence-decl text:display-outline-level="0" text:name="{}"/>"#,
                    escape(name)
                );
            }
            sequences.push_str("</text:sequence-decls>");
        }
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><office:document-content {NAMESPACES}>{}<office:automatic-styles>{}</office:automatic-styles><office:body><office:text>{sequences}{body}</office:text></office:body></office:document-content>"#,
            self.font_faces(),
            self.automatic_xml
        )
    }

    fn block(
        &mut self,
        block: &Block,
        master: &mut Option<String>,
        section: &Section,
        out: &mut String,
    ) {
        match block {
            Block::Paragraph(paragraph) => self.paragraph(paragraph, master, section, out),
            // A table starting a section starts its master page.
            Block::Table(table) => self.table(table, master.take(), section, out),
        }
    }

    fn blocks(&mut self, blocks: &[Block], section: &Section, out: &mut String) {
        let mut none = None;
        for block in blocks {
            self.block(block, &mut none, section, out);
        }
        if blocks.is_empty() || matches!(blocks.last(), Some(Block::Table(_))) {
            out.push_str("<text:p/>");
        }
    }

    /// The automatic style of a paragraph: its style, what it sets over it,
    /// and the master page it starts.
    fn paragraph_style(
        &mut self,
        paragraph: &Paragraph,
        props: &ParagraphProps,
        master: Option<String>,
        after: Option<&str>,
        section: &Section,
    ) -> String {
        let parent = self
            .style_names
            .get(paragraph.style)
            .cloned()
            .unwrap_or_else(|| "Standard".into());
        let base_tabs = self
            .document
            .styles
            .get(paragraph.style)
            .and_then(|style| style.paragraph.tabs.clone());
        let mut properties = paragraph_properties(props, base_tabs.as_deref());
        if let Some(kind) = after {
            let _ = write!(properties.attributes, r#" fo:break-after="{kind}""#);
        }
        // The mark's look only where the paragraph has no text: a
        // paragraph style's text properties are all its text's.
        let empty = paragraph.inlines.iter().all(|inline| {
            !matches!(
                inline,
                Inline::Text { .. } | Inline::Tab { .. } | Inline::Field(_) | Inline::Symbol { .. }
            )
        });
        let mut text = if empty {
            text_properties(&paragraph.mark)
        } else {
            String::new()
        };
        text.push_str(hyphenate(props.no_hyphenation));
        if properties.is_empty() && text.is_empty() && master.is_none() {
            return parent;
        }
        let mut attributes = format!(r#" style:parent-style-name="{parent}""#);
        if let Some(master) = &master {
            let _ = write!(attributes, r#" style:master-page-name="{master}""#);
            if let Some(start) = section.page_numbers.and_then(|numbers| numbers.start) {
                properties
                    .attributes
                    .push_str(&format!(r#" style:page-number="{start}""#));
            }
        }
        let mut xml = format!("{attributes}>");
        xml.push_str(&properties.element());
        if !text.is_empty() {
            let _ = write!(xml, "<style:text-properties{text}/>");
        }
        self.automatic('P', "paragraph", xml)
    }

    fn paragraph(
        &mut self,
        paragraph: &Paragraph,
        master: &mut Option<String>,
        section: &Section,
        out: &mut String,
    ) {
        // A break inside the paragraph splits it: ODF breaks only before
        // or after a paragraph.
        let mut pieces: Vec<(Option<&str>, Vec<&Inline>)> = vec![(None, Vec::new())];
        for inline in &paragraph.inlines {
            match inline {
                Inline::PageBreak => pieces.push((Some("page"), Vec::new())),
                Inline::ColumnBreak => pieces.push((Some("column"), Vec::new())),
                _ => pieces.last_mut().expect("one piece").1.push(inline),
            }
        }
        // A break that ends the paragraph: the paragraph breaks after it.
        let mut after = None;
        if pieces.len() > 1 && pieces.last().is_some_and(|(_, inlines)| inlines.is_empty()) {
            after = pieces.pop().and_then(|(kind, _)| kind);
        }
        // A break that begins it: the paragraph breaks before.
        if pieces.len() > 1 && pieces[0].1.is_empty() {
            pieces.remove(0);
        }
        let last = pieces.len() - 1;
        let style_props = self
            .document
            .styles
            .get(paragraph.style)
            .map(|style| &style.paragraph);
        let (open, close) = self.list_wrapper(paragraph);
        for (index, (breaking, inlines)) in pieces.into_iter().enumerate() {
            // Each piece after a break: the paragraph's style breaking
            // before it (the first only where its style does not).
            let mut props = std::borrow::Cow::Borrowed(&paragraph.props);
            let style_breaks = match breaking {
                Some("page") => style_props.and_then(|style| style.page_break_before),
                Some(_) => style_props.and_then(|style| style.column_break_before),
                None => None,
            } == Some(true);
            if breaking.is_some() && (index > 0 || !style_breaks) {
                let props = props.to_mut();
                props.page_break_before = Some(breaking == Some("page"));
                props.column_break_before = Some(breaking == Some("column"));
            }
            let style = self.paragraph_style(
                paragraph,
                &props,
                if index == 0 { master.take() } else { None },
                if index == last { after } else { None },
                section,
            );
            let tag = if paragraph.style < self.document.styles.len()
                && self.document.styles[paragraph.style]
                    .outline_level
                    .is_some()
                && index == 0
                && paragraph.list.is_none()
            {
                let level = self.document.styles[paragraph.style]
                    .outline_level
                    .unwrap_or(1);
                (
                    format!(r#"<text:h text:style-name="{style}" text:outline-level="{level}">"#),
                    "</text:h>",
                )
            } else {
                (
                    format!(r#"<text:p text:style-name="{style}">"#),
                    "</text:p>",
                )
            };
            if index == 0 {
                out.push_str(&open);
            }
            out.push_str(&tag.0);
            if index == 0
                && matches!(paragraph.list, Some((None, _)))
                && let Some(number) = &paragraph.number_text
            {
                text_run(out, &format!("{number}\t"));
            }
            for inline in inlines {
                self.inline(inline, section, out);
            }
            out.push_str(tag.1);
            if index == 0 {
                out.push_str(&close);
            }
        }
    }

    /// The `text:list` elements around a list paragraph, continuing its
    /// instance's list.
    fn list_wrapper(&mut self, paragraph: &Paragraph) -> (String, String) {
        let Some((Some(instance), level)) = paragraph.list else {
            return (String::new(), String::new());
        };
        let Some(definition) = self.document.list_instances.get(instance) else {
            return (String::new(), String::new());
        };
        let id = format!("list{}", self.next_list);
        self.next_list += 1;
        let continues = self.list_ids.insert(instance, id.clone());
        let mut open = format!(
            r#"<text:list xml:id="{id}" text:style-name="L{}""#,
            definition.list + 1
        );
        let first = continues.is_none();
        match continues {
            Some(previous) => {
                let _ = write!(open, r#" text:continue-list="{previous}">"#);
            }
            None => open.push('>'),
        }
        let mut close = String::new();
        for depth in 0..=level {
            if depth == level {
                match (first, definition.starts[usize::from(level)]) {
                    (true, Some(start)) => {
                        let _ = write!(open, r#"<text:list-item text:start-value="{start}">"#);
                    }
                    _ => open.push_str("<text:list-item>"),
                }
            } else {
                open.push_str("<text:list-item><text:list>");
            }
        }
        for depth in 0..=level {
            close.push_str("</text:list-item>");
            if depth < level {
                close.push_str("</text:list>");
            }
        }
        close.push_str("</text:list>");
        (open, close)
    }

    fn inline(&mut self, inline: &Inline, section: &Section, out: &mut String) {
        match inline {
            Inline::Text { text, props } => self.span(props, out, |out| text_run(out, text)),
            Inline::Tab { props } => self.span(props, out, |out| out.push_str("<text:tab/>")),
            Inline::LineBreak => out.push_str("<text:line-break/>"),
            Inline::PageBreak | Inline::ColumnBreak => {}
            Inline::Field(field) => {
                let xml = self.field(field);
                self.span(&field.props, out, |out| out.push_str(&xml));
            }
            Inline::Symbol {
                character,
                font,
                props,
            } => {
                let mut props = props.clone();
                if font.is_some() {
                    props.font.clone_from(font);
                }
                self.span(&props, out, |out| text_run(out, &character.to_string()));
            }
            Inline::Shape(shape) => self.frame(shape, section, out),
            // Footnotes and endnotes as ODF notes, Ami Pro's notes as
            // annotations, bookmarks as ODF's (the notes run).
            Inline::Footnote(footnote) => {
                self.notes_written += 1;
                let id = self.notes_written;
                let class = if footnote.endnote {
                    "endnote"
                } else {
                    "footnote"
                };
                let mut body = String::new();
                self.blocks(&footnote.blocks, section, &mut body);
                let _ = write!(
                    out,
                    r#"<text:note text:id="ftn{id}" text:note-class="{class}"><text:note-citation>{id}</text:note-citation><text:note-body>{body}</text:note-body></text:note>"#
                );
            }
            Inline::Note(note) => {
                let mut body = String::new();
                self.blocks(&note.blocks, section, &mut body);
                let date = note
                    .date
                    .map(|date| {
                        format!(
                            "<dc:date>{:04}-{:02}-{:02}T{:02}:{:02}:{:02}</dc:date>",
                            date.year, date.month, date.day, date.hour, date.minute, date.second
                        )
                    })
                    .unwrap_or_default();
                let _ = write!(
                    out,
                    "<office:annotation><dc:creator>{}</dc:creator>{date}<meta:creator-initials>{}</meta:creator-initials>{body}</office:annotation>",
                    escape(&note.author),
                    escape(&note.initials)
                );
            }
            Inline::Bookmark { name, end } => {
                let _ = write!(
                    out,
                    r#"<text:bookmark-{} text:name="{}"/>"#,
                    if *end { "end" } else { "start" },
                    escape(name)
                );
            }
            Inline::Marker(_) => {}
        }
    }

    /// `content` in a span with `props`, or bare without formatting.
    fn span(&mut self, props: &CharProps, out: &mut String, content: impl FnOnce(&mut String)) {
        let text = text_properties(props);
        if text.is_empty() {
            content(out);
            return;
        }
        let name = self.automatic('T', "text", format!("><style:text-properties{text}/>"));
        let _ = write!(out, r#"<text:span text:style-name="{name}">"#);
        content(out);
        out.push_str("</text:span>");
    }

    fn field(&mut self, field: &Field) -> String {
        let result = escape(&field.result);
        match &field.kind {
            FieldKind::Page { format } => format!(
                r#"<text:page-number text:select-page="current" style:num-format="{}">{result}</text:page-number>"#,
                odf_number_format(*format)
            ),
            FieldKind::NumPages => format!("<text:page-count>{result}</text:page-count>"),
            FieldKind::Date {
                source, picture, ..
            } => {
                let style = self.date_style(picture);
                let element = match source {
                    DateSource::Now => "text:date",
                    DateSource::Created => "text:creation-date",
                    DateSource::Saved => "text:modification-date",
                };
                format!(r#"<{element} style:data-style-name="{style}">{result}</{element}>"#)
            }
            FieldKind::Time { source } => match source {
                DateSource::Now => format!("<text:time>{result}</text:time>"),
                DateSource::Created => format!("<text:creation-time>{result}</text:creation-time>"),
                DateSource::Saved => {
                    format!("<text:modification-time>{result}</text:modification-time>")
                }
            },
            FieldKind::Title => format!("<text:title>{result}</text:title>"),
            FieldKind::Description => format!("<text:description>{result}</text:description>"),
            FieldKind::Keywords => format!("<text:keywords>{result}</text:keywords>"),
            FieldKind::Custom(name) => format!(
                r#"<text:user-defined text:name="{}">{result}</text:user-defined>"#,
                escape(name)
            ),
            FieldKind::FileName { .. } => {
                format!(r#"<text:file-name text:display="name">{result}</text:file-name>"#)
            }
            FieldKind::Template => {
                format!(r#"<text:template-name text:display="name">{result}</text:template-name>"#)
            }
            FieldKind::Revisions => format!("<text:editing-cycles>{result}</text:editing-cycles>"),
            FieldKind::EditingTime => {
                format!("<text:editing-duration>{result}</text:editing-duration>")
            }
            FieldKind::Words => format!("<text:word-count>{result}</text:word-count>"),
            FieldKind::Characters => {
                format!("<text:character-count>{result}</text:character-count>")
            }
            FieldKind::Seq { name } => {
                let name = crate::fields::sequence_name(name);
                if !self.sequences.contains(&name) {
                    self.sequences.push(name.clone());
                }
                format!(
                    r#"<text:sequence text:name="{0}" text:formula="ooow:{0}+1" style:num-format="1">{result}</text:sequence>"#,
                    escape(&name)
                )
            }
            FieldKind::IndexEntry { primary, secondary } => {
                if secondary.is_empty() {
                    format!(
                        r#"<text:alphabetical-index-mark text:string-value="{}"/>"#,
                        escape(primary)
                    )
                } else {
                    format!(
                        r#"<text:alphabetical-index-mark text:string-value="{}" text:key1="{}"/>"#,
                        escape(secondary),
                        escape(primary)
                    )
                }
            }
            FieldKind::ContentsEntry { text, level } => format!(
                r#"<text:toc-mark text:string-value="{}" text:outline-level="{}"/>"#,
                escape(text),
                level.clamp(&1, &10)
            ),
        }
    }

    // ---- Frames ----

    fn frame(&mut self, shape: &Shape, section: &Section, out: &mut String) {
        let mut graphic = String::new();
        let anchor = match shape.anchor {
            Anchor::Inline { .. } => {
                graphic.push_str(r#" style:vertical-pos="top" style:vertical-rel="baseline""#);
                "as-char".to_owned()
            }
            Anchor::Floating {
                horizontal,
                vertical,
                wrap,
                behind,
            } => {
                let horizontal_rel = match horizontal.0 {
                    HorizontalFrom::Page => "page",
                    HorizontalFrom::Margin => "page-content",
                    HorizontalFrom::Column => "paragraph",
                };
                let vertical_rel = match vertical.0 {
                    VerticalFrom::Page => "page",
                    VerticalFrom::Margin => "page-content",
                    VerticalFrom::Paragraph => "paragraph",
                };
                let wrap = match wrap {
                    ShapeWrap::Square => r#" style:wrap="parallel""#.to_owned(),
                    ShapeWrap::TopAndBottom => r#" style:wrap="none""#.to_owned(),
                    ShapeWrap::None => format!(
                        r#" style:wrap="run-through" style:run-through="{}""#,
                        if behind { "background" } else { "foreground" }
                    ),
                };
                let _ = write!(
                    graphic,
                    r#"{wrap} style:vertical-pos="from-top" style:vertical-rel="{vertical_rel}" style:horizontal-pos="from-left" style:horizontal-rel="{horizontal_rel}""#
                );
                "paragraph".to_owned()
            }
        };
        match shape.line {
            Some(line) => {
                let border = border(line.kind, line.width, line.colour);
                let [top, right, bottom, left] = line.sides;
                for (on, side) in [
                    (top, "top"),
                    (right, "right"),
                    (bottom, "bottom"),
                    (left, "left"),
                ] {
                    let value = if on { border.as_str() } else { "none" };
                    let _ = write!(graphic, r#" fo:border-{side}="{value}""#);
                }
            }
            None => graphic.push_str(r#" fo:border="none""#),
        }
        match shape.fill {
            Some(colour) => {
                let _ = write!(
                    graphic,
                    r##" fo:background-color="#{}" draw:fill="solid" draw:fill-color="#{}""##,
                    hex(colour),
                    hex(colour)
                );
            }
            None => graphic.push_str(r#" fo:background-color="transparent" draw:fill="none""#),
        }
        if let Some(shadow) = shape.shadow {
            let _ = write!(
                graphic,
                r##" style:shadow="#{} {} {}""##,
                hex(shadow.colour),
                length(shadow.dx),
                length(shadow.dy)
            );
        }
        let (x, y) = match shape.anchor {
            Anchor::Floating {
                horizontal,
                vertical,
                ..
            } => (horizontal.1, vertical.1),
            Anchor::Inline { .. } => (0, 0),
        };
        let content = match &shape.content {
            ShapeContent::Text {
                blocks,
                insets,
                columns,
                column_gap,
            } => {
                let [top, right, bottom, left] = *insets;
                let _ = write!(
                    graphic,
                    r#" fo:padding-top="{}" fo:padding-right="{}" fo:padding-bottom="{}" fo:padding-left="{}" draw:textarea-vertical-align="top""#,
                    length(top),
                    length(right),
                    length(bottom),
                    length(left)
                );
                let columns = if *columns > 1 {
                    format!(
                        r#"<style:columns fo:column-count="{columns}" fo:column-gap="{}"/>"#,
                        length(*column_gap)
                    )
                } else {
                    String::new()
                };
                let mut inner = String::new();
                self.blocks(blocks, section, &mut inner);
                let corners = if shape.corner_radius > 0 {
                    format!(r#" draw:corner-radius="{}""#, length(shape.corner_radius))
                } else {
                    String::new()
                };
                (
                    format!("<draw:text-box{corners}>{inner}</draw:text-box>"),
                    columns,
                )
            }
            ShapeContent::Picture(picture) => {
                // The picture's place in the frame as padding where it
                // stands in, and a clip where the frame cuts it.
                let pad = |value: Twips| value.max(0);
                let _ = write!(
                    graphic,
                    r#" fo:padding-top="{}" fo:padding-left="{}" fo:padding-right="{}" fo:padding-bottom="{}""#,
                    length(pad(picture.top)),
                    length(pad(picture.left)),
                    length(pad(shape.width - picture.left - picture.width)),
                    length(pad(shape.height - picture.top - picture.height)),
                );
                let cut = |value: Twips| (-value).max(0);
                let (top, left) = (cut(picture.top), cut(picture.left));
                let right = cut(shape.width - picture.left - picture.width);
                let bottom = cut(shape.height - picture.top - picture.height);
                if top + left + right + bottom > 0 {
                    let _ = write!(
                        graphic,
                        r#" fo:clip="rect({}, {}, {}, {})""#,
                        length(top),
                        length(right),
                        length(bottom),
                        length(left)
                    );
                }
                let image = &self.document.images[picture.image];
                let mut href = format!(
                    r#"<draw:image xlink:href="Pictures/image{}.{}" xlink:type="simple" xlink:show="embed" xlink:actuate="onLoad""#,
                    picture.image + 1,
                    image.kind.extension()
                );
                if image.fallback.is_some() {
                    let _ = write!(
                        href,
                        r#"/><draw:image xlink:href="Pictures/image{}.png" xlink:type="simple" xlink:show="embed" xlink:actuate="onLoad""#,
                        picture.image + 1
                    );
                }
                href.push_str("/>");
                if !picture.description.is_empty() {
                    let _ = write!(
                        href,
                        "<svg:desc>{}</svg:desc>",
                        escape(&picture.description)
                    );
                }
                (href, String::new())
            }
            ShapeContent::Empty => (
                if shape.corner_radius > 0 {
                    format!(
                        r#"<draw:text-box draw:corner-radius="{}"><text:p/></draw:text-box>"#,
                        length(shape.corner_radius)
                    )
                } else {
                    r#"<draw:text-box><text:p/></draw:text-box>"#.to_owned()
                },
                String::new(),
            ),
        };
        let columns = content.1;
        let style = self.automatic(
            'F',
            "graphic",
            format!(r#" style:parent-style-name="Frame"><style:graphic-properties{graphic}>{columns}</style:graphic-properties>"#),
        );
        let name = format!("{} {}", escape(&shape.name), self.next_frame);
        self.next_frame += 1;
        let position = if anchor == "as-char" {
            String::new()
        } else {
            format!(r#" svg:x="{}" svg:y="{}""#, length(x), length(y))
        };
        let _ = write!(
            out,
            r#"<draw:frame draw:style-name="{style}" draw:name="{name}" text:anchor-type="{anchor}"{position} svg:width="{}" svg:height="{}" draw:z-index="{}">{}</draw:frame>"#,
            length(shape.width),
            length(shape.height),
            shape.z,
            content.0
        );
    }

    // ---- Tables ----

    fn table(
        &mut self,
        table: &Table,
        master: Option<String>,
        section: &Section,
        out: &mut String,
    ) {
        let number = self.next_table;
        self.next_table += 1;
        let width: Twips = table.columns.iter().sum();
        let mut starts = String::new();
        let mut page_number = String::new();
        if let Some(master) = &master {
            let _ = write!(starts, r#" style:master-page-name="{master}""#);
            if let Some(start) = section.page_numbers.and_then(|numbers| numbers.start) {
                let _ = write!(page_number, r#" style:page-number="{start}""#);
            }
        }
        let table_style = self.automatic(
            'A',
            "table",
            format!(
                r#"{starts}><style:table-properties style:width="{}" table:align="{}" fo:margin-left="{}"{page_number}{}/>"#,
                length(width),
                match table.align {
                    Alignment::Centre => "center",
                    Alignment::Right => "right",
                    _ => "left",
                },
                length(table.indent),
                if table.rows.first().is_some_and(|row| row.page_break_before) {
                    r#" fo:break-before="page""#
                } else {
                    ""
                }
            ),
        );
        let _ = write!(
            out,
            r#"<table:table table:name="Table{number}" table:style-name="{table_style}">"#
        );
        for column in &table.columns {
            let style = self.automatic(
                'C',
                "table-column",
                format!(
                    r#"><style:table-column-properties style:column-width="{}"/>"#,
                    length(*column)
                ),
            );
            let _ = write!(out, r#"<table:table-column table:style-name="{style}"/>"#);
        }
        // How many rows each joined block covers.
        let mut grid: Vec<Vec<(usize, VerticalMerge)>> = Vec::new();
        for row in &table.rows {
            let mut column = 0;
            let mut cells = Vec::new();
            for cell in &row.cells {
                cells.push((column, cell.merge));
                column += usize::from(cell.span.max(1));
            }
            grid.push(cells);
        }
        let spanned = |row: usize, column: usize| -> usize {
            let mut count = 1;
            while let Some(cells) = grid.get(row + count) {
                if cells
                    .iter()
                    .any(|(at, merge)| *at == column && *merge == VerticalMerge::Continue)
                {
                    count += 1;
                } else {
                    break;
                }
            }
            count
        };
        let headings = table.rows.iter().take_while(|row| row.heading).count();
        let [top, right, bottom, left] = table.cell_margins;
        for (row_index, row) in table.rows.iter().enumerate() {
            if row_index == 0 && headings > 0 {
                out.push_str("<table:table-header-rows>");
            }
            let height = match row.height {
                RowHeight::AtLeast(height) => {
                    format!(r#" style:min-row-height="{}""#, length(height))
                }
                RowHeight::Exact(height) => format!(r#" style:row-height="{}""#, length(height)),
            };
            let keep = if row.keep_together {
                r#" fo:keep-together="always""#
            } else {
                r#" fo:keep-together="auto""#
            };
            let breaks = if row.page_break_before && row_index > 0 {
                r#" fo:break-before="page""#
            } else {
                ""
            };
            let row_style = self.automatic(
                'R',
                "table-row",
                format!(r#"><style:table-row-properties{height}{keep}{breaks}/>"#),
            );
            let _ = write!(out, r#"<table:table-row table:style-name="{row_style}">"#);
            let mut column = 0;
            for cell in &row.cells {
                let span = usize::from(cell.span.max(1));
                if cell.merge == VerticalMerge::Continue {
                    for _ in 0..span {
                        out.push_str("<table:covered-table-cell/>");
                    }
                    column += span;
                    continue;
                }
                let mut properties = format!(
                    r#" fo:padding-top="{}" fo:padding-right="{}" fo:padding-bottom="{}" fo:padding-left="{}""#,
                    length(top),
                    length(right),
                    length(bottom),
                    length(left)
                );
                for (side, name) in [(0, "top"), (1, "right"), (2, "bottom"), (3, "left")] {
                    let value = cell.borders[side].map_or_else(
                        || "none".to_owned(),
                        |line| border(line.kind, line.width, line.colour),
                    );
                    let _ = write!(properties, r#" fo:border-{name}="{value}""#);
                }
                if let Some(fill) = cell.shading {
                    let _ = write!(properties, r##" fo:background-color="#{}""##, hex(fill));
                }
                let cell_style = self.automatic(
                    'E',
                    "table-cell",
                    format!(r#"><style:table-cell-properties{properties}/>"#),
                );
                let rows = if cell.merge == VerticalMerge::Start {
                    spanned(row_index, column)
                } else {
                    1
                };
                let _ = write!(
                    out,
                    r#"<table:table-cell table:style-name="{cell_style}" office:value-type="string""#
                );
                if span > 1 {
                    let _ = write!(out, r#" table:number-columns-spanned="{span}""#);
                }
                if rows > 1 {
                    let _ = write!(out, r#" table:number-rows-spanned="{rows}""#);
                }
                out.push('>');
                self.blocks(&cell.blocks, section, out);
                out.push_str("</table:table-cell>");
                for _ in 1..span {
                    out.push_str("<table:covered-table-cell/>");
                }
                column += span;
            }
            out.push_str("</table:table-row>");
            if headings > 0 && row_index + 1 == headings {
                out.push_str("</table:table-header-rows>");
            }
        }
        out.push_str("</table:table>");
    }

    // ---- styles.xml, meta.xml, the manifest ----

    /// A section's page layout and master page.
    fn master_page(&mut self, section: &Section, index: usize) -> String {
        let page = &section.page;
        let name = format!("MP{}", index + 1);
        let layout = format!("pm{}", index + 1);
        let header = !section.headers.is_empty();
        let footer = !section.footers.is_empty();
        let top = if header {
            page.header_distance
        } else {
            page.margin_top
        };
        let bottom = if footer {
            page.footer_distance
        } else {
            page.margin_bottom
        };
        let mut properties = format!(
            r#"fo:page-width="{}" fo:page-height="{}" style:print-orientation="{}" fo:margin-top="{}" fo:margin-bottom="{}" fo:margin-left="{}" fo:margin-right="{}""#,
            length(page.width),
            length(page.height),
            if page.landscape {
                "landscape"
            } else {
                "portrait"
            },
            length(top),
            length(bottom),
            length(page.margin_left),
            length(page.margin_right),
        );
        if let Some(numbers) = section.page_numbers {
            let _ = write!(
                properties,
                r#" style:num-format="{}""#,
                odf_number_format(numbers.format)
            );
        }
        if self.document.settings.mirror_margins {
            properties.push_str(r#" style:page-usage="mirrored""#);
        }
        if let Some(page_border) = page.border {
            let line = border(
                page_border.line.kind,
                page_border.line.width,
                page_border.line.colour,
            );
            for (on, side) in [
                (page_border.top, "top"),
                (page_border.bottom, "bottom"),
                (page_border.left, "left"),
                (page_border.right, "right"),
            ] {
                if on {
                    let _ = write!(properties, r#" fo:border-{side}="{line}""#);
                }
            }
        }
        let columns = &page.columns;
        let mut columns_xml = String::new();
        if columns.count > 1 {
            let _ = write!(
                columns_xml,
                r#"<style:columns fo:column-count="{}" fo:column-gap="{}">"#,
                columns.count,
                length(columns.gap)
            );
            if columns.line_between {
                columns_xml.push_str(
                    r##"<style:column-sep style:width="0.0071in" style:color="#000000" style:height="100%" style:vertical-align="top"/>"##,
                );
            }
            if let Some(widths) = &columns.widths {
                let mut before = 0;
                for (width, gap) in widths {
                    let _ = write!(
                        columns_xml,
                        r#"<style:column style:rel-width="{}*" fo:start-indent="{}" fo:end-indent="{}"/>"#,
                        width + before / 2 + gap / 2,
                        length(before / 2),
                        length(gap / 2)
                    );
                    before = *gap;
                }
            }
            columns_xml.push_str("</style:columns>");
        }
        let band = |height: Twips, margin: &str| {
            format!(
                r#"<style:header-footer-properties fo:min-height="{}" fo:{margin}="0in" style:dynamic-spacing="false"/>"#,
                length(height.max(0))
            )
        };
        let header_style = if header {
            format!(
                "<style:header-style>{}</style:header-style>",
                band(page.margin_top - page.header_distance, "margin-bottom")
            )
        } else {
            "<style:header-style/>".into()
        };
        let footer_style = if footer {
            format!(
                "<style:footer-style>{}</style:footer-style>",
                band(page.margin_bottom - page.footer_distance, "margin-top")
            )
        } else {
            "<style:footer-style/>".into()
        };
        let _ = write!(
            self.page_layouts,
            r#"<style:page-layout style:name="{layout}"><style:page-layout-properties {properties}>{columns_xml}</style:page-layout-properties>{header_style}{footer_style}</style:page-layout>"#
        );
        let mut master =
            format!(r#"<style:master-page style:name="{name}" style:page-layout-name="{layout}">"#);
        self.in_master = true;
        for (set, element) in [(&section.headers, "header"), (&section.footers, "footer")] {
            for (blocks, suffix) in [
                (&set.default, ""),
                (&set.even, "-left"),
                (&set.first, "-first"),
            ] {
                if let Some(blocks) = blocks {
                    let mut inner = String::new();
                    self.blocks(blocks, section, &mut inner);
                    let _ = write!(
                        master,
                        "<style:{element}{suffix}>{inner}</style:{element}{suffix}>"
                    );
                }
            }
        }
        self.in_master = false;
        master.push_str("</style:master-page>");
        self.master_xml.push_str(&master);
        self.masters.push((name.clone(), layout));
        name
    }

    fn font_faces(&self) -> String {
        let mut xml = String::from("<office:font-face-decls>");
        for font in &self.document.fonts {
            let family = match font.family {
                FontFamily::Roman => "roman",
                FontFamily::Swiss => "swiss",
                FontFamily::Modern => "modern",
                FontFamily::Script => "script",
                FontFamily::Decorative => "decorative",
                FontFamily::Unknown => "system",
            };
            let _ = write!(
                xml,
                r#"<style:font-face style:name="{0}" svg:font-family="'{0}'" style:font-family-generic="{family}" style:font-pitch="{1}"/>"#,
                escape(&font.name),
                if font.fixed { "fixed" } else { "variable" }
            );
        }
        xml.push_str("</office:font-face-decls>");
        xml
    }

    fn styles(&mut self) -> String {
        let document = self.document;
        // Footnote Options: the first number, numbering on each page
        // (ODF counts from the start value plus one).
        let settings = &document.settings;
        let notes = format!(
            r#"<text:notes-configuration text:note-class="footnote" style:num-format="1" text:start-value="{}" text:footnotes-position="page" text:start-numbering-at="{}"/><text:notes-configuration text:note-class="endnote" style:num-format="1" text:start-value="{}"/>"#,
            settings.footnote_start.max(1) - 1,
            if settings.footnotes_each_page {
                "page"
            } else {
                "document"
            },
            settings.footnote_start.max(1) - 1,
        );
        let mut named = String::new();
        named.push_str(
            r#"<style:default-style style:family="paragraph"><style:paragraph-properties fo:margin-top="0in" fo:margin-bottom="0in" style:tab-stop-distance="0.5in"/><style:text-properties fo:language="en" fo:country="US"/></style:default-style><style:style style:name="Frame" style:family="graphic"/>"#,
        );
        for (index, style) in document.styles.iter().enumerate() {
            let mut attributes = format!(
                r#" style:name="{}" style:display-name="{}" style:family="paragraph""#,
                self.style_names[index],
                escape(&style.name)
            );
            if let Some(parent) = style.based_on.filter(|parent| *parent != index) {
                let _ = write!(
                    attributes,
                    r#" style:parent-style-name="{}""#,
                    self.style_names[parent]
                );
            }
            let next = style.next.unwrap_or(index);
            let _ = write!(
                attributes,
                r#" style:next-style-name="{}""#,
                self.style_names[next]
            );
            if let Some(level) = style.outline_level {
                let _ = write!(attributes, r#" style:default-outline-level="{level}""#);
            }
            if let Some((list, _)) = style.list {
                let _ = write!(attributes, r#" style:list-style-name="L{}""#, list + 1);
            }
            let mut props = style.paragraph.clone();
            let tabs = props.tabs.take();
            let mut properties = paragraph_properties(&props, None);
            if let Some(tabs) = tabs {
                properties.tabs = tab_stops(&tabs);
            }
            let _ = write!(
                named,
                r#"<style:style{attributes}>{}<style:text-properties{}{}/></style:style>"#,
                properties.element(),
                text_properties(&style.character),
                hyphenate(style.paragraph.no_hyphenation)
            );
        }
        // The list styles, which named paragraph styles name.
        let lists = self.content_lists();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><office:document-styles {NAMESPACES}>{}<office:styles>{named}{lists}{notes}</office:styles><office:automatic-styles>{}{}</office:automatic-styles><office:master-styles>{}</office:master-styles></office:document-styles>"#,
            self.font_faces(),
            self.page_layouts,
            self.master_automatic_xml,
            self.master_xml
        )
    }

    /// The list styles as named styles.
    fn content_lists(&self) -> String {
        let mut lists = String::new();
        for (index, list) in self.document.lists.iter().enumerate() {
            let _ = write!(lists, r#"<text:list-style style:name="L{}">"#, index + 1);
            for (level_index, level) in list.levels.iter().enumerate() {
                // A bullet of more than one character (Ami Pro's "Note:"):
                // text with no number.
                if level.format == NumberFormat::Bullet && level.text.chars().count() == 1 {
                    let _ = write!(
                        lists,
                        r#"<text:list-level-style-bullet text:level="{}" text:bullet-char="{}">"#,
                        level_index + 1,
                        escape(&level.text.chars().next().unwrap_or('\u{2022}').to_string())
                    );
                } else {
                    let (prefix, suffix, display) = number_affixes(&level.text, level_index);
                    let _ = write!(
                        lists,
                        r#"<text:list-level-style-number text:level="{}" style:num-prefix="{}" style:num-suffix="{}" style:num-format="{}" text:start-value="{}" text:display-levels="{display}">"#,
                        level_index + 1,
                        escape(&prefix),
                        escape(&suffix),
                        odf_number_format(level.format),
                        level.start,
                    );
                }
                let _ = write!(
                    lists,
                    r#"<style:list-level-properties text:list-level-position-and-space-mode="label-alignment"{}><style:list-level-label-alignment text:label-followed-by="listtab" text:list-tab-stop-position="{}" fo:text-indent="{}" fo:margin-left="{}"/></style:list-level-properties>"#,
                    if level.right_align {
                        r#" fo:text-align="end""#
                    } else {
                        ""
                    },
                    length(level.indent_left),
                    length(-level.hanging.max(0)),
                    length(level.indent_left),
                );
                // The number's own font and look (superscript, bold).
                let mut props = level.props.clone();
                if level.font.is_some() {
                    props.font.clone_from(&level.font);
                }
                let text = text_properties(&props);
                if !text.is_empty() {
                    let _ = write!(lists, "<style:text-properties{text}/>");
                }
                lists.push_str(
                    if level.format == NumberFormat::Bullet && level.text.chars().count() == 1 {
                        "</text:list-level-style-bullet>"
                    } else {
                        "</text:list-level-style-number>"
                    },
                );
            }
            lists.push_str("</text:list-style>");
        }
        lists
    }

    fn meta(&self) -> String {
        let properties = &self.document.properties;
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><office:document-meta {NAMESPACES}><office:meta><meta:generator>AmiPro-Linux</meta:generator>"#
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
        for keyword in properties
            .keywords
            .split([',', ';'])
            .map(str::trim)
            .filter(|keyword| !keyword.is_empty())
        {
            let _ = write!(xml, "<meta:keyword>{}</meta:keyword>", escape(keyword));
        }
        if let Some(created) = properties.created {
            let _ = write!(
                xml,
                "<meta:creation-date>{}</meta:creation-date>",
                time(&created)
            );
        }
        if let Some(modified) = properties.modified {
            let _ = write!(xml, "<dc:date>{}</dc:date>", time(&modified));
        }
        if properties.revisions > 0 {
            let _ = write!(
                xml,
                "<meta:editing-cycles>{}</meta:editing-cycles>",
                properties.revisions
            );
        }
        let _ = write!(
            xml,
            r#"<meta:editing-duration>PT{}M</meta:editing-duration><meta:document-statistic meta:page-count="{}" meta:word-count="{}" meta:character-count="{}"/>"#,
            properties.editing_minutes, properties.pages, properties.words, properties.characters
        );
        if !properties.template.is_empty() {
            let _ = write!(
                xml,
                r#"<meta:template xlink:type="simple" xlink:href="{0}" xlink:title="{0}"/>"#,
                escape(&properties.template)
            );
        }
        for (name, value) in &properties.custom {
            let _ = write!(
                xml,
                r#"<meta:user-defined meta:name="{}">{}</meta:user-defined>"#,
                escape(name),
                escape(value)
            );
        }
        xml.push_str("</office:meta></office:document-meta>");
        xml
    }

    fn manifest(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3"><manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="application/vnd.oasis.opendocument.text"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>"#,
        );
        for (index, image) in self.document.images.iter().enumerate() {
            let _ = write!(
                xml,
                r#"<manifest:file-entry manifest:full-path="Pictures/image{}.{}" manifest:media-type="{}"/>"#,
                index + 1,
                image.kind.extension(),
                image.kind.media_type()
            );
            if image.fallback.is_some() {
                let _ = write!(
                    xml,
                    r#"<manifest:file-entry manifest:full-path="Pictures/image{}.png" manifest:media-type="image/png"/>"#,
                    index + 1
                );
            }
        }
        xml.push_str("</manifest:manifest>");
        xml
    }
}

/// Paragraph properties: attributes and the tab stops' element.
#[derive(Default)]
struct ParagraphXml {
    attributes: String,
    tabs: String,
}

impl ParagraphXml {
    fn is_empty(&self) -> bool {
        self.attributes.is_empty() && self.tabs.is_empty()
    }

    fn element(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        if self.tabs.is_empty() {
            format!("<style:paragraph-properties{}/>", self.attributes)
        } else {
            format!(
                "<style:paragraph-properties{}>{}</style:paragraph-properties>",
                self.attributes, self.tabs
            )
        }
    }
}

fn paragraph_properties(props: &ParagraphProps, _base_tabs: Option<&[Tab]>) -> ParagraphXml {
    let mut xml = String::new();
    if let Some(alignment) = props.alignment {
        let value = match alignment {
            Alignment::Left => "start",
            Alignment::Right => "end",
            Alignment::Centre => "center",
            Alignment::Justify => "justify",
        };
        let _ = write!(xml, r#" fo:text-align="{value}""#);
    }
    if let Some(left) = props.indent_left {
        let _ = write!(xml, r#" fo:margin-left="{}""#, length(left));
    }
    if let Some(right) = props.indent_right {
        let _ = write!(xml, r#" fo:margin-right="{}""#, length(right));
    }
    if let Some(first) = props.first_line {
        let _ = write!(xml, r#" fo:text-indent="{}""#, length(first));
    }
    if let Some(before) = props.space_before {
        let _ = write!(xml, r#" fo:margin-top="{}""#, length(before));
    }
    if let Some(after) = props.space_after {
        let _ = write!(xml, r#" fo:margin-bottom="{}""#, length(after));
    }
    match props.line {
        Some(LineSpacing::Single) => xml.push_str(r#" fo:line-height="100%""#),
        Some(LineSpacing::Multiple(multiple)) => {
            let _ = write!(xml, r#" fo:line-height="{}%""#, (multiple * 100.0).round());
        }
        Some(LineSpacing::Exact(height)) => {
            let _ = write!(xml, r#" fo:line-height="{}""#, length(height));
        }
        Some(LineSpacing::AtLeast(height)) => {
            let _ = write!(xml, r#" style:line-height-at-least="{}""#, length(height));
        }
        None => {}
    }
    let flag = |xml: &mut String, value: Option<bool>, name: &str, on: &str, off: &str| {
        if let Some(value) = value {
            let _ = write!(xml, r#" {name}="{}""#, if value { on } else { off });
        }
    };
    flag(
        &mut xml,
        props.keep_with_next,
        "fo:keep-with-next",
        "always",
        "auto",
    );
    flag(
        &mut xml,
        props.keep_together,
        "fo:keep-together",
        "always",
        "auto",
    );
    match (props.page_break_before, props.column_break_before) {
        (Some(true), _) => xml.push_str(r#" fo:break-before="page""#),
        (_, Some(true)) => xml.push_str(r#" fo:break-before="column""#),
        (Some(false), _) | (_, Some(false)) => xml.push_str(r#" fo:break-before="auto""#),
        _ => {}
    }
    for (line, side) in [(&props.border_top, "top"), (&props.border_bottom, "bottom")] {
        match line {
            Some(Some(line)) => {
                let _ = write!(
                    xml,
                    r#" fo:border-{side}="{}" fo:padding-{side}="{}""#,
                    border(line.kind, line.width, line.colour),
                    length(line.space)
                );
            }
            Some(None) => {
                let _ = write!(xml, r#" fo:border-{side}="none""#);
            }
            None => {}
        }
    }
    ParagraphXml {
        attributes: xml,
        tabs: props.tabs.as_deref().map(tab_stops).unwrap_or_default(),
    }
}

fn tab_stops(tabs: &[Tab]) -> String {
    let mut xml = String::from("<style:tab-stops>");
    for tab in tabs {
        let kind = match tab.kind {
            TabKind::Left => "left",
            TabKind::Centre => "center",
            TabKind::Right => "right",
            TabKind::Decimal => "char",
        };
        let _ = write!(
            xml,
            r#"<style:tab-stop style:position="{}" style:type="{kind}""#,
            length(tab.position)
        );
        if tab.kind == TabKind::Decimal {
            xml.push_str(r#" style:char=".""#);
        }
        match tab.leader {
            TabLeader::None => {}
            TabLeader::Dots => {
                xml.push_str(r#" style:leader-style="dotted" style:leader-text=".""#)
            }
            TabLeader::Hyphens => {
                xml.push_str(r#" style:leader-style="dash" style:leader-text="-""#)
            }
            TabLeader::Line => xml.push_str(r#" style:leader-style="solid" style:leader-text="_""#),
        }
        xml.push_str("/>");
    }
    xml.push_str("</style:tab-stops>");
    xml
}

/// The attributes of `style:text-properties` for `props`.
fn text_properties(props: &CharProps) -> String {
    let mut xml = String::new();
    if let Some(font) = &props.font {
        let _ = write!(xml, r#" style:font-name="{}""#, escape(font));
    }
    if let Some(size) = props.size {
        let _ = write!(xml, r#" fo:font-size="{}pt""#, f64::from(size) / 20.0);
    }
    if let Some(bold) = props.bold {
        let _ = write!(
            xml,
            r#" fo:font-weight="{}""#,
            if bold { "bold" } else { "normal" }
        );
    }
    if let Some(italic) = props.italic {
        let _ = write!(
            xml,
            r#" fo:font-style="{}""#,
            if italic { "italic" } else { "normal" }
        );
    }
    match props.underline {
        Some(Underline::None) => xml.push_str(r#" style:text-underline-style="none""#),
        Some(Underline::Single) => xml.push_str(
            r#" style:text-underline-style="solid" style:text-underline-width="auto" style:text-underline-color="font-color""#,
        ),
        Some(Underline::Words) => xml.push_str(
            r#" style:text-underline-style="solid" style:text-underline-mode="skip-white-space" style:text-underline-width="auto" style:text-underline-color="font-color""#,
        ),
        Some(Underline::Double) => xml.push_str(
            r#" style:text-underline-style="solid" style:text-underline-type="double" style:text-underline-width="auto" style:text-underline-color="font-color""#,
        ),
        None => {}
    }
    if let Some(strike) = props.strike {
        let _ = write!(
            xml,
            r#" style:text-line-through-style="{}""#,
            if strike { "solid" } else { "none" }
        );
    }
    match props.script {
        Some(Script::Super) => xml.push_str(r#" style:text-position="super 58%""#),
        Some(Script::Sub) => xml.push_str(r#" style:text-position="sub 58%""#),
        Some(Script::Normal) => xml.push_str(r#" style:text-position="0% 100%""#),
        None => {}
    }
    match (props.caps, props.small_caps) {
        (Some(true), _) => xml.push_str(r#" fo:text-transform="uppercase""#),
        (Some(false), _) => xml.push_str(r#" fo:text-transform="none""#),
        _ => {}
    }
    if let Some(small) = props.small_caps {
        let _ = write!(
            xml,
            r#" fo:font-variant="{}""#,
            if small { "small-caps" } else { "normal" }
        );
    }
    if let Some(colour) = props.colour {
        let _ = write!(xml, r##" fo:color="#{}""##, hex(colour));
    }
    xml
}

/// Text with ODF's spaces: a space after another, or at the start, as
/// `text:s`.
fn text_run(out: &mut String, text: &str) {
    let mut spaces = 0;
    let mut at_start = true;
    let flush = |out: &mut String, spaces: &mut usize, at_start: bool| {
        if *spaces == 0 {
            return;
        }
        let literal = usize::from(!at_start);
        if literal == 1 {
            out.push(' ');
        }
        let rest = *spaces - literal;
        if rest == 1 {
            out.push_str("<text:s/>");
        } else if rest > 1 {
            let _ = write!(out, r#"<text:s text:c="{rest}"/>"#);
        }
        *spaces = 0;
    };
    for character in text.chars() {
        if character == ' ' {
            spaces += 1;
            continue;
        }
        flush(out, &mut spaces, at_start);
        at_start = false;
        match character {
            '\t' => out.push_str("<text:tab/>"),
            '\n' => out.push_str("<text:line-break/>"),
            _ => out.push_str(&escape(&character.to_string())),
        }
    }
    flush(out, &mut spaces, at_start);
}

/// Style names: the name's letters with spaces as `_20_`, as LibreOffice
/// writes them, made unique.
fn style_names(document: &ExchangeDocument) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for style in &document.styles {
        let mut name = String::new();
        for character in style.name.chars() {
            if character.is_ascii_alphanumeric() || character == '-' || character == '.' {
                name.push(character);
            } else {
                let _ = write!(name, "_{:x}_", u32::from(character));
            }
        }
        if name.is_empty()
            || name
                .starts_with(|first: char| first.is_ascii_digit() || first == '-' || first == '.')
        {
            name = format!("Style_{name}");
        }
        let base = name.clone();
        let mut number = 2;
        while names.contains(&name) {
            name = format!("{base}{number}");
            number += 1;
        }
        names.push(name);
    }
    names
}

/// A list level's text split around its own number: the prefix, the
/// suffix, and how many levels the number shows (`%1.%2.` shows two).
fn number_affixes(text: &str, level: usize) -> (String, String, usize) {
    let own = format!("%{}", level + 1);
    let Some(at) = text.find(&own) else {
        return (text.replace("%%", "%"), String::new(), 1);
    };
    let before = &text[..at];
    let after = &text[at + own.len()..];
    // Cumulative numbers: the levels shown before the own one.
    let shown = before.matches('%').filter(|_| true).count();
    let prefix_start = if shown > 0 {
        before.find('%').unwrap_or(0)
    } else {
        before.len()
    };
    let display = shown + 1;
    (
        before[..prefix_start].replace("%%", "%"),
        after.replace("%%", "%"),
        display,
    )
}

/// Hyphenation as a text property: on where Ami Pro hyphenates.
fn hyphenate(no_hyphenation: Option<bool>) -> &'static str {
    match no_hyphenation {
        Some(false) => r#" fo:hyphenate="true""#,
        Some(true) => r#" fo:hyphenate="false""#,
        None => "",
    }
}

/// A Word date picture as a data style's parts: days, months, years, and
/// the text between them.
fn date_parts(picture: &str) -> String {
    let mut xml = String::new();
    let mut literal = String::new();
    let characters: Vec<char> = picture.chars().collect();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        let run = characters[index..]
            .iter()
            .take_while(|other| **other == character)
            .count();
        let part = match (character, run) {
            ('d', 1) => Some(r#"<number:day/>"#),
            ('d', 2) => Some(r#"<number:day number:style="long"/>"#),
            ('d', 3) => Some(r#"<number:day-of-week/>"#),
            ('d', _) => Some(r#"<number:day-of-week number:style="long"/>"#),
            ('M', 1) => Some(r#"<number:month/>"#),
            ('M', 2) => Some(r#"<number:month number:style="long"/>"#),
            ('M', 3) => Some(r#"<number:month number:textual="true"/>"#),
            ('M', _) => Some(r#"<number:month number:style="long" number:textual="true"/>"#),
            ('y', 1 | 2) => Some(r#"<number:year/>"#),
            ('y', _) => Some(r#"<number:year number:style="long"/>"#),
            _ => None,
        };
        match part {
            Some(part) => {
                if !literal.is_empty() {
                    let _ = write!(xml, "<number:text>{}</number:text>", escape(&literal));
                    literal.clear();
                }
                xml.push_str(part);
                index += run;
            }
            None => {
                if character != '\'' {
                    literal.push(character);
                }
                index += 1;
            }
        }
    }
    if !literal.is_empty() {
        let _ = write!(xml, "<number:text>{}</number:text>", escape(&literal));
    }
    xml
}

fn odf_number_format(format: NumberFormat) -> &'static str {
    match format {
        NumberFormat::Decimal => "1",
        NumberFormat::UpperRoman => "I",
        NumberFormat::LowerRoman => "i",
        NumberFormat::UpperLetter => "A",
        NumberFormat::LowerLetter => "a",
        NumberFormat::Bullet | NumberFormat::None => "",
    }
}

/// A length in points.
fn length(twips: Twips) -> String {
    format!("{}pt", f64::from(twips) / 20.0)
}

/// A border's value: width, style, colour.
fn border(kind: LineKind, width: Twips, colour: crate::Colour) -> String {
    let style = match kind {
        LineKind::Single => "solid",
        _ => "double",
    };
    format!("{} {style} #{}", length(width.max(1)), hex(colour))
}

fn time(clock: &amipro_document::WallClock) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        clock.year, clock.month, clock.day, clock.hour, clock.minute, clock.second
    )
}
