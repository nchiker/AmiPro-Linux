//! The ruler's model (`docs/Design/UI_SPEC.md` §19, from Ami Pro's current
//! ruler as `docs/research/RULER_NOTES.md` maps it): what the ruler shows at
//! the caret (the box it edits, every column of it, the caret's column, the
//! tabs and indents in effect), what a pointer hits on it, how a drag snaps
//! and stops, and each gesture as one command. Pure: tested without the
//! widget. Positions are page twips (from the page's left edge).

use amipro_document::{
    Document, FlowId, FrameId, Indents, LayoutBox, LayoutId, PageSide, Placement, TabKind,
    TabLeader, TabStop, Twips,
};
use amipro_editing::layout_setup::{self, Band, Pages};
use amipro_editing::{Command, IndentChange, LayoutSettings};
use amipro_layout::{Container, DocumentLayout, FrameBoxKind, PageBox};

use crate::page_editing::PageSelection;

/// Ami Pro's snap in inches: a dragged marker's position is truncated to a
/// multiple of 1/16 in (`10d0:1a40`).
pub const SNAP: f32 = 90.0;

/// Whose margins and columns the ruler's lower half edits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RulerBox {
    /// A page of a page layout.
    Page { layout: LayoutId, pages: Pages },
    /// A header or footer band.
    Band {
        layout: LayoutId,
        band: Band,
        pages: Pages,
    },
    /// A text frame's interior.
    Frame(FrameId),
    /// A table's columns, from a cell (TD6): a column's right edge sizes
    /// that column, as Ami Pro's grid line does.
    Table(FrameId),
    /// A table cell or a floating header or footer: tabs and indents only.
    TextOnly,
}

/// Something on the ruler a pointer can take.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Marker {
    /// A tab stop, by its index in the stops in effect.
    Tab(usize),
    /// The first line's indent (the upper triangle).
    First,
    /// The other lines' indent (the lower triangle).
    Rest,
    /// Both together (the bar).
    All,
    /// The right indent.
    Right,
    /// A column's left or right edge; the first column's left edge is the
    /// left margin and the last column's right edge the right margin.
    Edge { column: usize, right: bool },
    /// A column, moved between its neighbours.
    Column(usize),
    /// The gap after a column, moved with both columns' edges.
    Gap(usize),
}

impl Marker {
    /// What hovering it says (with its position).
    pub fn name(self, columns: usize) -> &'static str {
        match self {
            Self::Tab(_) => "Tab",
            Self::First => "First line indent",
            Self::Rest => "Indent of the other lines",
            Self::All => "Indent",
            Self::Right => "Right indent",
            Self::Edge {
                column: 0,
                right: false,
            } => "Left margin",
            Self::Edge {
                column,
                right: true,
            } if column + 1 == columns => "Right margin",
            Self::Edge { right: false, .. } => "Column's left edge",
            Self::Edge { right: true, .. } => "Column's right edge",
            Self::Column(_) => "Column",
            Self::Gap(_) => "Space between columns",
        }
    }
}

/// Which half of the ruler a point is in, and how far down the upper half.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Half {
    /// Tabs and indents; `depth` from 0 (top) to 1 (the middle line).
    Upper { depth: f32 },
    /// Margins and columns.
    Lower,
}

/// What the ruler shows at the caret.
#[derive(Clone, Debug, PartialEq)]
pub struct Ruler {
    /// The page's index in the layout.
    pub page: usize,
    pub target: RulerBox,
    pub page_width: f32,
    /// The box's left edge and width (a page, a band across the page, or a
    /// frame inside its rounded corners' inset).
    pub box_left: f32,
    pub box_width: f32,
    /// Every column's left and right edge.
    pub columns: Vec<(f32, f32)>,
    pub caret_column: usize,
    /// The tab stops in effect, from the caret column's left edge.
    pub tabs: Vec<TabStop>,
    pub indents: Indents,
    /// The paragraphs a tab or indent change goes to.
    pub flow: FlowId,
    pub first: usize,
    pub last: usize,
    /// The last paragraph a tab change goes to: a selection that ends at
    /// the start of an empty paragraph includes it for tabs only
    /// (`1170:052d`).
    pub tab_last: usize,
}

/// Truncates a position to the ruler's grid (`10d0:1a40`).
pub fn snap(x: f32) -> f32 {
    if x <= 0.0 {
        0.0
    } else {
        (x / SNAP).floor() * SNAP
    }
}

fn pages_of(document: &Document, layout: LayoutId, side: PageSide) -> Pages {
    match document.layout(layout).map(|layout| layout.left.is_some()) {
        Some(true) if side == PageSide::Left => Pages::Left,
        Some(true) => Pages::Right,
        _ => Pages::All,
    }
}

/// The columns of `layout` inside a text area from `x`, `width` wide, as
/// the engine places them.
fn box_columns(layout: &LayoutBox, x: f32, width: f32) -> Vec<(f32, f32)> {
    match &layout.column_edges {
        Some(edges) => edges
            .iter()
            .map(|edges| (x + edges.left as f32, x + edges.right as f32))
            .collect(),
        None => {
            let count = f32::from(layout.columns.max(1));
            let gutter = layout.gutter as f32;
            let column = ((width - gutter * (count - 1.0)) / count).max(0.0);
            (0..layout.columns.max(1))
                .map(|index| {
                    let left = x + f32::from(index) * (column + gutter);
                    (left, left + column)
                })
                .collect()
        }
    }
}

impl Ruler {
    /// The ruler at the caret of `selection`, on the page the caret is drawn
    /// on.
    pub fn at(
        document: &Document,
        layout: &DocumentLayout,
        selection: PageSelection,
    ) -> Option<Self> {
        let focus = selection.focus;
        let number = crate::page_surface::caret_page(document, layout, selection);
        let page_index = layout.pages.iter().position(|page| page.number == number)?;
        let page = &layout.pages[page_index];
        let (start, end) = selection.ordered();
        // A selection that ends at the very start of a paragraph leaves it
        // out, and the ruler shows the paragraph before (`1148:1c21`).
        let flow = document.flow(focus.flow)?;
        let ends_at_start = end.paragraph > start.paragraph && end.byte == 0;
        let last = if ends_at_start {
            end.paragraph - 1
        } else {
            end.paragraph
        };
        let tab_last = if ends_at_start && flow.paragraph(end.paragraph)?.text.is_empty() {
            end.paragraph
        } else {
            last
        };
        let source = if ends_at_start && focus == end {
            last
        } else {
            focus.paragraph
        };
        let paragraph = flow.paragraph(source)?;
        let mut style = document.styles().get(&paragraph.style)?.clone();
        paragraph.format.apply(&mut style);
        let line = page
            .lines
            .iter()
            .find(|line| line.flow == focus.flow && line.paragraph_index == source)
            .or_else(|| {
                page.lines
                    .iter()
                    .find(|line| line.flow == focus.flow && line.paragraph_index == focus.paragraph)
            })
            // Hidden text (V14): the box's last visible line.
            .or_else(|| page.lines.iter().rfind(|line| line.flow == focus.flow));
        let container = line.map_or(Container::Main, |line| line.container);
        let (target, box_left, box_width, columns, box_tabs) =
            Self::box_of(document, page, container)?;
        let line_x = line.map_or(columns[0].0, |line| line.x);
        let caret_column = columns
            .iter()
            .position(|(left, right)| line_x >= left - 0.5 && line_x <= right + 0.5)
            .unwrap_or(0);
        Some(Self {
            page: page_index,
            target,
            page_width: page.width,
            box_left,
            box_width,
            columns,
            caret_column,
            tabs: style.tabs_in_effect(&box_tabs).to_vec(),
            indents: style.indents,
            flow: focus.flow,
            first: start.paragraph,
            last,
            tab_last,
        })
    }

