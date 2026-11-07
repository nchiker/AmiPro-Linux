//! Frame selection and frame gestures on the page surface (`UI_SPEC.md` §11):
//! what a frame selection holds, which handles a frame has, and the commands
//! that a move, a resize, or a key press makes. The surface publishes
//! [`FrameAction`]s; the application runs their commands through the
//! session's history as one undo step. Geometry is page twips, as drawn: a
//! Repeat right/left frame on a left page is mirrored, and its commands are
//! mirrored back.

use amipro_document::{
    CellContent, Document, FlowId, FrameContent, FrameId, GroupId, PageSide, Placement, Rect,
    RepeatPages, Twips,
};
use amipro_editing::{Command, TextPosition};
use amipro_layout::{Bounds, Container, DocumentLayout, PageBox};

use crate::page_editing::PageSelection;

/// An arrow key moves a frame by 1 pt.
pub const STEP: Twips = 20;
/// Shift with an arrow key moves it by 0.10 in.
pub const LARGE_STEP: Twips = 144;
/// How far an arrow key moves a picture in its frame in picture mode, and
/// with Ctrl (Ami Pro's guide, ch. 19; PI8: 72 and 720 twips).
pub const PICTURE_STEP: Twips = 72;
pub const PICTURE_LARGE_STEP: Twips = 720;
/// A frame stays at least its interior margins plus 0.10 in.
pub const MINIMUM_INTERIOR: Twips = 144;

/// One frame, one group, or several Where placed frames on one page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameSelection {
    /// The frames picked, in the order picked. A group member stands for its
    /// group.
    pub frames: Vec<FrameId>,
    /// The 1-based page they were picked on (repeating frames are on many).
    pub page: u32,
}

impl FrameSelection {
    pub fn one(frame: FrameId, page: u32) -> Self {
        Self {
            frames: vec![frame],
            page,
        }
    }

    /// The frames that act together: each picked frame's group, in stack
    /// order, once each.
    pub fn members(&self, document: &Document) -> Vec<FrameId> {
        let mut members = Vec::new();
        for frame in &self.frames {
            for member in group_of(document, *frame) {
                if !members.contains(&member) {
                    members.push(member);
                }
            }
        }
        members
    }

    /// Shift+click on `frame`: Where placed frames on the selection's page
    /// add to it, and a picked frame leaves it unless it is the last one.
    /// Any other frame starts a new selection.
    pub fn toggled(&self, document: &Document, frame: FrameId) -> Self {
        if let Some(index) = self.frames.iter().position(|picked| *picked == frame) {
            let mut next = self.clone();
            if next.frames.len() > 1 {
                next.frames.remove(index);
            }
            return next;
        }
        let fixed_here = |id: FrameId| {
            matches!(
                document.frame(id).map(|frame| frame.placement),
                Some(Placement::Fixed { page, .. }) if page == self.page
            )
        };
        if fixed_here(frame) && self.frames.iter().all(|picked| fixed_here(*picked)) {
            let mut next = self.clone();
            next.frames.push(frame);
            next
        } else {
            Self::one(frame, self.page)
        }
    }

    /// The group when the selection is exactly one group.
    pub fn group(&self, document: &Document) -> Option<GroupId> {
        let mut groups = self
            .frames
            .iter()
            .map(|id| document.frame(*id).and_then(|frame| frame.group));
        let first = groups.next().flatten()?;
        groups.all(|group| group == Some(first)).then_some(first)
    }

    /// The frames to group: two or more ungrouped Where placed frames on
    /// the selection's page (Ami Pro groups only those).
    pub fn groupable(&self, document: &Document) -> Option<Vec<FrameId>> {
        let members = self.members(document);
        let ungrouped_here = |id: &FrameId| {
            document.frame(*id).is_some_and(|frame| {
                frame.group.is_none()
                    && matches!(frame.placement, Placement::Fixed { page, .. } if page == self.page)
            })
        };
        (members.len() >= 2 && members.iter().all(ungrouped_here)).then_some(members)
    }

