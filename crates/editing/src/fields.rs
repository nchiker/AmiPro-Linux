//! Fields in the text (`docs/research/FIELDS_NOTES.md`): inserting dates,
//! Doc Info entries, and power fields, and updating power fields by Ami
//! Pro's rule (decision FL4): on insertion, by Update and Update All, and
//! Auto run ones when the document opens (`1648:0000`, `1648:02a0`,
//! `1648:1103`, `1648:0c3b`). Each update is one undo step.

use std::collections::{BTreeMap, BTreeSet};

use amipro_document::{
    CharacterFormat, CharacterRun, Document, FieldContext, FieldSpan, FlowId, InlineObject,
    MarkPlace, Paragraph, PowerMark, SeqField, SetAside, Shown, has_result, power_fields,
    power_flag, power_kind,
};

use crate::field_language::{
    FieldEnvironment, FieldHost, Outcome, Value, evaluate, field_kind, seq_name, seq_number,
};
use crate::{Applied, ChangeReport, Command, EditError, TextPosition, replace_range};

/// Which fields an update takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateScope {
    /// Update All: every field but locked ones and those that show nothing
    /// by themselves (Button, PrintEscape, Index, User, TOC); Seq renumbered.
    All,
    /// Update: the field at the caret, unless locked.
    At(TextPosition),
    /// Update with text selected: the fields that start in it.
    Between(TextPosition, TextPosition),
    /// The Auto run fields, as the document opens (locked or not).
    AutoRun,
}

/// What an update found and would change.
#[derive(Debug, Default)]
pub struct FieldUpdate {
    /// The changes as one undo step, if any.
    pub command: Option<Command>,
    /// Fields whose prompts are not answered yet.
    pub waiting: usize,
    /// How many fields were worked out.
    pub updated: usize,
}

/// What fields are worked out with, besides the document.
pub struct FieldInputs<'a> {
    pub context: &'a FieldContext,
    pub variables: &'a mut BTreeMap<String, Value>,
    pub host: &'a mut dyn FieldHost,
}

/// The document's bookmark names, every kind, in small letters.
pub fn bookmark_names(document: &Document) -> BTreeSet<String> {
    amipro_document::bookmarks::bookmarks(document)
        .into_iter()
        .map(|bookmark| bookmark.name.to_lowercase())
        .collect()
}

/// A power field found in a stream, or a Seq object.
#[derive(Clone, Debug)]
enum Found {
    Marks(FieldSpan),
    Seq {
        paragraph: usize,
        byte: usize,
        field: SeqField,
    },
}

/// The fields of one flow in order: marks and Seq objects by place.
fn flow_fields(document: &Document, flow: FlowId) -> Vec<Found> {
    let Some(paragraphs) = document.flow(flow).map(|flow| &flow.paragraphs) else {
        return Vec::new();
    };
    let mut found: Vec<(usize, usize, usize, Found)> =
        power_fields(paragraphs.iter().map(AsRef::as_ref))
            .into_iter()
            .map(|span| (span.start.paragraph, span.start.byte, 0, Found::Marks(span)))
            .collect();
    for (index, paragraph) in paragraphs.iter().enumerate() {
        for (byte, object) in paragraph.object_offsets().zip(&paragraph.objects) {
            if let InlineObject::Seq(field) = object {
                found.push((
                    index,
                    byte,
                    1,
                    Found::Seq {
                        paragraph: index,
                        byte,
                        field: field.clone(),
                    },
                ));
            }
        }
    }
    found.sort_by_key(|(paragraph, byte, order, _)| (*paragraph, *byte, *order));
    found.into_iter().map(|(_, _, _, field)| field).collect()
}

/// Works out the fields `scope` takes and the changes that give them their
/// results (`1648:02a0`, `1550:0635`).
pub fn update_fields(
    document: &Document,
    scope: UpdateScope,
    inputs: &mut FieldInputs,
) -> FieldUpdate {
    let bookmarks = bookmark_names(document);
    let mut changes: Vec<Command> = Vec::new();
    let mut update = FieldUpdate::default();
    let flows: Vec<FlowId> = match scope {
        UpdateScope::At(at) => vec![at.flow],
        UpdateScope::Between(start, _) => vec![start.flow],
        UpdateScope::All | UpdateScope::AutoRun => crate::find::streams(document)
            .into_iter()
            .map(|stream| stream.flow)
            .collect(),
    };
    for flow in flows {
        let fields = flow_fields(document, flow);
        let at_caret = match scope {
            UpdateScope::At(at) => innermost_at(&fields, at),
            _ => None,
        };
        let mut seq: BTreeMap<String, u32> = BTreeMap::new();
        let mut flow_changes: Vec<Command> = Vec::new();
        for (index, found) in fields.iter().enumerate() {
            let mut environment = FieldEnvironment {
                document,
                context: inputs.context,
                variables: inputs.variables,
                bookmarks: &bookmarks,
                host: inputs.host,
            };
            match found {
                Found::Seq {
                    paragraph,
                    byte,
                    field,
                } => {
                    let name = seq_name(&field.code);
                    let previous = seq
                        .get(&name)
                        .copied()
                        .unwrap_or_else(|| carried_seq(document, flow, &name));
                    let renumber = match scope {
                        UpdateScope::All => true,
                        UpdateScope::At(_) => at_caret == Some(index),
                        UpdateScope::Between(start, end) => {
                            within(start, end, flow, *paragraph, *byte)
                        }
                        UpdateScope::AutoRun => false,
                    };
                    let number = if renumber {
                        seq_number(&field.code, previous, &mut environment)
                    } else {
                        field.number()
                    };
                    seq.insert(name, number);
                    if number != field.number() {
                        let value = (field.value & !0x3FFF) | number as u16;
                        flow_changes.push(Command::SetFieldObject {
                            at: TextPosition {
                                flow,
                                paragraph: *paragraph,
                                byte: *byte,
                            },
                            object: InlineObject::Seq(SeqField {
                                code: field.code.clone(),
                                value,
                            }),
                        });
                        update.updated += 1;
                    }
                }
                Found::Marks(span) => {
                    let mark = &span.mark;
                    let taken = match scope {
                        UpdateScope::All => {
                            !mark.locked()
                                && !matches!(
                                    mark.kind,
                                    power_kind::BUTTON
                                        | power_kind::PRINT_ESCAPE
                                        | power_kind::INDEX
                                        | power_kind::USER
                                        | power_kind::TOC
                                )
                        }
                        UpdateScope::At(_) => at_caret == Some(index) && !mark.locked(),
                        UpdateScope::Between(start, end) => {
                            within(start, end, flow, span.start.paragraph, span.start.byte)
                                && !mark.locked()
                        }
                        UpdateScope::AutoRun => mark.auto_run(),
                    };
                    // Set fields run for their variables wherever they are
                    // (Ami Pro runs them as it reads the file).
                    if !taken && mark.kind != power_kind::SET {
                        continue;
                    }
                    let outcome = evaluate(&mark.code, &mut environment);
                    if !taken {
                        continue;
                    }
                    match outcome {
                        Outcome::Waiting => update.waiting += 1,
                        Outcome::Kept => {}
                        Outcome::Nothing | Outcome::Result(_) => {
                            update.updated += 1;
                            // A field that read NumPages is updated again
                            // after pagination (`1648:061e`; VM twin AFD1).
                            // Its marks are rewritten where they are now,
                            // before its result changes length (the changes
                            // apply last first).
                            let marks_change = (reads_page_count(&mark.code)
                                && mark.value & power_flag::AFTER_PAGINATION == 0)
                                .then(|| Command::SetFieldMarks {
                                    flow,
                                    start: span.start,
                                    end: span.end,
                                    mark: PowerMark {
                                        value: mark.value | power_flag::AFTER_PAGINATION,
                                        ..mark.clone()
                                    },
                                });
                            if let Some(end) = span.end
                                && end.paragraph == span.start.paragraph
                            {
                                let result = match outcome {
                                    // Include's paragraphs as lines of one
                                    // paragraph (FX8).
                                    Outcome::Result(paragraphs) => paragraphs.join("\u{2028}"),
                                    _ => String::new(),
                                };
                                let current = document
                                    .flow(flow)
                                    .and_then(|flow| flow.paragraph(end.paragraph))
                                    .map(|paragraph| &paragraph.text[span.start.byte..end.byte]);
                                if current != Some(result.as_str()) {
                                    flow_changes.push(Command::SetFieldResult {
                                        flow,
                                        start: span.start,
                                        end,
                                        result,
                                    });
                                }
                            }
                            flow_changes.extend(marks_change);
                        }
                    }
                }
            }
        }
        // Last first, so that each change leaves the places before it.
        flow_changes.reverse();
        changes.extend(flow_changes);
    }
    if !changes.is_empty() {
        update.command = Some(if changes.len() == 1 {
            changes.remove(0)
        } else {
            Command::Several(changes)
        });
    }
    update
}

