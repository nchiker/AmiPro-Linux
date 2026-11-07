//! The interactive ruler above the pages (`docs/Design/UI_SPEC.md` §19):
//! always live, it shows the caret's box from its [`Ruler`] model (every
//! column, the caret column's tabs and indents, the margins) through the
//! pages' transform, and turns presses, drags, and releases into
//! [`RulerEvent`]s; the app makes each release one command. The upper half
//! holds tabs and indents, the lower half margins and columns; the button
//! at the left end sets the kind of tab a click adds.

use cosmic::iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer, text, widget};
use cosmic::iced::{
    Background, Border, Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Size,
    keyboard, mouse,
};

use amipro_document::{TabKind, TabLeader};

use crate::page_view::PageView;
use crate::ruler::{Half, Marker, Ruler};

/// The ruler's height: two halves of 17 pixels and a pixel above and below.
pub const HEIGHT: f32 = 36.0;
/// The tab-kind button at the left end.
pub const SELECTOR: f32 = 28.0;
/// How near (pixels) the pointer must be to take a marker.
const REACH: f32 = 5.0;
/// How far (pixels) below the ruler a tab must be released to be removed:
/// dragged off onto the page (`10d0:1c0c`).
const OFF: f32 = 8.0;

const PAPER: Color = Color::WHITE;
const MARGIN: Color = Color::from_rgb(0.88, 0.88, 0.88);
const INK: Color = Color::from_rgb(0.25, 0.25, 0.25);
const QUIET: Color = Color::from_rgb(0.7, 0.7, 0.7);
const EDGE: Color = Color::from_rgb(0.55, 0.55, 0.55);
const ACCENT: Color = Color::from_rgb(0.2, 0.45, 0.9);

/// What the ruler tells the app.
#[derive(Clone, Debug, PartialEq)]
pub enum RulerEvent {
    /// The pointer is over a marker (at this position, page twips), or none.
    Hover(Option<(Marker, f32)>),
    /// A marker being dragged is now at this position: the guide line.
    Drag(Marker, f32),
    /// A drag ended at this position; `true` when a tab was dragged off.
    Release(Marker, f32, bool),
    /// A click in the upper half where no marker is: a new tab there.
    AddTab(f32),
    /// The tab-kind button: the next kind.
    NextTabKind,
    /// A drag was abandoned (Esc).
    Cancel,
    /// The right button went down here, before the right-click menu opens
    /// on its release: the marker under it, and the part of the ruler.
    Context { marker: Option<Marker>, part: Part },
    /// From the keyboard (Edit › Go To Ruler): the ruler's line moved here.
    Cursor(f32),
    /// A key acting at the line's position.
    Key(RulerKey, f32),
    /// Esc: back to the text.
    Leave,
}

/// What a key does at the ruler's line (user's guide, ch. 8; `UI_SPEC.md`
/// §19).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RulerKey {
    /// Space: a tab of the chosen kind.
    AddTab,
    /// Del: the tab at the line.
    RemoveTab,
    /// 0, 1, 2, 3: All, First, Rest, Right.
    Indent(Marker),
    /// `[` and `]`: the caret column's left and right edges (the margins in
    /// one column).
    Edge { right: bool },
}

/// A step of the ruler's line, and with Ctrl a larger one (twips).
pub const STEP: f32 = crate::ruler::SNAP;
pub const LARGE_STEP: f32 = 720.0;

/// The parts of the ruler a right-click menu belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Part {
    /// The tab-kind button at the left end.
    Button,
    /// Tabs and indents.
    Upper,
    /// Margins and columns.
    Lower,
}

/// What an item of the ruler's right-click menus does (`UI_SPEC.md` §19).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RulerMenu {
    /// The pressed tab's kind, or its leader.
    TabKind(TabKind),
    TabLeader(TabLeader),
    RemoveTab,
    ClearTabs,
    RemoveRuler,
    /// The kind and leader of the tabs a click adds.
    NewTabKind(TabKind),
    NewTabLeader(TabLeader),
    Columns(u8),
    EvenColumns,
    /// Page Layout… for a page or a band, Frame Properties for a frame.
    BoxSettings,
}

