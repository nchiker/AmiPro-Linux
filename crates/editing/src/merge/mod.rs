//! Ami Pro's merge (`docs/research/MERGE_NOTES.md`; `UI_SPEC.md` §38):
//! the document's merge settings, data files of every kind
//! ([`data`]), conditions ([`conditions`]), and the merge itself.

pub mod conditions;
pub mod data;
pub mod envelopes;
pub mod labels;

use amipro_document::Document;
use amipro_document::merge::MergeSettings;

use crate::{Applied, ChangeReport, Command, EditError};

/// The command that gives the document these merge settings: one undo
/// step (the data file, the label values, the conditions).
pub fn set_merge_command(settings: MergeSettings) -> Command {
    Command::SetMerge(Box::new(settings))
}

pub(crate) fn set_merge(
    document: &Document,
    settings: &MergeSettings,
) -> Result<Applied, EditError> {
    if document.merge() == settings {
        return Err("the merge settings are unchanged");
    }
    let mut next = document.clone();
    next.set_merge(settings.clone());
    Ok(Applied {
        document: next,
        caret: None,
        report: ChangeReport {
            info: true,
            ..Default::default()
        },
    })
}

use std::collections::BTreeMap;

use amipro_document::{FieldContext, FlowId, power_fields, power_kind};

use crate::field_language::{FieldHost, Value};
use crate::fields::{FieldInputs, UpdateScope, update_fields};
use crate::{TextPosition, apply};
use data::DataTable;

/// One document a merge makes: the merge document with one record's
/// values (or several, with `NextRec`).
#[derive(Clone, Debug)]
pub struct MergedRecord {
    pub document: Document,
    /// The records it took, as indexes into the data file's records.
    pub records: Vec<usize>,
    /// Its `MergeRec`: its first record's number among the selected ones.
    pub number: u32,
}

/// The merge's host: the current record for the field language, over the
/// app's own host (prompts, Include).
struct RecordHost<'h, 't> {
    inner: &'h mut dyn FieldHost,
    table: &'t DataTable,
    selected: &'t [usize],
    position: usize,
    taken: Vec<usize>,
    skipped: bool,
}

impl RecordHost<'_, '_> {
    fn record(&self) -> Option<usize> {
        self.selected.get(self.position).copied()
    }
}

impl FieldHost for RecordHost<'_, '_> {
    fn include(
        &mut self,
        file: &str,
        bookmark: Option<&str>,
        paragraph_ends: bool,
        oem: bool,
    ) -> Option<Vec<String>> {
        self.inner.include(file, bookmark, paragraph_ends, oem)
    }

    fn query(&mut self, message: &str, default: &str) -> Option<String> {
        self.inner.query(message, default)
    }

    fn decide(&mut self, message: &str) -> Option<bool> {
        self.inner.decide(message)
    }

    fn warn(&mut self, message: String) {
        self.inner.warn(message);
    }

    fn bookmark_page(&mut self, name: &str) -> Option<u32> {
        self.inner.bookmark_page(name)
    }

    fn merge_value(&mut self, name: &str) -> Option<String> {
        self.table.field(name)?;
        // Past the last record (a `NextRec` too many) the fields are empty.
        Some(
            self.record()
                .and_then(|record| self.table.value(record, name))
                .unwrap_or("")
                .to_string(),
        )
    }

    fn merge_record(&mut self) -> Option<u32> {
        Some(self.position as u32 + 1)
    }

    fn next_record(&mut self) {
        self.position += 1;
        if let Some(record) = self.record() {
            self.taken.push(record);
        }
    }

    fn skip_record(&mut self) {
        self.skipped = true;
    }
}

/// What a merge is worked with.
pub struct MergeInputs<'a> {
    pub context: &'a FieldContext,
    pub host: &'a mut dyn FieldHost,
    /// MG9's tidying (doubled spaces, leading commas) besides Ami Pro's
    /// rule for paragraphs holding only empty fields.
    pub tidy: bool,
    /// Merge, view & print: the fields take the record in the document as
    /// it is, and nothing is left out or tidied (Ami Pro works in place, so
    /// edits made while viewing carry to later records).
    pub in_place: bool,
}

/// Works `document` with one record: its fields given the record's values
/// (every flow, in Ami Pro's stream order), as the preview shows it.
pub fn merge_one(
    document: &Document,
    table: &DataTable,
    selected: &[usize],
    position: usize,
    inputs: &mut MergeInputs,
) -> Option<MergedRecord> {
    let mut variables: BTreeMap<String, Value> = BTreeMap::new();
    merge_at(document, table, selected, position, inputs, &mut variables).map(|(record, _)| record)
}