/// Whether a code reads the page count (Ami Pro's NumPages).
fn reads_page_count(code: &str) -> bool {
    code.split(|character: char| !character.is_alphanumeric())
        .any(|word| word.eq_ignore_ascii_case("numpages"))
}

fn within(
    start: TextPosition,
    end: TextPosition,
    flow: FlowId,
    paragraph: usize,
    byte: usize,
) -> bool {
    flow == start.flow
        && (start.paragraph, start.byte) <= (paragraph, byte)
        && (paragraph, byte) <= (end.paragraph, end.byte)
}

/// The field holding `at`: the last one starting at or before it whose
/// result (or object) holds it (`1638:0438`).
fn innermost_at(fields: &[Found], at: TextPosition) -> Option<usize> {
    fields
        .iter()
        .enumerate()
        .rev()
        .find(|(_, found)| holds(found, at))
        .map(|(index, _)| index)
}

fn holds(found: &Found, at: TextPosition) -> bool {
    let place = (at.paragraph, at.byte);
    match found {
        Found::Seq {
            paragraph, byte, ..
        } => place == (*paragraph, *byte) || place == (*paragraph, *byte + 3),
        Found::Marks(span) => {
            let start = (span.start.paragraph, span.start.byte);
            let end = span.end.map_or(start, |end| (end.paragraph, end.byte));
            start <= place && place <= end
        }
    }
}

/// A field of the text, as the Fields panel and the status bar name it.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldAt {
    /// A date, the time, or a Doc Info entry, at the U+FFFC at `at`.
    Object {
        at: TextPosition,
        object: InlineObject,
    },
    /// A power field kept as marks.
    Power { flow: FlowId, span: FieldSpan },
}

impl FieldAt {
    /// The text range the whole field takes, to select it.
    pub fn range(&self) -> (TextPosition, TextPosition) {
        match self {
            Self::Object { at, .. } => (
                *at,
                TextPosition {
                    byte: at.byte + amipro_document::OBJECT_REPLACEMENT_CHARACTER.len_utf8(),
                    ..*at
                },
            ),
            Self::Power { flow, span } => {
                let end = span.end.unwrap_or(span.start);
                (
                    TextPosition {
                        flow: *flow,
                        paragraph: span.start.paragraph,
                        byte: span.start.byte,
                    },
                    TextPosition {
                        flow: *flow,
                        paragraph: end.paragraph,
                        byte: end.byte,
                    },
                )
            }
        }
    }
}

/// The field at `at`: an object right after or before it, or the innermost
/// power field holding it.
pub fn field_at(document: &Document, at: TextPosition) -> Option<FieldAt> {
    let paragraph = document.flow(at.flow)?.paragraph(at.paragraph)?;
    let object_here = paragraph
        .object_offsets()
        .zip(&paragraph.objects)
        .find(|(byte, object)| {
            is_field_object(object)
                && (*byte == at.byte
                    || *byte + amipro_document::OBJECT_REPLACEMENT_CHARACTER.len_utf8() == at.byte)
        })
        .map(|(byte, object)| FieldAt::Object {
            at: TextPosition { byte, ..at },
            object: object.clone(),
        });
    if object_here.is_some() {
        return object_here;
    }
    let fields = flow_fields(document, at.flow);
    let index = innermost_at(&fields, at)?;
    match &fields[index] {
        Found::Marks(span) => Some(FieldAt::Power {
            flow: at.flow,
            span: span.clone(),
        }),
        Found::Seq {
            paragraph,
            byte,
            field,
        } => Some(FieldAt::Object {
            at: TextPosition {
                flow: at.flow,
                paragraph: *paragraph,
                byte: *byte,
            },
            object: InlineObject::Seq(field.clone()),
        }),
    }
}

/// Whether an object is a field: a date, a Doc Info entry, or a Seq.
pub fn is_field_object(object: &InlineObject) -> bool {
    matches!(
        object,
        InlineObject::Date(_) | InlineObject::DocInfo(_) | InlineObject::Seq(_)
    )
}

/// Every field of the document in Ami Pro's stream order, for Next Field
/// and Previous Field (`1648:0d22`, `1648:0f11`).
pub fn all_fields(document: &Document) -> Vec<FieldAt> {
    let mut all = Vec::new();
    for stream in crate::find::streams(document) {
        let flow = stream.flow;
        let mut here: Vec<((usize, usize), FieldAt)> = Vec::new();
        for found in flow_fields(document, flow) {
            if let Found::Marks(span) = found {
                here.push((
                    (span.start.paragraph, span.start.byte),
                    FieldAt::Power { flow, span },
                ));
            }
        }
        if let Some(paragraphs) = document.flow(flow).map(|flow| &flow.paragraphs) {
            for (index, paragraph) in paragraphs.iter().enumerate() {
                for (byte, object) in paragraph.object_offsets().zip(&paragraph.objects) {
                    if is_field_object(object) {
                        here.push((
                            (index, byte),
                            FieldAt::Object {
                                at: TextPosition {
                                    flow,
                                    paragraph: index,
                                    byte,
                                },
                                object: object.clone(),
                            },
                        ));
                    }
                }
            }
        }
        here.sort_by_key(|(place, _)| *place);
        all.extend(here.into_iter().map(|(_, field)| field));
    }
    all
}

/// The field after (or before) `at` in Ami Pro's stream order, round to the
/// start, as Next Field and Previous Field select it.
pub fn next_field(document: &Document, at: TextPosition, forward: bool) -> Option<FieldAt> {
    let fields = all_fields(document);
    if fields.is_empty() {
        return None;
    }
    let order: Vec<FlowId> = crate::find::streams(document)
        .iter()
        .map(|stream| stream.flow)
        .collect();
    let stream_of = |flow: FlowId| order.iter().position(|id| *id == flow);
    let key =
        |position: TextPosition| (stream_of(position.flow), position.paragraph, position.byte);
    let here = key(at);
    if forward {
        fields
            .iter()
            .find(|field| key(field.range().0) > here)
            .or(fields.first())
            .cloned()
    } else {
        fields
            .iter()
            .rev()
            .find(|field| key(field.range().0) < here)
            .or(fields.last())
            .cloned()
    }
}

/// Inserting the power field `code` at `at`, worked out at once as Ami Pro
/// does (`1648:0000`): its result between its marks, a Seq as its object
/// numbered from the one before it of its name, a field without a result
/// as its mark. `flags` are Lock and Auto run.
pub fn insert_power_field(
    document: &Document,
    at: TextPosition,
    code: &str,
    flags: u16,
    inputs: &mut FieldInputs,
) -> Result<Command, EditError> {
    let kind = field_kind(code);
    let main = if at.flow == document.main_flow_id() {
        power_flag::MAIN_TEXT
    } else {
        0
    };
    let bookmarks = bookmark_names(document);
    let mut environment = FieldEnvironment {
        document,
        context: inputs.context,
        variables: inputs.variables,
        bookmarks: &bookmarks,
        host: inputs.host,
    };
    if kind == power_kind::SEQ {
        let name = seq_name(code);
        let previous = flow_fields(document, at.flow)
            .iter()
            .rev()
            .find_map(|found| match found {
                Found::Seq {
                    paragraph,
                    byte,
                    field,
                } if (*paragraph, *byte) < (at.paragraph, at.byte)
                    && seq_name(&field.code) == name =>
                {
                    Some(field.number())
                }
                _ => None,
            })
            .unwrap_or_else(|| carried_seq(document, at.flow, &name));
        let number = seq_number(code, previous, &mut environment);
        return Ok(Command::InsertFieldObject {
            at,
            object: InlineObject::Seq(SeqField {
                code: code.into(),
                value: main | number as u16,
            }),
        });
    }
    let result = match evaluate(code, &mut environment) {
        Outcome::Result(paragraphs) if has_result(kind) => paragraphs.join("\u{2028}"),
        _ => String::new(),
    };
    Ok(Command::InsertPowerField {
        at,
        mark: PowerMark {
            end: false,
            kind,
            value: main | flags,
            code: code.into(),
        },
        result,
    })
}

