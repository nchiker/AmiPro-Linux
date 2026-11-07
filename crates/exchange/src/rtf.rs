//! The Rich Text filter (RTF 1.9.1; EX1): the exchange document as one RTF
//! file. RTF readers take formatting from the control words, not from the
//! style a paragraph names, so every paragraph and run states its whole
//! formatting, as Word writes it. Frames are shapes (`\shp`) where Ami Pro
//! places them, each with a positioned paragraph as its result, which
//! older readers and Ami Pro itself read as a frame (EX3, `rtf.flt`'s
//! `posx`, `absw`).

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::{
    Alignment, Anchor, Block, CharProps, ExchangeDocument, Field, FieldKind, FontFamily,
    HorizontalFrom, ImageKind, Inline, LineKind, LineSpacing, NumberFormat, Paragraph,
    ParagraphProps, RowHeight, RuleLine, Script, Section, Shape, ShapeContent, ShapeWrap, TabKind,
    TabLeader, Table, Twips, Underline, VerticalFrom, VerticalMerge, rgb,
};

/// The document as RTF.
pub fn write(document: &ExchangeDocument) -> Vec<u8> {
    let mut writer = Writer::new(document);
    writer.document().into_bytes()
}

struct Writer<'a> {
    document: &'a ExchangeDocument,
    fonts: HashMap<String, usize>,
    colours: Vec<crate::Colour>,
    next_shape: u32,
    /// Writing a header or footer: its shapes say so (`\shpfhdr1`).
    in_header: bool,
}

impl<'a> Writer<'a> {
    fn new(document: &'a ExchangeDocument) -> Self {
        let fonts = document
            .fonts
            .iter()
            .enumerate()
            .map(|(index, font)| (font.name.clone(), index))
            .collect();
        Self {
            document,
            fonts,
            colours: Vec::new(),
            next_shape: 1,
            in_header: false,
        }
    }

    fn colour(&mut self, colour: crate::Colour) -> usize {
        match self.colours.iter().position(|known| *known == colour) {
            Some(index) => index + 1,
            None => {
                self.colours.push(colour);
                self.colours.len()
            }
        }
    }

    fn font(&mut self, name: &str) -> usize {
        if let Some(index) = self.fonts.get(name) {
            return *index;
        }
        let index = self.fonts.len();
        self.fonts.insert(name.to_owned(), index);
        index
    }

    fn document(&mut self) -> String {
        // The body first: it fills the font and colour tables.
        let mut body = String::new();
        let sections = &self.document.sections;
        for (index, section) in sections.iter().enumerate() {
            if index > 0 {
                body.push_str("\\sect");
            }
            self.section(section, &mut body);
        }
        let stylesheet = self.stylesheet();
        let lists = self.lists();
        let mut out = String::from("{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1\n");
        out.push_str("{\\fonttbl");
        let mut fonts: Vec<(&String, &usize)> = self.fonts.iter().collect();
        fonts.sort_by_key(|(_, index)| **index);
        for (name, index) in fonts {
            let family = self
                .document
                .fonts
                .iter()
                .find(|font| font.name == *name)
                .map_or(FontFamily::Unknown, |font| font.family);
            let family = match family {
                FontFamily::Roman => "froman",
                FontFamily::Swiss => "fswiss",
                FontFamily::Modern => "fmodern",
                FontFamily::Script => "fscript",
                FontFamily::Decorative => "fdecor",
                FontFamily::Unknown => "fnil",
            };
            let _ = write!(out, "{{\\f{index}\\{family}\\fcharset0 {};}}", text(name));
        }
        out.push_str("}\n{\\colortbl;");
        for colour in &self.colours {
            let (red, green, blue) = rgb(*colour);
            let _ = write!(out, "\\red{red}\\green{green}\\blue{blue};");
        }
        out.push_str("}\n");
        out.push_str(&stylesheet);
        out.push_str(&lists);
        // The rows' heights are Ami Pro's measure (`Settings::ami_pro_rows`).
        out.push_str(&format!("{{\\*\\generator {};}}", crate::GENERATOR));
        out.push_str(&self.info());
        let settings = &self.document.settings;
        out.push_str("\\deftab720");
        if settings.even_and_odd_headers {
            out.push_str("\\facingp");
        }
        if settings.mirror_margins {
            out.push_str("\\margmirror");
        }
        if settings.hyphenation {
            let _ = write!(out, "\\hyphauto1\\hyphhotz{}", settings.hyphenation_zone);
        }
        if let Some(first) = self.document.sections.first() {
            let page = &first.page;
            let _ = write!(
                out,
                "\\paperw{}\\paperh{}\\margl{}\\margr{}\\margt{}\\margb{}{}",
                page.width,
                page.height,
                page.margin_left,
                page.margin_right,
                page.margin_top,
                page.margin_bottom,
                if page.landscape { "\\landscape" } else { "" }
            );
        }
        out.push('\n');
        out.push_str(&body);
        out.push_str("}\n");
        out
    }

