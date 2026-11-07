//! The Markdown filter (MD2): CommonMark with GitHub's tables,
//! strikethrough, task boxes, and footnotes, as the exchange document's
//! main text in order.
//!
//! Headings are paragraphs whose style has an outline level (`#` to
//! `######`; deeper levels stop at six), lists the paragraphs with a list
//! (nested four spaces a level), quotes and code the styles named "Quote"
//! and "Code", a rule a paragraph with a line and no text. Bold, italic,
//! strikethrough, and fixed-pitch runs keep their marks; everything else of
//! a run's look, and every paragraph's, is left out (`losses::always_lost`).
//! Pictures are written beside the file and linked by relative name
//! ([`Markdown::pictures`]); a frame's text and picture follow the
//! paragraph that anchors them, as in plain text. Headers and footers are
//! left out. Links are not in the exchange document, so none is written.

use std::fmt::Write as _;
use std::sync::Arc;

use crate::{
    Alignment, Block, ExchangeDocument, ImageKind, Inline, NumberFormat, Paragraph, ShapeContent,
    Style, Table, VerticalMerge,
};

/// A Markdown file and the pictures to write beside it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Markdown {
    pub text: String,
    /// File name (no folder) and bytes of each picture the text links.
    pub pictures: Vec<(String, Arc<[u8]>)>,
}

