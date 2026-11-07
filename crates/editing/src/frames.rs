//! Frame and page-layout commands (`docs/research/DECISIONS.md`, Phase 2.2,
//! D4–D7), following Ami Pro's rules: a group moves and deletes as one, page
//! frames and layout frames reorder only within their own layer, and deleting
//! an anchored frame deletes its anchor.

use std::sync::Arc;

use amipro_document::{
    BreakKind, CellContent, CellMargins, CellSpan, Document, Flow, FlowId, Frame, FrameContent,
    FrameId, FrameStyle, GroupId, InlineObject, LayoutBox, LayoutId, LegacyData,
    OBJECT_REPLACEMENT_CHARACTER, PageGeometry, Paragraph, PictureSource, Placement, Rect, Size,
    Table, TableCell, TableRows, Twips, Wrap,
};

use crate::{Applied, ChangeReport, EditError, TextPosition, replace_range};

/// The content of a new frame.
#[derive(Clone, Debug, PartialEq)]
pub enum NewFrameContent {
    /// One empty paragraph in Body Text, as new frame text starts.
    Text,
    /// A grid of empty cells with these column widths and row heights.
    Table {
        columns: Vec<Twips>,
        rows: Vec<Twips>,
    },
    /// A picture whose bytes are copied into the document.
    EmbeddedPicture { bytes: Arc<[u8]>, extension: String },
    /// A picture that stays in its own file.
    LinkedPicture { path: String, extension: String },
}

/// A page layout's settings: everything but its frames, and the interiors
/// of its header and footer frames.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSettings {
    pub name: String,
    pub paper: u8,
    pub landscape: bool,
    pub right: PageGeometry,
    pub left: Option<PageGeometry>,
    pub mirror: bool,
    pub header_from_second_page: bool,
    pub footer_from_second_page: bool,
    /// The header frames' interiors: margins within the band, columns,
    /// gutter, balance, and tabs (Ami Pro's Header panel).
    pub header: BandBoxes,
    /// The footer frames' interiors (the Footer panel).
    pub footer: BandBoxes,
}

/// A header's or footer's interior on right and left pages. `None` leaves
/// that frame as it is (or the layout has no such frame).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BandBoxes {
    pub right: Option<LayoutBox>,
    pub left: Option<LayoutBox>,
}

impl LayoutSettings {
    /// Layout `layout`'s current settings, its bands' interiors included.
    pub fn of(document: &Document, layout: LayoutId) -> Option<Self> {
        let layout = document.layout(layout)?;
        let interior = |frame: Option<FrameId>| {
            frame
                .and_then(|frame| document.frame(frame))
                .map(|frame| frame.style.interior.clone())
        };
        Some(Self {
            name: layout.name.clone(),
            paper: layout.paper,
            landscape: layout.landscape,
            right: layout.right.clone(),
            left: layout.left.clone(),
            mirror: layout.mirror,
            header_from_second_page: layout.header_from_second_page,
            footer_from_second_page: layout.footer_from_second_page,
            header: BandBoxes {
                right: interior(layout.header.right),
                left: interior(layout.header.left),
            },
            footer: BandBoxes {
                right: interior(layout.footer.right),
                left: interior(layout.footer.left),
            },
        })
    }
}

fn unchanged(document: &Document) -> Applied {
    Applied {
        document: document.clone(),
        caret: None,
        report: ChangeReport::default(),
    }
}

fn changed(document: Document, report: ChangeReport) -> Result<Applied, EditError> {
    Ok(Applied {
        document,
        caret: None,
        report,
    })
}

fn frame_of(document: &Document, frame: FrameId) -> Result<&Frame, EditError> {
    document.frame(frame).ok_or("frame does not exist")
}

/// The frames that act together with `frame`: its group, in z-order, or
/// itself.
fn group_block(document: &Document, frame: FrameId) -> Result<Vec<FrameId>, EditError> {
    let group = frame_of(document, frame)?.group;
    Ok(match group {
        Some(group) => document
            .frame_stack()
            .iter()
            .copied()
            .filter(|id| document.frame(*id).is_some_and(|f| f.group == Some(group)))
            .collect(),
        None => vec![frame],
    })
}