    /// The one frame a resize or the inspector acts on: a single ungrouped
    /// frame.
    pub fn single(&self, document: &Document) -> Option<FrameId> {
        match self.members(document).as_slice() {
            [frame] => Some(*frame),
            _ => None,
        }
    }

    /// The selection with deleted frames dropped (after undo, redo, or an
    /// edit that removed an anchor), or `None` when nothing is left.
    pub fn valid(&self, document: &Document) -> Option<Self> {
        let frames: Vec<FrameId> = self
            .frames
            .iter()
            .copied()
            .filter(|frame| selectable(document, *frame))
            .collect();
        (!frames.is_empty()).then_some(Self {
            frames,
            page: self.page,
        })
    }
}

/// `frame` and the other members of its group, in stack order.
fn group_of(document: &Document, frame: FrameId) -> Vec<FrameId> {
    match document.frame(frame).and_then(|frame| frame.group) {
        Some(group) => document
            .frame_stack()
            .iter()
            .copied()
            .filter(|id| document.frame(*id).is_some_and(|f| f.group == Some(group)))
            .collect(),
        None => vec![frame],
    }
}

/// Whether the user selects `frame` as a frame: page frames and repeating
/// frames. Header and footer frames belong to their page layout; clicks
/// there enter their text.
pub fn selectable(document: &Document, frame: FrameId) -> bool {
    document.frame(frame).is_some_and(|frame| {
        // A click on a cell's picture puts the caret in the cell, as in Ami
        // Pro (T1, VM TF5).
        !matches!(
            frame.placement,
            Placement::Header { .. } | Placement::Footer { .. } | Placement::InCell
        )
    })
}

/// The eight handles of a selected frame.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Handle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

impl Handle {
    pub const ALL: [Self; 8] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Right,
        Self::BottomRight,
        Self::Bottom,
        Self::BottomLeft,
        Self::Left,
    ];

    /// The edges it moves: left, top, right, bottom.
    pub fn edges(self) -> [bool; 4] {
        match self {
            Self::TopLeft => [true, true, false, false],
            Self::Top => [false, true, false, false],
            Self::TopRight => [false, true, true, false],
            Self::Right => [false, false, true, false],
            Self::BottomRight => [false, false, true, true],
            Self::Bottom => [false, false, false, true],
            Self::BottomLeft => [true, false, false, true],
            Self::Left => [true, false, false, false],
        }
    }

    pub fn is_corner(self) -> bool {
        matches!(
            self,
            Self::TopLeft | Self::TopRight | Self::BottomRight | Self::BottomLeft
        )
    }

    /// Its centre on `bounds`.
    pub fn centre(self, bounds: Bounds) -> (f32, f32) {
        let x = match self {
            Self::TopLeft | Self::Left | Self::BottomLeft => bounds.x,
            Self::Top | Self::Bottom => bounds.x + bounds.width / 2.0,
            Self::TopRight | Self::Right | Self::BottomRight => bounds.right(),
        };
        let y = match self {
            Self::TopLeft | Self::Top | Self::TopRight => bounds.y,
            Self::Left | Self::Right => bounds.y + bounds.height / 2.0,
            Self::BottomLeft | Self::Bottom | Self::BottomRight => bounds.bottom(),
        };
        (x, y)
    }
}

/// The handles that resize a frame placed so: all of them for Where placed
/// and repeating frames; for anchored frames those that keep the edge the
/// anchor sets (the top of a With para above frame, the left of a Flow with
/// text frame), so a handle never promises a move the placement forbids.
pub fn resize_handles(placement: Placement) -> Vec<Handle> {
    let anchored_edge = match placement {
        Placement::Fixed { .. } | Placement::Repeating { .. } => None,
        Placement::BelowParagraph { .. } => Some(1),
        Placement::Inline { .. } => Some(0),
        // A frame in a cell takes its cell's size (§24).
        Placement::Header { .. } | Placement::Footer { .. } | Placement::InCell => {
            return Vec::new();
        }
    };
    Handle::ALL
        .into_iter()
        .filter(|handle| anchored_edge.is_none_or(|edge| !handle.edges()[edge]))
        .collect()
}