// ------------------------------------------------------------ codes shown

/// Ami Pro's colour for a code shown in the text: red (VM fixture FD3).
const CODE_RED: u32 = 0x0000FF;

/// One field's text swapped as codes show or hide: bytes `from..to` of a
/// paragraph became `length` bytes.
#[derive(Clone, Copy, Debug)]
struct Swap {
    flow: FlowId,
    paragraph: usize,
    from: usize,
    to: usize,
    length: usize,
}

/// Where text positions go as View › Show Power Fields swaps results and
/// codes: a position in a swapped field goes to the field's start.
#[derive(Clone, Debug, Default)]
pub struct CodeSwaps(Vec<Swap>);

impl CodeSwaps {
    pub fn map(&self, at: TextPosition) -> TextPosition {
        let mut shift = 0isize;
        for swap in self
            .0
            .iter()
            .filter(|swap| swap.flow == at.flow && swap.paragraph == at.paragraph)
        {
            if at.byte <= swap.from {
                break;
            }
            if at.byte < swap.to {
                return TextPosition {
                    byte: swap.from.saturating_add_signed(shift),
                    ..at
                };
            }
            shift += swap.length as isize - (swap.to - swap.from) as isize;
        }
        TextPosition {
            byte: at.byte.saturating_add_signed(shift),
            ..at
        }
    }
}

/// Whether an object in a result keeps a frame or a flow of its own, which
/// cannot be set aside.
fn owns_content(object: &InlineObject) -> bool {
    matches!(
        object,
        InlineObject::FrameAnchor(_) | InlineObject::FloatingHeaderFooter { .. }
    )
}

/// View › Show Power Fields on (`show`) or off, as Ami Pro switches it
/// (`1638:0d0b`; VM fixture FD3). On: each power field's result is set
/// aside in the document and its code shown in red between `<` and `>` in
/// its place, a Seq's in place of its number, the code of a field without
/// a result after its mark; the codes are ordinary text, edited as any
/// text. Off: each code is read back between its `<` and `>` (text typed
/// outside them stays outside the field; an emptied code deletes the
/// field), a changed code rewrites the field's marks, and the results go
/// back. Nested fields show as their outer field's code; dates and Doc
/// Info entries are as they are. Fields whose result spans paragraphs or
/// holds a frame's anchor keep their result.
pub fn swap_codes(document: &Document, show: bool) -> (Applied, CodeSwaps) {
    let unchanged = || {
        (
            Applied {
                document: document.clone(),
                caret: None,
                report: ChangeReport::default(),
            },
            CodeSwaps::default(),
        )
    };
    if document.shown_codes().is_some() == show {
        return unchanged();
    }
    let mut next = document.clone();
    let mut report = ChangeReport {
        codes: true,
        ..ChangeReport::default()
    };
    let mut swaps = Vec::new();
    if show {
        let mut aside = BTreeMap::new();
        for (flow, _) in document.flows() {
            show_flow(&mut next, flow, &mut aside, &mut swaps, &mut report);
        }
        next.set_shown_codes(Some(aside));
    } else {
        let aside = document.shown_codes().cloned().unwrap_or_default();
        for (flow, _) in document.flows() {
            hide_flow(&mut next, flow, &aside, &mut swaps, &mut report);
        }
        next.set_shown_codes(None);
    }
    report.changed.sort();
    report.changed.dedup();
    swaps.sort_by_key(|swap: &Swap| (swap.flow, swap.paragraph, swap.from));
    (
        Applied {
            document: next,
            caret: None,
            report,
        },
        CodeSwaps(swaps),
    )
}

/// Where the fields' text goes as codes show or hide, without changing
/// the document: for the caret and the selection.
pub fn code_swaps(document: &Document, show: bool) -> CodeSwaps {
    swap_codes(document, show).1
}

/// A place in a flow: a paragraph and a byte in it.
type Place = (usize, usize);

fn show_flow(
    next: &mut Document,
    flow: FlowId,
    aside: &mut BTreeMap<u32, SetAside>,
    swaps: &mut Vec<Swap>,
    report: &mut ChangeReport,
) {
    // The outermost fields with their first and last places (paragraph,
    // byte): those inside another go with its result.
    let mut outer: Vec<(Found, Place, Place)> = Vec::new();
    for found in flow_fields(next, flow) {
        let (start, end) = match &found {
            Found::Marks(span) => {
                let start = (span.start.paragraph, span.start.byte);
                (
                    start,
                    span.end.map_or(start, |end| (end.paragraph, end.byte)),
                )
            }
            Found::Seq {
                paragraph, byte, ..
            } => ((*paragraph, *byte), (*paragraph, *byte + OBJECT_LENGTH)),
        };
        if outer
            .last()
            .is_some_and(|(_, first, last)| *first <= start && end <= *last)
        {
            continue;
        }
        outer.push((found, start, end));
    }
    // Runs crossing a field's edges as the flow was: fields side by side
    // join again as each goes back.
    let original = next
        .flow(flow)
        .map(|flow| flow.paragraphs.clone())
        .unwrap_or_default();
    // Last first, so that each field leaves the places before it.
    for (found, (index, from), (end_index, to)) in outer.into_iter().rev() {
        if end_index != index {
            continue;
        }
        let id = aside.len() as u32 + 1;
        let Some(paragraph) = next
            .flow_mut(flow)
            .and_then(|flow| flow.paragraph_mut(index))
        else {
            continue;
        };
        let objects = objects_between(paragraph, from, to);
        if objects.iter().any(owns_content) {
            continue;
        }
        let (mark, shown) = match &found {
            Found::Marks(span) => (
                span.mark.clone(),
                if span.end.is_some() {
                    Shown::Result
                } else {
                    Shown::NoResult
                },
            ),
            Found::Seq { field, .. } => (
                PowerMark {
                    end: false,
                    kind: power_kind::SEQ,
                    value: field.value,
                    code: field.code.clone(),
                },
                Shown::Seq,
            ),
        };
        let code = format!("<{}>", mark.code);
        let end_mark = amipro_document::LegacyMark {
            byte: code.len(),
            raw: format!(
                "{}\0{id}",
                PowerMark {
                    end: true,
                    ..mark.clone()
                }
                .raw()
            ),
        };
        let kept = SetAside {
            shown,
            text: paragraph.text[from..to].to_owned(),
            runs: crate::runs_for_slice(&paragraph.runs, from, to, 0),
            objects,
            marks: match &found {
                Found::Marks(span) => match span.end {
                    Some(end) => paragraph.marks[span.start.mark + 1..end.mark]
                        .iter()
                        .map(|mark| amipro_document::LegacyMark {
                            byte: mark.byte - from,
                            raw: mark.raw.clone(),
                        })
                        .collect(),
                    None => Vec::new(),
                },
                Found::Seq { .. } => Vec::new(),
            },
            format: base_format(paragraph, from, to),
            joined: [from, to].map(|edge| {
                original[index]
                    .runs
                    .iter()
                    .any(|run| run.start < edge && edge < run.end)
            }),
        };
        let red = CharacterRun {
            start: 0,
            end: code.len(),
            format: CharacterFormat {
                colour: Some(CODE_RED),
                ..kept.format.clone()
            },
        };
        let (marks_out, marks_in) = match &found {
            // The code in place of the result; the end mark, after it,
            // carries the id.
            Found::Marks(span) => match span.end {
                Some(end) => (span.start.mark + 1..end.mark, Vec::new()),
                None => (span.start.mark + 1..span.start.mark + 1, vec![end_mark]),
            },
            // After the marks before the object.
            Found::Seq { .. } => {
                let first = paragraph.marks.partition_point(|kept| kept.byte <= from);
                (
                    first..first,
                    vec![
                        amipro_document::LegacyMark {
                            byte: 0,
                            raw: mark.raw(),
                        },
                        end_mark,
                    ],
                )
            }
        };
        let tagged_end = marks_out.start + marks_in.len().saturating_sub(1);
        let tag_after = matches!(&found, Found::Marks(span) if span.end.is_some());
        splice(
            paragraph,
            from..to,
            marks_out,
            Piece {
                text: &code,
                runs: vec![red],
                objects: Vec::new(),
                marks: marks_in,
            },
            [false, false],
        );
        if tag_after {
            let tagged = &mut paragraph.marks[tagged_end];
            tagged.raw = format!("{}\0{id}", tagged.raw);
        }
        aside.insert(id, kept);
        report.changed.push(paragraph.id);
        report.flows.insert(flow);
        swaps.push(Swap {
            flow,
            paragraph: index,
            from,
            to,
            length: code.len(),
        });
    }
}