pub const KINDS: [TabKind; 4] = [
    TabKind::Left,
    TabKind::Right,
    TabKind::Centre,
    TabKind::Decimal,
];

pub const LEADERS: [TabLeader; 4] = [
    TabLeader::None,
    TabLeader::Dots,
    TabLeader::Hyphens,
    TabLeader::Line,
];

/// A leader's name in the menus.
pub fn leader_name(leader: TabLeader) -> &'static str {
    match leader {
        TabLeader::None => "No Leader",
        TabLeader::Dots => "Dots",
        TabLeader::Hyphens => "Hyphens",
        TabLeader::Line => "Line",
    }
}

pub struct RulerView<'a, Message> {
    view: PageView,
    /// The page whose ruler shows.
    page: usize,
    scroll_x: f32,
    ruler: Option<Ruler>,
    tab: (TabKind, TabLeader),
    /// The keyboard's line (page twips) while the ruler has the keyboard.
    cursor: Option<f32>,
    on_event: Box<dyn Fn(RulerEvent) -> Message + 'a>,
}

pub fn ruler_view<'a, Message>(
    view: PageView,
    page: usize,
    scroll_x: f32,
    ruler: Option<Ruler>,
    tab: (TabKind, TabLeader),
    on_event: impl Fn(RulerEvent) -> Message + 'a,
) -> RulerView<'a, Message> {
    let page = page.min(view.pages.len().saturating_sub(1));
    RulerView {
        view,
        page,
        scroll_x,
        ruler,
        tab,
        cursor: None,
        on_event: Box::new(on_event),
    }
}

impl<Message> RulerView<'_, Message> {
    /// The ruler takes the keyboard, its line at `cursor` (Edit › Go To
    /// Ruler).
    pub fn keyboard(mut self, cursor: Option<f32>) -> Self {
        self.cursor = cursor;
        self
    }
}

#[derive(Debug, Default)]
struct State {
    /// The marker being dragged, where it was taken (page twips), and
    /// where it is now.
    drag: Option<(Marker, f32, f32)>,
    hover: Option<Marker>,
}

impl<Message> RulerView<'_, Message> {
    fn paper_x(&self) -> f32 {
        self.view
            .pages
            .get(self.page)
            .map_or(0.0, |page| page.x - self.scroll_x)
    }

    /// A page position (twips) as x in pixels within the ruler.
    pub fn at(&self, twips: f32) -> f32 {
        self.paper_x() + twips * self.view.scale
    }

    /// x in pixels within the ruler as a page position (twips).
    pub fn twips(&self, x: f32) -> f32 {
        (x - self.paper_x()) / self.view.scale.max(f32::EPSILON)
    }

    /// Tick positions (x in pixels within the ruler, inch number) for a
    /// ruler `width` wide: one per inch across the page.
    pub fn ticks(&self, width: f32) -> Vec<(f32, u32)> {
        let Some(page) = self.view.pages.get(self.page) else {
            return Vec::new();
        };
        let inch = 1_440.0 * self.view.scale;
        (0..)
            .map(|index| (self.paper_x() + index as f32 * inch, index))
            .take_while(|(x, _)| *x <= self.paper_x() + page.width + 0.5)
            .filter(|(x, _)| *x >= 0.0 && *x <= width)
            .collect()
    }

    /// Which half a point (within the ruler) is in.
    fn half(y: f32) -> Half {
        let middle = HEIGHT / 2.0;
        if y < middle {
            Half::Upper {
                depth: ((y - 1.0) / (middle - 1.0)).clamp(0.0, 1.0),
            }
        } else {
            Half::Lower
        }
    }

    fn hit(&self, position: Point) -> Option<Marker> {
        let ruler = self.ruler.as_ref()?;
        ruler.hit(self.twips(position.x), Self::half(position.y), self.reach())
    }

    /// How far from a marker a pointer takes it, in twips.
    fn reach(&self) -> f32 {
        REACH / self.view.scale.max(f32::EPSILON)
    }
}