/// The part of a move a placement allows: With para above frames move only
/// horizontally, Flow with text frames only vertically.
pub fn constrain(placement: Placement, dx: Twips, dy: Twips) -> (Twips, Twips) {
    match placement {
        Placement::BelowParagraph { .. } => (dx, 0),
        Placement::Inline { .. } => (0, dy),
        Placement::Header { .. } | Placement::Footer { .. } | Placement::InCell => (0, 0),
        Placement::Fixed { .. } | Placement::Repeating { .. } => (dx, dy),
    }
}

/// The smallest size of `frame`: its interior margins plus 0.10 in.
pub fn minimum_size(document: &Document, frame: FrameId) -> (Twips, Twips) {
    document
        .frame(frame)
        .map_or((MINIMUM_INTERIOR, MINIMUM_INTERIOR), |frame| {
            let margins = frame.style.interior.margins;
            (
                margins.left + margins.right + MINIMUM_INTERIOR,
                margins.top + margins.bottom + MINIMUM_INTERIOR,
            )
        })
}

/// `rect` after dragging `handle` by `dx` and `dy`. Corner handles move two
/// edges; with `keep_aspect` they keep the width-to-height ratio, the larger
/// change winning. No edge passes the opposite one less `minimum`.
pub fn resized(
    rect: Rect,
    handle: Handle,
    dx: Twips,
    dy: Twips,
    keep_aspect: bool,
    minimum: (Twips, Twips),
) -> Rect {
    let [left, top, right, bottom] = handle.edges();
    let mut next = rect;
    if left {
        next.left = (rect.left + dx).min(rect.right - minimum.0);
    }
    if right {
        next.right = (rect.right + dx).max(rect.left + minimum.0);
    }
    if top {
        next.top = (rect.top + dy).min(rect.bottom - minimum.1);
    }
    if bottom {
        next.bottom = (rect.bottom + dy).max(rect.top + minimum.1);
    }
    if keep_aspect && handle.is_corner() && rect.width() > 0 && rect.height() > 0 {
        let ratio = f64::from(rect.width()) / f64::from(rect.height());
        let (width, height) = (f64::from(next.width()), f64::from(next.height()));
        let width_change = (width / f64::from(rect.width()) - 1.0).abs();
        let height_change = (height / f64::from(rect.height()) - 1.0).abs();
        let (width, height) = if width_change >= height_change {
            (width, width / ratio)
        } else {
            (height * ratio, height)
        };
        let width = (width.round() as Twips).max(minimum.0);
        let height = (height.round() as Twips).max(minimum.1);
        if left {
            next.left = next.right - width;
        } else {
            next.right = next.left + width;
        }
        if top {
            next.top = next.bottom - height;
        } else {
            next.bottom = next.top + height;
        }
    }
    next
}

/// The drawn rectangle as page twips.
pub fn rect_of(bounds: Bounds) -> Rect {
    Rect::new(
        bounds.x.round() as Twips,
        bounds.y.round() as Twips,
        bounds.right().round() as Twips,
        bounds.bottom().round() as Twips,
    )
}

/// Whether a Repeat right/left frame is drawn mirrored on `page`.
fn mirrored(placement: Placement, page: &PageBox) -> bool {
    matches!(
        placement,
        Placement::Repeating {
            pages: RepeatPages::RightLeft,
            ..
        }
    ) && page.side == PageSide::Left
}

