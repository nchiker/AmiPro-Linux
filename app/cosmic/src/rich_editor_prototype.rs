//! Isolated rich-editor interaction prototype.
//!
//! The production editor remains active. This module proves the semantic side
//! of the replacement boundary: view state stores only a selection, and every
//! text mutation is an `amipro_editing::Command` that the session's history
//! executes. The companion surface will consume the same renderer paragraph
//! geometry produced by `rich_paragraph` before it is enabled in the app.

#![allow(
    dead_code,
    reason = "the feature-gated prototype retains helpers while its interaction gates are verified"
)]

use std::sync::Arc;

use amipro_editing::{Command, Cursor, EditError, Editor};
use cosmic::iced::advanced::text::{self, Affinity, Paragraph as _};
use cosmic::iced::advanced::widget::{self, Widget};
use cosmic::iced::advanced::{
    Clipboard, InputMethod, Layout, Shell, input_method, layout, renderer,
};
use cosmic::iced::keyboard::{self, Key, key::Named};
use cosmic::iced::{Color, Element, Event, Font, Length, Point, Rectangle, Size, Vector, mouse};

use crate::command::{self, CommandId};
use crate::rich_paragraph::project_paragraph;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Selection {
    pub anchor: usize,
    pub focus: usize,
}

impl Selection {
    pub fn caret(byte: usize) -> Self {
        Self {
            anchor: byte,
            focus: byte,
        }
    }

    pub fn ordered(self) -> (usize, usize) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    pub fn is_caret(self) -> bool {
        self.anchor == self.focus
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Click { byte: usize, extend: bool },
    MoveTo { byte: usize, extend: bool },
    Insert(String),
    Backspace,
    Delete,
    MoveLeft { extend: bool },
    MoveRight { extend: bool },
    MoveHome { extend: bool },
    MoveEnd { extend: bool },
    SelectAll,
    ParagraphBreak,
    Undo,
    Redo,
}

/// Explicit identity for renderer layout derived from one document session.
///
/// `generation` changes whenever semantic text or formatting changes. Selection,
/// focus, and outer scrolling deliberately do not affect this identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutEpoch {
    pub session: u64,
    pub generation: u64,
}

impl LayoutEpoch {
    pub const fn new(session: u64, generation: u64) -> Self {
        Self {
            session,
            generation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LayoutCacheKey {
    session: u64,
    display_scale: u32,
    width: u32,
    height: u32,
    default_size: u32,
    default_font: Font,
}

struct SurfaceState<P: text::Paragraph> {
    paragraphs: Vec<ShapedParagraph<P>>,
    layout_key: Option<LayoutCacheKey>,
    layout_generation: Option<u64>,
    layout_rebuilds: usize,
    paragraph_rebuilds: usize,
    focused: bool,
    window_focused: bool,
    dragging: bool,
    drag_hit: Option<usize>,
    preedit: Option<input_method::Preedit>,
}

impl<P: text::Paragraph> Default for SurfaceState<P> {
    fn default() -> Self {
        Self {
            paragraphs: Vec::new(),
            layout_key: None,
            layout_generation: None,
            layout_rebuilds: 0,
            paragraph_rebuilds: 0,
            focused: false,
            window_focused: true,
            dragging: false,
            drag_hit: None,
            preedit: None,
        }
    }
}

struct ShapedParagraph<P> {
    paragraph: P,
    bounds: Rectangle,
    underlines: Vec<(usize, f32)>,
    source: Arc<amipro_document::Paragraph>,
    style: amipro_document::ParagraphStyle,
    space_after: f32,
}

/// A renderer-native editing surface kept outside the production UI.
///
/// It owns no text. It draws a projection borrowed from `Document` and emits
/// semantic actions for the application/controller to apply.
pub struct Surface<'a, Message> {
    document: &'a amipro_document::Document,
    layout_epoch: LayoutEpoch,
    display_scale: f32,
    selection: Selection,
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
    on_command: Option<Box<dyn Fn(CommandId) -> Message + 'a>>,
    width: Length,
    height: Length,
}

pub fn surface<'a, Message>(
    document: &'a amipro_document::Document,
    layout_epoch: LayoutEpoch,
    display_scale: f32,
    selection: Selection,
    on_action: impl Fn(Action) -> Message + 'a,
) -> Surface<'a, Message> {
    Surface {
        document,
        layout_epoch,
        display_scale,
        selection,
        on_action: Box::new(on_action),
        on_command: None,
        width: Length::Fill,
        height: Length::Fill,
    }
}

