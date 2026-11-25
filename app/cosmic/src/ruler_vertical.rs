//! View › Vertical Ruler (`docs/Design/UI_SPEC.md` §48; CK11): a ruler
//! along the workspace's left edge that scrolls with the pages and scales
//! with the zoom. For each page it marks the top and bottom margins (the
//! header's and footer's areas) and the active frame's or table's vertical
//! extent, and its two margin markers drag: a release is the same
//! `ModifyLayout` command the horizontal ruler's margin handles make, for
//! the same pages of the page layout (`ruler::pages_of`), snapped to the
//! same grid. Ami Pro's vertical ruler (`RULER_NOTES.md`; View Preferences
//! bit 0x200) showed the top and bottom margins and dragged them.

use amipro_document::{Document, LayoutId, PageSide, Twips};
use amipro_editing::layout_setup::{MarginSide, Pages};
use amipro_editing::{Command, LayoutSettings};
use amipro_layout::{Container, DocumentLayout};

use cosmic::iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer, text, widget};
use cosmic::iced::{Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Size};
use cosmic::iced::{keyboard, mouse};

use crate::page_editing::PageSelection;
use crate::page_view::PageView;
use crate::ruler::snap;

/// The ruler's width in pixels.
pub const WIDTH: f32 = 22.0;
/// How near (pixels) the pointer must be to take a marker.
const REACH: f32 = 5.0;
/// The least height the text area keeps between the markers.
const LEAST_BODY: f32 = 720.0;

// The horizontal ruler's colours, so the two read as one instrument.
const PAPER: Color = Color::WHITE;
const MARGIN: Color = Color::from_rgb(0.88, 0.88, 0.88);
const BAND: Color = Color::from_rgb(0.80, 0.85, 0.93);
const INK: Color = Color::from_rgb(0.25, 0.25, 0.25);
const EDGE: Color = Color::from_rgb(0.55, 0.55, 0.55);
const ACCENT: Color = Color::from_rgb(0.2, 0.45, 0.9);

/// One of the two margin markers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Edge {
    /// Where the top margin (the header's area) ends.
    Top,
    /// Where the bottom margin (the footer's area) starts.
    Bottom,
}

impl Edge {
    pub fn name(self) -> &'static str {
        match self {
            Self::Top => "Top margin",
            Self::Bottom => "Bottom margin",
        }
    }
}

/// What the vertical ruler tells the app.
#[derive(Clone, Debug, PartialEq)]
pub enum VerticalEvent {
    /// The pointer is over a marker (page index, position in page twips from
    /// the top), or none.
    Hover(Option<(Edge, usize, f32)>),
    /// A marker being dragged is now at this position.
    Drag(Edge, usize, f32),
    /// A drag ended at this position.
    Release(Edge, usize, f32),
    /// A drag was abandoned (Esc).
    Cancel,
}

/// One page as the vertical ruler shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct PageMargins {
    pub layout: LayoutId,
    pub pages: Pages,
    pub height: f32,
    /// The top and bottom margins (twips).
    pub top: f32,
    pub bottom: f32,
    /// Whether the page has a header and a footer in its layout.
    pub header: bool,
    pub footer: bool,
}

impl PageMargins {
    /// A marker's place from the page's top.
    pub fn position(&self, edge: Edge) -> f32 {
        match edge {
            Edge::Top => self.top,
            Edge::Bottom => self.height - self.bottom,
        }
    }

    /// The margin a marker at `at` makes.
    pub fn margin(&self, edge: Edge, at: f32) -> f32 {
        match edge {
            Edge::Top => at,
            Edge::Bottom => self.height - at,
        }
    }
}

/// What the vertical ruler shows: every page's margins, and the extent of
/// the box the caret is in when it is a frame or table.
#[derive(Clone, Debug, PartialEq)]
pub struct VerticalRuler {
    pub pages: Vec<PageMargins>,
    /// The caret's frame or table on the caret's page: page index, top, and
    /// height (twips).
    pub active: Option<(usize, f32, f32)>,
}