/// The commands that move the selection by `dx` and `dy` as drawn on
/// `page`: one `MoveFrame` per group or frame, with each placement's
/// constraint. Empty when nothing would move.
pub fn move_commands(
    document: &Document,
    selection: &FrameSelection,
    page: &PageBox,
    dx: Twips,
    dy: Twips,
) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut moved_groups = Vec::new();
    for frame_id in selection.members(document) {
        let Some(frame) = document.frame(frame_id) else {
            continue;
        };
        if let Some(group) = frame.group {
            if moved_groups.contains(&group) {
                continue;
            }
            moved_groups.push(group);
        }
        let (mut dx, dy) = constrain(frame.placement, dx, dy);
        if mirrored(frame.placement, page) {
            dx = -dx;
        }
        if dx != 0 || dy != 0 {
            commands.push(Command::MoveFrame {
                frame: frame_id,
                dx,
                dy,
            });
        }
    }
    commands
}

/// The commands that change `frame` from the drawn rectangle `from` to `to`
/// on `page`. Where placed and repeating frames take the rectangle; an
/// anchored frame moves along its free axis first when its free edge moved,
/// then takes the size. Empty when nothing changes.
pub fn resize_commands(
    document: &Document,
    frame: FrameId,
    page: &PageBox,
    from: Rect,
    to: Rect,
) -> Vec<Command> {
    let Some(placement) = document.frame(frame).map(|frame| frame.placement) else {
        return Vec::new();
    };
    if from == to {
        return Vec::new();
    }
    match placement {
        Placement::Fixed { .. } => vec![Command::ResizeFrame { frame, rect: to }],
        Placement::Repeating { rect, .. } => {
            let rect = if mirrored(placement, page) {
                Rect::new(
                    rect.left - (to.right - from.right),
                    to.top,
                    rect.right - (to.left - from.left),
                    to.bottom,
                )
            } else {
                to
            };
            vec![Command::ResizeFrame { frame, rect }]
        }
        Placement::BelowParagraph { .. } | Placement::Inline { .. } => {
            let (dx, dy) = constrain(placement, to.left - from.left, to.top - from.top);
            let mut commands = Vec::new();
            if dx != 0 || dy != 0 {
                commands.push(Command::MoveFrame { frame, dx, dy });
            }
            if to.width() != from.width() || to.height() != from.height() {
                commands.push(Command::ResizeFrame { frame, rect: to });
            }
            commands
        }
        Placement::Header { .. } | Placement::Footer { .. } | Placement::InCell => Vec::new(),
    }
}

/// The front-most selectable frame on `page` under `x`, `y` (page twips).
pub fn frame_at(document: &Document, page: &PageBox, x: f32, y: f32) -> Option<FrameId> {
    page.frames
        .iter()
        .rev()
        .find(|frame| {
            x >= frame.bounds.x
                && x <= frame.bounds.right()
                && y >= frame.bounds.y
                && y <= frame.bounds.bottom()
                && selectable(document, frame.frame)
        })
        .map(|frame| frame.frame)
}

/// The next selectable frame on `page` after `current`, in stack order
/// (back to front, wrapping), or the previous one with `backwards`.
pub fn next_frame(
    document: &Document,
    page: &PageBox,
    current: Option<FrameId>,
    backwards: bool,
) -> Option<FrameId> {
    let mut order: Vec<FrameId> = Vec::new();
    for frame in &page.frames {
        if selectable(document, frame.frame) && !order.contains(&frame.frame) {
            order.push(frame.frame);
        }
    }
    if order.is_empty() {
        return None;
    }
    let count = order.len();
    let index = match current.and_then(|current| order.iter().position(|id| *id == current)) {
        Some(index) if backwards => (index + count - 1) % count,
        Some(index) => (index + 1) % count,
        None if backwards => count - 1,
        None => 0,
    };
    Some(order[index])
}

/// The page frame whose text, or one of whose cells, `flow` is: the frame
/// that Esc from frame text selects.
pub fn page_frame_of_flow(document: &Document, flow: FlowId) -> Option<FrameId> {
    document
        .frames()
        .find(|frame| match &frame.content {
            FrameContent::Text(text) => *text == flow,
            FrameContent::Table(table) => table.cells.iter().any(
                |cell| matches!(cell.content, CellContent::Text(cell_flow) if cell_flow == flow),
            ),
            _ => false,
        })
        .map(|frame| frame.id)
        .filter(|frame| selectable(document, *frame))
}

