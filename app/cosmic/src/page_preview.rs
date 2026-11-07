//! A miniature of a laid-out page (File › New…'s preview, `UI_SPEC.md`
//! §39, CM1): the paper, its columns, its frames' outlines, and each line
//! of text as a grey bar, scaled to fit a box. Drawn from the layout
//! engine's page, so it shows what the document would look like, not a
//! picture of it.

use cosmic::iced::advanced::{Layout, Widget, layout, renderer, widget};
use cosmic::iced::{Border, Color, Element, Length, Point, Rectangle, Size, mouse};

/// One page in page points (1/72 in), as the layout engine set it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Miniature {
    pub width: f32,
    pub height: f32,
    pub columns: Vec<Rectangle>,
    pub frames: Vec<Rectangle>,
    /// Each line's set width and height.
    pub lines: Vec<Rectangle>,
}

impl Miniature {
    /// The first page of `layout`, if it has one.
    #[cfg(feature = "page-surface")]
    pub fn first_page(layout: &amipro_layout::DocumentLayout) -> Option<Self> {
        let page = layout.pages.first()?;
        let rect = |bounds: &amipro_layout::Bounds| {
            Rectangle::new(
                Point::new(bounds.x, bounds.y),
                Size::new(bounds.width, bounds.height),
            )
        };
        Some(Self {
            width: page.width,
            height: page.height,
            columns: page.columns.iter().map(rect).collect(),
            frames: page
                .frames
                .iter()
                .map(|frame| rect(&frame.bounds))
                .collect(),
            lines: page
                .lines
                .iter()
                .filter(|line| line.width > 0.5)
                .map(|line| {
                    Rectangle::new(
                        Point::new(line.x, line.top),
                        Size::new(line.width, line.height),
                    )
                })
                .collect(),
        })
    }
}

pub struct PagePreview<'a> {
    page: &'a Miniature,
    width: f32,
    height: f32,
}

/// The miniature scaled to fit `width` × `height` pixels.
pub fn page_preview(page: &Miniature, width: f32, height: f32) -> PagePreview<'_> {
    PagePreview {
        page,
        width,
        height,
    }
}

const PAPER: Color = Color::WHITE;
const EDGE: Color = Color::from_rgb(0.55, 0.55, 0.55);
const GUIDE: Color = Color::from_rgb(0.82, 0.86, 0.92);
const TEXT: Color = Color::from_rgb(0.62, 0.62, 0.62);

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for PagePreview<'_>
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.width), Length::Fixed(self.height))
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(
            Length::Fixed(self.width),
            Length::Fixed(self.height),
            Size::ZERO,
        ))
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let page = self.page;
        if page.width <= 0.0 || page.height <= 0.0 {
            return;
        }
        let scale = (bounds.width / page.width).min(bounds.height / page.height);
        let origin = Point::new(
            bounds.x + (bounds.width - page.width * scale) / 2.0,
            bounds.y + (bounds.height - page.height * scale) / 2.0,
        );
        let place = |rect: &Rectangle| {
            Rectangle::new(
                Point::new(origin.x + rect.x * scale, origin.y + rect.y * scale),
                Size::new(rect.width * scale, rect.height * scale),
            )
        };
        let quad = |renderer: &mut Renderer, rect: Rectangle, fill: Option<Color>, edge: Color| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: rect,
                    border: Border {
                        color: edge,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..renderer::Quad::default()
                },
                fill.unwrap_or(Color::TRANSPARENT),
            );
        };
        let paper = Rectangle::new(origin, Size::new(page.width * scale, page.height * scale));
        quad(renderer, paper, Some(PAPER), EDGE);
        for column in &page.columns {
            quad(renderer, place(column), None, GUIDE);
        }
        for frame in &page.frames {
            quad(renderer, place(frame), None, EDGE);
        }
        for line in &page.lines {
            let rect = place(line);
            // A bar a little under the line's height, so lines stay apart.
            let bar = Rectangle::new(
                Point::new(rect.x, rect.y + rect.height * 0.25),
                Size::new(rect.width, (rect.height * 0.5).max(1.0)),
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: bar,
                    ..renderer::Quad::default()
                },
                TEXT,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<PagePreview<'a>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(preview: PagePreview<'a>) -> Self {
        Element::new(preview)
    }
}