fn hide_flow(
    next: &mut Document,
    flow: FlowId,
    aside: &BTreeMap<u32, SetAside>,
    swaps: &mut Vec<Swap>,
    report: &mut ChangeReport,
) {
    let Some(paragraphs) = next.flow(flow).map(|flow| flow.paragraphs.clone()) else {
        return;
    };
    let shown: Vec<FieldSpan> = power_fields(paragraphs.iter().map(AsRef::as_ref))
        .into_iter()
        .filter(|span| {
            span.end.is_some_and(|end| {
                end.paragraph == span.start.paragraph
                    && amipro_document::set_aside_id(&paragraphs[end.paragraph].marks[end.mark].raw)
                        .is_some()
            })
        })
        .collect();
    for span in shown.into_iter().rev() {
        let end = span.end.expect("a shown code has its end mark");
        let Some(paragraph) = next
            .flow_mut(flow)
            .and_then(|flow| flow.paragraph_mut(span.start.paragraph))
        else {
            continue;
        };
        let id = amipro_document::set_aside_id(&paragraph.marks[end.mark].raw);
        let Some(kept) = id.and_then(|id| aside.get(&id)) else {
            continue;
        };
        let (from, to) = (span.start.byte, end.byte);
        if objects_between(paragraph, from, to)
            .iter()
            .any(owns_content)
        {
            continue;
        }
        // The code is what stands between the first `<` and the last `>`;
        // text typed outside them goes outside the field, out of red.
        let text = &paragraph.text[from..to];
        let (code_from, code_to) = match (text.find('<'), text.rfind('>')) {
            (Some(open), Some(close)) if open < close => (from + open, from + close + 1),
            _ => (from, to),
        };
        let code: String = paragraph.text[code_from..code_to]
            .trim_start_matches('<')
            .trim_end_matches('>')
            .chars()
            .filter(|character| *character != amipro_document::OBJECT_REPLACEMENT_CHARACTER)
            .collect();
        let typed = [code_from > from, code_to < to];
        if typed.contains(&true) {
            let red = CharacterFormat {
                colour: Some(CODE_RED),
                ..kept.format.clone()
            };
            for range in [from..code_from, code_to..to] {
                restore_format(paragraph, range, &red, &kept.format);
            }
        }
        let (start_mark, end_mark) = (span.start.mark, end.mark);
        paragraph.marks[start_mark].byte = code_from;
        paragraph.marks[end_mark].byte = code_to;
        let original = span.mark.clone();
        let changed = code != original.code;
        let kind = if changed {
            field_kind(&code)
        } else {
            original.kind
        };
        let join = [kept.joined[0] || typed[0], kept.joined[1] || typed[1]];
        let length = if code.is_empty() {
            // An emptied code deletes the field.
            splice(
                paragraph,
                code_from..code_to,
                start_mark..end_mark + 1,
                Piece::default(),
                [true, true],
            );
            0
        } else if kind == power_kind::SEQ {
            let (value, runs) = match kept.shown {
                Shown::Seq => (original.value, kept.runs.clone()),
                _ => (original.value & power_flag::MAIN_TEXT, Vec::new()),
            };
            let object = InlineObject::Seq(SeqField { code, value });
            let character = amipro_document::OBJECT_REPLACEMENT_CHARACTER.to_string();
            splice(
                paragraph,
                code_from..code_to,
                start_mark..end_mark + 1,
                Piece {
                    text: &character,
                    runs,
                    objects: vec![object],
                    marks: Vec::new(),
                },
                join,
            );
            OBJECT_LENGTH
        } else {
            let fresh = changed || kept.shown == Shown::Seq;
            let mark = PowerMark {
                end: false,
                kind,
                value: match kept.shown {
                    Shown::Seq => original.value & power_flag::MAIN_TEXT,
                    _ => original.value,
                },
                code,
            };
            let result = has_result(kind) && kept.shown == Shown::Result;
            let piece = if result {
                Piece {
                    text: &kept.text,
                    runs: kept.runs.clone(),
                    objects: kept.objects.clone(),
                    marks: kept.marks.clone(),
                }
            } else {
                Piece::default()
            };
            let (inner, length_in) = (piece.marks.len(), piece.text.len());
            let end_kept = has_result(kind);
            splice(
                paragraph,
                code_from..code_to,
                start_mark + 1..end_mark + usize::from(!end_kept),
                piece,
                join,
            );
            if end_kept {
                let end_mark = &mut paragraph.marks[start_mark + 1 + inner];
                end_mark.raw = if fresh {
                    PowerMark {
                        end: true,
                        ..mark.clone()
                    }
                    .raw()
                } else {
                    plain_raw(&end_mark.raw)
                };
            }
            if fresh {
                paragraph.marks[start_mark].raw = mark.raw();
            }
            length_in
        };
        report.changed.push(paragraph.id);
        report.flows.insert(flow);
        swaps.push(Swap {
            flow,
            paragraph: span.start.paragraph,
            from: code_from,
            to: code_to,
            length,
        });
    }
    // Marks whose field went away keep no id.
    let Some(flow_mut) = next.flow_mut(flow) else {
        return;
    };
    for index in 0..flow_mut.paragraphs.len() {
        if flow_mut.paragraphs[index]
            .marks
            .iter()
            .any(|mark| mark.raw.contains('\0'))
        {
            let paragraph = flow_mut.paragraph_mut(index).expect("the paragraph exists");
            for mark in &mut paragraph.marks {
                mark.raw = plain_raw(&mark.raw);
            }
            report.changed.push(paragraph.id);
            report.flows.insert(flow);
        }
    }
}

/// Text typed beside a code took its red; outside the field it takes the
/// format the code was shown in again.
fn restore_format(
    paragraph: &mut Paragraph,
    range: std::ops::Range<usize>,
    red: &CharacterFormat,
    format: &CharacterFormat,
) {
    if range.is_empty() {
        return;
    }
    let mut runs = Vec::with_capacity(paragraph.runs.len() + 2);
    for run in paragraph.runs.drain(..) {
        if run.format != *red || run.end <= range.start || range.end <= run.start {
            runs.push(run);
            continue;
        }
        let (inside_start, inside_end) = (run.start.max(range.start), run.end.min(range.end));
        if run.start < inside_start {
            runs.push(CharacterRun {
                end: inside_start,
                ..run.clone()
            });
        }
        if !format.is_empty() {
            runs.push(CharacterRun {
                start: inside_start,
                end: inside_end,
                format: format.clone(),
            });
        }
        if inside_end < run.end {
            runs.push(CharacterRun {
                start: inside_end,
                ..run
            });
        }
    }
    paragraph.runs = runs;
}

/// A mark's text without the id it carries while codes show.
fn plain_raw(raw: &str) -> String {
    raw.split('\0').next().unwrap_or(raw).to_owned()
}

const OBJECT_LENGTH: usize = amipro_document::OBJECT_REPLACEMENT_CHARACTER.len_utf8();

fn objects_between(paragraph: &Paragraph, from: usize, to: usize) -> Vec<InlineObject> {
    paragraph
        .object_offsets()
        .zip(&paragraph.objects)
        .filter(|(offset, _)| (from..to).contains(offset))
        .map(|(_, object)| object.clone())
        .collect()
}

/// The format a code shown in place of bytes `from..to` takes before its
/// red: the result's first character's (or the text's before an empty one).
fn base_format(paragraph: &Paragraph, from: usize, to: usize) -> CharacterFormat {
    paragraph
        .runs
        .iter()
        .find(|run| run.start <= from && from < run.end)
        .or_else(|| {
            (from == to)
                .then(|| {
                    paragraph
                        .runs
                        .iter()
                        .find(|run| run.start < from && run.end == from)
                })
                .flatten()
        })
        .map(|run| run.format.clone())
        .unwrap_or_default()
}

