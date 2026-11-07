//! Markdown read as CommonMark with GitHub's tables, strikethrough, task
//! lists, and footnotes (MD2; `pulldown-cmark`).
//!
//! Headings become the styles "Heading 1" to "Heading 6" with outline
//! levels; paragraphs Body Text; quotes the style "Quote" (nested quotes
//! indent), code blocks "Code" (a paragraph a line); a rule is a paragraph
//! with a line below it. Lists are the exchange lists (bullets, numbers,
//! nested to nine levels), task boxes the characters ☐ and ☑. Tables are
//! tables, footnotes footnotes. A link keeps its text and a picture its
//! description (the file is not read: a Markdown file names its pictures),
//! both listed in the findings; HTML stays as text. The file is UTF-8,
//! with or without a byte order mark.
//!
//! The reader is one pass over the parser's events with counters for the
//! open quotes and lists, so no input nests deeper than the stack.

use std::collections::HashMap;

use pulldown_cmark::{Alignment as MdAlignment, Event, Options, Parser, Tag, TagEnd};

use crate::import::{FindingKind, Findings};
use crate::{
    Alignment, Block, Cell, CharProps, ExchangeDocument, Font, FontFamily, Footnote, Inline,
    LineKind, List, ListInstance, ListLevel, NumberFormat, Paragraph, ParagraphProps, Row,
    RowHeight, RuleLine, Section, Style, Table, VerticalMerge,
};

/// The font of code.
pub(crate) const MONOSPACE: &str = "Courier New";
/// The text width of the Letter page the file is given, in twips.
const TEXT_WIDTH: i32 = 9360;
/// How far a quote or a list level indents, in twips.
const STEP: i32 = 720;
/// Deepest quote or list level kept apart.
const MAX_DEPTH: usize = 8;

/// `bytes` as an exchange document. Never fails: what is not UTF-8 is read
/// with replacement characters, and the findings say so.
pub fn read_markdown(bytes: &[u8], findings: &mut Findings) -> ExchangeDocument {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let bad = bytes
        .utf8_chunks()
        .filter(|chunk| !chunk.invalid().is_empty())
        .count();
    findings.add(FindingKind::BadCharacters, bad as u32, None);
    let text = String::from_utf8_lossy(bytes);
    let mut options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    options.remove(Options::ENABLE_SMART_PUNCTUATION);
    let events: Vec<Event> = Parser::new_ext(&text, options).collect();

    let mut reader = Reader::new(findings);
    // Footnote texts first: a reference may come before its definition.
    let mut index = 0;
    while index < events.len() {
        if let Event::Start(Tag::FootnoteDefinition(label)) = &events[index] {
            let end = matching_end(&events, index);
            let label = label.to_string();
            let blocks = reader.sub_run(&events[index + 1..end]);
            reader.footnotes.entry(label).or_insert(blocks);
            index = end + 1;
        } else {
            index += 1;
        }
    }
    let mut blocks = reader.run(&events);
    if blocks.is_empty() {
        blocks.push(Block::Paragraph(Paragraph::default()));
    }
    reader.finish(blocks)
}

/// The index of the end event that closes the start at `start`.
fn matching_end(events: &[Event], start: usize) -> usize {
    let mut depth = 0usize;
    for (index, event) in events.iter().enumerate().skip(start) {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return index;
                }
            }
            _ => {}
        }
    }
    events.len().saturating_sub(1)
}

struct TableState {
    aligns: Vec<MdAlignment>,
    rows: Vec<Row>,
    cells: Vec<Cell>,
    head: bool,
}

