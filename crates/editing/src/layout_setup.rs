//! Setting a page layout up as Ami Pro's Modify Page Layout does
//! (`docs/research/PAGE_LAYOUT_NOTES.md`, from the decompiled
//! `AMIPROUI.DLL`): which pages a change goes to, Mirror, each value's
//! limits, and what a change does to the columns and to the header and
//! footer bands. Each method changes a [`LayoutSettings`] or refuses with
//! the reason, leaving it as it was; the caller applies the result as one
//! [`crate::Command::ModifyLayout`], so each change is one undo step.

use amipro_document::{
    Border, Colour, ColumnLine, LayoutBox, LinePosition, PageGeometry, Sides, TabStop, Twips,
};

use crate::frames::LayoutSettings;
use crate::{Applied, ChangeReport, EditError};

/// Which pages a change applies to: Ami Pro's Pages group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pages {
    /// Every page alike: the layout has no left pages of its own.
    All,
    /// Right (odd) pages; the first change gives the layout its own left
    /// pages, copies of the right ones.
    Right,
    /// Left (even) pages, likewise.
    Left,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MarginSide {
    Left,
    Right,
    Top,
    Bottom,
}

/// A page's header or footer: the band its top (bottom) margin makes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Band {
    Header,
    Footer,
}

/// The longest length Ami Pro's boxes take: 22 inches (`AMIUICNT.DLL`).
pub const MAX_LENGTH: Twips = 31_680;
/// Ami Pro's gutter for new columns (0.17 in).
pub const DEFAULT_GUTTER: Twips = LayoutBox::DEFAULT_GUTTER;
/// Ami Pro's paper code for a typed size.
pub const CUSTOM_PAPER: u8 = 7;
/// The entries in Ami Pro's line-style lists for pages.
pub const LINE_STYLES: u16 = 12;
/// Most tab stops a layout keeps.
pub const MAX_TABS: usize = LayoutBox::MAX_TABS;

/// A paper size Ami Pro offers, portrait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Paper {
    pub code: u8,
    pub name: &'static str,
    pub width: Twips,
    pub height: Twips,
}

/// Ami Pro's paper sizes in its dialog's order, as it stores them: its
/// sizes pass through boxes of two-decimal inches and are read back
/// truncated (A4 is shown 8.27 × 11.69 in and stored 11,908 × 16,833; VM
/// fixture G13).
pub const PAPERS: [Paper; 6] = [
    Paper {
        code: 1,
        name: "Letter",
        width: 12_240,
        height: 15_840,
    },
    Paper {
        code: 2,
        name: "Legal",
        width: 12_240,
        height: 20_160,
    },
    Paper {
        code: 4,
        name: "A4",
        width: 11_908,
        height: 16_833,
    },
    Paper {
        code: 3,
        name: "A3",
        width: 16_833,
        height: 23_803,
    },
    Paper {
        code: 5,
        name: "A5",
        width: 8_395,
        height: 11_908,
    },
    Paper {
        code: 6,
        name: "B5",
        width: 9_979,
        height: 14_169,
    },
];

pub type Refusal = &'static str;

const TOO_LONG: Refusal = "A length is at most 22 inches";
const NEGATIVE: Refusal = "A length cannot be less than 0";
const NO_ROOM_ACROSS: Refusal = "The left and right margins leave no room for the columns";
const NO_ROOM_DOWN: Refusal = "The top and bottom margins leave no room for the text";
const COLUMN_COUNT: Refusal = "A page has 1 to 8 columns";
const NEEDS_COLUMNS: Refusal = "This needs two or more columns";
const PAPER_TOO_SMALL: Refusal = "The margins and columns do not fit that page";
const MIRROR_NEEDS_SIDE: Refusal = "Mirror is for right and left pages: choose Right or Left";
const LEFT_PAGES_DIFFER: Refusal = "This layout's left pages differ: choose Right or Left";
const MIRROR_IS_ON: Refusal = "Mirrored pages differ: turn Mirror off first";
const TOO_MANY_TABS: Refusal = "A layout has at most 22 tabs";
const NO_PAGE_LINES: Refusal = "Choose a side for the lines first";
const NO_COLUMN_LINE: Refusal = "Turn on the line between columns first";
const NO_LINE: Refusal = "Turn on a line first";
const LINE_STYLE: Refusal = "Line styles are 1 to 12";
const NO_BAND: Refusal = "This layout has no such header or footer";
const BAND_TOO_TALL: Refusal =
    "Its top and bottom margins are more than its height (the page margin)";

fn length(value: Twips) -> Result<Twips, Refusal> {
    if value < 0 {
        Err(NEGATIVE)
    } else if value > MAX_LENGTH {
        Err(TOO_LONG)
    } else {
        Ok(value)
    }
}

/// Whether `layout`'s columns fit `width`: each at least a twip wide
/// between the margins (`1238:0573`).
fn fits_across(layout: &LayoutBox, width: Twips) -> bool {
    if let Some(edges) = &layout.column_edges {
        let text_width = width - layout.margins.left - layout.margins.right;
        return edges.iter().all(|edges| edges.width() >= 1)
            && edges.last().is_some_and(|last| last.right <= text_width);
    }
    let columns = i64::from(layout.columns.max(1));
    let gutter = if columns > 1 {
        i64::from(layout.gutter)
    } else {
        0
    };
    i64::from(layout.margins.left)
        + i64::from(layout.margins.right)
        + columns
        + (columns - 1) * gutter
        <= i64::from(width)
}

/// Whether a page keeps room for text: the model keeps some height between
/// the margins (Ami Pro's box allows none, and then cannot place text).
fn fits_down(geometry: &PageGeometry) -> bool {
    i64::from(geometry.layout.margins.top) + i64::from(geometry.layout.margins.bottom)
        < i64::from(geometry.height)
}

fn page_fits(geometry: &PageGeometry) -> Result<(), Refusal> {
    if !fits_across(&geometry.layout, geometry.width) {
        Err(NO_ROOM_ACROSS)
    } else if !fits_down(geometry) {
        Err(NO_ROOM_DOWN)
    } else {
        Ok(())
    }
}