/// Text put into a paragraph by [`splice`], with its runs, objects, and
/// marks (bytes from its start).
#[derive(Default)]
struct Piece<'a> {
    text: &'a str,
    runs: Vec<CharacterRun>,
    objects: Vec<InlineObject>,
    marks: Vec<amipro_document::LegacyMark>,
}

/// Puts `piece` in place of `bytes` of `paragraph` and of its marks
/// `marks_out`; the marks before stay, those after move with the text after.
/// At each edge marked in `join`, runs of one format become one again.
fn splice(
    paragraph: &mut Paragraph,
    bytes: std::ops::Range<usize>,
    marks_out: std::ops::Range<usize>,
    piece: Piece,
    join: [bool; 2],
) {
    let (from, to) = (bytes.start, bytes.end);
    let length = piece.text.len();
    let mut runs = crate::runs_for_slice(&paragraph.runs, 0, from, 0);
    runs.extend(piece.runs.into_iter().map(|run| CharacterRun {
        start: run.start + from,
        end: run.end + from,
        ..run
    }));
    runs.extend(crate::runs_for_slice(
        &paragraph.runs,
        to,
        paragraph.text.len(),
        from + length,
    ));
    for (edge, join) in [from, from + length].into_iter().zip(join) {
        if !join {
            continue;
        }
        if let Some(index) = runs.iter().position(|run| run.end == edge)
            && runs
                .get(index + 1)
                .is_some_and(|after| after.start == edge && after.format == runs[index].format)
        {
            runs[index].end = runs[index + 1].end;
            runs.remove(index + 1);
        }
    }
    let character = amipro_document::OBJECT_REPLACEMENT_CHARACTER;
    let before = paragraph.text[..from].matches(character).count();
    let inside = paragraph.text[from..to].matches(character).count();
    paragraph
        .objects
        .splice(before..before + inside, piece.objects);
    let mut marks = paragraph.marks[..marks_out.start].to_vec();
    marks.extend(
        piece
            .marks
            .into_iter()
            .map(|mark| amipro_document::LegacyMark {
                byte: mark.byte + from,
                ..mark
            }),
    );
    marks.extend(
        paragraph.marks[marks_out.end..]
            .iter()
            .map(|mark| amipro_document::LegacyMark {
                byte: mark.byte.max(to) - to + from + length,
                raw: mark.raw.clone(),
            }),
    );
    paragraph.text.replace_range(from..to, piece.text);
    paragraph.runs = runs;
    paragraph.marks = marks;
}

/// The document with its codes hidden while they show, to work a field
/// command out on, as Ami Pro reads edited codes back before it updates
/// or inserts a field (`docs/research/FIELDS_NOTES.md`, "Commands").
pub struct CodesHidden {
    pub document: Document,
    swaps: CodeSwaps,
}

impl CodesHidden {
    /// A position of the document with codes in the hidden one.
    pub fn map(&self, at: TextPosition) -> TextPosition {
        self.swaps.map(at)
    }

    /// One step: the codes hidden, `work` (worked out on the hidden
    /// document), and the codes shown again unless `then_show` is false
    /// (Update All switches them off). None when nothing would change.
    pub fn step(
        &self,
        document: &Document,
        work: Vec<Command>,
        then_show: bool,
    ) -> Option<Command> {
        let mut commands = vec![Command::HidePowerFields];
        commands.extend(work);
        if then_show {
            commands.push(Command::ShowPowerFields);
        }
        let command = Command::Several(commands);
        let after = crate::apply(document, &command).ok()?;
        (after.document != *document).then_some(command)
    }
}

/// The document with codes hidden, while codes show.
pub fn codes_hidden(document: &Document) -> Option<CodesHidden> {
    document.shown_codes()?;
    let (applied, swaps) = swap_codes(document, false);
    Some(CodesHidden {
        document: applied.document,
        swaps,
    })
}

// ---------------------------------------------------------------- commands

/// `Command::InsertPowerField`: the result typed at `at` between the field's
/// marks, or its mark alone for a kind without one.
pub(crate) fn apply_insert_power_field(
    document: &Document,
    at: TextPosition,
    mark: &PowerMark,
    result: &str,
) -> Result<Applied, EditError> {
    if mark.kind == power_kind::SEQ || mark.end {
        return Err("not the start of a field kept as marks");
    }
    // End marks of fields that close where the new one goes stay before
    // it: a field put in right after another follows it rather than going
    // inside it (two merge fields clicked in turn).
    let closing = document
        .flow(at.flow)
        .and_then(|flow| flow.paragraph(at.paragraph))
        .map_or(0, |paragraph| {
            paragraph
                .marks
                .iter()
                .skip_while(|kept| kept.byte < at.byte)
                .take_while(|kept| {
                    kept.byte == at.byte && PowerMark::parse(&kept.raw).is_some_and(|mark| mark.end)
                })
                .count()
        });
    let mut applied = replace_range(document.clone(), at, at, result, Vec::new(), None)?;
    let paragraph = applied
        .document
        .flow_mut(at.flow)
        .and_then(|flow| flow.paragraph_mut(at.paragraph))
        .ok_or("paragraph out of bounds")?;
    let end_byte = at.byte + result.len();
    // (`replace_range` kept those end marks before the result.)
    // The marks that were at `at` moved after the result; the field's start
    // goes before it and its end right after it, before them.
    let closed = paragraph
        .marks
        .iter()
        .position(|kept| kept.byte >= at.byte)
        .map_or(paragraph.marks.len(), |place| place + closing);
    let first_after = paragraph
        .marks
        .iter()
        .position(|kept| kept.byte >= end_byte)
        .unwrap_or(paragraph.marks.len())
        .max(closed);
    let mut added = vec![amipro_document::LegacyMark {
        byte: at.byte,
        raw: mark.raw(),
    }];
    if has_result(mark.kind) {
        added.push(amipro_document::LegacyMark {
            byte: end_byte,
            raw: PowerMark {
                end: true,
                ..mark.clone()
            }
            .raw(),
        });
    }
    let start_place = closed.min(first_after);
    if added.len() == 2 {
        paragraph
            .marks
            .insert(first_after, added.pop().expect("two marks"));
    }
    paragraph
        .marks
        .insert(start_place, added.pop().expect("the start mark"));
    applied.caret = Some(TextPosition {
        byte: end_byte,
        ..at
    });
    Ok(applied)
}

/// What Backspace (`forward` false) or Delete at the caret `at` takes when
/// a power field is beside it: empty fields at `at` (they show nothing)
/// first, else the whole field ending (Delete: starting) there, the
/// outermost; `None` when no field is beside it.
pub fn field_beside(
    document: &Document,
    at: TextPosition,
    forward: bool,
) -> Option<(TextPosition, TextPosition)> {
    let flow = document.flow(at.flow)?;
    let spans = power_fields(flow.paragraphs.iter().map(AsRef::as_ref));
    let here = |place: MarkPlace| place.paragraph == at.paragraph && place.byte == at.byte;
    let whole = || {
        spans
            .iter()
            .filter_map(|span| span.end.map(|end| (span.start, end)))
            .filter(|(start, end)| start.paragraph == end.paragraph)
    };
    if whole().any(|(start, end)| here(start) && here(end)) {
        return Some((at, at));
    }
    if forward {
        whole()
            .filter(|(start, _)| here(*start))
            .max_by_key(|(_, end)| end.byte)
            .map(|(_, end)| {
                (
                    at,
                    TextPosition {
                        byte: end.byte,
                        ..at
                    },
                )
            })
    } else {
        whole()
            .filter(|(_, end)| here(*end))
            .min_by_key(|(start, _)| start.byte)
            .map(|(start, _)| {
                (
                    TextPosition {
                        byte: start.byte,
                        ..at
                    },
                    at,
                )
            })
    }
}

/// Whether a power field lies wholly between `start` and `end` (its marks
/// at the ends included), so deleting the range must take it.
pub fn holds_whole_field(document: &Document, start: TextPosition, end: TextPosition) -> bool {
    !whole_fields(document, start, end).is_empty()
}