fn merge_at(
    document: &Document,
    table: &DataTable,
    selected: &[usize],
    position: usize,
    inputs: &mut MergeInputs,
    variables: &mut BTreeMap<String, Value>,
) -> Option<(MergedRecord, usize)> {
    let mut host = RecordHost {
        inner: &mut *inputs.host,
        table,
        selected,
        position,
        taken: selected.get(position).copied().into_iter().collect(),
        skipped: false,
    };
    let update = {
        let mut field_inputs = FieldInputs {
            context: inputs.context,
            variables,
            host: &mut host,
        };
        update_fields(document, UpdateScope::All, &mut field_inputs)
    };
    let mut merged = match &update.command {
        Some(command) => {
            apply(document, command).map_or_else(|_| document.clone(), |applied| applied.document)
        }
        None => document.clone(),
    };
    let next = host.position + 1;
    if host.skipped {
        return Some((
            MergedRecord {
                document: merged,
                records: Vec::new(),
                number: position as u32 + 1,
            },
            next,
        ));
    }
    if !inputs.in_place {
        drop_empty_field_paragraphs(&mut merged);
        if inputs.tidy {
            tidy_empty_fields(&mut merged);
        }
    }
    Some((
        MergedRecord {
            document: merged,
            records: host.taken,
            number: position as u32 + 1,
        },
        next,
    ))
}

/// Where a place in `shown`, a record merged into `source` (the preview),
/// is in `source`: its paragraph by id, the same place in the text between
/// fields, and after the field when it is in a field's value (typing there
/// goes after the field, MC11). The user's second use (2026-11-04): the
/// preview's caret, kept as it was when the preview went off, put Enter
/// and fields inside the letter's fields.
pub fn source_position(source: &Document, shown: &Document, at: TextPosition) -> TextPosition {
    let clamp = |at: TextPosition| {
        let Some(flow) = source.flow(at.flow) else {
            return TextPosition {
                flow: source.main_flow_id(),
                paragraph: 0,
                byte: 0,
            };
        };
        let paragraph = at.paragraph.min(flow.paragraphs.len().saturating_sub(1));
        let text = flow
            .paragraphs
            .get(paragraph)
            .map_or("", |paragraph| paragraph.text.as_str());
        TextPosition {
            flow: at.flow,
            paragraph,
            byte: char_floor(text, at.byte),
        }
    };
    let Some(shown_paragraph) = shown
        .flow(at.flow)
        .and_then(|flow| flow.paragraphs.get(at.paragraph))
    else {
        return clamp(at);
    };
    let Some((index, paragraph)) = source.flow(at.flow).and_then(|flow| {
        flow.paragraphs
            .iter()
            .enumerate()
            .find(|(_, paragraph)| paragraph.id == shown_paragraph.id)
    }) else {
        return clamp(at);
    };
    TextPosition {
        flow: at.flow,
        paragraph: index,
        byte: source_byte(paragraph, shown_paragraph, at.byte),
    }
}