/// The band's height: the page's top margin for a header, its bottom
/// margin for a footer.
fn band_height(geometry: &PageGeometry, band: Band) -> Twips {
    match band {
        Band::Header => geometry.layout.margins.top,
        Band::Footer => geometry.layout.margins.bottom,
    }
}

/// Shrinks a band's own top and bottom margins to its height, the bottom
/// one first (`1240:09ee`).
fn fit_band(interior: &mut LayoutBox, height: Twips) {
    let margins = &mut interior.margins;
    let excess = margins.top + margins.bottom - height.max(0);
    if excess <= 0 {
        return;
    }
    let bottom = (margins.bottom - excess).max(0);
    let left_over = excess - (margins.bottom - bottom);
    margins.bottom = bottom;
    margins.top = (margins.top - left_over).max(0);
}

/// One tab list for the whole box, stored on its first column, as Ami Pro
/// writes it (`11c8:24d6`).
fn one_tab_list(layout: &mut LayoutBox, stops: Vec<TabStop>) {
    let columns = usize::from(layout.columns.max(1));
    layout.tabs = std::iter::once(stops)
        .chain(std::iter::repeat_with(Vec::new))
        .take(columns)
        .collect();
}

/// The ruler's column and margin drags (`1148:0d0c`; VM fixtures RU2, RU9,
/// RU10): the columns' `edges` from the left edge of a box `width` wide;
/// the first column's left edge is the left margin and the last column's
/// right edge sets the right margin. Nothing else moves.
pub fn set_box_edges(
    layout: &mut LayoutBox,
    width: Twips,
    edges: &[(Twips, Twips)],
) -> Result<(), Refusal> {
    let (Some(first), Some(last)) = (edges.first(), edges.last()) else {
        return Err(COLUMN_COUNT);
    };
    if edges.len() > 8 {
        return Err(COLUMN_COUNT);
    }
    let mut previous = 0;
    for &(left, right) in edges {
        if left < previous || right <= left {
            return Err(NO_ROOM_ACROSS);
        }
        previous = right;
    }
    if last.1 > width {
        return Err(NO_ROOM_ACROSS);
    }
    layout.margins.left = first.0;
    layout.margins.right = width - last.1;
    let text = last.1 - first.0;
    let relative = edges
        .iter()
        .map(|&(left, right)| amipro_document::ColumnEdges {
            left: left - first.0,
            right: right - first.0,
        })
        .collect();
    layout.set_edges(relative, text);
    Ok(())
}

/// The ruler's Columns (Ami Pro's Cols, `1148:1937`): even columns with a
/// 240-twip gutter, whatever the gaps were (VM fixtures RU11, RU12).
pub fn ruler_columns(layout: &mut LayoutBox, columns: u8) -> Result<(), Refusal> {
    if !(1..=8).contains(&columns) {
        return Err(COLUMN_COUNT);
    }
    set_columns_of(layout, columns);
    layout.gutter = DEFAULT_GUTTER;
    Ok(())
}

/// The gutter a column count spreads new columns with (`1238:008a`): the
/// gap the columns share, or 240 for one column or gaps that differ
/// (`1238:0000`).
pub fn count_gutter(layout: &LayoutBox) -> Twips {
    LayoutSettings::gutter_shown(layout).unwrap_or(DEFAULT_GUTTER)
}

/// Uneven columns made even, as a column count spreads them: the gap they
/// share, when every gap is alike, else 240.
pub fn even_columns(layout: &mut LayoutBox) {
    layout.gutter = count_gutter(layout);
    layout.column_edges = None;
}

/// A column count: even columns with the gutter the columns had
/// (`1238:008a`); one column has no gutter (240) and no balance.
fn set_columns_of(layout: &mut LayoutBox, columns: u8) {
    let stops = layout.tab_stops().to_vec();
    layout.gutter = count_gutter(layout);
    layout.columns = columns;
    layout.column_edges = None;
    one_tab_list(layout, stops);
    layout.settle_gutter();
    if columns < 2 {
        layout.balance = false;
    }
}

/// What a change of the text area does to uneven columns, as Ami Pro's
/// dialog does it for a margin (`1238:02d0`): with every gap alike they
/// become even; otherwise the columns stay where they are on the page and
/// only the outer edges follow, the first column's left to the new left
/// margin and the last column's right to the new right one. `before` is
/// the box's margins before the change, and the box spans `width` (the
/// page's width) before and after; `old_width` the width before a resize.
/// The page size takes the same rule, from its right (no evidence: Ami
/// Pro's Page settings panel was not traced for uneven columns).
pub fn refit_columns(
    layout: &mut LayoutBox,
    before: amipro_document::Margins,
    old_width: Twips,
    width: Twips,
) -> Result<(), Refusal> {
    if layout.column_edges.is_none() {
        return Ok(());
    }
    let old_text = old_width - before.left - before.right;
    let text = width - layout.margins.left - layout.margins.right;
    if layout.gaps_alike(old_text) {
        layout.gutter = count_gutter(layout);
        layout.column_edges = None;
        return Ok(());
    }
    let shift = layout.margins.left - before.left;
    let mut edges = layout.edges(old_text);
    for edges in &mut edges {
        edges.left -= shift;
        edges.right -= shift;
    }
    if let Some(first) = edges.first_mut() {
        first.left = 0;
    }
    if let Some(last) = edges.last_mut() {
        last.right = text;
    }
    let mut previous = 0;
    for edges in &edges {
        if edges.left < previous || edges.width() < 1 {
            return Err(NO_ROOM_ACROSS);
        }
        previous = edges.right;
    }
    layout.set_edges(edges, text);
    Ok(())
}

/// A page kind mirrored (`1248:110d`): margins exchanged, columns reversed
/// (even columns stay even), and a line on only one of the left and right
/// sides moved to the other.
fn mirrored_page(geometry: &PageGeometry) -> PageGeometry {
    let mut mirrored = geometry.clone();
    mirrored.layout = mirrored_box(&geometry.layout, geometry.width);
    if let Some(border) = &mut mirrored.border
        && border.sides.left != border.sides.right
    {
        std::mem::swap(&mut border.sides.left, &mut border.sides.right);
    }
    mirrored
}