/// Where a frame's anchor is in the main flow.
pub(crate) fn anchor_position(document: &Document, frame: FrameId) -> Option<TextPosition> {
    let flow = document.main_flow_id();
    document
        .paragraphs()
        .iter()
        .enumerate()
        .find_map(|(paragraph, source)| {
            source
                .object_offsets()
                .zip(&source.objects)
                .find(|(_, object)| object.anchored_frame() == Some(frame))
                .map(|(byte, _)| TextPosition {
                    flow,
                    paragraph,
                    byte,
                })
        })
}

/// Deletes a frame and everything it owns: its flows, and a table's frames
/// in its cells (Ami Pro's Delete Entire Table). Its anchor, if any, is the
/// caller's; a frame in a cell leaves its cell empty.
pub(crate) fn delete_frame_tree(
    document: &mut Document,
    frame: FrameId,
    report: &mut ChangeReport,
) {
    let Some(removed) = document.remove_frame(frame) else {
        return;
    };
    report.removed_frames.push(frame);
    for flow in removed.owned_flows() {
        if document.remove_flow(flow).is_some() {
            report.removed_flows.push(flow);
        }
    }
    for inner in removed.owned_frames() {
        delete_frame_tree(document, inner, report);
    }
}

/// A new picture frame's picture area, the size its picture is stored at
/// until it is drawn: the frame inside its margins and corner inset.
fn picture_area(placement: Placement, style: &FrameStyle) -> Size {
    let size = placement.size().unwrap_or(Size {
        width: 1_440,
        height: 1_440,
    });
    let insets = style.text_insets(size.width, size.height);
    Size {
        width: (size.width - insets.left - insets.right).max(1),
        height: (size.height - insets.top - insets.bottom).max(1),
    }
}

/// Makes a new empty flow in Body Text, for frame text and table cells.
fn new_text_flow(document: &mut Document, report: &mut ChangeReport) -> FlowId {
    let flow = document.create_flow(document.body_text_style());
    report.added_flows.push(flow);
    flow
}

/// The wrap a placement allows: Flow with text frames wrap around, and a
/// frame that becomes With para above changes Wrap around to No wrap beside,
/// as Ami Pro's Modify Frame Layout does (F16).
fn wrap_for(placement: Placement, wrap: Wrap) -> Wrap {
    match placement {
        Placement::Inline { .. } => Wrap::Around,
        Placement::BelowParagraph { .. } if wrap == Wrap::Around => Wrap::NoWrapBeside,
        _ => wrap,
    }
}

fn check_style(placement: Placement, style: &FrameStyle) -> Result<(), EditError> {
    if wrap_for(placement, style.wrap) != style.wrap {
        return Err("Flow with text frames wrap around; With para above frames do not");
    }
    Ok(())
}

pub(crate) fn create_frame(
    document: &Document,
    placement: Placement,
    style: &FrameStyle,
    content: &NewFrameContent,
    anchor: Option<TextPosition>,
    layout: Option<LayoutId>,
) -> Result<Applied, EditError> {
    placement.validate_for_command()?;
    check_style(placement, style)?;
    if matches!(
        placement,
        Placement::Header { .. } | Placement::Footer { .. }
    ) {
        return Err("header and footer frames belong to page layouts");
    }
    if placement.is_anchored() != anchor.is_some() {
        return Err("anchored frames, and only they, need an anchor position");
    }
    let layout = match placement {
        Placement::Repeating { .. } => {
            Some(layout.unwrap_or_else(|| document.document_layout().id))
        }
        _ => None,
    };
    if let Some(layout) = layout
        && document.layout(layout).is_none()
    {
        return Err("layout does not exist");
    }

    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let content = match content {
        NewFrameContent::Text => FrameContent::Text(new_text_flow(&mut next, &mut report)),
        NewFrameContent::Table { columns, rows } => {
            if columns.is_empty() || rows.is_empty() {
                return Err("a table needs at least one column and one row");
            }
            // Every cell's text is Table Text (user's guide), made with the
            // first table.
            let styles = next.styles().len();
            let table_text = next.table_text_style();
            report.styles |= next.styles().len() != styles;
            let cells = (0..columns.len() * rows.len())
                .map(|_| TableCell {
                    content: CellContent::Text({
                        let flow = next.create_flow(table_text);
                        report.added_flows.push(flow);
                        flow
                    }),
                    span: CellSpan::SINGLE,
                    frame: None,
                    format: Default::default(),
                    formula: None,
                    legacy: LegacyData::default(),
                })
                .collect();
            FrameContent::Table(Table {
                columns: columns.clone(),
                rows: rows.clone(),
                cell_margins: CellMargins::default(),
                spacing: Default::default(),
                row_options: Default::default(),
                layout: amipro_document::TableLayout::inferred(rows, columns),
                read_rows: Default::default(),
                cells,
                legacy: LegacyData::default(),
            })
        }
        NewFrameContent::EmbeddedPicture { bytes, extension } => {
            FrameContent::Picture(crate::pictures::fresh(
                PictureSource::Embedded(next.add_object(Arc::clone(bytes))),
                extension.clone(),
                picture_area(placement, style),
            ))
        }
        NewFrameContent::LinkedPicture { path, extension } => {
            FrameContent::Picture(crate::pictures::fresh(
                PictureSource::Linked(path.clone()),
                extension.clone(),
                picture_area(placement, style),
            ))
        }
    };
    let id = next.allocate_frame_id();
    let mut legacy = LegacyData::default();
    if matches!(content, FrameContent::Text(_)) {
        // A new frame's text is a `[txt]` without paragraphs until it is
        // typed in, as Ami Pro writes it (FC1), and as the importer marks
        // such a frame.
        legacy.push("txt", "none");
    }
    next.insert_frame(Frame {
        id,
        placement,
        style: style.clone(),
        group: None,
        content,
        legacy,
    });
    report.frames.insert(id);
    if let Some(layout) = layout {
        next.layout_mut(layout)
            .ok_or("layout does not exist")?
            .repeating
            .push(id);
        report.layouts.insert(layout);
    } else {
        next.frame_stack_mut().push(id);
    }
    if let Some(at) = anchor {
        return attach_anchor(next, at, id, report);
    }
    changed(next, report)
}