fn char_floor(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

/// `byte` of `shown` in `source`, the same paragraph with other field
/// values: the power marks the two share (by code, in order; tidying may
/// have taken an empty field's out) bound the same pieces in both, and the
/// paragraph's end is its end.
fn source_byte(
    source: &amipro_document::Paragraph,
    shown: &amipro_document::Paragraph,
    byte: usize,
) -> usize {
    if byte >= shown.text.len() {
        return source.text.len();
    }
    let marks =
        |paragraph: &amipro_document::Paragraph| -> Vec<(usize, amipro_document::PowerMark)> {
            paragraph
                .marks
                .iter()
                .filter_map(|mark| {
                    amipro_document::PowerMark::parse(&mark.raw).map(|power| (mark.byte, power))
                })
                .collect()
        };
    let (shown_marks, source_marks) = (marks(shown), marks(source));
    // Pairs of (shown byte, source byte, end mark), matched in order.
    let mut bounds: Vec<(usize, usize, bool)> = Vec::new();
    let mut next = 0;
    for (at, mark) in &shown_marks {
        if let Some(found) = source_marks[next..]
            .iter()
            .position(|(_, other)| other.end == mark.end && other.code == mark.code)
        {
            let (source_at, _) = source_marks[next + found];
            bounds.push((*at, source_at, mark.end));
            next += found + 1;
        }
    }
    let mut depth = 0usize;
    let (mut shown_start, mut source_start) = (0, 0);
    for k in 0..=bounds.len() {
        let (shown_end, source_end) = bounds.get(k).map_or(
            (shown.text.len(), source.text.len()),
            |&(shown, source, _)| (shown, source),
        );
        if byte <= shown_end || k == bounds.len() {
            if depth == 0 {
                let place = source_start + byte.saturating_sub(shown_start);
                return char_floor(&source.text, place.min(source_end));
            }
            // In a field's value: after the outermost field.
            let mut open = depth;
            for &(_, at, end) in &bounds[k..] {
                if end {
                    open -= 1;
                    if open == 0 {
                        return at;
                    }
                } else {
                    open += 1;
                }
            }
            return source.text.len();
        }
        if bounds[k].2 {
            depth = depth.saturating_sub(1);
        } else {
            depth += 1;
        }
        (shown_start, source_start) = (shown_end, source_end);
    }
    char_floor(&source.text, byte)
}

/// Merges `document` with every selected record (`1550:0599`): each
/// document starts at the record after the last one the previous took
/// (`NextRec` takes more); a record whose fields reach `Skip` is left out
/// but counted by `MergeRec`.
pub fn merge_all(
    document: &Document,
    table: &DataTable,
    selected: &[usize],
    inputs: &mut MergeInputs,
) -> Vec<MergedRecord> {
    let mut variables: BTreeMap<String, Value> = BTreeMap::new();
    let mut out = Vec::new();
    let mut position = 0;
    while position < selected.len() {
        let Some((record, next)) =
            merge_at(document, table, selected, position, inputs, &mut variables)
        else {
            break;
        };
        if !record.records.is_empty() {
            out.push(record);
        }
        position = next.max(position + 1);
    }
    out
}

/// The names the document's merge fields take (`MergeField NAME`), in
/// Ami Pro's keys, each once, in stream order.
pub fn merge_field_names(document: &Document) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for stream in crate::find::streams(document) {
        let Some(flow) = document.flow(stream.flow) else {
            continue;
        };
        for field in power_fields(flow.paragraphs.iter().map(AsRef::as_ref)) {
            if field.mark.end {
                continue;
            }
            let mut words = field.mark.code.split_whitespace();
            while let Some(word) = words.next() {
                if word.eq_ignore_ascii_case("mergefield")
                    && let Some(name) = words.next()
                {
                    let key = data::field_key(name.trim_matches('"'));
                    if !key.is_empty() && !names.contains(&key) {
                        names.push(key);
                    }
                }
            }
        }
    }
    names
}

/// The document's merge fields the data file lacks (Ami Pro's 3225 and
/// 3098: "There are merge fields in the current document that do not match
/// this records file"; VM MGM8). A merge shows them empty.
pub fn unknown_fields(document: &Document, table: &DataTable) -> Vec<String> {
    merge_field_names(document)
        .into_iter()
        .filter(|name| table.field(name).is_none())
        .collect()
}

/// A paragraph's text outside its objects: what shows when its fields'
/// results are as they are.
fn shows_nothing(text: &str) -> bool {
    text.chars().all(|character| {
        character.is_whitespace() || character == amipro_document::OBJECT_REPLACEMENT_CHARACTER
    }) && !text.contains(amipro_document::OBJECT_REPLACEMENT_CHARACTER)
}

/// Whether a field belongs to the merge: a merge field, or a field that
/// takes records (`NextRec`, `Skip`), whose paragraphs Ami Pro writes with
/// `<:S+0>` and shows without height (VM MGM1, MGM6, MGS6).
fn is_merge_mark(mark: &amipro_document::PowerMark) -> bool {
    if mark.kind == power_kind::MERGE_FIELD {
        return true;
    }
    let code = mark.code.to_ascii_lowercase();
    code.split_whitespace()
        .any(|word| word == "nextrec" || word == "skip")
}