/// A box across a page `width` wide, mirrored: margins exchanged and
/// uneven columns reversed, gaps kept.
fn mirrored_box(layout: &LayoutBox, width: Twips) -> LayoutBox {
    let mut mirrored = layout.clone();
    std::mem::swap(&mut mirrored.margins.left, &mut mirrored.margins.right);
    let text = width - layout.margins.left - layout.margins.right;
    if let Some(edges) = &mut mirrored.column_edges {
        edges.reverse();
        for edges in edges.iter_mut() {
            *edges = amipro_document::ColumnEdges {
                left: text - edges.right,
                right: text - edges.left,
            };
        }
    }
    mirrored.settle_gutter();
    mirrored
}

impl LayoutSettings {
    /// What the Pages group shows for the layout: All when every page is
    /// alike, otherwise Right.
    pub fn pages(&self) -> Pages {
        if self.left.is_some() {
            Pages::Right
        } else {
            Pages::All
        }
    }

    /// The page kind a change to `pages` reads from.
    pub fn page(&self, pages: Pages) -> &PageGeometry {
        match (pages, &self.left) {
            (Pages::Left, Some(left)) => left,
            _ => &self.right,
        }
    }

    /// The band interior a change to `pages` reads from, if the layout has
    /// that frame.
    pub fn band(&self, band: Band, pages: Pages) -> Option<&LayoutBox> {
        let boxes = match band {
            Band::Header => &self.header,
            Band::Footer => &self.footer,
        };
        match pages {
            Pages::Left if self.left.is_some() => boxes.left.as_ref(),
            _ => boxes.right.as_ref(),
        }
    }

    /// The gutter Ami Pro shows: none for one column, and for uneven
    /// columns their gap only while every gap is alike (`1238:0000`).
    pub fn gutter_shown(layout: &LayoutBox) -> Option<Twips> {
        if layout.columns < 2 {
            return None;
        }
        match &layout.column_edges {
            None => Some(layout.gutter),
            Some(edges) => {
                let gaps: Vec<Twips> = edges
                    .windows(2)
                    .map(|pair| pair[1].left - pair[0].right)
                    .collect();
                let first = *gaps.first()?;
                gaps.iter()
                    .all(|gap| (gap - first).abs() <= 1)
                    .then_some(first)
            }
        }
    }

    /// The ruler's column and margin drags on the page: `edges` from the
    /// page's left edge ([`set_box_edges`]).
    pub fn set_page_edges(
        &mut self,
        pages: Pages,
        edges: &[(Twips, Twips)],
    ) -> Result<(), Refusal> {
        self.change(pages, |settings, pages| {
            let geometry = settings.page_mut(pages);
            set_box_edges(&mut geometry.layout, geometry.width, edges)?;
            page_fits(geometry)
        })
    }

    /// The same in a header or footer, whose band spans the page.
    pub fn set_band_edges(
        &mut self,
        band: Band,
        pages: Pages,
        edges: &[(Twips, Twips)],
    ) -> Result<(), Refusal> {
        self.change_band(band, pages, |interior, geometry| {
            set_box_edges(interior, geometry.width, edges)
        })
    }

    /// The ruler's Columns on the page, or in a band.
    pub fn set_ruler_columns(
        &mut self,
        band: Option<Band>,
        pages: Pages,
        columns: u8,
    ) -> Result<(), Refusal> {
        match band {
            None => self.change(pages, |settings, pages| {
                let geometry = settings.page_mut(pages);
                ruler_columns(&mut geometry.layout, columns)?;
                page_fits(geometry)
            }),
            Some(band) => self.change_band(band, pages, |interior, geometry| {
                ruler_columns(interior, columns)?;
                fits_across(interior, geometry.width)
                    .then_some(())
                    .ok_or(NO_ROOM_ACROSS)
            }),
        }
    }

    /// Even Columns: the page's uneven columns spread evenly again with the
    /// box's gutter (`UI_SPEC.md` §19).
    pub fn set_even_columns(&mut self, pages: Pages) -> Result<(), Refusal> {
        self.change(pages, |settings, pages| {
            let geometry = settings.page_mut(pages);
            even_columns(&mut geometry.layout);
            page_fits(geometry)
        })
    }

    /// Even Columns for a header's or footer's columns.
    pub fn set_band_even_columns(&mut self, band: Band, pages: Pages) -> Result<(), Refusal> {
        self.change_band(band, pages, |interior, geometry| {
            even_columns(interior);
            fits_across(interior, geometry.width)
                .then_some(())
                .ok_or(NO_ROOM_ACROSS)
        })
    }

    /// Makes every page alike (Pages: All): the left pages, their header
    /// and footer included, give way to the right ones (`1248:0a9d`).
    pub fn set_all_pages(&mut self) -> Result<(), Refusal> {
        if self.mirror {
            return Err(MIRROR_IS_ON);
        }
        self.left = None;
        self.header.left = None;
        self.footer.left = None;
        Ok(())
    }

    /// Mirror: left pages are the right ones reflected, or the other way
    /// round, from the kind being edited.
    pub fn set_mirror(&mut self, pages: Pages, on: bool) -> Result<(), Refusal> {
        if pages == Pages::All {
            return Err(MIRROR_NEEDS_SIDE);
        }
        self.mirror = on;
        if on {
            self.ensure_left_pages();
            self.after_change(pages);
        }
        Ok(())
    }

    /// One of Ami Pro's paper sizes, in the current orientation, for every
    /// page of the layout.
    pub fn set_paper(&mut self, code: u8) -> Result<(), Refusal> {
        let paper = PAPERS
            .iter()
            .find(|paper| paper.code == code)
            .ok_or("Ami Pro has no such paper size")?;
        let (width, height) = if self.landscape {
            (paper.height, paper.width)
        } else {
            (paper.width, paper.height)
        };
        self.resize(width, height)?;
        self.paper = code;
        Ok(())
    }

    /// A typed page size (Custom), as shown: width across, height down.
    pub fn set_custom_size(&mut self, width: Twips, height: Twips) -> Result<(), Refusal> {
        length(width)?;
        length(height)?;
        self.resize(width, height)?;
        self.paper = CUSTOM_PAPER;
        Ok(())
    }

