//! Ami Pro's Find & Replace (`docs/research/FIND_NOTES.md`; `UI_SPEC.md`
//! §27): its patterns, the text streams in its order, the matches in them,
//! and each match's replacement. Replacements are ordinary edits; Replace
//! All is one [`Command::Several`].

use std::ops::Range;

use amipro_document::{
    CellContent, Document, FlowId, FrameContent, FrameId, InlineObject,
    OBJECT_REPLACEMENT_CHARACTER, Placement, StyleId,
};

use crate::{Command, TextPosition};

/// How a tab shows in the Find and Replace fields: Ami Pro's mark for the
/// tab Ctrl+Tab types there (the guide, ch. 20, "To delete tabs").
pub const TAB_MARK: char = '»';
/// How a return shows in the fields: Ami Pro's mark for Ctrl+Enter (ch. 20,
/// "To delete returns").
pub const RETURN_MARK: char = '¶';

/// What to find and how: Ami Pro's Find & Replace Options (dialog 1121) and
/// Attributes (dialog 1007). The defaults are Ami Pro's (VM FR1, decision
/// Find F2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindOptions {
    /// Find Whole word only: no letter or digit right before or after a
    /// match (FR1B).
    pub whole_word: bool,
    /// Find Exact case: capitals must match.
    pub match_case: bool,
    /// Replace Exact case off: the replacement takes the found text's
    /// capitals (FR1, FR1B); on, it is as typed (FR2). Decision Find F3.
    pub keep_capitals: bool,
    /// Beginning of document: the search starts at the top of the first
    /// stream it searches rather than at the caret.
    pub from_beginning: bool,
    /// Include other text streams: the streams after the caret's in Ami
    /// Pro's order (FR5, FR6).
    pub other_streams: bool,
    /// Find & replace type Style: Find and Replace name paragraph styles.
    pub styles: bool,
    /// Find Exact attributes: only text with exactly these attributes.
    pub find_attributes: Option<Attributes>,
    /// Replace Exact attributes: the replacement gets exactly these.
    pub replace_attributes: Option<Attributes>,
}

impl Default for FindOptions {
    fn default() -> Self {
        Self {
            whole_word: true,
            match_case: false,
            keep_capitals: true,
            from_beginning: false,
            other_streams: true,
            styles: false,
            find_attributes: None,
            replace_attributes: None,
        }
    }
}

/// Ami Pro's six Find & Replace attributes: Normal is none of the five.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Attributes {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub word_underline: bool,
    pub small_caps: bool,
}

/// Where a text stream sits in Ami Pro's search order (`FIND_NOTES.md`,
/// "Text streams"): the main text, Where placed frames, footnotes, With
/// para above and Flow with text frames and page tables, floating headers
/// and footers, then fixed headers, footers, and repeating frames. Notes
/// and Draw frames are not searched.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StreamKind {
    Main,
    PlacedFrame,
    Footnote,
    AnchoredFrame,
    FloatingHeaderFooter,
    Header,
    Footer,
    RepeatingFrame,
}

/// A text stream: a flow and what holds it (a frame's text or one of its
/// table's cells).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stream {
    pub flow: FlowId,
    pub kind: StreamKind,
    pub frame: Option<FrameId>,
}

/// Every text stream of `document` in Ami Pro's search order. A table's
/// cells go row by row.
pub fn streams(document: &Document) -> Vec<Stream> {
    let mut found = vec![Stream {
        flow: document.main_flow_id(),
        kind: StreamKind::Main,
        frame: None,
    }];
    let push_frame = |found: &mut Vec<Stream>, id: FrameId, kind: StreamKind| {
        if let Some(frame) = document.frame(id) {
            for flow in frame_flows(&frame.content) {
                found.push(Stream {
                    flow,
                    kind,
                    frame: Some(id),
                });
            }
        }
    };
    for &id in document.frame_stack() {
        if document
            .frame(id)
            .is_some_and(|frame| matches!(frame.placement, Placement::Fixed { .. }))
        {
            push_frame(&mut found, id, StreamKind::PlacedFrame);
        }
    }
    // Footnotes, in the order of their references (the notes run).
    for paragraph in &document.main_flow().paragraphs {
        for object in &paragraph.objects {
            if let InlineObject::Footnote { flow } = object {
                found.push(Stream {
                    flow: *flow,
                    kind: StreamKind::Footnote,
                    frame: None,
                });
            }
        }
    }
    let mut floating = Vec::new();
    for paragraph in &document.main_flow().paragraphs {
        for object in &paragraph.objects {
            match object {
                InlineObject::FrameAnchor(id)
                    if document
                        .frame(*id)
                        .is_some_and(|frame| frame.placement.is_anchored()) =>
                {
                    push_frame(&mut found, *id, StreamKind::AnchoredFrame);
                }
                InlineObject::FloatingHeaderFooter { flow, .. } => floating.push(*flow),
                _ => {}
            }
        }
    }
    found.extend(floating.into_iter().map(|flow| Stream {
        flow,
        kind: StreamKind::FloatingHeaderFooter,
        frame: None,
    }));
    for kind in [StreamKind::Header, StreamKind::Footer] {
        for layout in document.layouts() {
            let sides = if kind == StreamKind::Header {
                &layout.header
            } else {
                &layout.footer
            };
            for id in [sides.right, sides.left].into_iter().flatten() {
                push_frame(&mut found, id, kind);
            }
        }
    }
    for layout in document.layouts() {
        for &id in &layout.repeating {
            push_frame(&mut found, id, StreamKind::RepeatingFrame);
        }
    }
    found
}