struct Reader<'f> {
    findings: &'f mut Findings,
    styles: Vec<Style>,
    style_index: HashMap<String, usize>,
    fonts: Vec<Font>,
    lists: Vec<List>,
    instances: Vec<ListInstance>,
    footnotes: HashMap<String, Vec<Block>>,
    // The run's state.
    cur: Vec<Inline>,
    bold: u32,
    italic: u32,
    strike: u32,
    quote: usize,
    /// The open lists: instance and level.
    open_lists: Vec<(usize, usize)>,
    /// The next paragraph is the item's first: it carries the list.
    item_marker: Option<(usize, u8)>,
    in_item: u32,
    in_code: bool,
    in_html: bool,
    /// The open link: its address and the text seen inside it (the model has
    /// no hyperlink, so the address is kept as text after the link's text).
    link: Option<(String, String)>,
    table: Option<TableState>,
}

impl<'f> Reader<'f> {
    fn new(findings: &'f mut Findings) -> Self {
        let mut reader = Self {
            findings,
            styles: Vec::new(),
            style_index: HashMap::new(),
            fonts: Vec::new(),
            lists: Vec::new(),
            instances: Vec::new(),
            footnotes: HashMap::new(),
            cur: Vec::new(),
            bold: 0,
            italic: 0,
            strike: 0,
            quote: 0,
            open_lists: Vec::new(),
            item_marker: None,
            in_item: 0,
            in_code: false,
            in_html: false,
            link: None,
            table: None,
        };
        reader.style("Body Text");
        reader.lists = vec![list(NumberFormat::Bullet), list(NumberFormat::Decimal)];
        reader
    }

    fn finish(self, blocks: Vec<Block>) -> ExchangeDocument {
        ExchangeDocument {
            fonts: self.fonts,
            styles: self.styles,
            lists: self.lists,
            list_instances: self.instances,
            sections: vec![Section {
                page: crate::read_text::letter(),
                new_page: false,
                headers: Default::default(),
                footers: Default::default(),
                page_numbers: None,
                blocks,
            }],
            ..ExchangeDocument::default()
        }
    }

    /// The style `name`, made on first use.
    fn style(&mut self, name: &str) -> usize {
        if let Some(index) = self.style_index.get(name) {
            return *index;
        }
        let mut style = Style {
            name: name.to_owned(),
            based_on: None,
            next: None,
            paragraph: ParagraphProps::default(),
            character: CharProps::default(),
            outline_level: None,
            list: None,
        };
        if let Some(level) = name
            .strip_prefix("Heading ")
            .and_then(|level| level.parse::<u8>().ok())
        {
            let points = [28, 22, 18, 15, 13, 12][usize::from(level.clamp(1, 6)) - 1];
            style.outline_level = Some(level);
            style.character.bold = Some(true);
            style.character.size = Some(points * 20);
            style.paragraph.space_before = Some(240);
            style.paragraph.space_after = Some(120);
            style.paragraph.keep_with_next = Some(true);
            style.next = Some(0);
        } else if name == "Quote" {
            style.paragraph.indent_left = Some(STEP);
        } else if name == "Code" {
            style.character.font = Some(MONOSPACE.to_owned());
            style.character.size = Some(18 * 20);
            self.mono_font();
        }
        self.styles.push(style);
        self.style_index
            .insert(name.to_owned(), self.styles.len() - 1);
        self.styles.len() - 1
    }

    fn mono_font(&mut self) {
        if !self.fonts.iter().any(|font| font.name == MONOSPACE) {
            self.fonts.push(Font {
                name: MONOSPACE.to_owned(),
                family: FontFamily::Modern,
                fixed: true,
            });
        }
    }

    /// A new instance of list 0 (bullets) or 1 (numbers).
    fn instance(&mut self, ordered: bool, level: usize, start: Option<u64>) -> usize {
        let mut starts = [None; 9];
        if let Some(start) = start
            && start != 1
        {
            starts[level.min(8)] = Some(start.min(u64::from(u32::MAX)) as u32);
        }
        self.instances.push(ListInstance {
            list: usize::from(ordered),
            starts,
        });
        self.instances.len() - 1
    }

    fn props(&self) -> CharProps {
        CharProps {
            bold: (self.bold > 0).then_some(true),
            italic: (self.italic > 0).then_some(true),
            strike: (self.strike > 0).then_some(true),
            ..CharProps::default()
        }
    }