    /// The box a line in `container` sits in: what the ruler edits, its
    /// left edge and width, its columns, and its tab list.
    #[allow(clippy::type_complexity)]
    fn box_of(
        document: &Document,
        page: &PageBox,
        container: Container,
    ) -> Option<(RulerBox, f32, f32, Vec<(f32, f32)>, Vec<TabStop>)> {
        let page_layout = document
            .layout(page.layout)
            .unwrap_or_else(|| document.document_layout());
        match container {
            Container::Main | Container::Floating(_) | Container::Footnote(_) => {
                let geometry = match (&page_layout.left, page.side) {
                    (Some(left), PageSide::Left) => left,
                    _ => &page_layout.right,
                };
                let columns = page
                    .columns
                    .iter()
                    .map(|column| (column.x, column.right()))
                    .collect();
                let target = if matches!(container, Container::Main) {
                    RulerBox::Page {
                        layout: page.layout,
                        pages: pages_of(document, page.layout, page.side),
                    }
                } else {
                    RulerBox::TextOnly
                };
                Some((
                    target,
                    0.0,
                    page.width,
                    columns,
                    geometry.layout.tab_stops().to_vec(),
                ))
            }
            Container::Frame(id) => {
                let frame = document.frame(id)?;
                let frame_box = page.frames.iter().find(|frame_box| frame_box.frame == id)?;
                let interior = &frame.style.interior;
                let margins = interior.margins;
                // Inside the rounded corners' inset: the box starts where the
                // left margin starts from.
                let left = frame_box.interior.x - margins.left as f32;
                let width = frame_box.interior.width + (margins.left + margins.right) as f32;
                let target = match frame.placement {
                    Placement::Header { side } | Placement::Footer { side } => RulerBox::Band {
                        layout: page.layout,
                        band: if matches!(frame.placement, Placement::Header { .. }) {
                            Band::Header
                        } else {
                            Band::Footer
                        },
                        pages: if page_layout.left.is_some() {
                            if side == PageSide::Left {
                                Pages::Left
                            } else {
                                Pages::Right
                            }
                        } else {
                            Pages::All
                        },
                    },
                    _ => RulerBox::Frame(id),
                };
                Some((
                    target,
                    left,
                    width,
                    box_columns(interior, frame_box.interior.x, frame_box.interior.width),
                    interior.tab_stops().to_vec(),
                ))
            }
            Container::Cell { frame, cell } => {
                // The part of the table on this page that shows the cell.
                let bounds = page
                    .frames
                    .iter()
                    .filter(|frame_box| frame_box.frame == frame)
                    .find_map(|frame_box| match &frame_box.kind {
                        FrameBoxKind::Table { cells, .. } => {
                            cells.get(cell).filter(|bounds| bounds.width > 0.0)
                        }
                        _ => None,
                    })?;
                let holder = document.frame(frame)?;
                let tabs = holder.style.interior.tab_stops().to_vec();
                // The table's columns where its part on this page stands,
                // in the room it has: its text column, or its frame (TD6).
                let part = page.frames.iter().find(|frame_box| {
                    frame_box.frame == frame
                        && matches!(&frame_box.kind, FrameBoxKind::Table { cells, .. }
                            if cells.get(cell).is_some_and(|bounds| bounds.width > 0.0))
                });
                if let (Some(part), amipro_document::FrameContent::Table(table)) =
                    (part, &holder.content)
                {
                    let mut x = part.interior.x + table.spacing.left as f32;
                    let mut columns = Vec::with_capacity(table.columns.len());
                    for (index, width) in table.columns.iter().enumerate() {
                        columns.push((x, x + *width as f32));
                        x += (*width + table.column_gutter(index)) as f32;
                    }
                    let (left, width) = match holder.placement {
                        Placement::Fixed { .. } => (part.interior.x, part.interior.width),
                        _ => page
                            .columns
                            .iter()
                            .find(|column| {
                                part.bounds.x >= column.x - 1.0 && part.bounds.x < column.right()
                            })
                            .map_or((0.0, page.width), |column| (column.x, column.width)),
                    };
                    return Some((RulerBox::Table(frame), left, width, columns, tabs));
                }
                Some((
                    RulerBox::TextOnly,
                    bounds.x,
                    bounds.width,
                    vec![(bounds.x, bounds.right())],
                    tabs,
                ))
            }
        }
    }

    /// The caret's column.
    pub fn column(&self) -> (f32, f32) {
        self.columns[self.caret_column.min(self.columns.len() - 1)]
    }

    /// Whether the columns' widths or gaps differ (by more than a twip).
    pub fn uneven(&self) -> bool {
        let widths: Vec<f32> = self.columns.iter().map(|(l, r)| r - l).collect();
        let gaps: Vec<f32> = self
            .columns
            .windows(2)
            .map(|pair| pair[1].0 - pair[0].1)
            .collect();
        let differ = |values: &[f32]| {
            values
                .first()
                .is_some_and(|first| values.iter().any(|value| (value - first).abs() > 1.0))
        };
        differ(&widths) || differ(&gaps)
    }

    /// Whether the lower half edits margins and columns here.
    pub fn edits_columns(&self) -> bool {
        self.target != RulerBox::TextOnly
    }

    /// Where a marker stands.
    pub fn position(&self, marker: Marker) -> f32 {
        let (left, right) = self.column();
        let indents = self.indents;
        match marker {
            Marker::Tab(index) => {
                left + self.tabs.get(index).map_or(0, |stop| stop.position) as f32
            }
            Marker::First => left + (indents.all + indents.first) as f32,
            Marker::Rest => left + (indents.all + indents.rest) as f32,
            Marker::All => left + indents.all as f32,
            Marker::Right => right - indents.right as f32,
            Marker::Edge { column, right } => {
                let (l, r) = self.columns[column];
                if right { r } else { l }
            }
            Marker::Column(column) => self.columns[column].0,
            Marker::Gap(column) => self.columns[column + 1].0,
        }
    }

    /// What is at `x` in `half`, within `tolerance` twips of a marker. In
    /// the upper half a tab comes before the indent markers, as in Ami
    /// Pro (`10d0:090c`), except where the first line's triangle and the
    /// bar are drawn, so that an indent marker standing on a tab can still
    /// be taken.
    pub fn hit(&self, x: f32, half: Half, tolerance: f32) -> Option<Marker> {
        let near = |marker: Marker| (self.position(marker) - x).abs() <= tolerance;
        match half {
            Half::Upper { depth } => {
                // The indent triangles: the first line's from the top, the
                // others' from the middle, the bar between them.
                let left_marker = if depth < 0.4 {
                    Marker::First
                } else if depth < 0.7 {
                    Marker::All
                } else {
                    Marker::Rest
                };
                let indent = [left_marker, Marker::First, Marker::Rest, Marker::All]
                    .into_iter()
                    .find(|marker| near(*marker))
                    .or_else(|| near(Marker::Right).then_some(Marker::Right));
                let tab = self.tab_near(x, tolerance);
                if depth < 0.7 {
                    indent.or(tab)
                } else {
                    tab.or(indent)
                }
            }
            Half::Lower if matches!(self.target, RulerBox::Table(_)) => (0..self.columns.len())
                .map(|column| Marker::Edge {
                    column,
                    right: true,
                })
                .find(|marker| near(*marker)),
            Half::Lower if self.edits_columns() => {
                for column in 0..self.columns.len() {
                    for right in [false, true] {
                        let marker = Marker::Edge { column, right };
                        if near(marker) {
                            return Some(marker);
                        }
                    }
                }
                for (column, (left, right)) in self.columns.iter().enumerate() {
                    if x > *left && x < *right {
                        return Some(Marker::Column(column));
                    }
                    if let Some((next, _)) = self.columns.get(column + 1)
                        && x >= *right
                        && x <= *next
                    {
                        return Some(Marker::Gap(column));
                    }
                }
                None
            }
            Half::Lower => None,
        }
    }