/// The document as Markdown; its pictures are named `{stem}-1.png` and so
/// on (`stem` is the document's file name without the extension).
pub fn write(document: &ExchangeDocument, stem: &str) -> Markdown {
    let mut writer = Writer {
        document,
        stem: encode_path(stem),
        pictures: Vec::new(),
        names: vec![None; document.images.len()],
        footnotes: Vec::new(),
        pieces: Vec::new(),
        counters: Vec::new(),
        in_footnote: false,
    };
    for section in &document.sections {
        writer.blocks(&section.blocks);
    }
    let mut text = writer.assemble();
    if !writer.footnotes.is_empty() {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        for (number, body) in writer.footnotes.iter().enumerate() {
            let _ = write!(text, "[^{}]: ", number + 1);
            text.push_str(body.replace("\n\n", "\n\n    ").trim_end());
            text.push_str("\n\n");
        }
        while text.ends_with("\n\n") {
            text.pop();
        }
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    Markdown {
        text,
        pictures: writer.pictures,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Paragraph,
    Heading,
    Item,
    Code,
    Quote(usize),
    Rule,
    Table,
}

struct ListItem {
    ordered: bool,
    level: u8,
    /// The number the list starts from.
    start: u32,
    instance: Option<usize>,
}

/// An open list level: the number of its last item.
struct Counter {
    ordered: bool,
    instance: Option<usize>,
    last: u32,
}

struct Piece {
    kind: Kind,
    /// An item that starts a new list (not the one before's).
    new_list: bool,
    text: String,
}

struct Writer<'a> {
    document: &'a ExchangeDocument,
    stem: String,
    pictures: Vec<(String, Arc<[u8]>)>,
    /// The file name of each image already written.
    names: Vec<Option<String>>,
    /// Footnote bodies, in reference order.
    footnotes: Vec<String>,
    pieces: Vec<Piece>,
    /// The open list levels.
    counters: Vec<Counter>,
    in_footnote: bool,
}

impl Writer<'_> {
    fn blocks(&mut self, blocks: &[Block]) {
        for block in blocks {
            match block {
                Block::Paragraph(paragraph) => self.paragraph(paragraph),
                Block::Table(table) => {
                    self.counters.clear();
                    let text = self.table(table);
                    if !text.is_empty() {
                        self.pieces.push(Piece {
                            kind: Kind::Table,
                            new_list: false,
                            text,
                        });
                    }
                }
            }
        }
    }

    fn paragraph(&mut self, paragraph: &Paragraph) {
        let mut frames = Vec::new();
        let text = self.inlines(&paragraph.inlines, &mut frames);
        let style = self.document.styles.get(paragraph.style);
        let name = style.map_or("", |style| style.name.as_str());
        let blank = text.trim().is_empty();
        let ruled = matches!(paragraph.props.border_top, Some(Some(_)))
            || matches!(paragraph.props.border_bottom, Some(Some(_)));
        if blank && ruled && frames.is_empty() {
            self.counters.clear();
            self.pieces.push(Piece {
                kind: Kind::Rule,
                new_list: false,
                text: "---".to_owned(),
            });
        } else if is_code(name) {
            self.counters.clear();
            let line = code_line(paragraph, &text);
            match self.pieces.last_mut() {
                Some(Piece {
                    kind: Kind::Code,
                    new_list: false,
                    text,
                }) => {
                    // Keep the closing fence for the end.
                    text.push('\n');
                    text.push_str(&line);
                }
                _ => self.pieces.push(Piece {
                    kind: Kind::Code,
                    new_list: false,
                    text: line,
                }),
            }
        } else if let Some(level) = self.heading_level(style) {
            self.counters.clear();
            if !blank {
                let line = text.replace("\\\n", " ").replace('\n', " ");
                self.pieces.push(Piece {
                    kind: Kind::Heading,
                    new_list: false,
                    text: format!("{} {}", "#".repeat(usize::from(level.min(6))), line.trim()),
                });
            }
        } else if let Some(item) = self.list_of(paragraph, style) {
            let (marker, indent, new_list) = self.item_marker(&item);
            let mut text = text;
            // A task box written as the characters is GitHub's box.
            let rest = text
                .strip_prefix('\u{2610}')
                .map(|rest| ("[ ] ", rest))
                .or_else(|| text.strip_prefix('\u{2611}').map(|rest| ("[x] ", rest)));
            let task = rest.map(|(box_, rest)| (box_, rest.trim_start().to_owned()));
            let mut line = format!("{}{marker}", " ".repeat(indent));
            let continuation = " ".repeat(indent + marker.len());
            if let Some((box_, rest)) = task {
                line.push_str(box_);
                text = rest;
            } else {
                text = escape_line_start(&text);
            }
            let text = text.replace('\n', &format!("\n{continuation}"));
            line.push_str(&text);
            self.pieces.push(Piece {
                kind: Kind::Item,
                new_list,
                text: line,
            });
        } else if !blank {
            self.counters.clear();
            if is_quote(name) {
                let depth = paragraph
                    .props
                    .indent_left
                    .map_or(1, |indent| (indent / 720).clamp(1, 8) as usize);
                let prefix = "> ".repeat(depth);
                let text = escape_line_start(&text);
                let text = text.replace('\n', &format!("\n{prefix}"));
                self.pieces.push(Piece {
                    kind: Kind::Quote(depth),
                    new_list: false,
                    text: format!("{prefix}{text}"),
                });
            } else {
                self.pieces.push(Piece {
                    kind: Kind::Paragraph,
                    new_list: false,
                    text: escape_line_start(&text),
                });
            }
        }
        for frame in frames {
            if let ShapeContent::Text { blocks, .. } = frame {
                self.counters.clear();
                self.blocks(&blocks);
            }
        }
    }

    /// The outline level of `style` (its own or a style it is based on), or
    /// the number in a name "Heading 3".
    fn heading_level(&self, style: Option<&Style>) -> Option<u8> {
        let mut current = style;
        for _ in 0..16 {
            let style = current?;
            if let Some(level) = style.outline_level {
                return Some(level.max(1));
            }
            current = style
                .based_on
                .and_then(|index| self.document.styles.get(index));
        }
        let style = style?;
        style
            .name
            .strip_prefix("Heading ")
            .and_then(|level| level.parse().ok())
    }

    /// Whether the paragraph is a list item, and which.
    fn list_of(&self, paragraph: &Paragraph, style: Option<&Style>) -> Option<ListItem> {
        let from_list = |list: usize, level: u8, instance: Option<usize>| {
            let list = self.document.lists.get(list)?;
            let level_def = list.levels.get(usize::from(level.min(8)))?;
            let ordered = !matches!(level_def.format, NumberFormat::Bullet | NumberFormat::None);
            let start = instance
                .and_then(|instance| self.document.list_instances.get(instance))
                .and_then(|instance| instance.starts[usize::from(level.min(8))])
                .unwrap_or(level_def.start);
            Some(ListItem {
                ordered,
                level,
                start,
                instance,
            })
        };
        match paragraph.list {
            Some((Some(instance), level)) => {
                let list = self.document.list_instances.get(instance)?.list;
                from_list(list, level, Some(instance))
            }
            Some((None, level)) => {
                let ordered = paragraph
                    .number_text
                    .as_deref()
                    .is_some_and(|text| text.chars().any(char::is_alphanumeric));
                Some(ListItem {
                    ordered,
                    level,
                    start: 1,
                    instance: None,
                })
            }
            None => {
                let (list, level) = style.and_then(|style| style.list)?;
                from_list(list, level, None)
            }
        }
    }

    /// The marker of the next item and the indent it takes, counting the
    /// numbers of the open levels.
    fn item_marker(&mut self, item: &ListItem) -> (String, usize, bool) {
        // A level deeper than the one before is one level deeper only.
        let level = usize::from(item.level).min(self.counters.len()).min(8);
        self.counters.truncate(level + 1);
        let mut new_list = false;
        let number = match self.counters.get_mut(level) {
            Some(open) if open.ordered == item.ordered && open.instance == item.instance => {
                open.last += 1;
                open.last
            }
            Some(open) => {
                new_list = level == 0;
                *open = Counter {
                    ordered: item.ordered,
                    instance: item.instance,
                    last: item.start,
                };
                item.start
            }
            None => {
                self.counters.push(Counter {
                    ordered: item.ordered,
                    instance: item.instance,
                    last: item.start,
                });
                item.start
            }
        };
        let marker = if item.ordered {
            format!("{number}. ")
        } else {
            "- ".to_owned()
        };
        (marker, level * 4, new_list)
    }

    fn assemble(&self) -> String {
        let mut out = String::new();
        let mut previous: Option<Kind> = None;
        for piece in &self.pieces {
            if let Some(last) = previous {
                match (last, piece.kind) {
                    (Kind::Item, Kind::Item) if !piece.new_list => out.push('\n'),
                    (Kind::Quote(a), Kind::Quote(b)) => {
                        out.push('\n');
                        out.push_str("> ".repeat(a.min(b)).trim_end());
                        out.push('\n');
                    }
                    _ => out.push_str("\n\n"),
                }
            }
            if piece.kind == Kind::Code {
                let longest = longest_run(&piece.text, '`').max(2);
                let fence = "`".repeat(longest + 1);
                let _ = write!(out, "{fence}\n{}\n{fence}", piece.text);
            } else {
                out.push_str(&piece.text);
            }
            previous = Some(piece.kind);
        }
        out
    }

    /// The text of `inlines`; pictures and frames' text are linked or
    /// handed back in `frames`.
    fn inlines(&mut self, inlines: &[Inline], frames: &mut Vec<ShapeContent>) -> String {
        // Runs of one look first, so the marks hug whole runs.
        enum Atom {
            Run(String, Look),
            Raw(String),
        }
        let mut atoms: Vec<Atom> = Vec::new();
        let push = |atoms: &mut Vec<Atom>, text: String, look: Look| {
            if let Some(Atom::Run(last, last_look)) = atoms.last_mut()
                && *last_look == look
            {
                last.push_str(&text);
                return;
            }
            atoms.push(Atom::Run(text, look));
        };
        for inline in inlines {
            match inline {
                Inline::Text { text, props } => {
                    let look = Look::of(props, self.document);
                    let text: String = text.chars().filter(|c| *c != '\u{AD}').collect();
                    push(&mut atoms, text, look);
                }
                Inline::Tab { props } => {
                    push(&mut atoms, "\t".to_owned(), Look::of(props, self.document))
                }
                Inline::LineBreak => atoms.push(Atom::Raw("\\\n".to_owned())),
                Inline::PageBreak | Inline::ColumnBreak => {}
                Inline::Field(field) => {
                    let look = Look::of(&field.props, self.document);
                    push(&mut atoms, field.result.clone(), look);
                }
                Inline::Symbol {
                    character, props, ..
                } => push(
                    &mut atoms,
                    character.to_string(),
                    Look::of(props, self.document),
                ),
                Inline::Shape(shape) => match &shape.content {
                    ShapeContent::Picture(picture) => {
                        if let Some(name) = self.picture(picture.image) {
                            atoms.push(Atom::Raw(format!(
                                "![{}]({name})",
                                escape(&picture.description.replace('\n', " "))
                            )));
                        }
                    }
                    content @ ShapeContent::Text { .. } => frames.push(content.clone()),
                    ShapeContent::Empty => {}
                },
                Inline::Footnote(footnote) if !self.in_footnote => {
                    let body = self.footnote_body(&footnote.blocks);
                    self.footnotes.push(body);
                    atoms.push(Atom::Raw(format!("[^{}]", self.footnotes.len())));
                }
                Inline::Footnote(_)
                | Inline::Note(_)
                | Inline::Bookmark { .. }
                | Inline::Marker(_) => {}
            }
        }
        let mut out = String::new();
        for atom in atoms {
            match atom {
                Atom::Raw(text) => out.push_str(&text),
                Atom::Run(text, look) => out.push_str(&look.wrap(&text)),
            }
        }
        out
    }

    fn footnote_body(&mut self, blocks: &[Block]) -> String {
        let saved = (
            std::mem::take(&mut self.pieces),
            std::mem::take(&mut self.counters),
            self.in_footnote,
        );
        self.in_footnote = true;
        self.blocks(blocks);
        let text = self.assemble();
        (self.pieces, self.counters, self.in_footnote) = saved;
        text
    }

    /// The file name of image `index`, written beside the document once.
    fn picture(&mut self, index: usize) -> Option<String> {
        let image = self.document.images.get(index)?;
        if let Some(Some(name)) = self.names.get(index) {
            return Some(name.clone());
        }
        let (bytes, extension) = match (&image.fallback, image.kind) {
            (Some(png), ImageKind::Wmf | ImageKind::Emf) => (png.clone(), "png"),
            _ => (image.bytes.clone(), image.kind.extension()),
        };
        let name = format!("{}-{}.{extension}", self.stem, self.pictures.len() + 1);
        self.pictures.push((name.clone(), bytes));
        if let Some(slot) = self.names.get_mut(index) {
            *slot = Some(name.clone());
        }
        Some(name)
    }

    /// A GFM pipe table: the first row is the header.
    fn table(&mut self, table: &Table) -> String {
        let mut grid: Vec<Vec<String>> = Vec::new();
        let mut aligns: Vec<Alignment> = Vec::new();
        for (row_index, row) in table.rows.iter().enumerate() {
            let mut cells = Vec::new();
            for cell in &row.cells {
                let mut parts = Vec::new();
                let mut first_alignment = None;
                for block in &cell.blocks {
                    let part = match block {
                        Block::Paragraph(paragraph) => {
                            first_alignment = first_alignment.or(paragraph.props.alignment);
                            let mut frames = Vec::new();
                            self.inlines(&paragraph.inlines, &mut frames)
                        }
                        Block::Table(inner) => {
                            let text = self.table(inner);
                            text.replace('\n', " ")
                        }
                    };
                    let part = part.replace("\\\n", " ").replace(['\n', '\r'], " ");
                    if !part.trim().is_empty() {
                        parts.push(part.trim().to_owned());
                    }
                }
                if row_index == 0 {
                    aligns.push(first_alignment.unwrap_or(Alignment::Justify));
                }
                let text = if cell.merge == VerticalMerge::Continue {
                    String::new()
                } else {
                    parts.join(" ")
                };
                cells.push(text);
                for _ in 1..cell.span.max(1) {
                    cells.push(String::new());
                    if row_index == 0 {
                        aligns.push(Alignment::Justify);
                    }
                }
            }
            grid.push(cells);
        }
        let width = grid.iter().map(Vec::len).max().unwrap_or(0);
        if width == 0 {
            return String::new();
        }
        aligns.resize(width, Alignment::Justify);
        let line = |cells: &[String]| {
            let mut line = String::from("|");
            for column in 0..width {
                let _ = write!(line, " {} |", cells.get(column).map_or("", String::as_str));
            }
            line
        };
        let mut out = line(&grid[0]);
        out.push('\n');
        out.push('|');
        for align in &aligns {
            out.push_str(match align {
                Alignment::Left => " :--- |",
                Alignment::Centre => " :---: |",
                Alignment::Right => " ---: |",
                Alignment::Justify => " --- |",
            });
        }
        for row in &grid[1..] {
            out.push('\n');
            out.push_str(&line(row));
        }
        out
    }
}