/// The glyph of a tab stop: a stem and its foot (left, right, both), and a
/// dot for decimal, standing on `base`.
fn tab_glyph<Renderer: renderer::Renderer>(
    renderer: &mut Renderer,
    x: f32,
    base: f32,
    kind: TabKind,
    leader: TabLeader,
    colour: Color,
) {
    let x = x.round();
    let quad = |renderer: &mut Renderer, bounds: Rectangle| {
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                ..renderer::Quad::default()
            },
            colour,
        );
    };
    quad(
        renderer,
        Rectangle::new(Point::new(x, base - 8.0), Size::new(1.5, 8.0)),
    );
    let (foot_left, foot_width) = match kind {
        TabKind::Left => (x, 5.0),
        TabKind::Right => (x - 4.0, 5.5),
        TabKind::Centre | TabKind::Decimal => (x - 4.0, 9.5),
    };
    quad(
        renderer,
        Rectangle::new(
            Point::new(foot_left, base - 1.5),
            Size::new(foot_width, 1.5),
        ),
    );
    if kind == TabKind::Decimal {
        quad(
            renderer,
            Rectangle::new(Point::new(x + 3.0, base - 6.0), Size::new(2.0, 2.0)),
        );
    }
    if leader != TabLeader::None {
        // The leader as a few dots before the stop.
        for step in 1..=3 {
            quad(
                renderer,
                Rectangle::new(
                    Point::new(x - 2.0 - 3.0 * step as f32, base - 2.0),
                    Size::new(1.0, 1.0),
                ),
            );
        }
    }
}