    /// The tab nearest `x` within `tolerance` twips: what a right-click
    /// in the upper half opens the menu of, wherever indent markers stand.
    pub fn tab_near(&self, x: f32, tolerance: f32) -> Option<Marker> {
        let distance = |marker: &Marker| (self.position(*marker) - x).abs();
        (0..self.tabs.len())
            .map(Marker::Tab)
            .filter(|marker| distance(marker) <= tolerance)
            .min_by(|a, b| distance(a).total_cmp(&distance(b)))
    }

    /// The range a marker moves in (`10d0:090c`, `10d0:17a2`).
    fn range(&self, marker: Marker) -> (f32, f32) {
        let (left, right) = self.column();
        let indents = self.indents;
        let all = left + indents.all as f32;
        let right_indent = right - indents.right as f32;
        let deepest = indents.first.max(indents.rest) as f32;
        let box_right = self.box_left + self.box_width;
        match marker {
            Marker::Tab(index) => {
                let leftmost = all + indents.first.min(indents.rest).min(0) as f32;
                let previous = index.checked_sub(1).map_or(leftmost, |previous| {
                    self.position(Marker::Tab(previous)) + 1.0
                });
                let next = self.tabs.get(index + 1).map_or(right_indent, |_| {
                    self.position(Marker::Tab(index + 1)) - 1.0
                });
                (previous.max(leftmost), next.min(right_indent))
            }
            Marker::First | Marker::Rest => (all, right_indent - 1.0),
            Marker::All => (left, right_indent - deepest - 1.0),
            Marker::Right => (all + deepest + 1.0, right),
            Marker::Edge {
                column,
                right: false,
            } => {
                let floor = column
                    .checked_sub(1)
                    .map_or(self.box_left, |previous| self.columns[previous].1);
                (floor, self.columns[column].1 - 1.0)
            }
            Marker::Edge {
                column,
                right: true,
            } if matches!(self.target, RulerBox::Table(_)) => {
                // Ami Pro: from the column's left edge to its edge and the
                // room the table has left (`1498:20b7`).
                let last = self.columns.last().map_or(0.0, |column| column.1);
                let room = (box_right - last).max(0.0);
                (self.columns[column].0 + 1.0, self.columns[column].1 + room)
            }
            Marker::Edge {
                column,
                right: true,
            } => {
                let ceiling = self
                    .columns
                    .get(column + 1)
                    .map_or(box_right, |next| next.0);
                (self.columns[column].0 + 1.0, ceiling)
            }
            Marker::Column(column) => {
                let (l, r) = self.columns[column];
                let floor = column
                    .checked_sub(1)
                    .map_or(self.box_left, |previous| self.columns[previous].1);
                let ceiling = self
                    .columns
                    .get(column + 1)
                    .map_or(box_right, |next| next.0);
                (floor, ceiling - (r - l))
            }
            Marker::Gap(column) => {
                let gap = self.columns[column + 1].0 - self.columns[column].1;
                (
                    self.columns[column].0 + 1.0 + gap,
                    self.columns[column + 1].1 - 1.0,
                )
            }
        }
    }

    /// Where a marker taken at `from` and dragged to `to` lands: its
    /// position moved by the pointer, truncated to the grid, kept within
    /// its range (`10d0:120d`).
    pub fn dragged(&self, marker: Marker, from: f32, to: f32) -> f32 {
        let (low, high) = self.range(marker);
        let moved = snap(self.position(marker) + to - from);
        if high < low {
            return self.position(marker);
        }
        moved.clamp(low, high)
    }

    /// The command a drag of `marker` to `at` (from [`Ruler::dragged`])
    /// makes, or `None` when nothing changes. `off` is a tab released off
    /// the ruler, which removes it (`10d0:1c0c`).
    pub fn release(
        &self,
        document: &Document,
        marker: Marker,
        at: f32,
        off: bool,
    ) -> Option<Command> {
        let (left, right) = self.column();
        let rounded = at.round() as Twips;
        match marker {
            Marker::Tab(index) => {
                let mut tabs = self.tabs.clone();
                if off {
                    tabs.remove(index);
                } else {
                    let position = (at - left).round() as Twips;
                    if tabs[index].position == position {
                        return None;
                    }
                    tabs[index].position = position;
                }
                Some(self.tabs_command(Some(tabs)))
            }
            Marker::First | Marker::Rest | Marker::All | Marker::Right => {
                let value = match marker {
                    Marker::Right => (right - at).round() as Twips,
                    Marker::All => (at - left).round() as Twips,
                    _ => (at - left).round() as Twips - self.indents.all,
                };
                let change = match marker {
                    Marker::First => IndentChange {
                        first_line: Some(value),
                        ..IndentChange::default()
                    },
                    Marker::Rest => IndentChange {
                        rest: Some(value),
                        ..IndentChange::default()
                    },
                    Marker::All => IndentChange {
                        all: Some(value),
                        ..IndentChange::default()
                    },
                    _ => IndentChange {
                        right: Some(value),
                        ..IndentChange::default()
                    },
                };
                if change.applied_to(self.indents) == self.indents {
                    return None;
                }
                Some(Command::SetParagraphIndents {
                    flow: self.flow,
                    first: self.first,
                    last: self.last,
                    indents: change,
                })
            }
            Marker::Edge {
                column,
                right: true,
            } if matches!(self.target, RulerBox::Table(_)) => {
                let RulerBox::Table(frame) = self.target else {
                    unreachable!()
                };
                let width = (at - self.columns[column].0).round() as Twips;
                let current = (self.columns[column].1 - self.columns[column].0).round() as Twips;
                (width != current && width >= 1).then_some(Command::EditTable {
                    frame,
                    edit: amipro_editing::tables::TableEdit::SetSizes {
                        rows: 0..0,
                        columns: column..column + 1,
                        sizes: amipro_editing::tables::TrackSizes {
                            width: Some(width),
                            ..Default::default()
                        },
                    },
                })
            }
            Marker::Edge { .. } | Marker::Column(_) | Marker::Gap(_) => {
                let mut columns: Vec<(Twips, Twips)> = self
                    .columns
                    .iter()
                    .map(|(l, r)| (l.round() as Twips, r.round() as Twips))
                    .collect();
                match marker {
                    Marker::Edge {
                        column,
                        right: false,
                    } => columns[column].0 = rounded,
                    Marker::Edge {
                        column,
                        right: true,
                    } => columns[column].1 = rounded,
                    Marker::Column(column) => {
                        let shift = rounded - columns[column].0;
                        columns[column].0 += shift;
                        columns[column].1 += shift;
                    }
                    Marker::Gap(column) => {
                        let shift = rounded - columns[column + 1].0;
                        columns[column].1 += shift;
                        columns[column + 1].0 += shift;
                    }
                    _ => {}
                }
                if columns.iter().zip(&self.columns).all(|(new, old)| {
                    new.0 == old.0.round() as Twips && new.1 == old.1.round() as Twips
                }) {
                    return None;
                }
                let origin = self.box_left.round() as Twips;
                let edges = columns
                    .iter()
                    .map(|(l, r)| (l - origin, r - origin))
                    .collect();
                self.columns_command(document, ColumnChange::Edges(edges))
            }
        }
    }

