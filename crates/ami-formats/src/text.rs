//! SAM paragraph text: splitting a body into paragraphs and decoding one
//! paragraph's escapes, character runs, inline objects, and unmapped controls
//! (`docs/research/SAM_FORMAT_NOTES.md`, "Document body";
//! `docs/research/SAM_PAGE_FRAME_SPEC.md`, section 5).

use std::collections::{BTreeMap, BTreeSet};

use amipro_document::{
    Alignment, Caps, CharacterFormat, CharacterRun, Colour, DateField, DateSource, DocInfoField,
    HeaderFooterKind, Indents, LegacyMark, LineSpacing, OBJECT_REPLACEMENT_CHARACTER,
    PageNumberField, PageSides, ParagraphFormat,
};

use crate::decode_windows_1252;

/// One paragraph of SAM text, before styles and objects are resolved.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ParsedParagraph {
    /// The `@Style@` tag, if any. Untagged paragraphs are Body Text.
    pub style: Option<String>,
    pub text: String,
    /// `<:I…>`, `<:S…>`, and `<+@>`…`<+C>`: the paragraph's own indents,
    /// line spacing, and alignment.
    pub format: ParagraphFormat,
    pub runs: Vec<CharacterRun>,
    /// One per U+FFFC in `text`, in order.
    pub objects: Vec<PendingObject>,
    pub marks: Vec<LegacyMark>,
    /// The character format in force at the paragraph's end.
    pub end_format: CharacterFormat,
    /// The height Ami Pro cached for the paragraph (`<:#h,w>`), in twips:
    /// the line heights are calibrated from it (`DECISIONS.md`, "Calibrated
    /// line heights").
    pub cached_height: Option<i32>,
    /// The whole cache, height and width, kept for the SAM writer.
    pub cache: Option<(i32, i32)>,
}

/// An inline object as SAM writes it, before frames and layouts have IDs.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PendingObject {
    /// `<:A n>` or `<:t n>`: the n-th anchored frame record.
    Anchor { index: usize, raw: String },
    /// `<:p…>`: a page break, maybe centring and switching to a named layout.
    PageBreak {
        centre: bool,
        restart: bool,
        layout: Option<String>,
        raw: String,
    },
    /// `<:c>`.
    ColumnBreak,
    /// `<:H…>` or `<:h…>` with its own paragraphs.
    Floating {
        kind: HeaderFooterKind,
        pages: PageSides,
        paragraphs: Vec<ParsedParagraph>,
    },
    /// `<:P STYLE,START,TEXT>`.
    PageNumber(PageNumberField),
    /// `<:B<*X>>`: one of Ami Pro's bullets, put into the text.
    Bullet(char),
    /// `<:Dx>`, `<:rx>`, `<:kx>`, `<:b…>`: a date or the time.
    Date(DateField),
    /// `<:d…>`: a Doc Info entry.
    DocInfo(DocInfoField),
    /// `<:X4,…>`: a Seq power field.
    Seq(amipro_document::SeqField),
    /// `<:N…>`: a note's fields and its paragraphs.
    Note {
        note: amipro_document::NoteInfo,
        paragraphs: Vec<ParsedParagraph>,
    },
    /// `<:F`: a footnote's paragraphs (the main text only).
    Footnote { paragraphs: Vec<ParsedParagraph> },
    /// `<:xN,ROW>`: a footnote's reference in a table cell.
    TableFootnote { number: u16, row: u16 },
}

/// What paragraph parsing found that the model does not map, for the report.
#[derive(Debug, Default)]
pub(crate) struct TextReport {
    /// Distinct unmapped controls, by their first line.
    pub unmapped: BTreeSet<String>,
    pub legacy_marks: usize,
    pub dropped_caches: usize,
    pub anchors_resolved: usize,
    /// Anchor marks that name no anchored frame record, or one already
    /// anchored. They are kept as legacy marks.
    pub anchors_unresolved: usize,
    /// Controls whose `<` has no matching `>`, kept as text.
    pub unbalanced_controls: usize,
    /// The pitch and family bytes `<:f…>` gave each face (K − 0x20), by
    /// lower-case name, counted.
    pub face_kinds: FaceKindVotes,
}

/// Pitch and family bytes seen for each face (lower-case name), counted.
pub(crate) type FaceKindVotes = BTreeMap<String, BTreeMap<u8, usize>>;

/// Where a paragraph is: the main text may hold anchors, breaks, and
/// floating headers; other text keeps those controls as legacy marks.
pub(crate) struct ParseContext {
    pub main: bool,
    /// The number of anchored frame records (`<:A n>`/`<:t n>` index them).
    pub anchored_frames: usize,
    pub used_anchors: BTreeSet<usize>,
}

impl ParseContext {
    pub(crate) fn outside_main() -> Self {
        Self {
            main: false,
            anchored_frames: 0,
            used_anchors: BTreeSet::new(),
        }
    }
}

/// Splits body lines (a `[edoc]` body, a frame's `[txt]`, or a table cell)
/// into logical paragraphs, and returns them with the number of lines read,
/// including the closing `>` line.
///
/// As Ami Pro's body reader does (decompile `1118:0e55`, `1118:0920`; VM
/// fixture EP1): a line is everything up to its line break, nothing
/// trimmed. A paragraph starts at the next line. A line that is only a
/// style tag (`@Body Single@`) is a whole empty paragraph, and so is an
/// empty line where a paragraph starts (an empty Body Text paragraph).
/// Otherwise the paragraph runs on to the next empty line, which ends it,
/// and its physical lines are joined without a separator because Ami Pro
/// keeps the trailing space. The body ends at a line that is exactly `>`
/// (or an `[Embedded]` line) outside any control. Controls such as notes
/// can span lines and contain their own empty and `>` lines; line breaks
/// inside a control are kept as `\n`.
pub(crate) fn split_body<'a>(lines: impl Iterator<Item = &'a str>) -> (Vec<String>, usize) {
    let mut paragraphs = Vec::new();
    // The paragraph being read, if one has started.
    let mut current: Option<String> = None;
    let mut depth = 0usize;
    let mut consumed = 0;
    for line in lines {
        consumed += 1;
        if depth == 0 {
            if line == ">" || line.trim().eq_ignore_ascii_case("[embedded]") {
                break;
            }
            match current.as_mut() {
                None if line.is_empty() || is_bare_tag(line) => {
                    paragraphs.push(line.to_owned());
                    continue;
                }
                None => current = Some(String::new()),
                Some(_) if line.is_empty() => {
                    paragraphs.extend(current.take());
                    continue;
                }
                Some(_) => {}
            }
        } else if let Some(paragraph) = current.as_mut() {
            paragraph.push('\n');
        }
        current.get_or_insert_with(String::new).push_str(line);
        depth = control_depth_after(line, depth);
    }
    paragraphs.extend(current);
    (paragraphs, consumed)
}

/// Controls that span lines (notes, floating headers and footers) whose
/// closing `>` is not alone on its line. Ami Pro reads such a control's
/// paragraphs until a lone `>` line (its writer, `11b0:0660`, always ends
/// the line there), so text after the `>` on the same line makes it read
/// on into the rest of the document. This reader still ends the control
/// at the `>`; the count lets the import report it.
pub(crate) fn controls_ending_mid_line<'a>(lines: impl Iterator<Item = &'a str>) -> usize {
    let mut depth = 0usize;
    let mut found = 0;
    for line in lines {
        if depth == 0 && line == ">" {
            break;
        }
        let after = control_depth_after(line, depth);
        if depth > 0 && after == 0 && line != ">" {
            found += 1;
        }
        depth = after;
    }
    found
}

