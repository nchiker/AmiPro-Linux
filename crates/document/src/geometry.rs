//! Page and frame geometry: units, rectangles, lines, columns, tabs, and page
//! layouts (`docs/research/DECISIONS.md`, Phase 2.2, D4; the SAM encoding is
//! in `docs/research/SAM_PAGE_FRAME_SPEC.md`).

use crate::{DocumentError, FrameId, LayoutId};

/// Lengths in twips (1/1440 inch), the unit SAM stores.
pub type Twips = i32;

pub const TWIPS_PER_POINT: f32 = 20.0;

/// A Windows `COLORREF`: `0x00BBGGRR` (black 0, white 16777215, red 255).
pub type Colour = u32;

pub const BLACK: Colour = 0;
pub const WHITE: Colour = 0x00FF_FFFF;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Margins {
    pub top: Twips,
    pub right: Twips,
    pub bottom: Twips,
    pub left: Twips,
}

impl Margins {
    pub const fn uniform(value: Twips) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    fn is_valid(self) -> bool {
        [self.top, self.right, self.bottom, self.left]
            .iter()
            .all(|margin| *margin >= 0)
    }
}

/// A rectangle in twips from the page's top-left corner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub left: Twips,
    pub top: Twips,
    pub right: Twips,
    pub bottom: Twips,
}

impl Rect {
    pub const fn new(left: Twips, top: Twips, right: Twips, bottom: Twips) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(self) -> Twips {
        self.right - self.left
    }

    pub fn height(self) -> Twips {
        self.bottom - self.top
    }

    pub fn is_valid(self) -> bool {
        self.left < self.right && self.top < self.bottom
    }

    /// The rectangle moved by `dx` and `dy`, or `None` on overflow.
    pub fn offset(self, dx: Twips, dy: Twips) -> Option<Self> {
        Some(Self {
            left: self.left.checked_add(dx)?,
            top: self.top.checked_add(dy)?,
            right: self.right.checked_add(dx)?,
            bottom: self.bottom.checked_add(dy)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Size {
    pub width: Twips,
    pub height: Twips,
}

impl Size {
    pub fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Which sides of a box carry a line.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sides {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Sides {
    pub const ALL: Self = Self {
        left: true,
        right: true,
        top: true,
        bottom: true,
    };
    pub const NONE: Self = Self {
        left: false,
        right: false,
        top: false,
        bottom: false,
    };

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

/// Where a line sits relative to its box edge. The numbers are SAM's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinePosition {
    Middle = 1,
    Inside = 2,
    Outside = 3,
    CloseToInside = 4,
    CloseToOutside = 5,
}

/// Lines around a frame or a page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Border {
    pub sides: Sides,
    /// 1-based index into Ami Pro's line-style list; unknown indexes are kept.
    pub style: u16,
    pub colour: Colour,
    pub position: LinePosition,
}

/// The line drawn between columns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColumnLine {
    pub style: u16,
    pub colour: Colour,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabKind {
    Left,
    Centre,
    Right,
    Decimal,
}

/// What fills a tab's gap before the text it moves (the type's top two
/// bits in SAM; VM fixture TB2).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabLeader {
    #[default]
    None,
    Hyphens,
    Dots,
    Line,
}

/// A tab stop, positioned from its column's left edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TabStop {
    pub kind: TabKind,
    pub position: Twips,
    pub leader: TabLeader,
}

impl TabStop {
    /// A stop of `kind` at `position` without a leader.
    pub fn new(kind: TabKind, position: Twips) -> Self {
        Self {
            kind,
            position,
            leader: TabLeader::None,
        }
    }
}

/// A column's left and right edges, in twips from the left of its box's
/// text area (the left margin; inside a frame, its rounded corners' inset
/// too).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColumnEdges {
    pub left: Twips,
    pub right: Twips,
}

impl ColumnEdges {
    pub fn width(self) -> Twips {
        self.right - self.left
    }
}

/// Margins, columns, and tabs of a page or of a frame's interior (R7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutBox {
    pub margins: Margins,
    /// 1 to 8.
    pub columns: u8,
    /// The space between even columns. Ami Pro stores none: it works the
    /// gutter out from the columns (`1238:0000`), 240 for one column and
    /// the first gap otherwise, so a box with one column or uneven columns
    /// keeps that value ([`LayoutBox::settle_gutter`]).
    pub gutter: Twips,
    /// Columns whose widths or gaps differ, as Ami Pro's ruler makes them
    /// (VM fixtures RU2, RU9, RU10; `docs/research/RULER_NOTES.md`): one
    /// pair of edges per column. `None` while the columns are even, spread
    /// across the text area with `gutter` between them.
    pub column_edges: Option<Vec<ColumnEdges>>,
    /// One tab list per column, as Ami Pro keeps them.
    pub tabs: Vec<Vec<TabStop>>,
    pub balance: bool,
    pub line_between: Option<ColumnLine>,
}

impl LayoutBox {
    /// The gutter of one column, and Ami Pro's default (0.17 in).
    pub const DEFAULT_GUTTER: Twips = 240;

