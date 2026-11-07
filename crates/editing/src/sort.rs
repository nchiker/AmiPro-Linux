//! Tools › Sort… (`UI_SPEC.md` §41, decision TN3; `docs/research/
//! TABLE_DATA_NOTES.md` sections 10.7 and 11.7): paragraphs or table rows
//! in records sorted by up to three keys, Ami Pro's way.
//!
//! Ami Pro sorts with a **heap sort** (decompile `1360:23de` builds,
//! `17f6` takes the largest out, `259c` sifts), and equal records come out
//! in an order the heap decides, not the text's. This module repeats the
//! heap sort's comparisons and its order of moves, so equal records end up
//! where Ami Pro's end up (the VM's TDX5 and TDY1 are the vectors). The
//! heap for an Ascending sort puts the smallest key at the root, so the
//! array comes out descending and Ami Pro writes it back from its last
//! record to its first; a Descending sort is the same heap the other way
//! round.
//!
//! A record is one paragraph, or `record` paragraphs (rows). Its **fields**
//! are the pieces between delimiters (the tab by default; a table's are its
//! cells), counted through all the record's paragraphs; a key takes the
//! 1st, the 2nd, or (All) the whole text of the field it names. A word is
//! a run between spaces (and tabs, when the delimiter is not the tab).
//!
//! Alphanumeric keys compare by the `SORTTAB` weights and then by bytes
//! (`contents::index_order`, the VM's TDX1–TDX6). Numeric keys are reduced
//! to their digits (`1,000` is 1000, `1e2` is 12, a `-` before the digits
//! or `(`, or a `-` at the end, makes the number negative); text without a
//! digit sorts after the numbers.

use std::cmp::Ordering;
use std::sync::Arc;

use amipro_document::{Document, FlowId, InlineObject, OBJECT_REPLACEMENT_CHARACTER, Paragraph};

use crate::contents::index_order;
use crate::{Applied, ChangeReport, EditError, TextPosition};

/// Message 3112: a page or column break lies in the text; continue to sort
/// across it (set [`SortSettings::confirmed`]).
pub const SORT_ACROSS_BREAK: &str = "the text has a page or column break";
/// Message 3184: connected cells lie in the rows; continuing disconnects them.
pub const SORT_CONNECTED: &str = "the rows have connected cells";
/// Message 3193: a table with footnotes is not sorted.
pub const SORT_FOOTNOTES: &str = "a table with footnotes cannot be sorted";
/// Fewer than two records.
pub const SORT_NOTHING: &str = "nothing to sort";

/// Whether a refusal from a sort is one of Ami Pro's questions (the
/// sort goes on when answered with Continue).
pub fn is_question(error: &str) -> bool {
    error == SORT_ACROSS_BREAK || error == SORT_CONNECTED
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SortKind {
    #[default]
    Alphanumeric,
    Numeric,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SortWord {
    #[default]
    First,
    Second,
    All,
}

/// One level of the sort.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SortKey {
    /// The field, 1 to 99.
    pub field: u8,
    pub kind: SortKind,
    pub word: SortWord,
}

impl SortKey {
    pub const MAX_FIELD: u8 = 99;
}

/// The dialog's settings (Ami Pro keeps its last ones).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SortSettings {
    /// Levels 1 to 3 (a level without a field is left out).
    pub keys: Vec<SortKey>,
    pub descending: bool,
    /// The field delimiter; `None` is Tab delimited. Text only.
    pub delimiter: Option<char>,
    /// Paragraphs (rows) in a record, 1 to 99.
    pub record: usize,
    /// The answer Continue to a question ([`is_question`]).
    pub confirmed: bool,
}

impl Default for SortSettings {
    fn default() -> Self {
        Self {
            keys: vec![SortKey {
                field: 1,
                kind: SortKind::Alphanumeric,
                word: SortWord::All,
            }],
            descending: false,
            delimiter: None,
            record: 1,
            confirmed: false,
        }
    }
}

/// A key worked out for a record.
#[derive(Clone, Debug)]
pub(crate) enum Key {
    Text(String),
    Number {
        negative: bool,
        integer: Vec<u8>,
        fraction: Vec<u8>,
    },
    /// Numeric text with no digit.
    NoNumber(String),
}