/// Where Enter puts the caret in a selected frame: the start of its text, or
/// of its first cell's. Pictures have no text.
pub fn text_entry(document: &Document, frame: FrameId) -> Option<TextPosition> {
    let flow = match &document.frame(frame)?.content {
        FrameContent::Text(flow) => *flow,
        // The first cell is a grid's top-left, never covered by a join.
        FrameContent::Table(table) => {
            let CellContent::Text(flow) = table.cells.first()?.content;
            flow
        }
        _ => return None,
    };
    Some(TextPosition {
        flow,
        paragraph: 0,
        byte: 0,
    })
}

/// Where the caret goes when a frame selection is left for the main text:
/// an anchored frame's anchor, else the first main-text line on its page.
pub fn text_after_frames(
    document: &Document,
    layout: &DocumentLayout,
    selection: &FrameSelection,
) -> PageSelection {
    let main = document.main_flow_id();
    if let Some(frame) = selection.frames.first() {
        for (index, paragraph) in document.paragraphs().iter().enumerate() {
            let anchor = paragraph
                .object_offsets()
                .zip(&paragraph.objects)
                .find(|(_, object)| {
                    matches!(object, amipro_document::InlineObject::FrameAnchor(id) if id == frame)
                });
            if let Some((byte, _)) = anchor {
                let page = layout.page_of(index, byte).unwrap_or(selection.page);
                return PageSelection::caret(
                    TextPosition {
                        flow: main,
                        paragraph: index,
                        byte,
                    },
                    page,
                );
            }
        }
    }
    layout
        .pages
        .iter()
        .find(|page| page.number == selection.page)
        .and_then(|page| {
            page.lines
                .iter()
                .find(|line| line.container == Container::Main)
        })
        .map_or_else(
            || PageSelection::start(document),
            |line| {
                PageSelection::caret(
                    TextPosition {
                        flow: main,
                        paragraph: line.paragraph_index,
                        byte: line.bytes.start,
                    },
                    selection.page,
                )
            },
        )
}

/// What the page surface asks the application to do with frames.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameAction {
    /// Select the frame under the pointer; with `extend` (Shift), add it to
    /// the selection or take it out.
    Select {
        frame: FrameId,
        page: u32,
        extend: bool,
    },
    /// Move the selection by twips as drawn: a drag's release, or an arrow
    /// key (`key`: presses within a second form one undo step).
    Move { dx: Twips, dy: Twips, key: bool },
    /// Give the selected frame the drawn rectangle `to` (a handle's drag);
    /// `with_picture`: Shift and a corner handle on a picture frame size the
    /// picture with it (Custom; Ami Pro's guide, ch. 19; P6).
    Resize {
        from: Rect,
        to: Rect,
        with_picture: bool,
    },
    /// Picture mode (`UI_SPEC.md` §25, P6): move the selected frame's
    /// picture in it by twips (a drag's release, or an arrow key: `key`,
    /// presses within a second form one undo step).
    Pan { dx: Twips, dy: Twips, key: bool },
    /// Picture mode ends (Esc); the frame stays selected.
    LeavePicture,
    /// Move the selected frame's right and bottom edges (Alt+arrow keys).
    ResizeBy { dx: Twips, dy: Twips },
    /// Select the next or previous frame on the page (Tab, Shift+Tab).
    Cycle { backwards: bool },
    /// Put the caret in the selected frame's text: at a double-click's
    /// position, or at its start (Enter).
    EnterText { at: Option<(TextPosition, u32)> },
    /// Delete the selected frames, after confirmation (Delete).
    Delete,
    /// Back to the main text (Esc).
    Deselect,
}

#[cfg(test)]
mod tests {
    use amipro_document::{FrameStyle, Margins, Size as FrameSize};
    use amipro_editing::{History, apply};