    // ---- The header groups ----

    fn stylesheet(&mut self) -> String {
        let document = self.document;
        let mut out = String::from("{\\stylesheet");
        for (index, style) in document.styles.iter().enumerate() {
            let mut props = String::new();
            self.paragraph_words(&style.paragraph, None, &mut props);
            let character = self.char_words(&style.character);
            let _ = write!(out, "{{\\s{index}{props}{character}");
            if let Some(parent) = style.based_on.filter(|parent| *parent != index) {
                let _ = write!(out, "\\sbasedon{parent}");
            }
            let _ = write!(out, "\\snext{}", style.next.unwrap_or(index));
            if let Some(level) = style.outline_level {
                let _ = write!(out, "\\outlinelevel{}", level.saturating_sub(1));
            }
            // The style's list: the override written for the list itself.
            if let Some((list, level)) = style.list {
                let _ = write!(
                    out,
                    "\\ls{}\\ilvl{level}",
                    document.list_instances.len() + list + 1
                );
            }
            let _ = write!(out, " {};}}", text(&style.name));
        }
        out.push_str("}\n");
        out
    }

    fn lists(&mut self) -> String {
        let document = self.document;
        if document.lists.is_empty() {
            return String::new();
        }
        let mut out = String::from("{\\*\\listtable");
        for (index, list) in document.lists.iter().enumerate() {
            let _ = write!(
                out,
                "{{\\list\\listtemplateid{}{}",
                index + 1,
                if list.levels.len() > 1 {
                    ""
                } else {
                    "\\listsimple"
                }
            );
            for (level_index, level) in list.levels.iter().enumerate() {
                let nfc = match level.format {
                    NumberFormat::Decimal => 0,
                    NumberFormat::UpperRoman => 1,
                    NumberFormat::LowerRoman => 2,
                    NumberFormat::UpperLetter => 3,
                    NumberFormat::LowerLetter => 4,
                    NumberFormat::Bullet => 23,
                    NumberFormat::None => 255,
                };
                let (template, numbers) = level_text(&level.text);
                let restart = if level_index > 0 && !level.restart_after_higher {
                    "\\levelnorestart1"
                } else {
                    ""
                };
                let props = self.char_words(&level.props);
                let _ = write!(
                    out,
                    "{{\\listlevel\\levelnfc{nfc}\\levelnfcn{nfc}\\leveljc{0}\\leveljcn{0}\\levelfollow0\\levelstartat{1}{restart}{{\\leveltext{template};}}{{\\levelnumbers{numbers};}}{props}\\fi-{2}\\li{3}}}",
                    if level.right_align { 2 } else { 0 },
                    level.start,
                    level.hanging.max(0),
                    level.indent_left
                );
            }
            let _ = write!(out, "\\listid{}}}", index + 1);
        }
        out.push_str("}\n{\\*\\listoverridetable");
        for (index, instance) in document.list_instances.iter().enumerate() {
            let levels = document.lists[instance.list].levels.len().min(9);
            let _ = write!(
                out,
                "{{\\listoverride\\listid{}\\listoverridecount{levels}",
                instance.list + 1
            );
            for level in 0..levels {
                let start = instance.starts[level]
                    .unwrap_or(document.lists[instance.list].levels[level].start);
                let _ = write!(
                    out,
                    "{{\\lfolevel\\listoverridestartat\\levelstartat{start}}}"
                );
            }
            let _ = write!(out, "\\ls{}}}", index + 1);
        }
        for list in 0..document.lists.len() {
            let _ = write!(
                out,
                "{{\\listoverride\\listid{}\\listoverridecount0\\ls{}}}",
                list + 1,
                document.list_instances.len() + list + 1
            );
        }
        out.push_str("}\n");
        out
    }

    fn info(&self) -> String {
        let properties = &self.document.properties;
        let mut out = String::from("{\\info");
        for (word, value) in [
            ("title", &properties.title),
            ("keywords", &properties.keywords),
            ("doccomm", &properties.description),
        ] {
            if !value.is_empty() {
                let _ = write!(out, "{{\\{word} {}}}", text(value));
            }
        }
        for (word, time) in [
            ("creatim", properties.created),
            ("revtim", properties.modified),
        ] {
            if let Some(time) = time {
                let _ = write!(
                    out,
                    "{{\\{word}\\yr{}\\mo{}\\dy{}\\hr{}\\min{}\\sec{}}}",
                    time.year, time.month, time.day, time.hour, time.minute, time.second
                );
            }
        }
        let _ = write!(
            out,
            "{{\\version{}}}{{\\edmins{}}}{{\\nofpages{}}}{{\\nofwords{}}}{{\\nofchars{}}}",
            properties.revisions.max(1),
            properties.editing_minutes,
            properties.pages,
            properties.words,
            properties.characters
        );
        if !properties.custom.is_empty() {
            out.push_str("{\\*\\userprops ");
            for (name, value) in &properties.custom {
                let _ = write!(
                    out,
                    "{{\\propname {}}}\\proptype30{{\\staticval {}}}",
                    text(name),
                    text(value)
                );
            }
            out.push('}');
        }
        out.push_str("}\n");
        out
    }

