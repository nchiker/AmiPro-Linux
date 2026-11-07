//! Insert › File… and Paste from other programs (`docs/Design/UI_SPEC.md`
//! §31, IM3, IM12): a document read from a file (`amipro_exchange`'s
//! `realise`, run with this document as its base) put at a position as one
//! command.
//!
//! The document's own styles keep their definitions; the source's other
//! styles are added. Its main text's paragraphs go in whole at the
//! position: before the paragraph when the position is at its start, after
//! it at its end, and between its two halves in the middle; the position
//! stays where it was (Ami Pro's Insert). Frames the text anchors, and the
//! source's Where placed frames (on the pages from the position's on), come
//! with their text, tables, and pictures under new identities. The source's
//! page layouts, headers, and footers do not: its breaks to another layout
//! are plain page breaks, its floating headers and footers left out (IM3).

use std::collections::BTreeMap;
use std::sync::Arc;

use amipro_document::{
    CellContent, Document, Flow, FlowId, Frame, FrameContent, FrameId, GroupId, InlineObject,
    ObjectId, Paragraph, Placement, StyleId,
};

use crate::{Applied, ChangeReport, EditError, TextPosition};

const OBJECT: char = '\u{FFFC}';

/// The identities the source's parts take in the document.
struct Copier<'a> {
    source: &'a Document,
    next: Document,
    styles: BTreeMap<StyleId, StyleId>,
    objects: BTreeMap<ObjectId, ObjectId>,
    groups: BTreeMap<GroupId, GroupId>,
    /// The page the source's first page lands on.
    first_page: u32,
    /// The page frames copied, each with its place in the source's stack.
    stacked: Vec<(usize, FrameId)>,
    report: ChangeReport,
}

/// Puts `source`'s main text at `at` in `document` (see the module), its
/// Where placed frames from page `first_page` on.
pub fn insert_document(
    document: &Document,
    at: TextPosition,
    source: &Document,
    first_page: u32,
) -> Result<Applied, EditError> {
    let flow = document.flow(at.flow).ok_or("flow does not exist")?;
    let paragraph = flow
        .paragraph(at.paragraph)
        .ok_or("paragraph out of bounds")?;
    if at.byte > paragraph.text.len() || !paragraph.text.is_char_boundary(at.byte) {
        return Err("the position is not in its paragraph");
    }
    let main = at.flow == document.main_flow_id();
    let mut copier = Copier {
        source,
        next: document.clone(),
        styles: BTreeMap::new(),
        objects: BTreeMap::new(),
        groups: BTreeMap::new(),
        first_page: first_page.max(1),
        stacked: Vec::new(),
        report: ChangeReport::default(),
    };
    copier.styles();
    let mut inserted: Vec<Paragraph> = source
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| copier.paragraph(paragraph, main))
        .collect();
    if main {
        // Where placed frames on the pages from the position's on.
        let fixed: Vec<FrameId> = source
            .frames()
            .filter(|frame| matches!(frame.placement, Placement::Fixed { .. }))
            .map(|frame| frame.id)
            .collect();
        for id in fixed {
            copier.frame(id);
        }
    }
    // The page frames in front of the document's, in the source's order.
    let mut stacked = std::mem::take(&mut copier.stacked);
    stacked.sort_by_key(|(order, _)| *order);
    copier
        .next
        .frame_stack_mut()
        .extend(stacked.into_iter().map(|(_, id)| id));
    // A source of one empty paragraph inserts nothing.
    if inserted.len() == 1 && inserted[0].text.is_empty() {
        inserted.clear();
    }
    let mut next = copier.next;
    let mut report = copier.report;
    let mut paragraphs = next
        .flow(at.flow)
        .ok_or("flow does not exist")?
        .paragraphs
        .clone();
    let target = Arc::clone(&paragraphs[at.paragraph]);
    let count = inserted.len();
    report.flows.insert(at.flow);
    report
        .inserted
        .extend(inserted.iter().map(|paragraph| paragraph.id));
    let caret = if at.byte == 0 {
        paragraphs.splice(
            at.paragraph..at.paragraph,
            inserted.into_iter().map(Arc::new),
        );
        TextPosition {
            paragraph: at.paragraph + count,
            ..at
        }
    } else if at.byte == target.text.len() {
        paragraphs.splice(
            at.paragraph + 1..at.paragraph + 1,
            inserted.into_iter().map(Arc::new),
        );
        at
    } else {
        // The paragraph's two halves around the inserted ones.
        let (first, second) = split(&mut next, &target, at.byte);
        report.changed.push(first.id);
        report.inserted.push(second.id);
        let mut pieces = vec![Arc::new(first)];
        pieces.extend(inserted.into_iter().map(Arc::new));
        pieces.push(Arc::new(second));
        paragraphs.splice(at.paragraph..=at.paragraph, pieces);
        TextPosition {
            paragraph: at.paragraph + count + 1,
            byte: 0,
            ..at
        }
    };
    next.replace_flow_paragraphs(at.flow, paragraphs)
        .map_err(|_| "flow does not exist")?;
    Ok(Applied {
        document: next,
        caret: Some(caret),
        report,
    })
}