/// Whether `line` is only a style tag, `@NAME@`, which Ami Pro writes for
/// an empty paragraph in that style without a blank line after it (an
/// escaped `@@` starts text, not a tag).
fn is_bare_tag(line: &str) -> bool {
    line.strip_prefix('@')
        .filter(|rest| !rest.starts_with('@'))
        .and_then(|rest| rest.find('@').map(|end| end + 1 == rest.len()))
        .unwrap_or(false)
}

pub(crate) fn control_depth_after(line: &str, mut depth: usize) -> usize {
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '<' if chars.peek() == Some(&'<') => {
                chars.next();
            }
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

/// Decodes a one-character escape control, or returns `None` for any other
/// control. Verified against files written by Ami Pro 3.1: `<[>` is `[`,
/// `<;>` is `>`, `</X>` is Windows-1252 byte `X + 0x40` (0x80–0xBF), and
/// `<\X>` is byte `X + 0x80` (0xC0–0xFF). A literal `<` is written `<<`.
pub(crate) fn decode_escape(control: &str) -> Option<char> {
    let byte = escape_byte(control)?;
    if byte < 0x20 {
        return None;
    }
    // Byte 0xAD (`</m>`) is a hyphen Ami Pro shows and may break after,
    // as after `-` (the line breaker `10d0:1f9e`; VM fixture T02): not the
    // invisible soft hyphen Windows-1252 names, which is `<*7>`. In the
    // corpora it writes "1-2-3" (2,980 times in 35 files).
    if byte == 0xAD {
        return Some(AMI_PRO_HYPHEN);
    }
    decode_windows_1252(&[byte]).chars().next()
}

/// The character Ami Pro's byte 0xAD stands for in body text.
pub(crate) const AMI_PRO_HYPHEN: char = '\u{2010}';

/// The byte a one-character escape stands for, including `<*X>` (byte
/// X − 0x20, 0x00–0x1F; V19).
fn escape_byte(control: &str) -> Option<u8> {
    let mut chars = control.chars();
    let first = chars.next()?;
    let second = chars.next();
    if chars.next().is_some() {
        return None;
    }
    Some(match (first, second) {
        ('[', None) => b'[',
        (';', None) => b'>',
        ('/', Some(code @ '\u{40}'..='\u{7f}')) => code as u8 + 0x40,
        ('\\', Some(code @ '\u{40}'..='\u{7f}')) => code as u8 + 0x80,
        ('*', Some(code @ '\u{20}'..='\u{3f}')) => code as u8 - 0x20,
        _ => return None,
    })
}

/// The escape at the start of `text`, as its byte and the text after it.
/// `<*>>` is byte 0x1E: its character is the `>`.
pub(crate) fn leading_escape(text: &str) -> Option<(u8, &str)> {
    let rest = text.strip_prefix('<')?;
    if let Some(after) = rest.strip_prefix("*>>") {
        return Some((0x1E, after));
    }
    let end = rest.find('>')?;
    Some((escape_byte(&rest[..end])?, &rest[end + 1..]))
}

/// A style name as Ami Pro writes it, in a `[tag]` record (`11c8:0bf5`:
/// `<` doubled, other bytes escaped) or a paragraph tag (`11b0:129e`, which
/// also escapes `>` and `[`), decoded.
pub(crate) fn decode_name(raw: &str) -> String {
    let mut name = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('<') {
        name.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(after) = rest.strip_prefix("<<") {
            name.push('<');
            rest = after;
        } else if let Some((byte, after)) = leading_escape(rest) {
            name.push_str(&decode_windows_1252(&[byte]));
            rest = after;
        } else {
            name.push('<');
            rest = &rest[1..];
        }
    }
    name.push_str(rest);
    name
}

/// Reads a flag byte written raw or as an escape at the start of `text`, and
/// returns it with the rest of the text.
fn flag_byte(text: &str) -> Option<(u8, &str)> {
    if text.starts_with('<') {
        return leading_escape(text);
    }
    let mut chars = text.chars();
    let character = chars.next()?;
    u8::try_from(u32::from(character))
        .ok()
        .map(|byte| (byte, chars.as_str()))
}

/// Parses one paragraph. `report` collects unmapped controls.
pub(crate) fn parse_paragraph(
    input: &str,
    report: &mut TextReport,
    context: &mut ParseContext,
) -> ParsedParagraph {
    let mut parsed = ParsedParagraph::default();
    let mut rest = input;
    if rest.starts_with("@@") {
        // An escaped literal `@`, not a style tag; decoded below.
    } else if let Some(after_at) = rest.strip_prefix('@')
        && let Some(end) = after_at.find('@')
    {
        parsed.style = Some(decode_name(&after_at[..end]));
        rest = &after_at[end + 1..];
    }
    let chars: Vec<char> = rest.chars().collect();
    let mut format = CharacterFormat::default();
    let mut run_start = None;
    // The paragraph's alignment marks so far, combined (below).
    let mut alignment_marks = 0u8;
    // Where character format changes stand in `parsed.marks`, until the
    // runs and the end format replace them (below).
    let mut format_marks = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '@' && parsed.text.is_empty() && chars.get(index + 1) == Some(&'@') {
            // Ami Pro reads `@@` as `@` at the first text position only, after
            // any tag and leading controls; later `@@` stays literal.
            parsed.text.push('@');
            index += 2;
            continue;
        }
        if chars[index] != '<' {
            // A stray U+FFFC in legacy text is not an object.
            parsed
                .text
                .push(if chars[index] == OBJECT_REPLACEMENT_CHARACTER {
                    amipro_document::REPLACEMENT_CHARACTER
                } else {
                    chars[index]
                });
            index += 1;
            continue;
        }
        if chars.get(index + 1) == Some(&'<') {
            parsed.text.push('<');
            index += 2;
            continue;
        }
        let start = index;
        let mut depth = 1usize;
        index += 1;
        while index < chars.len() && depth > 0 {
            match chars[index] {
                '<' if chars.get(index + 1) == Some(&'<') => index += 1,
                '<' => depth += 1,
                '>' => depth -= 1,
                _ => {}
            }
            index += 1;
        }
        if depth != 0 {
            report.unbalanced_controls += 1;
            parsed
                .text
                .extend(chars[start..].iter().map(|character| match character {
                    &OBJECT_REPLACEMENT_CHARACTER => amipro_document::REPLACEMENT_CHARACTER,
                    other => *other,
                }));
            break;
        }
        let control: String = chars[start + 1..index - 1].iter().collect();
        if let Some(character) = decode_escape(&control) {
            parsed.text.push(character);
            continue;
        }
        let mut changed = format.clone();
        if character_attribute(&control, &mut changed)
            || font_change(&control, &mut changed, &mut report.face_kinds)
            || overstrike_change(&control, &mut changed)
        {
            format_marks.push(parsed.marks.len());
            parsed.marks.push(LegacyMark {
                byte: parsed.text.len(),
                raw: format!("<{control}>"),
            });
            if changed != format {
                finish_run(
                    &mut parsed.runs,
                    run_start.take(),
                    parsed.text.len(),
                    &format,
                );
                format = changed;
                run_start = (!format.is_empty()).then_some(parsed.text.len());
            }
            continue;
        }
        if paragraph_format(&control, &mut parsed.format, &mut alignment_marks) {
            continue;
        }
        match control_meaning(&control, report, context) {
            Meaning::Character(character) => parsed.text.push(character),
            Meaning::Object(object) => {
                parsed.text.push(OBJECT_REPLACEMENT_CHARACTER);
                parsed.objects.push(object);
            }
            Meaning::Dropped => {}
            Meaning::Cache(height, width) => {
                report.dropped_caches += 1;
                parsed.cached_height = parsed.cached_height.or(height);
                if parsed.cache.is_none()
                    && let (Some(height), Some(width)) = (height, width)
                {
                    parsed.cache = Some((height, width));
                }
            }
            Meaning::Unmapped => {
                // Multi-line controls such as notes are reported by their
                // first line so that note text does not flood the report.
                report
                    .unmapped
                    .insert(control.lines().next().unwrap_or_default().to_owned());
                report.legacy_marks += 1;
                parsed.marks.push(LegacyMark {
                    byte: parsed.text.len(),
                    raw: format!("<{control}>"),
                });
            }
        }
    }
    finish_run(&mut parsed.runs, run_start, parsed.text.len(), &format);
    // The runs carry the format changes, and the paragraph's end format the
    // one in force after its last character, which sets the line of a
    // paragraph without text (a `<:f200,,>` line is 10 pt high; decompile).
    for index in format_marks.into_iter().rev() {
        parsed.marks.remove(index);
    }
    parsed.end_format = format;
    parsed
}