/// The flows a frame's content holds: its text, or its table's cells row
/// by row.
fn frame_flows(content: &FrameContent) -> Vec<FlowId> {
    match content {
        FrameContent::Text(flow) => vec![*flow],
        FrameContent::Table(table) => table
            .cells
            .iter()
            .filter(|cell| !cell.span.is_covered())
            .map(|cell| match cell.content {
                CellContent::Text(flow) => flow,
            })
            .collect(),
        FrameContent::Picture(_) | FrameContent::Legacy(_) => Vec::new(),
    }
}

/// The streams a search started in stream `from` goes through, in order
/// (`FIND_NOTES.md`; the guide, ch. 20): the caret's stream alone without
/// Include other text streams; with it, the caret's and those after it
/// (from the main text, every stream), or with Beginning of document every
/// stream. A flow that is no stream (a covered cell) is searched alone.
pub fn search_streams(document: &Document, from: FlowId, options: &FindOptions) -> Vec<FlowId> {
    let all = streams(document);
    let Some(at) = all.iter().position(|stream| stream.flow == from) else {
        return vec![from];
    };
    if !options.other_streams {
        return vec![from];
    }
    // Beginning of document with the other streams: the whole document,
    // from the top of the first page (the guide).
    let first = if options.from_beginning { 0 } else { at };
    all[first..].iter().map(|stream| stream.flow).collect()
}

/// One element of a pattern.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Token {
    Char(char),
    /// `?`: one character.
    One,
    /// `*`: one or more characters of a word, never none (the guide; FR3).
    Many,
    Tab,
    /// A paragraph's end or a line break.
    Return,
}

/// A Find or Replace with text in Ami Pro's syntax: `?` and `*` wildcards;
/// `<?>`, `<*>`, `<»>`, `<¶>` for the characters themselves; `»` or a tab for a
/// tab, `¶` or a line break for a return.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pattern {
    tokens: Vec<Token>,
}

impl Pattern {
    pub fn parse(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let mut tokens = Vec::new();
        let mut index = 0;
        while index < chars.len() {
            let c = chars[index];
            // A CRLF is one return.
            if c == '\r' && chars.get(index + 1) == Some(&'\n') {
                index += 1;
                continue;
            }
            if c == '<'
                && let (Some(inner), Some('>')) = (chars.get(index + 1), chars.get(index + 2))
                && matches!(*inner, '?' | '*' | TAB_MARK | RETURN_MARK)
            {
                tokens.push(Token::Char(*inner));
                index += 3;
                continue;
            }
            tokens.push(match c {
                '?' => Token::One,
                '*' => Token::Many,
                TAB_MARK | '\t' => Token::Tab,
                RETURN_MARK | '\n' | '\r' | '\u{2028}' => Token::Return,
                c => Token::Char(c),
            });
            index += 1;
        }
        // Text from input never names an object.
        tokens.retain(|token| *token != Token::Char(OBJECT_REPLACEMENT_CHARACTER));
        Self { tokens }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

/// A flow's text as one sequence of characters, a return between
/// paragraphs, with each character's place.
struct StreamText {
    chars: Vec<char>,
    /// Each character's paragraph and byte, and one more for the end.
    places: Vec<(usize, usize)>,
}

impl StreamText {
    fn of(document: &Document, flow: FlowId) -> Option<Self> {
        let flow = document.flow(flow)?;
        let mut chars = Vec::new();
        let mut places = Vec::new();
        for (index, paragraph) in flow.paragraphs.iter().enumerate() {
            if index > 0 {
                chars.push('\n');
                places.push((index - 1, flow.paragraphs[index - 1].text.len()));
            }
            for (byte, c) in paragraph.text.char_indices() {
                chars.push(c);
                places.push((index, byte));
            }
        }
        let last = flow.paragraphs.len().saturating_sub(1);
        places.push((last, flow.paragraphs.get(last).map_or(0, |p| p.text.len())));
        Some(Self { chars, places })
    }

    fn position(&self, flow: FlowId, index: usize) -> TextPosition {
        let (paragraph, byte) = self.places[index];
        TextPosition {
            flow,
            paragraph,
            byte,
        }
    }

    /// The character index at or after `at`.
    fn index_of(&self, at: TextPosition) -> usize {
        self.places
            .partition_point(|place| *place < (at.paragraph, at.byte))
    }
}

/// A match: where it lies, and what each wildcard matched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Match {
    pub start: TextPosition,
    pub end: TextPosition,
    /// The text each wildcard matched, in order.
    pub captures: Vec<String>,
    /// The text found.
    pub text: String,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric()
}

/// A character in small letters, for matching without Exact case: its
/// lower case where that is one character (each text is folded once per
/// search, not at every comparison).
fn folded(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(lower), None) => lower,
        _ => c,
    }
}