/// Ami Pro's rule for empty fields (G ch. 28; MG9): a paragraph holding
/// only merge fields whose results are empty takes no space, so merged
/// output leaves it out. Paragraphs without merge fields are kept, empty
/// or not.
pub fn drop_empty_field_paragraphs(document: &mut Document) {
    let flows: Vec<FlowId> = crate::find::streams(document)
        .into_iter()
        .map(|stream| stream.flow)
        .collect();
    for flow in flows {
        let Some(paragraphs) = document.flow(flow).map(|flow| flow.paragraphs.clone()) else {
            continue;
        };
        let fields = power_fields(paragraphs.iter().map(AsRef::as_ref));
        let mut drop: Vec<usize> = paragraphs
            .iter()
            .enumerate()
            .filter(|(index, paragraph)| {
                shows_nothing(&paragraph.text)
                    && fields
                        .iter()
                        .any(|field| field.start.paragraph == *index && is_merge_mark(&field.mark))
                    && paragraphs.len() > 1
            })
            .map(|(index, _)| index)
            .collect();
        // A flow keeps at least one paragraph.
        if drop.len() == paragraphs.len() {
            drop.pop();
        }
        for index in drop.into_iter().rev() {
            let (start, end) = if index + 1 < paragraphs.len() {
                (
                    TextPosition {
                        flow,
                        paragraph: index,
                        byte: 0,
                    },
                    TextPosition {
                        flow,
                        paragraph: index + 1,
                        byte: 0,
                    },
                )
            } else {
                let previous = &paragraphs[index - 1];
                (
                    TextPosition {
                        flow,
                        paragraph: index - 1,
                        byte: previous.text.len(),
                    },
                    TextPosition {
                        flow,
                        paragraph: index,
                        byte: paragraphs[index].text.len(),
                    },
                )
            };
            if let Ok(applied) = apply(
                document,
                &Command::ReplaceText {
                    start,
                    end,
                    text: String::new(),
                },
            ) {
                *document = applied.document;
                // The paragraph left at `index` holds the next one's text:
                // it keeps that one's id, so a place in the merged document
                // maps back to its paragraph (`source_position`).
                if index + 1 < paragraphs.len()
                    && let Some(kept) = document
                        .flow_mut(flow)
                        .and_then(|flow| flow.paragraph_mut(index))
                {
                    kept.id = paragraphs[index + 1].id;
                }
            }
        }
    }
}

/// MG9's tidying where an empty merge field left them: a doubled space
/// becomes one, and a comma with nothing before it on its line goes.
pub fn tidy_empty_fields(document: &mut Document) {
    let flows: Vec<FlowId> = crate::find::streams(document)
        .into_iter()
        .map(|stream| stream.flow)
        .collect();
    for flow in flows {
        while let Some(paragraphs) = document.flow(flow).map(|flow| flow.paragraphs.clone()) {
            let fields = power_fields(paragraphs.iter().map(AsRef::as_ref));
            let mut cut: Option<(usize, usize, usize)> = None;
            for field in &fields {
                if field.mark.kind != power_kind::MERGE_FIELD {
                    continue;
                }
                let Some(end) = field.end else { continue };
                if end.paragraph != field.start.paragraph || end.byte != field.start.byte {
                    continue;
                }
                let paragraph = field.start.paragraph;
                let text = &paragraphs[paragraph].text;
                let at = field.start.byte;
                let before = &text[..at];
                let after = &text[at..];
                if before.ends_with(' ') && (after.starts_with(' ') || after.is_empty()) {
                    cut = Some((paragraph, at - 1, at));
                } else if before.trim().is_empty() && after.starts_with(", ") {
                    cut = Some((paragraph, at, at + 2));
                } else if before.ends_with(", ")
                    && (after.starts_with(',') || after.trim().is_empty())
                {
                    cut = Some((paragraph, at - 2, at));
                }
                if cut.is_some() {
                    break;
                }
            }
            let Some((paragraph, from, to)) = cut else {
                break;
            };
            let command = Command::ReplaceText {
                start: TextPosition {
                    flow,
                    paragraph,
                    byte: from,
                },
                end: TextPosition {
                    flow,
                    paragraph,
                    byte: to,
                },
                text: String::new(),
            };
            match apply(document, &command) {
                Ok(applied) => *document = applied.document,
                Err(_) => break,
            }
        }
    }
}

/// The merged records as one document (Merge to New Document, and Ami
/// Pro's Merge & save as): the first record's document, then each later
/// one inserted at the end after a page break (`1550:0354`), frames and
/// all.
pub fn joined(records: &[MergedRecord]) -> Option<Document> {
    let mut document = records.first()?.document.clone();
    for record in &records[1..] {
        let main = document.main_flow_id();
        let end = |document: &Document| {
            let paragraphs = &document.main_flow().paragraphs;
            let last = paragraphs.len() - 1;
            TextPosition {
                flow: main,
                paragraph: last,
                byte: paragraphs[last].text.len(),
            }
        };
        let at = end(&document);
        let commands = [
            Command::InsertBreak {
                at,
                kind: amipro_document::BreakKind::Page,
                centre: false,
                layout: None,
            },
            Command::ReplaceText {
                start: at,
                end: at,
                text: "\n".into(),
            },
        ];
        for command in &commands {
            if let Ok(applied) = apply(&document, command) {
                document = applied.document;
            }
        }
        let at = end(&document);
        let insert = Command::InsertDocument {
            at,
            source: crate::InsertedDocument(std::sync::Arc::new(record.document.clone())),
            first_page: 1,
        };
        if let Ok(applied) = apply(&document, &insert) {
            document = applied.document;
        }
    }
    Some(document)
}