/// Applies a Text menu attribute mark, `<+X>` on or `<-X>` off, to `format`
/// and returns whether `control` was one (VM fixtures O1, O6; decompile;
/// `SAM_FORMAT_NOTES.md`, "Character attributes"). Off returns bold,
/// italic, the underlines, and the caps to the style's setting (`None`);
/// strikethrough, superscript, subscript, protected text (`*`), and no
/// hyphenation (`.`) simply end (`Some(false)`). Each caps attribute
/// replaces the others. Marks Ami Pro ignores stay legacy marks.
fn character_attribute(control: &str, format: &mut CharacterFormat) -> bool {
    let mut chars = control.chars();
    let (Some(sign @ ('+' | '-')), Some(attribute), None) =
        (chars.next(), chars.next(), chars.next())
    else {
        return false;
    };
    let on = sign == '+';
    let returns = on.then_some(true);
    let ends = Some(on);
    let caps = |kind| if on { Some(kind) } else { None };
    match attribute {
        '!' => format.bold = returns,
        '"' => format.italic = returns,
        '#' => format.underline = returns,
        '$' => format.word_underline = returns,
        ')' => format.double_underline = returns,
        '%' => format.strikethrough = ends,
        '&' => format.superscript = ends,
        '\'' => format.subscript = ends,
        '+' => format.caps = caps(Caps::Upper),
        ',' => format.caps = caps(Caps::Lower),
        '-' => format.caps = caps(Caps::Initial),
        '(' => format.caps = caps(Caps::Small),
        '*' => format.protected = ends,
        '.' => format.no_hyphenation = ends,
        _ => return false,
    }
    true
}

/// Applies Text › Special Effects' overstrike, `<:O+C>` on with character
/// C or `<:O-C>` off, and returns whether `control` was one (O1; decompile
/// `11b8:030a`, `10d0:2d0e`: a new character replaces the last, and off
/// ends it whatever its character). C may be an escape.
fn overstrike_change(control: &str, format: &mut CharacterFormat) -> bool {
    let Some(rest) = control.strip_prefix(":O") else {
        return false;
    };
    let mut chars = rest.chars();
    let (Some(sign @ ('+' | '-')), character) = (chars.next(), chars.as_str()) else {
        return false;
    };
    let decoded = match character {
        "<<" => Some('<'),
        _ if character.starts_with('<') && character.ends_with('>') => {
            decode_escape(&character[1..character.len() - 1])
        }
        _ => {
            let mut one = character.chars();
            one.next().filter(|_| one.next().is_none())
        }
    };
    let Some(decoded) = decoded else {
        return false;
    };
    format.overstrike = (sign == '+').then_some(decoded);
    true
}

/// Applies Text › Font's `<:fSIZE,KFACE,R,G,B>` to `format` and returns
/// whether `control` was one (O4, O5; decompile). Each font change replaces
/// the one before, so `<:f>` returns to the style's font: an empty field is
/// the style's. SIZE is in twentieths of a point; K, the face's Windows
/// pitch and family plus 0x20, only picks a substitute for a missing face:
/// it is counted in `kinds` for the document's face kinds. Ami Pro stops
/// reading at a field with a character other
/// than a digit where a number belongs, keeping what it read before, and
/// takes the colour only when all three numbers read (each modulo 256).
fn font_change(control: &str, format: &mut CharacterFormat, kinds: &mut FaceKindVotes) -> bool {
    let Some(arguments) = control.strip_prefix(":f") else {
        return false;
    };
    format.font_size_points = None;
    format.font_family = None;
    format.colour = None;
    let number = |field: &str| {
        field.bytes().try_fold(0u32, |value, digit| {
            digit
                .is_ascii_digit()
                .then(|| value.wrapping_mul(10).wrapping_add(u32::from(digit - b'0')))
        })
    };
    let mut fields = arguments.splitn(3, ',');
    let (Some(size), Some(face)) = (fields.next(), fields.next()) else {
        return true;
    };
    let Some(size) = number(size) else {
        return true;
    };
    // Sizes are 16-bit twentieths; 0 is no size.
    format.font_size_points = (1..=0x7FFF).contains(&size).then(|| size as f32 / 20.0);
    // The first character is K; the face has at most 31 characters.
    let name: String = face.chars().skip(1).take(31).collect();
    format.font_family = (!name.trim().is_empty()).then(|| name.trim().to_owned());
    if let (Some(family), Some(kind)) = (
        &format.font_family,
        face.chars()
            .next()
            .and_then(|kind| u8::try_from(u32::from(kind).checked_sub(0x20)?).ok()),
    ) {
        *kinds
            .entry(family.to_lowercase())
            .or_default()
            .entry(kind)
            .or_default() += 1;
    }
    if face.chars().count() > 32 {
        return true;
    }
    let colour: Vec<_> = fields
        .next()
        .filter(|colour| !colour.is_empty())
        .map(|colour| colour.split(',').map(number).collect())
        .unwrap_or_default();
    if let [Some(red), Some(green), Some(blue)] = colour[..] {
        format.colour = Some(((blue & 0xFF) << 16 | (green & 0xFF) << 8 | (red & 0xFF)) as Colour);
    }
    true
}

/// Reads Text › Alignment (below), Text › Indention (`<:I
/// ALL,FIRST,REST,RIGHT>`, twips), and Text › Spacing (`<:S+-1>` Single, `<:S+-2>` 1 1/2, `<:S+-3>` Double, `<:S+N>`
/// Custom N twips) into `format`, and returns whether `control` was one
/// (VM fixtures O3). Either applies to its whole paragraph wherever it
/// stands, and a later one replaces an earlier one (O6). `<:S->` changes
/// nothing (O5) and stays a legacy mark.
fn paragraph_format(control: &str, format: &mut ParagraphFormat, alignment_marks: &mut u8) -> bool {
    // Text › Alignment: `<+@>` left, `<+A>` right, `<+B>` centre, `<+C>`
    // justify, with no off mark (O2). Ami Pro ORs a paragraph's marks into
    // one field, 1 left, 2 right, 3 centre, 4 justify; any other result is
    // the style's alignment (decompile; fits O6 and O7, where centre won).
    let code = match control {
        "+@" => Some(1),
        "+A" => Some(2),
        "+B" => Some(3),
        "+C" => Some(4),
        _ => None,
    };
    if let Some(code) = code {
        *alignment_marks |= code;
        format.alignment = match *alignment_marks {
            1 => Some(Alignment::Left),
            2 => Some(Alignment::Right),
            3 => Some(Alignment::Centre),
            4 => Some(Alignment::Justify),
            _ => None,
        };
        return true;
    }
    if let Some(values) = control.strip_prefix(":I") {
        let values: Vec<_> = values
            .split(',')
            .map(|value| value.trim().parse::<i32>().ok())
            .collect();
        if let [Some(all), Some(first), Some(rest), Some(right)] = values[..] {
            format.indents = Some(Indents {
                all,
                first,
                rest,
                right,
            });
            return true;
        }
    } else if let Some(values) = control.strip_prefix(":R") {
        // A tab ruler for this paragraph (decompile `11b8:030a`): a flag,
        // which must not be 0, a count of stops (22 kept), and that many
        // type and position pairs; a trailing comma ends it.
        let numbers: Vec<Option<i64>> = values
            .split(',')
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().parse().ok())
            .collect();
        if let [Some(flag), Some(count), pairs @ ..] = &numbers[..]
            && *flag != 0
            && *count >= 0
            && pairs.len() >= 2 * *count as usize
        {
            let count = (*count as usize).min(22);
            let tabs = pairs[..2 * count]
                .chunks(2)
                .filter_map(|pair| crate::tab_stop(pair[0]?, pair[1]?))
                .collect();
            format.tabs = Some(tabs);
            return true;
        }
    } else if let Some(value) = control.strip_prefix(":S+") {
        let line = match value.trim().parse::<i32>() {
            Ok(-1) => LineSpacing::Single,
            Ok(-2) => LineSpacing::OneAndAHalf(0),
            Ok(-3) => LineSpacing::Double(0),
            Ok(height) if height > 0 => LineSpacing::Custom(height),
            _ => return false,
        };
        format.line_spacing = Some(line);
        return true;
    }
    false
}