    // ---- Sections ----

    fn section(&mut self, section: &Section, out: &mut String) {
        let page = &section.page;
        let _ = write!(
            out,
            "\\sectd{}\\pgwsxn{}\\pghsxn{}\\marglsxn{}\\margrsxn{}\\margtsxn{}\\margbsxn{}\\headery{}\\footery{}{}",
            if section.new_page {
                "\\sbkpage"
            } else {
                "\\sbknone"
            },
            page.width,
            page.height,
            page.margin_left,
            page.margin_right,
            page.margin_top,
            page.margin_bottom,
            page.header_distance,
            page.footer_distance,
            if page.landscape { "\\lndscpsxn" } else { "" }
        );
        let columns = &page.columns;
        if columns.count > 1 {
            let _ = write!(out, "\\cols{}\\colsx{}", columns.count, columns.gap);
            if columns.line_between {
                out.push_str("\\linebetcol");
            }
            if let Some(widths) = &columns.widths {
                for (index, (width, gap)) in widths.iter().enumerate() {
                    let _ = write!(out, "\\colno{}\\colw{width}\\colsr{gap}", index + 1);
                }
            }
        }
        if let Some(numbers) = section.page_numbers {
            let format = match numbers.format {
                NumberFormat::UpperRoman => "\\pgnucrm",
                NumberFormat::LowerRoman => "\\pgnlcrm",
                NumberFormat::UpperLetter => "\\pgnucltr",
                NumberFormat::LowerLetter => "\\pgnlcltr",
                _ => "\\pgndec",
            };
            out.push_str(format);
            if let Some(start) = numbers.start {
                let _ = write!(out, "\\pgnrestart\\pgnstarts{start}");
            }
        }
        if let Some(border) = page.border {
            for (on, side) in [
                (border.top, "t"),
                (border.left, "l"),
                (border.bottom, "b"),
                (border.right, "r"),
            ] {
                if on {
                    let colour = self.colour(border.line.colour);
                    let _ = write!(
                        out,
                        "\\pgbrdropt32\\pgbrdr{side}{}\\brsp{}\\brdrcf{colour}",
                        border_words(border.line.kind, border.line.width),
                        (border.distance / 20).clamp(0, 31)
                    );
                }
            }
        }
        if section.headers.first.is_some() || section.footers.first.is_some() {
            out.push_str("\\titlepg");
        }
        out.push('\n');
        let facing = self.document.settings.even_and_odd_headers;
        for (set, word) in [(&section.headers, "header"), (&section.footers, "footer")] {
            let slots = [
                (&set.default, if facing { "r" } else { "" }),
                (&set.even, "l"),
                (&set.first, "f"),
            ];
            for (blocks, suffix) in slots {
                if let Some(blocks) = blocks {
                    let _ = write!(out, "{{\\{word}{suffix} ");
                    self.in_header = true;
                    self.blocks(blocks, false, out);
                    self.in_header = false;
                    out.push_str("}\n");
                }
            }
        }
        self.blocks(&section.blocks, false, out);
    }

    fn blocks(&mut self, blocks: &[Block], in_table: bool, out: &mut String) {
        for (index, block) in blocks.iter().enumerate() {
            match block {
                Block::Paragraph(paragraph) => {
                    let last = index + 1 == blocks.len();
                    self.paragraph(paragraph, in_table, in_table && last, out);
                }
                Block::Table(table) => self.table(table, out),
            }
        }
        if blocks.is_empty() {
            out.push_str("\\pard\\plain");
            if in_table {
                out.push_str("\\intbl");
            }
            out.push_str("\\par\n");
        }
    }

    // ---- Paragraphs ----

    fn paragraph(
        &mut self,
        paragraph: &Paragraph,
        in_table: bool,
        ends_cell: bool,
        out: &mut String,
    ) {
        let document = self.document;
        let style = document.styles.get(paragraph.style);
        let mut props = style
            .map(|style| style.paragraph.clone())
            .unwrap_or_default();
        overlay_paragraph(&mut props, &paragraph.props);
        let _ = write!(out, "\\pard\\plain\\s{}", paragraph.style);
        if in_table {
            out.push_str("\\intbl");
        }
        let numbering = match paragraph.list {
            Some((Some(instance), level)) => Some((instance + 1, level)),
            _ => None,
        };
        self.paragraph_words(&props, numbering, out);
        let base = style
            .map(|style| style.character.clone())
            .unwrap_or_default();
        out.push(' ');
        if matches!(paragraph.list, Some((None, _)))
            && let Some(number) = &paragraph.number_text
        {
            let words = self.char_words(&base);
            let _ = write!(out, "{{\\plain{words} {}\\tab}}", text(number));
        }
        for inline in &paragraph.inlines {
            self.inline(inline, &base, out);
        }
        // The paragraph mark's formatting, in effect at its end.
        let mut mark = base.clone();
        mark.overlay(&paragraph.mark);
        let words = self.char_words(&mark);
        out.push_str(&words);
        out.push_str(if ends_cell { "\\cell\n" } else { "\\par\n" });
    }