    /// Portrait or Landscape: the page turns, width and height exchanged.
    pub fn set_landscape(&mut self, landscape: bool) -> Result<(), Refusal> {
        if landscape != self.landscape {
            let (width, height) = (self.right.width, self.right.height);
            self.resize(height, width)?;
            self.landscape = landscape;
        }
        Ok(())
    }

    fn resize(&mut self, width: Twips, height: Twips) -> Result<(), Refusal> {
        let mut next = self.clone();
        for geometry in std::iter::once(&mut next.right).chain(next.left.as_mut()) {
            let old_width = geometry.width;
            geometry.width = width;
            geometry.height = height;
            let margins = geometry.layout.margins;
            refit_columns(&mut geometry.layout, margins, old_width, width)
                .map_err(|_| PAPER_TOO_SMALL)?;
            page_fits(geometry).map_err(|_| PAPER_TOO_SMALL)?;
        }
        for boxes in [&mut next.header, &mut next.footer] {
            for interior in [boxes.right.as_mut(), boxes.left.as_mut()]
                .into_iter()
                .flatten()
            {
                let margins = interior.margins;
                let old_width = self.right.width;
                refit_columns(interior, margins, old_width, width).map_err(|_| PAPER_TOO_SMALL)?;
            }
        }
        *self = next;
        Ok(())
    }

    /// A page margin. A top or bottom margin is also its header's or
    /// footer's height, whose own margins shrink to fit (`1240:09ee`).
    pub fn set_margin(
        &mut self,
        pages: Pages,
        side: MarginSide,
        value: Twips,
    ) -> Result<(), Refusal> {
        let value = length(value)?;
        self.change(pages, |settings, pages| {
            let geometry = settings.page_mut(pages);
            let before = geometry.layout.margins;
            let margins = &mut geometry.layout.margins;
            match side {
                MarginSide::Left => margins.left = value,
                MarginSide::Right => margins.right = value,
                MarginSide::Top => margins.top = value,
                MarginSide::Bottom => margins.bottom = value,
            }
            refit_columns(&mut geometry.layout, before, geometry.width, geometry.width)?;
            page_fits(geometry)?;
            let band = match side {
                MarginSide::Top => Band::Header,
                MarginSide::Bottom => Band::Footer,
                _ => return Ok(()),
            };
            if let Some(interior) = settings.band_mut(band, pages) {
                fit_band(interior, value);
            }
            Ok(())
        })
    }

    /// The number of columns, 1 to 8, spread evenly with the gutter; one
    /// column has no gutter and no balance.
    pub fn set_columns(&mut self, pages: Pages, columns: u8) -> Result<(), Refusal> {
        if !(1..=8).contains(&columns) {
            return Err(COLUMN_COUNT);
        }
        self.change(pages, |settings, pages| {
            let geometry = settings.page_mut(pages);
            set_columns_of(&mut geometry.layout, columns);
            page_fits(geometry)
        })
    }

    /// The space between columns.
    pub fn set_gutter(&mut self, pages: Pages, gutter: Twips) -> Result<(), Refusal> {
        let gutter = length(gutter)?;
        self.change(pages, |settings, pages| {
            let geometry = settings.page_mut(pages);
            if geometry.layout.columns < 2 {
                return Err(NEEDS_COLUMNS);
            }
            geometry.layout.gutter = gutter;
            geometry.layout.column_edges = None;
            page_fits(geometry)
        })
    }

    /// Column balance: the last page's columns end level.
    pub fn set_balance(&mut self, pages: Pages, on: bool) -> Result<(), Refusal> {
        self.change(pages, |settings, pages| {
            let layout = &mut settings.page_mut(pages).layout;
            if on && layout.columns < 2 {
                return Err(NEEDS_COLUMNS);
            }
            layout.balance = on;
            Ok(())
        })
    }

    /// The page's tab stops: one list for every column, in order of
    /// position, at most 22.
    pub fn set_tabs(&mut self, pages: Pages, stops: Vec<TabStop>) -> Result<(), Refusal> {
        let stops = checked_tabs(stops)?;
        self.change(pages, |settings, pages| {
            one_tab_list(&mut settings.page_mut(pages).layout, stops);
            Ok(())
        })
    }

    /// The sides with a line around the page; none removes the lines. New
    /// lines take the first style, Inside, and the line between columns'
    /// colour.
    pub fn set_page_line_sides(&mut self, pages: Pages, sides: Sides) -> Result<(), Refusal> {
        self.change(pages, |settings, pages| {
            let geometry = settings.page_mut(pages);
            geometry.border = if sides.is_none() {
                None
            } else {
                let colour = geometry
                    .layout
                    .line_between
                    .map_or(amipro_document::BLACK, |line| line.colour);
                Some(geometry.border.map_or(
                    Border {
                        sides,
                        style: 1,
                        colour,
                        position: LinePosition::Inside,
                    },
                    |border| Border { sides, ..border },
                ))
            };
            Ok(())
        })
    }

    /// The style of the lines around the page, 1 to 12.
    pub fn set_page_line_style(&mut self, pages: Pages, style: u16) -> Result<(), Refusal> {
        check_line_style(style)?;
        self.change(pages, |settings, pages| {
            let border = settings
                .page_mut(pages)
                .border
                .as_mut()
                .ok_or(NO_PAGE_LINES)?;
            border.style = style;
            Ok(())
        })
    }

    /// Where the lines around the page sit against the page's edge.
    pub fn set_page_line_position(
        &mut self,
        pages: Pages,
        position: LinePosition,
    ) -> Result<(), Refusal> {
        self.change(pages, |settings, pages| {
            let border = settings
                .page_mut(pages)
                .border
                .as_mut()
                .ok_or(NO_PAGE_LINES)?;
            border.position = position;
            Ok(())
        })
    }

    /// The line between columns, on or off; a new one takes the first style
    /// and the page lines' colour. It shows with two or more columns.
    pub fn set_column_line(&mut self, pages: Pages, on: bool) -> Result<(), Refusal> {
        self.change(pages, |settings, pages| {
            let geometry = settings.page_mut(pages);
            geometry.layout.line_between = if on {
                Some(
                    geometry.layout.line_between.unwrap_or(ColumnLine {
                        style: 1,
                        colour: geometry
                            .border
                            .map_or(amipro_document::BLACK, |border| border.colour),
                    }),
                )
            } else {
                None
            };
            Ok(())
        })
    }