enum Meaning {
    Character(char),
    Object(PendingObject),
    /// A control Ami Pro's reader drops (Seq's cached result, `<:X~4;N>`).
    Dropped,
    /// `<:#HEIGHT,WIDTH>`, with the height and width when they read.
    Cache(Option<i32>, Option<i32>),
    Unmapped,
}

fn control_meaning(control: &str, report: &mut TextReport, context: &mut ParseContext) -> Meaning {
    // `<*X>` special characters (V19): hard return and discretionary hyphen.
    match escape_byte(control) {
        Some(0x0A) => return Meaning::Character('\u{2028}'),
        Some(0x17) => return Meaning::Character('\u{AD}'),
        Some(_) => return Meaning::Unmapped,
        None => {}
    }
    let Some(body) = control.strip_prefix(':') else {
        return Meaning::Unmapped;
    };
    let mut chars = body.chars();
    let Some(kind) = chars.next() else {
        return Meaning::Unmapped;
    };
    let arguments = chars.as_str();
    if !context.main && matches!(kind, 'A' | 't' | 'c' | 'p' | 'H' | 'h' | 'F') {
        // Anchors, breaks, floating headers, and footnotes belong to the
        // main text only.
        return Meaning::Unmapped;
    }
    match kind {
        '#' => {
            let mut values = arguments.split(',').map(|value| value.trim().parse().ok());
            let height = values.next().flatten();
            let width = values.next().flatten();
            Meaning::Cache(height, width)
        }
        'A' | 't' => match arguments.trim().parse::<usize>() {
            Ok(index) if index < context.anchored_frames && context.used_anchors.insert(index) => {
                report.anchors_resolved += 1;
                Meaning::Object(PendingObject::Anchor {
                    index,
                    raw: format!("<{control}>"),
                })
            }
            Ok(_) => {
                report.anchors_unresolved += 1;
                Meaning::Unmapped
            }
            Err(_) => Meaning::Unmapped,
        },
        'c' if arguments.is_empty() => Meaning::Object(PendingObject::ColumnBreak),
        'p' => match flag_byte(arguments) {
            // 0x40 centres the text above the break; 0x80 means a layout name
            // follows (G20, G21, V23).
            // 0x02 marks the break after a generated TOC, after which
            // page numbers start again (TI1T2, TI3T).
            Some((flags, rest)) if flags & !0xC2 == 0 => {
                let named = flags & 0x80 != 0;
                if named == rest.is_empty() {
                    return Meaning::Unmapped;
                }
                Meaning::Object(PendingObject::PageBreak {
                    centre: flags & 0x40 != 0,
                    restart: flags & 0x02 != 0,
                    layout: named.then(|| rest.to_owned()),
                    raw: format!("<{control}>"),
                })
            }
            _ => Meaning::Unmapped,
        },
        'H' | 'h' => floating(kind, arguments, report).map_or(Meaning::Unmapped, Meaning::Object),
        'N' => note(arguments, report).map_or(Meaning::Unmapped, Meaning::Object),
        // A footnote has no fields: its paragraphs follow at once
        // (`11b8:030a`; VM fixture RNF1).
        'F' => match arguments.strip_prefix('\n') {
            Some(body) => Meaning::Object(PendingObject::Footnote {
                paragraphs: inner_paragraphs(body, report),
            }),
            None => Meaning::Unmapped,
        },
        // A table footnote's number in the table and the row holding its
        // text (VM fixture RNF6).
        'x' => {
            let mut fields = arguments.split(',');
            match (
                fields.next().and_then(canonical_number),
                fields.next().and_then(canonical_number),
                fields.next(),
            ) {
                (Some(number), Some(row), None) => Meaning::Object(PendingObject::TableFootnote {
                    number: number as u16,
                    row: row as u16,
                }),
                _ => Meaning::Unmapped,
            }
        }
        // An inserted bullet (`1560:0122`; VM fixture BL1): the code as text
        // writes a code, one of Ami Pro's 17. Any other form stays a mark.
        'B' => match leading_escape(arguments) {
            Some((byte, "")) if amipro_document::is_bullet_code(char::from(byte)) => {
                Meaning::Object(PendingObject::Bullet(char::from(byte)))
            }
            _ => Meaning::Unmapped,
        },
        // A Seq field is an object, as its mark draws its number; the cached
        // number Ami Pro writes after it is dropped, as its reader drops it
        // (`11b8:030a`, `1628:0115`; VM fixtures FD3, FD6).
        'X' => match amipro_document::PowerMark::parse(&format!("<{control}>")) {
            Some(mark) if mark.kind == amipro_document::power_kind::SEQ && !mark.end => {
                Meaning::Object(PendingObject::Seq(amipro_document::SeqField {
                    code: mark.code,
                    value: mark.value,
                }))
            }
            None if arguments.starts_with("~4;") => Meaning::Dropped,
            _ => Meaning::Unmapped,
        },
        // A date or the time: the letter and one byte, the format letter or
        // for the time a byte Ami Pro ignores (`11b8:030a`; VM fixture FD1).
        'D' | 'r' | 'k' | 'b' => match argument_bytes(arguments).as_deref() {
            Some(&[format]) => Meaning::Object(PendingObject::Date(DateField {
                source: match kind {
                    'D' => DateSource::Today,
                    'r' => DateSource::LastSaved,
                    'k' => DateSource::Created,
                    _ => DateSource::Time,
                },
                format,
            })),
            _ => Meaning::Unmapped,
        },
        // A Doc Info entry: the date format letter, then the entry's code as
        // a 16-bit word, low byte first (`1560:0374`; VM fixture FD2).
        'd' => match argument_bytes(arguments).as_deref() {
            Some(&[format, low, high]) => Meaning::Object(PendingObject::DocInfo(DocInfoField {
                code: u16::from_le_bytes([low, high]),
                format,
            })),
            _ => Meaning::Unmapped,
        },
        // A style character, the start page, the start number, and the
        // leading text (`11b8:030a`, `11f8:023f`): `<:P10,0,Page >` is style
        // 1, no start page, no start number. Numbers Ami Pro would not write
        // back the same way stay legacy marks.
        'P' => {
            let mut chars = arguments.chars();
            let Some(style) = chars.next() else {
                return Meaning::Unmapped;
            };
            let mut fields = chars.as_str().splitn(3, ',');
            match (
                fields.next().and_then(canonical_number),
                fields.next().and_then(canonical_number),
                fields.next(),
            ) {
                (Some(start_page), Some(start_number), Some(text)) => {
                    Meaning::Object(PendingObject::PageNumber(PageNumberField {
                        style,
                        start_page,
                        start_number,
                        leading_text: text.to_owned(),
                    }))
                }
                _ => Meaning::Unmapped,
            }
        }
        _ => Meaning::Unmapped,
    }
}