/// The raw marks of the power fields wholly between `start` and `end`.
fn whole_fields(document: &Document, start: TextPosition, end: TextPosition) -> Vec<String> {
    let Some(flow) = document.flow(start.flow) else {
        return Vec::new();
    };
    let (from, to) = ((start.paragraph, start.byte), (end.paragraph, end.byte));
    let raw = |place: MarkPlace| {
        flow.paragraphs
            .get(place.paragraph)
            .and_then(|paragraph| paragraph.marks.get(place.mark))
            .map(|mark| mark.raw.clone())
    };
    let mut raws = Vec::new();
    for span in power_fields(flow.paragraphs.iter().map(AsRef::as_ref)) {
        let last = span.end.unwrap_or(span.start);
        if from <= (span.start.paragraph, span.start.byte) && (last.paragraph, last.byte) <= to {
            raws.extend(raw(span.start));
            if let Some(end) = span.end {
                raws.extend(raw(end));
            }
        }
    }
    raws
}

/// `Command::DeleteFields`: the range's text goes, and with it the fields
/// wholly in it (marks strictly inside go with the text; the marks at its
/// ends, then at `start`, are taken out too).
pub(crate) fn apply_delete_fields(
    document: &Document,
    start: TextPosition,
    end: TextPosition,
) -> Result<Applied, EditError> {
    let (start, end) = if (end.paragraph, end.byte) < (start.paragraph, start.byte) {
        (end, start)
    } else {
        (start, end)
    };
    let gone = whole_fields(document, start, end);
    if gone.is_empty() && start == end {
        return Err("nothing to delete");
    }
    let mut applied = replace_range(document.clone(), start, end, "", Vec::new(), None)?;
    let paragraph = applied
        .document
        .flow_mut(start.flow)
        .and_then(|flow| flow.paragraph_mut(start.paragraph))
        .ok_or("paragraph out of bounds")?;
    for raw in gone {
        if let Some(index) = paragraph
            .marks
            .iter()
            .position(|mark| mark.byte == start.byte && mark.raw == raw)
        {
            paragraph.marks.remove(index);
        }
    }
    let id = paragraph.id;
    applied.report.flows.insert(start.flow);
    if !applied.report.changed.contains(&id) && !applied.report.inserted.contains(&id) {
        applied.report.changed.push(id);
    }
    applied.caret = Some(start);
    Ok(applied)
}

/// `Command::SetFieldResult`: the text between a field's marks becomes
/// `result`, the marks staying around it (`1578:04a0`).
pub(crate) fn apply_set_field_result(
    document: &Document,
    flow: FlowId,
    start: MarkPlace,
    end: MarkPlace,
    result: &str,
) -> Result<Applied, EditError> {
    if start.paragraph != end.paragraph || start.byte > end.byte {
        return Err("a field's result is in one paragraph");
    }
    let before = document
        .flow(flow)
        .and_then(|flow| flow.paragraph(start.paragraph))
        .ok_or("paragraph out of bounds")?;
    // The marks at the start that come before the field's own end up after
    // the new text with it; they go back.
    let staying = before
        .marks
        .iter()
        .enumerate()
        .filter(|(index, kept)| kept.byte == start.byte && *index <= start.mark)
        .count();
    let prefix = before
        .marks
        .iter()
        .filter(|kept| kept.byte < start.byte)
        .count();
    let from = TextPosition {
        flow,
        paragraph: start.paragraph,
        byte: start.byte,
    };
    let to = TextPosition {
        byte: end.byte,
        ..from
    };
    let mut applied = replace_range(document.clone(), from, to, result, Vec::new(), None)?;
    let paragraph = applied
        .document
        .flow_mut(flow)
        .and_then(|flow| flow.paragraph_mut(start.paragraph))
        .ok_or("paragraph out of bounds")?;
    for kept in paragraph.marks.iter_mut().skip(prefix).take(staying) {
        kept.byte = start.byte;
    }
    applied.caret = None;
    Ok(applied)
}

