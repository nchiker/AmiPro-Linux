//! The page surface's view transform (`docs/Design/UI_SPEC.md`, "Page
//! workspace"): one mapping between twips on a laid-out page and pixels in
//! the scrolled workspace. Drawing, hit testing, the caret, selection, IME,
//! the ruler, and the scroll extents all use it, so zoom changes only the
//! display scale.

use amipro_layout::{Bounds, DocumentLayout};
use cosmic::iced::{Point, Rectangle, Size};

use crate::workspace_geometry::SCREEN_PIXELS_PER_POINT;

/// Pixels per twip at 100%: 96 pixels per inch.
pub const ACTUAL_SIZE: f32 = SCREEN_PIXELS_PER_POINT / 20.0;
/// Space between pages (`UI_SPEC.md`, "Screen geometry").
pub const PAGE_GAP: f32 = 24.0;
/// Space around the pages in the workspace.
pub const PADDING: f32 = 32.0;
/// The zoom percentages Zoom In and Zoom Out step through.
pub const ZOOM_STEPS: &[u16] = &[25, 33, 50, 67, 75, 90, 100, 125, 150, 200, 300, 400];
const MIN_PERCENT: u16 = 25;
const MAX_PERCENT: u16 = 400;
/// Letter, for a layout without pages.
const DEFAULT_PAGE: (f32, f32) = (12_240.0, 15_840.0);

/// The display scale: the widest page fitted to the workspace (never above
/// 100%), or a percentage of actual size.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Zoom {
    #[default]
    FitWidth,
    Percent(u16),
    /// View › Full Page (Ctrl+D): the whole page, or both facing pages, in
    /// the window (`UI_SPEC.md` §39, CM8).
    FullPage,
}

impl Zoom {
    pub fn percent(percent: u16) -> Self {
        Self::Percent(percent.clamp(MIN_PERCENT, MAX_PERCENT))
    }

    /// The next step above `current` (the percentage now shown).
    pub fn zoom_in(current: u16) -> Self {
        Self::percent(
            ZOOM_STEPS
                .iter()
                .copied()
                .find(|step| *step > current)
                .unwrap_or(MAX_PERCENT),
        )
    }

    /// The next step below `current`.
    pub fn zoom_out(current: u16) -> Self {
        Self::percent(
            ZOOM_STEPS
                .iter()
                .rev()
                .copied()
                .find(|step| *step < current)
                .unwrap_or(MIN_PERCENT),
        )
    }
}

/// Where every page of a layout is in the workspace, in pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct PageView {
    /// Pixels per twip.
    pub scale: f32,
    /// Each page's rectangle in workspace pixels.
    pub pages: Vec<Rectangle>,
    /// The workspace content: every page with the padding around them.
    pub size: Size,
}

impl PageView {
    /// Stacks the pages vertically, centred in `viewport` when they fit and
    /// anchored to the leading padding when they do not.
    #[cfg(test)]
    pub fn new(layout: &DocumentLayout, zoom: Zoom, viewport: Size) -> Self {
        Self::arranged(layout, zoom, viewport, false)
    }

    /// The pages one under another, or with `facing` in pairs as Ami Pro's
    /// Facing Pages shows them (CM8, CK8): left pages even, right pages
    /// odd, so page 1 stands alone on the right.
    pub fn arranged(layout: &DocumentLayout, zoom: Zoom, viewport: Size, facing: bool) -> Self {
        let sizes: Vec<(f32, f32)> = if layout.pages.is_empty() {
            vec![DEFAULT_PAGE]
        } else {
            layout
                .pages
                .iter()
                .map(|page| (page.width, page.height))
                .collect()
        };
        let widest = sizes.iter().map(|size| size.0).fold(1.0, f32::max);
        let tallest = sizes.iter().map(|size| size.1).fold(1.0, f32::max);
        // Facing pages: two pages across with the gap (pixels) between.
        let (across, gap) = if facing {
            (2.0 * widest, PAGE_GAP)
        } else {
            (widest, 0.0)
        };
        let smallest = ACTUAL_SIZE * f32::from(MIN_PERCENT) / 100.0;
        let fit_width = (viewport.width - 2.0 * PADDING - gap).max(1.0) / across;
        let scale = match zoom {
            Zoom::FitWidth => fit_width.clamp(smallest, ACTUAL_SIZE),
            Zoom::FullPage => fit_width
                .min((viewport.height - 2.0 * PADDING).max(1.0) / tallest)
                .clamp(smallest, ACTUAL_SIZE * f32::from(MAX_PERCENT) / 100.0),
            Zoom::Percent(percent) => {
                ACTUAL_SIZE * f32::from(percent.clamp(MIN_PERCENT, MAX_PERCENT)) / 100.0
            }
        };
        let content = if facing {
            2.0 * widest * scale + PAGE_GAP
        } else {
            widest * scale
        };
        let width = (content + 2.0 * PADDING).max(viewport.width);
        let centre = width / 2.0;
        let mut y = PADDING;
        let mut pages = Vec::with_capacity(sizes.len());
        if facing {
            // Page n (1-based) is on the right when odd; a row is a left
            // page and the right page after it.
            let mut row_height = 0.0_f32;
            for (index, (page_width, page_height)) in sizes.iter().enumerate() {
                let right = index % 2 == 0;
                let size = Size::new(page_width * scale, page_height * scale);
                let x = if right {
                    centre + PAGE_GAP / 2.0
                } else {
                    centre - PAGE_GAP / 2.0 - size.width
                };
                pages.push(Rectangle::new(Point::new(x, y), size));
                row_height = row_height.max(size.height);
                if right {
                    y += row_height + PAGE_GAP;
                    row_height = 0.0;
                }
            }
            if row_height > 0.0 {
                y += row_height + PAGE_GAP;
            }
        } else {
            for (page_width, page_height) in &sizes {
                let page = Rectangle::new(
                    Point::new((width - page_width * scale) / 2.0, y),
                    Size::new(page_width * scale, page_height * scale),
                );
                y += page.height + PAGE_GAP;
                pages.push(page);
            }
        }
        Self {
            scale,
            pages,
            size: Size::new(width, y - PAGE_GAP + PADDING),
        }
    }