impl VerticalRuler {
    pub fn at(document: &Document, layout: &DocumentLayout, selection: PageSelection) -> Self {
        let pages = layout
            .pages
            .iter()
            .map(|page| {
                let page_layout = document
                    .layout(page.layout)
                    .unwrap_or_else(|| document.document_layout());
                let geometry = match (&page_layout.left, page.side) {
                    (Some(left), PageSide::Left) => left,
                    _ => &page_layout.right,
                };
                let side = |frames: &amipro_document::SideFrames| {
                    if page.side == PageSide::Left && page_layout.left.is_some() {
                        frames.left
                    } else {
                        frames.right
                    }
                    .is_some()
                };
                PageMargins {
                    layout: page.layout,
                    pages: crate::ruler::pages_of(document, page.layout, page.side),
                    height: page.height,
                    top: geometry.layout.margins.top as f32,
                    bottom: geometry.layout.margins.bottom as f32,
                    header: side(&page_layout.header),
                    footer: side(&page_layout.footer),
                }
            })
            .collect();
        Self {
            pages,
            active: Self::active_box(document, layout, selection),
        }
    }

    fn active_box(
        document: &Document,
        layout: &DocumentLayout,
        selection: PageSelection,
    ) -> Option<(usize, f32, f32)> {
        let focus = selection.focus;
        let number = crate::page_surface::caret_page(document, layout, selection);
        let index = layout.pages.iter().position(|page| page.number == number)?;
        let page = &layout.pages[index];
        let line = page
            .lines
            .iter()
            .find(|line| line.flow == focus.flow && line.paragraph_index == focus.paragraph)
            .or_else(|| page.lines.iter().rfind(|line| line.flow == focus.flow))?;
        let frame = match line.container {
            Container::Frame(id) | Container::Cell { frame: id, .. } => id,
            _ => return None,
        };
        let bounds = page
            .frames
            .iter()
            .filter(|frame_box| frame_box.frame == frame)
            .map(|frame_box| frame_box.bounds)
            .find(|bounds| bounds.height > 0.0)?;
        Some((index, bounds.y, bounds.height))
    }

    /// Where a marker taken at `from` and dragged to `to` lands: moved by
    /// the pointer, truncated to the horizontal ruler's grid, and kept
    /// clear of the other marker.
    pub fn dragged(&self, page: usize, edge: Edge, from: f32, to: f32) -> f32 {
        let Some(margins) = self.pages.get(page) else {
            return from;
        };
        let (low, high) = match edge {
            Edge::Top => (0.0, margins.position(Edge::Bottom) - LEAST_BODY),
            Edge::Bottom => (margins.top + LEAST_BODY, margins.height),
        };
        if high < low {
            return margins.position(edge);
        }
        snap(margins.position(edge) + to - from).clamp(low, high)
    }

    /// The command a drag of `edge` on `page` to `at` makes (the page
    /// layout's margin for the page's pages, as the horizontal ruler's
    /// margins and Page Layout Properties make it), or `None` when nothing
    /// changes.
    pub fn release(
        &self,
        document: &Document,
        page: usize,
        edge: Edge,
        at: f32,
    ) -> Option<Command> {
        let margins = self.pages.get(page)?;
        let value = margins.margin(edge, at).round() as Twips;
        if value == margins.margin(edge, margins.position(edge)).round() as Twips {
            return None;
        }
        let mut settings = LayoutSettings::of(document, margins.layout)?;
        let side = match edge {
            Edge::Top => MarginSide::Top,
            Edge::Bottom => MarginSide::Bottom,
        };
        settings.set_margin(margins.pages, side, value).ok()?;
        (settings != LayoutSettings::of(document, margins.layout)?).then_some(
            Command::ModifyLayout {
                layout: margins.layout,
                settings,
            },
        )
    }
}

