//! Draw mode (`docs/Design/UI_SPEC.md` §33, decisions DW1–DW9;
//! `docs/research/DRAWING_NOTES.md`, "Draw mode"): editing a drawing in its
//! frame on the page with Ami Pro's tools and gestures and the run's
//! additions (DW5). This is the mode's state and logic, independent of the
//! widget: the page surface sends pointer and key events in the drawing's
//! own twips ([`Event`]), and each finished gesture gives the drawing's new
//! objects ([`Outcome`]), which the application applies as one
//! `Command::SetDrawing`. What the surface draws over the drawing (handles,
//! the rubber band, the grid, guides) is [`DrawMode::overlay`].

use amipro_editing::drawing::{self, Align, Order, StyleChange};
use amipro_pictures::sdw::{self, Colour, Drawing, Fill, Object, Pen, Point, Rect, Shape, Text};

/// A point in the drawing's twips, y down.
pub(crate) type At = (f64, f64);

/// Ami Pro's ten tools, in the icon bar's order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Tool {
    #[default]
    Select,
    Hand,
    Line,
    Polyline,
    Polygon,
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Arc,
    Text,
}

impl Tool {
    pub(crate) const ALL: [Tool; 10] = [
        Tool::Select,
        Tool::Hand,
        Tool::Line,
        Tool::Polyline,
        Tool::Polygon,
        Tool::Rectangle,
        Tool::RoundedRectangle,
        Tool::Ellipse,
        Tool::Arc,
        Tool::Text,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Hand => "Hand: move the drawing in its frame",
            Tool::Line => "Line (Shift: level, plumb, or 45°)",
            Tool::Polyline => "Polyline (double-click ends; Shift and drag: freehand)",
            Tool::Polygon => "Polygon (double-click ends; Shift and drag: freehand)",
            Tool::Rectangle => "Rectangle (Shift: square)",
            Tool::RoundedRectangle => "Rounded rectangle (Shift: square)",
            Tool::Ellipse => "Ellipse (Shift: circle)",
            Tool::Arc => "Arc",
            Tool::Text => "Text",
        }
    }

    /// Whether the tool draws a shape by dragging from corner to corner.
    fn drags_a_shape(self) -> bool {
        matches!(
            self,
            Tool::Line | Tool::Rectangle | Tool::RoundedRectangle | Tool::Ellipse | Tool::Arc
        )
    }
}

/// The grid (Grid Settings; `AmiDraw.ini [Grid]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Grid {
    pub show: bool,
    pub snap: bool,
    /// Twips between lines.
    pub spacing: i32,
    /// Dots at the crossings, else dotted lines.
    pub dots: bool,
}

impl Default for Grid {
    /// Ami Pro's: hidden, no snap, Medium (¼ in), dots.
    fn default() -> Self {
        Self {
            show: false,
            snap: false,
            spacing: 360,
            dots: true,
        }
    }
}

/// The look new text objects get (the Text menu's font in Draw mode).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextLook {
    pub face: String,
    /// Twips, negative: the character height (Ami Pro's −240 for 12 pt).
    pub height: i16,
    pub colour: Colour,
    pub attributes: u8,
}

impl Default for TextLook {
    /// Ami Pro's: Times New Roman 12 pt, black.
    fn default() -> Self {
        Self {
            face: "Times New Roman".into(),
            height: -240,
            colour: Colour::BLACK,
            attributes: 0,
        }
    }
}

/// A gesture's modifier keys.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Keys {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// What the page surface reports in Draw mode.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Event {
    /// The left button went down in the frame.
    Press {
        at: At,
        keys: Keys,
        double: bool,
    },
    /// The pointer moved (button down or a polyline being drawn).
    Drag {
        at: At,
        keys: Keys,
    },
    Release {
        at: At,
        keys: Keys,
    },
    /// The right button: cancels a gesture, as in Ami Pro.
    Cancel,
    Key(Key, Keys),
    /// Typed text (the Text tool).
    Text(String),
    /// A press outside the frame, or Done: Draw mode ends (DW9).
    Leave,
}

/// The keys Draw mode takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Key {
    Left,
    Right,
    Up,
    Down,
    Delete,
    Backspace,
    Enter,
    Escape,
    Tab,
}

/// What a step of the mode asks the application to do.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Outcome {
    /// The drawing's objects now, to set as one step.
    pub drawing: Option<Drawing>,
    /// The step continues the last one (typing a text object): amend it.
    pub amend: bool,
    /// The Hand tool: move the drawing in its frame by this many twips.
    pub pan: Option<(i32, i32)>,
    /// Leave Draw mode.
    pub leave: bool,
    pub status: Option<&'static str>,
}

impl Outcome {
    fn set(drawing: Drawing) -> Self {
        Self {
            drawing: Some(drawing),
            ..Self::default()
        }
    }
}

/// How the surface measures text and how big things are on screen.
pub(crate) struct Context<'a> {
    pub drawing: &'a Drawing,
    /// Twips a screen pixel is, in the drawing (for hit tolerances).
    pub pixel: f64,
    /// A text's natural width and height in twips (its font at its size).
    pub measure: &'a dyn Fn(&Text) -> (f64, f64),
}

/// A selection handle's place on the selection's rectangle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HandleKind {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    /// A line's end (0 or 1).
    End(usize),
}

impl HandleKind {
    const SIDES: [HandleKind; 8] = [
        HandleKind::TopLeft,
        HandleKind::Top,
        HandleKind::TopRight,
        HandleKind::Right,
        HandleKind::BottomRight,
        HandleKind::Bottom,
        HandleKind::BottomLeft,
        HandleKind::Left,
    ];

    fn is_corner(self) -> bool {
        matches!(
            self,
            HandleKind::TopLeft
                | HandleKind::TopRight
                | HandleKind::BottomRight
                | HandleKind::BottomLeft
        )
    }