    /// One column with the given margins and no tabs.
    pub fn single_column(margins: Margins) -> Self {
        Self {
            margins,
            columns: 1,
            gutter: Self::DEFAULT_GUTTER,
            column_edges: None,
            tabs: vec![Vec::new()],
            balance: false,
            line_between: None,
        }
    }

    /// Even columns across a text area `text_width` wide, as Ami Pro's
    /// dialog and Cols spread them (`1148:1937`, `1238:02d0`): each
    /// (text_width − (n − 1)·gutter) / n wide, whole twips, the first at
    /// the left; no gutter when the gutters would not fit.
    pub fn even_edges(&self, text_width: Twips) -> Vec<ColumnEdges> {
        even_edges(self.columns, self.gutter, text_width)
    }

    /// The columns' edges across a text area `text_width` wide: the uneven
    /// ones the box keeps, or even ones.
    pub fn edges(&self, text_width: Twips) -> Vec<ColumnEdges> {
        match &self.column_edges {
            Some(edges) => edges.clone(),
            None => self.even_edges(text_width),
        }
    }

    /// Gives the box `edges` (one pair per column) in a text area
    /// `text_width` wide. Edges whose widths and gaps are alike within a
    /// twip, spread across the text area, are even columns again, as a
    /// file read back takes them: the box keeps none and takes their first
    /// gap as its gutter.
    pub fn set_edges(&mut self, edges: Vec<ColumnEdges>, text_width: Twips) {
        self.columns = edges.len().clamp(1, 8) as u8;
        if self.tabs.len() != usize::from(self.columns) {
            // One list for the box, on its first column.
            let stops = self.tab_stops().to_vec();
            self.tabs = std::iter::once(stops)
                .chain(std::iter::repeat_with(Vec::new))
                .take(usize::from(self.columns))
                .collect();
        }
        if let [first, second, ..] = edges[..]
            && alike(&edges)
            && first.left == 0
            && (0..Twips::from(self.columns)).contains(&(text_width - edges[edges.len() - 1].right))
        {
            // Even columns end short of the text area by the twips their
            // widths leave over.
            self.gutter = second.left - first.right;
            self.column_edges = None;
            return;
        }
        self.column_edges = (edges.len() > 1).then_some(edges);
        self.settle_gutter();
    }

    /// Makes the gutter what Ami Pro works out from the columns
    /// (`1238:0000`), which is all a file can hold: 240 for one column,
    /// the first gap for uneven columns; even columns keep theirs.
    pub fn settle_gutter(&mut self) {
        if self.columns < 2 {
            self.gutter = Self::DEFAULT_GUTTER;
        } else if let Some([first, second, ..]) = self.column_edges.as_deref() {
            self.gutter = (second.left - first.right).max(0);
        }
    }

    /// Whether every gap between the columns is alike within a twip, as
    /// Ami Pro's dialog tests before it respaces (`1238:0000`).
    pub fn gaps_alike(&self, text_width: Twips) -> bool {
        let edges = self.edges(text_width);
        let gaps: Vec<Twips> = edges
            .windows(2)
            .map(|pair| pair[1].left - pair[0].right)
            .collect();
        gaps.first()
            .is_none_or(|first| gaps.iter().all(|gap| (gap - first).abs() <= 1))
    }

    /// Most tab stops a box keeps (`1058:1843`).
    pub const MAX_TABS: usize = 22;