pub fn surface_with_commands<'a, Message>(
    document: &'a amipro_document::Document,
    layout_epoch: LayoutEpoch,
    display_scale: f32,
    selection: Selection,
    on_action: impl Fn(Action) -> Message + 'a,
    on_command: impl Fn(CommandId) -> Message + 'a,
) -> Surface<'a, Message> {
    Surface {
        document,
        layout_epoch,
        display_scale,
        selection,
        on_action: Box::new(on_action),
        on_command: Some(Box::new(on_command)),
        width: Length::Fill,
        height: Length::Fill,
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Surface<'_, Message>
where
    Message: 'static,
    Renderer: text::Renderer<Font = Font>,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<SurfaceState<Renderer::Paragraph>>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(SurfaceState::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let resolved = limits.resolve(self.width, self.height, Size::ZERO);
        let paragraph_bounds = Size::new(resolved.width.max(1.0), resolved.height.max(1.0));
        let size = renderer.default_size();
        let font = renderer.default_font();
        let key = LayoutCacheKey {
            session: self.layout_epoch.session,
            display_scale: self.display_scale.to_bits(),
            width: paragraph_bounds.width.to_bits(),
            height: paragraph_bounds.height.to_bits(),
            default_size: size.0.to_bits(),
            default_font: font,
        };
        let state = tree
            .state
            .downcast_mut::<SurfaceState<Renderer::Paragraph>>();
        let font_system_is_current = state.paragraphs.first().is_none_or(|shaped| {
            !matches!(
                shaped.paragraph.compare(text::Text {
                    content: (),
                    bounds: paragraph_bounds,
                    size,
                    line_height: text::LineHeight::Relative(1.2),
                    font,
                    align_x: text::Alignment::Default,
                    align_y: cosmic::iced::alignment::Vertical::Top,
                    shaping: text::Shaping::Advanced,
                    wrapping: text::Wrapping::WordOrGlyph,
                    ellipsize: text::Ellipsize::None,
                }),
                text::Difference::Shape
            )
        });
        let geometry_changed = state.layout_key != Some(key) || !font_system_is_current;
        let paragraph_count_changed = state.paragraphs.len() != self.document.paragraphs().len();
        if geometry_changed || paragraph_count_changed {
            let mut y = 0.0;
            state.paragraphs = (0..self.document.paragraphs().len())
                .map(|paragraph_index| {
                    let shaped = shape_paragraph::<Renderer>(
                        self.document,
                        paragraph_index,
                        self.display_scale,
                        paragraph_bounds,
                        size,
                        font,
                        y,
                    );
                    y += shaped.bounds.height + shaped.space_after;
                    shaped
                })
                .collect();
            state.layout_key = Some(key);
            state.layout_generation = Some(self.layout_epoch.generation);
            state.layout_rebuilds += 1;
            state.paragraph_rebuilds += state.paragraphs.len();
        } else if state.layout_generation != Some(self.layout_epoch.generation) {
            let mut y = 0.0;
            for (paragraph_index, source) in self.document.paragraphs().iter().enumerate() {
                let style = &self.document.styles()[&source.style];
                let cached = &state.paragraphs[paragraph_index];
                if !(Arc::ptr_eq(&cached.source, source) || *cached.source == **source)
                    || cached.style != *style
                {
                    state.paragraphs[paragraph_index] = shape_paragraph::<Renderer>(
                        self.document,
                        paragraph_index,
                        self.display_scale,
                        paragraph_bounds,
                        size,
                        font,
                        y,
                    );
                    state.paragraph_rebuilds += 1;
                }
                let cached = &mut state.paragraphs[paragraph_index];
                cached.bounds.x = 0.0;
                cached.bounds.y = y;
                cached.bounds.width = resolved.width.max(1.0);
                y += cached.bounds.height + cached.space_after;
            }
            state.layout_generation = Some(self.layout_epoch.generation);
            state.layout_rebuilds += 1;
        }
        layout::Node::new(resolved)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree
            .state
            .downcast_mut::<SurfaceState<Renderer::Paragraph>>();
        let bounds = layout.bounds();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(bounds) {
                    state.focused = true;
                    state.dragging = true;
                    let byte = document_hit(&state.paragraphs, self.document, position);
                    state.drag_hit = Some(byte);
                    shell.publish((self.on_action)(Action::Click {
                        byte,
                        extend: false,
                    }));
                    shell.capture_event();
                    shell.request_redraw();
                } else {
                    state.focused = false;
                    state.dragging = false;
                    state.drag_hit = None;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                if let Some(position) = cursor.position() {
                    let position = Point::new(
                        (position.x - bounds.x).clamp(0.0, bounds.width),
                        (position.y - bounds.y).clamp(0.0, bounds.height),
                    );
                    let byte = document_hit(&state.paragraphs, self.document, position);
                    if state.drag_hit != Some(byte) {
                        state.drag_hit = Some(byte);
                        shell.publish((self.on_action)(Action::Click { byte, extend: true }));
                        shell.request_redraw();
                    }
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging {
                    shell.capture_event();
                    shell.request_redraw();
                }
                state.dragging = false;
                state.drag_hit = None;
            }
            Event::InputMethod(input_method::Event::Opened) if state.focused => {
                state.preedit = Some(input_method::Preedit::new());
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Preedit(content, selection))
                if state.focused =>
            {
                state.preedit = Some(input_method::Preedit {
                    content: content.clone(),
                    selection: selection.clone(),
                    text_size: None,
                });
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Commit(content)) if state.focused => {
                shell.publish((self.on_action)(Action::Insert(content.clone())));
                shell.capture_event();
            }
            Event::InputMethod(input_method::Event::Closed) => {
                state.preedit = None;
                shell.request_redraw();
            }
            Event::Window(cosmic::iced::window::Event::Focused) => {
                state.window_focused = true;
                shell.request_redraw();
            }
            Event::Window(cosmic::iced::window::Event::Unfocused) => {
                state.window_focused = false;
                state.preedit = None;
                shell.request_redraw();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                text,
                ..
            }) if state.focused => {
                let shortcut = command::command_for_shortcut(*modifiers, key, Some(physical_key))
                    .filter(|command| {
                        self.on_command.is_some()
                            && command::metadata(*command).implementation
                                == command::Implementation::Implemented
                            && *command != CommandId::Delete
                    });
                let vertical = match key {
                    Key::Named(Named::ArrowUp) => Some(VerticalDirection::Up),
                    Key::Named(Named::ArrowDown) => Some(VerticalDirection::Down),
                    _ => None,
                };
                if let Some(command) = shortcut {
                    shell.publish((self.on_command.as_ref().expect("filtered above"))(command));
                    shell.capture_event();
                } else if let Some(direction) = vertical {
                    let byte = vertical_hit(
                        &state.paragraphs,
                        self.document,
                        self.selection.focus,
                        direction,
                    );
                    shell.publish((self.on_action)(Action::MoveTo {
                        byte,
                        extend: modifiers.shift(),
                    }));
                    shell.capture_event();
                } else if let Some(action) = key_action(key, *modifiers) {
                    shell.publish((self.on_action)(action));
                    shell.capture_event();
                } else if modifiers.command() && self.on_command.is_none() {
                    match key.as_ref() {
                        Key::Character("a") => {
                            shell.publish((self.on_action)(Action::SelectAll));
                            shell.capture_event();
                        }
                        Key::Character("c") => {
                            if let Some(selected) = selected_text(self.document, self.selection) {
                                clipboard.write(
                                    cosmic::iced::advanced::clipboard::Kind::Standard,
                                    selected,
                                );
                            }
                            shell.capture_event();
                        }
                        Key::Character("x") => {
                            if let Some(selected) = selected_text(self.document, self.selection) {
                                clipboard.write(
                                    cosmic::iced::advanced::clipboard::Kind::Standard,
                                    selected,
                                );
                                shell.publish((self.on_action)(Action::Delete));
                            }
                            shell.capture_event();
                        }
                        Key::Character("v") => {
                            if let Some(value) =
                                clipboard.read(cosmic::iced::advanced::clipboard::Kind::Standard)
                            {
                                shell.publish((self.on_action)(Action::Insert(value)));
                            }
                            shell.capture_event();
                        }
                        Key::Character("z") if modifiers.shift() => {
                            shell.publish((self.on_action)(Action::Redo));
                            shell.capture_event();
                        }
                        Key::Character("z") => {
                            shell.publish((self.on_action)(Action::Undo));
                            shell.capture_event();
                        }
                        _ => {}
                    }
                } else if let Some(value) = text.as_ref().filter(|value| !value.is_empty()) {
                    shell.publish((self.on_action)(Action::Insert(value.to_string())));
                    shell.capture_event();
                }
            }
            Event::Window(cosmic::iced::window::Event::RedrawRequested(_))
                if state.focused && state.window_focused =>
            {
                let cursor = document_caret(
                    &state.paragraphs,
                    self.document,
                    self.selection.focus,
                    bounds.position(),
                )
                .unwrap_or(Rectangle::new(bounds.position(), Size::new(1.0, 1.0)));
                shell.request_input_method(&InputMethod::Enabled {
                    cursor,
                    purpose: input_method::Purpose::Normal,
                    preedit: state.preedit.as_ref().map(input_method::Preedit::as_ref),
                });
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if !bounds.intersects(viewport) {
            return;
        }
        let state = tree
            .state
            .downcast_ref::<SurfaceState<Renderer::Paragraph>>();
        let translation = bounds.position() - Point::ORIGIN;
        let (start, end) = self.selection.ordered();
        let selection_cursors = (start != end)
            .then(|| {
                Some((
                    cursor_at_offset(self.document, start).ok()?,
                    cursor_at_offset(self.document, end).ok()?,
                ))
            })
            .flatten();
        for (index, shaped) in state.paragraphs.iter().enumerate() {
            let paragraph_origin = bounds.position() + Vector::new(0.0, shaped.bounds.y);
            if let Some((selection_start, selection_end)) = selection_cursors
                && index >= selection_start.paragraph
                && index <= selection_end.paragraph
            {
                let start = if index == selection_start.paragraph {
                    selection_start.byte
                } else {
                    0
                };
                let end = if index == selection_end.paragraph {
                    selection_end.byte
                } else {
                    self.document.paragraphs()[index].text.len()
                };
                for selection in shaped.paragraph.highlight(
                    0,
                    (start, Affinity::Before),
                    (end, Affinity::Before),
                ) {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: selection + (paragraph_origin - Point::ORIGIN),
                            ..renderer::Quad::default()
                        },
                        Color::from_rgba(0.2, 0.45, 0.9, 0.28),
                    );
                }
            }
            renderer.fill_paragraph(
                &shaped.paragraph,
                paragraph_origin,
                defaults.text_color,
                *viewport,
            );
            draw_underlines(shaped, renderer, translation, defaults.text_color);
        }
        if state.focused
            && state.window_focused
            && self.selection.is_caret()
            && let Some(caret) = document_caret(
                &state.paragraphs,
                self.document,
                self.selection.focus,
                bounds.position(),
            )
        {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: caret,
                    ..renderer::Quad::default()
                },
                defaults.text_color,
            );
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }
}

fn shape_paragraph<Renderer>(
    document: &amipro_document::Document,
    paragraph_index: usize,
    display_scale: f32,
    paragraph_bounds: Size,
    size: cosmic::iced::Pixels,
    font: Font,
    y: f32,
) -> ShapedParagraph<Renderer::Paragraph>
where
    Renderer: text::Renderer<Font = Font>,
{
    let projection = project_paragraph(document, paragraph_index, display_scale)
        .expect("validated document has rich paragraph projections");
    let minimum_height = projection
        .spans
        .iter()
        .filter_map(|span| span.display.size)
        .map(|size| size.0 * 1.2)
        .fold(1.0_f32, f32::max);
    let spans: Vec<_> = projection
        .spans
        .into_iter()
        .map(|span| span.display)
        .collect();
    let underlines = spans
        .iter()
        .enumerate()
        .filter(|(_, span)| span.underline)
        .map(|(index, span)| (index, span.size.unwrap_or(size).0))
        .collect();
    let paragraph = Renderer::Paragraph::with_spans(text::Text {
        content: spans.as_slice(),
        bounds: paragraph_bounds,
        size,
        line_height: text::LineHeight::Relative(1.2),
        font,
        align_x: text::Alignment::Default,
        align_y: cosmic::iced::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::WordOrGlyph,
        ellipsize: text::Ellipsize::None,
    });
    let height = paragraph.min_bounds().height.max(minimum_height);
    ShapedParagraph {
        paragraph,
        bounds: Rectangle::new(
            Point::new(0.0, y),
            Size::new(paragraph_bounds.width, height),
        ),
        underlines,
        source: document.paragraphs()[paragraph_index].clone(),
        style: document.styles()[&document.paragraphs()[paragraph_index].style].clone(),
        space_after: projection.space_after,
    }
}

impl<'a, Message, Theme, Renderer> From<Surface<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'a,
    Renderer: text::Renderer<Font = Font> + 'a,
{
    fn from(surface: Surface<'a, Message>) -> Self {
        Element::new(surface)
    }
}

fn hit_byte<P: text::Paragraph>(paragraph: &P, position: Point, fallback: usize) -> usize {
    paragraph
        .hit_test(position)
        .map_or(fallback, text::Hit::cursor)
}

fn document_hit<P: text::Paragraph>(
    paragraphs: &[ShapedParagraph<P>],
    document: &amipro_document::Document,
    position: Point,
) -> usize {
    let Some((index, shaped)) = paragraphs.iter().enumerate().find(|(_, shaped)| {
        position.y >= shaped.bounds.y && position.y < shaped.bounds.y + shaped.bounds.height
    }) else {
        if let Some((index, _)) = paragraphs
            .iter()
            .enumerate()
            .find(|(_, shaped)| position.y < shaped.bounds.y)
        {
            return offset_for_cursor(
                document,
                Cursor {
                    paragraph: index,
                    byte: 0,
                },
            )
            .unwrap_or(0);
        }
        return document_len(document);
    };
    let source = &document.paragraphs()[index].text;
    let local = Point::new(
        position.x.clamp(0.0, shaped.bounds.width),
        (position.y - shaped.bounds.y).clamp(0.0, shaped.bounds.height),
    );
    let byte = hit_byte(&shaped.paragraph, local, source.len()).min(source.len());
    offset_for_cursor(
        document,
        Cursor {
            paragraph: index,
            byte,
        },
    )
    .unwrap_or_else(|_| document_len(document))
}