    use super::*;

    fn bounds(rect: Rect) -> Bounds {
        Bounds {
            x: rect.left as f32,
            y: rect.top as f32,
            width: rect.width() as f32,
            height: rect.height() as f32,
        }
    }

    fn page(document: &Document, side: PageSide, frames: &[FrameId]) -> PageBox {
        PageBox {
            number: 1,
            shown: 1,
            layout: document.document_layout().id,
            side,
            width: 12_240.0,
            height: 15_840.0,
            columns: Vec::new(),
            strokes: Vec::new(),
            footnote_separator: None,
            frames: frames
                .iter()
                .map(|id| {
                    let rect = match document.frame(*id).unwrap().placement {
                        Placement::Fixed { rect, .. } | Placement::Repeating { rect, .. } => rect,
                        _ => Rect::new(1_440, 1_440, 2_880, 2_880),
                    };
                    amipro_layout::FrameBox {
                        frame: *id,
                        bounds: bounds(rect),
                        interior: bounds(rect),
                        strokes: Vec::new(),
                        layer: amipro_layout::Layer::Page,
                        kind: amipro_layout::FrameBoxKind::Text,
                    }
                })
                .collect(),
            lines: Vec::new(),
        }
    }

    fn fixed(document: &mut Document, rect: Rect) -> FrameId {
        let frame =
            document.create_text_frame(Placement::Fixed { page: 1, rect }, FrameStyle::default());
        document.frame_stack_mut().push(frame);
        frame
    }

    fn placement(document: &Document, frame: FrameId) -> Placement {
        document.frame(frame).unwrap().placement
    }

    #[test]
    fn shift_clicks_gather_where_placed_frames_on_one_page() {
        let mut document = Document::from_plain_text("Text");
        let a = fixed(&mut document, Rect::new(1_440, 1_440, 2_880, 2_880));
        let b = fixed(&mut document, Rect::new(4_320, 1_440, 5_760, 2_880));
        let selection = FrameSelection::one(a, 1).toggled(&document, b);
        assert_eq!(selection.frames, [a, b]);
        assert_eq!(selection.groupable(&document), Some(vec![a, b]));
        assert_eq!(selection.single(&document), None);
        // Shift+click on a picked frame drops it; the last one stays.
        let one = selection.toggled(&document, a);
        assert_eq!(one.frames, [b]);
        assert_eq!(one.toggled(&document, b).frames, [b]);

        let grouped = apply(&document, &Command::Group { frames: vec![a, b] })
            .unwrap()
            .document;
        let group = FrameSelection::one(a, 1);
        assert_eq!(group.members(&grouped), [a, b]);
        assert!(group.group(&grouped).is_some());
        assert_eq!(group.groupable(&grouped), None);
        // Deleted frames leave the selection.
        let deleted = apply(&grouped, &Command::DeleteFrame { frame: a })
            .unwrap()
            .document;
        assert_eq!(group.valid(&deleted), None);
    }

    #[test]
    fn moves_keep_each_placement_constraint_and_move_groups_once() {
        let mut document = Document::from_plain_text("Text");
        let a = fixed(&mut document, Rect::new(1_440, 1_440, 2_880, 2_880));
        let b = fixed(&mut document, Rect::new(4_320, 1_440, 5_760, 2_880));
        let document = apply(&document, &Command::Group { frames: vec![a, b] })
            .unwrap()
            .document;
        let right = page(&document, PageSide::Right, &[a, b]);
        // Both members picked: the group moves once.
        let selection = FrameSelection {
            frames: vec![a, b],
            page: 1,
        };
        let commands = move_commands(&document, &selection, &right, 20, -40);
        assert_eq!(
            commands,
            [Command::MoveFrame {
                frame: a,
                dx: 20,
                dy: -40
            }]
        );
        let moved = apply(&document, &commands[0]).unwrap().document;
        assert_eq!(
            placement(&moved, b),
            Placement::Fixed {
                page: 1,
                rect: Rect::new(4_340, 1_400, 5_780, 2_840)
            }
        );

        assert_eq!(
            constrain(
                Placement::BelowParagraph {
                    x_offset: 0,
                    size: FrameSize {
                        width: 1,
                        height: 1
                    }
                },
                30,
                40
            ),
            (30, 0)
        );
        assert_eq!(
            constrain(
                Placement::Inline {
                    y_offset: 0,
                    size: FrameSize {
                        width: 1,
                        height: 1
                    }
                },
                30,
                40
            ),
            (0, 40)
        );
    }