    /// The style of the line between columns, 1 to 12.
    pub fn set_column_line_style(&mut self, pages: Pages, style: u16) -> Result<(), Refusal> {
        check_line_style(style)?;
        self.change(pages, |settings, pages| {
            let line = settings
                .page_mut(pages)
                .layout
                .line_between
                .as_mut()
                .ok_or(NO_COLUMN_LINE)?;
            line.style = style;
            Ok(())
        })
    }

    /// The one colour of the page lines and the line between columns.
    pub fn set_line_colour(&mut self, pages: Pages, colour: Colour) -> Result<(), Refusal> {
        self.change(pages, |settings, pages| {
            let geometry = settings.page_mut(pages);
            if geometry.border.is_none() && geometry.layout.line_between.is_none() {
                return Err(NO_LINE);
            }
            if let Some(border) = &mut geometry.border {
                border.colour = colour;
            }
            if let Some(line) = &mut geometry.layout.line_between {
                line.colour = colour;
            }
            Ok(())
        })
    }

    /// A margin inside the header or footer band. Left and right keep room
    /// for its columns across the page; top and bottom together are at most
    /// the band's height.
    pub fn set_band_margin(
        &mut self,
        band: Band,
        pages: Pages,
        side: MarginSide,
        value: Twips,
    ) -> Result<(), Refusal> {
        let value = length(value)?;
        self.change_band(band, pages, |interior, geometry| {
            let before = interior.margins;
            let margins = &mut interior.margins;
            match side {
                MarginSide::Left => margins.left = value,
                MarginSide::Right => margins.right = value,
                MarginSide::Top => margins.top = value,
                MarginSide::Bottom => margins.bottom = value,
            }
            refit_columns(interior, before, geometry.width, geometry.width)?;
            if !fits_across(interior, geometry.width) {
                return Err(NO_ROOM_ACROSS);
            }
            if interior.margins.top + interior.margins.bottom > band_height(geometry, band) {
                return Err(BAND_TOO_TALL);
            }
            Ok(())
        })
    }

    /// The header's or footer's columns, 1 to 8.
    pub fn set_band_columns(
        &mut self,
        band: Band,
        pages: Pages,
        columns: u8,
    ) -> Result<(), Refusal> {
        if !(1..=8).contains(&columns) {
            return Err(COLUMN_COUNT);
        }
        self.change_band(band, pages, |interior, geometry| {
            set_columns_of(interior, columns);
            fits_across(interior, geometry.width)
                .then_some(())
                .ok_or(NO_ROOM_ACROSS)
        })
    }

    /// The header's or footer's gutter.
    pub fn set_band_gutter(
        &mut self,
        band: Band,
        pages: Pages,
        gutter: Twips,
    ) -> Result<(), Refusal> {
        let gutter = length(gutter)?;
        self.change_band(band, pages, |interior, geometry| {
            if interior.columns < 2 {
                return Err(NEEDS_COLUMNS);
            }
            interior.gutter = gutter;
            interior.column_edges = None;
            fits_across(interior, geometry.width)
                .then_some(())
                .ok_or(NO_ROOM_ACROSS)
        })
    }

    /// The header's or footer's column balance.
    pub fn set_band_balance(&mut self, band: Band, pages: Pages, on: bool) -> Result<(), Refusal> {
        self.change_band(band, pages, |interior, _| {
            if on && interior.columns < 2 {
                return Err(NEEDS_COLUMNS);
            }
            interior.balance = on;
            Ok(())
        })
    }

    /// The header's or footer's tab stops.
    pub fn set_band_tabs(
        &mut self,
        band: Band,
        pages: Pages,
        stops: Vec<TabStop>,
    ) -> Result<(), Refusal> {
        let stops = checked_tabs(stops)?;
        self.change_band(band, pages, |interior, _| {
            one_tab_list(interior, stops);
            Ok(())
        })
    }

    /// Begin on second page: the header or footer leaves the layout's first
    /// page empty.
    pub fn set_from_second_page(&mut self, band: Band, on: bool) {
        match band {
            Band::Header => self.header_from_second_page = on,
            Band::Footer => self.footer_from_second_page = on,
        }
    }

    // ------------------------------------------------------------ helpers

    fn ensure_left_pages(&mut self) {
        if self.left.is_none() {
            self.left = Some(self.right.clone());
            self.header.left.clone_from(&self.header.right);
            self.footer.left.clone_from(&self.footer.right);
        }
    }

    fn page_mut(&mut self, pages: Pages) -> &mut PageGeometry {
        match pages {
            Pages::Left => self.left.as_mut().expect("left pages are made first"),
            _ => &mut self.right,
        }
    }

    fn band_mut(&mut self, band: Band, pages: Pages) -> Option<&mut LayoutBox> {
        let boxes = match band {
            Band::Header => &mut self.header,
            Band::Footer => &mut self.footer,
        };
        match pages {
            Pages::Left => boxes.left.as_mut(),
            _ => boxes.right.as_mut(),
        }
    }

    /// Runs `edit` on a copy for the page kind `pages` names, making left
    /// pages first for Right or Left, then mirrors; keeps the copy only if
    /// every step succeeds.
    fn change(
        &mut self,
        pages: Pages,
        edit: impl FnOnce(&mut Self, Pages) -> Result<(), Refusal>,
    ) -> Result<(), Refusal> {
        let mut next = self.clone();
        match pages {
            Pages::All if next.left.is_some() => return Err(LEFT_PAGES_DIFFER),
            Pages::All => {}
            Pages::Right | Pages::Left => next.ensure_left_pages(),
        }
        edit(&mut next, pages)?;
        next.after_change(pages);
        *self = next;
        Ok(())
    }

    fn change_band(
        &mut self,
        band: Band,
        pages: Pages,
        edit: impl FnOnce(&mut LayoutBox, &PageGeometry) -> Result<(), Refusal>,
    ) -> Result<(), Refusal> {
        if self.band(band, pages).is_none() {
            return Err(NO_BAND);
        }
        self.change(pages, |settings, pages| {
            let geometry = settings.page(pages).clone();
            let interior = settings.band_mut(band, pages).ok_or(NO_BAND)?;
            edit(interior, &geometry)
        })
    }