    fn place(self, rect: Rect) -> At {
        let (l, t, r, b) = (
            f64::from(rect.left),
            f64::from(rect.top),
            f64::from(rect.right),
            f64::from(rect.bottom),
        );
        let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
        match self {
            HandleKind::TopLeft => (l, t),
            HandleKind::Top => (cx, t),
            HandleKind::TopRight => (r, t),
            HandleKind::Right => (r, cy),
            HandleKind::BottomRight => (r, b),
            HandleKind::Bottom => (cx, b),
            HandleKind::BottomLeft => (l, b),
            HandleKind::Left => (l, cy),
            HandleKind::End(_) => (l, t),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Gesture {
    /// A shape tool's drag.
    Shape {
        start: At,
        current: At,
    },
    /// A polyline or polygon: the points so far and the pointer.
    Poly {
        points: Vec<At>,
        current: At,
        /// Shift and a drag: a point every 40 twips.
        freehand: bool,
        pressed: bool,
    },
    Marquee {
        start: At,
        current: At,
    },
    Move {
        start: At,
        current: At,
    },
    Resize {
        handle: HandleKind,
        from: Rect,
        current: At,
    },
    /// Point editing: a shape point dragged.
    Point {
        index: usize,
        current: At,
    },
    /// Point editing: the object turned by a corner arrow.
    Turn {
        centre: At,
        start: At,
        current: At,
    },
    /// Point editing: the centre of rotation moved.
    Centre {
        current: At,
    },
    /// The Hand tool.
    Pan {
        start: At,
        current: At,
    },
}

/// A text object being typed.
#[derive(Clone, Debug, PartialEq)]
struct Typing {
    /// The object's index once it exists.
    index: Option<usize>,
    origin: At,
    text: String,
    /// A byte offset in `text`.
    caret: usize,
}

/// The session's Draw mode on one drawing frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DrawMode {
    pub frame: amipro_document::FrameId,
    pub tool: Tool,
    /// Top-level objects selected, by index.
    pub selection: Vec<usize>,
    /// The look new objects get, and what Line Style and Fill Pattern show
    /// with nothing selected.
    pub pen: Pen,
    pub fill: Fill,
    pub text: TextLook,
    pub grid: Grid,
    /// Rotate's step: tenths of a degree, positive clockwise (10°
    /// clockwise by default, `ROTATEPROC`).
    pub rotate_step: i32,
    /// Point editing on this object (a double-click on it).
    pub editing: Option<usize>,
    /// Point editing's centre of rotation, when moved.
    pub centre: Option<At>,
    gesture: Option<Gesture>,
    typing: Option<Typing>,
    /// Smart guides shown while moving (DW5).
    guides: Vec<(At, At)>,
}

/// A drag no longer than this each way makes nothing (`1018:2bac`).
const CLICK: f64 = 40.0;
/// Handles' half-size, in screen pixels.
const HANDLE: f64 = 5.0;
/// How close the pointer must come to a line, in screen pixels.
const NEAR: f64 = 4.0;
/// Smart guides snap within this many screen pixels.
const GUIDE: f64 = 6.0;
/// What a nudge moves, in twips (Shift ten times as far).
const NUDGE: i32 = 72;
/// Where Ctrl+D puts its copies.
const DUPLICATE: i32 = 144;

impl DrawMode {
    pub(crate) fn new(frame: amipro_document::FrameId) -> Self {
        Self {
            frame,
            tool: Tool::Select,
            selection: Vec::new(),
            pen: Pen::default(),
            fill: Fill::default(),
            text: TextLook::default(),
            grid: Grid::default(),
            rotate_step: 100,
            editing: None,
            centre: None,
            gesture: None,
            typing: None,
            guides: Vec::new(),
        }
    }

    /// Whether the surface should report pointer moves without a button
    /// (a polyline follows the pointer).
    pub(crate) fn tracking(&self) -> bool {
        matches!(self.gesture, Some(Gesture::Poly { .. }))
    }

    /// Whether a text object is being typed.
    pub(crate) fn typing(&self) -> bool {
        self.typing.is_some()
    }

    /// Chooses a tool; what was being drawn or typed ends first (Ami
    /// Pro's `1018:207e`).
    pub(crate) fn choose(&mut self, tool: Tool, context: &Context<'_>) -> Outcome {
        let outcome = self.finish(context);
        self.tool = tool;
        self.editing = None;
        self.centre = None;
        if tool != Tool::Select {
            self.selection.clear();
        }
        outcome
    }

    /// Ends whatever is in progress: a polyline is made, a text object
    /// kept, a drag dropped.
    pub(crate) fn finish(&mut self, context: &Context<'_>) -> Outcome {
        let mut outcome = Outcome::default();
        match self.gesture.take() {
            Some(Gesture::Poly { points, .. }) => {
                if let Some(drawing) = self.make_poly(context.drawing, points) {
                    outcome = Outcome::set(drawing);
                }
            }
            _ => self.guides.clear(),
        }
        self.typing = None;
        outcome
    }

    fn snap(&self, at: At) -> At {
        if !self.grid.snap || self.grid.spacing <= 0 {
            return at;
        }
        let step = f64::from(self.grid.spacing);
        ((at.0 / step).round() * step, (at.1 / step).round() * step)
    }

    /// Handles one event.
    pub(crate) fn handle(&mut self, event: Event, context: &Context<'_>) -> Outcome {
        self.selection
            .retain(|&index| index < context.drawing.objects.len());
        if self
            .editing
            .is_some_and(|index| index >= context.drawing.objects.len())
        {
            self.editing = None;
        }
        match event {
            Event::Press { at, keys, double } => self.press(at, keys, double, context),
            Event::Drag { at, keys } => {
                self.drag(at, keys, context);
                Outcome::default()
            }
            Event::Release { at, keys } => self.release(at, keys, context),
            Event::Cancel => {
                self.gesture = None;
                self.guides.clear();
                Outcome::default()
            }
            Event::Key(key, keys) => self.key(key, keys, context),
            Event::Text(text) => self.type_text(&text, context),
            Event::Leave => Outcome {
                leave: true,
                ..self.finish(context)
            },
        }
    }

    fn press(&mut self, at: At, keys: Keys, double: bool, context: &Context<'_>) -> Outcome {
        let drawing = context.drawing;
        // A polyline or polygon: each click adds a point, a double-click
        // (or a click on a polygon's first point) ends it.
        let snapped = if keys.shift { at } else { self.snap(at) };
        let polygon = self.tool == Tool::Polygon;
        if let Some(Gesture::Poly {
            points,
            current,
            freehand,
            pressed,
        }) = &mut self.gesture
        {
            let at = snapped;
            *current = at;
            *pressed = true;
            let closes = polygon
                && points.len() >= 3
                && points.first().is_some_and(|first| {
                    (first.0 - at.0).abs() <= CLICK && (first.1 - at.1).abs() <= CLICK
                });
            if double || closes {
                let points = std::mem::take(points);
                self.gesture = None;
                return match self.make_poly(drawing, points) {
                    Some(drawing) => Outcome::set(drawing),
                    None => Outcome::default(),
                };
            }
            *freehand = keys.shift;
            if points
                .last()
                .is_none_or(|last| (last.0 - at.0).abs() > 1.0 || (last.1 - at.1).abs() > 1.0)
            {
                points.push(at);
            }
            return Outcome::default();
        }
        // A double-click with a drawing tool returns to the selection
        // arrow, as a double-click with it (`LIBMESSAGE`).
        if double && self.tool != Tool::Select && self.tool != Tool::Text {
            self.tool = Tool::Select;
        }
        match self.tool {
            Tool::Select => self.press_select(at, keys, double, context),
            Tool::Hand => {
                self.gesture = Some(Gesture::Pan {
                    start: at,
                    current: at,
                });
                Outcome::default()
            }
            Tool::Polyline | Tool::Polygon => {
                let at = if keys.shift { at } else { self.snap(at) };
                self.gesture = Some(Gesture::Poly {
                    points: vec![at],
                    current: at,
                    freehand: keys.shift,
                    pressed: true,
                });
                Outcome::default()
            }
            Tool::Text => self.press_text(at, context),
            _ => {
                let at = self.snap(at);
                self.gesture = Some(Gesture::Shape {
                    start: at,
                    current: at,
                });
                Outcome::default()
            }
        }
    }

    fn press_select(&mut self, at: At, keys: Keys, double: bool, context: &Context<'_>) -> Outcome {
        let drawing = context.drawing;
        let reach = HANDLE * context.pixel;
        // Point editing: the centre, the turning arrows, the points.
        if let Some(index) = self.editing {
            let object = &drawing.objects[index];
            let centre = self.centre.unwrap_or_else(|| centre_of(object.rect));
            if near(at, centre, reach) {
                self.gesture = Some(Gesture::Centre { current: at });
                return Outcome::default();
            }
            for corner in turn_handles(object.rect, context.pixel) {
                if near(at, corner, reach) {
                    self.gesture = Some(Gesture::Turn {
                        centre,
                        start: at,
                        current: at,
                    });
                    return Outcome::default();
                }
            }
            let points = object.points();
            if let Some(point) = points
                .iter()
                .position(|point| near(at, as_at(*point), reach))
            {
                if double {
                    // A double-click on a vertex deletes it.
                    let mut changed = drawing.clone();
                    if delete_point(&mut changed.objects[index], point) {
                        return Outcome::set(changed);
                    }
                    return Outcome::default();
                }
                self.gesture = Some(Gesture::Point {
                    index: point,
                    current: at,
                });
                return Outcome::default();
            }
            // A drag on a segment adds a vertex there.
            if let Some(segment) = segment_at(object, at, NEAR * context.pixel) {
                let mut changed = drawing.clone();
                if insert_point(&mut changed.objects[index], segment, at) {
                    self.gesture = Some(Gesture::Point {
                        index: segment + 1,
                        current: at,
                    });
                    return Outcome::set(changed);
                }
            }
            if double && hit(object, at, NEAR * context.pixel) {
                self.editing = None;
                self.centre = None;
                return Outcome::default();
            }
        }
        // The selection's handles.
        if !keys.shift
            && let Some((handle, from)) = self.handle_at(drawing, at, reach)
        {
            if double {
                // A double-click on a handle flips (`LIBMESSAGE`).
                let mut changed = drawing.clone();
                match handle {
                    HandleKind::Left | HandleKind::Right => {
                        drawing::flip(&mut changed, &self.selection, true);
                    }
                    HandleKind::Top | HandleKind::Bottom => {
                        drawing::flip(&mut changed, &self.selection, false);
                    }
                    _ => {
                        drawing::flip(&mut changed, &self.selection, true);
                        drawing::flip(&mut changed, &self.selection, false);
                    }
                }
                return Outcome::set(changed);
            }
            self.gesture = Some(Gesture::Resize {
                handle,
                from,
                current: at,
            });
            return Outcome::default();
        }
        let under = objects_at(drawing, at, NEAR * context.pixel);
        let Some(&top) = under.first() else {
            if !keys.shift && !keys.ctrl {
                self.selection.clear();
                self.editing = None;
                self.gesture = Some(Gesture::Marquee {
                    start: at,
                    current: at,
                });
            }
            return Outcome::default();
        };
        if double {
            // A double-click on an object edits its points.
            self.selection = vec![top];
            self.editing = Some(top);
            self.centre = None;
            return Outcome {
                status: Some(
                    "Editing points: drag a point, a corner arrow to turn, the dot to move the centre",
                ),
                ..Outcome::default()
            };
        }
        if keys.ctrl {
            // Ctrl+click: the next object below at this point.
            let next = self
                .selection
                .first()
                .and_then(|current| under.iter().position(|index| index == current))
                .map_or(top, |at| under[(at + 1) % under.len()]);
            self.selection = vec![next];
            self.editing = None;
            return Outcome::default();
        }
        if keys.shift {
            if let Some(at) = self.selection.iter().position(|&index| index == top) {
                self.selection.remove(at);
            } else {
                self.selection.push(top);
            }
            return Outcome::default();
        }
        if !self.selection.contains(&top) {
            self.selection = vec![top];
            self.editing = None;
        }
        self.gesture = Some(Gesture::Move {
            start: at,
            current: at,
        });
        Outcome::default()
    }

    /// The handle under `at`: the selection's joint rectangle's eight (or a
    /// constrained shape's four corners), or a single line's ends.
    fn handle_at(&self, drawing: &Drawing, at: At, reach: f64) -> Option<(HandleKind, Rect)> {
        for (handle, place) in self.handles(drawing) {
            if near(at, place, reach) {
                let from = drawing::bounds(drawing, &self.selection)?;
                return Some((handle, from));
            }
        }
        None
    }

    fn handles(&self, drawing: &Drawing) -> Vec<(HandleKind, At)> {
        if self.editing.is_some() {
            return Vec::new();
        }
        let Some(rect) = drawing::bounds(drawing, &self.selection) else {
            return Vec::new();
        };
        if let [index] = self.selection[..]
            && let Some(Object {
                shape: Shape::Line { points, .. },
                ..
            }) = drawing.objects.get(index)
        {
            return vec![
                (HandleKind::End(0), as_at(points[0])),
                (HandleKind::End(1), as_at(points[1])),
            ];
        }
        let constrained = self
            .selection
            .iter()
            .all(|&index| drawing.objects[index].constrained());
        HandleKind::SIDES
            .iter()
            .filter(|handle| !constrained || handle.is_corner())
            .map(|&handle| (handle, handle.place(rect)))
            .collect()
    }

    fn press_text(&mut self, at: At, context: &Context<'_>) -> Outcome {
        let mut outcome = self.finish(context);
        let drawing = outcome.drawing.as_ref().unwrap_or(context.drawing);
        // A click on a text object edits it; elsewhere starts a new one.
        let found = objects_at(drawing, at, NEAR * context.pixel)
            .into_iter()
            .find(|&index| matches!(drawing.objects[index].shape, Shape::Text(_)));
        self.typing = Some(match found {
            Some(index) => {
                let Shape::Text(text) = &drawing.objects[index].shape else {
                    unreachable!("a text object")
                };
                self.selection = vec![index];
                Typing {
                    index: Some(index),
                    origin: as_at(text.origin),
                    text: text.text.clone(),
                    caret: text.text.len(),
                }
            }
            None => {
                self.selection.clear();
                Typing {
                    index: None,
                    origin: self.snap(at),
                    text: String::new(),
                    caret: 0,
                }
            }
        });
        outcome.status = Some("Type the text; Enter starts another below, Esc ends");
        outcome
    }

    fn drag(&mut self, at: At, keys: Keys, context: &Context<'_>) {
        let snapped = self.snap(at);
        let others = self.others(context.drawing);
        let pixel = context.pixel;
        let selection_bounds = drawing::bounds(context.drawing, &self.selection);
        let mut guides = Vec::new();
        match &mut self.gesture {
            Some(Gesture::Shape { start, current }) => {
                *current = constrain(self.tool, *start, snapped, keys.shift);
            }
            Some(Gesture::Poly {
                points,
                current,
                freehand,
                pressed,
            }) => {
                *current = if keys.shift { at } else { snapped };
                // Freehand: a point every time the pointer has gone more
                // than 40 twips.
                if *freehand
                    && *pressed
                    && points.last().is_some_and(|last| {
                        (last.0 - at.0).abs() > CLICK || (last.1 - at.1).abs() > CLICK
                    })
                {
                    points.push(at);
                }
            }
            Some(Gesture::Marquee { current, .. })
            | Some(Gesture::Point { current, .. })
            | Some(Gesture::Centre { current })
            | Some(Gesture::Pan { current, .. })
            | Some(Gesture::Turn { current, .. })
            | Some(Gesture::Resize { current, .. }) => *current = snapped,
            Some(Gesture::Move { start, current }) => {
                let mut moved = (snapped.0 - start.0, snapped.1 - start.1);
                if let Some(rect) = selection_bounds
                    && !self.grid.snap
                {
                    moved = smart_guides(rect, moved, &others, GUIDE * pixel, &mut guides);
                }
                *current = (start.0 + moved.0, start.1 + moved.1);
            }
            None => {}
        }
        self.guides = guides;
    }

    /// The rectangles of the objects not selected (for smart guides).
    fn others(&self, drawing: &Drawing) -> Vec<Rect> {
        drawing
            .objects
            .iter()
            .enumerate()
            .filter(|(index, object)| {
                !self.selection.contains(index) && object.kind != sdw::kind::CHART
            })
            .map(|(_, object)| object.rect)
            .collect()
    }

    fn release(&mut self, at: At, keys: Keys, context: &Context<'_>) -> Outcome {
        let drawing = context.drawing;
        self.drag(at, keys, context);
        self.guides.clear();
        let Some(gesture) = self.gesture.take() else {
            return Outcome::default();
        };
        match gesture {
            Gesture::Poly {
                points,
                current,
                freehand,
                ..
            } => {
                // Releasing a freehand drag ends the object; a click keeps
                // the polyline going.
                if freehand {
                    let mut points = points;
                    points.push(current);
                    return match self.make_poly(drawing, points) {
                        Some(drawing) => Outcome::set(drawing),
                        None => Outcome::default(),
                    };
                }
                self.gesture = Some(Gesture::Poly {
                    points,
                    current,
                    freehand: false,
                    pressed: false,
                });
                Outcome::default()
            }
            Gesture::Shape { start, current } => {
                if (current.0 - start.0).abs() <= CLICK && (current.1 - start.1).abs() <= CLICK {
                    return Outcome::default();
                }
                let object = self.shape(start, current, keys.shift);
                let mut changed = drawing.clone();
                self.selection = drawing::add(&mut changed, vec![object]);
                Outcome::set(changed)
            }
            Gesture::Marquee { start, current } => {
                // A click on nothing selects nothing.
                if (current.0 - start.0).abs() <= CLICK && (current.1 - start.1).abs() <= CLICK {
                    return Outcome::default();
                }
                let area = rect_of(start, current);
                self.selection = drawing
                    .objects
                    .iter()
                    .enumerate()
                    .filter(|(_, object)| {
                        object.kind != sdw::kind::CHART && overlaps(object.rect, area)
                    })
                    .map(|(index, _)| index)
                    .collect();
                Outcome::default()
            }
            Gesture::Move { start, current } => {
                let (dx, dy) = (
                    (current.0 - start.0).round() as i32,
                    (current.1 - start.1).round() as i32,
                );
                if dx.abs() as f64 <= CLICK && dy.abs() as f64 <= CLICK {
                    return Outcome::default();
                }
                let mut changed = drawing.clone();
                if keys.shift {
                    // Shift at the release copies (Ami Pro).
                    self.selection = drawing::duplicate(&mut changed, &self.selection, dx, dy);
                } else {
                    drawing::move_by(&mut changed, &self.selection, dx, dy);
                }
                Outcome::set(changed)
            }
            Gesture::Resize {
                handle,
                from,
                current,
            } => {
                let mut changed = drawing.clone();
                if let HandleKind::End(end) = handle {
                    let [index] = self.selection[..] else {
                        return Outcome::default();
                    };
                    let object = &mut changed.objects[index];
                    let constrained = object.constrained();
                    if let Shape::Line { points, .. } = &mut object.shape {
                        let other = as_at(points[1 - end]);
                        let to = if constrained {
                            constrain(Tool::Line, other, current, true)
                        } else {
                            current
                        };
                        points[end] = to_point(to);
                        object.refresh_rect();
                    }
                    return Outcome::set(changed);
                }
                let constrained = self
                    .selection
                    .iter()
                    .all(|&index| drawing.objects[index].constrained());
                let to = resized(
                    from,
                    handle,
                    current,
                    constrained || keys.shift && handle.is_corner(),
                );
                drawing::resize(&mut changed, &self.selection, to);
                Outcome::set(changed)
            }
            Gesture::Point { index, current } => {
                let Some(object) = self.editing else {
                    return Outcome::default();
                };
                let mut changed = drawing.clone();
                set_point(&mut changed.objects[object], index, to_point(current));
                Outcome::set(changed)
            }
            Gesture::Turn {
                centre,
                start,
                current,
            } => {
                let before = (start.1 - centre.1).atan2(start.0 - centre.0);
                let after = (current.1 - centre.1).atan2(current.0 - centre.0);
                let tenths = ((after - before).to_degrees() * 10.0).round() as i32;
                if tenths == 0 {
                    return Outcome::default();
                }
                let mut changed = drawing.clone();
                drawing::rotate(&mut changed, &self.selection, tenths, Some(centre));
                Outcome::set(changed)
            }
            Gesture::Centre { current } => {
                self.centre = Some(current);
                Outcome::default()
            }
            Gesture::Pan { start, current } => {
                let (dx, dy) = (
                    (current.0 - start.0).round() as i32,
                    (current.1 - start.1).round() as i32,
                );
                Outcome {
                    pan: (dx != 0 || dy != 0).then_some((dx, dy)),
                    ..Outcome::default()
                }
            }
        }
    }

    /// The shape a drag from `start` to `end` makes with the tool.
    fn shape(&self, start: At, end: At, shift: bool) -> Object {
        let (a, b) = (to_point(start), to_point(end));
        let rect = sdw::Rect::of_points(&[a, b]);
        let rect = Rect::new(
            rect.left,
            rect.top,
            if a.x == b.x { rect.left } else { rect.right },
            if a.y == b.y { rect.top } else { rect.bottom },
        );
        match self.tool {
            Tool::Line => Object::line(a, b, self.pen, shift),
            Tool::Rectangle => Object::rectangle(rect, self.pen, self.fill, shift),
            Tool::RoundedRectangle => Object::rounded_rectangle(rect, self.pen, self.fill, shift),
            Tool::Ellipse => Object::ellipse(rect, self.pen, self.fill, shift),
            _ => Object::arc(a, b, self.pen),
        }
    }

    /// The polyline or polygon `points` make, added in front.
    fn make_poly(&mut self, drawing: &Drawing, mut points: Vec<At>) -> Option<Drawing> {
        points.dedup_by(|a, b| (a.0 - b.0).abs() < 1.0 && (a.1 - b.1).abs() < 1.0);
        let points: Vec<Point> = points.into_iter().map(to_point).collect();
        let polygon = self.tool == Tool::Polygon;
        let needed = if polygon { 3 } else { 2 };
        let rect = sdw::Rect::of_points(&points);
        if points.len() < needed
            || (f64::from(rect.width()) <= CLICK && f64::from(rect.height()) <= CLICK)
        {
            return None;
        }
        let object = if polygon {
            Object::polygon(
                points,
                sdw::Pen {
                    start: 0,
                    end: 0,
                    ..self.pen
                },
                self.fill,
            )
        } else {
            Object::polyline(points, self.pen)
        };
        let mut changed = drawing.clone();
        self.selection = drawing::add(&mut changed, vec![object]);
        Some(changed)
    }

    fn key(&mut self, key: Key, keys: Keys, context: &Context<'_>) -> Outcome {
        if self.typing.is_some() {
            return self.type_key(key, context);
        }
        let drawing = context.drawing;
        match key {
            Key::Escape => {
                if self.gesture.is_some() {
                    return self.finish(context);
                }
                if self.editing.take().is_some() {
                    self.centre = None;
                    return Outcome::default();
                }
                Outcome {
                    leave: true,
                    ..Outcome::default()
                }
            }
            Key::Enter => {
                if matches!(self.gesture, Some(Gesture::Poly { .. })) {
                    return self.finish(context);
                }
                Outcome::default()
            }
            Key::Delete | Key::Backspace => {
                if self.selection.is_empty() {
                    return Outcome::default();
                }
                let mut changed = drawing.clone();
                drawing::delete(&mut changed, &self.selection);
                self.selection.clear();
                self.editing = None;
                Outcome::set(changed)
            }
            Key::Tab => {
                let count = drawing.objects.len();
                if count == 0 {
                    return Outcome::default();
                }
                let current = self.selection.last().copied();
                let next = match (current, keys.shift) {
                    (None, false) => 0,
                    (None, true) => count - 1,
                    (Some(at), false) => (at + 1) % count,
                    (Some(at), true) => (at + count - 1) % count,
                };
                self.selection = vec![next];
                self.editing = None;
                Outcome::default()
            }
            Key::Left | Key::Right | Key::Up | Key::Down => {
                if self.selection.is_empty() {
                    return Outcome::default();
                }
                let step = if keys.alt {
                    context.pixel.round().max(1.0) as i32
                } else if self.grid.snap {
                    self.grid.spacing
                } else {
                    NUDGE
                } * if keys.shift { 10 } else { 1 };
                let (dx, dy) = match key {
                    Key::Left => (-step, 0),
                    Key::Right => (step, 0),
                    Key::Up => (0, -step),
                    _ => (0, step),
                };
                let mut changed = drawing.clone();
                drawing::move_by(&mut changed, &self.selection, dx, dy);
                Outcome {
                    amend: true,
                    ..Outcome::set(changed)
                }
            }
        }
    }

    fn type_text(&mut self, typed: &str, context: &Context<'_>) -> Outcome {
        let Some(typing) = &mut self.typing else {
            return Outcome::default();
        };
        let typed: String = typed.chars().filter(|c| !c.is_control()).collect();
        if typed.is_empty() {
            return Outcome::default();
        }
        typing.text.insert_str(typing.caret, &typed);
        typing.caret += typed.len();
        self.put_text(context)
    }

    fn type_key(&mut self, key: Key, context: &Context<'_>) -> Outcome {
        let Some(typing) = &mut self.typing else {
            return Outcome::default();
        };
        let before =
            |text: &str, at: usize| text[..at].chars().next_back().map_or(0, char::len_utf8);
        let after = |text: &str, at: usize| text[at..].chars().next().map_or(0, char::len_utf8);
        match key {
            Key::Left => {
                typing.caret -= before(&typing.text, typing.caret);
                Outcome::default()
            }
            Key::Right => {
                typing.caret += after(&typing.text, typing.caret);
                Outcome::default()
            }
            Key::Backspace => {
                let width = before(&typing.text, typing.caret);
                if width == 0 {
                    return Outcome::default();
                }
                typing.caret -= width;
                typing
                    .text
                    .replace_range(typing.caret..typing.caret + width, "");
                self.put_text(context)
            }
            Key::Delete => {
                let width = after(&typing.text, typing.caret);
                if width == 0 {
                    return Outcome::default();
                }
                typing
                    .text
                    .replace_range(typing.caret..typing.caret + width, "");
                self.put_text(context)
            }
            Key::Enter => {
                // Another text object below this one (Ami Pro).
                let origin = typing.origin;
                let line = f64::from(self.text.height.unsigned_abs()) * 1.2;
                self.typing = Some(Typing {
                    index: None,
                    origin: (origin.0, origin.1 + line),
                    text: String::new(),
                    caret: 0,
                });
                self.selection.clear();
                Outcome::default()
            }
            Key::Escape | Key::Tab | Key::Up | Key::Down => {
                self.typing = None;
                Outcome::default()
            }
        }
    }

    /// The text being typed put into the drawing (one step, amended while
    /// typing goes on).
    fn put_text(&mut self, context: &Context<'_>) -> Outcome {
        let Some(typing) = &mut self.typing else {
            return Outcome::default();
        };
        let mut changed = context.drawing.clone();
        let existing = typing
            .index
            .and_then(|index| changed.objects.get(index))
            .and_then(|object| match &object.shape {
                Shape::Text(text) => Some(text.clone()),
                _ => None,
            });
        let amend = typing.index.is_some();
        if typing.text.is_empty() {
            if let Some(index) = typing.index.take() {
                drawing::delete(&mut changed, &[index]);
                self.selection.clear();
                return Outcome {
                    amend,
                    ..Outcome::set(changed)
                };
            }
            return Outcome::default();
        }
        let mut text = existing.unwrap_or_else(|| Text {
            origin: to_point(typing.origin),
            size: Point::new(0, 0),
            face: self.text.face.clone(),
            pitch_and_family: 0,
            height: self.text.height,
            colour: self.text.colour,
            attributes: self.text.attributes,
            attributes2: 0,
            angle: 0,
            text: String::new(),
            kept: sdw::TextKept::default(),
        });
        text.text = typing.text.clone();
        let (width, height) = (context.measure)(&text);
        text.size = Point::new(
            width.round().max(1.0) as i16,
            height.round().max(1.0) as i16,
        );
        let object = Object::text(text);
        match typing.index {
            Some(index) if index < changed.objects.len() => changed.objects[index] = object,
            _ => {
                let added = drawing::add(&mut changed, vec![object]);
                typing.index = added.first().copied();
            }
        }
        self.selection = typing.index.into_iter().collect();
        Outcome {
            amend,
            ..Outcome::set(changed)
        }
    }

    /// A Draw menu command on the selection.
    pub(crate) fn command(&mut self, command: Command, context: &Context<'_>) -> Outcome {
        let drawing = context.drawing;
        let mut changed = drawing.clone();
        let selection = self.selection.clone();
        match command {
            Command::SelectAll => {
                let finished = self.finish(context);
                let objects = finished.drawing.as_ref().unwrap_or(drawing).objects.len();
                // Ami Pro's Select All is a toggle.
                self.selection = if self.selection.len() == objects {
                    Vec::new()
                } else {
                    (0..objects).collect()
                };
                self.tool = Tool::Select;
                self.editing = None;
                return finished;
            }
            Command::Group => match drawing::group(&mut changed, &selection) {
                Some(index) => self.selection = vec![index],
                None => return Outcome::default(),
            },
            Command::Ungroup => {
                let [index] = selection[..] else {
                    return Outcome::default();
                };
                let members = drawing::ungroup(&mut changed, index);
                if members.is_empty() {
                    return Outcome::default();
                }
                self.selection = members;
            }
            Command::Order(order) => {
                self.selection = drawing::order(&mut changed, &selection, order)
            }
            Command::Rotate(tenths) => {
                drawing::rotate(&mut changed, &selection, tenths, self.centre)
            }
            Command::Flip { horizontal } => drawing::flip(&mut changed, &selection, horizontal),
            Command::Align(how) => drawing::align(&mut changed, &selection, how),
            Command::Distribute { across } => drawing::distribute(&mut changed, &selection, across),
            Command::Duplicate => {
                self.selection = drawing::duplicate(&mut changed, &selection, DUPLICATE, DUPLICATE);
            }
            Command::Delete => {
                drawing::delete(&mut changed, &selection);
                self.selection.clear();
                self.editing = None;
            }
            Command::Extract => {
                let [index] = selection[..] else {
                    return Outcome::default();
                };
                let object = &drawing.objects[index];
                if let Some(pen) = object.pen() {
                    self.pen = *pen;
                }
                if let Some(fill) = object.fill() {
                    self.fill = *fill;
                }
                return Outcome {
                    status: Some("Line and fill extracted"),
                    ..Outcome::default()
                };
            }
            Command::Apply => {
                drawing::set_style(
                    &mut changed,
                    &selection,
                    &StyleChange::line_and_fill(&self.pen, &self.fill),
                );
            }
            Command::Style(change) => {
                // With nothing selected, the look of new objects.
                self.restyle_defaults(&change);
                drawing::set_style(&mut changed, &selection, &change);
            }
            Command::Place(rect) => {
                drawing::resize(&mut changed, &selection, rect);
            }
            Command::Add(objects) => {
                let finished = self.finish(context);
                let mut changed = finished.drawing.unwrap_or_else(|| drawing.clone());
                self.tool = Tool::Select;
                self.editing = None;
                self.selection = drawing::add(&mut changed, objects);
                return Outcome::set(changed);
            }
        }
        if changed == *drawing {
            return Outcome::default();
        }
        Outcome::set(changed)
    }

    fn restyle_defaults(&mut self, change: &StyleChange) {
        let mut probe = Object::rectangle(Rect::new(0, 0, 10, 10), self.pen, self.fill, false);
        drawing_style(&mut probe, change);
        if let (Some(pen), Some(fill)) = (probe.pen(), probe.fill()) {
            // Endings are a line's.
            self.pen = Pen {
                start: change.start.unwrap_or(self.pen.start),
                end: change.end.unwrap_or(self.pen.end),
                ..*pen
            };
            self.fill = *fill;
        }
        if let Some(face) = &change.face {
            self.text.face = face.clone();
        }
        if let Some(height) = change.height {
            self.text.height = height;
        }
        if let Some(colour) = change.text_colour {
            self.text.colour = colour;
        }
        for (on, bit) in [
            (change.bold, Text::BOLD),
            (change.italic, Text::ITALIC),
            (change.underline, Text::UNDERLINE),
        ] {
            if let Some(on) = on {
                if on {
                    self.text.attributes |= bit;
                } else {
                    self.text.attributes &= !bit;
                }
            }
        }
    }

    /// What the surface draws over the drawing, in its twips.
    pub(crate) fn overlay(&self, drawing: &Drawing, pixel: f64) -> Overlay {
        let mut overlay = Overlay {
            grid: self
                .grid
                .show
                .then_some((self.grid.spacing, self.grid.dots)),
            guides: self.guides.clone(),
            ..Overlay::default()
        };
        let selected: Vec<&Object> = self
            .selection
            .iter()
            .filter_map(|&index| drawing.objects.get(index))
            .collect();
        for object in &selected {
            overlay.boxes.push(object.rect);
        }
        overlay.handles = self
            .handles(drawing)
            .into_iter()
            .map(|(_, at)| at)
            .collect();
        if let Some(object) = self.editing.and_then(|index| drawing.objects.get(index)) {
            overlay.points = object.points().into_iter().map(as_at).collect();
            overlay.turns = turn_handles(object.rect, pixel).to_vec();
            overlay.centre = Some(self.centre.unwrap_or_else(|| centre_of(object.rect)));
        }
        match &self.gesture {
            Some(Gesture::Shape { start, current }) => {
                let object = self.shape(*start, *current, false);
                overlay.outlines.extend(outlines(&object));
            }
            Some(Gesture::Poly {
                points, current, ..
            }) => {
                let mut line = points.clone();
                line.push(*current);
                overlay.outlines.push((line, false));
            }
            Some(Gesture::Marquee { start, current }) => {
                overlay.marquee = Some(rect_of(*start, *current));
            }
            Some(Gesture::Move { start, current }) => {
                let map = sdw::Affine::translate(current.0 - start.0, current.1 - start.1);
                for object in &selected {
                    let mut moved = (*object).clone();
                    moved.transform(&map);
                    overlay.outlines.extend(outlines(&moved));
                }
            }
            Some(Gesture::Resize {
                handle,
                from,
                current,
            }) => {
                if let HandleKind::End(end) = handle
                    && let [object] = selected[..]
                    && let Shape::Line { points, .. } = &object.shape
                {
                    overlay
                        .outlines
                        .push((vec![as_at(points[1 - end]), *current], false));
                } else {
                    let to = resized(*from, *handle, *current, false);
                    let map = sdw::Affine::rect_to_rect(*from, to);
                    for object in &selected {
                        let mut sized = (*object).clone();
                        sized.transform(&map);
                        overlay.outlines.extend(outlines(&sized));
                    }
                }
            }
            Some(Gesture::Point { index, current }) => {
                if let Some(object) = self.editing.and_then(|at| drawing.objects.get(at)) {
                    let mut edited = object.clone();
                    set_point(&mut edited, *index, to_point(*current));
                    overlay.outlines.extend(outlines(&edited));
                }
            }
            Some(Gesture::Turn {
                centre,
                start,
                current,
            }) => {
                let before = (start.1 - centre.1).atan2(start.0 - centre.0);
                let after = (current.1 - centre.1).atan2(current.0 - centre.0);
                let tenths = ((after - before).to_degrees() * 10.0).round() as i32;
                for object in &selected {
                    let mut turned = (*object).clone();
                    turned.rotate(tenths, *centre);
                    overlay.outlines.extend(outlines(&turned));
                }
            }
            Some(Gesture::Centre { current }) => overlay.centre = Some(*current),
            Some(Gesture::Pan { start, current }) => {
                let size = drawing.natural_size();
                let (dx, dy) = (current.0 - start.0, current.1 - start.1);
                overlay.outlines.push((
                    vec![
                        (dx, dy),
                        (f64::from(size.width) + dx, dy),
                        (f64::from(size.width) + dx, f64::from(size.height) + dy),
                        (dx, f64::from(size.height) + dy),
                    ],
                    true,
                ));
            }
            None => {}
        }
        if let Some(typing) = &self.typing {
            let height = f64::from(self.text.height.unsigned_abs());
            let at = typing
                .index
                .and_then(|index| drawing.objects.get(index))
                .and_then(|object| match &object.shape {
                    Shape::Text(text) => {
                        let before: String = text.text[..typing.caret.min(text.text.len())].into();
                        let share = if text.text.is_empty() {
                            0.0
                        } else {
                            before.chars().count() as f64 / text.text.chars().count() as f64
                        };
                        Some((
                            f64::from(text.origin.x) + f64::from(text.size.x) * share,
                            f64::from(text.origin.y),
                            f64::from(text.size.y),
                        ))
                    }
                    _ => None,
                })
                .unwrap_or((typing.origin.0, typing.origin.1, height * 1.15));
            overlay.caret = Some(((at.0, at.1), (at.0, at.1 + at.2)));
        }
        overlay
    }
}

/// A Draw menu or draw bar command on the selection.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Command {
    SelectAll,
    Group,
    Ungroup,
    Order(Order),
    /// Tenths of a degree, positive clockwise.
    Rotate(i32),
    Flip {
        horizontal: bool,
    },
    Align(Align),
    Distribute {
        across: bool,
    },
    Duplicate,
    Delete,
    /// Extract Line & Fill: the one selected object's look becomes the
    /// current one.
    Extract,
    /// Apply Line & Fill: the current look on the selection.
    Apply,
    /// A change from Drawing Properties (and the defaults for new objects).
    Style(StyleChange),
    /// Position and size: the selection onto this rectangle.
    Place((f64, f64, f64, f64)),
    /// Paste and Import Drawing: these objects in front, selected.
    Add(Vec<Object>),
}

/// What the surface draws over the drawing in Draw mode, in the drawing's
/// twips.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Overlay {
    /// Shapes being drawn, moved, sized, or turned (closed when true).
    pub outlines: Vec<(Vec<At>, bool)>,
    /// Each selected object's rectangle.
    pub boxes: Vec<Rect>,
    pub handles: Vec<At>,
    /// Point editing: the shape's points, the turning arrows, the centre.
    pub points: Vec<At>,
    pub turns: Vec<At>,
    pub centre: Option<At>,
    pub marquee: Option<(f64, f64, f64, f64)>,
    pub guides: Vec<(At, At)>,
    /// The grid's spacing and whether it is dots.
    pub grid: Option<(i32, bool)>,
    /// The text caret, top to bottom.
    pub caret: Option<(At, At)>,
}