/// The widget: draws the model through the pages' transform and turns the
/// pointer into [`VerticalEvent`]s.
pub struct VerticalRulerView<'a, Message> {
    view: PageView,
    scroll_y: f32,
    ruler: VerticalRuler,
    on_event: Box<dyn Fn(VerticalEvent) -> Message + 'a>,
}

pub fn vertical_ruler_view<'a, Message>(
    view: PageView,
    scroll_y: f32,
    ruler: VerticalRuler,
    on_event: impl Fn(VerticalEvent) -> Message + 'a,
) -> VerticalRulerView<'a, Message> {
    VerticalRulerView {
        view,
        scroll_y,
        ruler,
        on_event: Box::new(on_event),
    }
}

#[derive(Debug, Default)]
struct State {
    /// The marker being dragged: its page, where it was taken (page twips),
    /// and where it is now.
    drag: Option<(Edge, usize, f32, f32)>,
    hover: Option<(Edge, usize)>,
}

impl<Message> VerticalRulerView<'_, Message> {
    /// A page's top as y in pixels within the ruler.
    pub fn top_of(&self, page: usize) -> f32 {
        self.view
            .pages
            .get(page)
            .map_or(0.0, |rect| rect.y - self.scroll_y)
    }

    /// A page position (twips from its top) as y within the ruler.
    pub fn at(&self, page: usize, twips: f32) -> f32 {
        self.top_of(page) + twips * self.view.scale
    }

    /// y within the ruler as twips from page `page`'s top.
    pub fn twips(&self, page: usize, y: f32) -> f32 {
        (y - self.top_of(page)) / self.view.scale.max(f32::EPSILON)
    }

    /// The marker under a point within the ruler.
    pub fn hit(&self, position: Point) -> Option<(Edge, usize)> {
        self.ruler
            .pages
            .iter()
            .enumerate()
            .take(self.view.pages.len())
            .flat_map(|(page, margins)| {
                [Edge::Top, Edge::Bottom]
                    .into_iter()
                    .map(move |edge| (edge, page, margins.position(edge)))
            })
            .map(|(edge, page, twips)| (edge, page, (self.at(page, twips) - position.y).abs()))
            .filter(|(_, _, distance)| *distance <= REACH)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(edge, page, _)| (edge, page))
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for VerticalRulerView<'_, Message>
where
    Renderer: text::Renderer<Font = Font>,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(WIDTH), Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fixed(WIDTH), Length::Fill, Size::ZERO))
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_in(bounds) else {
                    return;
                };
                shell.capture_event();
                if let Some((edge, page)) = self.hit(position) {
                    let from = self.twips(page, position.y);
                    let at = self.ruler.pages[page].position(edge);
                    state.drag = Some((edge, page, from, at));
                    shell.publish((self.on_event)(VerticalEvent::Drag(edge, page, at)));
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some((edge, page, from, at)) = state.drag {
                    let Some(position) = cursor.position() else {
                        return;
                    };
                    let y = self.twips(page, position.y - bounds.y);
                    let moved = self.ruler.dragged(page, edge, from, y);
                    if moved != at {
                        state.drag = Some((edge, page, from, moved));
                        shell.publish((self.on_event)(VerticalEvent::Drag(edge, page, moved)));
                        shell.request_redraw();
                    }
                    shell.capture_event();
                    return;
                }
                let hover = cursor
                    .position_in(bounds)
                    .and_then(|position| self.hit(position));
                if hover != state.hover {
                    state.hover = hover;
                    let shown = hover
                        .map(|(edge, page)| (edge, page, self.ruler.pages[page].position(edge)));
                    shell.publish((self.on_event)(VerticalEvent::Hover(shown)));
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some((edge, page, _, at)) = state.drag.take() {
                    shell.publish((self.on_event)(VerticalEvent::Release(edge, page, at)));
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) if state.drag.is_some() => {
                state.drag = None;
                shell.publish((self.on_event)(VerticalEvent::Cancel));
                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.drag.is_some() {
            return mouse::Interaction::ResizingVertically;
        }
        match cursor
            .position_in(layout.bounds())
            .and_then(|position| self.hit(position))
        {
            Some(_) => mouse::Interaction::ResizingVertically,
            None => mouse::Interaction::default(),
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let scale = self.view.scale;
        // A rectangle of the ruler clipped to its height.
        let fill = |renderer: &mut Renderer, x: f32, top: f32, width: f32, height: f32, colour| {
            // Whole device rows, so the lines stay crisp.
            let edge = |y: f32| (bounds.y + y).round() - bounds.y;
            let (from, to) = (edge(top).max(0.0), edge(top + height).min(bounds.height));
            if to > from {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(bounds.x + x, bounds.y + from),
                            Size::new(width, to - from),
                        ),
                        ..renderer::Quad::default()
                    },
                    colour,
                );
            }
        };
        let active = state
            .drag
            .map(|(edge, page, _, _)| (edge, page))
            .or(state.hover);
        for (index, margins) in self.ruler.pages.iter().enumerate() {
            let Some(rect) = self.view.pages.get(index) else {
                break;
            };
            let top = self.top_of(index);
            let height = rect.height;
            if top > bounds.height || top + height < 0.0 {
                continue;
            }
            let strip = |renderer: &mut Renderer, from: f32, to: f32, colour| {
                fill(
                    renderer,
                    1.0,
                    top + from * scale,
                    WIDTH - 2.0,
                    (to - from) * scale,
                    colour,
                );
            };
            // The page's edge, then the margins grey (the header's and
            // footer's areas tinted) and the text area white.
            fill(renderer, 0.0, top - 1.0, WIDTH, height + 2.0, EDGE);
            strip(renderer, 0.0, margins.height, MARGIN);
            let (body_top, body_bottom) = (margins.top, margins.height - margins.bottom);
            strip(renderer, body_top, body_bottom, PAPER);
            if margins.header {
                strip(renderer, 0.0, body_top, BAND);
            }
            if margins.footer {
                strip(renderer, body_bottom, margins.height, BAND);
            }
            // The caret's frame or table: a bar down the ruler's inner side.
            if let Some((page, y, extent)) = self.ruler.active
                && page == index
            {
                fill(
                    renderer,
                    1.0,
                    top + y * scale,
                    3.0,
                    (extent * scale).max(1.0),
                    ACCENT,
                );
            }
            // Quarter-inch ticks from the page's top, the inches longer
            // and numbered.
            let quarter = 360.0 * scale;
            if quarter > 2.0 {
                let mut step = 0u32;
                while (step as f32) * 360.0 <= margins.height {
                    let y = top + step as f32 * quarter;
                    if y >= -1.0 && y <= bounds.height {
                        let length = match step % 4 {
                            0 => 8.0,
                            2 => 5.0,
                            _ => 3.0,
                        };
                        fill(renderer, WIDTH - 1.0 - length, y, length, 1.0, INK);
                        if step.is_multiple_of(4) && step > 0 && y > 2.0 {
                            renderer.fill_text(
                                text::Text {
                                    content: (step / 4).to_string(),
                                    bounds: Size::new(WIDTH - 10.0, 12.0),
                                    size: Pixels(9.0),
                                    line_height: text::LineHeight::default(),
                                    font: Font::default(),
                                    align_x: text::Alignment::Left,
                                    align_y: cosmic::iced::alignment::Vertical::Top,
                                    shaping: text::Shaping::Basic,
                                    wrapping: text::Wrapping::None,
                                    ellipsize: text::Ellipsize::None,
                                },
                                Point::new(bounds.x + 4.0, bounds.y + y + 2.0),
                                INK,
                                bounds,
                            );
                        }
                    }
                    step += 1;
                }
            }
            // The two margin markers.
            for edge in [Edge::Top, Edge::Bottom] {
                let at = match state.drag {
                    Some((dragged, page, _, at)) if dragged == edge && page == index => at,
                    _ => margins.position(edge),
                };
                let colour = if active == Some((edge, index)) {
                    ACCENT
                } else {
                    INK
                };
                fill(
                    renderer,
                    1.0,
                    top + at * scale - 1.0,
                    WIDTH - 2.0,
                    2.0,
                    colour,
                );
            }
        }
    }
}