/// A run's marks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Look {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
}

impl Look {
    fn of(props: &crate::CharProps, document: &ExchangeDocument) -> Self {
        let code = props.font.as_deref().is_some_and(|name| {
            document
                .fonts
                .iter()
                .any(|font| font.fixed && font.name == name)
                || matches!(
                    name.to_ascii_lowercase().as_str(),
                    "courier new" | "courier" | "consolas" | "monospace" | "lucida console"
                )
        });
        Self {
            bold: props.bold == Some(true),
            italic: props.italic == Some(true),
            strike: props.strike == Some(true),
            code,
        }
    }

    /// `text` with the marks outside its spaces (a mark beside a space
    /// does not open or close).
    fn wrap(self, text: &str) -> String {
        let core = text.trim_matches([' ', '\t']);
        if core.is_empty() {
            return escape(text);
        }
        let start = text.find(core).unwrap_or(0);
        let (lead, trail) = (&text[..start], &text[start + core.len()..]);
        let mut inner = if self.code {
            code_span(core)
        } else {
            escape(core)
        };
        if !self.code && (self.bold || self.italic) {
            let marks = match (self.bold, self.italic) {
                (true, true) => "***",
                (true, false) => "**",
                _ => "*",
            };
            inner = format!("{marks}{inner}{marks}");
        }
        if self.strike {
            inner = format!("~~{inner}~~");
        }
        format!("{lead}{inner}{trail}")
    }
}