    /// The zoom percentage shown.
    pub fn percent(&self) -> u16 {
        (self.scale / ACTUAL_SIZE * 100.0).round() as u16
    }

    /// A point on page `page` (0-based), in twips, as workspace pixels.
    pub fn point(&self, page: usize, x: f32, y: f32) -> Point {
        let origin = self.pages[page].position();
        Point::new(origin.x + x * self.scale, origin.y + y * self.scale)
    }

    pub fn rect(&self, page: usize, bounds: Bounds) -> Rectangle {
        Rectangle::new(
            self.point(page, bounds.x, bounds.y),
            Size::new(bounds.width * self.scale, bounds.height * self.scale),
        )
    }

    /// The page nearest to `point` vertically: the gaps belong half to the
    /// page above and half to the page below.
    pub fn page_at(&self, point: Point) -> usize {
        let row = self
            .pages
            .iter()
            .position(|page| point.y < page.y + page.height + PAGE_GAP / 2.0)
            .unwrap_or(self.pages.len().saturating_sub(1));
        // Facing pages: of the pages on that row, the one nearest across.
        let Some(top) = self.pages.get(row).map(|page| page.y) else {
            return row;
        };
        let distance = |page: &Rectangle| {
            if point.x < page.x {
                page.x - point.x
            } else {
                (point.x - (page.x + page.width)).max(0.0)
            }
        };
        self.pages
            .iter()
            .enumerate()
            .filter(|(_, page)| (page.y - top).abs() < 0.5)
            .min_by(|(_, a), (_, b)| distance(a).total_cmp(&distance(b)))
            .map_or(row, |(index, _)| index)
    }

    /// `point` in twips from the top-left corner of page `page`.
    pub fn to_page(&self, page: usize, point: Point) -> (f32, f32) {
        let origin = self.pages[page].position();
        (
            (point.x - origin.x) / self.scale,
            (point.y - origin.y) / self.scale,
        )
    }

    /// The pages that intersect `viewport`.
    pub fn visible(&self, viewport: Rectangle) -> impl Iterator<Item = usize> + '_ {
        self.pages
            .iter()
            .enumerate()
            .filter(move |(_, page)| page.intersects(&viewport))
            .map(|(index, _)| index)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use amipro_document::{Document, PageSide};
    use amipro_layout::PageBox;

    use super::*;

    fn layout(sizes: &[(f32, f32)]) -> DocumentLayout {
        let layout_id = Document::default().document_layout().id;
        DocumentLayout {
            pages: sizes
                .iter()
                .enumerate()
                .map(|(index, (width, height))| {
                    Arc::new(PageBox {
                        number: index as u32 + 1,
                        shown: index as u32 + 1,
                        layout: layout_id,
                        side: PageSide::Right,
                        width: *width,
                        height: *height,
                        columns: Vec::new(),
                        frames: Vec::new(),
                        lines: Vec::new(),
                        strokes: Vec::new(),
                        footnote_separator: None,
                    })
                })
                .collect(),
            line_numbers: Vec::new(),
        }
    }

    #[test]
    fn pages_stack_with_gaps_and_centre_when_they_fit() {
        let letter = layout(&[(12_240.0, 15_840.0), (12_240.0, 15_840.0)]);
        let view = PageView::new(&letter, Zoom::Percent(100), Size::new(1_200.0, 800.0));
        let near = |a: f32, b: f32| (a - b).abs() < 0.01;
        assert_eq!(view.percent(), 100);
        // 8.5 × 11 in at 96 pixels per inch.
        assert!(near(view.pages[0].width, 816.0) && near(view.pages[0].height, 1_056.0));
        assert!(near(view.pages[0].x, (1_200.0 - 816.0) / 2.0));
        assert_eq!(view.pages[0].y, PADDING);
        assert!(near(view.pages[1].y, PADDING + 1_056.0 + PAGE_GAP));
        assert_eq!(view.size.width, 1_200.0);
        assert!(near(
            view.size.height,
            2.0 * PADDING + 2.0 * 1_056.0 + PAGE_GAP
        ));
    }