    /// A click in the upper half where no marker is: a new tab of `kind`
    /// and `leader` where clicked, not snapped, between the leftmost indent
    /// and the right indent, at most 22 (`1070:0935`; VM fixture RU1).
    pub fn add_tab(&self, x: f32, kind: TabKind, leader: TabLeader) -> Option<Command> {
        let (left, right) = self.column();
        let indents = self.indents;
        let leftmost = left + (indents.all + indents.first.min(indents.rest).min(0)) as f32;
        if x < leftmost
            || x > right - indents.right as f32
            || self.tabs.len() >= LayoutBox::MAX_TABS
        {
            return None;
        }
        let position = (x - left).round() as Twips;
        if self.tabs.iter().any(|stop| stop.position == position) {
            return None;
        }
        let mut tabs = self.tabs.clone();
        tabs.push(TabStop {
            kind,
            position,
            leader,
        });
        Some(self.tabs_command(Some(tabs)))
    }

    /// A tab's type and leader changed (its right-click menu).
    pub fn set_tab(&self, index: usize, kind: TabKind, leader: TabLeader) -> Option<Command> {
        let mut tabs = self.tabs.clone();
        let stop = tabs.get_mut(index)?;
        if (stop.kind, stop.leader) == (kind, leader) {
            return None;
        }
        stop.kind = kind;
        stop.leader = leader;
        Some(self.tabs_command(Some(tabs)))
    }

    /// One tab removed.
    pub fn remove_tab(&self, index: usize) -> Option<Command> {
        let mut tabs = self.tabs.clone();
        (index < tabs.len()).then(|| {
            tabs.remove(index);
            self.tabs_command(Some(tabs))
        })
    }

    /// Clear Tabs: a paragraph ruler without stops (`1148:1329`, 0xFFFD).
    pub fn clear_tabs(&self) -> Command {
        self.tabs_command(Some(Vec::new()))
    }

    /// Page › Ruler › Insert (`0x173`): the paragraphs take their own ruler,
    /// the stops in effect.
    pub fn insert_ruler(&self) -> Command {
        self.tabs_command(Some(self.tabs.clone()))
    }

    /// Remove Paragraph Ruler: back to the style's or the box's stops (Page ›
    /// Ruler › Remove, `1170:0245`).
    pub fn remove_ruler(&self) -> Command {
        self.tabs_command(None)
    }

    fn tabs_command(&self, tabs: Option<Vec<TabStop>>) -> Command {
        Command::SetParagraphTabs {
            flow: self.flow,
            first: self.first,
            last: self.tab_last,
            tabs,
        }
    }

    /// Columns ▸ n: even columns with a 240-twip gutter (Ami Pro's Cols,
    /// `1148:1937`).
    pub fn set_columns(&self, document: &Document, columns: u8) -> Option<Command> {
        self.columns_command(document, ColumnChange::Count(columns))
    }

    /// Even Columns.
    pub fn even_columns(&self, document: &Document) -> Option<Command> {
        self.columns_command(document, ColumnChange::Even)
    }

    /// One command that makes `change` to the box's layout, through the
    /// page layout's rules for a page or a band (Mirror included).
    fn columns_command(&self, document: &Document, change: ColumnChange) -> Option<Command> {
        let width = self.box_width.round() as Twips;
        let changed = |settings: &LayoutSettings, layout: LayoutId| {
            (settings != &LayoutSettings::of(document, layout)?).then(|| Command::ModifyLayout {
                layout,
                settings: settings.clone(),
            })
        };
        match self.target {
            RulerBox::Page { layout, pages } => {
                let mut settings = LayoutSettings::of(document, layout)?;
                match change {
                    ColumnChange::Edges(edges) => settings.set_page_edges(pages, &edges),
                    ColumnChange::Count(columns) => {
                        settings.set_ruler_columns(None, pages, columns)
                    }
                    ColumnChange::Even => settings.set_even_columns(pages),
                }
                .ok()?;
                changed(&settings, layout)
            }
            RulerBox::Band {
                layout,
                band,
                pages,
            } => {
                let mut settings = LayoutSettings::of(document, layout)?;
                match change {
                    ColumnChange::Edges(edges) => settings.set_band_edges(band, pages, &edges),
                    ColumnChange::Count(columns) => {
                        settings.set_ruler_columns(Some(band), pages, columns)
                    }
                    ColumnChange::Even => settings.set_band_even_columns(band, pages),
                }
                .ok()?;
                changed(&settings, layout)
            }
            RulerBox::Frame(frame) => {
                let current = &document.frame(frame)?.style;
                let mut style = current.clone();
                match change {
                    ColumnChange::Edges(edges) => {
                        layout_setup::set_box_edges(&mut style.interior, width, &edges).ok()?
                    }
                    ColumnChange::Count(columns) => {
                        layout_setup::ruler_columns(&mut style.interior, columns).ok()?
                    }
                    ColumnChange::Even => layout_setup::even_columns(&mut style.interior),
                }
                (style != *current).then_some(Command::SetFrameStyle { frame, style })
            }
            // A table's columns even within their total (TD7); no count.
            RulerBox::Table(frame) => match change {
                ColumnChange::Even => Some(Command::EditTable {
                    frame,
                    edit: amipro_editing::tables::TableEdit::Distribute {
                        rows: None,
                        columns: Some(0..self.columns.len()),
                    },
                }),
                _ => None,
            },
            RulerBox::TextOnly => None,
        }
    }
}

/// What a column gesture or menu item does to the box.
enum ColumnChange {
    /// Every column's edges, from the box's left edge.
    Edges(Vec<(Twips, Twips)>),
    /// The ruler's Columns ▸ n.
    Count(u8),
    Even,
}

#[cfg(test)]
mod tests {
    use amipro_document::{Document, ParagraphFormat, TabKind, TabLeader, TabStop};
    use amipro_editing::{Command, apply};

    use super::*;
    use crate::page_layout::SessionPages;

    /// RUB: four Body Text paragraphs on Ami Pro's Standard layout (Letter,
    /// one-inch margins, stops every half inch).
    fn rub() -> (Document, SessionPages) {
        let document = Document::from_plain_text(
            "First paragraph of the ruler base.\nSecond one.\nThird one.\nFourth one.",
        );
        let mut pages = SessionPages::default();
        pages.update(&document);
        (document, pages)
    }

    fn caret(document: &Document, paragraph: usize, byte: usize) -> PageSelection {
        PageSelection::caret(
            amipro_editing::TextPosition {
                flow: document.main_flow_id(),
                paragraph,
                byte,
            },
            1,
        )
    }

    fn ruler(document: &Document, pages: &SessionPages, selection: PageSelection) -> Ruler {
        Ruler::at(document, pages.layout(), selection).unwrap()
    }

    fn run(document: &Document, command: Command) -> Document {
        apply(document, &command).unwrap().document
    }

    #[test]
    fn the_ruler_shows_the_caret_paragraphs_box_tabs_and_indents() {
        let (document, pages) = rub();
        let ruler = ruler(&document, &pages, caret(&document, 1, 2));
        assert!(matches!(
            ruler.target,
            RulerBox::Page {
                pages: Pages::All,
                ..
            }
        ));
        assert_eq!(ruler.columns, [(1_440.0, 10_800.0)]);
        assert_eq!(ruler.tabs.len(), 12);
        assert_eq!(ruler.position(Marker::Tab(0)), 2_160.0);
        assert_eq!((ruler.first, ruler.last), (1, 1));
        assert_eq!(snap(3_228.0), 3_150.0);
    }