    /// With Mirror, the other page kind becomes the edited one reflected,
    /// its header and footer too (`1248:1071`).
    fn after_change(&mut self, pages: Pages) {
        if !self.mirror || self.left.is_none() || pages == Pages::All {
            return;
        }
        let width = self.right.width;
        let reflect = |boxes: &mut crate::frames::BandBoxes| {
            let (from, to) = match pages {
                Pages::Left => (boxes.left.as_ref(), &mut boxes.right),
                _ => (boxes.right.as_ref(), &mut boxes.left),
            };
            if let (Some(from), Some(_)) = (from, to.as_ref()) {
                *to = Some(mirrored_box(from, width));
            }
        };
        reflect(&mut self.header);
        reflect(&mut self.footer);
        match pages {
            Pages::Left => {
                let left = self.left.as_ref().expect("checked above");
                self.right = mirrored_page(left);
            }
            _ => self.left = Some(mirrored_page(&self.right)),
        }
    }
}

fn checked_tabs(mut stops: Vec<TabStop>) -> Result<Vec<TabStop>, Refusal> {
    if stops.len() > MAX_TABS {
        return Err(TOO_MANY_TABS);
    }
    for stop in &stops {
        length(stop.position)?;
    }
    stops.sort_by_key(|stop| stop.position);
    Ok(stops)
}

fn check_line_style(style: u16) -> Result<(), Refusal> {
    (1..=LINE_STYLES)
        .contains(&style)
        .then_some(())
        .ok_or(LINE_STYLE)
}

/// Page › Line Numbering (dialog 1130, `1368:0295`; VM LN18): the count of
/// Every N lines is 1 to 99 (0 and 100 are refused), and the "Based on"
/// style is one of the document's (13 characters, the dialog's field). The
/// values are the document's, not a page layout's, and are kept while
/// numbering is off (LN17).
pub(crate) fn set_line_numbering(
    document: &amipro_document::Document,
    numbering: &amipro_document::LineNumbering,
) -> Result<Applied, EditError> {
    if !amipro_document::LineNumbering::INTERVAL_RANGE.contains(&numbering.interval) {
        return Err("Every N lines is 1 to 99");
    }
    let known = numbering.style == document.line_numbering().style
        || document
            .styles()
            .values()
            .any(|style| style.name == numbering.style);
    if numbering.style.chars().count() > 13 || !known {
        return Err("Based on is one of the document's styles");
    }
    if document.line_numbering() == numbering {
        return Err("Line Numbering is already so");
    }
    let mut next = document.clone();
    next.set_line_numbering(numbering.clone());
    Ok(Applied {
        document: next,
        caret: None,
        report: ChangeReport {
            info: true,
            ..ChangeReport::default()
        },
    })
}

#[cfg(test)]
mod tests {
    use amipro_document::{Document, TabKind};

    use super::*;
    use crate::{Command, apply};

    fn standard() -> (Document, LayoutSettings) {
        let document = Document::default();
        let settings = LayoutSettings::of(&document, document.document_layout().id).unwrap();
        (document, settings)
    }

    fn applied(document: &Document, settings: &LayoutSettings) -> Document {
        apply(
            document,
            &Command::ModifyLayout {
                layout: document.document_layout().id,
                settings: settings.clone(),
            },
        )
        .unwrap()
        .document
    }

    fn edges(layout: &LayoutBox) -> Option<Vec<(Twips, Twips)>> {
        layout.column_edges.as_ref().map(|edges| {
            edges
                .iter()
                .map(|edges| (edges.left, edges.right))
                .collect()
        })
    }

    fn uneven(pairs: &[(Twips, Twips)]) -> Vec<amipro_document::ColumnEdges> {
        pairs
            .iter()
            .map(|&(left, right)| amipro_document::ColumnEdges { left, right })
            .collect()
    }

    /// Three columns whose gaps differ (240 and 750), as ruler drags make
    /// them, on a Letter page with one-inch margins.
    fn uneven_three() -> (Document, LayoutSettings) {
        let (document, mut settings) = standard();
        settings.set_columns(Pages::All, 3).unwrap();
        settings
            .right
            .layout
            .set_edges(uneven(&[(0, 2_000), (2_240, 5_000), (5_750, 9_360)]), 9_360);
        (document, settings)
    }

    #[test]
    fn a_margin_moves_only_the_outer_edge_of_uneven_columns() {
        let (document, mut settings) = uneven_three();
        // The left margin moves only the first edge; the other columns
        // stay where they were on the page (`1238:02d0`).
        settings
            .set_margin(Pages::All, MarginSide::Left, 1_800)
            .unwrap();
        assert_eq!(
            edges(&settings.right.layout),
            Some(vec![(0, 1_640), (1_880, 4_640), (5_390, 9_000)])
        );
        settings
            .set_margin(Pages::All, MarginSide::Right, 2_160)
            .unwrap();
        assert_eq!(
            edges(&settings.right.layout),
            Some(vec![(0, 1_640), (1_880, 4_640), (5_390, 8_280)])
        );
        // A margin that leaves the last column no width is refused.
        assert!(
            settings
                .set_margin(Pages::All, MarginSide::Right, 7_000)
                .is_err()
        );
        applied(&document, &settings).validate().unwrap();
        // RU9: two columns have one gap, alike by Ami Pro's test, so the
        // dialog's margin makes them even (only the ruler moves one edge).
        let (_, mut ru9) = standard();
        ru9.set_columns(Pages::All, 2).unwrap();
        ru9.right
            .layout
            .set_edges(uneven(&[(0, 4_050), (4_800, 9_360)]), 9_360);
        assert!(ru9.right.layout.column_edges.is_some());
        ru9.set_margin(Pages::All, MarginSide::Left, 1_800).unwrap();
        assert_eq!(edges(&ru9.right.layout), None);
    }