    #[test]
    fn a_page_wider_than_the_workspace_anchors_to_the_padding() {
        let landscape = layout(&[(15_840.0, 12_240.0), (12_240.0, 15_840.0)]);
        let view = PageView::new(&landscape, Zoom::Percent(200), Size::new(800.0, 600.0));
        assert_eq!(
            view.pages[0].x, PADDING,
            "both edges are reachable by scrolling"
        );
        assert_eq!(view.size.width, 15_840.0 * view.scale + 2.0 * PADDING);
        // The narrower portrait page is centred on the landscape one.
        assert_eq!(
            view.pages[1].center_x(),
            view.pages[0].center_x(),
            "pages share one centre line"
        );
    }

    #[test]
    fn fit_width_never_exceeds_actual_size_or_falls_below_the_minimum() {
        let letter = layout(&[(12_240.0, 15_840.0)]);
        let wide = PageView::new(&letter, Zoom::FitWidth, Size::new(3_000.0, 800.0));
        assert_eq!(wide.percent(), 100);
        let narrow = PageView::new(&letter, Zoom::FitWidth, Size::new(300.0, 800.0));
        assert_eq!(narrow.percent(), 29, "(300 − 64) / 816");
        let tiny = PageView::new(&letter, Zoom::FitWidth, Size::new(80.0, 800.0));
        assert_eq!(tiny.percent(), 25);
    }

    #[test]
    fn points_round_trip_through_the_transform_at_every_zoom() {
        let pages = layout(&[(12_240.0, 15_840.0), (11_906.0, 16_838.0)]);
        for percent in [25, 80, 100, 125, 200, 400] {
            let view = PageView::new(&pages, Zoom::Percent(percent), Size::new(1_000.0, 700.0));
            for page in 0..2 {
                let point = view.point(page, 1_440.0, 2_880.0);
                assert_eq!(view.page_at(point), page);
                let (x, y) = view.to_page(page, point);
                assert!((x - 1_440.0).abs() < 0.01 && (y - 2_880.0).abs() < 0.01);
            }
            let gap_top = view.pages[0].y + view.pages[0].height + 1.0;
            assert_eq!(view.page_at(Point::new(0.0, gap_top)), 0);
            let gap_bottom = view.pages[1].y - 1.0;
            assert_eq!(view.page_at(Point::new(0.0, gap_bottom)), 1);
        }
    }

    #[test]
    fn zoom_steps_up_and_down_within_the_limits() {
        assert_eq!(Zoom::zoom_in(100), Zoom::Percent(125));
        assert_eq!(Zoom::zoom_in(110), Zoom::Percent(125));
        assert_eq!(Zoom::zoom_out(100), Zoom::Percent(90));
        assert_eq!(Zoom::zoom_in(400), Zoom::Percent(400));
        assert_eq!(Zoom::zoom_out(25), Zoom::Percent(25));
        assert_eq!(Zoom::percent(1_000), Zoom::Percent(400));
    }

    #[test]
    fn only_pages_in_the_viewport_are_visible() {
        let pages = layout(&[(12_240.0, 15_840.0); 4]);
        let view = PageView::new(&pages, Zoom::Percent(50), Size::new(800.0, 600.0));
        let viewport = Rectangle::new(
            Point::new(0.0, view.pages[1].y + 10.0),
            Size::new(800.0, 100.0),
        );
        assert_eq!(view.visible(viewport).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn facing_pages_pair_even_left_and_odd_right_with_page_one_alone() {
        let pages = layout(&[(12_240.0, 15_840.0); 4]);
        let view = PageView::arranged(&pages, Zoom::Percent(50), Size::new(1_200.0, 800.0), true);
        let [one, two, three, four] = [0, 1, 2, 3].map(|index| view.pages[index]);
        let centre = view.size.width / 2.0;
        // Page 1 on the right of the first row; 2 and 3 side by side below.
        assert!(one.x > centre && two.x + two.width < centre);
        assert!((two.y - three.y).abs() < 0.01 && two.y > one.y + one.height);
        assert!(three.x > centre && four.x + four.width < centre);
        assert!(four.y > three.y);
        // A click finds the page across, not only down.
        let on_two = Point::new(two.x + 10.0, two.y + 10.0);
        let on_three = Point::new(three.x + 10.0, three.y + 10.0);
        assert_eq!(view.page_at(on_two), 1);
        assert_eq!(view.page_at(on_three), 2);
        // Full Page fits a whole page in the window, one or a pair across.
        let full = PageView::arranged(&pages, Zoom::FullPage, Size::new(1_200.0, 800.0), false);
        assert!(full.pages[0].height <= 800.0 - 2.0 * PADDING + 0.5);
        let pair = PageView::arranged(&pages, Zoom::FullPage, Size::new(1_400.0, 2_000.0), true);
        assert!(pair.pages[1].width * 2.0 + PAGE_GAP <= 1_400.0 - 2.0 * PADDING + 0.5);
        assert!(pair.pages[1].width * 2.0 + PAGE_GAP >= 1_400.0 - 2.0 * PADDING - 0.5);
    }
}
