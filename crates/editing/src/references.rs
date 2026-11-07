//! Notes, bookmarks, footnotes, and Footnote Options as commands (the notes
//! run; `docs/Design/UI_SPEC.md` §34, NR1–NR10;
//! `docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`). Each is one undo step,
//! where Ami Pro's Remove All Notes and its bookmarks undo nothing.

use std::sync::Arc;

use amipro_document::bookmarks::{
    self, BookmarkMark, BookmarkTarget, CellBookmark, NameProblem, bookmark_named,
};
use amipro_document::{
    CellContent, Document, FlowId, FootnoteOptions, FrameContent, FrameId, InlineObject, NoteInfo,
    OBJECT_REPLACEMENT_CHARACTER, Paragraph, ParagraphStyle, StyleId,
};

use crate::{Applied, ChangeReport, EditError, TextPosition, replace_range};

/// Where a new bookmark goes (Edit › Bookmarks › Add).
#[derive(Clone, Debug, PartialEq)]
pub enum NewBookmark {
    /// The text from `start` to `end` in one flow, or a place when they
    /// are the same: `<:Zname>` … `<:Z~name>`.
    Text {
        start: TextPosition,
        end: TextPosition,
    },
    /// A block of cells from (`row`, `column`): `<:Zrows,columns;name>` in
    /// its top-left cell.
    Cells {
        frame: FrameId,
        row: usize,
        column: usize,
        rows: u16,
        columns: u16,
    },
    /// One cell: a `[book]` line naming the table by its sequence number.
    Cell {
        frame: FrameId,
        row: usize,
        column: usize,
    },
    /// A frame: its name.
    Frame(FrameId),
}

/// Ami Pro's refusal of a bookmark inside another (string 3161).
pub const NESTED: &str = "Bookmarks cannot be placed inside one another.";

pub(crate) fn name_error(problem: NameProblem) -> EditError {
    problem.message()
}

/// The note at `at` (its U+FFFC), with its owned flow.
pub fn note_at(document: &Document, at: TextPosition) -> Option<(NoteInfo, FlowId)> {
    let paragraph = document.flow(at.flow)?.paragraph(at.paragraph)?;
    paragraph
        .object_offsets()
        .zip(&paragraph.objects)
        .find_map(|(byte, object)| match object {
            InlineObject::Note { note, flow } if byte == at.byte => Some((note.clone(), *flow)),
            _ => None,
        })
}

/// Every note, in the order of the text streams' flows (the main text
/// first, then by flow), with its place.
pub fn notes(document: &Document) -> Vec<(TextPosition, NoteInfo, FlowId)> {
    let main = document.main_flow_id();
    let mut found = Vec::new();
    let mut flows: Vec<FlowId> = document.flows().map(|(id, _)| id).collect();
    flows.sort_by_key(|id| (*id != main, *id));
    for id in flows {
        let Some(flow) = document.flow(id) else {
            continue;
        };
        for (index, paragraph) in flow.paragraphs.iter().enumerate() {
            for (byte, object) in paragraph.object_offsets().zip(&paragraph.objects) {
                if let InlineObject::Note { note, flow } = object {
                    found.push((
                        TextPosition {
                            flow: id,
                            paragraph: index,
                            byte,
                        },
                        note.clone(),
                        *flow,
                    ));
                }
            }
        }
    }
    found
}