impl Copier<'_> {
    /// The source's styles: the document's of the same name, else added
    /// under an identity of their own.
    fn styles(&mut self) {
        let mut known: Vec<(String, StyleId)> = self
            .next
            .styles()
            .values()
            .map(|style| (style.name.to_lowercase(), style.id))
            .collect();
        let mut next_id = self
            .next
            .styles()
            .keys()
            .map(|id| id.0)
            .max()
            .unwrap_or(0)
            .max(
                self.source
                    .styles()
                    .keys()
                    .map(|id| id.0)
                    .max()
                    .unwrap_or(0),
            );
        let mut added = Vec::new();
        for style in self.source.styles().values() {
            let name = style.name.to_lowercase();
            if let Some((_, id)) = known.iter().find(|(known, _)| *known == name) {
                self.styles.insert(style.id, *id);
                continue;
            }
            next_id += 1;
            let id = StyleId(next_id);
            self.styles.insert(style.id, id);
            known.push((name, id));
            added.push(style.clone());
        }
        for mut style in added {
            style.id = self.styles[&style.id];
            style.parent = style
                .parent
                .and_then(|parent| self.styles.get(&parent).copied());
            style.next = style.next.and_then(|next| self.styles.get(&next).copied());
            self.next.styles_mut().insert(style.id, style);
            self.report.styles = true;
        }
    }

    fn style(&self, id: StyleId) -> StyleId {
        self.styles
            .get(&id)
            .copied()
            .unwrap_or_else(|| self.next.body_text_style())
    }

    /// A paragraph under a new identity in the document's styles, its
    /// frames copied; objects the target flow cannot hold, floating headers
    /// and footers, and the source's layouts left out.
    fn paragraph(&mut self, paragraph: &Paragraph, main: bool) -> Paragraph {
        let mut copy = paragraph.clone();
        copy.id = self.next.allocate_paragraph_id();
        copy.style = self.style(paragraph.style);
        copy.cache = None;
        let mut kept = Vec::with_capacity(paragraph.objects.len());
        let mut dropped = Vec::new();
        let offsets: Vec<usize> = paragraph.object_offsets().collect();
        for (offset, object) in offsets.iter().zip(&paragraph.objects) {
            let object = match object {
                InlineObject::FloatingHeaderFooter { .. } => None,
                InlineObject::Break { kind, centre, .. } if main => Some(InlineObject::Break {
                    kind: *kind,
                    centre: *centre,
                    layout: None,
                    restart: false,
                }),
                InlineObject::FrameAnchor(frame) if main => {
                    self.frame(*frame).map(InlineObject::FrameAnchor)
                }
                other if main || other.allowed_in_any_flow() => Some(other.clone()),
                _ => None,
            };
            match object {
                Some(object) => kept.push(object),
                None => dropped.push(*offset),
            }
        }
        copy.objects = kept;
        if !dropped.is_empty() {
            remove_characters(&mut copy, &dropped);
        }
        copy
    }

    /// A frame and what it owns under new identities: its text, its table's
    /// cells and their frames, its picture's bytes.
    fn frame(&mut self, id: FrameId) -> Option<FrameId> {
        let frame = self.source.frame(id)?.clone();
        let new_id = self.next.allocate_frame_id();
        let content = match &frame.content {
            FrameContent::Text(flow) => FrameContent::Text(self.flow(*flow)?),
            FrameContent::Table(table) => {
                let mut table = table.clone();
                for cell in &mut table.cells {
                    let CellContent::Text(flow) = cell.content;
                    cell.content = CellContent::Text(self.flow(flow)?);
                    cell.frame = cell.frame.and_then(|frame| self.frame(frame));
                }
                FrameContent::Table(table)
            }
            FrameContent::Picture(picture) => {
                let mut picture = picture.clone();
                if let amipro_document::PictureSource::Embedded(object) = picture.source {
                    picture.source = amipro_document::PictureSource::Embedded(self.object(object)?);
                }
                picture.rendering = picture.rendering.and_then(|object| self.object(object));
                FrameContent::Picture(picture)
            }
            FrameContent::Legacy(legacy) => FrameContent::Legacy(legacy.clone()),
        };
        let placement = match frame.placement {
            Placement::Fixed { page, rect } => Placement::Fixed {
                page: page.saturating_add(self.first_page - 1),
                rect,
            },
            other => other,
        };
        let group = frame.group.map(|group| {
            *self
                .groups
                .entry(group)
                .or_insert_with(|| self.next.allocate_group_id())
        });
        if placement.is_page_frame() {
            let order = self
                .source
                .frame_stack()
                .iter()
                .position(|frame| *frame == id)
                .unwrap_or(usize::MAX);
            self.stacked.push((order, new_id));
        }
        self.next.insert_frame(Frame {
            id: new_id,
            placement,
            style: frame.style.clone(),
            group,
            content,
            legacy: frame.legacy.clone(),
        });
        self.report.frames.insert(new_id);
        Some(new_id)
    }

    fn flow(&mut self, id: FlowId) -> Option<FlowId> {
        let flow = self.source.flow(id)?;
        let paragraphs: Vec<Arc<Paragraph>> = flow
            .paragraphs
            .iter()
            .map(|paragraph| Arc::new(self.paragraph(paragraph, false)))
            .collect();
        let new_id = self.next.allocate_flow_id();
        self.next.insert_flow(new_id, Flow { paragraphs });
        self.report.added_flows.push(new_id);
        Some(new_id)
    }

    fn object(&mut self, id: ObjectId) -> Option<ObjectId> {
        if let Some(copy) = self.objects.get(&id) {
            return Some(*copy);
        }
        let bytes = Arc::clone(&self.source.objects().get(&id)?.bytes);
        let copy = self.next.add_object(bytes);
        self.objects.insert(id, copy);
        Some(copy)
    }
}