impl<'a, Message, Theme, Renderer> From<VerticalRulerView<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer<Font = Font> + 'a,
{
    fn from(ruler: VerticalRulerView<'a, Message>) -> Self {
        Element::new(ruler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page_layout::SessionPages;

    fn letter() -> (Document, SessionPages) {
        let document = Document::from_plain_text("One\nTwo\nThree");
        let mut pages = SessionPages::default();
        pages.update(&document);
        (document, pages)
    }

    fn caret(document: &Document) -> PageSelection {
        PageSelection::caret(
            amipro_editing::TextPosition {
                flow: document.main_flow_id(),
                paragraph: 0,
                byte: 0,
            },
            1,
        )
    }

    #[test]
    fn a_page_shows_its_margins_and_a_marker_is_where_the_text_area_starts() {
        let (document, pages) = letter();
        let ruler = VerticalRuler::at(&document, pages.layout(), caret(&document));
        let page = &ruler.pages[0];
        assert_eq!(
            (page.height, page.top, page.bottom),
            (15_840.0, 1_440.0, 1_440.0)
        );
        assert_eq!(page.position(Edge::Top), 1_440.0);
        assert_eq!(page.position(Edge::Bottom), 14_400.0);
        assert_eq!(page.margin(Edge::Bottom, 14_220.0), 1_620.0);
        assert_eq!(ruler.active, None);
    }

    #[test]
    fn a_drag_snaps_to_the_grid_and_keeps_the_markers_apart() {
        let (document, pages) = letter();
        let ruler = VerticalRuler::at(&document, pages.layout(), caret(&document));
        // Taken at 1.0 in and moved 0.3 in: truncated to the 1/16 in grid.
        let moved = ruler.dragged(0, Edge::Top, 1_440.0, 1_440.0 + 440.0);
        assert_eq!(moved, 1_800.0);
        // Far down: stopped a body's least height above the bottom marker.
        let moved = ruler.dragged(0, Edge::Top, 1_440.0, 20_000.0);
        assert_eq!(moved, 14_400.0 - LEAST_BODY);
        assert_eq!(ruler.dragged(0, Edge::Top, 1_440.0, -5_000.0), 0.0);
        assert_eq!(ruler.dragged(0, Edge::Bottom, 14_400.0, 99_000.0), 15_840.0);
    }

    #[test]
    fn a_release_sets_the_layouts_margin_as_page_layout_properties_does() {
        let (document, pages) = letter();
        let ruler = VerticalRuler::at(&document, pages.layout(), caret(&document));
        assert_eq!(ruler.release(&document, 0, Edge::Top, 1_440.0), None);
        let command = ruler.release(&document, 0, Edge::Top, 1_080.0).unwrap();
        let changed = amipro_editing::apply(&document, &command).unwrap().document;
        assert_eq!(changed.document_layout().right.layout.margins.top, 1_080);
        let command = ruler.release(&document, 0, Edge::Bottom, 14_760.0).unwrap();
        let changed = amipro_editing::apply(&document, &command).unwrap().document;
        assert_eq!(changed.document_layout().right.layout.margins.bottom, 1_080);
        // A marker past its neighbour is refused, not applied.
        assert_eq!(ruler.release(&document, 0, Edge::Top, 15_000.0), None);
    }
}