/// A small triangle of stacked quads, pointing down from `edge` or up to
/// it, centred on `x`.
fn triangle<Renderer: renderer::Renderer>(
    renderer: &mut Renderer,
    x: f32,
    edge: f32,
    down: bool,
    colour: Color,
) {
    const ROWS: usize = 5;
    for row in 0..ROWS {
        let half = (ROWS - row) as f32;
        let y = if down {
            edge + row as f32
        } else {
            edge - 1.0 - row as f32
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle::new(Point::new(x - half, y), Size::new(2.0 * half, 1.0)),
                ..renderer::Quad::default()
            },
            colour,
        );
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for RulerView<'_, Message>
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
        Size::new(Length::Fill, Length::Fixed(HEIGHT))
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fill, Length::Fixed(HEIGHT), Size::ZERO))
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
                if position.x < SELECTOR {
                    shell.publish((self.on_event)(RulerEvent::NextTabKind));
                    return;
                }
                let Some(ruler) = &self.ruler else {
                    return;
                };
                let x = self.twips(position.x);
                if let Some(marker) = self.hit(position) {
                    let at = ruler.position(marker);
                    state.drag = Some((marker, x, at));
                    shell.publish((self.on_event)(RulerEvent::Drag(marker, at)));
                } else if matches!(Self::half(position.y), Half::Upper { .. }) {
                    shell.publish((self.on_event)(RulerEvent::AddTab(x)));
                }
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                // Not captured: the right-click menu around the ruler opens
                // on the release.
                if let Some(position) = cursor.position_in(bounds) {
                    let part = if position.x < SELECTOR {
                        Part::Button
                    } else if matches!(Self::half(position.y), Half::Upper { .. }) {
                        Part::Upper
                    } else {
                        Part::Lower
                    };
                    // Only tabs have a menu of their own in the upper half,
                    // so a tab under an indent marker is the one pressed.
                    let marker = match part {
                        Part::Button => None,
                        Part::Upper => self
                            .ruler
                            .as_ref()
                            .and_then(|ruler| ruler.tab_near(self.twips(position.x), self.reach())),
                        Part::Lower => self.hit(position),
                    };
                    shell.publish((self.on_event)(RulerEvent::Context { marker, part }));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some((marker, from, at)) = state.drag {
                    let (Some(position), Some(ruler)) = (cursor.position(), &self.ruler) else {
                        return;
                    };
                    let x = self.twips(position.x - bounds.x);
                    let moved = ruler.dragged(marker, from, x);
                    if moved != at {
                        state.drag = Some((marker, from, moved));
                        shell.publish((self.on_event)(RulerEvent::Drag(marker, moved)));
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
                        .zip(self.ruler.as_ref())
                        .map(|(marker, ruler)| (marker, ruler.position(marker)));
                    shell.publish((self.on_event)(RulerEvent::Hover(shown)));
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some((marker, _, at)) = state.drag.take() {
                    let below = cursor
                        .position()
                        .is_some_and(|position| position.y > bounds.y + bounds.height + OFF);
                    let above = cursor
                        .position()
                        .is_some_and(|position| position.y < bounds.y);
                    let off = matches!(marker, Marker::Tab(_)) && (below || above);
                    shell.publish((self.on_event)(RulerEvent::Release(marker, at, off)));
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
                if self.cursor.is_some() && state.drag.is_none() =>
            {
                // The ruler has the keyboard: every key is its own.
                shell.capture_event();
                let Some(cursor) = self.cursor else {
                    return;
                };
                let page_width = self.view.pages.get(self.page).map_or(12_240.0, |page| {
                    page.width / self.view.scale.max(f32::EPSILON)
                });
                let step = if modifiers.control() {
                    LARGE_STEP
                } else {
                    STEP
                };
                use keyboard::key::Named;
                let event = match key.as_ref() {
                    keyboard::Key::Named(Named::ArrowLeft) => Some(RulerEvent::Cursor(
                        crate::ruler::snap((cursor - step).max(0.0)),
                    )),
                    keyboard::Key::Named(Named::ArrowRight) => Some(RulerEvent::Cursor(
                        crate::ruler::snap(cursor + step).min(page_width),
                    )),
                    keyboard::Key::Character(" ") => {
                        Some(RulerEvent::Key(RulerKey::AddTab, cursor))
                    }
                    keyboard::Key::Named(Named::Delete) => {
                        Some(RulerEvent::Key(RulerKey::RemoveTab, cursor))
                    }
                    keyboard::Key::Named(Named::Escape) => Some(RulerEvent::Leave),
                    keyboard::Key::Character(character) => {
                        let key = match character {
                            "0" => Some(RulerKey::Indent(Marker::All)),
                            "1" => Some(RulerKey::Indent(Marker::First)),
                            "2" => Some(RulerKey::Indent(Marker::Rest)),
                            "3" => Some(RulerKey::Indent(Marker::Right)),
                            "[" => Some(RulerKey::Edge { right: false }),
                            "]" => Some(RulerKey::Edge { right: true }),
                            _ => None,
                        };
                        key.map(|key| RulerEvent::Key(key, cursor))
                    }
                    _ => None,
                };
                if let Some(event) = event {
                    shell.publish((self.on_event)(event));
                    shell.request_redraw();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) if state.drag.is_some() => {
                state.drag = None;
                shell.publish((self.on_event)(RulerEvent::Cancel));
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
        let marker = state.drag.map(|(marker, _, _)| marker).or_else(|| {
            cursor
                .position_in(layout.bounds())
                .and_then(|position| self.hit(position))
        });
        match marker {
            Some(Marker::Edge { .. } | Marker::Column(_) | Marker::Gap(_)) => {
                mouse::Interaction::ResizingHorizontally
            }
            Some(_) if state.drag.is_some() => mouse::Interaction::Grabbing,
            Some(_) => mouse::Interaction::Grab,
            None if cursor
                .position_in(layout.bounds())
                .is_some_and(|position| position.x < SELECTOR) =>
            {
                mouse::Interaction::Pointer
            }
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
        let Some(page) = self.view.pages.get(self.page) else {
            return;
        };
        let fill = |renderer: &mut Renderer, rect: Rectangle, colour: Color| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: rect,
                    ..renderer::Quad::default()
                },
                colour,
            );
        };
        let middle = bounds.y + HEIGHT / 2.0;
        let paper_left = self.paper_x();
        let paper_right = paper_left + page.width;
        let strip = Rectangle::new(
            Point::new(bounds.x + paper_left.max(0.0), bounds.y + 1.0),
            Size::new(
                (paper_right.min(bounds.width) - paper_left.max(0.0)).max(0.0),
                HEIGHT - 2.0,
            ),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: strip,
                border: Border {
                    color: EDGE,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..renderer::Quad::default()
            },
            MARGIN,
        );
        let clip = |left: f32, right: f32| {
            let left = (bounds.x + left).max(strip.x + 1.0);
            let right = (bounds.x + right).min(strip.x + strip.width - 1.0);
            (right > left).then_some((left, right))
        };
        // The columns white in both halves; margins and gaps stay grey.
        let columns: Vec<(f32, f32)> = self.ruler.as_ref().map_or_else(Vec::new, |ruler| {
            ruler
                .columns
                .iter()
                .map(|(left, right)| (self.at(*left), self.at(*right)))
                .collect()
        });
        for (left, right) in &columns {
            if let Some((left, right)) = clip(*left, *right) {
                fill(
                    renderer,
                    Rectangle::new(
                        Point::new(left, strip.y + 1.0),
                        Size::new(right - left, strip.height - 2.0),
                    ),
                    PAPER,
                );
            }
        }
        fill(
            renderer,
            Rectangle::new(Point::new(strip.x, middle), Size::new(strip.width, 1.0)),
            QUIET,
        );
        // Quarter-inch ticks up from the bottom, the inches longer and
        // numbered in the lower half.
        let quarter = 360.0 * self.view.scale;
        let mut x = paper_left;
        let mut index = 0u32;
        while x <= paper_right + 0.5 && quarter > 2.0 {
            if x >= SELECTOR && x <= bounds.width {
                let length = match index % 4 {
                    0 => 6.0,
                    2 => 4.0,
                    _ => 2.0,
                };
                fill(
                    renderer,
                    Rectangle::new(
                        Point::new(bounds.x + x, bounds.y + HEIGHT - 2.0 - length),
                        Size::new(1.0, length),
                    ),
                    INK,
                );
            }
            x += quarter;
            index += 1;
        }
        for (x, inch) in self.ticks(bounds.width) {
            if inch > 0 && x > SELECTOR + 8.0 {
                renderer.fill_text(
                    text::Text {
                        content: inch.to_string(),
                        bounds: Size::new(24.0, 12.0),
                        size: Pixels(9.0),
                        line_height: text::LineHeight::default(),
                        font: Font::default(),
                        align_x: text::Alignment::Left,
                        align_y: cosmic::iced::alignment::Vertical::Top,
                        shaping: text::Shaping::Basic,
                        wrapping: text::Wrapping::None,
                        ellipsize: text::Ellipsize::None,
                    },
                    // Just right of its tick, clear of a margin's handle.
                    Point::new(bounds.x + x + 3.0, middle + 2.0),
                    INK,
                    bounds,
                );
            }
        }
        let active = state.drag.map(|(marker, _, _)| marker).or(state.hover);
        let colour = |marker: Marker| if active == Some(marker) { ACCENT } else { INK };
        if let Some(ruler) = &self.ruler {
            // Column edges, the margins included, as handles in the lower
            // half.
            if ruler.edits_columns() {
                for (column, (left, right)) in columns.iter().enumerate() {
                    for (x, right_edge) in [(*left, false), (*right, true)] {
                        let marker = Marker::Edge {
                            column,
                            right: right_edge,
                        };
                        if x >= SELECTOR && x <= bounds.width {
                            fill(
                                renderer,
                                Rectangle::new(
                                    Point::new(bounds.x + x - 1.0, middle + 1.0),
                                    Size::new(2.0, HEIGHT / 2.0 - 2.0),
                                ),
                                colour(marker),
                            );
                        }
                    }
                }
            }
            // The caret column's tabs, then its indents.
            let base = middle - 2.0;
            for (index, stop) in ruler.tabs.iter().enumerate() {
                let x = self.at(ruler.position(Marker::Tab(index)));
                if x >= SELECTOR && x <= bounds.width {
                    tab_glyph(
                        renderer,
                        bounds.x + x,
                        base,
                        stop.kind,
                        stop.leader,
                        colour(Marker::Tab(index)),
                    );
                }
            }
            let first = self.at(ruler.position(Marker::First));
            let rest = self.at(ruler.position(Marker::Rest));
            let all = self.at(ruler.position(Marker::All));
            let right = self.at(ruler.position(Marker::Right));
            triangle(
                renderer,
                bounds.x + first,
                bounds.y + 2.0,
                true,
                colour(Marker::First),
            );
            triangle(
                renderer,
                bounds.x + rest,
                middle,
                false,
                colour(Marker::Rest),
            );
            fill(
                renderer,
                Rectangle::new(
                    Point::new(bounds.x + all - 4.0, bounds.y + 8.0),
                    Size::new(8.0, 3.0),
                ),
                colour(Marker::All),
            );
            triangle(
                renderer,
                bounds.x + right,
                middle,
                false,
                colour(Marker::Right),
            );
            // While a marker moves, a guide on the ruler too.
            if let Some((_, _, at)) = state.drag {
                let x = self.at(at);
                fill(
                    renderer,
                    Rectangle::new(
                        Point::new(bounds.x + x, bounds.y + 1.0),
                        Size::new(1.0, HEIGHT - 2.0),
                    ),
                    ACCENT,
                );
            }
        }
        // The keyboard's line, solid across the ruler.
        if let Some(cursor) = self.cursor {
            let x = self.at(cursor);
            fill(
                renderer,
                Rectangle::new(
                    Point::new(bounds.x + x - 1.0, bounds.y + 1.0),
                    Size::new(2.0, HEIGHT - 2.0),
                ),
                INK,
            );
        }
        // The tab-kind button at the left end, over the ruler.
        let button = Rectangle::new(
            Point::new(bounds.x + 2.0, bounds.y + 4.0),
            Size::new(SELECTOR - 4.0, HEIGHT - 8.0),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: button,
                border: Border {
                    color: EDGE,
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..renderer::Quad::default()
            },
            Background::Color(PAPER),
        );
        tab_glyph(
            renderer,
            button.x + button.width / 2.0 - 1.0,
            button.y + button.height / 2.0 + 5.0,
            self.tab.0,
            self.tab.1,
            INK,
        );
    }
}

impl<'a, Message, Theme, Renderer> From<RulerView<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer<Font = Font> + 'a,
{
    fn from(ruler: RulerView<'a, Message>) -> Self {
        Element::new(ruler)
    }
}

/// The next kind of tab the button gives.
pub fn next_kind(kind: TabKind) -> TabKind {
    match kind {
        TabKind::Left => TabKind::Right,
        TabKind::Right => TabKind::Centre,
        TabKind::Centre => TabKind::Decimal,
        TabKind::Decimal => TabKind::Left,
    }
}

/// A kind's name, for the status bar and menus.
pub fn kind_name(kind: TabKind) -> &'static str {
    match kind {
        TabKind::Left => "Left",
        TabKind::Right => "Right",
        TabKind::Centre => "Centre",
        TabKind::Decimal => "Decimal",
    }
}

/// A position from the paper's left edge, as Ami Pro's ruler row showed it.
pub fn inches(twips: f32) -> String {
    format!("{:.2} in", twips / 1_440.0)
}