/// Matches `tokens` at `at`, returning the end; `captures` gets each
/// wildcard's characters. `*` takes as many of the word's characters as
/// still let the rest match.
fn match_here(
    tokens: &[Token],
    text: &[char],
    at: usize,
    captures: &mut Vec<Range<usize>>,
) -> Option<usize> {
    let Some((first, rest)) = tokens.split_first() else {
        return Some(at);
    };
    let next = text.get(at).copied();
    match *first {
        Token::Char(c) => {
            next.filter(|n| *n != '\n' && *n == c)?;
            match_here(rest, text, at + 1, captures)
        }
        Token::Tab => {
            next.filter(|n| *n == '\t')?;
            match_here(rest, text, at + 1, captures)
        }
        Token::Return => {
            next.filter(|n| *n == '\n' || *n == '\u{2028}')?;
            match_here(rest, text, at + 1, captures)
        }
        Token::One => {
            next.filter(|n| *n != '\n' && *n != OBJECT_REPLACEMENT_CHARACTER)?;
            captures.push(at..at + 1);
            let end = match_here(rest, text, at + 1, captures);
            if end.is_none() {
                captures.pop();
            }
            end
        }
        Token::Many => {
            let mut stop = at;
            while stop < text.len() && is_word(text[stop]) {
                stop += 1;
            }
            for end in (at + 1..=stop).rev() {
                captures.push(at..end);
                if let Some(found) = match_here(rest, text, end, captures) {
                    return Some(found);
                }
                captures.pop();
            }
            None
        }
    }
}

/// Every match of `pattern` in `flow`, first to last, none overlapping.
fn matches_in(
    document: &Document,
    flow: FlowId,
    pattern: &Pattern,
    options: &FindOptions,
    within: Option<(TextPosition, TextPosition)>,
) -> Vec<Match> {
    let Some(text) = StreamText::of(document, flow) else {
        return Vec::new();
    };
    if pattern.is_empty() {
        return Vec::new();
    }
    let (first, last) = within.map_or((0, text.chars.len()), |(start, end)| {
        (text.index_of(start), text.index_of(end))
    });
    // Without Exact case, the text and the pattern in small letters.
    let lower: Vec<char>;
    let (chars, tokens): (&[char], Vec<Token>) = if options.match_case {
        (&text.chars, pattern.tokens.clone())
    } else {
        lower = text.chars.iter().map(|c| folded(*c)).collect();
        let tokens = pattern
            .tokens
            .iter()
            .map(|token| match token {
                Token::Char(c) => Token::Char(folded(*c)),
                other => *other,
            })
            .collect();
        (&lower, tokens)
    };
    let mut found = Vec::new();
    let mut at = first;
    while at < last {
        let mut captures = Vec::new();
        let end = match_here(&tokens, chars, at, &mut captures)
            .filter(|end| *end > at && *end <= last)
            .filter(|end| {
                !options.whole_word
                    || ((at == 0 || !is_word(text.chars[at - 1]))
                        && text.chars.get(*end).is_none_or(|c| !is_word(*c)))
            })
            .filter(|end| {
                options.find_attributes.is_none_or(|wanted| {
                    attributes_throughout(document, flow, &text, at..*end) == Some(wanted)
                })
            });
        match end {
            Some(end) => {
                let take = |range: Range<usize>| text.chars[range].iter().collect::<String>();
                found.push(Match {
                    start: text.position(flow, at),
                    end: text.position(flow, end),
                    captures: captures.into_iter().map(take).collect(),
                    text: take(at..end),
                });
                at = end;
            }
            None => at += 1,
        }
    }
    found
}