    #[test]
    fn a_click_adds_an_unsnapped_tab_to_a_copy_of_the_stops_ru1() {
        let (document, pages) = rub();
        let ruler = ruler(&document, &pages, caret(&document, 0, 0));
        // RU1: a click at 3.24 in from the margin.
        let command = ruler
            .add_tab(1_440.0 + 3_228.0, TabKind::Left, TabLeader::None)
            .unwrap();
        let ruled = run(&document, command);
        let tabs = ruled.paragraphs()[0].format.tabs.as_ref().unwrap();
        assert_eq!(tabs.len(), 13);
        assert!(tabs.contains(&TabStop::new(TabKind::Left, 3_228)));
        assert_eq!(ruled.paragraphs()[1].format.tabs, None);
        // Not past the right indent, and never a 23rd.
        assert!(
            ruler
                .add_tab(10_900.0, TabKind::Left, TabLeader::None)
                .is_none()
        );
    }

    #[test]
    fn a_tab_drag_snaps_and_off_the_ruler_removes_it_ru3_ru4() {
        let (document, pages) = rub();
        let ruler = ruler(&document, &pages, caret(&document, 0, 0));
        // RU3: the stop at 2.5 in on the ruler (2160 from the margin)
        // dragged right lands on the grid: 2700, between its neighbours.
        let tab = ruler
            .hit(3_600.0, Half::Upper { depth: 0.5 }, 60.0)
            .unwrap();
        assert_eq!(tab, Marker::Tab(2));
        let at = ruler.dragged(tab, 3_600.0, 3_600.0 + 570.0);
        assert_eq!(at, 4_140.0);
        let ruled = run(&document, ruler.release(&document, tab, at, false).unwrap());
        let positions: Vec<_> = ruled.paragraphs()[0]
            .format
            .tabs
            .as_ref()
            .unwrap()
            .iter()
            .map(|stop| stop.position)
            .collect();
        assert!(positions.contains(&2_700) && !positions.contains(&2_160));
        // Not past the next stop.
        assert_eq!(ruler.dragged(tab, 3_600.0, 9_000.0), 4_319.0);
        // RU4: the stop at 2880 dragged off the ruler is removed.
        let removed = run(
            &document,
            ruler
                .release(&document, Marker::Tab(3), 4_320.0, true)
                .unwrap(),
        );
        let tabs = removed.paragraphs()[0].format.tabs.as_ref().unwrap();
        assert_eq!(tabs.len(), 11);
        assert!(!tabs.iter().any(|stop| stop.position == 2_880));
    }

    #[test]
    fn indent_drags_set_one_value_ru5_to_ru8() {
        let (document, pages) = rub();
        let ruler = ruler(&document, &pages, caret(&document, 0, 0));
        let drag = |marker: Marker, by: f32| {
            let from = ruler.position(marker);
            let at = ruler.dragged(marker, from, from + by);
            let command = ruler.release(&document, marker, at, false).unwrap();
            run(&document, command).paragraphs()[0]
                .format
                .indents
                .unwrap()
        };
        let first = drag(Marker::First, 720.0);
        assert_eq!(
            (first.all, first.first, first.rest, first.right),
            (0, 720, 0, 0)
        );
        let rest = drag(Marker::Rest, 360.0);
        assert_eq!(
            (rest.all, rest.first, rest.rest, rest.right),
            (0, 0, 360, 0)
        );
        let all = drag(Marker::All, 720.0);
        assert_eq!((all.all, all.first, all.rest, all.right), (720, 0, 0, 0));
        // RU8: the right indent's position truncates to the grid.
        let right = drag(Marker::Right, -740.0);
        assert_eq!(right.right, 810);
        // The markers' places in the upper half.
        assert_eq!(
            ruler.hit(1_440.0, Half::Upper { depth: 0.1 }, 60.0),
            Some(Marker::First)
        );
        assert_eq!(
            ruler.hit(1_440.0, Half::Upper { depth: 0.9 }, 60.0),
            Some(Marker::Rest)
        );
        assert_eq!(
            ruler.hit(1_440.0, Half::Upper { depth: 0.55 }, 60.0),
            Some(Marker::All)
        );
        assert_eq!(
            ruler.hit(10_800.0, Half::Upper { depth: 0.9 }, 60.0),
            Some(Marker::Right)
        );
        // RU5's first line on the stop at 0.5 in: the triangle where it is
        // drawn, the tab below it (Ami Pro takes tabs first, `10d0:090c`),
        // and a right-click's menu is the tab's.
        let mut indented = document.clone();
        indented.paragraph_mut(0).format.indents = Some(first);
        let ruler = super::Ruler::at(&indented, pages.layout(), caret(&indented, 0, 0)).unwrap();
        assert_eq!(
            ruler.position(Marker::First),
            ruler.position(Marker::Tab(0))
        );
        assert_eq!(
            ruler.hit(2_160.0, Half::Upper { depth: 0.1 }, 60.0),
            Some(Marker::First)
        );
        assert_eq!(
            ruler.hit(2_160.0, Half::Upper { depth: 0.9 }, 60.0),
            Some(Marker::Tab(0))
        );
        assert_eq!(ruler.tab_near(2_170.0, 60.0), Some(Marker::Tab(0)));
        assert_eq!(ruler.tab_near(1_800.0, 60.0), None);
    }

    #[test]
    fn column_drags_write_the_edges_ami_pro_writes_ru2_ru9_ru10_ru11() {
        let (document, pages) = rub();
        let ruler = ruler(&document, &pages, caret(&document, 0, 0));
        let two = run(&document, ruler.set_columns(&document, 2).unwrap());
        let mut pages = SessionPages::default();
        pages.update(&two);
        let ruler = super::Ruler::at(&two, pages.layout(), caret(&two, 0, 0)).unwrap();
        assert_eq!(ruler.columns, [(1_440.0, 6_000.0), (6_240.0, 10_800.0)]);
        let edges = |document: &Document| {
            let layout = &document.document_layout().right.layout;
            (
                layout.margins.left,
                layout.edges(12_240 - layout.margins.left - layout.margins.right),
            )
        };
        // RU2: the gap dragged to 5 in.
        let gap = ruler.hit(6_100.0, Half::Lower, 30.0).unwrap();
        assert_eq!(gap, Marker::Gap(0));
        let at = ruler.dragged(gap, 6_100.0, 6_100.0 + 1_100.0);
        assert_eq!(at, 7_290.0);
        let ru2 = run(&two, ruler.release(&two, gap, at, false).unwrap());
        let (left, ru2_edges) = edges(&ru2);
        assert_eq!(left, 1_440);
        assert_eq!(
            ru2_edges
                .iter()
                .map(|e| (e.left, e.right))
                .collect::<Vec<_>>(),
            [(0, 5_610), (5_850, 9_360)]
        );
        // RU9: column 1's right edge dragged left to 5490.
        let edge = Marker::Edge {
            column: 0,
            right: true,
        };
        assert_eq!(ruler.hit(6_000.0, Half::Lower, 30.0), Some(edge));
        let at = ruler.dragged(edge, 6_000.0, 5_520.0);
        assert_eq!(at, 5_490.0);
        let ru9 = run(&two, ruler.release(&two, edge, at, false).unwrap());
        let mut pages = SessionPages::default();
        pages.update(&ru9);
        let ruler = super::Ruler::at(&ru9, pages.layout(), caret(&ru9, 0, 0)).unwrap();
        // RU10: the left margin dragged 0.25 in moves only the first edge.
        let margin = Marker::Edge {
            column: 0,
            right: false,
        };
        let at = ruler.dragged(margin, 1_440.0, 1_800.0);
        let ru10 = run(&ru9, ruler.release(&ru9, margin, at, false).unwrap());
        let (left, ru10_edges) = edges(&ru10);
        assert_eq!(left, 1_800);
        assert_eq!(
            ru10_edges
                .iter()
                .map(|e| (e.left + 1_800, e.right + 1_800))
                .collect::<Vec<_>>(),
            [(1_800, 5_490), (6_240, 10_800)]
        );
        // RU11: Columns 3 makes them even with a 240 gutter.
        let mut pages = SessionPages::default();
        pages.update(&ru10);
        let ruler = super::Ruler::at(&ru10, pages.layout(), caret(&ru10, 0, 0)).unwrap();
        let ru11 = run(&ru10, ruler.set_columns(&ru10, 3).unwrap());
        let (_, ru11_edges) = edges(&ru11);
        assert_eq!(
            ru11_edges
                .iter()
                .map(|e| (e.left + 1_800, e.right + 1_800))
                .collect::<Vec<_>>(),
            [(1_800, 4_640), (4_880, 7_720), (7_960, 10_800)]
        );
    }