/// `Command::SetFieldMarks`: a field's marks rewritten for `mark` (its
/// code, kind, and flags).
pub(crate) fn apply_set_field_marks(
    document: &Document,
    flow: FlowId,
    start: MarkPlace,
    end: Option<MarkPlace>,
    mark: &PowerMark,
) -> Result<Applied, EditError> {
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for (place, end_mark) in std::iter::once((start, false)).chain(end.map(|end| (end, true))) {
        let paragraph = next
            .flow_mut(flow)
            .and_then(|flow| flow.paragraph_mut(place.paragraph))
            .ok_or("paragraph out of bounds")?;
        let kept = paragraph
            .marks
            .get_mut(place.mark)
            .filter(|kept| kept.byte == place.byte && PowerMark::parse(&kept.raw).is_some())
            .ok_or("there is no field mark there")?;
        // A mark keeps the id it carries while codes show.
        let suffix = kept.raw.find('\0').map(|at| kept.raw[at..].to_owned());
        kept.raw = PowerMark {
            end: end_mark,
            ..mark.clone()
        }
        .raw()
            + suffix.as_deref().unwrap_or("");
        report.changed.push(paragraph.id);
    }
    report.flows.insert(flow);
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// `Command::SetFieldObject`: a date, Doc Info entry, or Seq set in place.
pub(crate) fn apply_set_field_object(
    document: &Document,
    at: TextPosition,
    object: &InlineObject,
) -> Result<Applied, EditError> {
    if !is_field_object(object) {
        return Err("not a field");
    }
    let paragraph = document
        .flow(at.flow)
        .and_then(|flow| flow.paragraph(at.paragraph))
        .ok_or("paragraph out of bounds")?;
    let index = paragraph
        .object_offsets()
        .position(|offset| offset == at.byte)
        .filter(|index| is_field_object(&paragraph.objects[*index]))
        .ok_or("there is no field there")?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let paragraph = next
        .flow_mut(at.flow)
        .and_then(|flow| flow.paragraph_mut(at.paragraph))
        .ok_or("paragraph out of bounds")?;
    if paragraph.objects[index] != *object {
        paragraph.objects[index] = object.clone();
        report.changed.push(paragraph.id);
        report.flows.insert(at.flow);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// The value a Seq field of `name` continues from when no earlier one of
/// its name stands in its stream: a master document's member takes the
/// highest value of the members before, from its `[sequence]`, in the main
/// text (`1640:03c3`, `1660:09df`; `docs/research/MASTER_NOTES.md` §4).
pub fn carried_seq(document: &Document, flow: FlowId, name: &str) -> u32 {
    if flow != document.main_flow_id() {
        return 0;
    }
    document
        .contents()
        .sequence
        .as_ref()
        .and_then(|sequence| sequence.seq_value(name))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply;
    use crate::field_language::NoHost;
    use amipro_document::{DateField, DateSource, WallClock};

    fn context() -> FieldContext {
        FieldContext {
            now: WallClock {
                year: 2026,
                month: 10,
                day: 20,
                hour: 11,
                minute: 34,
                second: 0,
            },
            pages: 3,
            ..FieldContext::default()
        }
    }

    fn insert(document: &Document, paragraph: usize, byte: usize, code: &str) -> Document {
        let context = context();
        let mut variables = BTreeMap::new();
        let mut host = NoHost::default();
        let mut inputs = FieldInputs {
            context: &context,
            variables: &mut variables,
            host: &mut host,
        };
        let at = TextPosition {
            flow: document.main_flow_id(),
            paragraph,
            byte,
        };
        let command = insert_power_field(document, at, code, 0, &mut inputs).unwrap();
        let next = apply(document, &command).unwrap().document;
        next.validate().unwrap();
        next
    }

    fn marks(document: &Document, paragraph: usize) -> Vec<(usize, String)> {
        document.paragraphs()[paragraph]
            .marks
            .iter()
            .map(|mark| (mark.byte, mark.raw.clone()))
            .collect()
    }

    fn update(document: &Document, scope: UpdateScope, pages: u32) -> (Document, FieldUpdate) {
        let context = FieldContext { pages, ..context() };
        let mut variables = BTreeMap::new();
        let mut host = NoHost::default();
        let mut inputs = FieldInputs {
            context: &context,
            variables: &mut variables,
            host: &mut host,
        };
        let found = update_fields(document, scope, &mut inputs);
        let next = match &found.command {
            Some(command) => apply(document, command).unwrap().document,
            None => document.clone(),
        };
        next.validate().unwrap();
        (next, found)
    }

    #[test]
    fn inserted_fields_are_worked_out_at_once_between_their_marks_fd3() {
        let document = Document::from_plain_text("Pages: .");
        let document = insert(&document, 0, 7, "NumPages");
        assert_eq!(document.paragraphs()[0].text, "Pages: 3.");
        assert_eq!(
            marks(&document, 0),
            [
                (7, "<:X3,-32768;NumPages>".into()),
                (8, "<:X~3,-32768;NumPages>".into())
            ]
        );
        // A field without a result is its mark; Seq is an object numbered
        // after the one before it.
        let document = insert(&document, 0, 0, "Set total 3");
        assert_eq!(
            marks(&document, 0)[0],
            (0, "<:X5,-32768;Set total 3>".into())
        );
        let document = insert(&document, 0, 0, "Seq Figure");
        let document = insert(
            &document,
            0,
            document.paragraphs()[0].text.len(),
            "Seq Figure %N2",
        );
        let seqs: Vec<String> = document.paragraphs()[0]
            .objects
            .iter()
            .filter_map(|object| match object {
                InlineObject::Seq(field) => Some(field.text()),
                _ => None,
            })
            .collect();
        assert_eq!(seqs, ["1", "II"]);
    }

    #[test]
    fn update_puts_the_new_result_between_the_marks_and_keeps_its_bold_fd3b() {
        let document = insert(&Document::from_plain_text("Pages: ."), 0, 7, "NumPages");
        // Typed into (FD3: "OctobeXr"), then updated with twelve pages. Text
        // typed at the result's end is after the field, as Ami Pro's caret
        // stands after a field it puts in (MC11); at its start, before it.
        let typed = apply(
            &document,
            &Command::insert(
                TextPosition {
                    flow: document.main_flow_id(),
                    paragraph: 0,
                    byte: 8,
                },
                "X",
            ),
        )
        .unwrap()
        .document;
        assert_eq!(typed.paragraphs()[0].text, "Pages: 3X.");
        assert_eq!(marks(&typed, 0)[1].0, 8);
        let caret = TextPosition {
            flow: typed.main_flow_id(),
            paragraph: 0,
            byte: 8,
        };
        let (updated, found) = update(&typed, UpdateScope::At(caret), 12);
        assert_eq!(found.updated, 1);
        assert_eq!(updated.paragraphs()[0].text, "Pages: 12X.");
        // It read NumPages: updated again after pagination (0x4000; AFD1).
        assert_eq!(
            marks(&updated, 0),
            [
                (7, "<:X3,-16384;NumPages>".into()),
                (9, "<:X~3,-16384;NumPages>".into())
            ]
        );
        // An up-to-date field makes no change; a locked one is left.
        assert!(update(&updated, UpdateScope::All, 12).1.command.is_none());
        let start = power_fields(updated.paragraphs().iter().map(AsRef::as_ref))[0].clone();
        let locked = apply(
            &updated,
            &Command::SetFieldMarks {
                flow: updated.main_flow_id(),
                start: start.start,
                end: start.end,
                mark: PowerMark {
                    value: start.mark.value | power_flag::LOCK,
                    ..start.mark.clone()
                },
            },
        )
        .unwrap()
        .document;
        assert!(update(&locked, UpdateScope::All, 40).1.command.is_none());
    }

    #[test]
    fn update_all_works_out_fields_in_order_with_their_variables_fd3() {
        let mut document = Document::from_plain_text("a\nb\nc");
        document = insert(&document, 0, 1, "Set total 3");
        document = insert(&document, 1, 1, "total * 2 + 1");
        document = insert(&document, 2, 1, "Seq A");
        document = insert(&document, 2, 1, "Seq A");
        // A new value for total and a renumbering, all in one step.
        let start = power_fields(document.paragraphs().iter().map(AsRef::as_ref))[0].clone();
        document = apply(
            &document,
            &Command::SetFieldMarks {
                flow: document.main_flow_id(),
                start: start.start,
                end: start.end,
                mark: PowerMark {
                    code: "Set total 5".into(),
                    ..start.mark
                },
            },
        )
        .unwrap()
        .document;
        let (updated, found) = update(&document, UpdateScope::All, 1);
        assert!(matches!(found.command, Some(Command::Several(_))));
        assert_eq!(updated.paragraphs()[1].text, "b11");
        let numbers: Vec<u32> = updated.paragraphs()[2]
            .objects
            .iter()
            .filter_map(|object| match object {
                InlineObject::Seq(field) => Some(field.number()),
                _ => None,
            })
            .collect();
        assert_eq!(numbers, [1, 2]);
    }

    fn show(document: &Document) -> Document {
        let shown = apply(document, &Command::ShowPowerFields).unwrap().document;
        shown.validate().unwrap();
        shown
    }

    fn hide(document: &Document) -> Document {
        let hidden = apply(document, &Command::HidePowerFields).unwrap().document;
        hidden.validate().unwrap();
        hidden
    }

    fn at(document: &Document, byte: usize) -> TextPosition {
        TextPosition {
            flow: document.main_flow_id(),
            paragraph: 0,
            byte,
        }
    }

    #[test]
    fn show_power_fields_shows_each_code_in_red_fd3() {
        let mut document = insert(&Document::from_plain_text("Pages: . A"), 0, 7, "NumPages");
        document = insert(&document, 0, 10, "Seq Figure");
        document = insert(&document, 0, 0, "Set total 3");
        let shown = show(&document);
        assert!(shown.shown_codes().is_some());
        let paragraph = &shown.paragraphs()[0];
        assert_eq!(
            paragraph.text,
            "<Set total 3>Pages: <NumPages>. <Seq Figure>A"
        );
        let red: Vec<&str> = paragraph
            .runs
            .iter()
            .filter(|run| run.format.colour == Some(0x0000FF))
            .map(|run| &paragraph.text[run.start..run.end])
            .collect();
        assert_eq!(red, ["<Set total 3>", "<NumPages>", "<Seq Figure>"]);
        // Each code is a field between its marks, so the panel and Next
        // Field find it.
        let found: Vec<(usize, usize)> = all_fields(&shown)
            .iter()
            .map(|field| (field.range().0.byte, field.range().1.byte))
            .collect();
        assert_eq!(found, [(0, 13), (20, 30), (32, 44)]);
        // Off again, the document is as it was.
        assert_eq!(hide(&shown), document);
    }

    #[test]
    fn codes_set_aside_formatting_objects_and_nested_fields_and_give_them_back() {
        let mut document = insert(&Document::from_plain_text("a  b"), 0, 2, "NumPages");
        // Bold over the result's first character and the text before it.
        let main = document.main_flow_id();
        let bold = CharacterFormat {
            bold: Some(true),
            ..CharacterFormat::default()
        };
        document = apply(
            &document,
            &Command::FormatCharacters {
                start: at(&document, 0),
                end: at(&document, 3),
                format: bold.clone(),
            },
        )
        .unwrap()
        .document;
        // A field nested in the result and a date object in it.
        let paragraph = document.paragraph_mut(0);
        paragraph.text = "a 3\u{FFFC}x b".into();
        paragraph.objects = vec![InlineObject::Date(DateField {
            source: DateSource::Today,
            format: 1,
        })];
        let end = paragraph.marks.pop().unwrap();
        paragraph.marks.push(amipro_document::LegacyMark {
            byte: 6,
            raw: PowerMark {
                end: false,
                kind: power_kind::GENERAL,
                value: power_flag::MAIN_TEXT,
                code: "Set x 1".into(),
            }
            .raw(),
        });
        paragraph
            .marks
            .push(amipro_document::LegacyMark { byte: 7, ..end });
        document.validate().unwrap();
        let shown = show(&document);
        let paragraph = &shown.paragraphs()[0];
        assert_eq!(paragraph.text, "a <NumPages> b");
        assert!(paragraph.objects.is_empty());
        assert_eq!(paragraph.marks.len(), 2);
        // The code is bold as the result's first character was, and red.
        let code = paragraph
            .runs
            .iter()
            .find(|run| run.start == 2)
            .expect("the code's run");
        assert_eq!(code.end, 12);
        assert_eq!(code.format.bold, Some(true));
        assert_eq!(code.format.colour, Some(0x0000FF));
        assert_eq!(hide(&shown), document);
        assert_eq!(main, shown.main_flow_id());
    }

    #[test]
    fn an_edited_code_is_read_back_and_text_outside_its_brackets_stays_outside() {
        let document = insert(&Document::from_plain_text("[] end"), 0, 1, "NumPages");
        let shown = show(&document);
        assert_eq!(shown.paragraphs()[0].text, "[<NumPages>] end");
        // Typed: the code changed, and text after its `>`.
        let edited = apply(
            &shown,
            &Command::Several(vec![
                Command::ReplaceText {
                    start: at(&shown, 11),
                    end: at(&shown, 11),
                    text: "!".into(),
                },
                Command::ReplaceText {
                    start: at(&shown, 5),
                    end: at(&shown, 10),
                    text: "Words".into(),
                },
            ]),
        )
        .unwrap()
        .document;
        assert_eq!(edited.paragraphs()[0].text, "[<NumWords>!] end");
        let hidden = hide(&edited);
        assert!(hidden.shown_codes().is_none());
        // The old result stays until an update; the text typed after the
        // `>` is after the field.
        assert_eq!(hidden.paragraphs()[0].text, "[3!] end");
        let fields = power_fields(hidden.paragraphs().iter().map(AsRef::as_ref));
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].mark.code, "NumWords");
        assert_eq!(fields[0].end.map(|end| end.byte), Some(2));
        assert!(marks(&hidden, 0).iter().all(|(_, raw)| !raw.contains('\0')));
        // The `!` typed after the code took its red; outside the field it
        // is as the code was shown, before its red.
        assert!(hidden.paragraphs()[0].runs.is_empty());
    }

    #[test]
    fn runs_as_they_were_come_back_split_or_whole() {
        let mut document = insert(&Document::from_plain_text("ab . c"), 0, 3, "NumPages");
        document = insert(&document, 0, 6, "Seq Figure");
        let italic = CharacterFormat {
            italic: Some(true),
            ..CharacterFormat::default()
        };
        // Two runs of one format meeting at the first field's start (as
        // read from a file), and one run over the Seq and the text around.
        let paragraph = document.paragraph_mut(0);
        assert_eq!(paragraph.text, "ab 3. \u{FFFC}c");
        paragraph.runs = vec![
            CharacterRun {
                start: 0,
                end: 3,
                format: italic.clone(),
            },
            CharacterRun {
                start: 3,
                end: 4,
                format: italic.clone(),
            },
            CharacterRun {
                start: 5,
                end: 10,
                format: italic,
            },
        ];
        document.validate().unwrap();
        let shown = show(&document);
        assert_eq!(shown.paragraphs()[0].text, "ab <NumPages>. <Seq Figure>c");
        assert_eq!(hide(&shown), document);
    }

    #[test]
    fn one_run_over_fields_side_by_side_comes_back_whole() {
        let mut document = insert(&Document::from_plain_text("x"), 0, 0, "NumPages");
        document = insert(&document, 0, 0, "NumPages");
        let paragraph = document.paragraph_mut(0);
        assert_eq!(paragraph.text, "33x");
        paragraph.runs = vec![CharacterRun {
            start: 0,
            end: 3,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        }];
        let shown = show(&document);
        assert_eq!(shown.paragraphs()[0].text, "<NumPages><NumPages>x");
        assert_eq!(hide(&shown), document);
    }

    #[test]
    fn an_emptied_code_deletes_its_field_and_a_seq_stays_a_seq() {
        let mut document = insert(&Document::from_plain_text("x y"), 0, 1, "NumPages");
        document = insert(&document, 0, 3, "Seq Table");
        let shown = show(&document);
        assert_eq!(shown.paragraphs()[0].text, "x<NumPages> <Seq Table>y");
        let edited = apply(
            &shown,
            &Command::Several(vec![
                Command::ReplaceText {
                    start: at(&shown, 17),
                    end: at(&shown, 22),
                    text: "Figure".into(),
                },
                Command::ReplaceText {
                    start: at(&shown, 1),
                    end: at(&shown, 11),
                    text: String::new(),
                },
            ]),
        )
        .unwrap()
        .document;
        let hidden = hide(&edited);
        let paragraph = &hidden.paragraphs()[0];
        assert_eq!(paragraph.text, "x \u{FFFC}y");
        assert!(paragraph.marks.is_empty());
        assert!(matches!(
            &paragraph.objects[..],
            [InlineObject::Seq(SeqField { code, .. })] if code == "Seq Figure"
        ));
    }

    #[test]
    fn updates_and_insertions_with_codes_shown_read_the_codes_back_first() {
        let document = insert(&Document::from_plain_text("Pages: ."), 0, 7, "NumPages");
        let shown = show(&document);
        // The code edited to a sum, then Update at the caret in it.
        let edited = apply(
            &shown,
            &Command::ReplaceText {
                start: at(&shown, 8),
                end: at(&shown, 16),
                text: "2+5".into(),
            },
        )
        .unwrap()
        .document;
        assert_eq!(edited.paragraphs()[0].text, "Pages: <2+5>.");
        let hidden = codes_hidden(&edited).expect("codes show");
        let scope = UpdateScope::At(hidden.map(at(&edited, 9)));
        let (_, found) = update(&hidden.document, scope, 3);
        let context = context();
        let mut variables = BTreeMap::new();
        let mut host = NoHost::default();
        let mut inputs = FieldInputs {
            context: &context,
            variables: &mut variables,
            host: &mut host,
        };
        let work = update_fields(&hidden.document, scope, &mut inputs);
        assert_eq!(found.updated, 1);
        let step = hidden
            .step(&edited, work.command.into_iter().collect(), true)
            .expect("a change");
        let after = apply(&edited, &step).unwrap().document;
        // Still showing its code, with the new result set aside.
        assert_eq!(after.paragraphs()[0].text, "Pages: <2+5>.");
        assert_eq!(hide(&after).paragraphs()[0].text, "Pages: 7.");
        // Nothing changed: no step.
        let again = codes_hidden(&shown).unwrap();
        assert!(again.step(&shown, Vec::new(), true).is_none());
        // An insertion with codes shown shows the new field's code, and
        // the caret goes after it.
        let at_end = hidden.map(at(&edited, 13));
        let command = {
            let mut variables = BTreeMap::new();
            let mut inputs = FieldInputs {
                context: &context,
                variables: &mut variables,
                host: &mut host,
            };
            insert_power_field(&hidden.document, at_end, "NumPages", 0, &mut inputs).unwrap()
        };
        let step = hidden.step(&edited, vec![command], true).unwrap();
        let applied = apply(&edited, &step).unwrap();
        assert_eq!(
            applied.document.paragraphs()[0].text,
            "Pages: <2+5>.<NumPages>"
        );
        assert_eq!(applied.caret.map(|caret| caret.byte), Some(23));
    }

    #[test]
    fn fields_are_found_at_the_caret_and_one_after_another() {
        let mut document = Document::from_plain_text("Date: x Pages: .");
        let paragraph = document.paragraph_mut(0);
        paragraph.text = "Date: \u{FFFC} Pages: .".into();
        paragraph.objects.push(InlineObject::Date(DateField {
            source: DateSource::Today,
            format: b'b',
        }));
        let document = insert(&document, 0, 17, "NumPages");
        let flow = document.main_flow_id();
        let at = |byte| TextPosition {
            flow,
            paragraph: 0,
            byte,
        };
        assert!(matches!(
            field_at(&document, at(6)),
            Some(FieldAt::Object { .. })
        ));
        assert!(matches!(
            field_at(&document, at(18)),
            Some(FieldAt::Power { .. })
        ));
        assert_eq!(field_at(&document, at(1)), None);
        let next = next_field(&document, at(0), true).unwrap();
        assert_eq!(next.range(), (at(6), at(9)));
        let after = next_field(&document, at(6), true).unwrap();
        assert_eq!(after.range(), (at(17), at(18)));
        // Round to the first, and back.
        assert_eq!(
            next_field(&document, at(17), true).unwrap().range().0,
            at(6)
        );
        assert_eq!(
            next_field(&document, at(6), false).unwrap().range().0,
            at(17)
        );
    }
}