/// The attributes every character of `range` shows, if they all show the
/// same (Find Exact attributes); returns, tabs, and objects are passed
/// over.
fn attributes_throughout(
    document: &Document,
    flow: FlowId,
    text: &StreamText,
    range: Range<usize>,
) -> Option<Attributes> {
    let paragraphs = &document.flow(flow)?.paragraphs;
    let mut seen: Option<Attributes> = None;
    for index in range {
        let c = text.chars[index];
        if c == '\n' || c == '\t' || c == OBJECT_REPLACEMENT_CHARACTER {
            continue;
        }
        let (paragraph, byte) = text.places[index];
        let paragraph = paragraphs.get(paragraph)?;
        let style = document.styles().get(&paragraph.style)?;
        let format = paragraph.format_at(byte);
        let these = shown_attributes(format, style);
        if seen.is_some_and(|seen| seen != these) {
            return None;
        }
        seen = Some(these);
    }
    seen
}

/// The five attributes text with `format` shows in `style`.
pub fn shown_attributes(
    format: &amipro_document::CharacterFormat,
    style: &amipro_document::ParagraphStyle,
) -> Attributes {
    use crate::CharacterAttribute as A;
    Attributes {
        bold: A::Bold.is_on(format, style),
        italic: A::Italic.is_on(format, style),
        underline: A::Underline.is_on(format, style),
        word_underline: A::WordUnderline.is_on(format, style),
        small_caps: A::Caps(amipro_document::Caps::Small).is_on(format, style),
    }
}

/// Every match of `find` in `streams`, stream by stream, each first to
/// last; with `within`, only between those positions of their flow (a
/// selection). With [`FindOptions::styles`], `find` names a paragraph
/// style and each paragraph of it is a match.
pub fn find_all(
    document: &Document,
    find: &str,
    streams: &[FlowId],
    options: &FindOptions,
    within: Option<(TextPosition, TextPosition)>,
) -> Vec<Match> {
    if options.styles {
        let Some(style) = style_named(document, find) else {
            return Vec::new();
        };
        return streams
            .iter()
            .filter(|flow| within.is_none_or(|(start, _)| start.flow == **flow))
            .flat_map(|flow| style_matches(document, *flow, style, within))
            .collect();
    }
    let pattern = Pattern::parse(find);
    streams
        .iter()
        .flat_map(|flow| {
            let range = within.filter(|(start, _)| start.flow == *flow);
            if within.is_some() && range.is_none() {
                return Vec::new();
            }
            matches_in(document, *flow, &pattern, options, range)
        })
        .collect()
}

/// The paragraph style named `name`, whatever its capitals.
pub fn style_named(document: &Document, name: &str) -> Option<StyleId> {
    let name = name.trim();
    document
        .styles()
        .values()
        .find(|style| style.name.eq_ignore_ascii_case(name))
        .map(|style| style.id)
}

/// The paragraphs of `flow` in `style`, each a whole-paragraph match.
fn style_matches(
    document: &Document,
    flow: FlowId,
    style: StyleId,
    within: Option<(TextPosition, TextPosition)>,
) -> Vec<Match> {
    let Some(content) = document.flow(flow) else {
        return Vec::new();
    };
    content
        .paragraphs
        .iter()
        .enumerate()
        .filter(|(index, paragraph)| {
            paragraph.style == style
                && within
                    .is_none_or(|(start, end)| (start.paragraph..=end.paragraph).contains(index))
        })
        .map(|(index, paragraph)| Match {
            start: TextPosition {
                flow,
                paragraph: index,
                byte: 0,
            },
            end: TextPosition {
                flow,
                paragraph: index,
                byte: paragraph.text.len(),
            },
            captures: Vec::new(),
            text: paragraph.text.clone(),
        })
        .collect()
}

/// What replaces `found`: `replace` with each wildcard filled with what the
/// wildcard in the same place among Find's matched (the guide: `*eet*` and
/// `meet*` turn "sheets" into "meetsh"), its codes as text (a return as a
/// line feed, which [`Command::ReplaceText`] makes a paragraph break), and
/// then, with [`FindOptions::keep_capitals`], set in the found text's
/// capitals.
pub fn replacement(found: &Match, replace: &str, options: &FindOptions) -> String {
    let pattern = Pattern::parse(replace);
    let mut captures = found.captures.iter();
    let mut text = String::new();
    for token in &pattern.tokens {
        match token {
            Token::Char(c) => text.push(*c),
            Token::Tab => text.push('\t'),
            Token::Return => text.push('\n'),
            Token::One | Token::Many => {
                if let Some(capture) = captures.next() {
                    text.push_str(capture);
                }
            }
        }
    }
    if options.keep_capitals {
        set_in_capitals_of(&text, &found.text)
    } else {
        text
    }
}