/// Inserts `frame`'s anchor at `at` and merges the reports.
pub(crate) fn attach_anchor(
    next: Document,
    at: TextPosition,
    frame: FrameId,
    report: ChangeReport,
) -> Result<Applied, EditError> {
    if at.flow != next.main_flow_id() {
        return Err("inline objects occur only in the main flow");
    }
    let mut applied = replace_range(
        next,
        at,
        at,
        &OBJECT_REPLACEMENT_CHARACTER.to_string(),
        vec![InlineObject::FrameAnchor(frame)],
        None,
    )?;
    applied.report.merge(report);
    Ok(applied)
}

/// Removes `frame`'s anchor character without deleting the frame.
fn detach_anchor(next: Document, frame: FrameId) -> Result<Applied, EditError> {
    let at = anchor_position(&next, frame).ok_or("anchored frame has no anchor")?;
    let end = TextPosition {
        byte: at.byte + OBJECT_REPLACEMENT_CHARACTER.len_utf8(),
        ..at
    };
    replace_range(next, at, end, "", Vec::new(), Some(frame))
}

pub(crate) fn delete_frame(document: &Document, frame: FrameId) -> Result<Applied, EditError> {
    let placement = frame_of(document, frame)?.placement;
    match placement {
        Placement::Header { .. } | Placement::Footer { .. } => {
            Err("header and footer frames belong to page layouts")
        }
        Placement::BelowParagraph { .. } | Placement::Inline { .. } => {
            // Deleting the anchor deletes the frame with it.
            let at = anchor_position(document, frame).ok_or("anchored frame has no anchor")?;
            let end = TextPosition {
                byte: at.byte + OBJECT_REPLACEMENT_CHARACTER.len_utf8(),
                ..at
            };
            replace_range(document.clone(), at, end, "", Vec::new(), None)
        }
        // Del in the frame's cell (T1): the cell is left empty, and its
        // table is laid out again.
        Placement::InCell => {
            let mut next = document.clone();
            let mut report = ChangeReport::default();
            if let Some((table, _)) = document.cell_holding(frame) {
                report.frames.insert(table);
            }
            delete_frame_tree(&mut next, frame, &mut report);
            changed(next, report)
        }
        Placement::Fixed { .. } | Placement::Repeating { .. } => {
            let mut next = document.clone();
            let mut report = ChangeReport::default();
            for member in group_block(document, frame)? {
                delete_frame_tree(&mut next, member, &mut report);
            }
            if let Placement::Repeating { .. } = placement {
                report.layouts.extend(
                    document
                        .layouts()
                        .iter()
                        .filter(|layout| layout.repeating.contains(&frame))
                        .map(|layout| layout.id),
                );
            }
            changed(next, report)
        }
    }
}