/// A control's arguments as bytes, escapes decoded (`11b8:0000`): `<<` is
/// `<`, `<*X>` and the others their byte, anything else its Windows code
/// page byte.
fn argument_bytes(arguments: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(arguments.len());
    let mut rest = arguments;
    while let Some(character) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("<<") {
            bytes.push(b'<');
            rest = after;
        } else if character == '<' {
            let (byte, after) = leading_escape(rest)?;
            bytes.push(byte);
            rest = after;
        } else {
            bytes.push(crate::writer::windows_1252(character));
            rest = &rest[character.len_utf8()..];
        }
    }
    Some(bytes)
}

/// A decimal number as Ami Pro writes one (`%u`): digits only, no leading
/// zero, at most 65535.
fn canonical_number(text: &str) -> Option<u32> {
    let canonical = !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical
        .then(|| text.parse::<u32>().ok())
        .flatten()
        .filter(|number| *number <= 0xFFFF)
}

/// A floating header or footer: `<:H` or `<:h`, a flag byte (1 header,
/// 2 footer, 4 left pages, 8 right pages; V17, V18), a line break, its
/// paragraphs, and the control's closing `>` line.
fn floating(kind: char, arguments: &str, report: &mut TextReport) -> Option<PendingObject> {
    let (flags, rest) = flag_byte(arguments)?;
    if flags & !0x0F != 0 {
        return None;
    }
    let kind = if kind == 'H' {
        HeaderFooterKind::Header
    } else {
        HeaderFooterKind::Footer
    };
    let pages = match (flags & 4 != 0, flags & 8 != 0) {
        (true, false) => PageSides::Left,
        (false, true) => PageSides::Right,
        _ => PageSides::All,
    };
    let body = rest.strip_prefix('\n').unwrap_or(rest);
    Some(PendingObject::Floating {
        kind,
        pages,
        paragraphs: inner_paragraphs(body, report),
    })
}

/// The paragraphs of a control that holds text (a floating header, a note,
/// a footnote): its lines up to the one before the closing `>` line.
fn inner_paragraphs(body: &str, report: &mut TextReport) -> Vec<ParsedParagraph> {
    let body = body.strip_suffix('\n').unwrap_or(body);
    let (raw_paragraphs, _) = split_body(body.split('\n'));
    let mut inner = ParseContext::outside_main();
    raw_paragraphs
        .iter()
        .map(|paragraph| parse_paragraph(paragraph, report, &mut inner))
        .collect()
}

/// A note: `<:N` and its time, then either nothing more (Ami Pro's older
/// short form) or its initials, colour, number, and whether it is in the
/// main text, split on commas; a line break; its paragraphs (`11b8:030a`;
/// VM fixtures RNN1–RNN3). The main flag is where the note stands, which
/// the writer works out.
fn note(arguments: &str, report: &mut TextReport) -> Option<PendingObject> {
    let (fields, body) = arguments.split_once('\n')?;
    let fields: Vec<&str> = fields.split(',').collect();
    let time = canonical_long(fields[0])?;
    let note = match fields[..] {
        [_] => amipro_document::NoteInfo {
            time,
            initials: None,
            colour: None,
            number: None,
        },
        [_, initials, colour, number, main] => {
            if !matches!(main, "0" | "1") {
                return None;
            }
            let initials = argument_bytes(initials)?;
            amipro_document::NoteInfo {
                time,
                initials: Some(decode_windows_1252(&initials)),
                colour: Some(canonical_long(colour)?),
                number: Some(u16::try_from(canonical_number(number)?).ok()?),
            }
        }
        _ => return None,
    };
    Some(PendingObject::Note {
        note,
        paragraphs: inner_paragraphs(body, report),
    })
}

/// A decimal number as Ami Pro writes a long (`ltoa`): digits only, no
/// leading zero, within 32 bits.
fn canonical_long(text: &str) -> Option<u32> {
    let canonical = !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical.then(|| text.parse::<u32>().ok()).flatten()
}