    #[test]
    fn a_selection_takes_every_paragraph_it_covers_but_one_it_only_reaches() {
        let (document, pages) = rub();
        let flow = document.main_flow_id();
        let selection = PageSelection {
            anchor: amipro_editing::TextPosition {
                flow,
                paragraph: 0,
                byte: 3,
            },
            focus: amipro_editing::TextPosition {
                flow,
                paragraph: 2,
                byte: 0,
            },
            page: 1,
        };
        let ruler = ruler(&document, &pages, selection);
        assert_eq!((ruler.first, ruler.last, ruler.tab_last), (0, 1, 1));
        let ruled = run(&document, ruler.clear_tabs());
        let tabs: Vec<_> = ruled
            .paragraphs()
            .iter()
            .map(|paragraph| paragraph.format.tabs.clone())
            .collect();
        assert_eq!(tabs, [Some(Vec::new()), Some(Vec::new()), None, None]);
        // Remove Paragraph Ruler returns them to the layout's stops.
        let mut with_format = ruled.clone();
        with_format.paragraph_mut(3).format = ParagraphFormat::default();
        let removed = run(&with_format, ruler.remove_ruler());
        assert!(removed.paragraphs().iter().all(|p| p.format.tabs.is_none()));
    }

    #[test]
    fn a_tabs_menu_and_even_columns_change_one_thing_each() {
        let (document, pages) = rub();
        let ruler = ruler(&document, &pages, caret(&document, 0, 0));
        let dotted = run(
            &document,
            ruler.set_tab(0, TabKind::Right, TabLeader::Dots).unwrap(),
        );
        let tabs = dotted.paragraphs()[0].format.tabs.clone().unwrap();
        assert_eq!(
            tabs[0],
            TabStop {
                kind: TabKind::Right,
                position: 720,
                leader: TabLeader::Dots
            }
        );
        assert!(ruler.set_tab(0, TabKind::Left, TabLeader::None).is_none());
        let fewer = run(&document, ruler.remove_tab(11).unwrap());
        assert_eq!(
            fewer.paragraphs()[0].format.tabs.as_ref().unwrap().len(),
            11
        );
        assert!(ruler.remove_tab(12).is_none());
        assert_eq!(
            Marker::Edge {
                column: 0,
                right: false
            }
            .name(2),
            "Left margin"
        );
        assert_eq!(
            Marker::Edge {
                column: 1,
                right: true
            }
            .name(2),
            "Right margin"
        );
        assert_eq!(Marker::Gap(0).name(2), "Space between columns");
        // Even Columns on RU2's columns, and nothing to do on even ones.
        assert!(ruler.even_columns(&document).is_none());
        let mut uneven = document.clone();
        uneven.document_layout_mut().right.layout.set_edges(
            vec![
                amipro_document::ColumnEdges {
                    left: 0,
                    right: 5_610,
                },
                amipro_document::ColumnEdges {
                    left: 5_850,
                    right: 9_360,
                },
            ],
            9_360,
        );
        let mut pages = SessionPages::default();
        pages.update(&uneven);
        let ruler = super::Ruler::at(&uneven, pages.layout(), caret(&uneven, 0, 0)).unwrap();
        let even = run(&uneven, ruler.even_columns(&uneven).unwrap());
        assert_eq!(even.document_layout().right.layout.column_edges, None);
    }

    fn flow_caret(flow: FlowId, page: u32) -> PageSelection {
        PageSelection::caret(
            amipro_editing::TextPosition {
                flow,
                paragraph: 0,
                byte: 0,
            },
            page,
        )
    }

    #[test]
    fn in_a_frame_the_ruler_edits_its_interior_and_its_texts_tabs() {
        let document = crate::page_surface::page_fixture();
        let frame = document.frame_stack().last().copied().unwrap();
        let amipro_document::FrameContent::Text(flow) = document.frame(frame).unwrap().content
        else {
            unreachable!()
        };
        let mut pages = SessionPages::default();
        pages.update(&document);
        let ruler = Ruler::at(&document, pages.layout(), flow_caret(flow, 1)).unwrap();
        assert_eq!(ruler.target, RulerBox::Frame(frame));
        let margins = document.frame(frame).unwrap().style.interior.margins;
        assert_eq!(ruler.box_left, 6_480.0);
        assert_eq!(ruler.box_width, 3_600.0);
        assert_eq!(
            ruler.columns,
            [(
                6_480.0 + margins.left as f32,
                10_080.0 - margins.right as f32
            )]
        );
        // The right margin dragged in half an inch: the frame's interior.
        let edge = Marker::Edge {
            column: 0,
            right: true,
        };
        let from = ruler.position(edge);
        let at = ruler.dragged(edge, from, from - 720.0);
        let command = ruler.release(&document, edge, at, false).unwrap();
        assert!(matches!(command, Command::SetFrameStyle { .. }));
        let changed = run(&document, command);
        let interior = &changed.frame(frame).unwrap().style.interior;
        assert_eq!(interior.margins.left, margins.left);
        assert_eq!(interior.margins.right, (10_080.0 - at).round() as i32);
        // Two columns in the frame, and a tab in its text.
        let two = run(&document, ruler.set_columns(&document, 2).unwrap());
        assert_eq!(two.frame(frame).unwrap().style.interior.columns, 2);
        let (left, _) = ruler.column();
        let tabbed = run(
            &document,
            ruler
                .add_tab(left + 1_000.0, TabKind::Left, TabLeader::None)
                .unwrap(),
        );
        let tabs = tabbed.flow(flow).unwrap().paragraphs[0]
            .format
            .tabs
            .clone()
            .unwrap();
        assert_eq!(tabs, [TabStop::new(TabKind::Left, 1_000)]);
    }

    #[test]
    fn in_a_header_the_ruler_edits_the_band_through_the_page_layout() {
        let document = crate::page_surface::page_fixture();
        let header = document.document_layout().header.right.unwrap();
        let amipro_document::FrameContent::Text(flow) = document.frame(header).unwrap().content
        else {
            unreachable!()
        };
        let mut pages = SessionPages::default();
        pages.update(&document);
        let ruler = Ruler::at(&document, pages.layout(), flow_caret(flow, 1)).unwrap();
        assert!(matches!(
            ruler.target,
            RulerBox::Band {
                band: Band::Header,
                pages: Pages::All,
                ..
            }
        ));
        // The band's left margin dragged in a quarter inch.
        let edge = Marker::Edge {
            column: 0,
            right: false,
        };
        let from = ruler.position(edge);
        let at = ruler.dragged(edge, from, from + 360.0);
        let command = ruler.release(&document, edge, at, false).unwrap();
        assert!(matches!(command, Command::ModifyLayout { .. }));
        let changed = run(&document, command);
        let interior = &changed.frame(header).unwrap().style.interior;
        assert_eq!(interior.margins.left as f32, at);
        changed.validate().unwrap();
    }