    fn push_text(&mut self, text: &str, props: CharProps) {
        let mut run = String::new();
        for character in text.chars() {
            match character {
                '\t' => {
                    self.flush_run(&mut run, &props);
                    self.cur.push(Inline::Tab {
                        props: props.clone(),
                    });
                }
                character if character.is_control() => {}
                character => run.push(character),
            }
        }
        self.flush_run(&mut run, &props);
    }

    fn flush_run(&mut self, run: &mut String, props: &CharProps) {
        if run.is_empty() {
            return;
        }
        if let Some(Inline::Text {
            text,
            props: previous,
        }) = self.cur.last_mut()
            && previous == props
        {
            text.push_str(run);
            run.clear();
            return;
        }
        self.cur.push(Inline::Text {
            text: std::mem::take(run),
            props: props.clone(),
        });
    }

    /// The pending inlines as a paragraph in `style` (nothing when empty
    /// and not `force`d); the first of an item carries the item's list.
    fn emit(&mut self, style: Option<usize>, force: bool, out: &mut Vec<Block>) {
        if self.cur.is_empty() && !force {
            return;
        }
        let inlines = std::mem::take(&mut self.cur);
        let style = style.unwrap_or_else(|| {
            if self.quote > 0 {
                self.style("Quote")
            } else {
                0
            }
        });
        let mut props = ParagraphProps::default();
        if self.quote > 0 {
            props.indent_left = Some(self.quote.min(MAX_DEPTH) as i32 * STEP);
        }
        let list = self
            .item_marker
            .take()
            .map(|(instance, level)| (Some(instance), level));
        out.push(Block::Paragraph(Paragraph {
            style,
            props,
            list,
            inlines,
            ..Paragraph::default()
        }));
    }

    fn sub_run(&mut self, events: &[Event]) -> Vec<Block> {
        let saved = (
            std::mem::take(&mut self.cur),
            self.quote,
            std::mem::take(&mut self.open_lists),
            self.item_marker.take(),
            self.in_item,
            self.table.take(),
        );
        self.quote = 0;
        self.in_item = 0;
        let blocks = self.run(events);
        (
            self.cur,
            self.quote,
            self.open_lists,
            self.item_marker,
            self.in_item,
            self.table,
        ) = saved;
        blocks
    }