    /// The tab stops every column of the box uses. Ami Pro keeps one list
    /// per box and writes it after the first column (`11c8:24d6`); its
    /// reader keeps the first list that is not empty, at most 22 stops
    /// (`1058:1843`; `docs/research/PAGE_LAYOUT_NOTES.md`).
    pub fn tab_stops(&self) -> &[TabStop] {
        self.tabs
            .iter()
            .find(|tabs| !tabs.is_empty())
            .map_or(&[], |tabs| &tabs[..tabs.len().min(Self::MAX_TABS)])
    }

    /// Checks the model's own rules (margins and gutter not negative, 1 to
    /// 8 columns, one tab list per column); `what` names the box in errors.
    pub fn validate(&self, what: &str) -> Result<(), DocumentError> {
        let invalid = |message: &str| Err(DocumentError::Invalid(format!("{what}: {message}")));
        if !self.margins.is_valid() {
            return invalid("negative margin");
        }
        if !(1..=8).contains(&self.columns) {
            return invalid("column count must be 1 to 8");
        }
        if self.gutter < 0 {
            return invalid("negative gutter");
        }
        if self.tabs.len() != usize::from(self.columns) {
            return invalid("one tab list per column is required");
        }
        if self.tabs.iter().flatten().any(|tab| tab.position < 0) {
            return invalid("negative tab position");
        }
        if let Some(edges) = &self.column_edges {
            if edges.len() != usize::from(self.columns) || edges.len() < 2 {
                return invalid("uneven columns need one pair of edges per column, two or more");
            }
            let mut previous = 0;
            for edges in edges {
                if edges.left < previous || edges.right <= edges.left {
                    return invalid("column edges must go left to right without overlapping");
                }
                previous = edges.right;
            }
        }
        Ok(())
    }

    /// Checks the uneven columns against the text area's width.
    pub fn validate_width(&self, what: &str, text_width: Twips) -> Result<(), DocumentError> {
        match self.column_edges.as_ref().and_then(|edges| edges.last()) {
            Some(last) if last.right > text_width => Err(DocumentError::Invalid(format!(
                "{what}: a column runs past the right margin"
            ))),
            _ => Ok(()),
        }
    }
}

/// Widths and gaps alike within a twip.
fn alike(edges: &[ColumnEdges]) -> bool {
    let width = edges[0].width();
    let gap = edges
        .get(1)
        .map_or(0, |second| second.left - edges[0].right);
    edges.iter().all(|edges| (edges.width() - width).abs() <= 1)
        && edges
            .windows(2)
            .all(|pair| (pair[1].left - pair[0].right - gap).abs() <= 1)
}

fn even_edges(columns: u8, gutter: Twips, text_width: Twips) -> Vec<ColumnEdges> {
    let count = i64::from(columns.max(1));
    let mut gutter = i64::from(gutter);
    if i64::from(text_width) < (count - 1) * gutter {
        gutter = 0;
    }
    let width = (i64::from(text_width) - (count - 1) * gutter) / count;
    (0..count)
        .map(|index| {
            let left = index * (width + gutter);
            ColumnEdges {
                left: left as Twips,
                right: (left + width) as Twips,
            }
        })
        .collect()
}

/// The geometry of one kind of page (right or left).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageGeometry {
    pub width: Twips,
    pub height: Twips,
    pub layout: LayoutBox,
    /// Lines around the page.
    pub border: Option<Border>,
}

impl PageGeometry {
    /// The width between the page's left and right margins.
    pub fn text_width(&self) -> Twips {
        self.width - self.layout.margins.left - self.layout.margins.right
    }

    pub(crate) fn validate(&self, what: &str) -> Result<(), DocumentError> {
        self.layout.validate(what)?;
        self.layout.validate_width(what, self.text_width())?;
        let margins = self.layout.margins;
        if self.width <= 0
            || self.height <= 0
            || i64::from(margins.left) + i64::from(margins.right) >= i64::from(self.width)
            || i64::from(margins.top) + i64::from(margins.bottom) >= i64::from(self.height)
        {
            return Err(DocumentError::Invalid(format!(
                "invalid page geometry in {what}"
            )));
        }
        Ok(())
    }
}

