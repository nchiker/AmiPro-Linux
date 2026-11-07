//! Ami Pro's power fields as its files keep them
//! (`docs/research/FIELDS_NOTES.md`, "Power fields"): a start mark
//! `<:XK,V;CODE>`, then for the kinds with a result that result as ordinary
//! text and an end mark `<:X~K,V;CODE>` (kept as legacy marks, so that
//! they move, are deleted, and are written with the text around them); Seq
//! is an object, as its mark draws its number.

use crate::{LegacyMark, Paragraph, decode_windows_1252, windows_1252};

/// The kinds of power field, as Ami Pro numbers them (`1628:005c`).
pub mod kind {
    pub const GENERAL: u8 = 3;
    pub const SEQ: u8 = 4;
    pub const SET: u8 = 5;
    pub const BUTTON: u8 = 6;
    pub const PRINT_ESCAPE: u8 = 7;
    pub const INDEX: u8 = 8;
    pub const USER: u8 = 9;
    pub const TOC: u8 = 11;
    pub const MERGE_FIELD: u8 = 12;
}

/// The value word's flags (`1648:055c`, `1648:05bd`; VM fixture FD3).
pub mod flag {
    /// Update and Update All leave the field as it is.
    pub const LOCK: u16 = 0x2000;
    /// The field is updated when the document is opened.
    pub const AUTO_RUN: u16 = 0x1000;
    /// Updated again after the pages are counted (it read NumPages).
    pub const AFTER_PAGINATION: u16 = 0x4000;
    /// Made in the main text (INFERRED from the corpus).
    pub const MAIN_TEXT: u16 = 0x8000;
}

/// Whether a field of `kind` keeps a result between two marks.
pub fn has_result(kind: u8) -> bool {
    matches!(
        kind,
        kind::GENERAL | kind::BUTTON | kind::USER | kind::MERGE_FIELD
    )
}

/// One of a power field's marks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerMark {
    /// The end mark (`<:X~…>`) rather than the start.
    pub end: bool,
    pub kind: u8,
    /// The record's value word (Ami Pro writes it as a signed number).
    pub value: u16,
    /// The field's code, as typed.
    pub code: String,
}

impl PowerMark {
    /// A kept mark read as a power field's, or `None` for another control
    /// (`1628:0000`, `1630:0993`): the kind, a comma, the value, a
    /// semicolon, the code.
    pub fn parse(raw: &str) -> Option<Self> {
        // While codes show, an end mark carries the id of what its field
        // set aside after a NUL (`SetAside`).
        let raw = raw.split('\0').next().unwrap_or(raw);
        let body = raw.strip_prefix("<:X")?.strip_suffix('>')?;
        let (end, body) = match body.strip_prefix('~') {
            Some(rest) => (true, rest),
            None => (false, body),
        };
        let (kind, rest) = body.split_once(',')?;
        let (value, code) = rest.split_once(';')?;
        Some(Self {
            end,
            kind: kind.trim().parse().ok()?,
            value: value.trim().parse::<i32>().ok()? as u16,
            code: unescape(code)?,
        })
    }

    /// The mark as Ami Pro writes it (`1630:119d`, `1628:0160`).
    pub fn raw(&self) -> String {
        format!(
            "<:X{}{},{};{}>",
            if self.end { "~" } else { "" },
            self.kind,
            self.value as i16,
            escape(&self.code)
        )
    }

    pub fn locked(&self) -> bool {
        self.kind != kind::SEQ && self.value & flag::LOCK != 0
    }

    pub fn auto_run(&self) -> bool {
        self.kind != kind::SEQ && self.value & flag::AUTO_RUN != 0
    }
}