    fn run(&mut self, events: &[Event]) -> Vec<Block> {
        let mut out = Vec::new();
        let mut index = 0;
        while index < events.len() {
            let event = &events[index];
            index += 1;
            match event {
                Event::Start(tag) => match tag {
                    Tag::Paragraph => self.emit(None, false, &mut out),
                    Tag::Heading { level, .. } => {
                        self.emit(None, false, &mut out);
                        // Headings keep their own look: bold is the style's.
                        self.bold = 0;
                        let _ = level;
                    }
                    Tag::BlockQuote(_) => {
                        self.emit(None, false, &mut out);
                        self.quote += 1;
                    }
                    Tag::CodeBlock(_) => {
                        self.emit(None, false, &mut out);
                        self.in_code = true;
                    }
                    Tag::HtmlBlock => {
                        self.emit(None, false, &mut out);
                        self.in_html = true;
                        self.findings.count(FindingKind::Html);
                    }
                    Tag::List(start) => {
                        self.emit(None, false, &mut out);
                        let level = self.open_lists.len().min(MAX_DEPTH);
                        let instance = self.instance(start.is_some(), level, *start);
                        self.open_lists.push((instance, level));
                    }
                    Tag::Item => {
                        self.emit(None, false, &mut out);
                        self.in_item += 1;
                        if let Some((instance, level)) = self.open_lists.last() {
                            self.item_marker = Some((*instance, *level as u8));
                        }
                    }
                    Tag::FootnoteDefinition(_) => {
                        index = matching_end(events, index - 1) + 1;
                    }
                    Tag::Table(aligns) => {
                        self.emit(None, false, &mut out);
                        self.table = Some(TableState {
                            aligns: aligns.clone(),
                            rows: Vec::new(),
                            cells: Vec::new(),
                            head: false,
                        });
                    }
                    Tag::TableHead => {
                        if let Some(table) = &mut self.table {
                            table.head = true;
                            table.cells.clear();
                        }
                    }
                    Tag::TableRow => {
                        if let Some(table) = &mut self.table {
                            table.cells.clear();
                        }
                    }
                    Tag::TableCell => self.cur.clear(),
                    Tag::Emphasis => self.italic += 1,
                    Tag::Strong => self.bold += 1,
                    Tag::Strikethrough => self.strike += 1,
                    Tag::Link { dest_url, .. } => {
                        self.findings.count(FindingKind::Links);
                        self.link = Some((dest_url.to_string(), String::new()));
                    }
                    Tag::Image { .. } => self.findings.count(FindingKind::LinkedPictures),
                    _ => {}
                },
                Event::End(end) => match end {
                    TagEnd::Paragraph => self.emit(None, false, &mut out),
                    TagEnd::Heading(level) => {
                        let number = *level as usize;
                        let style = self.style(&format!("Heading {number}"));
                        self.emit(Some(style), true, &mut out);
                    }
                    TagEnd::BlockQuote(_) => {
                        self.emit(None, false, &mut out);
                        self.quote = self.quote.saturating_sub(1);
                    }
                    TagEnd::CodeBlock => {
                        self.in_code = false;
                    }
                    TagEnd::HtmlBlock => {
                        self.in_html = false;
                        self.emit(None, false, &mut out);
                    }
                    TagEnd::List(_) => {
                        self.emit(None, false, &mut out);
                        self.open_lists.pop();
                    }
                    TagEnd::Item => {
                        // An empty item still shows its marker.
                        let force = self.item_marker.is_some();
                        self.emit(None, force, &mut out);
                        self.in_item = self.in_item.saturating_sub(1);
                    }
                    TagEnd::Link => {
                        // "text (address)", unless the text is the address.
                        if let Some((address, text)) = self.link.take()
                            && !address.is_empty()
                            && text != address
                            && text.trim_start_matches("mailto:")
                                != address.trim_start_matches("mailto:")
                        {
                            let props = self.props();
                            self.push_text(&format!(" ({address})"), props);
                        }
                    }
                    TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
                    TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
                    TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
                    TagEnd::TableCell => self.end_cell(),
                    TagEnd::TableHead => self.end_row(true),
                    TagEnd::TableRow => self.end_row(false),
                    TagEnd::Table => {
                        if let Some(table) = self.table.take() {
                            out.push(Block::Table(finish_table(table)));
                        }
                    }
                    _ => {}
                },
                Event::Text(text) => {
                    if self.in_code {
                        self.code_text(text, &mut out);
                    } else if self.in_html {
                        self.html_text(text);
                    } else {
                        let props = self.props();
                        if let Some((_, seen)) = &mut self.link {
                            seen.push_str(text);
                        }
                        self.push_text(text, props);
                    }
                }
                Event::Code(text) => {
                    self.mono_font();
                    let mut props = self.props();
                    props.font = Some(MONOSPACE.to_owned());
                    self.push_text(text, props);
                }
                Event::InlineMath(text) | Event::DisplayMath(text) => {
                    let props = self.props();
                    self.push_text(text, props);
                }
                Event::Html(text) => {
                    if self.in_html {
                        self.html_text(text);
                    } else {
                        self.findings.count(FindingKind::Html);
                        let props = self.props();
                        self.push_text(text.trim_end_matches('\n'), props);
                    }
                }
                Event::InlineHtml(text) => {
                    self.findings.count(FindingKind::Html);
                    let props = self.props();
                    self.push_text(text, props);
                }
                Event::FootnoteReference(label) => {
                    let blocks = self
                        .footnotes
                        .get(label.as_ref())
                        .cloned()
                        .unwrap_or_else(|| {
                            vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text {
                                    text: label.to_string(),
                                    props: CharProps::default(),
                                }],
                                ..Paragraph::default()
                            })]
                        });
                    self.cur.push(Inline::Footnote(Box::new(Footnote {
                        endnote: false,
                        blocks,
                        props: CharProps::default(),
                    })));
                }
                Event::SoftBreak => {
                    let props = self.props();
                    self.push_text(" ", props);
                }
                Event::HardBreak => self.cur.push(Inline::LineBreak),
                Event::Rule => {
                    self.emit(None, false, &mut out);
                    out.push(Block::Paragraph(Paragraph {
                        props: ParagraphProps {
                            border_bottom: Some(Some(RuleLine {
                                kind: LineKind::Single,
                                width: 15,
                                colour: 0,
                                space: 60,
                            })),
                            ..ParagraphProps::default()
                        },
                        ..Paragraph::default()
                    }));
                }
                Event::TaskListMarker(done) => {
                    let props = self.props();
                    self.push_text(if *done { "\u{2611} " } else { "\u{2610} " }, props);
                }
            }
        }
        self.emit(None, false, &mut out);
        out
    }

    /// Code block text: a paragraph a line (the parser gives whole lines).
    fn code_text(&mut self, text: &str, out: &mut Vec<Block>) {
        let style = self.style("Code");
        for line in text.split_inclusive('\n') {
            self.push_text(line.trim_end_matches('\n'), CharProps::default());
            self.emit(Some(style), true, out);
        }
    }

    fn html_text(&mut self, text: &str) {
        let props = self.props();
        let mut lines = text.split('\n').peekable();
        while let Some(line) = lines.next() {
            self.push_text(line, props.clone());
            if lines.peek().is_some_and(|rest| !rest.is_empty()) {
                self.cur.push(Inline::LineBreak);
            }
        }
    }

    fn end_cell(&mut self) {
        let Some(table) = &mut self.table else {
            return;
        };
        let column = table.cells.len();
        let alignment = match table.aligns.get(column) {
            Some(MdAlignment::Left) => Some(Alignment::Left),
            Some(MdAlignment::Center) => Some(Alignment::Centre),
            Some(MdAlignment::Right) => Some(Alignment::Right),
            _ => None,
        };
        let line = RuleLine {
            kind: LineKind::Single,
            width: 15,
            colour: 0,
            space: 0,
        };
        table.cells.push(Cell {
            span: 1,
            merge: VerticalMerge::None,
            blocks: vec![Block::Paragraph(Paragraph {
                props: ParagraphProps {
                    alignment,
                    ..ParagraphProps::default()
                },
                inlines: std::mem::take(&mut self.cur),
                ..Paragraph::default()
            })],
            borders: [Some(line); 4],
            shading: None,
        });
    }

    fn end_row(&mut self, head: bool) {
        if let Some(table) = &mut self.table {
            let cells = std::mem::take(&mut table.cells);
            table.rows.push(Row {
                height: RowHeight::AtLeast(0),
                heading: head,
                keep_together: false,
                page_break_before: false,
                cells,
            });
            if head {
                table.head = false;
            }
        }
    }
}

fn finish_table(table: TableState) -> Table {
    let columns = table
        .rows
        .iter()
        .map(|row| row.cells.len())
        .max()
        .unwrap_or(1)
        .max(1);
    Table {
        columns: vec![TEXT_WIDTH / columns as i32; columns],
        rows: table.rows,
        indent: 0,
        align: Alignment::Left,
        cell_margins: [0, 108, 0, 108],
    }
}

/// A list of nine levels of one format.
fn list(format: NumberFormat) -> List {
    List {
        levels: (0..9)
            .map(|level| ListLevel {
                format,
                text: if format == NumberFormat::Bullet {
                    "\u{2022}".to_owned()
                } else {
                    format!("%{}.", level + 1)
                },
                start: 1,
                restart_after_higher: true,
                font: None,
                props: CharProps::default(),
                right_align: false,
                indent_left: STEP * (level + 1),
                hanging: STEP / 2,
            })
            .collect(),
    }
}