/// A note's text as plain lines, one per paragraph (Ami Pro's note window).
pub fn note_text(document: &Document, flow: FlowId) -> String {
    document.flow(flow).map_or_else(String::new, |flow| {
        flow.paragraphs
            .iter()
            .map(|paragraph| {
                paragraph
                    .text
                    .replace(OBJECT_REPLACEMENT_CHARACTER, "")
                    .replace('\u{2028}', "\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// The flows whose notes Ami Pro numbers 0: headers, footers, floating
/// headers and footers, and footnotes.
fn zero_numbered(document: &Document) -> Vec<FlowId> {
    let mut flows = Vec::new();
    for layout in document.layouts() {
        for frame in [
            layout.header.right,
            layout.header.left,
            layout.footer.right,
            layout.footer.left,
        ]
        .into_iter()
        .flatten()
        {
            if let Some(FrameContent::Text(flow)) =
                document.frame(frame).map(|frame| &frame.content)
            {
                flows.push(*flow);
            }
        }
    }
    for paragraph in document.paragraphs() {
        for object in &paragraph.objects {
            if let InlineObject::FloatingHeaderFooter { flow, .. }
            | InlineObject::Footnote { flow } = object
            {
                flows.push(*flow);
            }
        }
    }
    flows
}

/// Numbers the notes as Ami Pro does on every insertion and removal: the
/// main text's 1, 2, 3, each other text's on its own, headers, footers, and
/// footnotes 0 (G ch. 24; corpus). Notes in Ami Pro's short form keep none.
pub(crate) fn renumber(document: &mut Document) {
    let zero = zero_numbered(document);
    let ids: Vec<FlowId> = document.flows().map(|(id, _)| id).collect();
    for id in ids {
        let Some(flow) = document.flow(id) else {
            continue;
        };
        let mut count = 0u16;
        let mut changes = Vec::new();
        for (index, paragraph) in flow.paragraphs.iter().enumerate() {
            let mut objects = paragraph.objects.clone();
            let mut changed = false;
            for object in &mut objects {
                if let InlineObject::Note { note, .. } = object
                    && note.number.is_some()
                {
                    count += 1;
                    let number = if zero.contains(&id) { 0 } else { count };
                    if note.number != Some(number) {
                        note.number = Some(number);
                        changed = true;
                    }
                }
            }
            if changed {
                changes.push((index, objects));
            }
        }
        if let Some(flow) = document.flow_mut(id) {
            for (index, objects) in changes {
                let mut paragraph = (*flow.paragraphs[index]).clone();
                paragraph.objects = objects;
                flow.paragraphs[index] = Arc::new(paragraph);
            }
        }
    }
}

/// Plain-text paragraphs in `style`, one per line.
fn plain_paragraphs(document: &mut Document, text: &str, style: StyleId) -> Vec<Arc<Paragraph>> {
    text.replace("\r\n", "\n")
        .split('\n')
        .map(|line| {
            let clean: String = line
                .chars()
                .map(|character| match character {
                    OBJECT_REPLACEMENT_CHARACTER => amipro_document::REPLACEMENT_CHARACTER,
                    '\t' => '\t',
                    other if other.is_control() => ' ',
                    other => other,
                })
                .collect();
            Arc::new(document.new_paragraph(clean, style))
        })
        .collect()
}

/// Edit › Insert › Note at a place in any text: the note's own fields, its
/// text as plain paragraphs in Body Text.
pub(crate) fn insert_note(
    document: &Document,
    at: TextPosition,
    note: &NoteInfo,
    text: &str,
) -> Result<Applied, EditError> {
    let mut next = document.clone();
    let body = next.body_text_style();
    let flow = next.create_flow(body);
    let paragraphs = plain_paragraphs(&mut next, text, body);
    next.flow_mut(flow).ok_or("the note's flow")?.paragraphs = paragraphs;
    let mut note = note.clone();
    note.number.get_or_insert(1);
    let object = InlineObject::Note { note, flow };
    let mut applied = replace_range(
        next,
        at,
        at,
        &OBJECT_REPLACEMENT_CHARACTER.to_string(),
        vec![object],
        None,
    )?;
    applied.report.added_flows.push(flow);
    renumber(&mut applied.document);
    Ok(applied)
}

/// Changes the note at `at`: its fields, and its text when given (as plain
/// paragraphs, as Ami Pro's note window gives it back).
pub(crate) fn set_note(
    document: &Document,
    at: TextPosition,
    note: &NoteInfo,
    text: Option<&str>,
) -> Result<Applied, EditError> {
    let (_, flow) = note_at(document, at).ok_or("no note there")?;
    let mut next = document.clone();
    let paragraph = next
        .flow_mut(at.flow)
        .and_then(|flow| flow.paragraphs.get_mut(at.paragraph))
        .ok_or("paragraph out of bounds")?;
    let mut changed = (**paragraph).clone();
    let index = changed
        .object_offsets()
        .position(|byte| byte == at.byte)
        .ok_or("no note there")?;
    changed.objects[index] = InlineObject::Note {
        note: note.clone(),
        flow,
    };
    let id = changed.id;
    *paragraph = Arc::new(changed);
    let mut report = ChangeReport::default();
    report.flows.insert(at.flow);
    report.changed.push(id);
    if let Some(text) = text {
        let body = next.body_text_style();
        let paragraphs = plain_paragraphs(&mut next, text, body);
        next.flow_mut(flow).ok_or("the note's flow")?.paragraphs = paragraphs;
        report.flows.insert(flow);
    }
    next.validate().map_err(|_| "the note would not be valid")?;
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// Removes the note at `at`, or every note, and numbers the rest.
pub(crate) fn remove_notes(
    document: &Document,
    at: Option<TextPosition>,
) -> Result<Applied, EditError> {
    let mut places: Vec<TextPosition> = match at {
        Some(at) => {
            note_at(document, at).ok_or("no note there")?;
            vec![at]
        }
        None => notes(document).into_iter().map(|(at, _, _)| at).collect(),
    };
    if places.is_empty() {
        return Err("the document has no notes");
    }
    // Last first, so earlier places stay where they are.
    places.sort_by_key(|place| std::cmp::Reverse((place.flow, place.paragraph, place.byte)));
    let mut current = document.clone();
    let mut report = ChangeReport::default();
    for place in places {
        let end = TextPosition {
            byte: place.byte + OBJECT_REPLACEMENT_CHARACTER.len_utf8(),
            ..place
        };
        let applied = replace_range(current, place, end, "", Vec::new(), None)?;
        report.flows.extend(applied.report.flows.iter().copied());
        report
            .changed
            .extend(applied.report.changed.iter().copied());
        report
            .removed_flows
            .extend(applied.report.removed_flows.iter().copied());
        current = applied.document;
    }
    renumber(&mut current);
    Ok(Applied {
        document: current,
        caret: at,
        report,
    })
}

/// Whether `place` lies strictly inside a text bookmark of its flow.
fn inside_text_bookmark(document: &Document, place: TextPosition) -> bool {
    bookmarks::bookmarks(document).iter().any(|bookmark| {
        let BookmarkTarget::Text {
            start,
            end: Some(end),
        } = &bookmark.target
        else {
            return false;
        };
        start.flow == place.flow
            && (start.paragraph, start.byte) < (place.paragraph, place.byte)
            && (place.paragraph, place.byte) < (end.paragraph, end.byte)
    })
}

/// Inserts `mark` at `place`, after the marks already at its byte.
fn insert_mark(
    document: &mut Document,
    place: TextPosition,
    mark: &BookmarkMark,
    report: &mut ChangeReport,
) -> Result<(), EditError> {
    let paragraph = document
        .flow_mut(place.flow)
        .and_then(|flow| flow.paragraphs.get_mut(place.paragraph))
        .ok_or("paragraph out of bounds")?;
    if place.byte > paragraph.text.len() || !paragraph.text.is_char_boundary(place.byte) {
        return Err("byte out of bounds");
    }
    let mut changed = (**paragraph).clone();
    let index = changed
        .marks
        .iter()
        .position(|kept| kept.byte > place.byte)
        .unwrap_or(changed.marks.len());
    changed.marks.insert(index, mark.mark(place.byte));
    report.flows.insert(place.flow);
    report.changed.push(changed.id);
    *paragraph = Arc::new(changed);
    Ok(())
}

/// Edit › Bookmarks › Add: Ami Pro's names (NR7), no bookmark inside
/// another, each kind stored as Ami Pro stores it.
pub(crate) fn add_bookmark(
    document: &Document,
    name: &str,
    target: &NewBookmark,
) -> Result<Applied, EditError> {
    let name = bookmarks::check_name(document, name).map_err(name_error)?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    match target {
        NewBookmark::Text { start, end } => {
            if start.flow != end.flow {
                return Err("a bookmark's text stays in one flow");
            }
            let (start, end) = if (start.paragraph, start.byte) <= (end.paragraph, end.byte) {
                (*start, *end)
            } else {
                (*end, *start)
            };
            if inside_text_bookmark(document, start) || inside_text_bookmark(document, end) {
                return Err(NESTED);
            }
            insert_mark(
                &mut next,
                start,
                &BookmarkMark {
                    name: name.clone(),
                    end: false,
                    cells: None,
                },
                &mut report,
            )?;
            insert_mark(
                &mut next,
                end,
                &BookmarkMark {
                    name,
                    end: true,
                    cells: None,
                },
                &mut report,
            )?;
        }
        NewBookmark::Cells {
            frame,
            row,
            column,
            rows,
            columns,
        } => {
            let flow = cell_flow(document, *frame, *row, *column)?;
            insert_mark(
                &mut next,
                TextPosition {
                    flow,
                    paragraph: 0,
                    byte: 0,
                },
                &BookmarkMark {
                    name,
                    end: false,
                    cells: Some(((*rows).max(1), (*columns).max(1))),
                },
                &mut report,
            )?;
        }
        NewBookmark::Cell { frame, row, column } => {
            cell_flow(document, *frame, *row, *column)?;
            let table = match bookmarks::frame_sequence(document, *frame) {
                Some(number) => number,
                None => {
                    // Ami Pro gives the table its number first (`10b8:071f`).
                    let highest = document
                        .frames()
                        .filter_map(|frame| bookmarks::frame_sequence(document, frame.id))
                        .max()
                        .unwrap_or(0);
                    let frame_mut = next.frame_mut(*frame).ok_or("no such frame")?;
                    frame_mut.legacy.remove("frame.field13");
                    frame_mut
                        .legacy
                        .push("frame.field13", (highest + 1).to_string());
                    report.frames.insert(*frame);
                    highest + 1
                }
            };
            let mut cells = bookmarks::cell_bookmarks(&next);
            cells.push(CellBookmark {
                name,
                table,
                row: *row as u32,
                column: *column as u32,
            });
            bookmarks::set_cell_bookmarks(&mut next, &cells);
            report.info = true;
        }
        NewBookmark::Frame(frame) => {
            let frame_mut = next.frame_mut(*frame).ok_or("no such frame")?;
            frame_mut.style.name = Some(name);
            report.frames.insert(*frame);
        }
    }
    next.validate()
        .map_err(|_| "the bookmark would not be valid")?;
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// The text flow of a table's cell.
fn cell_flow(
    document: &Document,
    frame: FrameId,
    row: usize,
    column: usize,
) -> Result<FlowId, EditError> {
    let Some(FrameContent::Table(table)) = document.frame(frame).map(|frame| &frame.content) else {
        return Err("no such table");
    };
    let cell = table
        .cells
        .get(row * table.columns.len() + column)
        .filter(|_| column < table.columns.len())
        .ok_or("no such cell")?;
    match cell.content {
        CellContent::Text(flow) => Ok(flow),
    }
}

/// Removes (`to` is `None`) or renames a bookmark of any kind, leaving its
/// text, cells, or frame as they are.
pub(crate) fn change_bookmark(
    document: &Document,
    name: &str,
    to: Option<&str>,
) -> Result<Applied, EditError> {
    let bookmark = bookmark_named(document, name).ok_or("no bookmark of that name")?;
    let to = match to {
        Some(new) if new.trim().eq_ignore_ascii_case(&bookmark.name) => new.trim().to_owned(),
        Some(new) => bookmarks::check_name(document, new).map_err(name_error)?,
        None => String::new(),
    };
    let rename = !to.is_empty();
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let lower = bookmark.name.to_lowercase();
    match &bookmark.target {
        BookmarkTarget::Text { .. } | BookmarkTarget::Cells { .. } => {
            let ids: Vec<FlowId> = next.flows().map(|(id, _)| id).collect();
            for id in ids {
                let Some(flow) = next.flow_mut(id) else {
                    continue;
                };
                for paragraph in &mut flow.paragraphs {
                    let ours = |raw: &str| {
                        BookmarkMark::parse(raw)
                            .is_some_and(|mark| mark.name.to_lowercase() == lower)
                    };
                    if !paragraph.marks.iter().any(|mark| ours(&mark.raw)) {
                        continue;
                    }
                    let mut changed = (**paragraph).clone();
                    if rename {
                        for kept in &mut changed.marks {
                            if let Some(mut mark) = BookmarkMark::parse(&kept.raw)
                                && mark.name.to_lowercase() == lower
                            {
                                mark.name.clone_from(&to);
                                kept.raw = mark.raw();
                            }
                        }
                    } else {
                        changed.marks.retain(|mark| !ours(&mark.raw));
                    }
                    report.flows.insert(id);
                    report.changed.push(changed.id);
                    *paragraph = Arc::new(changed);
                }
            }
        }
        BookmarkTarget::Cell { .. } => {
            let mut cells = bookmarks::cell_bookmarks(&next);
            if rename {
                for cell in &mut cells {
                    if cell.name.to_lowercase() == lower {
                        cell.name.clone_from(&to);
                    }
                }
            } else {
                cells.retain(|cell| cell.name.to_lowercase() != lower);
            }
            bookmarks::set_cell_bookmarks(&mut next, &cells);
            report.info = true;
        }
        BookmarkTarget::Frame(frame) => {
            let frame_mut = next.frame_mut(*frame).ok_or("no such frame")?;
            frame_mut.style.name = rename.then_some(to);
            report.frames.insert(*frame);
        }
    }
    next.validate()
        .map_err(|_| "the bookmark would not be valid")?;
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// The Footnote style: the document's, or a new one made from Body Text as
/// Ami Pro makes it on the first footnote (`1098:0643`): First indent 720
/// twips, an arabic number (`<*:>`) as superscript.
fn footnote_style(
    document: &mut Document,
    report: &mut ChangeReport,
) -> Result<StyleId, EditError> {
    if let Some(style) = document
        .styles()
        .values()
        .find(|style| style.name.eq_ignore_ascii_case("Footnote"))
    {
        return Ok(style.id);
    }
    let body = document
        .styles()
        .get(&document.body_text_style())
        .cloned()
        .ok_or("the document has no styles")?;
    let id = StyleId(
        document
            .styles()
            .keys()
            .next_back()
            .map_or(1, |id| id.0 + 1),
    );
    let mut style = ParagraphStyle {
        id,
        name: "Footnote".into(),
        ..body
    };
    style.indents.first = 720;
    style.bullets = amipro_document::Bullets {
        text: "\u{1A}".into(),
        attributes: 256,
        ..amipro_document::Bullets::default()
    };
    style.kept.function_key = crate::styles::next_function_key(document.styles());
    document.styles_mut().insert(id, style);
    report.styles = true;
    Ok(id)
}

/// Tools › Footnotes › Insert footnote in the main text: the reference at
/// `at` and the footnote's first, empty paragraph in the Footnote style,
/// with the caret in it.
pub(crate) fn insert_footnote(document: &Document, at: TextPosition) -> Result<Applied, EditError> {
    if let Some((frame, _, _)) = crate::tables::cell_of_flow(document, at.flow) {
        return insert_table_footnote(document, frame, at);
    }
    if at.flow != document.main_flow_id() {
        return Err("footnotes go in the main text or a table cell");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let style = footnote_style(&mut next, &mut report)?;
    let flow = next.create_flow(style);
    let mut applied = replace_range(
        next,
        at,
        at,
        &OBJECT_REPLACEMENT_CHARACTER.to_string(),
        vec![InlineObject::Footnote { flow }],
        None,
    )?;
    applied.report.added_flows.push(flow);
    applied.report.styles |= report.styles;
    applied.caret = Some(TextPosition {
        flow,
        paragraph: 0,
        byte: 0,
    });
    Ok(applied)
}

/// Insert footnote in a table's cell (`14d8:0000`; VM RNF6): a row added at
/// the table's end, its cells joined into one holding the footnote in the
/// Footnote style, and the reference `<:xN,ROW>` at the caret, numbered
/// within the table; the caret in the new footnote.
fn insert_table_footnote(
    document: &Document,
    frame: FrameId,
    at: TextPosition,
) -> Result<Applied, EditError> {
    use crate::tables::{CellRange, TableEdit};
    let Some(FrameContent::Table(table)) = document.frame(frame).map(|frame| &frame.content) else {
        return Err("no such table");
    };
    let (rows, columns) = (table.rows.len(), table.columns.len());
    let number = table
        .cells
        .iter()
        .filter_map(|cell| match cell.content {
            CellContent::Text(flow) => document.flow(flow),
        })
        .flat_map(|flow| flow.paragraphs.iter())
        .flat_map(|paragraph| &paragraph.objects)
        .filter(|object| matches!(object, InlineObject::TableFootnote { .. }))
        .count()
        + 1;
    let mut report = ChangeReport::default();
    let mut current = document.clone();
    for edit in [
        TableEdit::InsertRows {
            at: rows,
            count: 1,
            like: Some(rows - 1),
        },
        TableEdit::Connect(CellRange {
            rows: rows..rows + 1,
            columns: 0..columns,
        }),
    ] {
        let applied = crate::apply(&current, &crate::Command::EditTable { frame, edit })?;
        merge(&mut report, &applied.report);
        current = applied.document;
    }
    // The footnote row's cell: the Footnote style, empty.
    let style = footnote_style(&mut current, &mut report)?;
    let Some(FrameContent::Table(table)) = current.frame(frame).map(|frame| &frame.content) else {
        return Err("no such table");
    };
    let CellContent::Text(row_flow) = table.cells[rows * columns].content;
    if let Some(flow) = current.flow_mut(row_flow) {
        for paragraph in &mut flow.paragraphs {
            let mut changed = (**paragraph).clone();
            changed.style = style;
            *paragraph = Arc::new(changed);
        }
    }
    let applied = replace_range(
        current,
        at,
        at,
        &OBJECT_REPLACEMENT_CHARACTER.to_string(),
        vec![InlineObject::TableFootnote {
            number: number as u16,
            row: rows as u16,
        }],
        None,
    )?;
    merge(&mut report, &applied.report);
    report.flows.insert(row_flow);
    let mut next = applied.document;
    next.validate()
        .map_err(|_| "the table footnote would not be valid")?;
    let _ = &mut next;
    Ok(Applied {
        document: next,
        caret: Some(TextPosition {
            flow: row_flow,
            paragraph: 0,
            byte: 0,
        }),
        report,
    })
}

fn merge(report: &mut ChangeReport, other: &ChangeReport) {
    report.flows.extend(other.flows.iter().copied());
    report.changed.extend(other.changed.iter().copied());
    report.inserted.extend(other.inserted.iter().copied());
    report.removed.extend(other.removed.iter().copied());
    report.added_flows.extend(other.added_flows.iter().copied());
    report
        .removed_flows
        .extend(other.removed_flows.iter().copied());
    report.frames.extend(other.frames.iter().copied());
    report.styles |= other.styles;
}

/// Footnote Options, with Ami Pro's dialog rules (`FOOTOPTSPROC`): Make
/// endnotes clears Reset number on each page, which clears Repeat footnote
/// number; the starting number is 1 to 9999; a message has at most 64
/// characters.
pub(crate) fn set_footnote_options(
    document: &Document,
    options: &FootnoteOptions,
) -> Result<Applied, EditError> {
    let mut options = options.clone();
    if !(1..=9_999).contains(&options.start) {
        return Err("the starting number is 1 to 9999");
    }
    if options.endnotes() {
        options.set(FootnoteOptions::RESET_EACH_PAGE, false);
    }
    if options.reset_each_page() {
        options.set(FootnoteOptions::REPEAT_NUMBER, false);
    }
    for message in [&mut options.continued_on, &mut options.continued_from] {
        *message = message
            .chars()
            .filter(|character| !character.is_control())
            .take(64)
            .collect();
    }
    if options.length < 0 || options.indent < 0 {
        return Err("the separator's length and indent are not negative");
    }
    let mut next = document.clone();
    next.set_footnote_options(options);
    Ok(Applied {
        document: next,
        caret: None,
        report: ChangeReport {
            info: true,
            ..ChangeReport::default()
        },
    })
}