/// The digits of `text` as Ami Pro reads a number (`1360:1662`).
fn number_key(text: &str) -> Key {
    let bytes: Vec<u8> = text.chars().map(amipro_document::windows_1252).collect();
    let (mut negative, mut seen_point) = (false, false);
    let (mut integer, mut fraction) = (Vec::new(), Vec::new());
    for (index, &byte) in bytes.iter().enumerate() {
        match byte {
            b'-' | b'(' => {
                if (integer.is_empty() && !seen_point) || index + 1 == bytes.len() {
                    negative = true;
                }
            }
            b'.' => seen_point = true,
            b'0'..=b'9' => {
                if seen_point {
                    fraction.push(byte);
                } else {
                    integer.push(byte);
                }
            }
            _ => {}
        }
    }
    if integer.is_empty() && fraction.is_empty() {
        Key::NoNumber(text.to_owned())
    } else {
        Key::Number {
            negative,
            integer,
            fraction,
        }
    }
}

/// Integer digits compared as numbers: no leading zeros, longer is larger.
fn compare_integers(a: &[u8], b: &[u8]) -> Ordering {
    let trim = |digits: &[u8]| {
        let start = digits
            .iter()
            .position(|d| *d != b'0')
            .unwrap_or(digits.len());
        digits[start..].to_vec()
    };
    let (a, b) = (trim(a), trim(b));
    a.len().cmp(&b.len()).then_with(|| a.cmp(&b))
}