/// Ami Pro's capitals rule (VM FR1, FR1B): a found text in capitals (two
/// letters or more, all capitals) gives the replacement in capitals; one
/// whose first letter is a capital capitalizes each word of the
/// replacement, the rest of each word in small letters ("aN" for "The" is
/// "An"); otherwise the replacement is in small letters.
pub fn set_in_capitals_of(replacement: &str, found: &str) -> String {
    let letters: Vec<char> = found.chars().filter(|c| c.is_alphabetic()).collect();
    let Some(first) = letters.first() else {
        return replacement.to_owned();
    };
    if letters.len() >= 2 && letters.iter().all(|c| !c.is_lowercase()) {
        return replacement.to_uppercase();
    }
    if first.is_uppercase() {
        let mut text = String::new();
        let mut start = true;
        for c in replacement.chars() {
            if c.is_alphanumeric() {
                if start {
                    text.extend(c.to_uppercase());
                } else {
                    text.extend(c.to_lowercase());
                }
                start = false;
            } else {
                text.push(c);
                start = true;
            }
        }
        return text;
    }
    replacement.to_lowercase()
}

/// The commands that replace one match: its text, then with Replace Exact
/// attributes the replacement's attributes; for a style search, the
/// paragraph's style.
pub fn replace_commands(
    document: &Document,
    found: &Match,
    replace: &str,
    options: &FindOptions,
) -> Vec<Command> {
    if options.styles {
        return style_named(document, replace)
            .map(|style| Command::SetParagraphStyle {
                flow: found.start.flow,
                first: found.start.paragraph,
                last: found.start.paragraph,
                style,
            })
            .into_iter()
            .collect();
    }
    let text = replacement(found, replace, options);
    let mut commands = vec![Command::ReplaceText {
        start: found.start,
        end: found.end,
        text: text.clone(),
    }];
    let end = end_of_inserted(found.start, &text);
    if let Some(wanted) = options.replace_attributes {
        if end != found.start {
            commands.extend(attribute_commands(found.start, end, wanted));
        }
    } else if end != found.start
        && let Some(format) = format_at(document, found.start)
    {
        // The replacement takes the found text's formatting (its first
        // character's), as text typed over a selection does: an italic
        // word replaced stays italic.
        commands.push(Command::FormatCharacters {
            start: found.start,
            end,
            format,
        });
    }
    commands
}

/// The direct character formatting of the character at `at`, if any.
fn format_at(document: &Document, at: TextPosition) -> Option<amipro_document::CharacterFormat> {
    document
        .flow(at.flow)?
        .paragraph(at.paragraph)?
        .runs
        .iter()
        .find(|run| run.start <= at.byte && at.byte < run.end)
        .map(|run| run.format.clone())
}

/// Where text inserted at `start` ends: lines after the first start their
/// own paragraphs.
fn end_of_inserted(start: TextPosition, text: &str) -> TextPosition {
    let mut lines = text.split('\n');
    let first = lines.next().unwrap_or_default();
    let mut end = TextPosition {
        byte: start.byte + first.len(),
        ..start
    };
    for line in lines {
        end = TextPosition {
            flow: start.flow,
            paragraph: end.paragraph + 1,
            byte: line.len(),
        };
    }
    end
}

/// Sets exactly `wanted` of the five attributes between two positions:
/// Normal turns each off.
fn attribute_commands(start: TextPosition, end: TextPosition, wanted: Attributes) -> Vec<Command> {
    use crate::CharacterAttribute as A;
    [
        (A::Bold, wanted.bold),
        (A::Italic, wanted.italic),
        (A::Underline, wanted.underline),
        (A::WordUnderline, wanted.word_underline),
        (A::Caps(amipro_document::Caps::Small), wanted.small_caps),
    ]
    .into_iter()
    .map(|(attribute, on)| Command::SetCharacterAttribute {
        start,
        end,
        attribute,
        on,
    })
    .collect()
}

/// Replace All: every match replaced as one step, the last first so that
/// each match is still where it was found. `None` when there is nothing to
/// replace.
pub fn replace_all(
    document: &Document,
    found: &[Match],
    replace: &str,
    options: &FindOptions,
) -> Option<Command> {
    let commands: Vec<Command> = found
        .iter()
        .rev()
        .flat_map(|found| replace_commands(document, found, replace, options))
        .collect();
    (!commands.is_empty()).then_some(Command::Several(commands))
}