    fn inline(&mut self, inline: &Inline, base: &CharProps, out: &mut String) {
        let full = |props: &CharProps| {
            let mut all = base.clone();
            all.overlay(props);
            all
        };
        match inline {
            Inline::Text {
                text: content,
                props,
            } => {
                let words = self.char_words(&full(props));
                let _ = write!(out, "{{\\plain{words} {}}}", text(content));
            }
            Inline::Tab { props } => {
                let words = self.char_words(&full(props));
                let _ = write!(out, "{{\\plain{words}\\tab}}");
            }
            Inline::LineBreak => out.push_str("\\line "),
            Inline::PageBreak => out.push_str("\\page "),
            Inline::ColumnBreak => out.push_str("\\column "),
            Inline::Field(field) => self.field(field, &full(&field.props), out),
            Inline::Symbol {
                character,
                font,
                props,
            } => {
                let mut props = full(props);
                if font.is_some() {
                    props.font.clone_from(font);
                }
                let words = self.char_words(&props);
                let _ = write!(out, "{{\\plain{words} {}}}", text(&character.to_string()));
            }
            Inline::Shape(shape) => self.shape(shape, out),
            // Footnotes (endnotes with `\ftnalt`), Ami Pro's notes as
            // annotations, and bookmarks (the notes run).
            Inline::Footnote(footnote) => {
                let mut body = String::new();
                self.blocks(&footnote.blocks, false, &mut body);
                let words = self.char_words(&full(&footnote.props));
                let alternative = if footnote.endnote { "\\ftnalt" } else { "" };
                let _ = write!(
                    out,
                    "{{\\plain{words}\\super\\chftn}}{{\\footnote{alternative}\\pard\\plain{{\\super\\chftn}} {body}}}"
                );
            }
            Inline::Note(note) => {
                let mut body = String::new();
                self.blocks(&note.blocks, false, &mut body);
                // The date as RTF's DTTM: minutes, hours, day, month, and
                // years since 1900 (no seconds).
                let date = note.date.map_or_else(String::new, |date| {
                    let dttm = u32::from(date.minute)
                        | u32::from(date.hour) << 6
                        | u32::from(date.day) << 11
                        | u32::from(date.month) << 16
                        | ((date.year - 1900).clamp(0, 511) as u32) << 20;
                    format!("{{\\*\\atndate {dttm}}}")
                });
                let _ = write!(
                    out,
                    "{{\\*\\atnid {}}}{{\\*\\atnauthor {}}}\\chatn{{\\*\\annotation{date}\\pard\\plain {body}}}",
                    text(&note.initials),
                    text(&note.author)
                );
            }
            Inline::Bookmark { name, end } => {
                let _ = write!(
                    out,
                    "{{\\*\\bkmk{} {}}}",
                    if *end { "end" } else { "start" },
                    text(name)
                );
            }
            Inline::Marker(_) => {}
        }
    }

    fn field(&mut self, field: &Field, props: &CharProps, out: &mut String) {
        let words = self.char_words(props);
        let instruction = crate::fields::instruction(&field.kind);
        let result = if matches!(
            field.kind,
            FieldKind::IndexEntry { .. } | FieldKind::ContentsEntry { .. }
        ) {
            String::new()
        } else {
            text(&field.result)
        };
        let _ = write!(
            out,
            "{{\\field{{\\*\\fldinst {{\\plain{words} {}}}}}{{\\fldrslt {{\\plain{words} {result}}}}}}}",
            text(&instruction)
        );
    }