    #[test]
    fn ruler_drags_set_edges_and_margins_as_ami_pro_writes_them_ru2_ru9_ru10_ru12() {
        let (document, mut settings) = standard();
        settings.set_ruler_columns(None, Pages::All, 2).unwrap();
        // RU2: the gap dragged: both columns' edges move, the gap stays.
        settings
            .set_page_edges(Pages::All, &[(1_440, 7_050), (7_290, 10_800)])
            .unwrap();
        assert_eq!(
            edges(&settings.right.layout),
            Some(vec![(0, 5_610), (5_850, 9_360)])
        );
        // RU9 then RU10: column 1's right edge, then the left margin.
        settings
            .set_page_edges(Pages::All, &[(1_440, 5_490), (6_240, 10_800)])
            .unwrap();
        settings
            .set_page_edges(Pages::All, &[(1_800, 5_490), (6_240, 10_800)])
            .unwrap();
        let layout = &settings.right.layout;
        assert_eq!((layout.margins.left, layout.margins.right), (1_800, 1_440));
        assert_eq!(edges(layout), Some(vec![(0, 3_690), (4_440, 9_000)]));
        // RU11: Cols 3 makes them even with a 240 gutter, not the 750 gap.
        settings.set_ruler_columns(None, Pages::All, 3).unwrap();
        let layout = &settings.right.layout;
        assert_eq!((layout.column_edges.clone(), layout.gutter), (None, 240));
        let even: Vec<_> = layout
            .even_edges(9_000)
            .iter()
            .map(|edges| (edges.left + 1_800, edges.right + 1_800))
            .collect();
        assert_eq!(even, [(1_800, 4_640), (4_880, 7_720), (7_960, 10_800)]);
        // Overlapping edges and edges off the page are refused.
        assert!(
            settings
                .set_page_edges(Pages::All, &[(1_440, 7_000), (6_000, 10_800)])
                .is_err()
        );
        assert!(
            settings
                .set_page_edges(Pages::All, &[(1_440, 7_000), (7_240, 12_300)])
                .is_err()
        );
        applied(&document, &settings).validate().unwrap();
    }

    #[test]
    fn counts_and_gutters_make_columns_even_and_mirror_reverses_them() {
        let (_, mut settings) = uneven_three();
        let mut gutter = settings.clone();
        gutter.set_gutter(Pages::All, 360).unwrap();
        assert_eq!(edges(&gutter.right.layout), None);
        let mut count = settings.clone();
        count.set_columns(Pages::All, 2).unwrap();
        assert_eq!(edges(&count.right.layout), None);
        // Mirror: the left pages' columns are the right ones reversed.
        settings.set_mirror(Pages::Right, true).unwrap();
        let left = settings.left.as_ref().unwrap();
        assert_eq!(
            edges(&left.layout),
            Some(vec![(0, 3_610), (4_360, 7_120), (7_360, 9_360)])
        );
        // A narrower paper moves the last column's right edge.
        let (_, mut paper) = uneven_three();
        paper.set_paper(4).unwrap();
        assert_eq!(
            edges(&paper.right.layout),
            Some(vec![(0, 2_000), (2_240, 5_000), (5_750, 9_028)])
        );
    }

    #[test]
    fn paper_sizes_are_stored_as_ami_pro_stores_them_g11_g12_g13() {
        let (_, mut settings) = standard();
        settings.set_paper(4).unwrap();
        assert_eq!(
            (settings.right.width, settings.right.height),
            (11_908, 16_833)
        );
        settings.set_landscape(true).unwrap();
        assert_eq!(
            (settings.right.width, settings.right.height),
            (16_833, 11_908)
        );
        settings.set_paper(2).unwrap();
        assert_eq!(
            (settings.right.width, settings.right.height),
            (20_160, 12_240)
        );
        assert_eq!((settings.paper, settings.landscape), (2, true));
        settings.set_custom_size(8_640, 12_960).unwrap();
        assert_eq!(settings.paper, CUSTOM_PAPER);
        assert_eq!(settings.set_custom_size(40_000, 12_960), Err(TOO_LONG));
        // One-inch margins leave no room on a page an inch and a half wide.
        assert_eq!(
            settings.set_custom_size(2_160, 12_960),
            Err(PAPER_TOO_SMALL)
        );
        assert_eq!(settings.right.width, 8_640, "a refusal changes nothing");
    }

    #[test]
    fn margins_and_columns_keep_every_column_a_twip_wide() {
        let (_, mut settings) = standard();
        // Letter, one-inch margins: 9,360 between them.
        settings.set_columns(Pages::All, 8).unwrap();
        assert_eq!(settings.right.layout.tabs.len(), 8);
        assert_eq!(
            settings.right.layout.tabs[0].len(),
            12,
            "the box's list stays first"
        );
        // 8 columns and 7 gutters of 240: the left margin can grow to
        // 12,240 − 1,440 − 8 − 1,680 = 9,112.
        settings
            .set_margin(Pages::All, MarginSide::Left, 9_112)
            .unwrap();
        assert_eq!(
            settings.set_margin(Pages::All, MarginSide::Left, 9_113),
            Err(NO_ROOM_ACROSS)
        );
        assert_eq!(settings.right.layout.margins.left, 9_112);
        assert_eq!(settings.set_gutter(Pages::All, 241), Err(NO_ROOM_ACROSS));
        settings.set_columns(Pages::All, 1).unwrap();
        assert_eq!(settings.set_gutter(Pages::All, 300), Err(NEEDS_COLUMNS));
        assert_eq!(settings.set_balance(Pages::All, true), Err(NEEDS_COLUMNS));
        assert_eq!(settings.set_columns(Pages::All, 9), Err(COLUMN_COUNT));
        assert_eq!(
            settings.set_margin(Pages::All, MarginSide::Top, 14_400),
            Err(NO_ROOM_DOWN)
        );
        assert_eq!(
            settings.set_margin(Pages::All, MarginSide::Top, -1),
            Err(NEGATIVE)
        );
    }