/// What a field set aside while View › Show Power Fields shows its code in
/// the text, as Ami Pro keeps each result in the field's record
/// (`1638:0d0b`; `docs/research/FIELDS_NOTES.md`, "Display").
#[derive(Clone, Debug, PartialEq)]
pub struct SetAside {
    pub shown: Shown,
    /// The result (a Seq's object), with its formatting, objects, and inner
    /// marks; bytes from its start.
    pub text: String,
    pub runs: Vec<crate::CharacterRun>,
    pub objects: Vec<crate::InlineObject>,
    pub marks: Vec<LegacyMark>,
    /// The format the code shows in, before its red.
    pub format: crate::CharacterFormat,
    /// Whether one run went on across the field's start, and its end.
    pub joined: [bool; 2],
}

/// What a field shown as its code was.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Shown {
    /// A field with its result between its marks.
    Result,
    /// A field without a result: an end mark was added to show its code.
    NoResult,
    /// A Seq field, its object shown as marks around its code.
    Seq,
}

/// The id a mark carries while codes show, if any.
pub fn set_aside_id(raw: &str) -> Option<u32> {
    raw.split_once('\0')?.1.parse().ok()
}

/// A Seq field: its code and the value word whose low 14 bits are its
/// number (`1640`, `1630:1623`; VM fixtures FD3, FD6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeqField {
    pub code: String,
    pub value: u16,
}

impl SeqField {
    /// The number Ami Pro shows: the value's low 14 bits. A Seq Ami Pro
    /// locked shows its lock bit in it (FD3's 8197); the app never sets
    /// the flags of a Seq.
    pub fn number(&self) -> u32 {
        u32::from(self.value & 0x3FFF)
    }

    /// The number in the code's `%N` style.
    pub fn text(&self) -> String {
        number_in_style(self.number(), number_style(&self.code))
    }
}

/// The digit of a code's `%N` format, 1 when it has none.
pub fn number_style(code: &str) -> u8 {
    code.find('%')
        .and_then(|at| {
            let mut chars = code[at + 1..].chars();
            (chars.next()?.eq_ignore_ascii_case(&'n'))
                .then(|| chars.next()?.to_digit(10))
                .flatten()
        })
        .map_or(1, |digit| digit as u8)
}

/// `number` in `%N` style `style` (AMIMACRO `1080:1118`): 1 decimal, 2 and
/// 3 Roman numerals in capitals and small letters, 4 and 5 letters.
pub fn number_in_style(number: u32, style: u8) -> String {
    match style {
        2 => roman_numeral(number).to_uppercase(),
        3 => roman_numeral(number),
        4 => letter_number(number).to_uppercase(),
        5 => letter_number(number),
        _ => number.to_string(),
    }
}

/// A power field in a flow: its start mark and, for the kinds with a
/// result, the end mark it pairs with (the last start with the same code,
/// as Ami Pro's reader pairs them, `1630:0aae`). Places are a paragraph
/// index, a byte, and the mark's index in that paragraph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldSpan {
    pub mark: PowerMark,
    pub start: MarkPlace,
    pub end: Option<MarkPlace>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MarkPlace {
    pub paragraph: usize,
    pub byte: usize,
    pub mark: usize,
}

/// Every power field kept as marks in `paragraphs` (a flow), in order;
/// a field of a kind with a result but no end mark is left out, as Ami Pro
/// drops it (`1648:143e`).
pub fn power_fields<'a>(paragraphs: impl IntoIterator<Item = &'a Paragraph>) -> Vec<FieldSpan> {
    let mut fields: Vec<FieldSpan> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    for (index, paragraph) in paragraphs.into_iter().enumerate() {
        for (number, LegacyMark { byte, raw }) in paragraph.marks.iter().enumerate() {
            let Some(mark) = PowerMark::parse(raw) else {
                continue;
            };
            let place = MarkPlace {
                paragraph: index,
                byte: *byte,
                mark: number,
            };
            if mark.end {
                if let Some(at) = open
                    .iter()
                    .rposition(|field| fields[*field].mark.code == mark.code)
                {
                    let field = open.remove(at);
                    fields[field].end = Some(place);
                } else if set_aside_id(raw).is_some() {
                    // While codes show, a Seq or a field without a result
                    // has an end mark after its code too.
                    if let Some(field) = fields
                        .iter_mut()
                        .rev()
                        .find(|field| field.end.is_none() && field.mark.code == mark.code)
                    {
                        field.end = Some(place);
                    }
                }
            } else {
                if has_result(mark.kind) {
                    open.push(fields.len());
                }
                fields.push(FieldSpan {
                    mark,
                    start: place,
                    end: None,
                });
            }
        }
    }
    fields.retain(|field| !has_result(field.mark.kind) || field.end.is_some());
    fields
}