    /// Paragraph control words for `props` (every one set in a style).
    fn paragraph_words(
        &mut self,
        props: &ParagraphProps,
        numbering: Option<(usize, u8)>,
        out: &mut String,
    ) {
        if let Some(alignment) = props.alignment {
            out.push_str(match alignment {
                Alignment::Left => "\\ql",
                Alignment::Right => "\\qr",
                Alignment::Centre => "\\qc",
                Alignment::Justify => "\\qj",
            });
        }
        if let Some(left) = props.indent_left {
            let _ = write!(out, "\\li{left}");
        }
        if let Some(right) = props.indent_right {
            let _ = write!(out, "\\ri{right}");
        }
        if let Some(first) = props.first_line {
            let _ = write!(out, "\\fi{first}");
        }
        if let Some(before) = props.space_before {
            let _ = write!(out, "\\sb{before}");
        }
        if let Some(after) = props.space_after {
            let _ = write!(out, "\\sa{after}");
        }
        match props.line {
            Some(LineSpacing::Single) | None => {}
            Some(LineSpacing::Multiple(multiple)) => {
                let _ = write!(out, "\\sl{}\\slmult1", (240.0 * multiple).round() as i32);
            }
            Some(LineSpacing::Exact(height)) => {
                let _ = write!(out, "\\sl-{height}\\slmult0");
            }
            Some(LineSpacing::AtLeast(height)) => {
                let _ = write!(out, "\\sl{height}\\slmult0");
            }
        }
        if props.keep_with_next == Some(true) {
            out.push_str("\\keepn");
        }
        if props.keep_together == Some(true) {
            out.push_str("\\keep");
        }
        if props.page_break_before == Some(true) {
            out.push_str("\\pagebb");
        }
        if props.no_hyphenation == Some(true) {
            out.push_str("\\hyphpar0");
        }
        for (line, side) in [(&props.border_top, "t"), (&props.border_bottom, "b")] {
            if let Some(Some(line)) = line {
                self.rule(line, side, out);
            }
        }
        if let Some(tabs) = &props.tabs {
            for tab in tabs {
                out.push_str(match tab.kind {
                    TabKind::Left => "",
                    TabKind::Centre => "\\tqc",
                    TabKind::Right => "\\tqr",
                    TabKind::Decimal => "\\tqdec",
                });
                out.push_str(match tab.leader {
                    TabLeader::None => "",
                    TabLeader::Dots => "\\tldot",
                    TabLeader::Hyphens => "\\tlhyph",
                    TabLeader::Line => "\\tlul",
                });
                let _ = write!(out, "\\tx{}", tab.position);
            }
        }
        if let Some((list, level)) = numbering {
            let _ = write!(out, "\\ls{list}\\ilvl{level}");
        }
    }

    fn rule(&mut self, line: &RuleLine, side: &str, out: &mut String) {
        let colour = self.colour(line.colour);
        let _ = write!(
            out,
            "\\brdr{side}{}\\brsp{}\\brdrcf{colour}",
            border_words(line.kind, line.width),
            line.space
        );
    }

    /// Character control words for `props`.
    fn char_words(&mut self, props: &CharProps) -> String {
        let mut out = String::new();
        if let Some(font) = &props.font {
            let index = self.font(font);
            let _ = write!(out, "\\f{index}");
        }
        if let Some(size) = props.size {
            let _ = write!(out, "\\fs{}", (size + 5) / 10);
        }
        // Only what is on: every run starts from `\\plain`, and Ami Pro's
        // own reader takes `\\b0` and the like for on (VM twin AF08).
        let flag = |out: &mut String, value: Option<bool>, word: &str| {
            if value == Some(true) {
                out.push_str(word);
            }
        };
        flag(&mut out, props.bold, "\\b");
        flag(&mut out, props.italic, "\\i");
        flag(&mut out, props.strike, "\\strike");
        flag(&mut out, props.caps, "\\caps");
        flag(&mut out, props.small_caps, "\\scaps");
        match props.underline {
            Some(Underline::Single) => out.push_str("\\ul"),
            Some(Underline::Words) => out.push_str("\\ulw"),
            Some(Underline::Double) => out.push_str("\\uldb"),
            Some(Underline::None) | None => {}
        }
        match props.script {
            Some(Script::Super) => out.push_str("\\super"),
            Some(Script::Sub) => out.push_str("\\sub"),
            Some(Script::Normal) | None => {}
        }
        if let Some(colour) = props.colour {
            let index = self.colour(colour);
            let _ = write!(out, "\\cf{index}");
        }
        out
    }

    // ---- Tables ----