#[derive(Clone, Copy)]
enum VerticalDirection {
    Up,
    Down,
}

fn vertical_hit<P: text::Paragraph>(
    paragraphs: &[ShapedParagraph<P>],
    document: &amipro_document::Document,
    byte: usize,
    direction: VerticalDirection,
) -> usize {
    let Ok(cursor) = cursor_at_offset(document, byte) else {
        return byte;
    };
    let Some(shaped) = paragraphs.get(cursor.paragraph) else {
        return byte;
    };
    let source = &document.paragraphs()[cursor.paragraph].text;
    let caret = caret_rectangle(&shaped.paragraph, source, cursor.byte, Point::ORIGIN);
    let (paragraph, y) = match direction {
        VerticalDirection::Up if caret.y > 0.0 => (cursor.paragraph, caret.y - 1.0),
        VerticalDirection::Up if cursor.paragraph > 0 => {
            let previous = cursor.paragraph - 1;
            (previous, paragraphs[previous].bounds.height.max(1.0) - 1.0)
        }
        VerticalDirection::Down if caret.y + caret.height + 1.0 < shaped.bounds.height => {
            (cursor.paragraph, caret.y + caret.height + 1.0)
        }
        VerticalDirection::Down if cursor.paragraph + 1 < paragraphs.len() => {
            (cursor.paragraph + 1, 0.0)
        }
        _ => return byte,
    };
    let target = &paragraphs[paragraph];
    let source = &document.paragraphs()[paragraph].text;
    let local_byte = hit_byte(&target.paragraph, Point::new(caret.x, y), source.len());
    offset_for_cursor(
        document,
        Cursor {
            paragraph,
            byte: local_byte.min(source.len()),
        },
    )
    .unwrap_or(byte)
}

fn document_caret<P: text::Paragraph>(
    paragraphs: &[ShapedParagraph<P>],
    document: &amipro_document::Document,
    offset: usize,
    origin: Point,
) -> Option<Rectangle> {
    let cursor = cursor_at_offset(document, offset).ok()?;
    let shaped = paragraphs.get(cursor.paragraph)?;
    Some(caret_rectangle(
        &shaped.paragraph,
        &document.paragraphs()[cursor.paragraph].text,
        cursor.byte,
        origin + Vector::new(0.0, shaped.bounds.y),
    ))
}

fn caret_rectangle<P: text::Paragraph>(
    paragraph: &P,
    source: &str,
    byte: usize,
    origin: Point,
) -> Rectangle {
    let position = paragraph
        .cursor_position(0, byte, Affinity::Before)
        .unwrap_or(Point::ORIGIN);
    let line_box = adjacent_highlight(paragraph, source, byte);
    caret_rectangle_from_geometry(
        position,
        line_box,
        f32::from(paragraph.line_height().to_absolute(paragraph.size())),
        origin,
    )
}

fn caret_rectangle_from_geometry(
    position: Point,
    line_box: Option<Rectangle>,
    fallback_height: f32,
    origin: Point,
) -> Rectangle {
    let y = line_box.map_or(position.y, |bounds| bounds.y);
    let height = line_box.map_or(fallback_height, |bounds| bounds.height);
    Rectangle::new(origin + Vector::new(position.x, y), Size::new(1.0, height))
}

fn adjacent_highlight<P: text::Paragraph>(
    paragraph: &P,
    source: &str,
    byte: usize,
) -> Option<Rectangle> {
    let range = if let Some(next) = next_text_boundary(source, byte) {
        (byte, next)
    } else {
        (previous_text_boundary(source, byte)?, byte)
    };
    paragraph
        .highlight(0, (range.0, Affinity::Before), (range.1, Affinity::Before))
        .into_iter()
        .next()
}

fn draw_underlines<P, Renderer>(
    shaped: &ShapedParagraph<P>,
    renderer: &mut Renderer,
    translation: Vector,
    color: Color,
) where
    P: text::Paragraph<Font = Font>,
    Renderer: text::Renderer<Font = Font>,
{
    for (index, size) in &shaped.underlines {
        for bounds in shaped.paragraph.span_bounds(*index) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle::new(
                        bounds.position()
                            + translation
                            + Vector::new(0.0, shaped.bounds.y + size * 1.05),
                        Size::new(bounds.width, 1.0),
                    ),
                    ..renderer::Quad::default()
                },
                color,
            );
        }
    }
}

fn selected_text(document: &amipro_document::Document, selection: Selection) -> Option<String> {
    let (start, end) = selection.ordered();
    (start != end).then(|| document.plain_text()[start..end].to_owned())
}

fn key_action(key: &Key, modifiers: keyboard::Modifiers) -> Option<Action> {
    let extend = modifiers.shift();
    match key {
        Key::Named(Named::ArrowLeft) => Some(Action::MoveLeft { extend }),
        Key::Named(Named::ArrowRight) => Some(Action::MoveRight { extend }),
        Key::Named(Named::Home) => Some(Action::MoveHome { extend }),
        Key::Named(Named::End) => Some(Action::MoveEnd { extend }),
        Key::Named(Named::Backspace) => Some(Action::Backspace),
        Key::Named(Named::Delete) => Some(Action::Delete),
        Key::Named(Named::Enter) => Some(Action::ParagraphBreak),
        _ => None,
    }
}

/// Maps one prototype surface action to a document command, or applies it
/// to `selection` when it only moves the selection.
///
/// Selection offsets address the derived `Document::plain_text()` projection;
/// every edit is mapped back to semantic paragraph positions. For an edit,
/// `selection` is unchanged and the caller places the caret where the
/// executed command reports it. Undo and Redo belong to the caller's history
/// and map to nothing here.
pub fn action_command(
    document: &amipro_document::Document,
    selection: &mut Selection,
    action: Action,
) -> Result<Option<Command>, EditError> {
    validate_selection(document, *selection)?;

    match action {
        Action::Click { byte, extend } | Action::MoveTo { byte, extend } => {
            cursor_at_offset(document, byte)?;
            move_focus(selection, byte, extend);
        }
        Action::Insert(replacement) => {
            return replace_selection(document, *selection, replacement).map(Some);
        }
        Action::Backspace | Action::Delete if selection.is_caret() => {
            // A single Backspace or Delete is its own command, so that the
            // object rules for single deletions apply.
            let forward = matches!(action, Action::Delete);
            let at = cursor_at_offset(document, selection.focus)?.in_flow(document.main_flow_id());
            return Ok(Some(Command::DeleteCharacter { at, forward }));
        }
        Action::Backspace | Action::Delete => {
            return replace_selection(document, *selection, String::new()).map(Some);
        }
        Action::MoveLeft { extend } => {
            let target = if !extend && !selection.is_caret() {
                selection.ordered().0
            } else {
                previous_document_boundary(document, selection.focus).unwrap_or(0)
            };
            move_focus(selection, target, extend);
        }
        Action::MoveRight { extend } => {
            let target = if !extend && !selection.is_caret() {
                selection.ordered().1
            } else {
                next_document_boundary(document, selection.focus)
                    .unwrap_or_else(|| document_len(document))
            };
            move_focus(selection, target, extend);
        }
        Action::MoveHome { extend } => {
            let cursor = cursor_at_offset(document, selection.focus)?;
            let target = offset_for_cursor(
                document,
                Cursor {
                    paragraph: cursor.paragraph,
                    byte: 0,
                },
            )?;
            move_focus(selection, target, extend);
        }
        Action::MoveEnd { extend } => {
            let cursor = cursor_at_offset(document, selection.focus)?;
            let target = offset_for_cursor(
                document,
                Cursor {
                    paragraph: cursor.paragraph,
                    byte: document.paragraphs()[cursor.paragraph].text.len(),
                },
            )?;
            move_focus(selection, target, extend);
        }
        Action::SelectAll => {
            selection.anchor = 0;
            selection.focus = document_len(document);
        }
        Action::ParagraphBreak => {
            return replace_selection(document, *selection, "\n".into()).map(Some);
        }
        Action::Undo | Action::Redo => {}
    }
    Ok(None)
}

/// Applies one prototype surface action through a main-flow [`Editor`], as
/// the app does through its session history. Tests use it.
pub fn apply_action(
    editor: &mut Editor,
    _legacy_paragraph: usize,
    selection: &mut Selection,
    action: Action,
) -> Result<(), EditError> {
    match action {
        Action::Undo => {
            editor.undo()?;
            *selection = Selection::caret(offset_for_cursor(editor.document(), editor.cursor())?);
        }
        Action::Redo => {
            editor.redo()?;
            *selection = Selection::caret(offset_for_cursor(editor.document(), editor.cursor())?);
        }
        action => match action_command(editor.document(), selection, action)? {
            Some(command) => {
                editor.execute(&command)?;
                *selection =
                    Selection::caret(offset_for_cursor(editor.document(), editor.cursor())?);
            }
            None => editor.set_cursor(cursor_at_offset(editor.document(), selection.focus)?)?,
        },
    }
    Ok(())
}