    #[test]
    fn a_smaller_top_margin_shrinks_the_headers_own_margins_bottom_first() {
        // New documents: header band margins 0.55 in over 0.05 in (G00).
        let (_, mut settings) = standard();
        let header = settings.header.right.clone().unwrap();
        assert_eq!((header.margins.top, header.margins.bottom), (792, 72));
        settings
            .set_margin(Pages::All, MarginSide::Top, 800)
            .unwrap();
        let header = settings.header.right.clone().unwrap();
        assert_eq!((header.margins.top, header.margins.bottom), (792, 8));
        settings
            .set_margin(Pages::All, MarginSide::Top, 500)
            .unwrap();
        let header = settings.header.right.clone().unwrap();
        assert_eq!((header.margins.top, header.margins.bottom), (500, 0));
        assert_eq!(
            settings.set_band_margin(Band::Header, Pages::All, MarginSide::Bottom, 1),
            Err(BAND_TOO_TALL)
        );
        settings
            .set_band_margin(Band::Footer, Pages::All, MarginSide::Top, 100)
            .unwrap();
        assert_eq!(settings.footer.right.as_ref().unwrap().margins.top, 100);
    }

    #[test]
    fn right_or_left_changes_give_the_layout_left_pages_g15() {
        let (document, mut settings) = standard();
        assert_eq!(settings.pages(), Pages::All);
        settings
            .set_margin(Pages::Left, MarginSide::Left, 2_160)
            .unwrap();
        let left = settings.left.clone().unwrap();
        assert_eq!(left.layout.margins.left, 2_160);
        assert_eq!(settings.right.layout.margins.left, 1_440);
        assert_eq!(settings.pages(), Pages::Right);
        assert_eq!(
            settings.set_margin(Pages::All, MarginSide::Left, 1_000),
            Err(LEFT_PAGES_DIFFER)
        );
        let document = applied(&document, &settings);
        let layout = document.document_layout();
        assert!(layout.header.left.is_some() && layout.footer.left.is_some());
        settings.set_all_pages().unwrap();
        let document = applied(&document, &settings);
        assert!(document.document_layout().left.is_none());
        assert!(document.document_layout().header.left.is_none());
    }

    #[test]
    fn mirror_reflects_the_edited_pages_onto_the_others_g16_g17() {
        let (document, mut settings) = standard();
        assert_eq!(
            settings.set_mirror(Pages::All, true),
            Err(MIRROR_NEEDS_SIDE)
        );
        settings.set_mirror(Pages::Right, true).unwrap();
        // G17: right pages 1.50 in from the left; left pages mirror them.
        settings
            .set_margin(Pages::Right, MarginSide::Left, 2_160)
            .unwrap();
        let left = settings.left.clone().unwrap();
        assert_eq!(
            (left.layout.margins.left, left.layout.margins.right),
            (1_440, 2_160)
        );
        settings
            .set_band_margin(Band::Header, Pages::Right, MarginSide::Left, 1_800)
            .unwrap();
        assert_eq!(settings.header.left.as_ref().unwrap().margins.right, 1_800);
        settings
            .set_page_line_sides(
                Pages::Right,
                Sides {
                    left: true,
                    ..Sides::NONE
                },
            )
            .unwrap();
        assert!(settings.left.as_ref().unwrap().border.unwrap().sides.right);
        assert_eq!(settings.set_all_pages(), Err(MIRROR_IS_ON));
        let document = applied(&document, &settings);
        let layout = document.document_layout();
        assert!(layout.mirror);
        let left_header = document.frame(layout.header.left.unwrap()).unwrap();
        assert_eq!(left_header.style.interior.margins.right, 1_800);
    }

    #[test]
    fn tabs_are_one_sorted_list_for_every_column() {
        let (_, mut settings) = standard();
        settings.set_columns(Pages::All, 2).unwrap();
        settings
            .set_tabs(
                Pages::All,
                vec![
                    TabStop::new(TabKind::Right, 2_000),
                    TabStop::new(TabKind::Left, 500),
                ],
            )
            .unwrap();
        assert_eq!(
            settings.right.layout.tabs,
            [
                vec![
                    TabStop::new(TabKind::Left, 500),
                    TabStop::new(TabKind::Right, 2_000)
                ],
                vec![]
            ]
        );
        let many = vec![TabStop::new(TabKind::Left, 100); 23];
        assert_eq!(settings.set_tabs(Pages::All, many), Err(TOO_MANY_TABS));
    }

    #[test]
    fn page_lines_and_the_column_line_share_one_colour_g07_g09_g10() {
        let (_, mut settings) = standard();
        assert_eq!(settings.set_line_colour(Pages::All, 255), Err(NO_LINE));
        assert_eq!(
            settings.set_page_line_style(Pages::All, 2),
            Err(NO_PAGE_LINES)
        );
        settings
            .set_page_line_sides(Pages::All, Sides::ALL)
            .unwrap();
        let border = settings.right.border.unwrap();
        assert_eq!((border.style, border.position), (1, LinePosition::Inside));
        settings.set_line_colour(Pages::All, 255).unwrap();
        settings.set_column_line(Pages::All, true).unwrap();
        assert_eq!(settings.right.layout.line_between.unwrap().colour, 255);
        settings.set_line_colour(Pages::All, 0xFF_0000).unwrap();
        assert_eq!(settings.right.border.unwrap().colour, 0xFF_0000);
        assert_eq!(
            settings.set_column_line_style(Pages::All, 13),
            Err(LINE_STYLE)
        );
        settings
            .set_page_line_position(Pages::All, LinePosition::Outside)
            .unwrap();
        settings
            .set_page_line_sides(Pages::All, Sides::NONE)
            .unwrap();
        assert!(settings.right.border.is_none());
    }

    #[test]
    fn a_change_lays_the_bands_out_in_the_same_undo_step() {
        let (document, mut settings) = standard();
        settings
            .set_margin(Pages::All, MarginSide::Top, 500)
            .unwrap();
        let result = apply(
            &document,
            &Command::ModifyLayout {
                layout: document.document_layout().id,
                settings: settings.clone(),
            },
        )
        .unwrap();
        let header = document.document_layout().header.right.unwrap();
        assert!(result.report.frames.contains(&header));
        assert_eq!(
            result
                .document
                .frame(header)
                .unwrap()
                .style
                .interior
                .margins
                .bottom,
            0
        );
        // Settings read back from the result are what was applied.
        assert_eq!(
            LayoutSettings::of(&result.document, document.document_layout().id).unwrap(),
            settings
        );
    }
}