    fn table(&mut self, table: &Table, out: &mut String) {
        let [top, right, bottom, left] = table.cell_margins;
        for row in &table.rows {
            let height = match row.height {
                RowHeight::AtLeast(height) => height,
                RowHeight::Exact(height) => -height,
            };
            let _ = write!(
                out,
                "\\trowd\\trgaph{}\\trleft{}\\trrh{height}\\trpaddl{left}\\trpaddt{top}\\trpaddr{right}\\trpaddb{bottom}\\trpaddfl3\\trpaddft3\\trpaddfr3\\trpaddfb3",
                (left + right) / 2,
                table.indent
            );
            match table.align {
                Alignment::Centre => out.push_str("\\trqc"),
                Alignment::Right => out.push_str("\\trqr"),
                _ => {}
            }
            if row.heading {
                out.push_str("\\trhdr");
            }
            if row.keep_together {
                out.push_str("\\trkeep");
            }
            let mut column = 0;
            let mut right_edge = table.indent;
            for cell in &row.cells {
                let span = usize::from(cell.span.max(1));
                right_edge += table.columns.iter().skip(column).take(span).sum::<Twips>();
                column += span;
                match cell.merge {
                    VerticalMerge::Start => out.push_str("\\clvmgf"),
                    VerticalMerge::Continue => out.push_str("\\clvmrg"),
                    VerticalMerge::None => {}
                }
                for (side, word) in [(0, "t"), (3, "l"), (2, "b"), (1, "r")] {
                    if let Some(line) = cell.borders[side] {
                        let colour = self.colour(line.colour);
                        let _ = write!(
                            out,
                            "\\clbrdr{word}{}\\brdrcf{colour}",
                            border_words(line.kind, line.width)
                        );
                    }
                }
                if let Some(fill) = cell.shading {
                    let colour = self.colour(fill);
                    let _ = write!(out, "\\clcbpat{colour}");
                }
                let _ = write!(out, "\\cellx{right_edge}");
            }
            out.push('\n');
            for (index, cell) in row.cells.iter().enumerate() {
                // A row starting a page: its first paragraph breaks before.
                if index == 0
                    && row.page_break_before
                    && let Some((Block::Paragraph(first), rest)) = cell.blocks.split_first()
                {
                    let mut first = first.clone();
                    first.props.page_break_before = Some(true);
                    let mut blocks = vec![Block::Paragraph(first)];
                    blocks.extend(rest.iter().cloned());
                    self.blocks(&blocks, true, out);
                    continue;
                }
                self.blocks(&cell.blocks, true, out);
            }
            out.push_str("\\row\n");
        }
        out.push_str("\\pard\\plain\n");
    }

    // ---- Frames ----

    fn shape(&mut self, shape: &Shape, out: &mut String) {
        let id = self.next_shape;
        self.next_shape += 1;
        // A picture flowing with the text is a picture in the line.
        if let (Anchor::Inline { .. }, ShapeContent::Picture(picture)) =
            (shape.anchor, &shape.content)
        {
            // At the size of the part its frame shows.
            let width = picture.width.min(shape.width - picture.left.max(0));
            let height = picture.height.min(shape.height - picture.top.max(0));
            if let Some(pict) = self.pict(picture.image, width.max(1), height.max(1)) {
                let _ = write!(out, "{{\\*\\shppict{pict}}}");
            }
            return;
        }
        let (horizontal, vertical, wrap, behind) = match shape.anchor {
            Anchor::Floating {
                horizontal,
                vertical,
                wrap,
                behind,
            } => (horizontal, vertical, wrap, behind),
            Anchor::Inline { .. } => (
                (HorizontalFrom::Column, 0),
                (VerticalFrom::Paragraph, 0),
                ShapeWrap::TopAndBottom,
                false,
            ),
        };
        let (bx, by) = (
            match horizontal.0 {
                HorizontalFrom::Page => "\\shpbxpage",
                HorizontalFrom::Margin => "\\shpbxmargin",
                HorizontalFrom::Column => "\\shpbxcolumn",
            },
            match vertical.0 {
                VerticalFrom::Page => "\\shpbypage",
                VerticalFrom::Margin => "\\shpbymargin",
                VerticalFrom::Paragraph => "\\shpbypara",
            },
        );
        let wrap_kind = match wrap {
            ShapeWrap::Square => 2,
            ShapeWrap::TopAndBottom => 1,
            ShapeWrap::None => 3,
        };
        let (left, top) = (horizontal.1, vertical.1);
        let _ = write!(
            out,
            "{{\\shp{{\\*\\shpinst\\shpleft{left}\\shptop{top}\\shpright{}\\shpbottom{}\\shpfhdr{}{bx}\\shpbxignore{by}\\shpbyignore\\shpwr{wrap_kind}\\shpwrk0\\shpfblwtxt{}\\shpz{}\\shplid{}",
            left + shape.width,
            top + shape.height,
            u8::from(self.in_header),
            u8::from(behind),
            shape.z,
            1024 + id
        );
        let property = |out: &mut String, name: &str, value: &str| {
            let _ = write!(out, "{{\\sp{{\\sn {name}}}{{\\sv {value}}}}}");
        };
        // Text boxes for text (LibreOffice shrinks a rounded rectangle
        // holding text to its text); rounded corners on empty frames.
        let kind = match (&shape.content, shape.corner_radius > 0) {
            (ShapeContent::Picture(_), _) => "75",
            (ShapeContent::Empty, true) => "2",
            (ShapeContent::Empty, false) => "1",
            (ShapeContent::Text { .. }, _) => "202",
        };
        property(out, "shapeType", kind);
        if !shape.name.is_empty() {
            property(out, "wzName", &text(&shape.name));
        }
        if shape.corner_radius > 0 && kind == "2" {
            let shorter = shape.width.min(shape.height).max(1);
            let adjust = i64::from(shape.corner_radius) * 21600 / i64::from(shorter);
            property(out, "adjustValue", &adjust.min(10800).to_string());
        }
        property(
            out,
            "posrelh",
            if horizontal.0 == HorizontalFrom::Page {
                "1"
            } else {
                "0"
            },
        );
        property(
            out,
            "posrelv",
            if vertical.0 == VerticalFrom::Page {
                "1"
            } else {
                "2"
            },
        );
        match shape.fill {
            Some(colour) => {
                property(out, "fFilled", "1");
                property(out, "fillColor", &bgr(colour).to_string());
            }
            None => property(out, "fFilled", "0"),
        }
        match shape.line {
            Some(line) => {
                property(out, "fLine", "1");
                property(out, "lineColor", &bgr(line.colour).to_string());
                property(out, "lineWidth", &(i64::from(line.width) * 635).to_string());
                let style = match line.kind {
                    LineKind::Single => "0",
                    LineKind::Double => "1",
                    LineKind::ThickThin => "2",
                    LineKind::ThinThick => "3",
                    LineKind::Triple => "4",
                };
                property(out, "lineStyle", style);
            }
            None => property(out, "fLine", "0"),
        }
        if let Some(shadow) = shape.shadow {
            property(out, "fShadow", "1");
            property(out, "shadowColor", &bgr(shadow.colour).to_string());
            property(
                out,
                "shadowOffsetX",
                &(i64::from(shadow.dx) * 635).to_string(),
            );
            property(
                out,
                "shadowOffsetY",
                &(i64::from(shadow.dy) * 635).to_string(),
            );
        }
        let mut result = String::new();
        match &shape.content {
            ShapeContent::Text { blocks, insets, .. } => {
                let [top, right, bottom, left] = *insets;
                for (name, value) in [
                    ("dxTextLeft", left),
                    ("dyTextTop", top),
                    ("dxTextRight", right),
                    ("dyTextBottom", bottom),
                ] {
                    property(out, name, &(i64::from(value) * 635).to_string());
                }
                let mut content = String::new();
                self.blocks(blocks, false, &mut content);
                let _ = write!(out, "{{\\shptxt {content}}}");
                // Older readers: a positioned paragraph holding the text.
                result.push_str(&self.positioned(shape, left_top(horizontal, vertical), &content));
            }
            ShapeContent::Picture(picture) => {
                if let Some(pict) = self.pict(picture.image, picture.width, picture.height) {
                    let _ = write!(out, "{{\\sp{{\\sn pib}}{{\\sv {pict}}}}}");
                    result.push_str(&self.positioned(
                        shape,
                        left_top(horizontal, vertical),
                        &format!("\\pard\\plain {pict}\\par\n"),
                    ));
                }
            }
            ShapeContent::Empty => {}
        }
        out.push('}');
        let _ = write!(out, "{{\\shprslt {result}}}}}");
    }