fn replace_selection(
    document: &amipro_document::Document,
    selection: Selection,
    replacement: String,
) -> Result<Command, EditError> {
    let (start, end) = selection.ordered();
    let flow = document.main_flow_id();
    Ok(Command::ReplaceText {
        start: cursor_at_offset(document, start)?.in_flow(flow),
        end: cursor_at_offset(document, end)?.in_flow(flow),
        text: replacement,
    })
}

fn move_focus(selection: &mut Selection, target: usize, extend: bool) {
    if extend {
        selection.focus = target;
    } else {
        *selection = Selection::caret(target);
    }
}

fn validate_selection(
    document: &amipro_document::Document,
    selection: Selection,
) -> Result<(), &'static str> {
    cursor_at_offset(document, selection.anchor)?;
    cursor_at_offset(document, selection.focus)?;
    Ok(())
}

pub fn cursor_at_offset(
    document: &amipro_document::Document,
    offset: usize,
) -> Result<Cursor, &'static str> {
    let mut paragraph_start = 0;
    for (paragraph, source) in document.paragraphs().iter().enumerate() {
        let paragraph_end = paragraph_start + source.text.len();
        if offset <= paragraph_end {
            let byte = offset - paragraph_start;
            return source
                .text
                .is_char_boundary(byte)
                .then_some(Cursor { paragraph, byte })
                .ok_or("selection is not a UTF-8 boundary");
        }
        paragraph_start = paragraph_end + 1;
    }
    Err("selection is outside the document")
}

pub fn offset_for_cursor(
    document: &amipro_document::Document,
    cursor: Cursor,
) -> Result<usize, &'static str> {
    let paragraph = document
        .paragraphs()
        .get(cursor.paragraph)
        .ok_or("paragraph out of bounds")?;
    if cursor.byte > paragraph.text.len() || !paragraph.text.is_char_boundary(cursor.byte) {
        return Err("cursor is not a UTF-8 boundary");
    }
    Ok(document
        .paragraphs()
        .iter()
        .take(cursor.paragraph)
        .map(|paragraph| paragraph.text.len() + 1)
        .sum::<usize>()
        + cursor.byte)
}

fn document_len(document: &amipro_document::Document) -> usize {
    document
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.text.len())
        .sum::<usize>()
        + document.paragraphs().len().saturating_sub(1)
}

fn previous_document_boundary(
    document: &amipro_document::Document,
    offset: usize,
) -> Option<usize> {
    let cursor = cursor_at_offset(document, offset).ok()?;
    if cursor.byte == 0 {
        (cursor.paragraph > 0).then(|| offset - 1)
    } else {
        document.paragraphs()[cursor.paragraph].text[..cursor.byte]
            .char_indices()
            .next_back()
            .map(|(byte, _)| offset - (cursor.byte - byte))
    }
}

fn next_document_boundary(document: &amipro_document::Document, offset: usize) -> Option<usize> {
    let cursor = cursor_at_offset(document, offset).ok()?;
    let text = &document.paragraphs()[cursor.paragraph].text;
    if cursor.byte == text.len() {
        (cursor.paragraph + 1 < document.paragraphs().len()).then_some(offset + 1)
    } else {
        Some(offset + text[cursor.byte..].chars().next()?.len_utf8())
    }
}

fn previous_text_boundary(text: &str, byte: usize) -> Option<usize> {
    (byte > 0)
        .then(|| {
            text[..byte]
                .char_indices()
                .next_back()
                .map(|(byte, _)| byte)
        })
        .flatten()
}