/// Removes the object characters at `offsets` (ascending) from a
/// paragraph's text, moving its runs and marks with the text after them.
fn remove_characters(paragraph: &mut Paragraph, offsets: &[usize]) {
    let width = OBJECT.len_utf8();
    for &offset in offsets.iter().rev() {
        if paragraph.text.get(offset..offset + width) != Some("\u{FFFC}") {
            continue;
        }
        paragraph.text.replace_range(offset..offset + width, "");
        let moved = |at: usize| {
            if at > offset {
                at.saturating_sub(width).max(offset)
            } else {
                at
            }
        };
        for run in &mut paragraph.runs {
            run.start = moved(run.start);
            run.end = moved(run.end);
        }
        paragraph.runs.retain(|run| run.start < run.end);
        for mark in &mut paragraph.marks {
            mark.byte = moved(mark.byte);
        }
    }
}

/// A paragraph cut at `byte`: the first half keeps its identity, the second
/// takes a new one.
fn split(next: &mut Document, paragraph: &Paragraph, byte: usize) -> (Paragraph, Paragraph) {
    let mut first = paragraph.clone();
    let mut second = paragraph.clone();
    second.id = next.allocate_paragraph_id();
    first.text.truncate(byte);
    second.text = paragraph.text[byte..].to_owned();
    let before = paragraph.text[..byte].matches(OBJECT).count();
    first.objects = paragraph.objects[..before.min(paragraph.objects.len())].to_vec();
    second.objects = paragraph.objects[before.min(paragraph.objects.len())..].to_vec();
    first.runs = paragraph
        .runs
        .iter()
        .filter(|run| run.start < byte)
        .map(|run| amipro_document::CharacterRun {
            start: run.start,
            end: run.end.min(byte),
            format: run.format.clone(),
        })
        .collect();
    second.runs = paragraph
        .runs
        .iter()
        .filter(|run| run.end > byte)
        .map(|run| amipro_document::CharacterRun {
            start: run.start.max(byte) - byte,
            end: run.end - byte,
            format: run.format.clone(),
        })
        .collect();
    first.marks = paragraph
        .marks
        .iter()
        .filter(|mark| mark.byte < byte)
        .cloned()
        .collect();
    second.marks = paragraph
        .marks
        .iter()
        .filter(|mark| mark.byte >= byte)
        .map(|mark| amipro_document::LegacyMark {
            byte: mark.byte - byte,
            raw: mark.raw.clone(),
        })
        .collect();
    first.end_format = crate::caret_format(paragraph, byte).clone();
    first.cache = None;
    second.cache = None;
    (first, second)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(document: &Document, paragraph: usize, byte: usize) -> TextPosition {
        TextPosition {
            flow: document.main_flow_id(),
            paragraph,
            byte,
        }
    }

    fn texts(document: &Document) -> Vec<String> {
        document
            .main_flow()
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.text.clone())
            .collect()
    }

    fn source_with(paragraphs: &[(&str, &str)], base: &Document) -> Document {
        let mut source = Document::empty("Source", base.styles().clone());
        let mut list = Vec::new();
        for (text, style) in paragraphs {
            let id = match source
                .styles()
                .values()
                .find(|known| known.name == *style)
                .map(|known| known.id)
            {
                Some(id) => id,
                None => {
                    let id = StyleId(source.styles().keys().map(|id| id.0).max().unwrap_or(0) + 1);
                    let mut defined = source.styles()[&source.body_text_style()].clone();
                    defined.id = id;
                    defined.name = (*style).into();
                    source.styles_mut().insert(id, defined);
                    id
                }
            };
            list.push(Arc::new(source.new_paragraph(*text, id)));
        }
        let main = source.main_flow_id();
        source.replace_flow_paragraphs(main, list).unwrap();
        source
    }

    #[test]
    fn paragraphs_go_in_whole_and_the_position_stays() {
        let document = Document::with_paragraphs([("Before after", StyleId(1))]);
        let source = source_with(&[("One", "Body Text"), ("Two", "Quote")], &document);
        // In the middle: between the halves.
        let applied = insert_document(&document, position(&document, 0, 7), &source, 1).unwrap();
        assert_eq!(texts(&applied.document), ["Before ", "One", "Two", "after"]);
        assert_eq!(applied.caret, Some(position(&applied.document, 3, 0)));
        // The new style added, the document's Body Text kept.
        let quote = &applied.document.main_flow().paragraphs[2];
        assert_eq!(applied.document.styles()[&quote.style].name, "Quote");
        let body = &applied.document.main_flow().paragraphs[1];
        assert_eq!(body.style, document.body_text_style());
        assert!(applied.report.styles);
        // At the start: before; at the end: after.
        let applied = insert_document(&document, position(&document, 0, 0), &source, 1).unwrap();
        assert_eq!(texts(&applied.document), ["One", "Two", "Before after"]);
        assert_eq!(applied.caret, Some(position(&applied.document, 2, 0)));
        let end = "Before after".len();
        let applied = insert_document(&document, position(&document, 0, end), &source, 1).unwrap();
        assert_eq!(texts(&applied.document), ["Before after", "One", "Two"]);
        assert!(applied.document.validate().is_ok());
    }

    #[test]
    fn frames_come_with_new_identities_and_layouts_stay() {
        let document = Document::with_paragraphs([("Text", StyleId(1))]);
        let mut source = source_with(&[("", "Body Text")], &document);
        let style = source.body_text_style();
        let frame = source.create_text_frame(
            Placement::Fixed {
                page: 2,
                rect: amipro_document::Rect {
                    left: 1_440,
                    top: 1_440,
                    right: 4_320,
                    bottom: 2_880,
                },
            },
            amipro_document::FrameStyle::default(),
        );
        let FrameContent::Text(flow) = source.frame(frame).unwrap().content else {
            panic!("a text frame")
        };
        let inner = Arc::new(source.new_paragraph("In the frame", style));
        source.replace_flow_paragraphs(flow, vec![inner]).unwrap();
        let main = source.main_flow_id();
        let mut paragraph = source.new_paragraph("Body\u{FFFC}", style);
        paragraph.objects.push(InlineObject::Break {
            kind: amipro_document::BreakKind::Page,
            centre: false,
            layout: Some(source.document_layout().id),
            restart: false,
        });
        source
            .replace_flow_paragraphs(main, vec![Arc::new(paragraph)])
            .unwrap();
        let applied = insert_document(&document, position(&document, 0, 4), &source, 3).unwrap();
        let next = &applied.document;
        assert!(next.validate().is_ok(), "{:?}", next.validate());
        let copied = next
            .frames()
            .find(|frame| matches!(frame.placement, Placement::Fixed { .. }))
            .unwrap();
        assert!(matches!(copied.placement, Placement::Fixed { page: 4, .. }));
        let FrameContent::Text(copied_flow) = copied.content else {
            panic!("a text frame")
        };
        assert_eq!(
            next.flow(copied_flow).unwrap().paragraphs[0].text,
            "In the frame"
        );
        assert!(next.frame_stack().contains(&copied.id));
        // The break to the source's layout is a plain page break.
        let body = &next.main_flow().paragraphs[1];
        assert!(matches!(
            body.objects[0],
            InlineObject::Break { layout: None, .. }
        ));
        assert_eq!(next.layouts().len(), document.layouts().len());
    }

    #[test]
    fn floating_headers_are_left_out_with_their_character() {
        let document = Document::with_paragraphs([("Text", StyleId(1))]);
        let mut source = source_with(&[("", "Body Text")], &document);
        let style = source.body_text_style();
        let flow = source.create_flow(style);
        let main = source.main_flow_id();
        let mut paragraph = source.new_paragraph("A\u{FFFC}B", style);
        paragraph.objects.push(InlineObject::FloatingHeaderFooter {
            kind: amipro_document::HeaderFooterKind::Header,
            pages: amipro_document::PageSides::All,
            flow,
        });
        paragraph.runs.push(amipro_document::CharacterRun {
            start: 4,
            end: 5,
            format: amipro_document::CharacterFormat {
                bold: Some(true),
                ..Default::default()
            },
        });
        source
            .replace_flow_paragraphs(main, vec![Arc::new(paragraph)])
            .unwrap();
        let applied = insert_document(&document, position(&document, 0, 0), &source, 1).unwrap();
        let inserted = &applied.document.main_flow().paragraphs[0];
        assert_eq!(inserted.text, "AB");
        assert!(inserted.objects.is_empty());
        assert_eq!((inserted.runs[0].start, inserted.runs[0].end), (1, 2));
        assert!(applied.document.validate().is_ok());
    }
}