    /// A positioned paragraph (`\posx`, `\absw`) with `content`, as Word 2
    /// wrote frames.
    fn positioned(
        &self,
        shape: &Shape,
        (h, v, left, top): (&str, &str, Twips, Twips),
        content: &str,
    ) -> String {
        let frame = format!(
            "{h}{v}\\posx{left}\\posy{top}\\absw{}\\absh-{}\\dxfrtext0\\dfrmtxtx0\\dfrmtxty0",
            shape.width, shape.height
        );
        content.replace("\\pard\\plain", &format!("\\pard\\plain{frame}"))
    }

    /// A picture as `\pict`, `width` × `height` twips, or `None` for a kind
    /// RTF does not hold (and no fallback).
    fn pict(&self, index: usize, width: Twips, height: Twips) -> Option<String> {
        let image = self.document.images.get(index)?;
        let (word, bytes): (&str, &[u8]) = match image.kind {
            ImageKind::Png => ("\\pngblip", &image.bytes),
            ImageKind::Jpeg => ("\\jpegblip", &image.bytes),
            // A placeable metafile without its 22-byte header.
            ImageKind::Wmf if image.bytes.len() > 22 => ("\\wmetafile8", &image.bytes[22..]),
            ImageKind::Emf => ("\\emfblip", &image.bytes),
            // A packed bitmap: the file without its 14-byte header.
            ImageKind::Bmp if image.bytes.len() > 14 => ("\\dibitmap0", &image.bytes[14..]),
            _ => ("\\pngblip", image.fallback.as_deref()?),
        };
        let (pixel_width, pixel_height) = image
            .pixels
            .unwrap_or(((width / 15).max(1) as u32, (height / 15).max(1) as u32));
        let (picw, pich) = if matches!(image.kind, ImageKind::Wmf | ImageKind::Emf) {
            // HIMETRIC: hundredths of a millimetre.
            (
                i64::from(width) * 2540 / 1440,
                i64::from(height) * 2540 / 1440,
            )
        } else {
            (i64::from(pixel_width), i64::from(pixel_height))
        };
        let mut out =
            format!("{{\\pict{word}\\picw{picw}\\pich{pich}\\picwgoal{width}\\pichgoal{height}\n");
        for chunk in bytes.chunks(64) {
            for byte in chunk {
                let _ = write!(out, "{byte:02x}");
            }
            out.push('\n');
        }
        out.push('}');
        Some(out)
    }
}

