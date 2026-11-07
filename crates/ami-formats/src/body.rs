//! Writing Ami Pro body text: the paragraphs of the main text, a frame's
//! `[txt]`, or a floating header, as Ami Pro's paragraph writer does
//! (decompile `11b0:1405`, with its helpers `11b0:0128` to `11b0:1377`).
//! The reader (`text.rs`) takes back what this writes; the model holds no
//! layout caches, so `<:#…>` is never written.
//!
//! A paragraph is its style tag (none for Body Text), its paragraph
//! controls (alignment, indents, spacing, ruler), then its text in physical
//! lines, then an empty line. Ami Pro breaks a line after more than 256
//! bytes of text, or after more than 65 when a space ends a stretch of
//! plain characters (the corpus's wrapped lines: 2,355 end in a space
//! between 66 and 256 bytes, the rest are 257 or longer).

use std::collections::BTreeMap;

use amipro_document::{
    Alignment, BreakKind, Caps, CharacterFormat, DateSource, Document, FlowId, FrameContent,
    FrameId, HeaderFooterKind, InlineObject, LayoutId, LineSpacing, OBJECT_REPLACEMENT_CHARACTER,
    PageSides, Paragraph, ParagraphStyle, StyleId,
};

use crate::writer::{tab_type, windows_1252};

/// Writes body text. `anchors` numbers the anchored frames as their
/// records are written (`<:A n>`, `<:t n>`).
pub(crate) struct Body<'a> {
    pub document: &'a Document,
    pub anchors: &'a BTreeMap<FrameId, usize>,
    /// The names layouts are written under (Ami Pro renames inserted
    /// layouts as it writes them); a layout not here keeps its own.
    pub layout_names: &'a BTreeMap<LayoutId, String>,
    /// Each style's fingerprint, worked out once (`write_cache`).
    pub style_prints: std::cell::RefCell<BTreeMap<StyleId, u64>>,
    /// The flows of the notes in the main text, whose `<:N` says so.
    pub main_notes: std::collections::BTreeSet<amipro_document::FlowId>,
    /// The style the flow being written leaves untagged, which a note's
    /// paragraphs share: Ami Pro writes a note in a table cell with the
    /// cell's default, so its Body Text is tagged there (corpus).
    untagged: std::cell::RefCell<String>,
}

impl<'a> Body<'a> {
    pub(crate) fn new(
        document: &'a Document,
        anchors: &'a BTreeMap<FrameId, usize>,
        layout_names: &'a BTreeMap<LayoutId, String>,
    ) -> Self {
        Self {
            document,
            anchors,
            layout_names,
            style_prints: std::cell::RefCell::default(),
            untagged: std::cell::RefCell::new(crate::BODY_TEXT.to_owned()),
            main_notes: document
                .main_flow()
                .paragraphs
                .iter()
                .flat_map(|paragraph| &paragraph.objects)
                .filter_map(|object| match object {
                    InlineObject::Note { flow, .. } => Some(*flow),
                    _ => None,
                })
                .collect(),
        }
    }

    /// Whether the paragraph's cache still holds: it and its style are as
    /// they were when it was read.
    fn cache_holds(&self, paragraph: &Paragraph) -> bool {
        let Some(cache) = paragraph.cache else {
            return false;
        };
        let Some(style) = self.document.styles().get(&paragraph.style) else {
            return false;
        };
        let print = *self
            .style_prints
            .borrow_mut()
            .entry(paragraph.style)
            .or_insert_with(|| style.fingerprint());
        paragraph.content_fingerprint_with(print) == cache.fingerprint
    }
}

/// One piece of a paragraph's text as the line breaker sees it: a plain
/// character (bytes 0x20 to 0x7F, escaped where needed) or anything else.
enum Piece {
    Plain(Small),
    Other(Vec<u8>),
    /// A floating header or footer: its opening, then its own lines, then
    /// its closing `>` on a line of its own.
    Floating(Vec<u8>, Vec<u8>),
}

/// A plain character as written, at most four bytes, without a heap
/// allocation for each character.
#[derive(Clone, Copy)]
struct Small {
    bytes: [u8; 4],
    len: u8,
}