    #[test]
    fn in_a_table_cell_the_ruler_has_the_tables_columns_and_the_cells_tabs() {
        let document = crate::page_surface::page_fixture();
        let created = apply(
            &document,
            &Command::CreateFrame {
                placement: amipro_document::Placement::Fixed {
                    page: 2,
                    rect: amipro_document::Rect::new(1_440, 2_000, 7_200, 3_000),
                },
                style: amipro_document::FrameStyle::default(),
                content: amipro_editing::NewFrameContent::Table {
                    columns: vec![2_000, 2_000],
                    rows: vec![400],
                },
                anchor: None,
                layout: None,
            },
        )
        .unwrap()
        .document;
        let table = created.frame_stack().last().copied().unwrap();
        let amipro_document::FrameContent::Table(table_data) =
            &created.frame(table).unwrap().content
        else {
            unreachable!()
        };
        let amipro_document::CellContent::Text(flow) = table_data.cells[1].content;
        let mut pages = SessionPages::default();
        pages.update(&created);
        let ruler = Ruler::at(&created, pages.layout(), flow_caret(flow, 2)).unwrap();
        // The table's columns, the caret's the second (TD6).
        assert_eq!(ruler.target, RulerBox::Table(table));
        assert_eq!(ruler.columns.len(), 2);
        assert_eq!(ruler.caret_column, 1);
        let (left, right) = ruler.column();
        assert!((right - left - 2_000.0).abs() < 1.0, "{left} {right}");
        // Only a column's right edge is taken: it sizes that column, as
        // Ami Pro's grid line does (`1498:2d2a`).
        assert_eq!(ruler.hit(left + 10.0, Half::Lower, 60.0), None);
        assert!(ruler.set_columns(&created, 2).is_none());
        let first_right = ruler.columns[0].1;
        let edge = Marker::Edge {
            column: 0,
            right: true,
        };
        assert_eq!(ruler.hit(first_right, Half::Lower, 60.0), Some(edge));
        let to = ruler.dragged(edge, first_right, first_right + 300.0);
        let Some(Command::EditTable { edit, .. }) = ruler.release(&created, edge, to, false) else {
            panic!("a table command");
        };
        let amipro_editing::tables::TableEdit::SetSizes { columns, sizes, .. } = edit else {
            panic!("sizes");
        };
        assert_eq!(columns, 0..1);
        // On the 1/16 in grid, the column wider.
        assert!(sizes.width.unwrap() > 2_000, "{sizes:?}");
        assert!(matches!(
            ruler.even_columns(&created),
            Some(Command::EditTable { .. })
        ));
        let tabbed = run(
            &created,
            ruler
                .add_tab(left + 500.0, TabKind::Centre, TabLeader::None)
                .unwrap(),
        );
        let tabs = tabbed.flow(flow).unwrap().paragraphs[0]
            .format
            .tabs
            .clone()
            .unwrap();
        assert!(tabs.contains(&TabStop::new(TabKind::Centre, 500)));
    }

    /// The `[edoc]` body without Ami Pro's paragraph caches (`<:#…>`), and
    /// the `[lay]` … `[elay]` records, one value a line.
    fn body_and_layouts(bytes: &[u8]) -> (String, Vec<String>) {
        let text = String::from_utf8_lossy(bytes);
        let strip = |text: &str| {
            let mut out = String::new();
            let mut rest = text;
            while let Some(start) = rest.find("<:#") {
                out.push_str(&rest[..start]);
                rest = rest[start..].split_once('>').map_or("", |(_, after)| after);
            }
            out.push_str(rest);
            out
        };
        let body = text
            .split("[edoc]")
            .nth(1)
            .and_then(|rest| rest.split("\n>").next())
            .unwrap_or_default();
        let body = strip(body)
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
        let mut layouts = Vec::new();
        let mut inside = false;
        for line in text.lines() {
            let line = line.trim();
            if line == "[lay]" {
                inside = true;
            }
            if inside && !line.is_empty() {
                layouts.push(strip(line));
            }
            if line == "[elay]" {
                inside = false;
            }
        }
        (body.trim().to_owned(), layouts)
    }

    /// One ruler gesture on `document` with the caret at the start of
    /// `paragraph`, as the ruler makes it.
    fn gesture(
        document: &Document,
        paragraph: usize,
        make: impl FnOnce(&Ruler, &Document) -> Option<Command>,
    ) -> Document {
        let mut pages = SessionPages::default();
        pages.update(document);
        let ruler = Ruler::at(document, pages.layout(), caret(document, paragraph, 0)).unwrap();
        let command = make(&ruler, document).expect("the gesture makes a command");
        run(document, command)
    }

    fn drag(ruler: &Ruler, document: &Document, marker: Marker, to: f32) -> Option<Command> {
        let from = ruler.position(marker);
        ruler.release(document, marker, ruler.dragged(marker, from, to), false)
    }