/// The frames of one header or footer: the right (or only) page's, and the
/// left page's when left pages differ.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SideFrames {
    pub right: Option<FrameId>,
    pub left: Option<FrameId>,
}

/// A named page layout. Every page has exactly one; the first layout of a
/// document is its own, and inserted layouts follow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageLayout {
    pub id: LayoutId,
    pub name: String,
    /// Ami Pro's paper code (1 Letter, 2 Legal, 4 A4, 7 Custom); the stored
    /// geometry is authoritative.
    pub paper: u8,
    pub landscape: bool,
    /// Right pages, or all pages when `left` is `None`. Right pages are odd.
    pub right: PageGeometry,
    pub left: Option<PageGeometry>,
    pub mirror: bool,
    pub header_from_second_page: bool,
    pub footer_from_second_page: bool,
    pub header: SideFrames,
    pub footer: SideFrames,
    /// Repeating frames, back to front. They paint below every page frame.
    pub repeating: Vec<FrameId>,
    pub legacy: LegacyData,
}

impl PageLayout {
    /// Letter with one-inch margins and no header or footer frames yet.
    pub fn letter(id: LayoutId, name: impl Into<String>) -> Self {
        // Ami Pro's default stops (`_DEFAULT.STY`'s Standard layout): left
        // tabs every half inch from the margin to six inches.
        let mut layout = LayoutBox::single_column(Margins::uniform(1_440));
        layout.tabs = vec![
            (1..=12)
                .map(|stop| TabStop::new(TabKind::Left, 720 * stop))
                .collect(),
        ];
        Self {
            id,
            name: name.into(),
            paper: 1,
            landscape: false,
            right: PageGeometry {
                width: 12_240,
                height: 15_840,
                layout,
                border: None,
            },
            left: None,
            mirror: false,
            header_from_second_page: false,
            footer_from_second_page: false,
            header: SideFrames::default(),
            footer: SideFrames::default(),
            repeating: Vec::new(),
            legacy: LegacyData::default(),
        }
    }

    /// The right page's geometry in points, as the current UI uses it.
    pub fn page_setup(&self) -> PageSetup {
        let points = |twips: Twips| twips as f32 / TWIPS_PER_POINT;
        let margins = self.right.layout.margins;
        PageSetup {
            width_points: points(self.right.width),
            height_points: points(self.right.height),
            margin_points: [
                points(margins.top),
                points(margins.right),
                points(margins.bottom),
                points(margins.left),
            ],
        }
    }

    /// Every frame this layout holds: header and footer frames, then its
    /// repeating frames.
    pub fn frames(&self) -> impl Iterator<Item = FrameId> + '_ {
        [
            self.header.right,
            self.header.left,
            self.footer.right,
            self.footer.left,
        ]
        .into_iter()
        .flatten()
        .chain(self.repeating.iter().copied())
    }

    /// Checks the layout's geometry and its left-page parts.
    pub fn validate(&self) -> Result<(), DocumentError> {
        let what = format!("layout {}", self.name);
        self.right.validate(&what)?;
        if let Some(left) = &self.left {
            left.validate(&what)?;
        } else if self.mirror || self.header.left.is_some() || self.footer.left.is_some() {
            return Err(DocumentError::Invalid(format!(
                "{what}: mirroring and left header or footer frames need left pages"
            )));
        }
        Ok(())
    }
}

/// Page geometry in points, as the current UI and native v1/v2 use it.
#[derive(Clone, Debug, PartialEq)]
pub struct PageSetup {
    pub width_points: f32,
    pub height_points: f32,
    /// Top, right, bottom, left.
    pub margin_points: [f32; 4],
}

/// Legacy-format data that the model does not interpret yet, kept with its
/// object in order so that a future writer can emit it again. Keys name the
/// field (`frame.field8`, `frmlay.3`) or the raw section (`section [xyz]`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LegacyData {
    pub values: Vec<LegacyValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyValue {
    pub key: String,
    pub value: String,
}

impl LegacyData {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn push(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.push(LegacyValue {
            key: key.into(),
            value: value.into(),
        });
    }

    /// Removes every value kept under `key`; whether there was one.
    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.values.len();
        self.values.retain(|value| value.key != key);
        self.values.len() != before
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|value| value.key == key)
            .map(|value| value.value.as_str())
    }
}