impl Small {
    fn new(bytes: &[u8]) -> Self {
        let mut small = Self {
            bytes: [0; 4],
            len: bytes.len().min(4) as u8,
        };
        small.bytes[..usize::from(small.len)].copy_from_slice(&bytes[..usize::from(small.len)]);
        small
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

/// Whether a kept mark starts a power field or a bookmark (`<:X…>`,
/// `<:Z…>`, not their end marks `<:X~…>`, `<:Z~…>`).
fn is_field_start(raw: &[u8]) -> bool {
    (raw.starts_with(b"<:X") || raw.starts_with(b"<:Z")) && !raw[3..].starts_with(b"~")
}

/// Drops the `<:R>` marks the writer writes itself: one at the start of
/// each paragraph without its own ruler after one with a ruler, in every
/// flow. Run once at import, so that a ruler set or removed later moves
/// them as Ami Pro would. Returns how many it dropped.
pub(crate) fn drop_derived_ruler_marks(document: &mut Document) -> usize {
    let mut dropped = 0;
    let flows: Vec<FlowId> = document.flows().map(|(id, _)| id).collect();
    for id in flows {
        let Some(flow) = document.flow(id) else {
            continue;
        };
        let mut after_ruler = false;
        let mut changed = Vec::new();
        for (index, paragraph) in flow.paragraphs.iter().enumerate() {
            let own = paragraph.format.tabs.is_some();
            if after_ruler
                && !own
                && let Some(mark) = paragraph
                    .marks
                    .iter()
                    .position(|mark| mark.byte == 0 && mark.raw == "<:R>")
            {
                changed.push((index, mark));
            }
            after_ruler = own;
        }
        for (index, mark) in changed {
            let Some(style) = document
                .flow(id)
                .and_then(|flow| flow.paragraph(index))
                .and_then(|paragraph| document.styles().get(&paragraph.style))
                .map(ParagraphStyle::fingerprint)
            else {
                continue;
            };
            let Some(paragraph) = document
                .flow_mut(id)
                .and_then(|flow| flow.paragraph_mut(index))
            else {
                continue;
            };
            // A cache read with the paragraph still holds without the mark.
            let held = paragraph.cache.is_some_and(|cache| {
                cache.fingerprint == paragraph.content_fingerprint_with(style)
            });
            paragraph.marks.remove(mark);
            dropped += 1;
            let fingerprint = paragraph.content_fingerprint_with(style);
            if held && let Some(cache) = paragraph.cache.as_mut() {
                cache.fingerprint = fingerprint;
            }
        }
    }
    dropped
}

impl Body<'_> {
    /// Every paragraph of `flow`, without the closing `>`.
    pub(crate) fn flow(&self, out: &mut Vec<u8>, flow: FlowId) {
        self.flow_untagged(out, flow, crate::BODY_TEXT);
    }

    /// Every paragraph of `flow`, those in style `untagged` without their
    /// tag: Body Text, or Table Text in a table's cells (the frame writer
    /// changes the paragraph writer's default style for them, `11c8:2a0f`
    /// with `1098:0509`; corpus cells tag Body Text).
    pub(crate) fn flow_untagged(&self, out: &mut Vec<u8>, flow: FlowId, untagged: &str) {
        if let Some(flow) = self.document.flow(flow) {
            let outer = self.untagged.replace(untagged.to_owned());
            let mut after_ruler = false;
            for paragraph in &flow.paragraphs {
                self.paragraph_untagged(out, paragraph, untagged, after_ruler);
                after_ruler = paragraph.format.tabs.is_some();
            }
            self.untagged.replace(outer);
        }
    }

    /// One paragraph; `after_ruler` when the paragraph before it in its
    /// flow has its own ruler.
    fn paragraph_untagged(
        &self,
        out: &mut Vec<u8>,
        paragraph: &Paragraph,
        untagged: &str,
        after_ruler: bool,
    ) {
        let styles = self.document.styles();
        let name = styles
            .get(&paragraph.style)
            .map_or(crate::BODY_TEXT, |style| style.name.as_str());
        let tagged = !name.eq_ignore_ascii_case(untagged);
        if tagged {
            out.push(b'@');
            for character in name.chars() {
                out.extend(plain_or_escape(character));
            }
            out.push(b'@');
        }
        // Paragraph controls, written before the text and not counted in
        // its lines (`11b0:1405`), in Ami Pro's order: alignment, `<:v±>`,
        // `<:s>`, indents, spacing, ruler. `<:v±>`, `<:s>`, and a `<:R>` the
        // writer would not write are kept as legacy marks at the start.
        let mut controls: Vec<u8> = Vec::new();
        let format = &paragraph.format;
        if let Some(alignment) = format.alignment {
            controls.extend(match alignment {
                Alignment::Left => b"<+@>",
                Alignment::Right => b"<+A>",
                Alignment::Centre => b"<+B>",
                Alignment::Justify => b"<+C>",
            });
        }
        let leading = |raw: &str| {
            paragraph
                .marks
                .iter()
                .filter(|mark| mark.byte == 0 && mark.raw == raw)
                .count()
        };
        for raw in ["<:v+>", "<:v->", "<:s>"] {
            for _ in 0..leading(raw) {
                controls.extend(raw.as_bytes());
            }
        }
        if let Some(indents) = format.indents {
            controls.extend(
                format!(
                    "<:I{},{},{},{}>",
                    indents.all, indents.first, indents.rest, indents.right
                )
                .as_bytes(),
            );
        }
        if let Some(spacing) = format.line_spacing {
            let value = match spacing {
                LineSpacing::Single => -1,
                LineSpacing::OneAndAHalf(_) => -2,
                LineSpacing::Double(_) => -3,
                LineSpacing::Custom(height) => height,
            };
            controls.extend(format!("<:S+{value}>").as_bytes());
        }
        if let Some(tabs) = &format.tabs {
            let mut ruler = format!("<:R1,{},", tabs.len());
            for stop in tabs {
                ruler.push_str(&format!("{},{},", tab_type(stop), stop.position));
            }
            ruler.push('>');
            controls.extend(ruler.as_bytes());
        } else if after_ruler {
            // Ami Pro ends the ruler before for a paragraph without its own
            // (`11b0:1405`; VM fixture RU3); read back, it sets nothing.
            controls.extend(b"<:R>");
        }
        for _ in 0..leading("<:R>") {
            controls.extend(b"<:R>");
        }
        // Ami Pro's cache, last of the controls (`11b0:0476`), only while the
        // paragraph and its style are as read: Ami Pro trusts a cache whose
        // width is the column's (VM fixture K1W).
        if let Some(cache) = paragraph.cache.filter(|_| self.cache_holds(paragraph)) {
            controls.extend(format!("<:#{},{}>", cache.height, cache.width).as_bytes());
        }
        let has_controls = !controls.is_empty();
        out.extend(controls);
        let pieces = self.pieces(paragraph);
        if pieces.is_empty() {
            // An empty paragraph: a bare tag line, or an empty line.
            if has_controls {
                out.extend(b"\r\n");
            }
            out.extend(b"\r\n");
            return;
        }
        write_lines(out, pieces);
        out.extend(b"\r\n");
    }

    /// The paragraph's text as pieces, with its format changes, objects,
    /// and legacy marks where they stand.
    fn pieces(&self, paragraph: &Paragraph) -> Vec<Piece> {
        let mut pieces = Vec::new();
        let mut written = CharacterFormat::default();
        let mut marks = paragraph
            .marks
            .iter()
            .filter(|mark| {
                !(mark.byte == 0 && ["<:v+>", "<:v->", "<:s>", "<:R>"].contains(&mark.raw.as_str()))
            })
            .peekable();
        let mut objects = paragraph.objects.iter();
        // Runs are sorted and do not overlap: walk them with the text.
        let default = CharacterFormat::default();
        let mut run = 0;
        // Fields' start marks that end the marks at a place come after the
        // formatting that starts there, as Ami Pro writes them (`<+!><:X…>`,
        // VM fixture FD3B); the marks keep their order.
        let mut starts: Vec<&[u8]> = Vec::new();
        for (byte, character) in paragraph.text.char_indices() {
            let mut here: Vec<&[u8]> = Vec::new();
            while let Some(mark) = marks.next_if(|mark| mark.byte <= byte) {
                here.push(mark.raw.as_bytes());
            }
            let tail = here
                .iter()
                .rev()
                .take_while(|raw| is_field_start(raw))
                .count();
            starts.extend(here.drain(here.len() - tail..));
            pieces.extend(here.into_iter().map(|raw| Piece::Other(raw.to_vec())));
            let before = run;
            while paragraph
                .runs
                .get(run)
                .is_some_and(|current| current.end <= byte)
            {
                run += 1;
            }
            let wanted = paragraph
                .runs
                .get(run)
                .filter(|current| current.start <= byte)
                .map_or(&default, |current| &current.format);
            // Two runs of one format, side by side: the file closed the
            // formatting and opened it again there (Ami Pro writes text
            // formatted at different times so, `<-!><+!>`), and so does the
            // writer. Ami Pro draws each run's underline on its own (O6).
            if run != before
                && *wanted == written
                && !written.is_empty()
                && paragraph.runs[before].end == byte
                && paragraph.runs[run].start == byte
            {
                pieces.extend(
                    self.changes(&written, &default)
                        .into_iter()
                        .map(Piece::Other),
                );
                pieces.extend(self.changes(&default, wanted).into_iter().map(Piece::Other));
            }
            if *wanted != written {
                for change in self.changes(&written, wanted) {
                    pieces.push(Piece::Other(change));
                }
                written = wanted.clone();
            }
            pieces.extend(starts.drain(..).map(|raw| Piece::Other(raw.to_vec())));
            if character == OBJECT_REPLACEMENT_CHARACTER {
                if let Some(object) = objects.next() {
                    pieces.push(self.object(object));
                }
                continue;
            }
            if byte == 0 && character == '@' {
                // `@@` is `@` at the first text position, after any tag and
                // leading controls (the reader, `1118:0e55`).
                pieces.push(Piece::Plain(Small::new(b"@@")));
                continue;
            }
            pieces.push(character_piece(character));
        }
        for mark in marks {
            pieces.push(Piece::Other(mark.raw.as_bytes().to_vec()));
        }
        if paragraph.end_format != written {
            for change in self.changes(&written, &paragraph.end_format) {
                pieces.push(Piece::Other(change));
            }
        }
        pieces
    }

    /// The controls that change `from` to `to`: attributes off, attributes
    /// on, the font, then overstrike (the order of the corpus's examples;
    /// Ami Pro's own follows the order formatting was applied in). Bold, italic, the underlines, and
    /// caps can only be on or the style's; protected text, no hyphenation,
    /// strikethrough, and the scripts can be on or off, but not return to
    /// the style's within a paragraph (the reader).
    fn changes(&self, from: &CharacterFormat, to: &CharacterFormat) -> Vec<Vec<u8>> {
        let mut changes = Vec::new();
        let mut off = Vec::new();
        let mut on = Vec::new();
        let returns = [
            (from.bold, to.bold, b'!'),
            (from.italic, to.italic, b'"'),
            (from.underline, to.underline, b'#'),
            (from.word_underline, to.word_underline, b'$'),
            (from.double_underline, to.double_underline, b')'),
        ];
        for (from, to, code) in returns {
            match (from == Some(true), to == Some(true)) {
                (false, true) => on.push(code),
                (true, false) => off.push(code),
                _ => {}
            }
        }
        // Normal caps (the app's way to switch a style's caps off) cannot
        // be written: Ami Pro's caps off returns to the style's.
        let caps_code = |caps: Option<Caps>| match caps? {
            Caps::Upper => Some(b'+'),
            Caps::Lower => Some(b','),
            Caps::Initial => Some(b'-'),
            Caps::Small => Some(b'('),
            Caps::Normal => None,
        };
        match (caps_code(from.caps), caps_code(to.caps)) {
            (Some(from), None) => off.push(from),
            (from, Some(to)) if from != Some(to) => on.push(to),
            _ => {}
        }
        let ends = [
            (from.strikethrough, to.strikethrough, b'%'),
            (from.superscript, to.superscript, b'&'),
            (from.subscript, to.subscript, b'\''),
            (from.protected, to.protected, b'*'),
            (from.no_hyphenation, to.no_hyphenation, b'.'),
        ];
        for (from, to, code) in ends {
            match (from, to) {
                (Some(true), Some(true)) | (Some(false), Some(false) | None) | (None, None) => {}
                (_, Some(true)) => on.push(code),
                (_, Some(false)) | (Some(true), None) => off.push(code),
            }
        }
        changes.extend(off.into_iter().map(|code| vec![b'<', b'-', code, b'>']));
        changes.extend(on.into_iter().map(|code| vec![b'<', b'+', code, b'>']));
        if (&from.font_family, from.font_size_points, from.colour)
            != (&to.font_family, to.font_size_points, to.colour)
        {
            changes.push(self.font(to));
        }
        if from.overstrike != to.overstrike {
            let control = match (from.overstrike, to.overstrike) {
                (_, Some(character)) => Some((b'+', character)),
                (Some(character), None) => Some((b'-', character)),
                (None, None) => None,
            };
            if let Some((sign, character)) = control {
                let mut bytes = vec![b'<', b':', b'O', sign];
                bytes.extend(plain_or_escape(character));
                bytes.push(b'>');
                changes.push(bytes);
            }
        }
        changes
    }

    /// `<:fSIZE,KFACE,R,G,B>` with the style's fields empty, or `<:f>`
    /// for the style's font (`text.rs`, `font_change`).
    fn font(&self, format: &CharacterFormat) -> Vec<u8> {
        if format.font_family.is_none()
            && format.font_size_points.is_none()
            && format.colour.is_none()
        {
            return b"<:f>".to_vec();
        }
        let mut control = b"<:f".to_vec();
        if let Some(size) = format.font_size_points {
            control.extend(((size * 20.0).round() as i64).to_string().as_bytes());
        }
        control.push(b',');
        if let Some(family) = &format.font_family {
            let kind = self
                .document
                .face_kinds()
                .get(&family.to_lowercase())
                .copied()
                .unwrap_or(0);
            control.push(kind.saturating_add(0x20));
            control.extend(family.chars().map(windows_1252));
        }
        control.push(b',');
        if let Some(colour) = format.colour {
            control.extend(
                format!(
                    "{},{},{}",
                    colour & 0xFF,
                    colour >> 8 & 0xFF,
                    colour >> 16 & 0xFF
                )
                .as_bytes(),
            );
        }
        control.push(b'>');
        control
    }

    fn object(&self, object: &InlineObject) -> Piece {
        match object {
            InlineObject::Break {
                kind,
                centre,
                layout,
                restart,
            } => match kind {
                BreakKind::Column => Piece::Other(b"<:c>".to_vec()),
                BreakKind::Page => {
                    let name = layout.and_then(|layout| {
                        self.layout_names.get(&layout).cloned().or_else(|| {
                            self.document
                                .layout(layout)
                                .map(|layout| layout.name.clone())
                        })
                    });
                    let flags = if *centre { 0x40 } else { 0 }
                        | if name.is_some() { 0x80 } else { 0 }
                        | if *restart { 0x02 } else { 0 };
                    let mut control = b"<:p".to_vec();
                    control.extend(byte_escape(flags));
                    if let Some(name) = name {
                        control.extend(name.chars().map(windows_1252));
                    }
                    control.push(b'>');
                    Piece::Other(control)
                }
            },
            InlineObject::FrameAnchor(frame) => {
                let index = self.anchors.get(frame).copied().unwrap_or(0);
                let table = self
                    .document
                    .frame(*frame)
                    .is_some_and(|frame| matches!(frame.content, FrameContent::Table(_)));
                let kind = if table { 't' } else { 'A' };
                Piece::Other(format!("<:{kind}{index}>").into_bytes())
            }
            // `<:B` and the code as text writes a code (VM fixture BL1).
            InlineObject::Bullet(code) => {
                let mut control = b"<:B".to_vec();
                control.extend(crate::writer::escape_text(&code.to_string()));
                control.push(b'>');
                Piece::Other(control)
            }
            // The letter and the format byte (`11b0:0660`; VM fixture FD1).
            InlineObject::Date(field) => {
                let letter = match field.source {
                    DateSource::Today => b'D',
                    DateSource::LastSaved => b'r',
                    DateSource::Created => b'k',
                    DateSource::Time => b'b',
                };
                let mut control = vec![b'<', b':', letter];
                crate::writer::escape_control_bytes(&[field.format], &mut control);
                control.push(b'>');
                Piece::Other(control)
            }
            // The format letter and the code, low byte first (FD2).
            InlineObject::DocInfo(field) => {
                let mut control = b"<:d".to_vec();
                let [low, high] = field.code.to_le_bytes();
                crate::writer::escape_control_bytes(&[field.format, low, high], &mut control);
                control.push(b'>');
                Piece::Other(control)
            }
            // The mark, then the number it shows, which Ami Pro writes for
            // other readers (`11b0:0660`; VM fixture FD6).
            InlineObject::Seq(field) => {
                let mark = amipro_document::PowerMark {
                    end: false,
                    kind: amipro_document::power_kind::SEQ,
                    value: field.value,
                    code: field.code.clone(),
                };
                let mut control = mark.raw().chars().map(windows_1252).collect::<Vec<u8>>();
                control.extend(b"<:X~4;");
                control.extend(crate::writer::escape_text(&field.text()));
                control.push(b'>');
                Piece::Other(control)
            }
            InlineObject::PageNumber(field) => {
                let mut control = b"<:P".to_vec();
                control.push(windows_1252(field.style));
                control.extend(format!("{},{},", field.start_page, field.start_number).bytes());
                control.extend(field.leading_text.chars().map(windows_1252));
                control.push(b'>');
                Piece::Other(control)
            }
            // The time, and unless the note was read in the short form, the
            // initials (escaped), the colour, the number, and whether it is
            // in the main text; its paragraphs as body text (`11b0:0660`;
            // VM fixtures RNN1–RNN3).
            InlineObject::Note { note, flow } => {
                let mut opening = format!("<:N{}", note.time).into_bytes();
                if note.initials.is_some() || note.colour.is_some() || note.number.is_some() {
                    opening.push(b',');
                    opening.extend(crate::writer::escape_text(
                        note.initials.as_deref().unwrap_or_default(),
                    ));
                    let main = self.main_notes.contains(flow);
                    opening.extend(
                        format!(
                            ",{},{},{}",
                            note.colour.unwrap_or(amipro_document::NoteInfo::YELLOW),
                            note.number.unwrap_or(0),
                            u8::from(main)
                        )
                        .bytes(),
                    );
                }
                let mut inner = Vec::new();
                let untagged = self.untagged.borrow().clone();
                self.flow_untagged(&mut inner, *flow, &untagged);
                Piece::Floating(opening, inner)
            }
            InlineObject::Footnote { flow } => {
                let mut inner = Vec::new();
                self.flow(&mut inner, *flow);
                Piece::Floating(b"<:F".to_vec(), inner)
            }
            InlineObject::TableFootnote { number, row } => {
                Piece::Other(format!("<:x{number},{row}>").into_bytes())
            }
            InlineObject::FloatingHeaderFooter { kind, pages, flow } => {
                let (letter, kind_bit) = match kind {
                    HeaderFooterKind::Header => (b'H', 1),
                    HeaderFooterKind::Footer => (b'h', 2),
                };
                let page_bits = match pages {
                    PageSides::All => 4 | 8,
                    PageSides::Left => 4,
                    PageSides::Right => 8,
                };
                let mut opening = vec![b'<', b':', letter];
                opening.extend(byte_escape(kind_bit | page_bits));
                let mut inner = Vec::new();
                self.flow(&mut inner, *flow);
                Piece::Floating(opening, inner)
            }
        }
    }
}

/// A character of text: a plain one (escaped `<<`, `<;>`, `<[>` where
/// Ami Pro escapes it, `11b0:1377`), or an escape (`11b0:120f`).
fn character_piece(character: char) -> Piece {
    match character {
        '\u{2028}' => Piece::Other(byte_escape(0x0A)),
        '\u{AD}' => Piece::Other(byte_escape(0x17)),
        '\t' => Piece::Other(vec![b'\t']),
        // Other control characters only come from a file that had them
        // raw (a DOS end-of-file byte): written back as they were.
        '\u{0}'..='\u{1f}' => Piece::Other(vec![character as u8]),
        _ => {
            let byte = windows_1252(character);
            if (0x20..0x80).contains(&byte) {
                Piece::Plain(Small::new(&plain_or_escape(character)))
            } else {
                Piece::Other(byte_escape(byte))
            }
        }
    }
}

/// A character as Ami Pro writes it in body text.
fn plain_or_escape(character: char) -> Vec<u8> {
    match windows_1252(character) {
        b'<' => b"<<".to_vec(),
        b'>' => b"<;>".to_vec(),
        b'[' => b"<[>".to_vec(),
        byte @ 0x20..0x80 => vec![byte],
        byte => byte_escape(byte),
    }
}

/// A byte outside plain text: `<*X>` below 0x20 (X − 0x20), `</X>` from
/// 0x80 to 0xBF (X + 0x40), `<\X>` from 0xC0 (X − 0x80).
fn byte_escape(byte: u8) -> Vec<u8> {
    match byte {
        b'\t' => vec![b'\t'],
        0x00..=0x1F => vec![b'<', b'*', byte + 0x20, b'>'],
        0x80..=0xBF => vec![b'<', b'/', byte - 0x40, b'>'],
        0xC0..=0xFF => vec![b'<', b'\\', byte.wrapping_add(0x80), b'>'],
        _ => vec![byte],
    }
}

/// Lays the pieces out in Ami Pro's physical lines (`11b0:1405`): after
/// each stretch of plain characters, and after anything else, a line ends
/// when it is longer than 65 bytes and ends in a space, or longer than 256;
/// within a stretch, it ends when longer than 256. After a break, a second
/// space after `. ` stays on the line.
fn write_lines(out: &mut Vec<u8>, pieces: Vec<Piece>) {
    let mut line: Vec<u8> = Vec::new();
    let mut pieces = pieces.into_iter().peekable();
    let flush = |out: &mut Vec<u8>, line: &mut Vec<u8>| {
        out.append(line);
        out.extend(b"\r\n");
    };
    let after_break =
        |line: &mut Vec<u8>, pieces: &mut std::iter::Peekable<std::vec::IntoIter<Piece>>| {
            if line.len() > 65
                && line.ends_with(b". ")
                && matches!(pieces.peek(), Some(Piece::Plain(bytes)) if bytes.as_slice() == b" ")
            {
                pieces.next();
                line.push(b' ');
            }
        };
    while let Some(piece) = pieces.next() {
        match piece {
            Piece::Plain(bytes) => {
                line.extend(bytes.as_slice());
                let run_ends = !matches!(pieces.peek(), Some(Piece::Plain(_)));
                if line.len() > 256 || run_ends && line.len() > 65 && line.ends_with(b" ") {
                    after_break(&mut line, &mut pieces);
                    flush(out, &mut line);
                }
            }
            // A control that spans lines (a note: its opening, its own
            // paragraphs, its closing `>`), kept with LF between its lines:
            // each line written with CR LF. The closing `>` ends its line
            // (`11b0:0660`), and the text goes on on the next: Ami Pro
            // ends a note only at a lone `>` line, and with text after it
            // read the rest of the document into the note and faulted
            // (MERCURY.SAM, VM 2026-10-13).
            Piece::Other(bytes) if bytes.contains(&b'\n') => {
                let mut parts = bytes.split(|byte| *byte == b'\n');
                line.extend(parts.next().unwrap_or_default());
                let rest: Vec<&[u8]> = parts.collect();
                if let Some((last, middle)) = rest.split_last() {
                    flush(out, &mut line);
                    for part in middle {
                        out.extend_from_slice(part);
                        out.extend(b"\r\n");
                    }
                    line.extend_from_slice(last);
                    flush(out, &mut line);
                }
            }
            Piece::Other(bytes) => {
                line.extend(bytes);
                if line.len() > 256 || line.len() > 65 && line.ends_with(b" ") {
                    after_break(&mut line, &mut pieces);
                    flush(out, &mut line);
                }
            }
            Piece::Floating(opening, inner) => {
                line.extend(opening);
                flush(out, &mut line);
                out.extend(inner);
                out.extend(b">\r\n");
            }
        }
    }
    if !line.is_empty() {
        flush(out, &mut line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_sam;

    /// The main text of `sam` written by `Body` and read again.
    fn written_main_text(sam: &str) -> (Document, Vec<u8>, Document) {
        let document = import_sam(sam.as_bytes(), None, "Body").unwrap().document;
        let mut text = Vec::new();
        Body::new(&document, &BTreeMap::new(), &BTreeMap::new())
            .flow(&mut text, document.main_flow_id());
        let mut again = b"[ver]\r\n\t4\r\n".to_vec();
        let start = sam.find("[edoc]").unwrap();
        again.extend(sam[..start].lines().skip(2).flat_map(|line| {
            let mut line = line.as_bytes().to_vec();
            line.extend(b"\r\n");
            line
        }));
        again.extend(b"[edoc]\r\n");
        again.extend(&text);
        again.extend(b">\r\n");
        let reread = import_sam(&again, None, "Body").unwrap().document;
        (document, text, reread)
    }

    fn paragraphs(document: &Document) -> Vec<String> {
        document
            .paragraphs()
            .iter()
            .map(|paragraph| {
                format!(
                    "{} {:?} {:?} {:?} {:?} {:?} {:?}",
                    document.styles()[&paragraph.style].name,
                    paragraph.text,
                    paragraph.format,
                    paragraph.runs,
                    paragraph.objects,
                    paragraph.marks,
                    paragraph.end_format
                )
            })
            .collect()
    }

    const STYLES: &str = "[ver]\r\n\t4\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
        [tag]\r\n\tTitle <<A>\r\n\t3\r\n\t[fnt]\r\n\t\tArial\r\n\t\t360\r\n\t\t0\r\n\t\t16385\r\n";

    #[test]
    fn a_paragraph_after_a_ruler_paragraph_ends_the_ruler_ru3() {
        // As Ami Pro writes RU3: the ruler, then `<:R>` on the next
        // paragraph only. Read back, no mark is kept for it, and a `<:R>`
        // Ami Pro would not write is kept where it stood.
        let body =
            "<:R1,2,1,720,1,2700,>Ruled\r\n\r\nNext\r\n\r\nAfter\r\n\r\n<:R>Kept\r\n\r\n>\r\n";
        let sam = format!("{STYLES}[edoc]\r\n{body}");
        let (document, text, reread) = written_main_text(&sam);
        assert!(
            document.paragraphs()[1].marks.is_empty(),
            "derived mark dropped"
        );
        assert_eq!(document.paragraphs()[0].marks.len(), 0);
        assert_eq!(
            document.paragraphs()[3].marks.len(),
            1,
            "a mark the writer would not write"
        );
        assert_eq!(paragraphs(&reread), paragraphs(&document));
        let text = String::from_utf8(text).unwrap();
        assert_eq!(
            text,
            "<:R1,2,1,720,1,2700,>Ruled\r\n\r\n<:R>Next\r\n\r\nAfter\r\n\r\n<:R>Kept\r\n\r\n"
        );
    }

    #[test]
    fn paragraphs_are_written_as_the_reader_reads_them() {
        // Escapes, tags, character formatting of every kind, paragraph
        // formatting, legacy marks, objects, and end formats.
        let body = "@Title <<A<;>@<+B>A <<tag<;>, <[>brackets], caf<\\i>, </R>quotes</S>\ttab<**>hard<*7>soft\r\n\r\n\
            <:s><:I720,0,360,0><:S+-2><:R1,2,1,720,3,4320,><+!>Bold<-!> <+\">it<+$>word<-$><-\"> <+(>small<-(>\r\n\
            <+%>struck<-%>after <+&>sup<-&> <:f200,2Times New Roman,255,0,0>red<:f> <:O+/>over<:O-/> <+*>kept<-*>\r\n\r\n\
            @@at first<:x unknown> mark\r\n\r\n\
            Page <:P10,1,>\r\n\r\n\
            <:f200,,>\r\n\r\n\
            @Title <<A<;>@\r\n\
            \r\n\
            <:s>\r\n\r\n\
            >\r\n";
        let sam = format!("{STYLES}[edoc]\r\n{body}");
        let (document, text, reread) = written_main_text(&sam);
        assert_eq!(paragraphs(&reread), paragraphs(&document));
        let text = String::from_utf8(text).unwrap();
        assert!(text.starts_with("@Title <<A<;>@<+B>A <<tag<;>, <[>brackets], caf<\\i>, </R>quotes</S>\ttab<**>hard<*7>soft\r\n\r\n"));
        assert!(text.contains("<:s><:I720,0,360,0><:S+-2><:R1,2,1,720,3,4320,><+!>Bold<-!>"));
        assert!(text.contains("<:f200,2Times New Roman,255,0,0>red<:f>"));
        // After a ruler paragraph, Ami Pro ends the ruler (RU3).
        assert!(
            text.contains("\r\n\r\n<:R>@@at first<:x unknown> mark\r\n\r\n"),
            "{text}"
        );
        // An empty tagged paragraph is its bare tag; an empty Body Text one
        // is an empty line.
        assert!(text.ends_with("<:f200,,>\r\n\r\n@Title <<A<;>@\r\n\r\n<:s>\r\n\r\n"));
    }

    #[test]
    fn inserted_bullets_are_written_as_ami_pro_writes_them_bl1() {
        // As BL1: a bullet, one in bold, one after a font change, and the
        // picture whose code Ami Pro writes as `<**>`.
        let body = "Alpha <:B<*0>> small dot\r\n\r\nBold <+!><:B<*1>> bold item<-!>\r\n\r\n\
            Big <:f480,2Times New Roman,0,0,0><:B<*1>> big item\r\n\r\nB11 <:B<**>> item 11\r\n\r\n>\r\n";
        let sam = format!("{STYLES}[edoc]\r\n{body}");
        let (document, text, reread) = written_main_text(&sam);
        assert_eq!(paragraphs(&reread), paragraphs(&document));
        assert_eq!(
            document.paragraphs()[3].objects,
            [InlineObject::Bullet('\u{A}')]
        );
        assert_eq!(
            String::from_utf8(text).unwrap(),
            "Alpha <:B<*0>> small dot\r\n\r\nBold <+!><:B<*1>> bold item<-!>\r\n\r\n\
            Big <:f480,2Times New Roman,0,0,0><:B<*1>> big item\r\n\r\nB11 <:B<**>> item 11\r\n\r\n"
        );
    }

    #[test]
    fn dates_and_doc_info_entries_are_written_as_ami_pro_writes_them_fd1_fd2() {
        // As FD1 and FD2 (Field2's code 9 written as a tab), with a format
        // byte that needs `<;>`.
        let body = "S: <:Db>\r\n\r\nH: <:b<*!>>\r\n\r\nR: <:rd> C: <:kc>\r\n\r\n\
            0: <:dc<*!><* >>\r\n\r\n15: <:dc\t<* >>\r\n\r\nOdd <:D<;>> <:D<<>\r\n\r\n>\r\n";
        let sam = format!("{STYLES}[edoc]\r\n{body}");
        let (document, text, reread) = written_main_text(&sam);
        assert_eq!(paragraphs(&reread), paragraphs(&document));
        assert_eq!(document.paragraphs()[5].objects.len(), 2);
        assert_eq!(
            String::from_utf8(text).unwrap(),
            "S: <:Db>\r\n\r\nH: <:b<*!>>\r\n\r\nR: <:rd> C: <:kc>\r\n\r\n\
            0: <:dc<*!><* >>\r\n\r\n15: <:dc\t<* >>\r\n\r\nOdd <:D<;>> <:D<<>\r\n\r\n"
        );
    }

    #[test]
    fn seq_fields_are_objects_written_as_ami_pro_writes_them_fd6() {
        // FD6's Seq lines, and a bold result whose formatting starts with
        // its field (FD3B): the start mark after the formatting.
        let body = "11: <:X4,-32767;Seq A><:X~4;1>\r\n\r\n17: <:X4,-32755;Seq A+3 %N2><:X~4;XIII>\r\n\r\n\
            2: <+!><:X3,-32768;CreateDate %Db>October 20, 2026<:X~3,-32768;CreateDate %Db><-!>\r\n\r\n>\r\n";
        let sam = format!("{STYLES}[edoc]\r\n{body}");
        let (document, text, reread) = written_main_text(&sam);
        assert_eq!(paragraphs(&reread), paragraphs(&document));
        assert_eq!(document.paragraphs()[1].text, "17: \u{FFFC}");
        assert_eq!(
            document.paragraphs()[1].objects,
            [InlineObject::Seq(amipro_document::SeqField {
                code: "Seq A+3 %N2".into(),
                value: 0x800D,
            })]
        );
        assert!(document.paragraphs()[1].marks.is_empty());
        assert_eq!(
            String::from_utf8(text).unwrap(),
            body.trim_end_matches(">\r\n").to_owned()
        );
    }

    #[test]
    fn floating_headers_page_breaks_and_anchors_are_written_in_the_main_text() {
        let body = "First<:p<* >>\r\n\r\n<:h<*.>\r\nFooter text\r\n\r\n>After\r\n\r\n>\r\n";
        let sam = format!("{STYLES}[edoc]\r\n{body}");
        let (document, text, reread) = written_main_text(&sam);
        assert_eq!(paragraphs(&reread).len(), paragraphs(&document).len());
        assert_eq!(
            String::from_utf8(text).unwrap(),
            "First<:p<* >>\r\n\r\n<:h<*.>\r\nFooter text\r\n\r\n>\r\nAfter\r\n\r\n"
        );
        let footer = reread
            .paragraphs()
            .iter()
            .find_map(|paragraph| match paragraph.objects.first() {
                Some(InlineObject::FloatingHeaderFooter { flow, .. }) => Some(*flow),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            reread.flow(footer).unwrap().paragraphs[0].text,
            "Footer text"
        );
    }

    #[test]
    fn a_note_ends_at_a_lone_closing_line_and_the_text_goes_on_after_it() {
        // MERCURY.SAM: text after a note starts on the line after its `>`;
        // written on the same line, Ami Pro read on past the note.
        let sam = format!(
            "{STYLES}[edoc]\r\nNotes print as annotations. \r\n<:N763670765,BEH,32768,2,1\r\nA note.\r\n\r\n\r\nCustomize it.\r\n\r\n>\r\n. \r\n\r\nNext.\r\n\r\n>\r\n"
        );
        let (document, text, reread) = written_main_text(&sam);
        let text = String::from_utf8(text).unwrap();
        assert!(
            text.contains("Customize it.\r\n\r\n>\r\n. \r\n\r\nNext."),
            "{text:?}"
        );
        assert_eq!(paragraphs(&reread), paragraphs(&document));
        let warns = |bytes: &[u8]| {
            import_sam(bytes, None, "Note")
                .unwrap()
                .report
                .warnings
                .iter()
                .any(|warning| warning.contains("Ami Pro reads on past them"))
        };
        assert!(!warns(sam.as_bytes()));
        assert!(warns(sam.replace(">\r\n. ", ">. ").as_bytes()));
    }

    #[test]
    fn ami_pros_visible_hyphen_and_the_discretionary_one_stay_apart() {
        // `</m>` (byte 0xAD) is a hyphen Ami Pro shows, as in "1-2-3"
        // (VM fixture T02); `<*7>` is the discretionary hyphen (V19).
        let sam = format!("{STYLES}[edoc]\r\n1</m>2</m>3 co<*7>operate\r\n\r\n>\r\n");
        let (document, text, reread) = written_main_text(&sam);
        assert_eq!(
            document.paragraphs()[0].text,
            "1\u{2010}2\u{2010}3 co\u{AD}operate"
        );
        assert_eq!(paragraphs(&reread), paragraphs(&document));
        assert_eq!(
            String::from_utf8(text).unwrap(),
            "1</m>2</m>3 co<*7>operate\r\n\r\n"
        );
    }

    #[test]
    fn a_cache_is_written_back_only_while_its_paragraph_is_as_read() {
        // Ami Pro trusts a cache whose width is the column's (VM fixture
        // K1W), so an edited paragraph is written without one.
        let sam = format!(
            "{STYLES}[edoc]\r\n<:#284,9360>Kept\r\n\r\n<+B><:#568,9360>Edited\r\n\r\n<:#284,9360>\r\n\r\n>\r\n"
        );
        let mut document = import_sam(sam.as_bytes(), None, "Caches").unwrap().document;
        assert!(
            document
                .paragraphs()
                .iter()
                .all(|paragraph| paragraph.cache.is_some())
        );
        document.paragraph_mut(1).text.push('!');
        let mut text = Vec::new();
        Body::new(&document, &BTreeMap::new(), &BTreeMap::new())
            .flow(&mut text, document.main_flow_id());
        assert_eq!(
            String::from_utf8(text).unwrap(),
            "<:#284,9360>Kept\r\n\r\n<+B>Edited!\r\n\r\n<:#284,9360>\r\n\r\n"
        );
    }

    #[test]
    fn lines_break_as_ami_pro_breaks_them() {
        // Plain text runs to 257 bytes; a line longer than 65 ends at a
        // space where a control follows; a second space after ". " stays.
        let long = "a".repeat(300);
        let words = format!("{} end.  <+!>bold<-!>", "word ".repeat(14));
        let sam = format!("{STYLES}[edoc]\r\n{long}\r\n\r\n{words}\r\n\r\n>\r\n");
        let (document, text, reread) = written_main_text(&sam);
        assert_eq!(paragraphs(&reread), paragraphs(&document));
        let text = String::from_utf8(text).unwrap();
        let lines: Vec<&str> = text.split("\r\n").collect();
        assert_eq!(lines[0].len(), 257);
        assert_eq!(lines[1], "a".repeat(43));
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], format!("{} end.  ", "word ".repeat(14)));
        assert_eq!(lines[4], "<+!>bold<-!>");
    }
}