/// Fractions compared as digit strings padded with zeros.
fn compare_fractions(a: &[u8], b: &[u8]) -> Ordering {
    let width = a.len().max(b.len());
    (0..width)
        .map(|i| (a.get(i).unwrap_or(&b'0'), b.get(i).unwrap_or(&b'0')))
        .map(|(x, y)| x.cmp(y))
        .find(|order| *order != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

fn compare_key(a: &Key, b: &Key) -> Ordering {
    match (a, b) {
        (Key::Text(a), Key::Text(b)) | (Key::NoNumber(a), Key::NoNumber(b)) => index_order(a, b),
        (Key::Number { .. }, Key::NoNumber(_)) => Ordering::Less,
        (Key::NoNumber(_), Key::Number { .. }) => Ordering::Greater,
        (
            Key::Number {
                negative: na,
                integer: ia,
                fraction: fa,
            },
            Key::Number {
                negative: nb,
                integer: ib,
                fraction: fb,
            },
        ) => {
            if na != nb {
                // A negative number comes before a positive one.
                return if *na {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            let order = compare_integers(ia, ib).then_with(|| compare_fractions(fa, fb));
            // Two negatives: the larger magnitude is the smaller number.
            if *na { order.reverse() } else { order }
        }
        _ => Ordering::Equal,
    }
}

/// Splits a record's paragraphs into fields, counted through all of them.
fn fields_of(paragraphs: &[String], delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    for text in paragraphs {
        for piece in text.split(delimiter) {
            fields.push(piece.to_owned());
        }
    }
    fields
}

/// The text a level's key takes from a field.
fn key_text(field: Option<&String>, word: SortWord, delimiter: char) -> String {
    let Some(field) = field else {
        return String::new();
    };
    let clean = |text: &str| -> String {
        text.chars()
            .filter(|c| *c as u32 >= 0x20 && *c != OBJECT_REPLACEMENT_CHARACTER)
            .collect()
    };
    let nth = match word {
        SortWord::All => return clean(field),
        SortWord::First => 0,
        SortWord::Second => 1,
    };
    let separator = |c: char| c == ' ' || (delimiter != '\t' && c == '\t');
    field
        .split(separator)
        .filter(|word| {
            !word
                .chars()
                .all(|c| (c as u32) < 0x20 || c == OBJECT_REPLACEMENT_CHARACTER)
        })
        .nth(nth)
        .map(clean)
        .unwrap_or_default()
}

/// One record's keys, level by level.
fn keys_of(paragraphs: &[String], settings: &SortSettings) -> Vec<Key> {
    let delimiter = settings.delimiter.unwrap_or('\t');
    keys_of_fields(&fields_of(paragraphs, delimiter), settings)
}

/// One record's keys from its fields (a table's cells).
pub(crate) fn keys_of_fields(fields: &[String], settings: &SortSettings) -> Vec<Key> {
    let delimiter = settings.delimiter.unwrap_or('\t');
    settings
        .keys
        .iter()
        .map(|key| {
            let text = key_text(
                fields.get(usize::from(key.field).wrapping_sub(1)),
                key.word,
                delimiter,
            );
            match key.kind {
                SortKind::Alphanumeric => Key::Text(text),
                SortKind::Numeric => number_key(&text),
            }
        })
        .collect()
}

fn compare_records(a: &[Key], b: &[Key]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(a, b)| compare_key(a, b))
        .find(|order| *order != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

/// Ami Pro's heap sort (`1360:23de`, `17f6`, `259c`): which record goes
/// where. `result[i]` is the original index of the record that ends up
/// `i`th. With fewer than two keys' worth of records the order is as it was.
pub(crate) fn sorted_order(records: &[Vec<Key>], descending: bool) -> Vec<usize> {
    let count = records.len();
    let mut heap: Vec<usize> = (0..count).collect();
    if count < 2 {
        return heap;
    }
    // Ascending keeps the smallest key at the root; `worse(a, b)`: record
    // `a` belongs deeper in the heap than `b`.
    let worse = |a: usize, b: usize| {
        let order = compare_records(&records[a], &records[b]);
        if descending {
            order == Ordering::Less
        } else {
            order == Ordering::Greater
        }
    };
    let sift = |heap: &mut Vec<usize>, mut node: usize, last: usize| {
        loop {
            let first_child = node * 2 + 1;
            if first_child > last {
                return;
            }
            let mut child = first_child;
            if first_child < last && worse(heap[first_child], heap[first_child + 1]) {
                child = first_child + 1;
            }
            if !worse(heap[node], heap[child]) {
                return;
            }
            heap.swap(node, child);
            node = child;
        }
    };
    for node in (0..=(count - 2) / 2).rev() {
        sift(&mut heap, node, count - 1);
    }
    for end in (1..count).rev() {
        heap.swap(0, end);
        sift(&mut heap, 0, end - 1);
    }
    // Written back from the last record to the first.
    heap.reverse();
    heap
}

/// The order of records given as their paragraphs' texts.
pub fn order_of_records(records: &[Vec<String>], settings: &SortSettings) -> Vec<usize> {
    let keys: Vec<Vec<Key>> = records.iter().map(|r| keys_of(r, settings)).collect();
    sorted_order(&keys, settings.descending)
}

/// The text a paragraph sorts by: without object marks.
fn sort_text(paragraph: &Paragraph) -> String {
    paragraph.text.replace(OBJECT_REPLACEMENT_CHARACTER, "")
}

/// Sorts paragraphs `first..=last` of `flow` as one undo step; the caret
/// goes to the start of the first.
pub(crate) fn sort_paragraphs(
    document: &Document,
    flow: FlowId,
    first: usize,
    last: usize,
    settings: &SortSettings,
) -> Result<Applied, EditError> {
    if settings.keys.is_empty() || settings.keys.len() > 3 || settings.record == 0 {
        return Err("the sort needs one to three keys and a record size");
    }
    let paragraphs = &document.flow(flow).ok_or("no such flow")?.paragraphs;
    if first > last || last >= paragraphs.len() {
        return Err("no such paragraphs");
    }
    let range = first..=last;
    let breaks = paragraphs[range.clone()].iter().any(|paragraph| {
        paragraph
            .objects
            .iter()
            .any(|object| matches!(object, InlineObject::Break { .. }))
    });
    if breaks && !settings.confirmed {
        return Err(SORT_ACROSS_BREAK);
    }
    let selected: Vec<Arc<Paragraph>> = paragraphs[range].to_vec();
    let records: Vec<&[Arc<Paragraph>]> = selected.chunks(settings.record).collect();
    if records.len() < 2 {
        return Err(SORT_NOTHING);
    }
    let texts: Vec<Vec<String>> = records
        .iter()
        .map(|record| record.iter().map(|p| sort_text(p)).collect())
        .collect();
    let order = order_of_records(&texts, settings);
    let sorted: Vec<Arc<Paragraph>> = order
        .iter()
        .flat_map(|&index| records[index].iter().cloned())
        .collect();
    if sorted.iter().zip(&selected).all(|(a, b)| Arc::ptr_eq(a, b)) {
        return Err(SORT_NOTHING);
    }
    let mut next = document.clone();
    let target = &mut next.flow_mut(flow).ok_or("no such flow")?.paragraphs;
    target.splice(first..=last, sorted.iter().cloned());
    crate::references::renumber(&mut next);
    let mut report = ChangeReport {
        changed: sorted.iter().map(|p| p.id).collect(),
        ..ChangeReport::default()
    };
    report.flows.insert(flow);
    Ok(Applied {
        caret: Some(TextPosition {
            flow,
            paragraph: first,
            byte: 0,
        }),
        document: next,
        report,
    })
}

#[cfg(test)]
mod tests;