fn finish_run(
    runs: &mut Vec<CharacterRun>,
    start: Option<usize>,
    end: usize,
    format: &CharacterFormat,
) {
    if let Some(start) = start.filter(|start| *start < end) {
        runs.push(CharacterRun {
            start,
            end,
            format: format.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ParsedParagraph {
        parse_paragraph(input, &mut TextReport::default(), &mut main_context())
    }

    #[test]
    fn leading_escapes_include_the_code_written_with_its_own_closing_bracket() {
        assert_eq!(leading_escape("<*>>."), Some((0x1E, ".")));
        assert_eq!(leading_escape("<*7>x"), Some((0x17, "x")));
        assert_eq!(leading_escape("<*<>"), Some((0x1C, "")));
        assert_eq!(leading_escape("<[>a"), Some((b'[', "a")));
        assert_eq!(leading_escape("<+!>"), None);
        assert_eq!(leading_escape("x"), None);
    }

    fn main_context() -> ParseContext {
        ParseContext {
            main: true,
            anchored_frames: 3,
            used_anchors: BTreeSet::new(),
        }
    }

    #[test]
    fn page_and_column_breaks_carry_their_flags_and_layout_names() {
        let parsed = parse("Plain<:p<* >> centred<:p@> named<:p</@>1> col<:c>");
        assert_eq!(
            parsed.text,
            "Plain\u{FFFC} centred\u{FFFC} named\u{FFFC} col\u{FFFC}"
        );
        assert!(matches!(
            &parsed.objects[..],
            [
                PendingObject::PageBreak {
                    centre: false,
                    layout: None,
                    ..
                },
                PendingObject::PageBreak {
                    centre: true,
                    layout: None,
                    ..
                },
                PendingObject::PageBreak {
                    centre: false,
                    layout: Some(name),
                    ..
                },
                PendingObject::ColumnBreak,
            ] if name == "1"
        ));
    }

    #[test]
    fn floating_headers_and_footers_own_their_paragraphs() {
        let parsed = parse("Second.<:H<*->\nFloating header.\n\n>");
        assert_eq!(parsed.text, "Second.\u{FFFC}");
        let [
            PendingObject::Floating {
                kind: HeaderFooterKind::Header,
                pages: PageSides::All,
                paragraphs,
            },
        ] = &parsed.objects[..]
        else {
            panic!("expected a floating header: {:?}", parsed.objects);
        };
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].text, "Floating header.");

        let footer = parse("Two.<:h<*.>\nFOOT ALL\n\n>");
        assert!(matches!(
            &footer.objects[..],
            [PendingObject::Floating {
                kind: HeaderFooterKind::Footer,
                pages: PageSides::All,
                ..
            }]
        ));
        // 0x0A: footer (2) on right pages (8).
        let right = parse("<:h<**>\nRight only\n\n>");
        assert!(matches!(
            &right.objects[..],
            [PendingObject::Floating {
                pages: PageSides::Right,
                ..
            }]
        ));
        // An empty line where a paragraph starts is an empty paragraph, as
        // the builder would make for an empty flow anyway.
        let empty = parse("<:H<*%>\n\n>");
        assert!(matches!(
            &empty.objects[..],
            [PendingObject::Floating { paragraphs, pages: PageSides::Left, .. }]
                if paragraphs.len() == 1 && paragraphs[0].text.is_empty()
        ));
    }

    #[test]
    fn tab_rulers_set_the_paragraphs_own_tabs() {
        use amipro_document::{TabKind, TabLeader, TabStop};
        let parsed = parse("<:s><:R1,2,1,1890,32771,4050,><:#326,10260>A\tB\tC");
        assert_eq!(parsed.text, "A\tB\tC");
        assert_eq!(
            parsed.format.tabs,
            Some(vec![
                TabStop::new(TabKind::Left, 1890),
                TabStop {
                    leader: TabLeader::Dots,
                    ..TabStop::new(TabKind::Right, 4050)
                },
            ])
        );
        assert!(parsed.marks.iter().all(|mark| !mark.raw.starts_with("<:R")));
        // A ruler without tabs, or one that does not read, stays a mark.
        for control in ["<:R>", "<:R0,1,1,720,>", "<:R1,3,1,720,>"] {
            let parsed = parse(&format!("{control}text"));
            assert!(parsed.format.tabs.is_none(), "{control}");
            assert_eq!(parsed.marks[0].raw, control);
        }
        assert_eq!(parse("<:R1,0,>x").format.tabs, Some(Vec::new()));
    }

    #[test]
    fn body_paragraphs_split_as_ami_pros_reader_does_ep1() {
        // Decompile `1118:0e55`; VM fixture EP1, which Ami Pro read and
        // wrote back as nine paragraphs.
        let lines = [
            "First",
            "",
            "",
            "Second",
            "",
            "@Body Single@",
            "@Body Single@Third",
            "",
            "@Body Single@",
            "Fourth untagged, ",
            "wrapped",
            "",
            "\t",
            "",
            "Fifth",
            "",
            ">",
            "after",
        ];
        let (paragraphs, consumed) = split_body(lines.into_iter());
        assert_eq!(
            paragraphs,
            [
                "First",
                "",
                "Second",
                "@Body Single@",
                "@Body Single@Third",
                "@Body Single@",
                "Fourth untagged, wrapped",
                "\t",
                "Fifth",
            ]
        );
        assert_eq!(consumed, 17, "up to and including the `>` line");
        // `@@` starts text, and a tag with text after it is not bare.
        let (paragraphs, _) = split_body(["@@x@", "y", "", ">"].into_iter());
        assert_eq!(paragraphs, ["@@x@y"]);
        // Inside a multi-line control, empty and `>` lines are the control's.
        let (paragraphs, _) = split_body(["Note<:N1", "", ">", "", "next", "", ">"].into_iter());
        assert_eq!(paragraphs, ["Note<:N1\n\n>", "next"]);
    }

    #[test]
    fn anchors_fields_special_characters_and_caches() {
        let parsed = parse("<:#284,9360><:t2>Para<**>two<*7>.<:A1> <:P10,0,Page >");
        assert_eq!(
            parsed.text,
            "\u{FFFC}Para\u{2028}two\u{AD}.\u{FFFC} \u{FFFC}"
        );
        assert!(matches!(
            &parsed.objects[..],
            [
                PendingObject::Anchor { index: 2, .. },
                PendingObject::Anchor { index: 1, .. },
                PendingObject::PageNumber(field),
            ] if field.style == '1' && field.start_page == 0 && field.leading_text == "Page "
        ));
        assert!(parsed.marks.is_empty(), "the cache is dropped, not kept");
    }

    #[test]
    fn unmapped_controls_become_legacy_marks_at_their_offsets() {
        let mut report = TextReport::default();
        let parsed = parse_paragraph(
            "<:R1,12,1,720,>Indented <:Q+/>text<*0><:N note\nline two\n>",
            &mut report,
            &mut main_context(),
        );
        assert_eq!(parsed.text, "Indented text");
        assert_eq!(
            parsed
                .marks
                .iter()
                .map(|mark| (mark.byte, mark.raw.as_str()))
                .collect::<Vec<_>>(),
            [
                (0, "<:R1,12,1,720,>"),
                (9, "<:Q+/>"),
                (13, "<*0>"),
                (13, "<:N note\nline two\n>"),
            ]
        );
        assert_eq!(report.legacy_marks, 4);
        assert!(report.unmapped.contains(":N note"));
    }

    #[test]
    fn table_footnotes_and_cell_notes_are_objects_rnf6() {
        let mut report = TextReport::default();
        let mut context = ParseContext::outside_main();
        let parsed = parse_paragraph("b1 cell<:x1,2>", &mut report, &mut context);
        assert_eq!(parsed.text, "b1 cell\u{FFFC}");
        assert_eq!(
            parsed.objects,
            [PendingObject::TableFootnote { number: 1, row: 2 }]
        );
        // A footnote outside the main text stays a kept mark; a note may
        // stand anywhere.
        let parsed = parse_paragraph("x<:F\n@Footnote@y\n\n>", &mut report, &mut context);
        assert!(parsed.objects.is_empty() && parsed.marks.len() == 1);
        let parsed = parse_paragraph("x<:N5,AB,255,0,0\ny\n\n>", &mut report, &mut context);
        assert!(matches!(parsed.objects[..], [PendingObject::Note { .. }]));
    }

    #[test]
    fn inserted_bullets_become_objects_bl1() {
        // VM fixture BL1: the ten shapes and the seven pictures, `<**>`
        // (0x0A) among them, which is not a hard return here.
        let written = [
            "<*0>", "<*1>", "<*2>", "<*3>", "<*4>", "<*5>", "<*6>", "<*7>", "<*8>", "<*9>", "<*(>",
            "<**>", "<*+>", "<*,>", "<*->", "<*.>", "<*/>",
        ];
        let text: String = written.iter().map(|code| format!("<:B{code}>x")).collect();
        let mut report = TextReport::default();
        let parsed = parse_paragraph(&text, &mut report, &mut ParseContext::outside_main());
        let bullets: Vec<char> = parsed
            .objects
            .iter()
            .map(|object| match object {
                PendingObject::Bullet(code) => *code,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(bullets, amipro_document::BULLET_CODES);
        assert_eq!(parsed.text, "\u{FFFC}x".repeat(17));
        assert!(parsed.marks.is_empty());
        assert_eq!(report.legacy_marks, 0);
        // Forms Ami Pro does not write stay legacy marks.
        let odd = parse("<:B<*A>>a<:B>b<:B<*1>x>c");
        assert!(odd.objects.is_empty());
        assert_eq!(odd.text, "abc");
        assert_eq!(odd.marks.len(), 3);
    }

    #[test]
    fn dates_times_and_doc_info_entries_become_objects_fd1_fd2() {
        // VM fixtures FD1 and FD2 as Ami Pro wrote them: a tab written as it
        // is (Field2's code 9), the time's list position as an escape.
        let parsed =
            parse("S: <:Db> H: <:b<*!>> R: <:rd> C: <:kc> F: <:dc<*!><* >> 2: <:dc\t<* >>");
        assert_eq!(
            parsed.text,
            "S: \u{FFFC} H: \u{FFFC} R: \u{FFFC} C: \u{FFFC} F: \u{FFFC} 2: \u{FFFC}"
        );
        let objects: Vec<String> = parsed
            .objects
            .iter()
            .map(|object| format!("{object:?}"))
            .collect();
        assert_eq!(
            objects,
            [
                "Date(DateField { source: Today, format: 98 })",
                "Date(DateField { source: Time, format: 1 })",
                "Date(DateField { source: LastSaved, format: 100 })",
                "Date(DateField { source: Created, format: 99 })",
                "DocInfo(DocInfoField { code: 1, format: 99 })",
                "DocInfo(DocInfoField { code: 9, format: 99 })",
            ]
        );
        assert!(parsed.marks.is_empty());
        // Forms Ami Pro does not write stay legacy marks.
        let odd = parse("<:D>a<:Dbb>b<:d<*!>>c<:b<*X>d");
        assert!(odd.objects.is_empty());
        assert_eq!(odd.marks.len(), 3);
    }

    #[test]
    fn indention_and_spacing_controls_set_the_paragraph_format_o3_o6() {
        let parsed = parse("<:I360,720,1080,1440>Indented");
        assert_eq!(
            parsed.format.indents,
            Some(Indents {
                all: 360,
                first: 720,
                rest: 1_080,
                right: 1_440
            })
        );
        assert_eq!(parsed.text, "Indented");
        assert!(parsed.marks.is_empty());
        let spacing = |input: &str| parse(input).format.line_spacing;
        assert_eq!(spacing("<:S+-1>Single"), Some(LineSpacing::Single));
        assert_eq!(spacing("<:S+-2>Half"), Some(LineSpacing::OneAndAHalf(0)));
        assert_eq!(spacing("<:S+-3>Double"), Some(LineSpacing::Double(0)));
        assert_eq!(spacing("<:S+432>Custom"), Some(LineSpacing::Custom(432)));
        assert_eq!(spacing("Plain"), None);
        // O6: anywhere in the paragraph, and a later one replaces an earlier.
        assert_eq!(spacing("Plain <:S+-3>double"), Some(LineSpacing::Double(0)));
        assert_eq!(
            spacing("<:S+-3>First <:S+-2>later"),
            Some(LineSpacing::OneAndAHalf(0))
        );
        assert_eq!(
            parse("<:I1440,0,0,0>First <:I0,720,0,0>later")
                .format
                .indents
                .map(|indents| (indents.all, indents.first)),
            Some((0, 720))
        );
        // O5: `<:S->` changes nothing and is kept; malformed ones are marks.
        let ended = parse("<:S+-3><:S->Double<:I1,2><:S+-9>");
        assert_eq!(ended.format.line_spacing, Some(LineSpacing::Double(0)));
        assert_eq!(ended.format.indents, None);
        assert_eq!(ended.marks.len(), 3);
    }

    #[test]
    fn alignment_marks_set_the_paragraph_alignment_o2_o7() {
        let alignment = |input: &str| parse(input).format.alignment;
        assert_eq!(alignment("<+@>Left"), Some(Alignment::Left));
        assert_eq!(alignment("<+A>Right"), Some(Alignment::Right));
        assert_eq!(alignment("<+B>Centre"), Some(Alignment::Centre));
        assert_eq!(alignment("<+C>Justified"), Some(Alignment::Justify));
        assert_eq!(alignment("Plain"), None);
        // Anywhere in the paragraph; two combine (O6, O7, decompile): left
        // and right make centre, and justify with another is the style's.
        assert_eq!(alignment("Plain <+B>centre"), Some(Alignment::Centre));
        assert_eq!(alignment("<+B>Centre <+@>left"), Some(Alignment::Centre));
        assert_eq!(alignment("<+A>Right <+B>centre"), Some(Alignment::Centre));
        assert_eq!(alignment("<+@>Left <+A>right"), Some(Alignment::Centre));
        assert_eq!(alignment("<+A>Right <+A>right"), Some(Alignment::Right));
        assert_eq!(alignment("<+C>Justified <+@>left"), None);
        assert_eq!(alignment("<+C>Justified <+@>left <+A>right"), None);
        assert_eq!(alignment("<+@>Left <+C>justified"), None);
        let parsed = parse("<+B>Centre");
        assert_eq!(parsed.text, "Centre");
        assert!(parsed.marks.is_empty());
    }

    /// The runs of `input` as (text, format) pairs.
    fn runs(input: &str) -> Vec<(String, CharacterFormat)> {
        let parsed = parse(input);
        parsed
            .runs
            .iter()
            .map(|run| {
                (
                    parsed.text[run.start..run.end].to_owned(),
                    run.format.clone(),
                )
            })
            .collect()
    }

    #[test]
    fn attribute_marks_become_runs_o1_o6() {
        let on = Some(true);
        let parsed = runs(
            "<+#>under<-#> <+$>word<-$> <+)>double<-)> <+%>struck<-%> \
             <+&>sup<-&> <+'>sub<-'> <++>upper<-+> <+,>lower<-,> <+->initial<--> \
             <+(>small<-(> plain",
        );
        let expected = [
            (
                "under",
                CharacterFormat {
                    underline: on,
                    ..Default::default()
                },
            ),
            (
                "word",
                CharacterFormat {
                    word_underline: on,
                    ..Default::default()
                },
            ),
            (
                "double",
                CharacterFormat {
                    double_underline: on,
                    ..Default::default()
                },
            ),
            (
                "struck",
                CharacterFormat {
                    strikethrough: on,
                    ..Default::default()
                },
            ),
        ];
        for (index, (text, format)) in expected.iter().enumerate() {
            assert_eq!(parsed[index].0, *text);
            assert_eq!(parsed[index].1, *format);
        }
        // Strikethrough, superscript, and subscript end with an explicit
        // off; the others return to the style.
        let off = |format: CharacterFormat| CharacterFormat {
            strikethrough: Some(false),
            ..format
        };
        assert_eq!(parsed[4].0, " ");
        assert_eq!(parsed[4].1, off(CharacterFormat::default()));
        assert_eq!(parsed[5].0, "sup");
        assert_eq!(
            parsed[5].1,
            off(CharacterFormat {
                superscript: on,
                ..Default::default()
            })
        );
        assert_eq!(parsed[7].0, "sub");
        assert_eq!(
            parsed[7].1,
            off(CharacterFormat {
                superscript: Some(false),
                subscript: on,
                ..Default::default()
            })
        );
        let tail = CharacterFormat {
            strikethrough: Some(false),
            superscript: Some(false),
            subscript: Some(false),
            ..Default::default()
        };
        let caps: Vec<_> = parsed[8..]
            .iter()
            .map(|(text, format)| {
                assert_eq!(
                    CharacterFormat {
                        caps: None,
                        ..format.clone()
                    },
                    tail
                );
                (text.as_str(), format.caps)
            })
            .collect();
        assert_eq!(
            caps,
            [
                (" ", None),
                ("upper", Some(Caps::Upper)),
                (" ", None),
                ("lower", Some(Caps::Lower)),
                (" ", None),
                ("initial", Some(Caps::Initial)),
                (" ", None),
                ("small", Some(Caps::Small)),
                (" plain", None),
            ]
        );
        assert!(parse("<+#>under<-#>").marks.is_empty());
    }

    #[test]
    fn attribute_marks_nest_exclude_and_keep_unknown_marks() {
        // `<-X>` ends the attribute however many `<+X>` came before (O6).
        let parsed = runs("<+#>a<+#>b<-#>c<-#>d");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "ab");
        // Each caps attribute clears the others; any caps off returns to the
        // style's caps (decompile).
        let caps: Vec<_> = runs("<++>A<+(>B<-+>C<+,>D<+->E<-(>F")
            .into_iter()
            .map(|(text, format)| (text, format.caps))
            .collect();
        assert_eq!(
            caps,
            [
                ("A".to_owned(), Some(Caps::Upper)),
                ("B".to_owned(), Some(Caps::Small)),
                ("D".to_owned(), Some(Caps::Lower)),
                ("E".to_owned(), Some(Caps::Initial)),
            ]
        );
        // Bold and italic still nest with the others.
        let parsed = runs("<+!>b<+\">bi<-!>i<-\">");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[1].1.bold, Some(true));
        assert_eq!(parsed[1].1.italic, Some(true));
        // Protected text and no hyphenation are runs (PT1); marks Ami Pro
        // ignores stay legacy marks.
        let parsed = runs("<+*>kept<-*> <+.>whole<-.> <+/>x<-0>");
        let on = |text: &str| {
            let format = &parsed.iter().find(|(run, _)| run == text).unwrap().1;
            (format.protected, format.no_hyphenation)
        };
        assert_eq!(on("kept"), (Some(true), None));
        assert_eq!(on("whole"), (Some(false), Some(true)));
        let parsed = parse("<+*>kept<-*> <+.>whole<-.> <+/>x<-0>");
        assert_eq!(parsed.text, "kept whole x");
        assert_eq!(
            parsed
                .marks
                .iter()
                .map(|mark| mark.raw.as_str())
                .collect::<Vec<_>>(),
            ["<+/>", "<-0>"]
        );
        // Overstrike (O1): its character, an escape included; off ends it
        // whatever its character.
        let parsed = runs("<:O+/>over<:O-/> <:O+<;>>gt<:O-X>. <:O+<<>lt");
        assert_eq!(
            parsed
                .iter()
                .map(|(text, format)| (text.as_str(), format.overstrike))
                .collect::<Vec<_>>(),
            [("over", Some('/')), ("gt", Some('>')), ("lt", Some('<'))]
        );
    }

    #[test]
    fn font_changes_set_face_size_and_colour_o4_o5() {
        let font = |format: &CharacterFormat| {
            (
                format.font_family.clone(),
                format.font_size_points,
                format.colour,
            )
        };
        let arial = Some("Arial".to_owned());
        // Each change replaces the one before; `<:f>` returns to the style's
        // font, not to an earlier change (O5).
        let parsed = runs("<:f360,BArial,0,0,0>big <:f200,,>small<:f> after");
        assert_eq!(parsed.len(), 2);
        assert_eq!(
            (parsed[0].0.as_str(), font(&parsed[0].1)),
            ("big ", (arial.clone(), Some(18.0), Some(0)))
        );
        assert_eq!(
            (parsed[1].0.as_str(), font(&parsed[1].1)),
            ("small", (None, Some(10.0), None))
        );
        let only = |input: &str| runs(input).first().map(|(_, format)| font(format));
        assert_eq!(
            only("<:f240,2Times New Roman,255,0,0>red"),
            Some((
                Some("Times New Roman".to_owned()),
                Some(12.0),
                Some(0x0000_00FF)
            ))
        );
        assert_eq!(only("<:f,BArial,>face"), Some((arial.clone(), None, None)));
        assert_eq!(only("<:f,,>nothing"), None);
        assert_eq!(only("<:f>nothing"), None);
        assert_eq!(only("<:f240>nothing"), None, "no comma: nothing reads");
        assert_eq!(
            only("<:f24x,BArial,>nothing"),
            None,
            "a bad size stops reading"
        );
        assert_eq!(
            only("<:f240,BArial,255,0>no colour"),
            Some((arial.clone(), Some(12.0), None))
        );
        assert_eq!(
            only("<:f240,BArial,25x,0,0>no colour"),
            Some((arial.clone(), Some(12.0), None))
        );
        assert_eq!(
            only("<:f,BArial,256,512,767>wrapped"),
            Some((arial.clone(), None, Some(0x00FF_0000)))
        );
        assert_eq!(
            only("<:f,BA very long face name of forty characters,0,0,255>cut"),
            Some((
                Some("A very long face name of forty ".trim().to_owned()),
                None,
                None
            )),
            "a face longer than 31 characters is cut and ends the reading"
        );
        // A paragraph's end format is the one in force after its last
        // character; a paragraph without text keeps its changes there.
        let parsed = parse("<:f200,,><+!>");
        assert!(parsed.text.is_empty() && parsed.runs.is_empty() && parsed.marks.is_empty());
        assert_eq!(
            parsed.end_format,
            CharacterFormat {
                font_size_points: Some(10.0),
                bold: Some(true),
                ..CharacterFormat::default()
            }
        );
        assert!(parse("<:f200,,>text<:f>").end_format.is_empty());
        assert_eq!(
            parse("<:f200,,>text").end_format.font_size_points,
            Some(10.0)
        );
        assert_eq!(parse("text<+\">").end_format.italic, Some(true));
        assert!(parse("<:f200,,><:f>").end_format.is_empty());
        // Font changes keep the attributes around them, and leave no marks.
        let parsed = parse("<+!>bold <:f300,,>big<:f> bold<-!>");
        assert!(parsed.marks.is_empty());
        assert_eq!(
            parsed
                .runs
                .iter()
                .map(|run| (
                    run.start,
                    run.end,
                    run.format.bold,
                    run.format.font_size_points
                ))
                .collect::<Vec<_>>(),
            [
                (0, 5, Some(true), None),
                (5, 8, Some(true), Some(15.0)),
                (8, 13, Some(true), None)
            ]
        );
    }

    #[test]
    fn anchors_resolve_once_and_only_in_the_main_text() {
        let mut report = TextReport::default();
        let mut context = main_context();
        let parsed = parse_paragraph("<:A0><:t0><:A7>x", &mut report, &mut context);
        assert_eq!(
            parsed.objects.len(),
            1,
            "a second mark for frame 0 and index 7 stay marks"
        );
        assert_eq!(parsed.marks.len(), 2);
        assert_eq!((report.anchors_resolved, report.anchors_unresolved), (1, 2));

        let mut report = TextReport::default();
        let frame_text = parse_paragraph(
            "In a frame<:A1><:p<* >><:P10,0,Page >",
            &mut report,
            &mut ParseContext::outside_main(),
        );
        assert_eq!(frame_text.text, "In a frame\u{FFFC}");
        assert!(matches!(
            frame_text.objects[..],
            [PendingObject::PageNumber(_)]
        ));
        assert_eq!(frame_text.marks.len(), 2);
    }

    #[test]
    fn page_number_fields_read_a_style_character_a_start_page_and_a_start_number() {
        // The corpora's forms (`PAGE_LAYOUT_NOTES.md`, "Page numbering").
        let field = |control: &str| {
            let parsed = parse_paragraph(
                control,
                &mut TextReport::default(),
                &mut ParseContext::outside_main(),
            );
            match &parsed.objects[..] {
                [PendingObject::PageNumber(field)] => Some(field.clone()),
                _ => None,
            }
        };
        let expect = |style, start_page, start_number, text: &str| PageNumberField {
            style,
            start_page,
            start_number,
            leading_text: text.into(),
        };
        assert_eq!(field("<:P10,0,Page >"), Some(expect('1', 0, 0, "Page ")));
        assert_eq!(field("<:P12,1,>"), Some(expect('1', 2, 1, "")));
        assert_eq!(field("<:P111,15,- >"), Some(expect('1', 11, 15, "- ")));
        assert_eq!(field("<:Pi1,1,>"), Some(expect('i', 1, 1, "")));
        assert_eq!(field("<:PI0,0,Page >"), Some(expect('I', 0, 0, "Page ")));
        assert_eq!(field("<:Pa0,0,x,y>"), Some(expect('a', 0, 0, "x,y")));
        // Numbers Ami Pro would write otherwise stay marks, so the bytes
        // come back unchanged.
        assert_eq!(field("<:P100,0,>"), None);
        assert_eq!(field("<:P1,0,>"), None);
        assert_eq!(field("<:P10,0>"), None);
        assert_eq!(field("<:P>"), None);
    }

    #[test]
    fn legacy_text_cannot_invent_objects() {
        let parsed = parse("odd \u{FFFC} char");
        assert!(parsed.objects.is_empty());
        assert!(!parsed.text.contains(OBJECT_REPLACEMENT_CHARACTER));
    }
}