pub(crate) fn move_frame(
    document: &Document,
    frame: FrameId,
    dx: Twips,
    dy: Twips,
) -> Result<Applied, EditError> {
    if dx == 0 && dy == 0 {
        return Ok(unchanged(document));
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for member in group_block(document, frame)? {
        let target = next.frame_mut(member).ok_or("frame does not exist")?;
        target.placement = match target.placement {
            Placement::Fixed { page, rect } => Placement::Fixed {
                page,
                rect: rect.offset(dx, dy).ok_or("frame position overflows")?,
            },
            Placement::Repeating { pages, rect } => Placement::Repeating {
                pages,
                rect: rect.offset(dx, dy).ok_or("frame position overflows")?,
            },
            Placement::BelowParagraph { x_offset, size } if dy == 0 => Placement::BelowParagraph {
                x_offset: x_offset.checked_add(dx).ok_or("frame position overflows")?,
                size,
            },
            Placement::Inline { y_offset, size } if dx == 0 => Placement::Inline {
                y_offset: y_offset.checked_add(dy).ok_or("frame position overflows")?,
                size,
            },
            Placement::BelowParagraph { .. } => {
                return Err("a With para above frame moves only horizontally");
            }
            Placement::Inline { .. } => {
                return Err("a Flow with text frame moves only vertically");
            }
            Placement::Header { .. } | Placement::Footer { .. } => {
                return Err("header and footer frames follow the page margins");
            }
            Placement::InCell => return Err(IN_CELL),
        };
        report.frames.insert(member);
    }
    changed(next, report)
}

/// Why a frame in a table cell is not moved, resized, or placed elsewhere:
/// it covers its cell's text area (Ami Pro disables Modify Frame Layout
/// there, TF5).
const IN_CELL: &str = "a frame in a table cell takes its cell's place and size";

pub(crate) fn set_frame_page(
    document: &Document,
    frame: FrameId,
    page: u32,
) -> Result<Applied, EditError> {
    if page == 0 {
        return Err("fixed frames are on page 1 or later");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for member in group_block(document, frame)? {
        let target = next.frame_mut(member).ok_or("frame does not exist")?;
        let Placement::Fixed { rect, .. } = target.placement else {
            return Err("only Where placed frames are on a page");
        };
        target.placement = Placement::Fixed { page, rect };
        report.frames.insert(member);
    }
    changed(next, report)
}

pub(crate) fn resize_frame(
    document: &Document,
    frame: FrameId,
    rect: Rect,
) -> Result<Applied, EditError> {
    if !rect.is_valid() {
        return Err("frame rectangle is empty or inverted");
    }
    let mut next = document.clone();
    let target = next.frame_mut(frame).ok_or("frame does not exist")?;
    let size = amipro_document::Size {
        width: rect.width(),
        height: rect.height(),
    };
    let old_width = match target.placement {
        Placement::Fixed { rect, .. } | Placement::Repeating { rect, .. } => rect.width(),
        Placement::BelowParagraph { size, .. } | Placement::Inline { size, .. } => size.width,
        Placement::Header { .. } | Placement::Footer { .. } => rect.width(),
        Placement::InCell => return Err(IN_CELL),
    };
    // Uneven columns inside follow the frame's right edge, or become even
    // when they no longer fit (no evidence for frames: the rule of Ami
    // Pro's page dialog, `crate::layout_setup::refit_columns`).
    let interior = &mut target.style.interior;
    let margins = interior.margins;
    if crate::layout_setup::refit_columns(interior, margins, old_width, rect.width()).is_err() {
        crate::layout_setup::even_columns(interior);
    }
    target.placement = match target.placement {
        Placement::Fixed { page, .. } => Placement::Fixed { page, rect },
        Placement::Repeating { pages, .. } => Placement::Repeating { pages, rect },
        Placement::BelowParagraph { x_offset, .. } => Placement::BelowParagraph { x_offset, size },
        // The top keeps its height above the baseline, so a taller frame
        // hangs further below it (F19).
        Placement::Inline {
            y_offset,
            size: old,
        } => Placement::Inline {
            y_offset: y_offset
                .checked_add(size.height - old.height)
                .ok_or("frame position overflows")?,
            size,
        },
        Placement::Header { .. } | Placement::Footer { .. } => {
            return Err("header and footer frames follow the page margins");
        }
        Placement::InCell => return Err(IN_CELL),
    };
    let mut report = ChangeReport::default();
    report.frames.insert(frame);
    changed(next, report)
}

pub(crate) fn set_frame_style(
    document: &Document,
    frame: FrameId,
    style: &FrameStyle,
) -> Result<Applied, EditError> {
    let current = frame_of(document, frame)?;
    check_style(current.placement, style)?;
    if current.style == *style {
        return Ok(unchanged(document));
    }
    let mut next = document.clone();
    next.frame_mut(frame).ok_or("frame does not exist")?.style = style.clone();
    let mut report = ChangeReport::default();
    report.frames.insert(frame);
    changed(next, report)
}

pub(crate) fn change_placement(
    document: &Document,
    frame: FrameId,
    placement: Placement,
    anchor: Option<TextPosition>,
    layout: Option<LayoutId>,
) -> Result<Applied, EditError> {
    placement.validate_for_command()?;
    let current = frame_of(document, frame)?;
    let old = current.placement;
    if old.is_in_cell() {
        return Err(IN_CELL);
    }
    if matches!(old, Placement::Header { .. } | Placement::Footer { .. })
        || matches!(
            placement,
            Placement::Header { .. } | Placement::Footer { .. }
        )
    {
        return Err("header and footer frames belong to page layouts");
    }
    if current.group.is_some() {
        return Err("ungroup the frame before changing its placement");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    report.frames.insert(frame);
    {
        let target = next.frame_mut(frame).ok_or("frame does not exist")?;
        target.placement = placement;
        target.style.wrap = wrap_for(placement, target.style.wrap);
    }

    // Move between the frame stack and a layout's repeating list.
    let old_layout = document
        .layouts()
        .iter()
        .find(|candidate| candidate.repeating.contains(&frame))
        .map(|candidate| candidate.id);
    let new_layout = match placement {
        Placement::Repeating { .. } => Some(
            layout
                .or(old_layout)
                .unwrap_or_else(|| document.document_layout().id),
        ),
        _ => None,
    };
    if old_layout != new_layout {
        if let Some(old_layout) = old_layout {
            next.layout_mut(old_layout)
                .ok_or("layout does not exist")?
                .repeating
                .retain(|id| *id != frame);
            report.layouts.insert(old_layout);
        } else {
            next.frame_stack_mut().retain(|id| *id != frame);
        }
        if let Some(new_layout) = new_layout {
            next.layout_mut(new_layout)
                .ok_or("layout does not exist")?
                .repeating
                .push(frame);
            report.layouts.insert(new_layout);
        } else {
            next.frame_stack_mut().push(frame);
        }
    }

    // Add or remove the anchor.
    match (old.is_anchored(), placement.is_anchored()) {
        (false, true) => {
            let at = anchor.ok_or("an anchored frame needs an anchor position")?;
            attach_anchor(next, at, frame, report)
        }
        (true, false) => {
            let mut applied = detach_anchor(next, frame)?;
            applied.report.merge(report);
            Ok(applied)
        }
        _ => changed(next, report),
    }
}

/// Moves `frame` (with its group) to the front or the back of its layer.
pub(crate) fn restack(
    document: &Document,
    frame: FrameId,
    to_front: bool,
) -> Result<Applied, EditError> {
    let placement = frame_of(document, frame)?.placement;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let reorder = |list: &mut Vec<FrameId>, block: &[FrameId]| {
        list.retain(|id| !block.contains(id));
        if to_front {
            list.extend_from_slice(block);
        } else {
            list.splice(0..0, block.iter().copied());
        }
    };
    if placement.is_in_cell() {
        return Err(IN_CELL);
    }
    if placement.is_page_frame() {
        // Within its kind: anchored frames are drawn below Where placed ones
        // whatever the stack says (FC8), as Ami Pro keeps them in separate
        // lists; the other kinds keep their places, so anchor numbers and
        // the frames in cells (a third list) do not change when a Where
        // placed frame moves.
        let block = group_block(document, frame)?;
        let anchored = |id: &FrameId| {
            document.frame(*id).map(|frame| {
                if frame.placement.is_in_cell() {
                    None
                } else {
                    Some(frame.placement.is_anchored())
                }
            })
        };
        let kind = Some(Some(placement.is_anchored()));
        let before = document.frame_stack().to_vec();
        let mut same: Vec<FrameId> = before
            .iter()
            .copied()
            .filter(|id| anchored(id) == kind)
            .collect();
        let unmoved = same.clone();
        reorder(&mut same, &block);
        if same == unmoved {
            return Ok(unchanged(document));
        }
        let mut moved = same.into_iter();
        let after: Vec<FrameId> = before
            .iter()
            .map(|id| {
                if anchored(id) == kind {
                    moved.next().unwrap_or(*id)
                } else {
                    *id
                }
            })
            .collect();
        *next.frame_stack_mut() = after;
        report.frames.extend(block);
    } else if let Placement::Repeating { .. } = placement {
        let layout = document
            .layouts()
            .iter()
            .find(|layout| layout.repeating.contains(&frame))
            .map(|layout| layout.id)
            .ok_or("repeating frame has no layout")?;
        let list = &mut next
            .layout_mut(layout)
            .ok_or("layout does not exist")?
            .repeating;
        let before = list.clone();
        reorder(list, &[frame]);
        if *list == before {
            return Ok(unchanged(document));
        }
        report.frames.insert(frame);
        report.layouts.insert(layout);
    } else {
        return Err("header and footer frames have no z-order");
    }
    changed(next, report)
}

pub(crate) fn group(document: &Document, frames: &[FrameId]) -> Result<Applied, EditError> {
    if frames.len() < 2 {
        return Err("a group needs two or more frames");
    }
    let mut page = None;
    for frame in frames {
        let found = frame_of(document, *frame)?;
        let Placement::Fixed {
            page: frame_page, ..
        } = found.placement
        else {
            return Err("only Where placed frames can be grouped");
        };
        if found.group.is_some() {
            return Err("a frame is already grouped");
        }
        if *page.get_or_insert(frame_page) != frame_page {
            return Err("grouped frames must be on one page");
        }
    }
    let mut next = document.clone();
    let group = next.allocate_group_id();
    // The group sits where its frontmost member was, members in their order.
    let stack = document.frame_stack();
    let members: Vec<FrameId> = stack
        .iter()
        .copied()
        .filter(|id| frames.contains(id))
        .collect();
    if members.len() != frames.len() {
        return Err("a frame appears twice");
    }
    let front = stack
        .iter()
        .rposition(|id| frames.contains(id))
        .expect("members are in the stack");
    let insert_at = front + 1 - members.len();
    let mut new_stack: Vec<FrameId> = stack
        .iter()
        .copied()
        .filter(|id| !frames.contains(id))
        .collect();
    new_stack.splice(insert_at..insert_at, members.iter().copied());
    *next.frame_stack_mut() = new_stack;
    let mut report = ChangeReport::default();
    for member in &members {
        next.frame_mut(*member).ok_or("frame does not exist")?.group = Some(group);
        report.frames.insert(*member);
    }
    changed(next, report)
}

pub(crate) fn ungroup(document: &Document, group: GroupId) -> Result<Applied, EditError> {
    let members: Vec<FrameId> = document
        .frames()
        .filter(|frame| frame.group == Some(group))
        .map(|frame| frame.id)
        .collect();
    if members.is_empty() {
        return Err("group does not exist");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for member in members {
        next.frame_mut(member).ok_or("frame does not exist")?.group = None;
        report.frames.insert(member);
    }
    changed(next, report)
}

/// Copies a flow with fresh paragraph IDs.
fn copy_flow(
    next: &mut Document,
    source: FlowId,
    report: &mut ChangeReport,
) -> Result<FlowId, EditError> {
    let paragraphs: Vec<Arc<Paragraph>> = next
        .flow(source)
        .ok_or("flow does not exist")?
        .paragraphs
        .clone();
    let id = next.allocate_flow_id();
    let paragraphs = paragraphs
        .iter()
        .map(|paragraph| {
            let mut copy = (**paragraph).clone();
            copy.id = next.allocate_paragraph_id();
            Arc::new(copy)
        })
        .collect();
    next.insert_flow(id, Flow { paragraphs });
    report.added_flows.push(id);
    Ok(id)
}

/// Copies a layout frame with fresh IDs for it and everything it owns.
fn copy_frame(
    next: &mut Document,
    source: FrameId,
    report: &mut ChangeReport,
) -> Result<FrameId, EditError> {
    let mut frame = next.frame(source).ok_or("frame does not exist")?.clone();
    frame.id = next.allocate_frame_id();
    // A new frame takes the next free sequence number when written, as Ami
    // Pro numbers a left page's header and footer and an inserted layout's
    // frames after the others (G15, G21): the copy keeps none of its own.
    frame.legacy.remove("frame.field13");
    frame.content = match frame.content {
        FrameContent::Text(flow) => FrameContent::Text(copy_flow(next, flow, report)?),
        FrameContent::Table(mut table) => {
            for cell in &mut table.cells {
                let CellContent::Text(flow) = cell.content;
                cell.content = CellContent::Text(copy_flow(next, flow, report)?);
                // A cell's picture is copied with it, at the front of the
                // frame stack, as Ami Pro puts a new cell frame at the front
                // of its list of them (`14d0:0000`).
                if let Some(inner) = cell.frame {
                    let copy = copy_frame(next, inner, report)?;
                    next.frame_stack_mut().insert(0, copy);
                    cell.frame = Some(copy);
                }
            }
            FrameContent::Table(table)
        }
        other => other,
    };
    let id = frame.id;
    next.insert_frame(frame);
    report.frames.insert(id);
    Ok(id)
}

/// Inserts a copy of layout `from` (fresh IDs throughout) and a page break at
/// `at` that switches to it from the next page.
pub(crate) fn insert_layout(
    document: &Document,
    at: TextPosition,
    from: LayoutId,
) -> Result<Applied, EditError> {
    if at.flow != document.main_flow_id() {
        return Err("inline objects occur only in the main flow");
    }
    let source = document
        .layout(from)
        .ok_or("layout does not exist")?
        .clone();
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let mut layout = source.clone();
    layout.id = next.allocate_layout_id();
    // Ami Pro names inserted layouts "1", "2", and so on.
    layout.name = (1..)
        .map(|number: u32| number.to_string())
        .find(|name| {
            !document
                .layouts()
                .iter()
                .any(|existing| existing.name == *name)
        })
        .expect("layout names are finite");
    for slot in [
        &mut layout.header.right,
        &mut layout.header.left,
        &mut layout.footer.right,
        &mut layout.footer.left,
    ] {
        if let Some(frame) = *slot {
            *slot = Some(copy_frame(&mut next, frame, &mut report)?);
        }
    }
    layout.repeating = source
        .repeating
        .iter()
        .map(|frame| copy_frame(&mut next, *frame, &mut report))
        .collect::<Result<_, _>>()?;
    let id = layout.id;
    next.layouts_mut().push(Arc::new(layout));
    report.layouts.insert(id);
    let mut applied = replace_range(
        next,
        at,
        at,
        &OBJECT_REPLACEMENT_CHARACTER.to_string(),
        vec![InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: Some(id),
            restart: false,
        }],
        None,
    )?;
    applied.report.merge(report);
    Ok(applied)
}

pub(crate) fn modify_layout(
    document: &Document,
    layout: LayoutId,
    settings: &LayoutSettings,
) -> Result<Applied, EditError> {
    let current = document
        .layout(layout)
        .ok_or("layout does not exist")?
        .clone();
    if settings.mirror && settings.left.is_none() {
        return Err("mirrored layouts have left pages");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    let mut updated = current.clone();
    updated.name.clone_from(&settings.name);
    updated.paper = settings.paper;
    updated.landscape = settings.landscape;
    updated.right = settings.right.clone();
    updated.left.clone_from(&settings.left);
    updated.mirror = settings.mirror;
    updated.header_from_second_page = settings.header_from_second_page;
    updated.footer_from_second_page = settings.footer_from_second_page;
    match (current.left.is_some(), settings.left.is_some()) {
        (false, true) => {
            // Left pages start with copies of the right pages' header and
            // footer.
            let copy = |next: &mut Document,
                        side: Option<FrameId>,
                        header: bool,
                        report: &mut ChangeReport| {
                side.map(|frame| {
                    copy_frame(next, frame, report).and_then(|copy| {
                        next.frame_mut(copy)
                            .ok_or("frame does not exist")?
                            .placement = if header {
                            Placement::Header {
                                side: amipro_document::PageSide::Left,
                            }
                        } else {
                            Placement::Footer {
                                side: amipro_document::PageSide::Left,
                            }
                        };
                        Ok(copy)
                    })
                })
                .transpose()
            };
            updated.header.left = copy(&mut next, current.header.right, true, &mut report)?;
            updated.footer.left = copy(&mut next, current.footer.right, false, &mut report)?;
        }
        (true, false) => {
            for frame in [current.header.left, current.footer.left]
                .into_iter()
                .flatten()
            {
                delete_frame_tree(&mut next, frame, &mut report);
            }
            updated.header.left = None;
            updated.footer.left = None;
        }
        _ => {}
    }
    // Columns kept unequal from the file become even again when the page
    // or its columns change, as Ami Pro's dialog makes them
    // (`PAGE_LAYOUT_NOTES.md`, "Margins and columns").
    let columns_of = |geometry: &PageGeometry| {
        (
            geometry.width,
            geometry.layout.margins.left,
            geometry.layout.margins.right,
            geometry.layout.columns,
            geometry.layout.gutter,
            geometry.layout.column_edges.clone(),
        )
    };
    if columns_of(&updated.right) != columns_of(&current.right) {
        updated.legacy.remove("rght.edges");
    }
    if updated.left.as_ref().map(columns_of) != current.left.as_ref().map(columns_of) {
        updated.legacy.remove("lft.edges");
    }
    updated
        .validate()
        .map_err(|_| "the page does not hold its margins and columns")?;
    let mut bands_changed = false;
    for (frame, interior) in [
        (updated.header.right, &settings.header.right),
        (updated.header.left, &settings.header.left),
        (updated.footer.right, &settings.footer.right),
        (updated.footer.left, &settings.footer.left),
    ] {
        let (Some(frame), Some(interior)) = (frame, interior) else {
            continue;
        };
        interior
            .validate("band")
            .map_err(|_| "the header or footer does not hold its margins and columns")?;
        let frame_mut = next.frame_mut(frame).ok_or("frame does not exist")?;
        if frame_mut.style.interior != *interior {
            frame_mut.style.interior = interior.clone();
            frame_mut.legacy.remove("frmlay.edges");
            report.frames.insert(frame);
            bands_changed = true;
        }
    }
    if updated == current && !bands_changed {
        return Ok(unchanged(document));
    }
    *next.layout_mut(layout).ok_or("layout does not exist")? = updated;
    report.layouts.insert(layout);
    changed(next, report)
}

pub(crate) fn remove_layout(document: &Document, layout: LayoutId) -> Result<Applied, EditError> {
    if document.document_layout().id == layout {
        return Err("the document's own layout cannot be removed");
    }
    let source = document
        .layout(layout)
        .ok_or("layout does not exist")?
        .clone();
    let named = document.flows().any(|(_, flow)| {
        flow.paragraphs.iter().any(|paragraph| {
            paragraph.objects.iter().any(|object| {
                matches!(object, InlineObject::Break { layout: Some(named), .. } if *named == layout)
            })
        })
    });
    if named {
        return Err("a page break still switches to this layout");
    }
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for frame in source.frames() {
        delete_frame_tree(&mut next, frame, &mut report);
    }
    next.layouts_mut()
        .retain(|candidate| candidate.id != layout);
    if next.start_layout_id() == Some(layout) {
        next.set_start_layout(None);
    }
    report.layouts.insert(layout);
    report.removed_layouts.push(layout);
    changed(next, report)
}

trait PlacementCommandCheck {
    fn validate_for_command(self) -> Result<(), EditError>;
}

impl PlacementCommandCheck for Placement {
    fn validate_for_command(self) -> Result<(), EditError> {
        let valid = match self {
            Placement::Fixed { page, rect } => page > 0 && rect.is_valid(),
            Placement::Repeating { rect, .. } => rect.is_valid(),
            Placement::BelowParagraph { size, .. } | Placement::Inline { size, .. } => {
                size.is_valid()
            }
            Placement::Header { .. } | Placement::Footer { .. } => true,
            // Frames go into cells with pictures (T3), not by placement.
            Placement::InCell => return Err(IN_CELL),
        };
        valid.then_some(()).ok_or("invalid frame placement")
    }
}

/// Sets how a table's rows take their height and continue across pages
/// (`UI_SPEC.md` §26): every row named must be one of the table's.
pub(crate) fn set_table_rows(
    document: &Document,
    frame: FrameId,
    rows: &TableRows,
) -> Result<Applied, EditError> {
    let Some(FrameContent::Table(table)) = document.frame(frame).map(|frame| &frame.content) else {
        return Err("the frame holds no table");
    };
    if rows
        .headings
        .iter()
        .chain(&rows.page_breaks)
        .any(|row| *row >= table.rows.len())
    {
        return Err("a heading or page break for a row the table does not have");
    }
    if table.row_options == *rows {
        return Ok(unchanged(document));
    }
    let mut next = document.clone();
    if let Some(FrameContent::Table(table)) = next.frame_mut(frame).map(|frame| &mut frame.content)
    {
        table.row_options = rows.clone();
    }
    let mut report = ChangeReport::default();
    report.frames.insert(frame);
    changed(next, report)
}