/// Where the page surface shows the drawing: the page, where the
/// drawing's origin is in page twips, page twips a drawing twip each way,
/// and the frame's rectangle (page twips), which gestures start in and the
/// overlay is clipped to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub page_index: usize,
    pub origin: (f32, f32),
    pub scale: (f32, f32),
    pub clip: amipro_layout::Bounds,
}

impl Placement {
    /// A page point (twips) in the drawing's twips.
    pub(crate) fn to_drawing(self, x: f32, y: f32) -> At {
        (
            f64::from((x - self.origin.0) / self.scale.0.max(1e-6)),
            f64::from((y - self.origin.1) / self.scale.1.max(1e-6)),
        )
    }

    /// A drawing point in page twips.
    pub(crate) fn to_page(self, at: At) -> (f32, f32) {
        (
            self.origin.0 + at.0 as f32 * self.scale.0,
            self.origin.1 + at.1 as f32 * self.scale.1,
        )
    }

    pub(crate) fn contains(self, x: f32, y: f32) -> bool {
        x >= self.clip.x
            && y >= self.clip.y
            && x <= self.clip.x + self.clip.width
            && y <= self.clip.y + self.clip.height
    }
}

/// The overlay drawn into a `width` × `height` pixmap, `to_pixel` mapping
/// the drawing's twips onto it: the accent colour, one pixel lines, the
/// grid in blue (Ami Pro's).
pub(crate) fn render_overlay(
    overlay: &Overlay,
    width: u32,
    height: u32,
    to_pixel: &dyn Fn(At) -> (f32, f32),
    pixel_twips: f64,
) -> Option<tiny_skia::Pixmap> {
    use tiny_skia::{Paint, PathBuilder, Stroke, StrokeDash, Transform};
    let mut pixmap = tiny_skia::Pixmap::new(width.max(1), height.max(1))?;
    let paint = |r: u8, g: u8, b: u8, a: u8| {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        paint.set_color_rgba8(r, g, b, a);
        paint
    };
    let accent = paint(51, 115, 230, 255);
    // The grid in a clear blue (Ami Pro's), its dots two pixels so they
    // show at any scale.
    let blue = paint(30, 90, 230, 255);
    let thin = Stroke {
        width: 1.0,
        ..Stroke::default()
    };
    let dashed = Stroke {
        width: 1.0,
        dash: StrokeDash::new(vec![4.0, 3.0], 0.0),
        ..Stroke::default()
    };
    let line = |pixmap: &mut tiny_skia::Pixmap,
                points: &[At],
                closed: bool,
                stroke: &Stroke,
                paint: &Paint| {
        let mut builder = PathBuilder::new();
        for (index, &at) in points.iter().enumerate() {
            let (x, y) = to_pixel(at);
            if index == 0 {
                builder.move_to(x, y);
            } else {
                builder.line_to(x, y);
            }
        }
        if closed {
            builder.close();
        }
        if let Some(path) = builder.finish() {
            pixmap.stroke_path(&path, paint, stroke, Transform::identity(), None);
        }
    };
    // The grid: dots at the crossings or dotted lines, when they are at
    // least two pixels apart (`GRIDPROC`).
    if let Some((spacing, dots)) = overlay.grid
        && f64::from(spacing) / pixel_twips >= 2.0
    {
        let step = f64::from(spacing);
        let (left, top) = (
            -to_pixel((0.0, 0.0)).0 as f64,
            -to_pixel((0.0, 0.0)).1 as f64,
        );
        let (x0, y0) = (
            (left * pixel_twips / step).floor() * step,
            (top * pixel_twips / step).floor() * step,
        );
        let (x1, y1) = (
            x0 + (f64::from(width) + 2.0) * pixel_twips,
            y0 + (f64::from(height) + 2.0) * pixel_twips,
        );
        let mut x = x0;
        while x <= x1 {
            if dots {
                let mut y = y0;
                while y <= y1 {
                    let (px, py) = to_pixel((x, y));
                    if let Some(rect) =
                        tiny_skia::Rect::from_xywh(px.round() - 1.0, py.round() - 1.0, 2.0, 2.0)
                    {
                        pixmap.fill_rect(rect, &blue, Transform::identity(), None);
                    }
                    y += step;
                }
            } else {
                line(&mut pixmap, &[(x, y0), (x, y1)], false, &dashed, &blue);
            }
            x += step;
        }
        if !dots {
            let mut y = y0;
            while y <= y1 {
                line(&mut pixmap, &[(x0, y), (x1, y)], false, &dashed, &blue);
                y += step;
            }
        }
    }
    for rect in &overlay.boxes {
        let (l, t, r, b) = (
            f64::from(rect.left),
            f64::from(rect.top),
            f64::from(rect.right),
            f64::from(rect.bottom),
        );
        line(
            &mut pixmap,
            &[(l, t), (r, t), (r, b), (l, b)],
            true,
            &dashed,
            &accent,
        );
    }
    for (points, closed) in &overlay.outlines {
        line(&mut pixmap, points, *closed, &thin, &accent);
    }
    if let Some((l, t, r, b)) = overlay.marquee {
        line(
            &mut pixmap,
            &[(l, t), (r, t), (r, b), (l, b)],
            true,
            &dashed,
            &accent,
        );
    }
    for (a, b) in &overlay.guides {
        line(
            &mut pixmap,
            &[*a, *b],
            false,
            &thin,
            &paint(230, 0, 160, 255),
        );
    }
    let white = paint(255, 255, 255, 255);
    let square = |pixmap: &mut tiny_skia::Pixmap, at: At, half: f32, fill: &Paint, edge: &Paint| {
        let (x, y) = to_pixel(at);
        if let Some(rect) =
            tiny_skia::Rect::from_xywh(x.round() - half, y.round() - half, 2.0 * half, 2.0 * half)
        {
            pixmap.fill_rect(rect, fill, Transform::identity(), None);
            let path = PathBuilder::from_rect(rect);
            pixmap.stroke_path(&path, edge, &thin, Transform::identity(), None);
        }
    };
    for &at in &overlay.handles {
        square(&mut pixmap, at, 3.5, &accent, &white);
    }
    for &at in &overlay.points {
        square(&mut pixmap, at, 3.0, &white, &accent);
    }
    for &at in &overlay.turns {
        let (x, y) = to_pixel(at);
        if let Some(path) = PathBuilder::from_circle(x, y, 4.5) {
            pixmap.stroke_path(
                &path,
                &accent,
                &Stroke {
                    width: 1.5,
                    ..Stroke::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
    if let Some(at) = overlay.centre {
        let (x, y) = to_pixel(at);
        if let Some(path) = PathBuilder::from_circle(x, y, 3.5) {
            pixmap.fill_path(
                &path,
                &paint(0, 0, 0, 255),
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }
    if let Some((a, b)) = overlay.caret {
        line(
            &mut pixmap,
            &[a, b],
            false,
            &Stroke {
                width: 1.5,
                ..Stroke::default()
            },
            &paint(0, 0, 0, 255),
        );
    }
    Some(pixmap)
}

fn drawing_style(object: &mut Object, change: &StyleChange) {
    let mut drawing = Drawing {
        objects: vec![object.clone()],
        ..Drawing::new()
    };
    drawing::set_style(&mut drawing, &[0], change);
    *object = drawing.objects.remove(0);
}

fn as_at(point: Point) -> At {
    (f64::from(point.x), f64::from(point.y))
}

fn to_point(at: At) -> Point {
    let clamp = |value: f64| {
        value
            .round()
            .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
    };
    Point::new(clamp(at.0), clamp(at.1))
}

fn near(a: At, b: At, reach: f64) -> bool {
    (a.0 - b.0).abs() <= reach && (a.1 - b.1).abs() <= reach
}

fn centre_of(rect: Rect) -> At {
    (
        (f64::from(rect.left) + f64::from(rect.right)) / 2.0,
        (f64::from(rect.top) + f64::from(rect.bottom)) / 2.0,
    )
}

/// Point editing's turning arrows: just outside the object's corners.
fn turn_handles(rect: Rect, pixel: f64) -> [At; 4] {
    let out = 10.0 * pixel;
    let (l, t, r, b) = (
        f64::from(rect.left) - out,
        f64::from(rect.top) - out,
        f64::from(rect.right) + out,
        f64::from(rect.bottom) + out,
    );
    [(l, t), (r, t), (r, b), (l, b)]
}

fn rect_of(a: At, b: At) -> (f64, f64, f64, f64) {
    (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

fn overlaps(rect: Rect, area: (f64, f64, f64, f64)) -> bool {
    f64::from(rect.right) >= area.0
        && f64::from(rect.left) <= area.2
        && f64::from(rect.bottom) >= area.1
        && f64::from(rect.top) <= area.3
}

/// Shift's constraints while drawing: a line level, plumb, or at 45°; a
/// square, rounded square, or circle.
fn constrain(tool: Tool, start: At, end: At, shift: bool) -> At {
    if !shift || !tool.drags_a_shape() || tool == Tool::Arc {
        return end;
    }
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    if tool == Tool::Line {
        let (ax, ay) = (dx.abs(), dy.abs());
        // The nearest of level, plumb, and the diagonal by the larger.
        return if ay < ax * 0.4142 {
            (end.0, start.1)
        } else if ax < ay * 0.4142 {
            (start.0, end.1)
        } else {
            let side = ax.max(ay);
            (start.0 + side * dx.signum(), start.1 + side * dy.signum())
        };
    }
    let side = dx.abs().max(dy.abs());
    (start.0 + side * dx.signum(), start.1 + side * dy.signum())
}

/// Where a handle dragged to `to` puts the selection's rectangle (left,
/// top, right, bottom; past the opposite side it flips).
fn resized(from: Rect, handle: HandleKind, to: At, keep_aspect: bool) -> (f64, f64, f64, f64) {
    let (mut l, mut t, mut r, mut b) = (
        f64::from(from.left),
        f64::from(from.top),
        f64::from(from.right),
        f64::from(from.bottom),
    );
    match handle {
        HandleKind::TopLeft => (l, t) = to,
        HandleKind::Top => t = to.1,
        HandleKind::TopRight => (r, t) = to,
        HandleKind::Right => r = to.0,
        HandleKind::BottomRight => (r, b) = to,
        HandleKind::Bottom => b = to.1,
        HandleKind::BottomLeft => (l, b) = to,
        HandleKind::Left => l = to.0,
        HandleKind::End(_) => {}
    }
    if keep_aspect && handle.is_corner() {
        let (w0, h0) = (
            f64::from(from.width().max(1)),
            f64::from(from.height().max(1)),
        );
        let (w, h) = (r - l, b - t);
        let scale = (w.abs() / w0).max(h.abs() / h0);
        let (w, h) = (w0 * scale * w.signum(), h0 * scale * h.signum());
        match handle {
            HandleKind::TopLeft => (l, t) = (r - w, b - h),
            HandleKind::TopRight => (r, t) = (l + w, b - h),
            HandleKind::BottomRight => (r, b) = (l + w, t + h),
            _ => (l, b) = (r - w, t + h),
        }
    }
    (l, t, r, b)
}

/// Smart guides (DW5): the moved rectangle's edges and centre snap to the
/// other objects' within `reach`; the guides found are drawn.
fn smart_guides(
    rect: Rect,
    moved: (f64, f64),
    others: &[Rect],
    reach: f64,
    guides: &mut Vec<(At, At)>,
) -> (f64, f64) {
    let lines = |rect: &Rect, horizontal: bool| -> [f64; 3] {
        if horizontal {
            [
                f64::from(rect.left),
                (f64::from(rect.left) + f64::from(rect.right)) / 2.0,
                f64::from(rect.right),
            ]
        } else {
            [
                f64::from(rect.top),
                (f64::from(rect.top) + f64::from(rect.bottom)) / 2.0,
                f64::from(rect.bottom),
            ]
        }
    };
    let mut result = moved;
    for horizontal in [true, false] {
        let shift = if horizontal { moved.0 } else { moved.1 };
        let mine = lines(&rect, horizontal).map(|value| value + shift);
        let mut best: Option<(f64, f64, &Rect)> = None;
        for other in others {
            for theirs in lines(other, horizontal) {
                for value in mine {
                    let gap = theirs - value;
                    if gap.abs() <= reach && best.is_none_or(|(found, ..)| gap.abs() < found.abs())
                    {
                        best = Some((gap, theirs, other));
                    }
                }
            }
        }
        if let Some((gap, at, other)) = best {
            let span = |a: Rect| {
                if horizontal {
                    (f64::from(a.top), f64::from(a.bottom))
                } else {
                    (f64::from(a.left), f64::from(a.right))
                }
            };
            let (s0, s1) = span(*other);
            let (m0, m1) = span(rect);
            let offset = if horizontal { moved.1 } else { moved.0 };
            let (from, to) = (s0.min(m0 + offset), s1.max(m1 + offset));
            if horizontal {
                result.0 += gap;
                guides.push(((at, from), (at, to)));
            } else {
                result.1 += gap;
                guides.push(((from, at), (to, at)));
            }
        }
    }
    result
}

/// An object's outline as polylines in its twips (curves flattened), for
/// hit tests and the overlay; closed when true.
pub(crate) fn outlines(object: &Object) -> Vec<(Vec<At>, bool)> {
    let cubic = |out: &mut Vec<At>, p0: At, p1: At, p2: At, p3: At| {
        for step in 1..=16 {
            let t = f64::from(step) / 16.0;
            let u = 1.0 - t;
            out.push((
                u * u * u * p0.0
                    + 3.0 * u * u * t * p1.0
                    + 3.0 * u * t * t * p2.0
                    + t * t * t * p3.0,
                u * u * u * p0.1
                    + 3.0 * u * u * t * p1.1
                    + 3.0 * u * t * t * p2.1
                    + t * t * t * p3.1,
            ));
        }
    };
    let points = |list: &[Point]| list.iter().map(|&point| as_at(point)).collect::<Vec<_>>();
    match &object.shape {
        Shape::Line { points: p, .. } => vec![(points(p), false)],
        Shape::Polyline { points: p, .. } => vec![(points(p), false)],
        Shape::Polygon { points: p, .. } => vec![(points(p), true)],
        Shape::Rectangle { corners, .. } => vec![(points(corners), true)],
        Shape::RoundedRectangle { points: p, .. } => {
            let p = points(p);
            let mut out = vec![p[0]];
            for corner in 0..4 {
                let at = corner * 4;
                if corner > 0 {
                    out.push(p[at]);
                }
                cubic(&mut out, p[at], p[at + 1], p[at + 2], p[at + 3]);
            }
            vec![(out, true)]
        }
        Shape::Ellipse { points: p, .. } => {
            let p = points(p);
            let mut out = vec![p[0]];
            for segment in 0..4 {
                let at = segment * 3;
                cubic(&mut out, p[at], p[at + 1], p[at + 2], p[at + 3]);
            }
            vec![(out, true)]
        }
        Shape::Arc { points: p, .. } => {
            let p = points(p);
            let mut out = vec![p[0]];
            cubic(&mut out, p[0], p[1], p[2], p[3]);
            vec![(out, false)]
        }
        Shape::Text(text) => vec![(points(&text.corners()), true)],
        Shape::Group { objects, .. } => objects.iter().flat_map(outlines).collect(),
        Shape::Bitmap { .. } | Shape::Other(_) => {
            let r = object.rect;
            vec![(
                points(&[
                    Point::new(r.left, r.top),
                    Point::new(r.right, r.top),
                    Point::new(r.right, r.bottom),
                    Point::new(r.left, r.bottom),
                ]),
                true,
            )]
        }
    }
}

fn distance_to_segment(at: At, a: At, b: At) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        (((at.0 - a.0) * dx + (at.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    };
    let (x, y) = (a.0 + t * dx, a.1 + t * dy);
    ((at.0 - x).powi(2) + (at.1 - y).powi(2)).sqrt()
}

fn inside(at: At, polygon: &[At]) -> bool {
    let mut inside = false;
    let mut j = polygon.len().wrapping_sub(1);
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[j]);
        if (a.1 > at.1) != (b.1 > at.1) && at.0 < (b.0 - a.0) * (at.1 - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Whether `at` is on `object`: inside a filled shape, text, or a bitmap;
/// near an outline.
fn hit(object: &Object, at: At, reach: f64) -> bool {
    if object.kind == sdw::kind::CHART {
        return false;
    }
    let reach = reach + object.pen().map_or(0.0, |pen| f64::from(pen.width) / 2.0);
    let filled = object.fill().is_some_and(Fill::visible)
        || matches!(object.shape, Shape::Text(_) | Shape::Bitmap { .. });
    outlines(object).iter().any(|(line, closed)| {
        let segments = line.windows(2).map(|pair| (pair[0], pair[1]));
        let closing = closed.then(|| {
            (
                *line.last().unwrap_or(&(0.0, 0.0)),
                *line.first().unwrap_or(&(0.0, 0.0)),
            )
        });
        segments
            .chain(closing)
            .any(|(a, b)| distance_to_segment(at, a, b) <= reach)
            || (filled && *closed && inside(at, line))
    })
}

/// The top-level objects at `at`, front first.
fn objects_at(drawing: &Drawing, at: At, reach: f64) -> Vec<usize> {
    (0..drawing.objects.len())
        .rev()
        .filter(|&index| hit(&drawing.objects[index], at, reach))
        .collect()
}

/// A polyline's or polygon's segment near `at` (its first point's index).
fn segment_at(object: &Object, at: At, reach: f64) -> Option<usize> {
    let (points, closed) = match &object.shape {
        Shape::Polyline { points, .. } => (points, false),
        Shape::Polygon { points, .. } => (points, true),
        _ => return None,
    };
    let count = points.len();
    let segments = if closed {
        count
    } else {
        count.saturating_sub(1)
    };
    (0..segments).find(|&index| {
        distance_to_segment(at, as_at(points[index]), as_at(points[(index + 1) % count])) <= reach
    })
}

/// Point editing: a shape point moved (`DrawMan` `0x52f`).
fn set_point(object: &mut Object, index: usize, to: Point) {
    match &mut object.shape {
        Shape::Line { points, .. } => {
            if let Some(point) = points.get_mut(index) {
                *point = to;
            }
        }
        Shape::Polyline { points, .. } | Shape::Polygon { points, .. } => {
            if let Some(point) = points.get_mut(index) {
                *point = to;
            }
        }
        Shape::Rectangle { corners, .. } => {
            if let Some(point) = corners.get_mut(index) {
                *point = to;
            }
        }
        Shape::RoundedRectangle { points, .. } => {
            if let Some(point) = points.get_mut(index) {
                *point = to;
            }
        }
        Shape::Ellipse { points, .. } => {
            if index < 12 {
                points[index] = to;
                if index == 0 {
                    points[12] = to;
                }
            }
        }
        Shape::Arc { points, .. } => {
            if let Some(point) = points.get_mut(index) {
                *point = to;
            }
        }
        _ => return,
    }
    object.refresh_rect();
}

/// A vertex added after `segment` (a drag on a polyline's segment).
fn insert_point(object: &mut Object, segment: usize, at: At) -> bool {
    match &mut object.shape {
        Shape::Polyline { points, .. } | Shape::Polygon { points, .. } => {
            points.insert(segment + 1, to_point(at));
            object.refresh_rect();
            true
        }
        _ => false,
    }
}

/// A vertex deleted (a double-click on it), keeping a polyline's two and a
/// polygon's three.
fn delete_point(object: &mut Object, index: usize) -> bool {
    let removed = match &mut object.shape {
        Shape::Polyline { points, .. } if points.len() > 2 => {
            points.remove(index);
            true
        }
        Shape::Polygon { points, .. } if points.len() > 3 => {
            points.remove(index);
            true
        }
        _ => false,
    };
    if removed {
        object.refresh_rect();
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(text: &Text) -> (f64, f64) {
        (
            text.text.chars().count() as f64 * 120.0,
            f64::from(text.height.unsigned_abs()) * 1.15,
        )
    }

    fn context(drawing: &Drawing) -> Context<'_> {
        Context {
            drawing,
            pixel: 15.0,
            measure: &measure,
        }
    }

    fn mode() -> DrawMode {
        let mut document = amipro_document::Document::from_plain_text("");
        DrawMode::new(document.allocate_frame_id())
    }

    /// Runs a drag from `a` to `b` and returns the new drawing.
    fn drag(mode: &mut DrawMode, drawing: &Drawing, a: At, b: At, keys: Keys) -> Option<Drawing> {
        let context = context(drawing);
        mode.handle(
            Event::Press {
                at: a,
                keys,
                double: false,
            },
            &context,
        );
        mode.handle(Event::Drag { at: b, keys }, &context);
        mode.handle(Event::Release { at: b, keys }, &context)
            .drawing
    }

    #[test]
    fn the_shape_tools_draw_with_shift_and_the_current_look() {
        let mut mode = mode();
        mode.tool = Tool::Rectangle;
        let drawing = Drawing::new();
        // A click makes nothing (40 twips each way).
        assert!(
            drag(
                &mut mode,
                &drawing,
                (100.0, 100.0),
                (130.0, 120.0),
                Keys::default()
            )
            .is_none()
        );
        let drawing = drag(
            &mut mode,
            &drawing,
            (100.0, 100.0),
            (1100.0, 600.0),
            Keys::default(),
        )
        .unwrap();
        assert_eq!(drawing.objects[0].rect, Rect::new(100, 100, 1100, 600));
        assert_eq!(drawing.objects[0].kind, sdw::kind::RECTANGLE);
        // The tool stays, and the new object is selected.
        assert_eq!(
            (mode.tool, mode.selection.clone()),
            (Tool::Rectangle, vec![0])
        );
        // Shift: a square, kept as Ami Pro's square type.
        let shift = Keys {
            shift: true,
            ..Keys::default()
        };
        let drawing = drag(&mut mode, &drawing, (0.0, 0.0), (900.0, 400.0), shift).unwrap();
        assert_eq!(drawing.objects[1].rect, Rect::new(0, 0, 900, 900));
        assert_eq!(drawing.objects[1].kind, sdw::kind::SQUARE);
        // A line with Shift goes level.
        mode.tool = Tool::Line;
        let drawing = drag(&mut mode, &drawing, (0.0, 0.0), (1000.0, 90.0), shift).unwrap();
        assert_eq!(drawing.objects[2].rect, Rect::new(0, 0, 1000, 1));
        // Snap to the grid.
        mode.tool = Tool::Ellipse;
        mode.grid.snap = true;
        let drawing = drag(
            &mut mode,
            &drawing,
            (170.0, 190.0),
            (1100.0, 700.0),
            Keys::default(),
        )
        .unwrap();
        assert_eq!(drawing.objects[3].rect, Rect::new(0, 360, 1080, 720));
    }

    #[test]
    fn polylines_take_clicks_and_end_on_a_double_click() {
        let mut mode = mode();
        mode.tool = Tool::Polygon;
        let drawing = Drawing::new();
        let context = context(&drawing);
        let click = |mode: &mut DrawMode, at: At, double: bool| {
            let keys = Keys::default();
            let out = mode.handle(Event::Press { at, keys, double }, &context);
            let release = mode.handle(Event::Release { at, keys }, &context);
            out.drawing.or(release.drawing)
        };
        assert!(click(&mut mode, (0.0, 0.0), false).is_none());
        assert!(mode.tracking());
        assert!(click(&mut mode, (1000.0, 0.0), false).is_none());
        assert!(click(&mut mode, (1000.0, 800.0), false).is_none());
        // A click near the first point closes the polygon.
        let made = click(&mut mode, (20.0, 10.0), false).unwrap();
        let Shape::Polygon { points, .. } = &made.objects[0].shape else {
            panic!();
        };
        assert_eq!(points.len(), 3);
        assert!(!mode.tracking());
        // A polyline ends on a double-click.
        mode.tool = Tool::Polyline;
        let context = super::tests::context(&made);
        let keys = Keys::default();
        mode.handle(
            Event::Press {
                at: (0.0, 2000.0),
                keys,
                double: false,
            },
            &context,
        );
        mode.handle(
            Event::Release {
                at: (0.0, 2000.0),
                keys,
            },
            &context,
        );
        mode.handle(
            Event::Press {
                at: (900.0, 2000.0),
                keys,
                double: false,
            },
            &context,
        );
        mode.handle(
            Event::Release {
                at: (900.0, 2000.0),
                keys,
            },
            &context,
        );
        let ended = mode
            .handle(
                Event::Press {
                    at: (900.0, 2600.0),
                    keys,
                    double: true,
                },
                &context,
            )
            .drawing
            .unwrap();
        assert_eq!(ended.objects.len(), 2);
        assert!(matches!(ended.objects[1].shape, Shape::Polyline { .. }));
    }

    fn two() -> Drawing {
        Drawing {
            objects: vec![
                Object::rectangle(
                    Rect::new(0, 0, 1000, 1000),
                    Pen::default(),
                    Fill::solid(Colour::rgb(255, 0, 0)),
                    false,
                ),
                Object::ellipse(
                    Rect::new(2000, 0, 3000, 1000),
                    Pen::default(),
                    Fill::default(),
                    false,
                ),
            ],
            ..Drawing::new()
        }
    }

    #[test]
    fn selecting_moving_sizing_and_copying() {
        let mut mode = mode();
        let drawing = two();
        let context = context(&drawing);
        let keys = Keys::default();
        // A click inside the filled rectangle selects it; inside the hollow
        // ellipse nothing (only its line).
        mode.handle(
            Event::Press {
                at: (500.0, 500.0),
                keys,
                double: false,
            },
            &context,
        );
        mode.handle(
            Event::Release {
                at: (500.0, 500.0),
                keys,
            },
            &context,
        );
        assert_eq!(mode.selection, [0]);
        mode.handle(
            Event::Press {
                at: (2500.0, 500.0),
                keys,
                double: false,
            },
            &context,
        );
        mode.handle(
            Event::Release {
                at: (2500.0, 500.0),
                keys,
            },
            &context,
        );
        assert!(mode.selection.is_empty());
        // Its line selects it.
        mode.handle(
            Event::Press {
                at: (2005.0, 500.0),
                keys,
                double: false,
            },
            &context,
        );
        mode.handle(
            Event::Release {
                at: (2005.0, 500.0),
                keys,
            },
            &context,
        );
        assert_eq!(mode.selection, [1]);
        // A marquee takes what it touches.
        let moved = drag(&mut mode, &drawing, (-100.0, -100.0), (2100.0, 200.0), keys);
        assert!(moved.is_none());
        assert_eq!(mode.selection, [0, 1]);
        // A drag moves both; with Shift it copies.
        let moved = drag(&mut mode, &drawing, (500.0, 500.0), (600.0, 700.0), keys).unwrap();
        assert_eq!(moved.objects[0].rect, Rect::new(100, 200, 1100, 1200));
        mode.selection = vec![0];
        // Shift held at the release copies (Ami Pro).
        mode.handle(
            Event::Press {
                at: (500.0, 500.0),
                keys,
                double: false,
            },
            &context,
        );
        mode.handle(
            Event::Drag {
                at: (500.0, 1500.0),
                keys,
            },
            &context,
        );
        let copied = mode
            .handle(
                Event::Release {
                    at: (500.0, 1500.0),
                    keys: Keys {
                        shift: true,
                        ..keys
                    },
                },
                &context,
            )
            .drawing
            .unwrap();
        assert_eq!(copied.objects.len(), 3);
        assert_eq!(mode.selection, [2]);
        // The bottom-right handle sizes; past the opposite side it flips.
        mode.selection = vec![0];
        let sized = drag(&mut mode, &drawing, (1000.0, 1000.0), (2000.0, 500.0), keys).unwrap();
        assert_eq!(sized.objects[0].rect, Rect::new(0, 0, 2000, 500));
        let flipped = drag(
            &mut mode,
            &drawing,
            (1000.0, 1000.0),
            (-500.0, 1000.0),
            keys,
        )
        .unwrap();
        let Shape::Rectangle { corners, .. } = &flipped.objects[0].shape else {
            panic!();
        };
        assert_eq!(corners[0], Point::new(0, 0));
        assert_eq!(flipped.objects[0].rect, Rect::new(-500, 0, 0, 1000));
    }

    #[test]
    fn keys_nudge_cycle_delete_and_leave() {
        let mut mode = mode();
        let drawing = two();
        let context = context(&drawing);
        let keys = Keys::default();
        assert!(
            mode.handle(Event::Key(Key::Tab, keys), &context)
                .drawing
                .is_none()
        );
        assert_eq!(mode.selection, [0]);
        let nudged = mode.handle(
            Event::Key(
                Key::Right,
                Keys {
                    shift: true,
                    ..keys
                },
            ),
            &context,
        );
        assert!(nudged.amend);
        assert_eq!(nudged.drawing.unwrap().objects[0].rect.left, 720);
        let gone = mode
            .handle(Event::Key(Key::Delete, keys), &context)
            .drawing
            .unwrap();
        assert_eq!(gone.objects.len(), 1);
        assert!(mode.handle(Event::Key(Key::Escape, keys), &context).leave);
    }

    #[test]
    fn text_objects_are_typed_in_place() {
        let mut mode = mode();
        mode.tool = Tool::Text;
        let drawing = Drawing::new();
        let context = context(&drawing);
        let keys = Keys::default();
        mode.handle(
            Event::Press {
                at: (300.0, 400.0),
                keys,
                double: false,
            },
            &context,
        );
        mode.handle(
            Event::Release {
                at: (300.0, 400.0),
                keys,
            },
            &context,
        );
        let first = mode.handle(Event::Text("Hi".into()), &context);
        assert!(!first.amend);
        let drawing = first.drawing.unwrap();
        let context = super::tests::context(&drawing);
        let second = mode.handle(Event::Text("!".into()), &context);
        assert!(second.amend);
        let drawing = second.drawing.unwrap();
        let Shape::Text(text) = &drawing.objects[0].shape else {
            panic!();
        };
        assert_eq!(
            (text.text.as_str(), text.origin),
            ("Hi!", Point::new(300, 400))
        );
        assert_eq!(text.size, Point::new(360, 276));
        assert_eq!(text.face, "Times New Roman");
        // Enter starts another below.
        let context = super::tests::context(&drawing);
        mode.handle(Event::Key(Key::Enter, keys), &context);
        let next = mode
            .handle(Event::Text("Two".into()), &context)
            .drawing
            .unwrap();
        assert_eq!(next.objects.len(), 2);
        let Shape::Text(text) = &next.objects[1].shape else {
            panic!();
        };
        assert_eq!(text.origin, Point::new(300, 688));
    }

    #[test]
    fn point_editing_moves_adds_and_deletes_points_and_turns() {
        let mut mode = mode();
        let drawing = Drawing {
            objects: vec![Object::polygon(
                vec![
                    Point::new(0, 0),
                    Point::new(1000, 0),
                    Point::new(1000, 1000),
                    Point::new(0, 1000),
                ],
                Pen::default(),
                Fill::solid(Colour::BLACK),
            )],
            ..Drawing::new()
        };
        let context = context(&drawing);
        let keys = Keys::default();
        mode.handle(
            Event::Press {
                at: (500.0, 500.0),
                keys,
                double: true,
            },
            &context,
        );
        assert_eq!(mode.editing, Some(0));
        // A point dragged.
        let moved = drag(
            &mut mode,
            &drawing,
            (1000.0, 1000.0),
            (1500.0, 1200.0),
            keys,
        )
        .unwrap();
        let Shape::Polygon { points, .. } = &moved.objects[0].shape else {
            panic!();
        };
        assert_eq!(points[2], Point::new(1500, 1200));
        // A segment dragged adds a point.
        let context = super::tests::context(&drawing);
        let added = mode
            .handle(
                Event::Press {
                    at: (500.0, 2.0),
                    keys,
                    double: false,
                },
                &context,
            )
            .drawing
            .unwrap();
        let Shape::Polygon { points, .. } = &added.objects[0].shape else {
            panic!();
        };
        assert_eq!(points.len(), 5);
        mode.handle(Event::Cancel, &context);
        // A double-click on a point deletes it.
        let deleted = mode
            .handle(
                Event::Press {
                    at: (0.0, 1000.0),
                    keys,
                    double: true,
                },
                &context,
            )
            .drawing
            .unwrap();
        let Shape::Polygon { points, .. } = &deleted.objects[0].shape else {
            panic!();
        };
        assert_eq!(points.len(), 3);
        // A corner arrow turns the object a quarter turn.
        let corner = turn_handles(drawing.objects[0].rect, 15.0)[1];
        let turned = drag(&mut mode, &drawing, corner, (1650.0, 1650.0), keys).unwrap();
        assert_eq!(turned.objects[0].rect, Rect::new(0, 0, 1000, 1000));
        assert_ne!(turned, drawing);
    }

    #[test]
    fn commands_group_restack_and_restyle() {
        let mut mode = mode();
        let drawing = two();
        let context = context(&drawing);
        assert!(mode.command(Command::SelectAll, &context).drawing.is_none());
        assert_eq!(mode.selection, [0, 1]);
        let grouped = mode.command(Command::Group, &context).drawing.unwrap();
        assert_eq!(grouped.objects.len(), 1);
        assert_eq!(mode.selection, [0]);
        let context = super::tests::context(&grouped);
        let ungrouped = mode.command(Command::Ungroup, &context).drawing.unwrap();
        assert_eq!(ungrouped.objects.len(), 2);
        // Extract the rectangle's look, apply it to the ellipse.
        let context = super::tests::context(&drawing);
        mode.selection = vec![0];
        mode.command(Command::Extract, &context);
        assert_eq!(mode.fill.pattern, Fill::SOLID);
        mode.selection = vec![1];
        let applied = mode.command(Command::Apply, &context).drawing.unwrap();
        assert_eq!(
            applied.objects[1].fill().unwrap().foreground,
            Colour::rgb(255, 0, 0)
        );
        // With nothing selected a style sets the look of new objects.
        mode.selection.clear();
        assert!(
            mode.command(
                Command::Style(StyleChange {
                    width: Some(60),
                    ..StyleChange::default()
                }),
                &context
            )
            .drawing
            .is_none()
        );
        assert_eq!(mode.pen.width, 60);
    }

    #[test]
    fn smart_guides_line_up_edges() {
        let mut guides = Vec::new();
        let moved = smart_guides(
            Rect::new(0, 0, 100, 100),
            (995.0, 300.0),
            &[Rect::new(1000, 0, 1500, 50)],
            60.0,
            &mut guides,
        );
        assert_eq!(moved, (1000.0, 300.0));
        assert_eq!(guides.len(), 1);
    }
}