    #[test]
    fn mirrored_repeating_frames_move_and_resize_back_to_their_rectangle() {
        let mut document = Document::from_plain_text("Text");
        let rect = Rect::new(1_440, 1_440, 2_880, 2_880);
        let frame = document.create_text_frame(
            Placement::Repeating {
                pages: RepeatPages::RightLeft,
                rect,
            },
            FrameStyle::default(),
        );
        let layout = document.document_layout().id;
        document.layout_mut(layout).unwrap().repeating.push(frame);
        document.validate().unwrap();
        let mut left = page(&document, PageSide::Left, &[frame]);
        // Drawn mirrored: x from the right edge.
        let drawn = Rect::new(12_240 - 2_880, 1_440, 12_240 - 1_440, 2_880);
        left.frames[0].bounds = bounds(drawn);
        let selection = FrameSelection::one(frame, 2);
        assert_eq!(
            move_commands(&document, &selection, &left, 100, 0),
            [Command::MoveFrame {
                frame,
                dx: -100,
                dy: 0
            }]
        );
        // Dragging the drawn right edge outwards moves the stored left edge.
        let to = resized(drawn, Handle::Right, 200, 0, false, (288, 288));
        assert_eq!(
            resize_commands(&document, frame, &left, drawn, to),
            [Command::ResizeFrame {
                frame,
                rect: Rect::new(1_240, 1_440, 2_880, 2_880)
            }]
        );
    }

    #[test]
    fn resizing_keeps_a_minimum_the_aspect_and_anchored_edges() {
        let rect = Rect::new(1_000, 1_000, 3_000, 2_000);
        assert_eq!(
            resized(rect, Handle::Left, 5_000, 0, false, (432, 432)),
            Rect::new(2_568, 1_000, 3_000, 2_000),
            "the left edge stops short of the right"
        );
        assert_eq!(
            resized(rect, Handle::BottomRight, 1_000, 100, true, (432, 432)),
            Rect::new(1_000, 1_000, 4_000, 2_500),
            "the larger change wins and the ratio stays 2:1"
        );
        assert_eq!(
            resized(rect, Handle::TopLeft, -200, 0, true, (432, 432)),
            Rect::new(800, 900, 3_000, 2_000)
        );
        assert_eq!(
            resized(rect, Handle::Top, 30, 30, true, (432, 432)),
            Rect::new(1_000, 1_030, 3_000, 2_000),
            "edge handles ignore the other axis and the ratio"
        );

        let size = FrameSize {
            width: 2_000,
            height: 1_000,
        };
        let below = Placement::BelowParagraph { x_offset: 0, size };
        let inline = Placement::Inline { y_offset: 0, size };
        assert!(!resize_handles(below).iter().any(|handle| handle.edges()[1]));
        assert!(
            !resize_handles(inline)
                .iter()
                .any(|handle| handle.edges()[0])
        );
        assert_eq!(
            resize_handles(Placement::Fixed {
                page: 1,
                rect: Rect::new(0, 0, 1, 1)
            })
            .len(),
            8
        );
    }