/// `text` as a code span.
fn code_span(text: &str) -> String {
    let text = text.replace('\n', " ");
    let fence = "`".repeat(longest_run(&text, '`') + 1);
    let pad = text.starts_with('`') || text.ends_with('`');
    if pad {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

fn longest_run(text: &str, mark: char) -> usize {
    let (mut longest, mut run) = (0, 0);
    for c in text.chars() {
        if c == mark {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

/// Markdown's specials in running text, each with a backslash.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(
            c,
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '&' | '~' | '|'
        ) {
            out.push('\\');
        }
        if c == '\n' || c == '\r' {
            out.push(' ');
        } else if c.is_control() && c != '\t' {
            // Control characters mean nothing in text.
        } else {
            out.push(c);
        }
    }
    out
}

/// The characters that open a block when they start a line, escaped:
/// `#`, `-`, `+`, `=`, `>` (already escaped), and `1.` or `1)`.
fn escape_line_start(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let trimmed = line.trim_start_matches(' ');
        let lead = &line[..line.len() - trimmed.len()];
        out.push_str(lead);
        let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
        match trimmed.chars().next() {
            Some('#' | '-' | '+' | '=') => {
                out.push('\\');
                out.push_str(trimmed);
            }
            Some(_)
                if digits > 0 && matches!(trimmed[digits..].chars().next(), Some('.' | ')')) =>
            {
                out.push_str(&trimmed[..digits]);
                out.push('\\');
                out.push_str(&trimmed[digits..]);
            }
            _ => out.push_str(trimmed),
        }
    }
    out
}

fn is_code(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "code" | "source code" | "preformatted text"
    )
}

fn is_quote(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "quote" | "block quote" | "block text" | "intense quote"
    )
}

/// A code paragraph's line as typed: not escaped.
fn code_line(paragraph: &Paragraph, _marked: &str) -> String {
    let mut line = String::new();
    for inline in &paragraph.inlines {
        match inline {
            Inline::Text { text, .. } => line.push_str(&text.replace(['\n', '\r'], " ")),
            Inline::Tab { .. } => line.push('\t'),
            Inline::LineBreak => line.push(' '),
            Inline::Field(field) => line.push_str(&field.result),
            Inline::Symbol { character, .. } => line.push(*character),
            _ => {}
        }
    }
    line.chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .collect()
}

/// A file name as a link: unreserved characters as they are.
fn encode_path(name: &str) -> String {
    let name = if name.is_empty() { "picture" } else { name };
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
            out.push(c);
        } else if c == ' ' {
            out.push('-');
        } else {
            out.push('_');
        }
    }
    out
}