fn left_top(
    horizontal: (HorizontalFrom, Twips),
    vertical: (VerticalFrom, Twips),
) -> (&'static str, &'static str, Twips, Twips) {
    (
        match horizontal.0 {
            HorizontalFrom::Page => "\\phpg",
            HorizontalFrom::Margin => "\\phmrg",
            HorizontalFrom::Column => "\\phcol",
        },
        match vertical.0 {
            VerticalFrom::Page => "\\pvpg",
            VerticalFrom::Margin => "\\pvmrg",
            VerticalFrom::Paragraph => "\\pvpara",
        },
        horizontal.1,
        vertical.1,
    )
}

/// What a paragraph sets put over its style's formatting.
fn overlay_paragraph(base: &mut ParagraphProps, over: &ParagraphProps) {
    fn take<T: Clone>(target: &mut Option<T>, value: &Option<T>) {
        if value.is_some() {
            target.clone_from(value);
        }
    }
    take(&mut base.alignment, &over.alignment);
    take(&mut base.indent_left, &over.indent_left);
    take(&mut base.indent_right, &over.indent_right);
    take(&mut base.first_line, &over.first_line);
    take(&mut base.space_before, &over.space_before);
    take(&mut base.space_after, &over.space_after);
    take(&mut base.line, &over.line);
    take(&mut base.keep_with_next, &over.keep_with_next);
    take(&mut base.keep_together, &over.keep_together);
    take(&mut base.page_break_before, &over.page_break_before);
    take(&mut base.column_break_before, &over.column_break_before);
    take(&mut base.tabs, &over.tabs);
    take(&mut base.border_top, &over.border_top);
    take(&mut base.border_bottom, &over.border_bottom);
    take(&mut base.no_hyphenation, &over.no_hyphenation);
}

/// A list level's text as RTF's `\leveltext` (its length, then the text
/// with each number as its level's index) and `\levelnumbers` (where the
/// numbers stand, 1-based).
fn level_text(template: &str) -> (String, String) {
    let mut characters: Vec<String> = Vec::new();
    let mut positions = Vec::new();
    let mut chars = template.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '%' {
            match chars.next() {
                Some('%') => characters.push(text("%")),
                Some(digit @ '1'..='9') => {
                    characters.push(format!("\\'{:02x}", digit as u8 - b'1'));
                    positions.push(characters.len());
                }
                _ => {}
            }
        } else {
            characters.push(text(&character.to_string()));
        }
    }
    let mut leveltext = format!("\\'{:02x}", characters.len().min(255));
    for character in &characters {
        leveltext.push_str(character);
    }
    let numbers = positions
        .iter()
        .map(|position| format!("\\'{position:02x}"))
        .collect();
    (leveltext, numbers)
}

/// `value` as RTF text: control characters escaped, characters past ASCII
/// as `\uN?`.
fn text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\t' => out.push_str("\\tab "),
            '\u{AD}' => out.push_str("\\-"),
            '\u{A0}' => out.push_str("\\~"),
            '\n' => out.push_str("\\line "),
            ' '..='~' => out.push(character),
            _ if (character as u32) < 0x20 => {}
            _ => {
                let code = character as u32;
                if code > 0xFFFF {
                    // Outside the basic plane: its UTF-16 pair.
                    let mut units = [0u16; 2];
                    for unit in character.encode_utf16(&mut units) {
                        let _ = write!(out, "\\u{}?", *unit as i16);
                    }
                } else {
                    let _ = write!(out, "\\u{}?", code as u16 as i16);
                }
            }
        }
    }
    out
}

/// A border's kind and width as RTF words: a single line wider than RTF's
/// 75 twips as a thick line of half the width (`\\brdrth` doubles it).
fn border_words(kind: LineKind, width: Twips) -> String {
    if kind == LineKind::Single && width > 75 {
        return format!("\\brdrth\\brdrw{}", (width / 2).clamp(1, 75));
    }
    format!("{}\\brdrw{}", border_word(kind), width.clamp(1, 75))
}

fn border_word(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Single => "\\brdrs",
        LineKind::Double => "\\brdrdb",
        LineKind::Triple => "\\brdrtriple",
        LineKind::ThickThin => "\\brdrthtnsg",
        LineKind::ThinThick => "\\brdrtnthsg",
    }
}

/// A colour as RTF shapes take it: `0x00BBGGRR`, which is Ami Pro's own.
fn bgr(colour: crate::Colour) -> u32 {
    colour
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_escapes_and_writes_unicode() {
        assert_eq!(text("a{b}\\c é ™"), "a\\{b\\}\\\\c \\u233? \\u8482?");
    }

    #[test]
    fn level_text_counts_and_places_numbers() {
        let (leveltext, numbers) = level_text("%1.%2.");
        assert_eq!(leveltext, "\\'04\\'00.\\'01.");
        assert_eq!(numbers, "\\'01\\'03");
    }
}