    #[test]
    fn an_anchored_frame_resized_from_its_free_edge_moves_and_resizes_in_one_step() {
        let mut document = Document::from_plain_text("Anchor here");
        let main = document.main_flow_id();
        let created = apply(
            &document,
            &Command::CreateFrame {
                placement: Placement::BelowParagraph {
                    x_offset: 720,
                    size: FrameSize {
                        width: 2_880,
                        height: 1_440,
                    },
                },
                style: FrameStyle {
                    wrap: amipro_document::Wrap::NoWrapBeside,
                    ..FrameStyle::default()
                },
                content: amipro_editing::NewFrameContent::Text,
                anchor: Some(TextPosition {
                    flow: main,
                    paragraph: 0,
                    byte: 6,
                }),
                layout: None,
            },
        )
        .unwrap();
        document = created.document;
        let frame = *document.frame_stack().last().unwrap();
        let drawn = Rect::new(2_160, 3_000, 5_040, 4_440);
        let mut on_page = page(&document, PageSide::Right, &[frame]);
        on_page.frames[0].bounds = bounds(drawn);
        let to = resized(drawn, Handle::Left, -360, 0, false, (432, 432));
        let commands = resize_commands(&document, frame, &on_page, drawn, to);
        assert_eq!(
            commands,
            [
                Command::MoveFrame {
                    frame,
                    dx: -360,
                    dy: 0
                },
                Command::ResizeFrame { frame, rect: to }
            ]
        );
        let original = document.clone();
        let mut history = History::new();
        history.execute(&mut document, (), &commands[0]).unwrap();
        history.amend(&mut document, (), &commands[1]).unwrap();
        assert_eq!(
            placement(&document, frame),
            Placement::BelowParagraph {
                x_offset: 360,
                size: FrameSize {
                    width: 3_240,
                    height: 1_440
                }
            }
        );
        history.undo(&mut document, ()).unwrap();
        assert_eq!(document, original, "one undo step");
        assert!(resize_commands(&original, frame, &on_page, drawn, drawn).is_empty());
    }

    #[test]
    fn hits_tab_order_and_text_entry_follow_the_stack() {
        let mut document = Document::from_plain_text("Text");
        let back = fixed(&mut document, Rect::new(1_440, 1_440, 4_320, 4_320));
        let front = fixed(&mut document, Rect::new(2_880, 2_880, 5_760, 5_760));
        let header = document.document_layout().header.right.unwrap();
        let mut on_page = page(&document, PageSide::Right, &[back, front]);
        on_page.frames.insert(
            0,
            amipro_layout::FrameBox {
                frame: header,
                bounds: bounds(Rect::new(0, 0, 12_240, 1_440)),
                interior: bounds(Rect::new(0, 0, 12_240, 1_440)),
                strokes: Vec::new(),
                layer: amipro_layout::Layer::Layout,
                kind: amipro_layout::FrameBoxKind::Text,
            },
        );
        assert_eq!(frame_at(&document, &on_page, 3_000.0, 3_000.0), Some(front));
        assert_eq!(frame_at(&document, &on_page, 2_000.0, 2_000.0), Some(back));
        assert_eq!(
            frame_at(&document, &on_page, 100.0, 100.0),
            None,
            "header frames are not selected"
        );
        assert_eq!(next_frame(&document, &on_page, None, false), Some(back));
        assert_eq!(
            next_frame(&document, &on_page, Some(back), false),
            Some(front)
        );
        assert_eq!(
            next_frame(&document, &on_page, Some(front), false),
            Some(back)
        );
        assert_eq!(next_frame(&document, &on_page, None, true), Some(front));

        let FrameContent::Text(flow) = document.frame(front).unwrap().content else {
            unreachable!()
        };
        assert_eq!(
            text_entry(&document, front),
            Some(TextPosition {
                flow,
                paragraph: 0,
                byte: 0
            })
        );
        assert_eq!(page_frame_of_flow(&document, flow), Some(front));
        let FrameContent::Text(header_flow) = document.frame(header).unwrap().content else {
            unreachable!()
        };
        assert_eq!(page_frame_of_flow(&document, header_flow), None);
        assert_eq!(
            minimum_size(&document, front),
            (144 + 144 + 144, 144 + 144 + 144)
        );
        let _ = Margins::uniform(0);
    }
}