/// A code as written in a mark, escapes decoded: `<<` is `<`, `<;>` is
/// `>`, `<[>` is `[`, `<*X>`, `</X>`, and `<\\X>` their bytes.
pub(crate) fn unescape(code: &str) -> Option<String> {
    let mut text = String::with_capacity(code.len());
    let mut rest = code;
    while let Some(at) = rest.find('<') {
        text.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(after) = rest.strip_prefix("<<") {
            text.push('<');
            rest = after;
            continue;
        }
        let close = rest.find('>')?;
        let inner: Vec<char> = rest[1..close].chars().collect();
        let byte = match inner.as_slice() {
            [';'] => b'>',
            ['['] => b'[',
            ['*', code] if ('\u{20}'..='\u{3F}').contains(code) => *code as u8 - 0x20,
            ['/', code] if ('\u{40}'..='\u{7F}').contains(code) => *code as u8 + 0x40,
            ['\\', code] if ('\u{40}'..='\u{7F}').contains(code) => *code as u8 + 0x80,
            _ => return None,
        };
        text.push_str(&decode_windows_1252(&[byte]));
        rest = &rest[close + 1..];
    }
    text.push_str(rest);
    Some(text)
}

/// A code escaped as Ami Pro's writer escapes a mark's text.
pub(crate) fn escape(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    for character in code.chars() {
        match character {
            '<' => out.push_str("<<"),
            '>' => out.push_str("<;>"),
            _ => match windows_1252(character) {
                byte @ 0x00..=0x1F if byte != b'\t' => {
                    out.push_str(&format!("<*{}>", char::from(byte + 0x20)));
                }
                byte @ 0x80..=0xBF => out.push_str(&format!("</{}>", char::from(byte - 0x40))),
                byte @ 0xC0..=0xFF => {
                    out.push_str(&format!("<\\{}>", char::from(byte.wrapping_add(0x80))));
                }
                _ => out.push(character),
            },
        }
    }
    out
}