fn next_text_boundary(text: &str, byte: usize) -> Option<usize> {
    (byte < text.len()).then(|| byte + text[byte..].chars().next().unwrap().len_utf8())
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    use amipro_document::{CharacterFormat, CharacterRun, Document};
    use cosmic::iced::advanced::{graphics, image};
    use cosmic::iced::{Background, Pixels, Transformation};

    use super::*;

    /// A headless renderer that uses the same COSMIC Text-backed paragraph
    /// implementation as Iced's graphical renderers. Drawing is intentionally
    /// a no-op; shaping, wrapping, hit testing, and cursor geometry are real.
    #[derive(Default)]
    struct ShapedTestRenderer {
        quad_count: usize,
        paragraph_count: usize,
    }

    impl renderer::Renderer for ShapedTestRenderer {
        fn start_layer(&mut self, _bounds: Rectangle) {}

        fn end_layer(&mut self) {}

        fn start_transformation(&mut self, _transformation: Transformation) {}

        fn end_transformation(&mut self) {}

        fn fill_quad(&mut self, _quad: renderer::Quad, _background: impl Into<Background>) {
            self.quad_count += 1;
        }

        fn reset(&mut self, _new_bounds: Rectangle) {}

        fn allocate_image(
            &mut self,
            _handle: &image::Handle,
            callback: impl FnOnce(Result<image::Allocation, image::Error>) + Send + 'static,
        ) {
            callback(Err(image::Error::Unsupported));
        }
    }

    impl text::Renderer for ShapedTestRenderer {
        type Font = Font;
        type Paragraph = graphics::text::Paragraph;
        type Editor = graphics::text::Editor;
        type Raw = ();

        const ICON_FONT: Font = Font::DEFAULT;
        const CHECKMARK_ICON: char = '0';
        const ARROW_DOWN_ICON: char = '0';
        const SCROLL_UP_ICON: char = '0';
        const SCROLL_DOWN_ICON: char = '0';
        const SCROLL_LEFT_ICON: char = '0';
        const SCROLL_RIGHT_ICON: char = '0';
        const ICED_LOGO: char = '0';

        fn default_font(&self) -> Self::Font {
            Font::DEFAULT
        }

        fn default_size(&self) -> Pixels {
            Pixels(14.0)
        }

        fn fill_paragraph(
            &mut self,
            _text: &Self::Paragraph,
            _position: Point,
            _color: Color,
            _clip_bounds: Rectangle,
        ) {
            self.paragraph_count += 1;
        }

        fn fill_editor(
            &mut self,
            _editor: &Self::Editor,
            _position: Point,
            _color: Color,
            _clip_bounds: Rectangle,
        ) {
        }

        fn fill_raw(&mut self, _raw: Self::Raw) {}

        fn fill_text(
            &mut self,
            _text: text::Text<String, Self::Font>,
            _position: Point,
            _color: Color,
            _clip_bounds: Rectangle,
        ) {
        }
    }

    fn expected_composite_point(
        document: &Document,
        paragraph_index: usize,
        byte: usize,
        width: f32,
        origin: Point,
    ) -> Point {
        let mut y = 0.0;
        for index in 0..=paragraph_index {
            let projection = project_paragraph(document, index, 1.0).unwrap();
            let minimum_height = projection
                .spans
                .iter()
                .filter_map(|span| span.display.size)
                .map(|size| size.0 * 1.2)
                .fold(1.0_f32, f32::max);
            let spans: Vec<_> = projection
                .spans
                .iter()
                .map(|span| span.display.clone())
                .collect();
            let paragraph = graphics::text::Paragraph::with_spans(text::Text {
                content: spans.as_slice(),
                bounds: Size::new(width, 360.0),
                size: Pixels(14.0),
                line_height: text::LineHeight::Relative(1.2),
                font: Font::DEFAULT,
                align_x: text::Alignment::Default,
                align_y: cosmic::iced::alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::WordOrGlyph,
                ellipsize: text::Ellipsize::None,
            });
            if index == paragraph_index {
                return origin
                    + Vector::new(
                        paragraph
                            .cursor_position(0, byte, Affinity::Before)
                            .unwrap()
                            .x
                            + 0.25,
                        y + paragraph
                            .cursor_position(0, byte, Affinity::Before)
                            .unwrap()
                            .y
                            + 2.0,
                    );
            }
            y += paragraph.min_bounds().height.max(minimum_height) + projection.space_after;
        }
        unreachable!()
    }

    #[test]
    fn typing_into_empty_document_inserts_at_initialized_caret() {
        let mut editor = Editor::new(Document::default());
        let mut selection = Selection::caret(0);

        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::Insert("Hello 🙂".into()),
        )
        .unwrap();

        assert_eq!(editor.document().paragraphs()[0].text, "Hello 🙂");
        assert_eq!(selection, Selection::caret("Hello 🙂".len()));
    }

    #[test]
    fn caret_movement_controls_insertion_in_existing_paragraph() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "ac".into();
        let mut editor = Editor::new(document);
        let mut selection = Selection::caret(2);

        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::MoveLeft { extend: false },
        )
        .unwrap();
        apply_action(&mut editor, 0, &mut selection, Action::Insert("b".into())).unwrap();

        assert_eq!(editor.document().paragraphs()[0].text, "abc");
        assert_eq!(selection, Selection::caret(2));
    }

    #[test]
    fn caret_uses_the_shaped_line_box_instead_of_the_default_font_height() {
        let caret = caret_rectangle_from_geometry(
            Point::new(8.0, 0.0),
            Some(Rectangle::new(Point::new(2.0, 3.0), Size::new(18.0, 30.0))),
            12.0,
            Point::new(100.0, 200.0),
        );

        assert_eq!(
            caret,
            Rectangle::new(Point::new(108.0, 203.0), Size::new(1.0, 30.0))
        );
    }

    #[test]
    fn backspace_delete_undo_and_redo_are_distinct_semantic_edits() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "abc".into();
        let mut editor = Editor::new(document);
        let mut selection = Selection::caret(1);

        apply_action(&mut editor, 0, &mut selection, Action::Delete).unwrap();
        assert_eq!(editor.document().paragraphs()[0].text, "ac");
        apply_action(&mut editor, 0, &mut selection, Action::Backspace).unwrap();
        assert_eq!(editor.document().paragraphs()[0].text, "c");
        apply_action(&mut editor, 0, &mut selection, Action::Undo).unwrap();
        assert_eq!(editor.document().paragraphs()[0].text, "ac");
        apply_action(&mut editor, 0, &mut selection, Action::Undo).unwrap();
        assert_eq!(editor.document().paragraphs()[0].text, "abc");
        apply_action(&mut editor, 0, &mut selection, Action::Redo).unwrap();
        assert_eq!(editor.document().paragraphs()[0].text, "ac");
    }

    #[test]
    fn paragraph_break_splits_and_backspace_rejoins_the_document() {
        let mut editor = Editor::new(Document::default());
        let mut selection = Selection::caret(0);

        apply_action(&mut editor, 0, &mut selection, Action::Insert("one".into())).unwrap();
        apply_action(&mut editor, 0, &mut selection, Action::ParagraphBreak).unwrap();
        apply_action(&mut editor, 0, &mut selection, Action::Insert("two".into())).unwrap();
        assert_eq!(editor.document().plain_text(), "one\ntwo");
        assert_eq!(selection, Selection::caret(7));

        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::MoveHome { extend: false },
        )
        .unwrap();
        apply_action(&mut editor, 0, &mut selection, Action::Backspace).unwrap();
        assert_eq!(editor.document().plain_text(), "onetwo");
        assert_eq!(selection, Selection::caret(3));
        apply_action(&mut editor, 0, &mut selection, Action::Undo).unwrap();
        assert_eq!(editor.document().plain_text(), "one\ntwo");
    }

    #[test]
    fn document_offsets_map_utf8_positions_across_paragraph_boundaries() {
        let document = Document::from_plain_text("A🙂\néZ");

        assert_eq!(document_len(&document), 9);
        assert_eq!(
            cursor_at_offset(&document, 6),
            Ok(Cursor {
                paragraph: 1,
                byte: 0
            })
        );
        assert_eq!(
            offset_for_cursor(
                &document,
                Cursor {
                    paragraph: 1,
                    byte: 2
                }
            ),
            Ok(8)
        );
        assert_eq!(
            cursor_at_offset(&document, 2),
            Err("selection is not a UTF-8 boundary")
        );
    }

    #[test]
    fn cross_paragraph_selection_replacement_and_clipboard_text_are_atomic() {
        let document = Document::from_plain_text("first\nsecond\nthird");
        let original = document.clone();
        let mut editor = Editor::new(document);
        let mut selection = Selection {
            anchor: 2,
            focus: 15,
        };

        assert_eq!(
            selected_text(editor.document(), selection).as_deref(),
            Some("rst\nsecond\nth")
        );
        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::Insert("X\nY".into()),
        )
        .unwrap();

        assert_eq!(editor.document().plain_text(), "fiX\nYird");
        assert_eq!(selection, Selection::caret(5));
        apply_action(&mut editor, 0, &mut selection, Action::Undo).unwrap();
        assert_eq!(editor.document(), &original);
        apply_action(&mut editor, 0, &mut selection, Action::Redo).unwrap();
        assert_eq!(editor.document().plain_text(), "fiX\nYird");
    }

    #[test]
    fn imported_single_paragraph_document_remains_editable() {
        let sample = b"[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[edoc]\r\nImported text\r\n\r\n>\r\n";
        let imported = amipro_formats::import_sam(sample, None, "Imported").unwrap();
        assert_eq!(imported.document.paragraphs().len(), 1);
        let end = imported.document.paragraphs()[0].text.len();
        let mut editor = Editor::new(imported.document);
        let mut selection = Selection::caret(end);

        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::Insert(" edited".into()),
        )
        .unwrap();

        assert_eq!(
            editor.document().paragraphs()[0].text,
            "Imported text edited"
        );
    }

    #[test]
    fn full_page_click_focuses_surface_and_routes_keyboard_and_ime_input() {
        let document = Document::default();
        let mut surface = surface(
            &document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        let mut tree = widget::Tree {
            tag: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::tag(&surface),
            id: None,
            state: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::state(&surface),
            children: Vec::new(),
        };
        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 300.0));
        let node = <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut surface,
            &mut tree,
            &renderer,
            &limits,
        );
        assert_eq!(node.size(), Size::new(400.0, 300.0));
        let layout = Layout::new(&node);
        let cursor = mouse::Cursor::Available(Point::new(200.0, 250.0));
        let viewport = Rectangle::new(Point::ORIGIN, node.size());
        let mut clipboard = cosmic::iced::advanced::clipboard::Null;
        let mut messages = Vec::new();

        {
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                layout,
                cursor,
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }
        assert_eq!(
            messages,
            [Action::Click {
                byte: 0,
                extend: false
            }]
        );
        messages.clear();

        let key = Key::Character("x".into());
        let key_event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: Some("x".into()),
            repeat: false,
        });
        {
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &key_event,
                layout,
                cursor,
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }
        assert_eq!(messages, [Action::Insert("x".into())]);
        messages.clear();

        let down_event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(Named::ArrowDown),
            modified_key: Key::Named(Named::ArrowDown),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::SHIFT,
            text: None,
            repeat: false,
        });
        {
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &down_event,
                layout,
                cursor,
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }
        assert_eq!(
            messages,
            [Action::MoveTo {
                byte: 0,
                extend: true
            }]
        );
        messages.clear();

        {
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &Event::InputMethod(input_method::Event::Commit("é".into())),
                layout,
                cursor,
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }
        assert_eq!(messages, [Action::Insert("é".into())]);
        messages.clear();

        let mut shell = Shell::new(&mut messages);
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
            &mut surface,
            &mut tree,
            &Event::Window(cosmic::iced::window::Event::RedrawRequested(
                cosmic::iced::time::Instant::now(),
            )),
            layout,
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        assert!(shell.input_method().is_enabled());
    }

    /// Shapes a document with inline objects, then routes real key and IME
    /// events from the surface into semantic edits beside the objects.
    #[test]
    fn surface_events_edit_beside_inline_objects_and_keep_them_intact() {
        let original = crate::object_fixture_document();
        let line_start = "Intro é\n".len();
        let mut editor = Editor::new(original.clone());
        let mut selection = Selection::caret(line_start + 8);
        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 300.0));
        let key = |key: Key, text: Option<&str>| {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: key.clone(),
                modified_key: key,
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::default(),
                text: text.map(Into::into),
                repeat: false,
            })
        };
        let events = [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            key(Key::Character("x".into()), Some("x")),
            key(Key::Named(Named::Backspace), None),
            key(Key::Named(Named::Backspace), None),
            Event::InputMethod(input_method::Event::Commit("\u{FFFC}é".into())),
        ];
        let mut generation = 0;
        let mut tree = None;
        let mut actions_seen = Vec::new();
        for event in events {
            let document = editor.document().clone();
            let mut surface = surface(
                &document,
                LayoutEpoch::new(1, generation),
                1.0,
                selection,
                |action| action,
            );
            let tree = tree.get_or_insert_with(|| widget::Tree {
                tag: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::tag(&surface),
                id: None,
                state: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::state(
                    &surface,
                ),
                children: Vec::new(),
            });
            let node = <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
                &mut surface,
                tree,
                &renderer,
                &limits,
            );
            let layout = Layout::new(&node);
            let cursor = mouse::Cursor::Available(Point::new(390.0, 290.0));
            let viewport = Rectangle::new(Point::ORIGIN, node.size());
            let mut clipboard = cosmic::iced::advanced::clipboard::Null;
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                tree,
                &event,
                layout,
                cursor,
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
            drop(surface);
            for action in messages {
                if matches!(action, Action::Click { .. }) {
                    // The click focuses the surface; keep the chosen caret.
                    continue;
                }
                actions_seen.push(action.clone());
                apply_action(&mut editor, 0, &mut selection, action).unwrap();
                editor.document().validate().unwrap();
                generation += 1;
            }
        }

        assert_eq!(
            actions_seen,
            [
                Action::Insert("x".into()),
                Action::Backspace,
                Action::Backspace,
                Action::Insert("\u{FFFC}é".into()),
            ]
        );
        let paragraph = &editor.document().paragraphs()[1];
        assert_eq!(paragraph.text, "a\u{FFFC}b\u{FFFD}éc 🙂");
        assert_eq!(paragraph.objects.len(), 1, "the break survives");
        assert_eq!(
            editor.document().flows().count(),
            original.flows().count() - 1,
            "Backspace onto the floating header deleted it and its flow"
        );
        while editor.undo().unwrap() {}
        assert_eq!(editor.document(), &original);
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum CommandTestMessage {
        Action(Action),
        Command(CommandId),
    }

    #[test]
    fn focused_surface_routes_non_latin_shortcut_through_command_registry() {
        let document = Document::default();
        let mut surface = surface_with_commands(
            &document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection::caret(0),
            CommandTestMessage::Action,
            CommandTestMessage::Command,
        );
        let mut tree = widget::Tree {
            tag: <Surface<'_, CommandTestMessage> as Widget<
                CommandTestMessage,
                (),
                ShapedTestRenderer,
            >>::tag(&surface),
            id: None,
            state: <Surface<'_, CommandTestMessage> as Widget<
                CommandTestMessage,
                (),
                ShapedTestRenderer,
            >>::state(&surface),
            children: Vec::new(),
        };
        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 300.0));
        let node = <Surface<'_, CommandTestMessage> as Widget<
            CommandTestMessage,
            (),
            ShapedTestRenderer,
        >>::layout(&mut surface, &mut tree, &renderer, &limits);
        let layout = Layout::new(&node);
        let cursor = mouse::Cursor::Available(Point::new(200.0, 250.0));
        let viewport = Rectangle::new(Point::ORIGIN, node.size());
        let mut clipboard = cosmic::iced::advanced::clipboard::Null;
        let mut messages = Vec::new();

        let mut shell = Shell::new(&mut messages);
        <Surface<'_, CommandTestMessage> as Widget<
            CommandTestMessage,
            (),
            ShapedTestRenderer,
        >>::update(
            &mut surface,
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            layout,
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        drop(shell);
        messages.clear();

        let logical = Key::Character("с".into());
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: logical.clone(),
            modified_key: logical,
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyC),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::CTRL,
            text: None,
            repeat: false,
        });
        let mut shell = Shell::new(&mut messages);
        <Surface<'_, CommandTestMessage> as Widget<
            CommandTestMessage,
            (),
            ShapedTestRenderer,
        >>::update(
            &mut surface,
            &mut tree,
            &event,
            layout,
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        let captured = shell.is_event_captured();
        drop(shell);
        assert!(captured);
        assert_eq!(messages, [CommandTestMessage::Command(CommandId::Copy)]);

        messages.clear();
        let page_down = Key::Named(Named::PageDown);
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: page_down.clone(),
            modified_key: page_down,
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::PageDown),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::CTRL,
            text: None,
            repeat: false,
        });
        let mut shell = Shell::new(&mut messages);
        <Surface<'_, CommandTestMessage> as Widget<
            CommandTestMessage,
            (),
            ShapedTestRenderer,
        >>::update(
            &mut surface,
            &mut tree,
            &event,
            layout,
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        let captured = shell.is_event_captured();
        drop(shell);
        assert!(captured);
        assert_eq!(
            messages,
            [CommandTestMessage::Command(CommandId::NextDocument)]
        );
    }

    #[test]
    fn shaped_widget_pointer_drag_maps_across_wrapped_mixed_paragraphs_when_scrolled() {
        let mut document = Document::from_plain_text(
            "First paragraph wraps across a visual line with café text.\n\
             Second 🙂 paragraph has larger mixed text and wraps too.\n\
             Third paragraph is the drag destination.",
        );
        document.paragraph_mut(1).runs = vec![CharacterRun {
            start: "Second ".len(),
            end: "Second 🙂 paragraph".len(),
            format: CharacterFormat {
                font_size_points: Some(22.0),
                bold: Some(true),
                underline: Some(true),
                ..CharacterFormat::default()
            },
        }];

        let start = offset_for_cursor(
            &document,
            Cursor {
                paragraph: 0,
                byte: "First paragraph wraps across a ".len(),
            },
        )
        .unwrap();
        let end = offset_for_cursor(
            &document,
            Cursor {
                paragraph: 2,
                byte: "Third paragraph".len(),
            },
        )
        .unwrap();
        let mut surface = surface(
            &document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection::caret(start),
            |action| action,
        );
        let mut tree = widget::Tree {
            tag: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::tag(&surface),
            id: None,
            state: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::state(&surface),
            children: Vec::new(),
        };
        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(190.0, 360.0));
        let mut node = <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut surface,
            &mut tree,
            &renderer,
            &limits,
        );
        // A negative widget origin models the coordinate transform applied by
        // a parent scrollable after the document has been scrolled downward.
        node.move_to_mut(Point::new(24.0, -8.0));
        let layout = Layout::new(&node);
        let bounds = layout.bounds();
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(260.0, 360.0));
        let start_point = expected_composite_point(
            &document,
            0,
            "First paragraph wraps across a ".len(),
            bounds.width,
            bounds.position(),
        );
        let end_point = expected_composite_point(
            &document,
            2,
            "Third paragraph".len(),
            bounds.width,
            bounds.position(),
        );
        assert!(viewport.contains(start_point));
        assert!(viewport.contains(end_point));

        let mut clipboard = cosmic::iced::advanced::clipboard::Null;
        let mut messages = Vec::new();
        {
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                layout,
                mouse::Cursor::Available(start_point),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }
        {
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: end_point,
                }),
                layout,
                mouse::Cursor::Available(end_point),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }
        let message_count_after_distinct_hit = messages.len();
        {
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: end_point,
                }),
                layout,
                mouse::Cursor::Available(end_point),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }
        assert_eq!(messages.len(), message_count_after_distinct_hit);
        {
            let mut shell = Shell::new(&mut messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                layout,
                mouse::Cursor::Available(end_point),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }

        assert_eq!(
            messages,
            [
                Action::Click {
                    byte: start,
                    extend: false,
                },
                Action::Click {
                    byte: end,
                    extend: true,
                },
            ]
        );

        let mut editor = Editor::new(document.clone());
        let mut selection = Selection::caret(0);
        for action in messages {
            apply_action(&mut editor, 0, &mut selection, action).unwrap();
        }
        assert_eq!(
            selection,
            Selection {
                anchor: start,
                focus: end
            }
        );
        assert!(
            selected_text(editor.document(), selection)
                .unwrap()
                .contains("Second 🙂 paragraph")
        );

        let before_idle_move = selection;
        let mut ignored = Vec::new();
        let mut shell = Shell::new(&mut ignored);
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
            &mut surface,
            &mut tree,
            &Event::Mouse(mouse::Event::CursorMoved {
                position: start_point,
            }),
            layout,
            mouse::Cursor::Available(start_point),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        assert!(ignored.is_empty());
        assert_eq!(selection, before_idle_move);

        let mut reverse_messages = Vec::new();
        for (event, point) in [
            (
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                end_point,
            ),
            (
                Event::Mouse(mouse::Event::CursorMoved {
                    position: start_point,
                }),
                start_point,
            ),
            (
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                start_point,
            ),
        ] {
            let mut shell = Shell::new(&mut reverse_messages);
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::update(
                &mut surface,
                &mut tree,
                &event,
                layout,
                mouse::Cursor::Available(point),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            assert!(shell.is_event_captured());
        }
        assert_eq!(
            reverse_messages,
            [
                Action::Click {
                    byte: end,
                    extend: false,
                },
                Action::Click {
                    byte: start,
                    extend: true,
                },
            ]
        );
    }

    #[test]
    #[ignore = "manual release-mode performance baseline"]
    fn rich_editor_pointer_performance_baseline() {
        const PARAGRAPHS: usize = 250;
        const PROJECTION_PASSES: usize = 100;
        const COLD_LAYOUT_PASSES: usize = 20;
        const WARM_LAYOUT_PASSES: usize = 1_000;
        const CONTROLLER_PASSES: usize = 1_000;
        const HIT_TESTS: usize = 100_000;
        const DRAW_PASSES: usize = 100;

        let document = if let Some(path) = std::env::var_os("AMIPRO_PERF_SAM") {
            amipro_formats::import_sam_path(path).unwrap().document
        } else {
            let paragraph_text = "Mixed café 🙂 text wraps across several words for representative shaping and hits.";
            let mut document = Document::from_plain_text(
                &(0..PARAGRAPHS)
                    .map(|index| format!("{index:03} {paragraph_text}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            for index in 0..document.paragraphs().len() {
                document.paragraph_mut(index).runs = vec![CharacterRun {
                    start: 4,
                    end: 4 + "Mixed café 🙂".len(),
                    format: CharacterFormat {
                        font_size_points: Some(16.0),
                        bold: Some(true),
                        underline: Some(true),
                        ..CharacterFormat::default()
                    },
                }];
            }
            document
        };

        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(560.0, 900.0));
        let valid_offsets: Vec<_> = (0..=document_len(&document))
            .filter(|offset| cursor_at_offset(&document, *offset).is_ok())
            .collect();
        let new_tree = || {
            let surface = surface(
                &document,
                LayoutEpoch::new(1, 0),
                1.0,
                Selection::caret(0),
                |action| action,
            );
            widget::Tree {
                tag: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::tag(&surface),
                id: None,
                state: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::state(
                    &surface,
                ),
                children: Vec::new(),
            }
        };

        let projection_started = Instant::now();
        for _ in 0..PROJECTION_PASSES {
            for paragraph in 0..document.paragraphs().len() {
                black_box(project_paragraph(&document, paragraph, 1.0).unwrap());
            }
        }
        let projection_elapsed = projection_started.elapsed();

        let cold_started = Instant::now();
        for _ in 0..COLD_LAYOUT_PASSES {
            let mut tree = new_tree();
            let mut surface = surface(
                &document,
                LayoutEpoch::new(1, 0),
                1.0,
                Selection::caret(0),
                |action| action,
            );
            black_box(<Surface<'_, Action> as Widget<
                Action,
                (),
                ShapedTestRenderer,
            >>::layout(
                &mut surface, &mut tree, &renderer, &limits
            ));
        }
        let cold_elapsed = cold_started.elapsed();

        let mut tree = new_tree();
        let mut initial_surface = surface(
            &document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        let initial_node = <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut initial_surface,
            &mut tree,
            &renderer,
            &limits,
        );
        black_box(&initial_node);

        let mut construction_elapsed = Duration::ZERO;
        let mut reuse_elapsed = Duration::ZERO;
        let mut last_node = initial_node;
        let mut last_surface = initial_surface;
        for pass in 0..WARM_LAYOUT_PASSES {
            let selection = Selection::caret(valid_offsets[pass % valid_offsets.len()]);
            let construction_started = Instant::now();
            let mut surface = surface(
                &document,
                LayoutEpoch::new(1, 0),
                1.0,
                selection,
                |action| action,
            );
            construction_elapsed += construction_started.elapsed();
            let reuse_started = Instant::now();
            let node = <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
                &mut surface,
                &mut tree,
                &renderer,
                &limits,
            );
            reuse_elapsed += reuse_started.elapsed();
            black_box(&node);
            last_node = node;
            last_surface = surface;
        }

        let controller_started = Instant::now();
        for pass in 0..CONTROLLER_PASSES {
            let mut editor = Editor::new(document.clone());
            let mut selection = Selection::caret(0);
            apply_action(
                &mut editor,
                0,
                &mut selection,
                Action::Click {
                    byte: valid_offsets[pass % valid_offsets.len()],
                    extend: false,
                },
            )
            .unwrap();
            black_box((editor, selection));
        }
        let controller_elapsed = controller_started.elapsed();

        let state = tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        let content_height = state.paragraphs.last().map_or(1.0, |paragraph| {
            paragraph.bounds.y + paragraph.bounds.height
        });
        let hit_started = Instant::now();
        let mut hit_checksum = 0;
        for index in 0..HIT_TESTS {
            let point = Point::new((index % 520) as f32, (index as f32 * 7.0) % content_height);
            hit_checksum ^= document_hit(&state.paragraphs, &document, point);
        }
        let hit_elapsed = hit_started.elapsed();
        black_box(hit_checksum);

        let layout = Layout::new(&last_node);
        let viewport = Rectangle::new(Point::ORIGIN, last_node.size());
        let mut draw_renderer = ShapedTestRenderer::default();
        let draw_started = Instant::now();
        for _ in 0..DRAW_PASSES {
            <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::draw(
                &last_surface,
                &tree,
                &mut draw_renderer,
                &(),
                &renderer::Style::default(),
                layout,
                mouse::Cursor::Unavailable,
                &viewport,
            );
        }
        let draw_elapsed = draw_started.elapsed();

        let mut edited_document = document.clone();
        let mut edit_tree = new_tree();
        let mut edit_surface = surface(
            &edited_document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut edit_surface,
            &mut edit_tree,
            &renderer,
            &limits,
        );
        drop(edit_surface);
        let edited_paragraphs = [
            0,
            edited_document.paragraphs().len() / 2,
            edited_document.paragraphs().len() - 1,
        ];
        let mut edit_layout_samples = Vec::with_capacity(300);
        for pass in 0..300 {
            let paragraph = edited_paragraphs[pass % edited_paragraphs.len()];
            if (pass / edited_paragraphs.len()) % 2 == 0 {
                edited_document.paragraph_mut(paragraph).text.push('x');
            } else {
                assert_eq!(
                    edited_document.paragraph_mut(paragraph).text.pop(),
                    Some('x')
                );
            }
            let mut surface = surface(
                &edited_document,
                LayoutEpoch::new(1, pass as u64 + 1),
                1.0,
                Selection::caret(0),
                |action| action,
            );
            let started = Instant::now();
            black_box(<Surface<'_, Action> as Widget<
                Action,
                (),
                ShapedTestRenderer,
            >>::layout(
                &mut surface, &mut edit_tree, &renderer, &limits
            ));
            edit_layout_samples.push(started.elapsed());
        }
        edit_layout_samples.sort_unstable();
        let edit_latency = |percentile: usize| {
            edit_layout_samples[(edit_layout_samples.len() - 1) * percentile / 100].as_secs_f64()
                * 1_000.0
        };
        let edit_state = edit_tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        assert_eq!(
            edit_state.paragraph_rebuilds,
            edited_document.paragraphs().len() + edit_layout_samples.len(),
            "one ordinary edit generation must reshape one paragraph"
        );

        eprintln!(
            "rich editor baseline: {} paragraphs; projection {:.3} ms/pass; cold projection+reshape {:.3} ms/pass; warm surface construction {:.3} us/pass; warm selection layout {:.3} us/pass; edited-paragraph layout median {:.3} ms, p95 {:.3} ms, worst {:.3} ms; selection controller+document clone {:.3} us/call; hit test {:.3} us/call; draw traversal {:.3} ms/pass",
            document.paragraphs().len(),
            projection_elapsed.as_secs_f64() * 1_000.0 / PROJECTION_PASSES as f64,
            cold_elapsed.as_secs_f64() * 1_000.0 / COLD_LAYOUT_PASSES as f64,
            construction_elapsed.as_secs_f64() * 1_000_000.0 / WARM_LAYOUT_PASSES as f64,
            reuse_elapsed.as_secs_f64() * 1_000_000.0 / WARM_LAYOUT_PASSES as f64,
            edit_latency(50),
            edit_latency(95),
            edit_layout_samples.last().unwrap().as_secs_f64() * 1_000.0,
            controller_elapsed.as_secs_f64() * 1_000_000.0 / CONTROLLER_PASSES as f64,
            hit_elapsed.as_secs_f64() * 1_000_000.0 / HIT_TESTS as f64,
            draw_elapsed.as_secs_f64() * 1_000.0 / DRAW_PASSES as f64,
        );
    }

    #[test]
    fn selection_typing_and_ime_commit_are_atomic_semantic_edits() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Bold 🙂 tail".into();
        document.paragraph_mut(0).runs = vec![CharacterRun {
            start: 0,
            end: 4,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        }];
        let original = document.clone();
        let mut editor = Editor::new(document);
        let mut selection = Selection {
            anchor: 5,
            focus: 9,
        };

        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::Insert("café".into()),
        )
        .unwrap();

        assert_eq!(editor.document().paragraphs()[0].text, "Bold café tail");
        assert_eq!(
            editor.document().paragraphs()[0].runs,
            original.paragraphs()[0].runs
        );
        assert_eq!(selection, Selection::caret(10));
        apply_action(&mut editor, 0, &mut selection, Action::Undo).unwrap();
        assert_eq!(editor.document(), &original);
        apply_action(&mut editor, 0, &mut selection, Action::Redo).unwrap();
        assert_eq!(editor.document().paragraphs()[0].text, "Bold café tail");
    }

    #[test]
    fn navigation_selection_and_deletion_stay_on_utf8_boundaries() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "A🙂é".into();
        let mut editor = Editor::new(document);
        let mut selection = Selection::caret(0);

        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::MoveRight { extend: false },
        )
        .unwrap();
        assert_eq!(selection, Selection::caret(1));
        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::MoveRight { extend: true },
        )
        .unwrap();
        assert_eq!(
            selection,
            Selection {
                anchor: 1,
                focus: 5
            }
        );
        apply_action(&mut editor, 0, &mut selection, Action::Delete).unwrap();
        assert_eq!(editor.document().paragraphs()[0].text, "Aé");
        assert_eq!(selection, Selection::caret(1));
        apply_action(&mut editor, 0, &mut selection, Action::Backspace).unwrap();
        assert_eq!(editor.document().paragraphs()[0].text, "é");
    }

    #[test]
    fn invalid_hit_position_cannot_enter_transient_state() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "🙂".into();
        let mut editor = Editor::new(document);
        let mut selection = Selection::caret(0);

        assert_eq!(
            apply_action(
                &mut editor,
                0,
                &mut selection,
                Action::Click {
                    byte: 1,
                    extend: false,
                },
            ),
            Err("selection is not a UTF-8 boundary")
        );
        assert_eq!(selection, Selection::caret(0));
    }

    #[test]
    fn home_end_left_and_select_all_share_the_same_selection_state() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "one two".into();
        let mut editor = Editor::new(document);
        let mut selection = Selection::caret(3);

        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::MoveLeft { extend: true },
        )
        .unwrap();
        assert_eq!(
            selection,
            Selection {
                anchor: 3,
                focus: 2
            }
        );
        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::MoveEnd { extend: false },
        )
        .unwrap();
        assert_eq!(selection, Selection::caret(7));
        apply_action(
            &mut editor,
            0,
            &mut selection,
            Action::MoveHome { extend: true },
        )
        .unwrap();
        assert_eq!(
            selection,
            Selection {
                anchor: 7,
                focus: 0
            }
        );
        apply_action(&mut editor, 0, &mut selection, Action::SelectAll).unwrap();
        assert_eq!(
            selection,
            Selection {
                anchor: 0,
                focus: 7
            }
        );
    }

    #[test]
    fn surface_defers_projection_until_layout_and_retains_shaped_state() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Rich text".into();
        document.paragraph_mut(0).runs = vec![CharacterRun {
            start: 0,
            end: 4,
            format: CharacterFormat {
                underline: Some(true),
                ..CharacterFormat::default()
            },
        }];

        let mut surface = surface(
            &document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection::caret(0),
            |_| (),
        );
        assert!(std::ptr::eq(surface.document, &document));
        let mut tree = widget::Tree {
            tag: <Surface<'_, ()> as Widget<(), (), ShapedTestRenderer>>::tag(&surface),
            id: None,
            state: <Surface<'_, ()> as Widget<(), (), ShapedTestRenderer>>::state(&surface),
            children: Vec::new(),
        };
        let state = tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        assert!(state.paragraphs.is_empty());

        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 300.0));
        <Surface<'_, ()> as Widget<(), (), ShapedTestRenderer>>::layout(
            &mut surface,
            &mut tree,
            &renderer,
            &limits,
        );
        let state = tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        assert_eq!(state.paragraphs.len(), 1);
        assert_eq!(state.paragraphs[0].underlines.len(), 1);
        assert_eq!(state.layout_rebuilds, 1);
    }

    #[test]
    fn layout_cache_reuses_selection_and_scroll_but_invalidates_geometry_and_document_changes() {
        let mut document = Document::from_plain_text(
            "first paragraph with enough words to wrap at a narrow width\nsecond paragraph",
        );
        document.paragraph_mut(1).runs = vec![CharacterRun {
            start: 0,
            end: "second".len(),
            format: CharacterFormat {
                font_size_points: Some(18.0),
                bold: Some(true),
                underline: Some(true),
                ..CharacterFormat::default()
            },
        }];
        let mut cached_surface = surface(
            &document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        let mut tree = widget::Tree {
            tag: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::tag(
                &cached_surface,
            ),
            id: None,
            state: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::state(
                &cached_surface,
            ),
            children: Vec::new(),
        };
        let renderer = ShapedTestRenderer::default();
        let wide = layout::Limits::new(Size::ZERO, Size::new(400.0, 300.0));
        let narrow = layout::Limits::new(Size::ZERO, Size::new(180.0, 300.0));
        let node = <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut cached_surface,
            &mut tree,
            &renderer,
            &wide,
        );
        {
            let state = tree
                .state
                .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
            assert_eq!(state.layout_rebuilds, 1);
            assert_eq!(state.paragraphs.len(), 2);
            assert_eq!(state.paragraphs[1].underlines.len(), 1);
        }

        let mut selection_surface = surface(
            &document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection {
                anchor: 2,
                focus: 20,
            },
            |action| action,
        );
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut selection_surface,
            &mut tree,
            &renderer,
            &wide,
        );
        let mut draw_renderer = ShapedTestRenderer::default();
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::draw(
            &selection_surface,
            &tree,
            &mut draw_renderer,
            &(),
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::new(Point::new(0.0, 40.0), Size::new(400.0, 120.0)),
        );
        assert_eq!(
            tree.state
                .downcast_ref::<SurfaceState<graphics::text::Paragraph>>()
                .layout_rebuilds,
            1,
            "selection and an outer-scroll viewport must reuse shaped text"
        );

        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut selection_surface,
            &mut tree,
            &renderer,
            &narrow,
        );
        assert_eq!(
            tree.state
                .downcast_ref::<SurfaceState<graphics::text::Paragraph>>()
                .layout_rebuilds,
            2,
            "width changes must invalidate wrapping"
        );

        let mut scaled_surface = surface(
            &document,
            LayoutEpoch::new(1, 0),
            0.8,
            Selection::caret(0),
            |action| action,
        );
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut scaled_surface,
            &mut tree,
            &renderer,
            &narrow,
        );
        assert_eq!(
            tree.state
                .downcast_ref::<SurfaceState<graphics::text::Paragraph>>()
                .layout_rebuilds,
            3,
            "display scale changes must invalidate metrics"
        );

        drop(cached_surface);
        drop(selection_surface);
        drop(scaled_surface);
        document = Document::from_plain_text("replacement text\nsecond paragraph");
        document.paragraph_mut(1).runs.clear();
        let mut edited_surface = surface(
            &document,
            LayoutEpoch::new(1, 1),
            0.8,
            Selection::caret(0),
            |action| action,
        );
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut edited_surface,
            &mut tree,
            &renderer,
            &narrow,
        );
        let state = tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        assert_eq!(state.layout_rebuilds, 4);
        assert_eq!(
            state.paragraphs[0].paragraph.buffer().lines[0].text(),
            "replacement text"
        );
        assert!(state.paragraphs[1].underlines.is_empty());
    }

    #[test]
    fn edit_generation_reshapes_only_the_changed_paragraph() {
        let mut document =
            Document::from_plain_text("unchanged first\nmiddle cafe\nunchanged third");
        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 300.0));
        let mut initial = surface(
            &document,
            LayoutEpoch::new(17, 0),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        let mut tree = widget::Tree {
            tag: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::tag(&initial),
            id: None,
            state: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::state(&initial),
            children: Vec::new(),
        };
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut initial,
            &mut tree,
            &renderer,
            &limits,
        );
        let state = tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        assert_eq!(state.paragraph_rebuilds, 3);

        drop(initial);
        document.paragraph_mut(1).text = "middle cafe 🙂".into();
        let mut edited = surface(
            &document,
            LayoutEpoch::new(17, 1),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut edited,
            &mut tree,
            &renderer,
            &limits,
        );
        let state = tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        assert_eq!(state.layout_rebuilds, 2);
        assert_eq!(state.paragraph_rebuilds, 4);
        assert_eq!(state.paragraphs[0].source.text, "unchanged first");
        assert_eq!(state.paragraphs[1].source.text, "middle cafe 🙂");
        assert_eq!(state.paragraphs[2].source.text, "unchanged third");

        drop(edited);
        document.paragraph_mut(1).style = amipro_document::StyleId(2);
        let mut restyled = surface(
            &document,
            LayoutEpoch::new(17, 2),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut restyled,
            &mut tree,
            &renderer,
            &limits,
        );
        let state = tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        assert_eq!(state.paragraph_rebuilds, 5);
        assert_eq!(state.paragraphs[1].style.id, amipro_document::StyleId(2));
    }

    #[test]
    fn cached_layout_still_draws_cross_paragraph_selection_highlights() {
        let document = Document::from_plain_text("alpha\nbeta");
        let mut caret_surface = surface(
            &document,
            LayoutEpoch::new(9, 0),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        let mut tree = widget::Tree {
            tag: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::tag(
                &caret_surface,
            ),
            id: None,
            state: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::state(
                &caret_surface,
            ),
            children: Vec::new(),
        };
        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 300.0));
        let node = <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut caret_surface,
            &mut tree,
            &renderer,
            &limits,
        );

        let mut selection_surface = surface(
            &document,
            LayoutEpoch::new(9, 0),
            1.0,
            Selection {
                anchor: 1,
                focus: 8,
            },
            |action| action,
        );
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut selection_surface,
            &mut tree,
            &renderer,
            &limits,
        );
        assert_eq!(
            tree.state
                .downcast_ref::<SurfaceState<graphics::text::Paragraph>>()
                .layout_rebuilds,
            1
        );

        let mut draw_renderer = ShapedTestRenderer::default();
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::draw(
            &selection_surface,
            &tree,
            &mut draw_renderer,
            &(),
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::new(Point::ORIGIN, node.size()),
        );
        assert_eq!(draw_renderer.paragraph_count, 2);
        assert!(
            draw_renderer.quad_count >= 2,
            "both selected paragraphs should emit highlight geometry"
        );
    }

    #[test]
    fn layout_cache_invalidates_same_generation_when_document_session_changes() {
        let mut first = Document::default();
        first.paragraph_mut(0).text = "first tab".into();
        let mut second = Document::default();
        second.paragraph_mut(0).text = "second tab".into();
        let mut first_surface = surface(
            &first,
            LayoutEpoch::new(41, 0),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        let mut tree = widget::Tree {
            tag: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::tag(
                &first_surface,
            ),
            id: None,
            state: <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::state(
                &first_surface,
            ),
            children: Vec::new(),
        };
        let renderer = ShapedTestRenderer::default();
        let limits = layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0));
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut first_surface,
            &mut tree,
            &renderer,
            &limits,
        );

        let mut other_tab = surface(
            &second,
            LayoutEpoch::new(42, 0),
            1.0,
            Selection::caret(0),
            |action| action,
        );
        <Surface<'_, Action> as Widget<Action, (), ShapedTestRenderer>>::layout(
            &mut other_tab,
            &mut tree,
            &renderer,
            &limits,
        );
        let state = tree
            .state
            .downcast_ref::<SurfaceState<graphics::text::Paragraph>>();
        assert_eq!(state.layout_rebuilds, 2);
        assert_eq!(
            state.paragraphs[0].paragraph.buffer().lines[0].text(),
            "second tab"
        );
    }

    #[test]
    fn physical_prototype_fixture_is_valid_and_exercises_mixed_metrics() {
        let document = Document::decode_native(include_str!(
            "../tests/fixtures/rich-editor-prototype.amipro"
        ))
        .unwrap();
        let surface = surface(
            &document,
            LayoutEpoch::new(1, 0),
            1.0,
            Selection::caret(0),
            |_| (),
        );

        assert_eq!(document.paragraphs().len(), 1);
        let projection = project_paragraph(surface.document, 0, surface.display_scale).unwrap();
        assert_eq!(projection.spans.len(), 7);
        assert!(projection.spans.iter().any(|span| span.display.underline));
        assert!(
            projection
                .spans
                .iter()
                .any(|span| span.display.size == Some(cosmic::iced::Pixels(22.0 * 96.0 / 72.0)))
        );
        assert!(
            projection
                .spans
                .iter()
                .any(|span| span.display.size == Some(cosmic::iced::Pixels(8.0 * 96.0 / 72.0)))
        );
    }
}