    /// VM fixtures RU1–RU12 (`RULER_NOTES.md`): each gesture made in Ami
    /// Pro, made here through the ruler's model on the same document,
    /// written as an Ami Pro document, and compared with Ami Pro's: the
    /// body and the page layout records. With `AMIPRO_TWIN_OUT`, the twins
    /// are written there as `ARUn.SAM` for Ami Pro to open.
    #[test]
    #[ignore = "needs the private VM fixtures (AMIPRO_VM_LAB)"]
    fn ruler_twins() {
        let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
            eprintln!("AMIPRO_VM_LAB is not set; skipping");
            return;
        };
        let lab = std::path::PathBuf::from(lab);
        let out = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from);
        let read = |name: &str| {
            amipro_formats::import_sam_path(lab.join(format!("{name}.SAM")))
                .unwrap()
                .document
        };
        let rub = read("RUB");
        // RU1 was made on another document: its own text without the ruler.
        let ru1 = read("RU1");
        let ru1_base = run(
            &ru1,
            Command::SetParagraphTabs {
                flow: ru1.main_flow_id(),
                first: 1,
                last: 1,
                tabs: None,
            },
        );
        let columns = |document: &Document, count: u8| {
            gesture(document, 0, |ruler, document| {
                ruler.set_columns(document, count)
            })
        };
        let two = columns(&rub, 2);
        let edge = |column, right| Marker::Edge { column, right };
        let ru9 = gesture(&two, 0, |ruler, document| {
            drag(ruler, document, edge(0, true), 5_520.0)
        });
        let ru10 = gesture(&ru9, 0, |ruler, document| {
            drag(ruler, document, edge(0, false), 1_800.0)
        });
        let cases: Vec<(&str, Document)> = vec![
            (
                "RU1",
                gesture(&ru1_base, 1, |ruler, _| {
                    ruler.add_tab(1_440.0 + 3_228.0, TabKind::Left, TabLeader::None)
                }),
            ),
            // RU2 was made on RU1: two columns, then the gap dragged.
            (
                "RU2",
                gesture(&columns(&ru1, 2), 0, |ruler, document| {
                    drag(ruler, document, Marker::Gap(0), 6_240.0 + 1_100.0)
                }),
            ),
            (
                "RU3",
                gesture(&rub, 1, |ruler, document| {
                    drag(ruler, document, Marker::Tab(2), 3_600.0 + 570.0)
                }),
            ),
            (
                "RU4",
                gesture(&rub, 1, |ruler, document| {
                    ruler.release(document, Marker::Tab(3), 4_320.0, true)
                }),
            ),
            (
                "RU5",
                gesture(&rub, 0, |ruler, document| {
                    drag(ruler, document, Marker::First, 1_440.0 + 730.0)
                }),
            ),
            (
                "RU6",
                gesture(&rub, 0, |ruler, document| {
                    drag(ruler, document, Marker::Rest, 1_440.0 + 370.0)
                }),
            ),
            (
                "RU7",
                gesture(&rub, 0, |ruler, document| {
                    drag(ruler, document, Marker::All, 1_440.0 + 730.0)
                }),
            ),
            (
                "RU8",
                gesture(&rub, 0, |ruler, document| {
                    drag(ruler, document, Marker::Right, 10_800.0 - 740.0)
                }),
            ),
            ("RU9", ru9.clone()),
            ("RU10", ru10.clone()),
            ("RU11", columns(&ru10, 3)),
            ("RU12", columns(&ru9, 3)),
        ];
        let options = amipro_formats::SamOptions::at(0);
        let mut different = Vec::new();
        for (name, document) in cases {
            let written = amipro_formats::sam_document(&document, &options);
            if let Some(out) = &out {
                std::fs::write(out.join(format!("A{name}.SAM")), &written).unwrap();
            }
            let ours = body_and_layouts(&written);
            let theirs = body_and_layouts(&std::fs::read(lab.join(format!("{name}.SAM"))).unwrap());
            if ours == theirs {
                eprintln!("{name}: the same");
            } else {
                if ours.0 != theirs.0 {
                    eprintln!(
                        "{name}: body differs\nours:\n{}\nAmi Pro's:\n{}",
                        ours.0, theirs.0
                    );
                }
                if ours.1 != theirs.1 {
                    let at = ours
                        .1
                        .iter()
                        .zip(&theirs.1)
                        .position(|(a, b)| a != b)
                        .unwrap_or(ours.1.len().min(theirs.1.len()));
                    eprintln!(
                        "{name}: layout differs at value {at}: ours {:?}, Ami Pro's {:?}",
                        &ours.1[at.saturating_sub(2)..(at + 4).min(ours.1.len())],
                        &theirs.1[at.saturating_sub(2)..(at + 4).min(theirs.1.len())]
                    );
                }
                different.push(name);
            }
        }
        assert!(different.is_empty(), "differ: {different:?}");
    }

    /// Random column gestures on the ruler and in Page Layout Properties,
    /// on the page, in the header, and in a frame, each followed by Save
    /// As's loss check: nothing to list, and every box's gutter what Ami
    /// Pro works out from its columns. (A stale gutter after a column drag
    /// listed "1 page layout"; found in the user's physical check.)
    #[test]
    fn column_gestures_leave_nothing_for_save_as_to_list() {
        let mut seed: u64 = 1;
        let mut next = move |n: u64| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % n
        };
        let options = amipro_formats::SamOptions::at(0);
        let (mut base, _) = rub();
        let frame = base.create_text_frame(
            Placement::Fixed {
                page: 1,
                rect: amipro_document::Rect::new(1_440, 7_200, 5_040, 9_360),
            },
            amipro_document::FrameStyle::default(),
        );
        base.frame_stack_mut().push(frame);
        let header = match base.frame(base.document_layout().header.right.unwrap()) {
            Some(amipro_document::Frame {
                content: amipro_document::FrameContent::Text(flow),
                ..
            }) => *flow,
            _ => unreachable!("a header frame holds text"),
        };
        let layout = base.document_layout().id;
        for _ in 0..60 {
            let mut document = base.clone();
            let mut log = Vec::new();
            for _ in 0..6 {
                let mut pages = SessionPages::default();
                pages.update(&document);
                let selection = match next(3) {
                    0 => flow_caret(header, 1),
                    _ => caret(&document, 0, 0),
                };
                let ruler = Ruler::at(&document, pages.layout(), selection).unwrap();
                let (what, command) = match next(9) {
                    0 => {
                        let columns = 1 + next(4) as u8;
                        (
                            format!("Columns {columns}"),
                            ruler.set_columns(&document, columns),
                        )
                    }
                    1 => ("Even Columns".into(), ruler.even_columns(&document)),
                    choice @ (2..=5) => {
                        let count = ruler.columns.len();
                        let column = next(count as u64) as usize;
                        let marker = match choice {
                            2 => Marker::Edge {
                                column,
                                right: false,
                            },
                            3 => Marker::Edge {
                                column,
                                right: true,
                            },
                            4 if column + 1 < count => Marker::Gap(column),
                            _ => Marker::Column(column),
                        };
                        let from = ruler.position(marker);
                        let at = ruler.dragged(marker, from, from + next(4_000) as f32 - 2_000.0);
                        (
                            format!("{marker:?} to {at}"),
                            ruler.release(&document, marker, at, false),
                        )
                    }
                    6 | 7 => {
                        let mut settings = LayoutSettings::of(&document, layout).unwrap();
                        let pages = match (&settings.left, next(2)) {
                            (None, _) => Pages::All,
                            (Some(_), 0) => Pages::Left,
                            (Some(_), _) => Pages::Right,
                        };
                        let operation = next(7);
                        let done = match operation {
                            0 => settings.set_columns(pages, 1 + next(4) as u8),
                            1 => settings.set_gutter(pages, next(1_500) as Twips),
                            2 => settings.set_margin(
                                pages,
                                layout_setup::MarginSide::Left,
                                360 + next(2_000) as Twips,
                            ),
                            3 => settings.set_mirror(Pages::Right, next(2) == 0),
                            4 => settings.set_even_columns(pages),
                            5 => settings.set_band_columns(Band::Header, pages, 1 + next(3) as u8),
                            _ => settings.set_band_gutter(Band::Header, pages, next(900) as Twips),
                        };
                        (
                            format!("panel {operation} on {pages:?}"),
                            done.ok()
                                .map(|()| Command::ModifyLayout { layout, settings }),
                        )
                    }
                    _ => {
                        let mut style = document.frame(frame).unwrap().style.clone();
                        let done = match next(3) {
                            0 => {
                                layout_setup::ruler_columns(&mut style.interior, 1 + next(3) as u8)
                                    .is_ok()
                            }
                            1 => layout_setup::set_box_edges(
                                &mut style.interior,
                                3_600,
                                &[
                                    (144, 900 + next(600) as Twips),
                                    (1_700 + next(300) as Twips, 3_456),
                                ],
                            )
                            .is_ok(),
                            _ => {
                                layout_setup::even_columns(&mut style.interior);
                                true
                            }
                        };
                        (
                            "frame columns".into(),
                            done.then_some(Command::SetFrameStyle { frame, style }),
                        )
                    }
                };
                let Some(command) = command else { continue };
                let Ok(applied) = apply(&document, &command) else {
                    continue;
                };
                document = applied.document;
                log.push(what);
                let layouts = document.layouts().iter().flat_map(|layout| {
                    std::iter::once(&layout.right.layout)
                        .chain(layout.left.as_ref().map(|left| &left.layout))
                });
                for layout in layouts.chain(document.frames().map(|frame| &frame.style.interior)) {
                    let mut settled = layout.clone();
                    settled.settle_gutter();
                    assert_eq!(settled.gutter, layout.gutter, "{log:?}");
                }
                let losses = amipro_formats::sam_losses(&document, &options).unwrap();
                assert_eq!(losses, [], "{log:?}");
            }
        }
    }
}