/// Lower-case roman numerals.
pub fn roman_numeral(mut number: u32) -> String {
    const NUMERALS: [(u32, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut text = String::new();
    for (value, numeral) in NUMERALS {
        while number >= value {
            text.push_str(numeral);
            number -= value;
        }
    }
    text
}

/// Lower-case letters as Ami Pro counts them: a to z, then aa, ab, … zz,
/// then three letters (`1308:0caa`).
pub fn letter_number(number: u32) -> String {
    let letter = |index: u32| char::from(b'a' + (index % 26) as u8);
    match number {
        0 => String::new(),
        1..=26 => letter(number - 1).to_string(),
        27..=702 => [letter((number - 27) / 26), letter((number - 27) % 26)]
            .into_iter()
            .collect(),
        _ => {
            let rest = number - 703;
            [
                letter(rest / 676),
                letter((rest % 676) / 26),
                letter(rest % 26),
            ]
            .into_iter()
            .collect()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_read_and_write_as_ami_pro_writes_them_fd3() {
        for raw in [
            "<:X3,-32768;NumPages>",
            "<:X~3,-32768;CreateDate %Db>",
            "<:X3,-32768;IF total <;> 2 \"big\" Else \"small\" EndIf>",
            "<:X3,-28672;undefinedvar>",
            "<:X3,-20480;CreateDate>",
            "<:X8,-32768;Index \"memory\" # \"extended\">",
            "<:X3,16384;NumPages>",
            "<:X12,0;MergeField NAME>",
            "<:X3,-32768;\"caf<\\i>\">",
        ] {
            let mark = PowerMark::parse(raw).unwrap();
            assert_eq!(mark.raw(), raw);
        }
        let mark =
            PowerMark::parse("<:X3,-32768;IF total <;> 2 \"big\" Else \"small\" EndIf>").unwrap();
        assert_eq!(mark.code, "IF total > 2 \"big\" Else \"small\" EndIf");
        assert_eq!((mark.kind, mark.value), (3, 0x8000));
        assert!(!mark.locked() && !mark.auto_run());
        let both = PowerMark::parse("<:X3,-20480;CreateDate>").unwrap();
        assert!(both.locked() && both.auto_run());
        assert_eq!(
            PowerMark::parse("<:X3,-32768;\"caf<\\i>\">").unwrap().code,
            "\"caf\u{e9}\""
        );
        // Seq's cached result and other controls are not marks of fields.
        assert_eq!(PowerMark::parse("<:X~4;III>"), None);
        assert_eq!(PowerMark::parse("<:Zname>"), None);
        assert_eq!(PowerMark::parse("<:X3,1;bad<?>>"), None);
    }

    #[test]
    fn seq_numbers_show_in_their_style_fd6() {
        let seq = |value: u16, code: &str| {
            SeqField {
                code: code.into(),
                value,
            }
            .text()
        };
        assert_eq!(seq(0x8001, "Seq Figure"), "1");
        assert_eq!(seq(0x800D, "Seq A+3 %N2"), "XIII");
        assert_eq!(seq(0x800E, "Seq A %N4"), "N");
        assert_eq!(seq(0x800F, "Seq A %N5"), "o");
        assert_eq!(seq(0x8010, "Seq A %N3"), "xvi");
        // FD3: a Seq Ami Pro locked shows its lock bit (8197).
        assert_eq!(seq(0xA005, "Seq Figure+2 %N2"), "MMMMMMMMCXCVII");
    }

    #[test]
    fn fields_pair_their_marks_and_drop_unpaired_ones() {
        let marks = |marks: &[(usize, &str)]| -> Vec<LegacyMark> {
            marks
                .iter()
                .map(|(byte, raw)| LegacyMark {
                    byte: *byte,
                    raw: (*raw).into(),
                })
                .collect()
        };
        // A result over two paragraphs (Include), a field with no result,
        // a nested field, and a start whose end was deleted.
        let mut document = crate::Document::from_plain_text("1: one\ntwo 3");
        document.paragraph_mut(0).marks = marks(&[
            (3, "<:X3,-32768;Include \"a.sam\">"),
            (5, "<:X5,-32768;Set x 1>"),
        ]);
        document.paragraph_mut(1).marks = marks(&[
            (3, "<:X~3,-32768;Include \"a.sam\">"),
            (4, "<:X3,-32768;x>"),
            (5, "<:X~3,-32768;x>"),
            (5, "<:X3,-32768;lost>"),
        ]);
        let fields = power_fields(document.paragraphs().iter().map(AsRef::as_ref));
        let shown: Vec<(&str, MarkPlace, Option<MarkPlace>)> = fields
            .iter()
            .map(|field| (field.mark.code.as_str(), field.start, field.end))
            .collect();
        let at = |paragraph, byte, mark| MarkPlace {
            paragraph,
            byte,
            mark,
        };
        assert_eq!(
            shown,
            [
                ("Include \"a.sam\"", at(0, 3, 0), Some(at(1, 3, 0))),
                ("Set x 1", at(0, 5, 1), None),
                ("x", at(1, 4, 1), Some(at(1, 5, 2))),
            ]
        );
    }
}
